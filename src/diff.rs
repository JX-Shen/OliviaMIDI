use crate::error::{Error, Result};
use crate::note::{Note, NoteId};
use crate::take::Take;
use serde::Serialize;
use std::collections::HashMap;

/// What differs between two Takes, in the Piece's terms.
///
/// A diff never reports Rig differences. Two Takes heard through different
/// soundfonts are the same Take here, which is the point: "the brass sounds
/// wrong" has to stay answerable as either a Piece problem or a Rig problem.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diff {
    /// The tolerance this diff was matched with, in Ticks.
    ///
    /// Carried rather than left behind, because it is what decided whether a
    /// note "moved" or was "deleted and re-added" — a diff read next week whose
    /// grouping depended on a number nobody recorded cannot be interrogated,
    /// which is the same reason `play` states its Rig. It is not a difference,
    /// so it has no bearing on `is_empty`.
    pub tolerance_ticks: u32,
    pub added: Vec<Note>,
    pub removed: Vec<Note>,
    pub changed: Vec<NoteChange>,
}

/// One note of the before Take and the note of the after Take it was matched
/// with, together with everything about it that differs.
///
/// Both notes are carried whole, and that is the load-bearing part. A note that
/// moved or was transposed has a *different identity on each side* — pitch and
/// start Tick are content an identity is derived from (ADR-0002) — so there is
/// no single name to report it under, and either name alone would leave a reader
/// unable to find the note in one of the two Takes it is being told about.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NoteChange {
    pub before: Note,
    pub after: Note,
    /// Never empty, and always in the fixed order of `Change`.
    pub changes: Vec<Change>,
}

/// One thing about a matched note that differs. Reported as a set rather than
/// as a single verdict: a note that was both moved and softened underwent two
/// changes, and a diff that named only the first would be hiding one of them
/// from the one command whose job is to reveal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    Pitch,
    Start,
    Duration,
    Velocity,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// The default tolerance, as the divisor a Take's ticks-per-quarter-note is
/// divided by: a sixteenth note.
///
/// A tolerance in Ticks means nothing on its own — 120 Ticks is a sixteenth note
/// at 480 ticks per quarter and longer than a quarter note at 96 — so the
/// default is a note value and the Take turns it into Ticks. A sixteenth is
/// small enough not to pair two different notes of a dense texture and large
/// enough to cover the nudging a quantise or a humanise does. Anything wider is
/// a judgement about this Piece, and belongs to whoever is looking at it:
/// `--tolerance`.
const DEFAULT_TOLERANCE_DIVISOR: u32 = 4;

