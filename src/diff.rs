use crate::controller::StatedController;
use crate::error::{Error, Result};
use crate::note::{Note, NoteId};
use crate::program::StatedProgram;
use crate::rank::{RankDisagreement, UnrankedSite};
use crate::take::{Take, Tempo};
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

    /// Where the two Takes put a channel on different Programs. Never a Rig
    /// difference: which Program is selected is in the file, and what it sounds
    /// like is not compared here at all.
    pub programs: Vec<ProgramDifference>,

    /// Where the two Takes hold different values for a Controller. Never a Rig
    /// difference either, and never a list of events: what is compared is what
    /// is in force (ADR-0007).
    pub controllers: Vec<ControllerDifference>,

    /// Where the two Takes place a causally-dependent pair of events at one
    /// Tick in the two possible orders — ADR-0008's amendment. Same events,
    /// same Ticks, and a synthesiser that hears two different things.
    pub rank_disagreements: Vec<RankDisagreement>,

    /// Where a Take writes such a pair across two tracks, so that the file
    /// states no order and this comparison could not be made.
    ///
    /// Not a difference, and `is_empty` does not consult it. It is here for the
    /// reason `tolerance_ticks` is: a reader told two Takes agree about ordering
    /// is owed the sites where the question could not be put.
    pub unranked_sites: Vec<UnrankedSite>,
    /// Where the two Takes are at different tempos. Read the same way, and
    /// added for the same reason: until it existed, a Take whose only change was
    /// its tempo made `is_empty` true, so `mid diff` printed "no differences"
    /// and exited 0 about two Takes nobody would mistake for each other by ear.
    /// See #32.
    pub tempos: Vec<TempoDifference>,

    /// Where the two Takes bend a channel differently. The state ADR-0007's
    /// Consequences name as the next one to join, joining. Same hole as tempo
    /// until it existed.
    pub bends: Vec<BendDifference>,
}

/// One stretch of the Piece and what tempo each Take is at across it.
///
/// A span rather than a row per statement, for the reason a Controller
/// difference is one: an accelerando is written as a run of tempo statements,
/// and reporting forty of them as forty differences is the failure ADR-0007
/// exists to prevent. `from` and `until` read as a half-open stretch, `None`
/// where the two never agree again.
///
/// No channel, because tempo has none: one tempo governs the whole Take.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TempoDifference {
    pub from: u32,
    pub until: Option<u32>,
    pub before: TempoSide,
    pub after: TempoSide,
}

/// What one Take is doing with the tempo across a span: at its start, at its
/// end, and the extremes it reaches anywhere in it.
///
/// Both extremes, where `ControllerSide` carries only a peak. A Controller runs
/// from nought upwards, so *the highest* is the whole of what a reader wants;
/// a tempo has no such floor, and a span whose two sides agree at both ends
/// would otherwise print two identical readings under a row asserting they
/// differ. Named in the music's terms rather than the file's — `fastest` is the
/// *smallest* number of microseconds per quarter note, and a field called
/// `lowest` would be read as the opposite of what it holds.
///
/// Every field is optional together, because a Take may state no tempo at all
/// across the span — the case a Take stating 120 must never be confused with.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TempoSide {
    pub at_start: Option<Tempo>,
    pub at_end: Option<Tempo>,
    pub fastest: Option<Tempo>,
    pub fastest_at: Option<u32>,
    pub slowest: Option<Tempo>,
    pub slowest_at: Option<u32>,
}

/// One channel, one stretch of the Piece, and how far each Take bends it there.
///
/// The Controller span shape, for the state that is not a Controller. See
/// `BendSide` for what each side carries and `StatedBend::value` for what the
/// numbers mean.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BendDifference {
    pub channel: u8,
    pub from: u32,
    pub until: Option<u32>,
    pub before: BendSide,
    pub after: BendSide,
}

