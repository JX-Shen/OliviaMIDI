//! Combine: one new Take from two Alternatives and their common Take (#51, #56).
//!
//! Each Alternative arrives with the Edit Set that made it, and that account is
//! replayed on the common Take before anything is read from it. What each side
//! actually changed becomes a demand on a note of the common Take; where the two
//! demand different things of one note the whole combination refuses, all the
//! conflicts together. Otherwise one coordinated Edit Set is run through the same
//! `apply` path every Take goes through, and the events sharing a Tick are put in
//! the order the sides established (ADR-0008, as amended under #51).
//!
//! The subset is existing notes' velocity, duration and deletion. Nothing here
//! adds an event, so every event of the combined Take is an event of the common
//! Take, and each is followed by the index it arrived at.

use crate::edit::{apply_traced, same_file, Edit, EditSet, Origins};
use crate::error::{Error, Result};
use crate::note::{Note, NoteId};
use crate::rank::{ranked_pair, RankedPairKind};
use crate::take::Take;
use crate::track::with_delta_times;
use midly::{MidiMessage, Smf, TrackEventKind};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// The combined Take, and what the library states about how it was made.
///
/// No `Display`, here or on anything below: `mid` decides the wording
/// (ADR-0005).
#[derive(Debug, Clone)]
pub struct Combined {
    /// The combined Take, not yet written anywhere.
    pub take: Take,
    /// Requests that asked a note for the value it already had in the common
    /// Take. Each contributes no demand.
    pub no_effect: Vec<NoEffect>,
    /// Sites where the two sides wrote a pair the rule does not rank in
    /// opposite orders, and the convention decided (ADR-0004).
    pub disclosed_sites: Vec<DisclosedSite>,
    /// Every End-of-Track the combined Take holds somewhere other than the
    /// common Take did, read from the combined Take itself.
    pub track_ends: Vec<TrackEnd>,
    /// Whether the combined Take is event-equal to the common Take (ADR-0003).
    pub unchanged: bool,
}

/// Which field of a note a request or a demand is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Velocity,
    Duration,
}

/// A request whose field already held the value it asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoEffect {
    pub side: usize,
    pub id: NoteId,
    pub field: Field,
}

/// A (track, Tick, channel) where the convention ordered a pair the two sides
/// wrote in opposite orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct DisclosedSite {
    pub track: usize,
    pub tick: u32,
    pub channel: u8,
}

/// An End-of-Track the construction moved: where it was, and where it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TrackEnd {
    pub track: usize,
    pub before: u32,
    pub after: u32,
}

/// What two sides demand of one note that cannot both be done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Conflict {
    /// Two different values for one field.
    Field {
        id: NoteId,
        field: Field,
        demands: Vec<Demand>,
    },
    /// One side deletes the note and the other changes it.
    DeleteAgainstChange {
        id: NoteId,
        #[serde(rename = "delete")]
        deleted_by: Vec<usize>,
        changes: Vec<FieldChange>,
    },
}

/// One side's value for the field a `Conflict::Field` is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Demand {
    pub side: usize,
    pub value: i64,
}

/// One side's change to a note the other side deletes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FieldChange {
    pub side: usize,
    pub field: Field,
    pub value: i64,
}

/// A pair the rule ranks, which the two sides wrote in opposite orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct OrderContradiction {
    pub track: usize,
    pub tick: u32,
    pub channel: u8,
    pub pair: RankedPairKind,
}

/// An event of the common Take: its track, and its index in that track.
type At = (usize, usize);

/// What one side's verified Edit Set asks of the notes of the common Take,
/// by each note's index in `Take::notes`. Terminal values only: a request
/// overwritten later in the same Edit Set is not a demand (#51).
#[derive(Default)]
struct Account {
    deleted: BTreeSet<usize>,
    velocity: BTreeMap<usize, i64>,
    /// The duration the note ends up with.
    duration: BTreeMap<usize, i64>,
    /// Every note a `resize_note` with a non-zero `delta_ticks` named, net zero
    /// or not: relocation is read from the account, not from the field (#51,
    /// confirmed 2026-09-30; #56, Decision of 2026-10-01).
    relocated: BTreeSet<usize>,
}

