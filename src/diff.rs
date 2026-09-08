use crate::controller::StatedController;
use crate::error::{Error, Result};
use crate::note::{Note, NoteId};
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
    /// Bend intervals excluded from value comparison, with each side's sources — #42.
    pub unranked_bends: Vec<UnrankedBend>,
    pub unranked_tempos: Vec<crate::UnrankedComparison<Tempo>>,
    pub unranked_programs: Vec<UnrankedProgram>,
    pub unranked_controllers: Vec<UnrankedController>,
    /// Channel-state/strike relations, distinct from continuing value conflicts — #42.
    pub unranked_state_sites: Vec<UnrankedStateSite>,
}

/// An uncomparable Bend interval. An empty side has no value conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnrankedBend {
    pub channel: u8,
    pub from: u32,
    pub until: Option<u32>,
    pub before: Vec<crate::Candidate<i16>>,
    pub after: Vec<crate::Candidate<i16>>,
}

/// Program intervals excluded from value comparison — #42.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnrankedProgram {
    pub channel: u8,
    #[serde(flatten)]
    pub interval: crate::UnrankedComparison<u8>,
}

/// Controller intervals excluded from value comparison — #42.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnrankedController {
    pub channel: u8,
    pub controller: u8,
    #[serde(flatten)]
    pub interval: crate::UnrankedComparison<u8>,
}

/// A Channel-state/strike site present in either Take; not a value interval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnrankedStateSite {
    pub site: crate::Unranked,
    pub in_before: bool,
    pub in_after: bool,
}

/// One stretch of the Piece and what tempo each Take is at across it.
///
/// A span rather than a row per statement, for the reason a Controller
/// difference is one: an accelerando is written as a run of tempo statements,
/// and reporting forty of them as forty differences is the failure ADR-0007
/// exists to prevent. `from` and `until` read as a half-open stretch, ending
/// at agreement or an indeterminate reading; `None` means neither occurs — #42.
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
/// what is in force and `until` the Tick they agree again or either reading
/// becomes indeterminate. `None` means neither occurs — #42.
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
    /// Exclusive end of this determinate value pair — #42.
    pub until: Option<u32>,
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
    /// Whether any part of the requested comparison could not be made — #42.
    pub fn has_unranked(&self) -> bool {
        !self.unranked_sites.is_empty()
            || !self.unranked_bends.is_empty()
            || !self.unranked_tempos.is_empty()
            || !self.unranked_programs.is_empty()
            || !self.unranked_controllers.is_empty()
            || !self.unranked_state_sites.is_empty()
    }

    /// Whether no determinate differences were found. See has_unranked — #42.
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

    let (rank_disagreements, unranked_sites) =
        crate::rank::rank_differences(before, after, &before_notes, &after_notes, &matched_to)?;

    let (bends, unranked_bends) = bend_differences(before, after)?;
    let (programs, unranked_programs) = program_differences(before, after)?;
    let (controllers, unranked_controllers) = controller_differences(before, after)?;
    let (tempos, unranked_tempos) = tempo_differences(before, after)?;
    Ok(Diff {
        tolerance_ticks,
        added,
        removed,
        changed,
        programs,
        unranked_programs,
        controllers,
        unranked_controllers,
        rank_disagreements,
        unranked_sites,
        tempos,
        unranked_tempos,
        bends,
        unranked_bends,
        unranked_state_sites: state_sites(before, after, &before_notes, &after_notes)?,
    })
}

/// Comparable differences and excluded intervals for one state address — #42.
struct Comparison<T> {
    moments: Vec<u32>,
    differences: Vec<(u32, Option<u32>)>,
    unranked: Vec<crate::UnrankedComparison<T>>,
}