/// How far one Take bends one channel across a span: at its start, at its end,
/// and the furthest each way anywhere in it.
///
/// Both directions, and here it is not a refinement but the only reading that
/// works. A span opens where the two Takes first differ, so `at_start` always
/// differs and always reveals *something*; what the extremes answer for is the
/// rest of the span. A bend is signed about a centre MIDI fixes, so a phrase
/// that dips below the note and returns never rises above where it began: a
/// single *peak* is equal to `at_start` for the whole of it and prints nothing,
/// and a span that opens on a difference of one unit reads as one unit while
/// containing a dive of four thousand. That is the reading this prevents.
///
/// Nought is a real value and is not `None`. A channel bent back to the centre
/// and a channel never bent are two different Pieces, the same distinction
/// `ControllerSide` draws between holding 0 and holding nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BendSide {
    pub at_start: Option<i16>,
    pub at_end: Option<i16>,
    pub furthest_down: Option<i16>,
    pub furthest_down_at: Option<u32>,
    pub furthest_up: Option<i16>,
    pub furthest_up_at: Option<u32>,
}

/// One Controller, one stretch of the Piece, and what each Take holds for it
/// there.
///
/// A span rather than a row per event, which is the whole of why forty
/// differences become one. `from` is the Tick the two Takes stop agreeing about
/// what is in force and `until` the Tick they agree again — `None` where they
/// never do, which is a different statement from agreeing at the last Tick
/// either of them happens to hold.
///
/// Nothing here claims that a stretch of events in one Take *is* a stretch in
/// the other, moved. That claim would need a parameter under ADR-0004; the two
/// sides are read separately and this only says where they differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ControllerDifference {
    pub channel: u8,
    pub controller: u8,
    pub from: u32,
    pub until: Option<u32>,
    pub before: ControllerSide,
    pub after: ControllerSide,
}

/// What one Take holds for one Controller across a span: at its start, at its
/// end, and the highest anywhere in it.
///
/// Every field is optional together, because a Take may hold nothing at all for
/// this Controller across the span — the case a Take stating 0 must never be
/// confused with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ControllerSide {
    pub at_start: Option<u8>,
    pub at_end: Option<u8>,
    pub peak: Option<u8>,
    pub peak_at: Option<u32>,
}

/// One channel, one moment, and the two Programs the Takes have it on there.
///
/// A state rather than an event, which is the whole of why this is readable. The
/// events are program change messages, and comparing those would report that a
/// byte moved between tracks or that an export re-stated the same Program at
/// every Bar. What a reader wants to know is that this part is on a horn now and
/// was on a violin before, and that is a question about what is *in force*.
///
/// `at` is the Tick from which the two disagree, and either side may be `None`:
/// a Take that states no Program for a channel and one that states program 0 are
/// two different Pieces, and this is the one place that difference is visible
/// rather than audible-by-accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ProgramDifference {
    pub channel: u8,
    pub at: u32,
    pub before: Option<u8>,
    pub after: Option<u8>,
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
    /// Whether the two Takes say the same thing about the Piece.
    ///
    /// The orchestration and the controller data count. A Take whose horn part
    /// was a violin part has
    /// changed, in the file and to the ear, and a diff answering "no differences"
    /// to it would be failing at the one job `CHARTER.md` gives it — being the
    /// surface a human checks the agent on.
    ///
    /// `tolerance_ticks` is still not a difference; it is what the matching was
    /// done with. See its own field.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.changed.is_empty()
            && self.programs.is_empty()
            && self.controllers.is_empty()
            && self.rank_disagreements.is_empty()
            && self.tempos.is_empty()
            && self.bends.is_empty()
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

    // A tolerance of nought asks for the first pass alone, and the bound cannot
    // deliver that by itself: a note replaced at its own Tick is nought Ticks
    // away, so the filter below admits it however tight the tolerance is, and
    // pitch reaches the comparison only as a tie-break. Not running the pass is
    // therefore the whole of the difference between matching by identity and
    // matching by nearness — #29.
    if tolerance_ticks > 0 {
        // Greedy, in the order `Take::notes` fixes — track order, then note-on
        // order — because greedy makes that order observable: two unmatched
        // notes the same distance from a candidate produce different pairings
        // depending on which is reached first. The order is part of `notes`'
        // contract, so the answer is the same on every run rather than whatever
        // iteration happened to do.
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
                // Nearest in Ticks, which is what the tolerance bounds. Pitch
                // breaks a tie because a transposed note sits at the same Tick
                // as whatever else did not move, and the after Take's own order
                // breaks the rest.
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

    let (rank_disagreements, unranked_sites) = crate::rank::rank_differences(before, after)?;

    Ok(Diff {
        tolerance_ticks,
        added,
        removed,
        changed,
        programs: program_differences(before, after)?,
        controllers: controller_differences(before, after)?,
        rank_disagreements,
        unranked_sites,
        tempos: tempo_differences(before, after)?,
        bends: bend_differences(before, after)?,
    })
}