/// Combine two Alternatives of `common` into one new Take.
///
/// `sides` is each Alternative with the Edit Set that made it from `common`.
/// The sides are verified in the order given, and each completely before the
/// next is looked at; a refusal names the first that fails.
pub fn combine(common: &Take, sides: &[(Take, EditSet)]) -> Result<Combined> {
    if sides.len() != 2 {
        return Err(Error::CombineNeedsTwoSides(sides.len()));
    }
    let notes = common.notes()?;

    // Verify every side before reading a demand from any of them.
    let mut traces = Vec::with_capacity(sides.len());
    for (side, (take, edit_set)) in sides.iter().enumerate() {
        let (replayed, origins) =
            apply_traced(common, edit_set, &[]).map_err(|source| Error::SourceReplayFailed {
                side,
                take: take.described_path(),
                source: Box::new(source),
            })?;
        if replayed.smf()? != take.smf()? {
            return Err(Error::SourceEvidenceMismatch {
                side,
                take: take.described_path(),
            });
        }
        traces.push(positions(&replayed, &origins)?);
    }

    let index_of: HashMap<&NoteId, usize> = notes
        .iter()
        .enumerate()
        .map(|(index, note)| (&note.id, index))
        .collect();
    let mut accounts = Vec::with_capacity(sides.len());
    for (side, (take, edit_set)) in sides.iter().enumerate() {
        accounts.push(account(side, take, edit_set, &notes, &index_of)?);
    }

    // Actual changes only; agreement counted once; nothing settled by
    // arithmetic or by picking a side.
    let mut no_effect = Vec::new();
    let mut conflicts = Vec::new();
    let mut coordinated = Vec::new();
    for (index, note) in notes.iter().enumerate() {
        let deleted_by: Vec<usize> = (0..accounts.len())
            .filter(|&side| accounts[side].deleted.contains(&index))
            .collect();
        let mut demanded: BTreeMap<Field, Vec<Demand>> = BTreeMap::new();
        for (side, account) in accounts.iter().enumerate() {
            for (field, asked, had) in [
                (
                    Field::Velocity,
                    account.velocity.get(&index),
                    i64::from(note.velocity),
                ),
                (
                    Field::Duration,
                    account.duration.get(&index),
                    i64::from(note.duration),
                ),
            ] {
                let Some(&value) = asked else { continue };
                if value == had {
                    no_effect.push(NoEffect {
                        side,
                        id: note.id.clone(),
                        field,
                    });
                } else {
                    demanded
                        .entry(field)
                        .or_default()
                        .push(Demand { side, value });
                }
            }
        }

        if !deleted_by.is_empty() {
            if demanded.is_empty() {
                coordinated.push(Edit::DeleteNote {
                    id: note.id.clone(),
                });
            } else {
                conflicts.push(Conflict::DeleteAgainstChange {
                    id: note.id.clone(),
                    deleted_by,
                    changes: demanded
                        .iter()
                        .flat_map(|(&field, demands)| {
                            demands.iter().map(move |demand| FieldChange {
                                side: demand.side,
                                field,
                                value: demand.value,
                            })
                        })
                        .collect(),
                });
            }
            continue;
        }
        for (field, demands) in demanded {
            let value = demands[0].value;
            if demands.iter().any(|demand| demand.value != value) {
                conflicts.push(Conflict::Field {
                    id: note.id.clone(),
                    field,
                    demands,
                });
                continue;
            }
            coordinated.push(match field {
                Field::Velocity => Edit::SetVelocity {
                    id: note.id.clone(),
                    velocity: value,
                },
                Field::Duration => Edit::ResizeNote {
                    id: note.id.clone(),
                    delta_ticks: value - i64::from(note.duration),
                },
            });
        }
    }
    no_effect.sort_by_key(|entry| (index_of[&entry.id], entry.field, entry.side));
    if !conflicts.is_empty() {
        return Err(Error::Conflicts(conflicts));
    }

    // One coordinated Edit Set, through the path every Take goes through.
    let (built, origins) = apply_traced(common, &EditSet { edits: coordinated }, &[])
        .map_err(|source| Error::CoreInvalid(Box::new(source)))?;

    let common_smf = common.smf()?;
    let release_of: HashMap<usize, At> = notes
        .iter()
        .enumerate()
        .map(|(index, note)| (index, (note.track, note.off_event)))
        .collect();
    let rules = Rules {
        common: &common_smf,
        relocated: accounts
            .iter()
            .map(|account| {
                account
                    .relocated
                    .iter()
                    .filter(|index| !account.deleted.contains(index))
                    .map(|index| release_of[index])
                    .collect()
            })
            .collect(),
        sides: traces,
        strike_of: notes
            .iter()
            .map(|note| ((note.track, note.off_event), note.on_event))
            .collect(),
    };

    let built_smf = built.smf()?;
    let mut disclosed = BTreeSet::new();
    let mut contradictions = Vec::new();
    let mut tracks = Vec::with_capacity(built_smf.tracks.len());
    for (track, (events, origin)) in built_smf.tracks.iter().zip(&origins).enumerate() {
        let mut placed: Vec<(u32, TrackEventKind, Option<usize>)> = Vec::new();
        let mut tick = 0u32;
        for (event, &came_from) in events.iter().zip(origin) {
            tick += event.delta.as_int();
            placed.push((tick, event.kind, came_from));
        }
        // End-of-Track stays where `apply` put it, last.
        let end = placed.pop();

        let mut ordered = Vec::with_capacity(placed.len());
        for group in placed.chunk_by(|a, b| a.0 == b.0) {
            let members: Option<Vec<usize>> = group.iter().map(|event| event.2).collect();
            let Some(members) = members.filter(|members| members.len() > 1) else {
                ordered.extend_from_slice(group);
                continue;
            };
            let tick = group[0].0;
            let at = rules.order(track, tick, &members)?;
            disclosed.extend(at.disclosed);
            contradictions.extend(at.contradictions);
            let by_origin: HashMap<usize, _> = group
                .iter()
                .map(|event| (event.2.unwrap(), *event))
                .collect();
            ordered.extend(at.order.iter().map(|index| by_origin[index]));
        }
        ordered.extend(end);
        tracks.push(with_delta_times(
            ordered.into_iter().map(|(tick, kind, _)| (tick, kind)),
        )?);
    }
    if !contradictions.is_empty() {
        return Err(Error::OrderContradiction(contradictions));
    }

    let take = Take::from_smf(&Smf {
        header: built_smf.header,
        tracks,
    })?;
    keeps_its_notes(&take, &built)?;

    let combined_smf = take.smf()?;
    let track_ends = common_smf
        .tracks
        .iter()
        .zip(&combined_smf.tracks)
        .enumerate()
        .filter_map(|(track, (before, after))| {
            let (before, after) = (end_of(before), end_of(after));
            (before != after).then_some(TrackEnd {
                track,
                before,
                after,
            })
        })
        .collect();
    let unchanged = combined_smf == common_smf;
    drop(combined_smf);

    Ok(Combined {
        take,
        no_effect,
        disclosed_sites: disclosed.into_iter().collect(),
        track_ends,
        unchanged,
    })
}