/// Compare two Takes: what was added, what was removed, and what stayed but
/// changed.
///
/// Matching is two passes (#6, under ADR-0004). First by identity — same track, channel,
/// pitch and start Tick — and then, among whatever is left, greedily by
/// nearest neighbour within the same track and channel, bounded by `tolerance`.
/// Whatever is still unmatched is Added or Removed.
///
/// The second pass is where this says something content addressing cannot. Two
/// notes matched there have *different identities*, and pairing them is a claim
/// that they are nonetheless the same note, moved. The tolerance is the whole of
/// the evidence for that claim, which is why it is a stated parameter and is
/// reported back on the `Diff`.
///
/// `tolerance` of `None` takes the default, which is a note value and so needs
/// the Take to say how many Ticks it is. `Some(0)` asks for the first pass
/// alone: matching by identity and nothing else.
///
/// Two Takes that count a different number of Ticks to the quarter note are
/// refused rather than compared. Every number here is a Tick, and Ticks in the
/// two files would not be the same unit — see `Error::PpqMismatch`.
pub fn diff(before: &Take, after: &Take, tolerance: Option<u32>) -> Result<Diff> {
    let before_ppq = before.ppq()?;
    let after_ppq = after.ppq()?;
    if before_ppq != after_ppq {
        return Err(Error::PpqMismatch {
            before: before.described_path(),
            before_ppq,
            after: after.described_path(),
            after_ppq,
        });
    }
    let tolerance_ticks =
        tolerance.unwrap_or_else(|| u32::from(before_ppq) / DEFAULT_TOLERANCE_DIVISOR);

    let before_notes = before.notes()?;
    let after_notes = after.notes()?;

    // Which after-note each before-note was matched with, by position in
    // `after_notes`, and the other way round. Positions rather than identities:
    // the second pass matches notes whose identities differ, so an identity is
    // not a key it can use.
    let mut matched_to: Vec<Option<usize>> = vec![None; before_notes.len()];
    let mut taken: Vec<bool> = vec![false; after_notes.len()];

    let after_by_id: HashMap<&NoteId, usize> = after_notes
        .iter()
        .enumerate()
        .map(|(index, note)| (&note.id, index))
        .collect();
    for (index, note) in before_notes.iter().enumerate() {
        if let Some(&found) = after_by_id.get(&note.id) {
            matched_to[index] = Some(found);
            taken[found] = true;
        }
    }

    // Greedy, in the order `Take::notes` fixes — track order, then note-on order
    // — because greedy makes that order observable: two unmatched notes the same
    // distance from a candidate produce different pairings depending on which is
    // reached first. The order is part of `notes`' contract, so the answer is
    // the same on every run rather than whatever iteration happened to do.
    for (index, note) in before_notes.iter().enumerate() {
        if matched_to[index].is_some() {
            continue;
        }
        let nearest = after_notes
            .iter()
            .enumerate()
            .filter(|&(candidate, other)| {
                !taken[candidate]
                    && other.track == note.track
                    && other.channel == note.channel
                    && other.start.abs_diff(note.start) <= tolerance_ticks
            })
            // Nearest in Ticks, which is what the tolerance bounds. Pitch breaks
            // a tie because a transposed note sits at the same Tick as whatever
            // else did not move, and the after Take's own order breaks the rest.
            .min_by_key(|&(candidate, other)| {
                (
                    other.start.abs_diff(note.start),
                    other.pitch.abs_diff(note.pitch),
                    candidate,
                )
            })
            .map(|(candidate, _)| candidate);
        if let Some(candidate) = nearest {
            matched_to[index] = Some(candidate);
            taken[candidate] = true;
        }
    }

    let mut removed = Vec::new();
    let mut changed = Vec::new();
    for (index, note) in before_notes.iter().enumerate() {
        let Some(candidate) = matched_to[index] else {
            removed.push(note.clone());
            continue;
        };
        let other = &after_notes[candidate];
        let changes = changes_between(note, other);
        // A matched pair that differs in nothing is not a difference. This is
        // reachable from the second pass as well as the first: two notes alike
        // in everything but their occurrence index have different identities and
        // are the same note.
        if !changes.is_empty() {
            changed.push(NoteChange {
                before: note.clone(),
                after: other.clone(),
                changes,
            });
        }
    }

    let added = after_notes
        .iter()
        .enumerate()
        .filter(|&(index, _)| !taken[index])
        .map(|(_, note)| note.clone())
        .collect();

    Ok(Diff {
        tolerance_ticks,
        added,
        removed,
        changed,
    })
}

/// Everything that differs between two matched notes, in the fixed order
/// pitch, start, duration, velocity.
///
/// The order is a presentation order over a set, not a priority that stops at
/// the first hit. Stopping would make a note that was moved *and* softened
/// report as moved alone, and the softening would be invisible in the one
/// command a human runs to find out what changed.
fn changes_between(before: &Note, after: &Note) -> Vec<Change> {
    [
        (Change::Pitch, before.pitch != after.pitch),
        (Change::Start, before.start != after.start),
        (Change::Duration, before.duration != after.duration),
        (Change::Velocity, before.velocity != after.velocity),
    ]
    .into_iter()
    .filter(|&(_, differs)| differs)
    .map(|(change, _)| change)
    .collect()
}