fn compare<T: Copy + Eq>(
    before: &crate::reading::Timeline<T>,
    after: &crate::reading::Timeline<T>,
) -> Comparison<T> {
    let mut moments: Vec<_> = std::iter::once(0)
        .chain(
            before
                .moments
                .iter()
                .chain(&after.moments)
                .map(|(tick, _)| *tick),
        )
        .collect();
    moments.sort_unstable();
    moments.dedup();
    let mut differences = Vec::new();
    let mut unranked: Vec<crate::UnrankedComparison<T>> = Vec::new();
    let mut open = None;
    for (index, &at) in moments.iter().enumerate() {
        let mine = before.at(at);
        let theirs = after.at(at);
        let differs = match (mine.determinate(), theirs.determinate()) {
            (Some(mine), Some(theirs)) => mine != theirs,
            _ => {
                let until = moments.get(index + 1).copied();
                if let Some(last) = unranked.last_mut().filter(|last| {
                    last.until == Some(at)
                        && last.before == mine.candidates()
                        && last.after == theirs.candidates()
                }) {
                    last.until = until;
                } else {
                    unranked.push(crate::UnrankedComparison {
                        from: at,
                        until,
                        before: mine.candidates().to_vec(),
                        after: theirs.candidates().to_vec(),
                    });
                }
                false
            }
        };
        match (open, differs) {
            (None, true) => open = Some(at),
            (Some(from), false) => {
                differences.push((from, Some(at)));
                open = None;
            }
            _ => {}
        }
    }
    if let Some(from) = open {
        differences.push((from, None));
    }
    Comparison {
        moments,
        differences,
        unranked,
    }
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

/// The scalar within an interval already proved determinate by `compare` — #42.
fn scalar_in_force(stated: &[(u32, i64)], at: u32) -> Option<i64> {
    stated
        .iter()
        .filter(|&&(tick, _)| tick <= at)
        .map(|&(_, value)| value)
        .next_back()
}

/// Compare Tempo only across determinate intervals — #42.
fn tempo_differences(
    before: &Take,
    after: &Take,
) -> Result<(Vec<TempoDifference>, Vec<crate::UnrankedComparison<Tempo>>)> {
    let before_stated = before.stated_tempos()?;
    let after_stated = after.stated_tempos()?;
    let comparison = compare(
        &crate::take::tempo_timeline(&before_stated),
        &crate::take::tempo_timeline(&after_stated),
    );
    let scalars = |stated: &[crate::StatedTempo]| -> Vec<(u32, i64)> {
        stated
            .iter()
            .map(|stated| (stated.tick, i64::from(stated.tempo.micros_per_quarter)))
            .collect()
    };
    let before_values = scalars(&before_stated);
    let after_values = scalars(&after_stated);
    let side = |reading: Reading| TempoSide {
        at_start: reading.at_start.map(as_tempo),
        at_end: reading.at_end.map(as_tempo),
        fastest: reading.lowest.map(|(micros, _)| as_tempo(micros)),
        fastest_at: reading.lowest.map(|(_, tick)| tick),
        slowest: reading.highest.map(|(micros, _)| as_tempo(micros)),
        slowest_at: reading.highest.map(|(_, tick)| tick),
    };
    let differences = comparison
        .differences
        .into_iter()
        .map(|(from, until)| TempoDifference {
            from,
            until,
            before: side(reading(&before_values, from, until, &comparison.moments)),
            after: side(reading(&after_values, from, until, &comparison.moments)),
        })
        .collect();
    Ok((
        differences,
        comparison
            .unranked
            .into_iter()
            .map(|interval| interval.map(Tempo::from_micros_per_quarter))
            .collect(),
    ))
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
fn bend_differences(
    before: &Take,
    after: &Take,
) -> Result<(Vec<BendDifference>, Vec<UnrankedBend>)> {
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
    let mut unranked = Vec::new();
    for channel in channels {
        let before_timeline = crate::bend::timeline(&before_stated, channel);
        let after_timeline = crate::bend::timeline(&after_stated, channel);
        let comparison = compare(&before_timeline, &after_timeline);
        // Keep raw within-Tick excursions for determinate intervals, as the
        // existing summaries do. These lists never decide comparability — #42.
        let on = |stated: &[crate::StatedBend]| -> Vec<(u32, i64)> {
            stated
                .iter()
                .filter(|stated| stated.channel == channel)
                .map(|stated| (stated.tick, i64::from(stated.value)))
                .collect()
        };
        let before_values = on(&before_stated);
        let after_values = on(&after_stated);
        for (from, until) in comparison.differences {
            differences.push(BendDifference {
                channel,
                from,
                until,
                before: side(reading(&before_values, from, until, &comparison.moments)),
                after: side(reading(&after_values, from, until, &comparison.moments)),
            });
        }
        unranked.extend(
            comparison
                .unranked
                .into_iter()
                .map(|interval| UnrankedBend {
                    channel,
                    from: interval.from,
                    until: interval.until,
                    before: interval.before,
                    after: interval.after,
                }),
        );
    }
    Ok((differences, unranked))
}

fn state_sites(
    before: &Take,
    after: &Take,
    before_notes: &[Note],
    after_notes: &[Note],
) -> Result<Vec<UnrankedStateSite>> {
    let sites = |take: &Take, notes: &[Note]| -> Result<Vec<crate::Unranked>> {
        let mut strikes: std::collections::BTreeMap<(u32, u8), std::collections::BTreeSet<usize>> =
            std::collections::BTreeMap::new();
        for note in notes {
            strikes
                .entry((note.start, note.channel))
                .or_default()
                .insert(note.track);
        }
        let mut found = Vec::new();
        let mut statements = Vec::new();
        for statement in take.stated_bends()? {
            statements.push((
                statement.tick,
                statement.channel,
                statement.track,
                crate::State::Bend,
                None,
            ));
        }
        for statement in take.stated_programs()? {
            statements.push((
                statement.tick,
                statement.channel,
                statement.track,
                crate::State::Program,
                None,
            ));
        }
        for statement in take.stated_controllers()? {
            statements.push((
                statement.tick,
                statement.channel,
                statement.track,
                crate::State::Controller,
                Some(statement.controller),
            ));
        }
        for (tick, channel, source_track, state, controller) in statements {
            if let Some(tracks) = strikes.get(&(tick, channel)) {
                for &track in tracks {
                    if track != source_track {
                        found.push(crate::Unranked {
                            tick,
                            channel: Some(channel),
                            controller,
                            track: source_track,
                            against_track: track,
                            against: crate::Against::Notes,
                            state,
                        });
                    }
                }
            }
        }
        found.sort_by_key(|site| {
            (
                site.tick,
                site.channel,
                site.state as u8,
                site.controller,
                site.track,
                site.against_track,
            )
        });
        found.dedup();
        Ok(found)
    };
    let before = sites(before, before_notes)?;
    let after = sites(after, after_notes)?;
    let mut union = before.clone();
    for site in &after {
        if !union.contains(site) {
            union.push(site.clone());
        }
    }
    union.sort_by_key(|site| {
        (
            site.tick,
            site.channel,
            site.state as u8,
            site.controller,
            site.track,
            site.against_track,
        )
    });
    Ok(union
        .into_iter()
        .map(|site| UnrankedStateSite {
            in_before: before.contains(&site),
            in_after: after.contains(&site),
            site,
        })
        .collect())
}

/// A bend back from the scalar it was compared as. `stated_bends` is where the
/// number came from, and it is `midly`'s signed reading of fourteen bits.
fn as_bend(value: i64) -> i16 {
    value as i16
}

/// Compare each determinate Program pair, bounded by the next pair or conflict — #42.
fn program_differences(
    before: &Take,
    after: &Take,
) -> Result<(Vec<ProgramDifference>, Vec<UnrankedProgram>)> {
    let before_stated = before.stated_programs()?;
    let after_stated = after.stated_programs()?;
    let mut channels: Vec<_> = before_stated
        .iter()
        .chain(&after_stated)
        .map(|stated| stated.channel)
        .collect();
    channels.sort_unstable();
    channels.dedup();
    let mut differences: Vec<ProgramDifference> = Vec::new();
    let mut unranked = Vec::new();
    for channel in channels {
        let mine = crate::program::timeline(&before_stated, channel);
        let theirs = crate::program::timeline(&after_stated, channel);
        let comparison = compare(&mine, &theirs);
        for (index, &at) in comparison.moments.iter().enumerate() {
            let (Some(before), Some(after)) =
                (mine.at(at).determinate(), theirs.at(at).determinate())
            else {
                continue;
            };
            if before == after {
                continue;
            }
            let until = comparison.moments.get(index + 1).copied();
            if let Some(last) = differences.last_mut().filter(|last| {
                last.channel == channel
                    && last.until == Some(at)
                    && last.before == before
                    && last.after == after
            }) {
                last.until = until;
            } else {
                differences.push(ProgramDifference {
                    channel,
                    at,
                    until,
                    before,
                    after,
                });
            }
        }
        unranked.extend(
            comparison
                .unranked
                .into_iter()
                .map(|interval| UnrankedProgram { channel, interval }),
        );
    }
    Ok((differences, unranked))
}

/// Compare each Controller address only across determinate intervals — #42.
fn controller_differences(
    before: &Take,
    after: &Take,
) -> Result<(Vec<ControllerDifference>, Vec<UnrankedController>)> {
    let before_stated = before.stated_controllers()?;
    let after_stated = after.stated_controllers()?;
    let mut pairs: Vec<_> = before_stated
        .iter()
        .chain(&after_stated)
        .map(|stated| (stated.channel, stated.controller))
        .collect();
    pairs.sort_unstable();
    pairs.dedup();
    let mut differences = Vec::new();
    let mut unranked = Vec::new();
    let side = |reading: Reading| ControllerSide {
        at_start: reading.at_start.map(|value| value as u8),
        at_end: reading.at_end.map(|value| value as u8),
        peak: reading.highest.map(|(value, _)| value as u8),
        peak_at: reading.highest.map(|(_, tick)| tick),
    };
    for (channel, controller) in pairs {
        let comparison = compare(
            &crate::controller::timeline(&before_stated, channel, controller),
            &crate::controller::timeline(&after_stated, channel, controller),
        );
        let scalars = |stated: &[StatedController]| -> Vec<(u32, i64)> {
            stated
                .iter()
                .filter(|stated| stated.channel == channel && stated.controller == controller)
                .map(|stated| (stated.tick, i64::from(stated.value)))
                .collect()
        };
        let before_values = scalars(&before_stated);
        let after_values = scalars(&after_stated);
        for (from, until) in comparison.differences {
            differences.push(ControllerDifference {
                channel,
                controller,
                from,
                until,
                before: side(reading(&before_values, from, until, &comparison.moments)),
                after: side(reading(&after_values, from, until, &comparison.moments)),
            });
        }
        unranked.extend(
            comparison
                .unranked
                .into_iter()
                .map(|interval| UnrankedController {
                    channel,
                    controller,
                    interval,
                }),
        );
    }
    Ok((differences, unranked))
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