/// The whole of `mid combine`: read the common Take and both sides, combine
/// them, and write the result somewhere none of the inputs is.
///
/// Nothing is written unless the combination succeeds, and the output is
/// refused before anything is read if it names any input — the same check, and
/// the same structure behind it, as `apply_to_new_take`.
pub fn combine_to_new_take(
    common: &Path,
    sides: &[(PathBuf, PathBuf)],
    output: &Path,
) -> Result<Combined> {
    let inputs = std::iter::once(common).chain(
        sides
            .iter()
            .flat_map(|(take, edits)| [take.as_path(), edits]),
    );
    for input in inputs {
        if same_file(input, output) {
            return Err(Error::CombineOverInput(output.to_path_buf()));
        }
    }
    let common = Take::read(common)?;
    let sides = sides
        .iter()
        .map(|(take, edits)| Ok((Take::read(take)?, EditSet::read(edits)?)))
        .collect::<Result<Vec<_>>>()?;
    let combined = combine(&common, &sides)?;
    combined.take.write(output)?;
    Ok(combined)
}

/// What one verified Edit Set asks of the common Take's notes.
fn account(
    side: usize,
    take: &Take,
    edit_set: &EditSet,
    notes: &[Note],
    index_of: &HashMap<&NoteId, usize>,
) -> Result<Account> {
    let mut account = Account::default();
    for (edit, asked) in edit_set.edits.iter().enumerate() {
        // The Edit Set has just replayed, so every identity in it resolves.
        let note = |id: &NoteId| index_of[id];
        match asked {
            Edit::SetVelocity { id, velocity } => {
                account.velocity.insert(note(id), *velocity);
            }
            Edit::ResizeNote { id, delta_ticks } => {
                // A step that moves nothing leaves the event stream as it was:
                // no relocation, and so no order claim (#56, Decision of
                // 2026-10-01). Its duration request reports no effect below.
                if *delta_ticks != 0 {
                    account.relocated.insert(note(id));
                }
                let had = i64::from(notes[note(id)].duration);
                *account.duration.entry(note(id)).or_insert(had) += delta_ticks;
            }
            Edit::DeleteNote { id } => {
                account.deleted.insert(note(id));
            }
            _ => {
                return Err(Error::EditOutsideCombination {
                    side,
                    take: take.described_path(),
                    edit,
                })
            }
        }
    }
    // A deleted note's earlier requests are steps, not where it ends up.
    for deleted in &account.deleted {
        account.velocity.remove(deleted);
        account.duration.remove(deleted);
    }
    Ok(account)
}