/// One stretch two readings of one state disagree over, each side read as
/// scalars.
///
/// `Stretch` and the two functions below it are the state comparison ADR-0007
/// asks for, with the state itself taken out: what is left is one algorithm over
/// a list of `(Tick, number)` pairs, which is what a Program, a Controller, a
/// tempo and a bend all reduce to. `i64` holds every one of them — a Controller's
/// seven bits, a bend's signed fourteen, a tempo's microseconds — so no caller
/// has to widen or lose anything to be compared here.
///
/// `controller_differences` predates this and still has its own copy of the
/// algorithm. Moving it over would mean either renaming its shipped `peak`
/// field or reading `highest` back into it, and #29's compatibility constraint
/// says not this release.
struct Stretch {
    from: u32,
    until: Option<u32>,
    before: Reading,
    after: Reading,
}

/// One Take's reading of one stretch: the value in force at each end, and the
/// extremes it reaches inside, each with the Tick *within the stretch* it is
/// first reached at.
///
/// Within, and that is the part to read twice. Where an extreme is the value
/// already in force when the stretch opens, its Tick is the stretch's own start
/// and not wherever the statement that set it happens to be — which may be many
/// Bars earlier, and is a place this stretch says nothing about. A reader
/// addressing an Edit at the Tick reported here is addressing the stretch being
/// described; a reader expecting the statement's own Tick is reading a field that
/// was never offering one. `ControllerSide::peak_at` has always worked this way.
struct Reading {
    at_start: Option<i64>,
    at_end: Option<i64>,
    lowest: Option<(i64, u32)>,
    highest: Option<(i64, u32)>,
}

/// Every stretch two readings of one state disagree over.
///
/// Compared at every Tick either reading says anything, plus Tick 0, because a
/// Take that opens holding a value and one that holds nothing differ from the
/// first note. A run of moments that disagree collapses into the one span that
/// states it, and `until` is the Tick they agree again rather than the last one
/// they were apart — half-open, as `ControllerDifference` documents.
///
/// Each list must be sorted by Tick, with the last statement at a Tick the one
/// in force there. Every `stated_*` reading on `Take` fixes that.
fn stretches(before: &[(u32, i64)], after: &[(u32, i64)]) -> Vec<Stretch> {
    let mut moments: Vec<u32> = std::iter::once(0)
        .chain(before.iter().chain(after.iter()).map(|&(tick, _)| tick))
        .collect();
    moments.sort_unstable();
    moments.dedup();

    let mut found = Vec::new();
    let mut open: Option<u32> = None;
    for &at in &moments {
        let differs = scalar_in_force(before, at) != scalar_in_force(after, at);
        match (open, differs) {
            (None, true) => open = Some(at),
            (Some(from), false) => {
                found.push(Stretch {
                    from,
                    until: Some(at),
                    before: reading(before, from, Some(at), &moments),
                    after: reading(after, from, Some(at), &moments),
                });
                open = None;
            }
            _ => {}
        }
    }
    if let Some(from) = open {
        found.push(Stretch {
            from,
            until: None,
            before: reading(before, from, None, &moments),
            after: reading(after, from, None, &moments),
        });
    }
    found
}