/// Where each event of the common Take sits in a Take `apply` wrote from it:
/// its Tick, and its position in its track.
fn positions(take: &Take, origins: &Origins) -> Result<HashMap<At, (u32, usize)>> {
    let mut found = HashMap::new();
    for (track, (events, origin)) in take.smf()?.tracks.iter().zip(origins).enumerate() {
        let mut tick = 0u32;
        for (position, (event, came_from)) in events.iter().zip(origin).enumerate() {
            tick += event.delta.as_int();
            if let Some(index) = came_from {
                found.insert((track, *index), (tick, position));
            }
        }
    }
    Ok(found)
}

/// The Tick a track ends at: its End-of-Track, or its last event where it
/// states none.
fn end_of(track: &[midly::TrackEvent]) -> u32 {
    track.iter().map(|event| event.delta.as_int()).sum()
}

/// What decides the order of the events sharing one Tick of the combined Take.
struct Rules<'a, 'b> {
    common: &'a Smf<'b>,
    /// Per side, the releases its account relocated, as events of the common
    /// Take.
    relocated: Vec<HashSet<At>>,
    /// Per side, where each event of the common Take sits in it.
    sides: Vec<HashMap<At, (u32, usize)>>,
    /// Each release of the common Take, and the index of the strike it ends.
    strike_of: HashMap<At, usize>,
}

/// The order found for one Tick, and what was found getting there.
struct Ordered {
    order: Vec<usize>,
    disclosed: Vec<DisclosedSite>,
    contradictions: Vec<OrderContradiction>,
}