/// What one reading holds across a stretch.
///
/// The extremes consider the value in force at `from` as well as everything
/// stated inside: that value is in force during the stretch like any other, and
/// a stretch the reading says nothing new in still holds something. Strictly
/// beyond, so a value reached twice is reported at the first of the two — the
/// rule `ControllerSide`'s peak already follows.
fn reading(stated: &[(u32, i64)], from: u32, until: Option<u32>, moments: &[u32]) -> Reading {
    let at_start = scalar_in_force(stated, from);
    let last = moments
        .iter()
        .copied()
        .rfind(|&at| until.map(|until| at < until).unwrap_or(true))
        .unwrap_or(from);

    let mut lowest = at_start.map(|value| (value, from));
    let mut highest = lowest;
    for &(tick, value) in stated
        .iter()
        .filter(|&&(tick, _)| from < tick && until.map(|until| tick < until).unwrap_or(true))
    {
        if lowest.map(|(low, _)| value < low).unwrap_or(true) {
            lowest = Some((value, tick));
        }
        if highest.map(|(high, _)| value > high).unwrap_or(true) {
            highest = Some((value, tick));
        }
    }

    Reading {
        at_start,
        at_end: scalar_in_force(stated, last),
        lowest,
        highest,
    }
}

/// The value in force at a Tick: the last thing said at or before it, in the
/// order the list arrived in.
fn scalar_in_force(stated: &[(u32, i64)], at: u32) -> Option<i64> {
    stated
        .iter()
        .filter(|&&(tick, _)| tick <= at)
        .map(|&(_, value)| value)
        .next_back()
}

/// Where the two Takes are at different tempos.
///
/// One comparison rather than one per channel, because a tempo governs the whole
/// Take: `stated_tempos` reads every statement wherever it was written, and the
/// last at or before a moment is the one in force there.
fn tempo_differences(before: &Take, after: &Take) -> Result<Vec<TempoDifference>> {
    let scalars = |take: &Take| -> Result<Vec<(u32, i64)>> {
        Ok(take
            .stated_tempos()?
            .into_iter()
            .map(|stated| (stated.tick, i64::from(stated.tempo.micros_per_quarter)))
            .collect())
    };
    let before_stated = scalars(before)?;
    let after_stated = scalars(after)?;

    // The extremes swap names on the way out. Fewer microseconds to the quarter
    // note is a faster tempo, so the lowest number is the fastest reading —
    // which is why `TempoSide` names them the way a musician would and this is
    // the one place the inversion is written.
    let side = |reading: Reading| TempoSide {
        at_start: reading.at_start.map(as_tempo),
        at_end: reading.at_end.map(as_tempo),
        fastest: reading.lowest.map(|(micros, _)| as_tempo(micros)),
        fastest_at: reading.lowest.map(|(_, tick)| tick),
        slowest: reading.highest.map(|(micros, _)| as_tempo(micros)),
        slowest_at: reading.highest.map(|(_, tick)| tick),
    };

    Ok(stretches(&before_stated, &after_stated)
        .into_iter()
        .map(|stretch| TempoDifference {
            from: stretch.from,
            until: stretch.until,
            before: side(stretch.before),
            after: side(stretch.after),
        })
        .collect())
}

/// A tempo back from the scalar it was compared as.
///
/// Every number here came out of `stated_tempos`, so it is a microseconds count
/// that fitted a `u32` before it was widened, and fits one again.
fn as_tempo(micros: i64) -> Tempo {
    Tempo::from_micros_per_quarter(micros as u32)
}

/// Where the two Takes bend a channel differently.
///
/// Per channel, because a bend is channel state: bending one channel says
/// nothing about any other. A channel neither Take ever bends cannot differ and
/// is not considered.
fn bend_differences(before: &Take, after: &Take) -> Result<Vec<BendDifference>> {
    let before_stated = before.stated_bends()?;
    let after_stated = after.stated_bends()?;

    let mut channels: Vec<u8> = before_stated
        .iter()
        .chain(after_stated.iter())
        .map(|stated| stated.channel)
        .collect();
    channels.sort_unstable();
    channels.dedup();

    let side = |reading: Reading| BendSide {
        at_start: reading.at_start.map(as_bend),
        at_end: reading.at_end.map(as_bend),
        furthest_down: reading.lowest.map(|(value, _)| as_bend(value)),
        furthest_down_at: reading.lowest.map(|(_, tick)| tick),
        furthest_up: reading.highest.map(|(value, _)| as_bend(value)),
        furthest_up_at: reading.highest.map(|(_, tick)| tick),
    };

    let mut differences = Vec::new();
    for channel in channels {
        let on = |stated: &[crate::bend::StatedBend]| -> Vec<(u32, i64)> {
            stated
                .iter()
                .filter(|stated| stated.channel == channel)
                .map(|stated| (stated.tick, i64::from(stated.value)))
                .collect()
        };
        differences.extend(
            stretches(&on(&before_stated), &on(&after_stated))
                .into_iter()
                .map(|stretch| BendDifference {
                    channel,
                    from: stretch.from,
                    until: stretch.until,
                    before: side(stretch.before),
                    after: side(stretch.after),
                }),
        );
    }
    Ok(differences)
}

/// A bend back from the scalar it was compared as. `stated_bends` is where the
/// number came from, and it is `midly`'s signed reading of fourteen bits.
fn as_bend(value: i64) -> i16 {
    value as i16
}

/// Where the two Takes have a channel on different Programs.
///
/// Compared at every Tick either Take says anything on that channel, plus Tick
/// 0, because a Take that opens with a Program and one that opens with none
/// differ from the first note. Consecutive moments agreeing on the same
/// disagreement collapse into the one row that states it: a Take that switches
/// to program 60 at Bar 3 while the other never does disagrees from Bar 3
/// onwards, and saying so once is saying it.
///
/// A channel neither Take ever states a Program for cannot differ and is not
/// considered. Two Takes that state the same Program at different Ticks *do*
/// differ, and the row falls at the earlier of the two — which is the Tick from
/// which they are on different instruments.
fn program_differences(before: &Take, after: &Take) -> Result<Vec<ProgramDifference>> {
    let before_stated = before.stated_programs()?;
    let after_stated = after.stated_programs()?;

    let mut channels: Vec<u8> = before_stated
        .iter()
        .chain(after_stated.iter())
        .map(|stated| stated.channel)
        .collect();
    channels.sort_unstable();
    channels.dedup();

    let mut differences = Vec::new();
    for channel in channels {
        let mut moments: Vec<u32> = std::iter::once(0)
            .chain(
                before_stated
                    .iter()
                    .chain(after_stated.iter())
                    .filter(|stated| stated.channel == channel)
                    .map(|stated| stated.tick),
            )
            .collect();
        moments.sort_unstable();
        moments.dedup();

        let mut said: Option<(Option<u8>, Option<u8>)> = None;
        for at in moments {
            let pair = (
                in_force(&before_stated, channel, at),
                in_force(&after_stated, channel, at),
            );
            if pair.0 == pair.1 {
                said = None;
                continue;
            }
            if said == Some(pair) {
                continue;
            }
            said = Some(pair);
            differences.push(ProgramDifference {
                channel,
                at,
                before: pair.0,
                after: pair.1,
            });
        }
    }
    Ok(differences)
}