impl Rules<'_, '_> {
    /// Order the events at one (track, Tick), given as indices into the
    /// common Take's track.
    ///
    /// A side that relocated one event of a pair and holds both at this Tick
    /// establishes their order; where no side relocated either, the common
    /// Take's order stands; where nobody establishes it, the convention does —
    /// ascending position in the common Take of the strike that owns each
    /// event, so that two releases of one pitch can never end their notes in
    /// the other order. Two sides establishing opposite orders refuse only
    /// where `ranked_pair` says the rule ranks the pair; otherwise the
    /// convention decides and the site is disclosed. #51.
    fn order(&self, track: usize, tick: u32, members: &[usize]) -> Result<Ordered> {
        let mut disclosed = Vec::new();
        let mut contradictions = Vec::new();
        let mut before: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for (i, &x) in members.iter().enumerate() {
            for &y in &members[i + 1..] {
                let touched: Vec<usize> = (0..self.sides.len())
                    .filter(|&side| {
                        self.relocated[side].contains(&(track, x))
                            || self.relocated[side].contains(&(track, y))
                    })
                    .collect();
                if touched.is_empty() {
                    let (first, second) = if x < y { (x, y) } else { (y, x) };
                    before.entry(first).or_default().insert(second);
                    continue;
                }
                let established: BTreeSet<(usize, usize)> = touched
                    .iter()
                    .filter_map(|&side| {
                        let held = |index| {
                            self.sides[side]
                                .get(&(track, index))
                                .filter(|(at, _)| *at == tick)
                                .map(|&(_, position)| position)
                        };
                        let (px, py) = (held(x)?, held(y)?);
                        Some(if px < py { (x, y) } else { (y, x) })
                    })
                    .collect();
                match established.len() {
                    0 => {}
                    1 => {
                        let (first, second) = *established.first().expect("one order");
                        before.entry(first).or_default().insert(second);
                    }
                    _ => {
                        let (x_event, y_event) = (self.event(track, x), self.event(track, y));
                        match x_event.zip(y_event).and_then(|(a, b)| ranked_pair(a, b)) {
                            Some(pair) => contradictions.push(OrderContradiction {
                                track,
                                tick,
                                channel: x_event.map_or(0, |(channel, _)| channel),
                                pair,
                            }),
                            None => {
                                for (channel, _) in [x_event, y_event].into_iter().flatten() {
                                    disclosed.push(DisclosedSite {
                                        track,
                                        tick,
                                        channel,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Kahn's algorithm, with the convention choosing among the events
        // nothing still holds back.
        let convention = |index: usize| {
            (
                self.strike_of
                    .get(&(track, index))
                    .copied()
                    .unwrap_or(index),
                index,
            )
        };
        let mut waiting: HashMap<usize, usize> = members.iter().map(|&m| (m, 0)).collect();
        for followers in before.values() {
            for follower in followers {
                *waiting.get_mut(follower).expect("a member") += 1;
            }
        }
        let mut free: BTreeSet<(usize, usize)> = members
            .iter()
            .filter(|member| waiting[member] == 0)
            .map(|&member| convention(member))
            .collect();
        let mut order = Vec::with_capacity(members.len());
        while let Some((_, next)) = free.pop_first() {
            order.push(next);
            for follower in before.get(&next).into_iter().flatten() {
                let count = waiting.get_mut(follower).expect("a member");
                *count -= 1;
                if *count == 0 {
                    free.insert(convention(*follower));
                }
            }
        }
        if order.len() != members.len() {
            return Err(Error::CombinedRanksCircular { track, tick });
        }
        Ok(Ordered {
            order,
            disclosed,
            contradictions,
        })
    }

    /// An event of the common Take as `ranked_pair` reads it: its channel and
    /// its message, or `None` for an event that is on no channel.
    fn event(&self, track: usize, index: usize) -> Option<(u8, &MidiMessage)> {
        match &self.common.tracks[track][index].kind {
            TrackEventKind::Midi { channel, message } => Some((channel.as_int(), message)),
            _ => None,
        }
    }
}

/// Refuse a combined Take whose notes do not read back as the coordinated
/// Take's: re-ordering within a Tick may change what a synthesiser meets first,
/// and must never change what a reader pairs.
///
/// Compared as a reader sees them — identity, length and velocity — and not by
/// which written release ended which strike. Two identical releases of one
/// pitch at one Tick hand back the same lengths whichever is read first, which
/// is the reason `apply`'s own `stay_distinct` gives for asking only a release's
/// Tick; this asks the same question of the finished Take.
///
/// It cannot fire if the ordering above is right, which is the point: an
/// argument is a thing a later change breaks silently.
fn keeps_its_notes(take: &Take, coordinated: &Take) -> Result<()> {
    let read = |take: &Take| -> Result<Vec<(NoteId, u32, u8)>> {
        let mut notes: Vec<_> = take
            .notes()?
            .into_iter()
            .map(|note| (note.id, note.duration, note.velocity))
            .collect();
        notes.sort();
        Ok(notes)
    };
    let (found, wanted) = (read(take)?, read(coordinated)?);
    if found != wanted {
        let lost = wanted
            .iter()
            .find(|note| !found.contains(note))
            .or_else(|| found.iter().find(|note| !wanted.contains(note)))
            .map_or_else(String::new, |(id, ..)| id.to_string());
        return Err(Error::NoteEventsLost(lost));
    }
    Ok(())
}