/// Where the two Takes hold different values for a Controller.
///
/// Compared at every Tick either Take says anything about that Controller, plus
/// Tick 0, because a Take that opens holding a value and one that holds nothing
/// differ from the first note. A run of moments that disagree collapses into the
/// one span that states it, which is what turns a crescendo's forty events into
/// a row.
///
/// A span closes at the first moment the two agree again, and `until` is that
/// Tick — the moment they are back together, not the last moment they were
/// apart, so that reading `from` and `until` as a half-open stretch is reading
/// it correctly. A span that never closes carries `None`.
fn controller_differences(before: &Take, after: &Take) -> Result<Vec<ControllerDifference>> {
    let before_stated = before.stated_controllers()?;
    let after_stated = after.stated_controllers()?;

    let mut pairs: Vec<(u8, u8)> = before_stated
        .iter()
        .chain(after_stated.iter())
        .map(|stated| (stated.channel, stated.controller))
        .collect();
    pairs.sort_unstable();
    pairs.dedup();

    let mut differences = Vec::new();
    for (channel, controller) in pairs {
        let mut moments: Vec<u32> = std::iter::once(0)
            .chain(
                before_stated
                    .iter()
                    .chain(after_stated.iter())
                    .filter(|stated| stated.channel == channel && stated.controller == controller)
                    .map(|stated| stated.tick),
            )
            .collect();
        moments.sort_unstable();
        moments.dedup();

        let mut open: Option<u32> = None;
        for &at in &moments {
            let differs = held(&before_stated, channel, controller, at)
                != held(&after_stated, channel, controller, at);
            match (open, differs) {
                (None, true) => open = Some(at),
                (Some(from), false) => {
                    differences.push(span(
                        &before_stated,
                        &after_stated,
                        channel,
                        controller,
                        from,
                        Some(at),
                        &moments,
                    ));
                    open = None;
                }
                _ => {}
            }
        }
        if let Some(from) = open {
            differences.push(span(
                &before_stated,
                &after_stated,
                channel,
                controller,
                from,
                None,
                &moments,
            ));
        }
    }
    Ok(differences)
}

/// One span, with each Take's reading of it.
fn span(
    before_stated: &[StatedController],
    after_stated: &[StatedController],
    channel: u8,
    controller: u8,
    from: u32,
    until: Option<u32>,
    moments: &[u32],
) -> ControllerDifference {
    ControllerDifference {
        channel,
        controller,
        from,
        until,
        before: side(before_stated, channel, controller, from, until, moments),
        after: side(after_stated, channel, controller, from, until, moments),
    }
}

/// What one Take holds for one Controller across a span.
///
/// The peak considers the value in force at `from` as well as everything stated
/// inside the span: that value is in force during the span like any other, and a
/// span the Take says nothing new in still holds something. Strictly greater, so
/// a value reached twice is reported at the first of the two.
fn side(
    stated: &[StatedController],
    channel: u8,
    controller: u8,
    from: u32,
    until: Option<u32>,
    moments: &[u32],
) -> ControllerSide {
    let at_start = held(stated, channel, controller, from);
    let last = moments
        .iter()
        .copied()
        .rfind(|&at| until.map(|until| at < until).unwrap_or(true))
        .unwrap_or(from);

    let mut peak = at_start;
    let mut peak_at = at_start.map(|_| from);
    for inside in stated
        .iter()
        .filter(|stated| stated.channel == channel && stated.controller == controller)
        .filter(|stated| {
            from < stated.tick && until.map(|until| stated.tick < until).unwrap_or(true)
        })
    {
        if peak.map(|peak| inside.value > peak).unwrap_or(true) {
            peak = Some(inside.value);
            peak_at = Some(inside.tick);
        }
    }

    ControllerSide {
        at_start,
        at_end: held(stated, channel, controller, last),
        peak,
        peak_at,
    }
}

/// The value in force for a Controller on a channel at a Tick: the last thing
/// said at or before it, in the order `stated_controllers` fixes.
fn held(stated: &[StatedController], channel: u8, controller: u8, at: u32) -> Option<u8> {
    stated
        .iter()
        .filter(|stated| {
            stated.channel == channel && stated.controller == controller && stated.tick <= at
        })
        .map(|stated| stated.value)
        .next_back()
}

/// The Program in force on a channel at a Tick: the last thing said at or before
/// it, in the order `stated_programs` fixes.
fn in_force(stated: &[StatedProgram], channel: u8, at: u32) -> Option<u8> {
    stated
        .iter()
        .filter(|stated| stated.channel == channel && stated.tick <= at)
        .map(|stated| stated.program)
        .next_back()
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
