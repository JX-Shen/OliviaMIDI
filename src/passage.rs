//! Cutting a passage out of a Take: the Bars a `--bars` range names, and the
//! state the Take had already set by the time they began.
//!
//! `crate::bars` decides which Ticks a Bar range covers. This is the surgery
//! that turns that span into a Take of its own, which `mid play --bars` needs
//! because FluidSynth plays a file from its beginning and has no range
//! playback. What travels and what is left behind is decided in #4.

use crate::bars::{BarRange, TickSpan};
use crate::error::{Error, Result};
use crate::rank::{role, RankedPairKind, Role};
use crate::take::Take;
use crate::track::with_delta_times;
use crate::unranked::State;
use midly::{MetaMessage, MidiMessage, TrackEvent, TrackEventKind};
use std::collections::{BTreeMap, HashSet};

impl Take {
    /// The Take restricted to a Bar range: that passage, and nothing else.
    ///
    /// The passage begins at Tick 0. One left at the Ticks it was found at
    /// would open with as many Bars of silence as precede it, which is the
    /// whole of what asking for a passage is trying to avoid.
    ///
    /// It carries the notes that start inside the range, whole; whatever else
    /// happens inside it; and the state the Take had already set by the time
    /// the range began. It leaves behind what belongs to a moment the passage
    /// does not contain. See #4.
    pub fn passage(&self, bars: BarRange) -> Result<Take> {
        let span = self.tick_span(bars)?;
        let mut smf = self.smf()?;

        // Both events of a note travel or neither does. A note-off left behind
        // by a note that started before the passage would release a note that
        // nothing in the passage ever struck.
        let mut kept: Vec<HashSet<usize>> = vec![HashSet::new(); smf.tracks.len()];
        for note in self.notes()? {
            if span.start <= note.start && note.start < span.end {
                kept[note.track].insert(note.on_event);
                kept[note.track].insert(note.off_event);
            }
        }

        // What each track puts at the passage's first Tick, gathered across all
        // of them, because the question it answers is a cross-track one.
        let mut landings: Vec<Landing> = Vec::new();
        for (index, track) in smf.tracks.iter_mut().enumerate() {
            *track = restricted(track, span, &kept[index], index, &mut landings)?;
        }
        keeps_the_orders_the_take_stated(&landings)?;
        Take::from_smf(&smf)
    }
}

/// One event the passage puts at its first Tick, and the Tick it held before.
///
/// Carries both the ranked-pair role and the state address/value. Passage
/// safety also checks relations outside the pairs compared by diff. See #34.
struct Landing {
    track: usize,
    channel: Option<u8>,
    role: Option<Role>,
    /// State kind, Controller number where applicable, and value in its own units.
    state: Option<(State, Option<u8>, i32)>,
    /// The Tick this event held in the Take the passage was cut from.
    from: u32,
}

/// Refuse a passage that would state no order where the Take stated one.
///
/// Inheriting is a collapse: everything the Take had already set arrives at the
/// passage's first Tick, from however many Ticks away. Within one track that
/// costs nothing — a Rank still orders them, and it orders them as they were.
/// Across two tracks it costs the order itself. A Program stated on one track at
/// Tick 0 and a chord struck on another at Tick 960 are ordered by *time* in the
/// Take, unambiguously and without any rule being needed; put both at the
/// passage's first Tick and a Rank is the only thing that could order them, and
/// a Rank does not run between tracks (ADR-0008). The passage would sound one
/// way or the other depending on the player.
///
/// So this is not the reading `Take::unranked` does. That one reports a site the
/// *author* left open, which is theirs to leave (ADR-0003) and is reported
/// rather than refused. This is a site `mid` would be creating, out of one that
/// was closed, while preparing an audition nobody asked to be approximate — and
/// `CHARTER.md` refuses rather than answering plausibly.
///
/// A pair that shared a Tick in the Take already is left alone: it arrived
/// unranked and the passage has taken nothing away.
fn keeps_the_orders_the_take_stated(landings: &[Landing]) -> Result<()> {
    let mut by_channel: BTreeMap<Option<u8>, BTreeMap<usize, Vec<&Landing>>> = BTreeMap::new();
    for landing in landings {
        by_channel
            .entry(landing.channel)
            .or_default()
            .entry(landing.track)
            .or_default()
            .push(landing);
    }
    for state in landings {
        let others = by_channel[&state.channel]
            .iter()
            .filter(|(track, _)| **track != state.track)
            .flat_map(|(_, events)| events);
        for other in others {
            if state.from == other.from {
                continue;
            }
            if let (Some(left), Some(right)) = (state.role, other.role) {
                if left.governing && !right.governing && left.pair == right.pair {
                    return Err(Error::PassageWouldLoseAnOrder {
                        stating: state.track,
                        sounding: other.track,
                        from: state.from,
                        at: other.from,
                        channel: state.channel.expect("a ranked channel event"),
                        state: left.pair.governing_named(),
                        against: left.pair.governed_named(),
                    });
                }
            }
            let Some((kind, controller, value)) = state.state else {
                continue;
            };
            let name = match kind {
                State::Program => "program change",
                State::Controller => "controller",
                State::Bend => "bend",
                State::Tempo => "tempo",
            };
            if let Some((other_kind, other_controller, other_value)) = other.state {
                if kind == other_kind && controller == other_controller && value != other_value {
                    let mut address = name.to_string();
                    if let Some(controller) = controller {
                        address.push_str(&format!(" CC{controller}"));
                    }
                    if let Some(channel) = state.channel {
                        address.push_str(&format!(" on channel {channel}"));
                    }
                    return Err(Error::PassageWouldLoseStateOrder {
                        state: address,
                        first_track: state.track,
                        first_tick: state.from,
                        first_value: value,
                        second_track: other.track,
                        second_tick: other.from,
                        second_value: other_value,
                    });
                }
            }
            if other.role.is_some_and(|role| {
                !role.governing && role.pair == RankedPairKind::ProgramBeforeStrike
            }) {
                return Err(Error::PassageWouldLoseAnOrder {
                    stating: state.track,
                    sounding: other.track,
                    from: state.from,
                    at: other.from,
                    channel: state.channel.expect("a state of the struck channel"),
                    state: name,
                    against: "strikes",
                });
            }
        }
    }
    Ok(())
}

/// One track of a Take, restricted to a passage and moved to Tick 0.
///
/// Three kinds of event survive: the notes of the passage, everything else that
/// happens inside it, and the state the Take had already set when it began,
/// gathered at Tick 0 in the order it was set. The Take's own end-of-track is
/// not one of them — the passage ends where the passage ends.
///
/// Leaving events behind merges the gaps around them, so a passage can need a
/// longer delta time than any the Take it came from held. That is why this
/// refuses rather than returning: the encoding rule is `track`'s
/// `with_delta_times`, and it is the same rule `apply` writes through.
fn restricted<'a>(
    track: &[TrackEvent<'a>],
    span: TickSpan,
    kept_notes: &HashSet<usize>,
    index_of_track: usize,
    landings: &mut Vec<Landing>,
) -> Result<Vec<TrackEvent<'a>>> {
    let mut inherited: Vec<TrackEventKind<'a>> = Vec::new();
    let mut inside: Vec<(u32, TrackEventKind<'a>)> = Vec::new();
    let mut tick = 0u32;

    // Everything the passage puts at its own first Tick, whether it was carried
    // there by inheritance or was already at the Tick the passage begins on.
    let mut lands = |at: u32, kind: &TrackEventKind<'a>, from: u32| {
        if at != 0 {
            return;
        }
        let (channel, role, state) = match kind {
            TrackEventKind::Meta(MetaMessage::Tempo(value)) => (
                None,
                None,
                Some((State::Tempo, None, value.as_int() as i32)),
            ),
            TrackEventKind::Midi { channel, message } => {
                let state = match message {
                    MidiMessage::ProgramChange { program } => {
                        Some((State::Program, None, i32::from(program.as_int())))
                    }
                    MidiMessage::Controller { controller, value } if controller.as_int() < 120 => {
                        Some((
                            State::Controller,
                            Some(controller.as_int()),
                            i32::from(value.as_int()),
                        ))
                    }
                    MidiMessage::PitchBend { bend } => {
                        Some((State::Bend, None, i32::from(bend.as_int())))
                    }
                    _ => None,
                };
                (Some(channel.as_int()), role(message), state)
            }
            _ => return,
        };
        if role.is_some() || state.is_some() {
            landings.push(Landing {
                track: index_of_track,
                channel,
                role,
                state,
                from,
            });
        }
    };

    for (index, event) in track.iter().enumerate() {
        tick += event.delta.as_int();
        if is_a_note(&event.kind) {
            // Whether this note is in the passage was settled by where it
            // starts. One that runs past the last Bar line keeps its note-off
            // wherever that falls: it was struck in this passage, and
            // shortening it here would make `play` disagree with the duration
            // `inspect` reports for the same note.
            if kept_notes.contains(&index) {
                let at = tick.saturating_sub(span.start);
                lands(at, &event.kind, tick);
                inside.push((at, event.kind));
            }
        } else if matches!(event.kind, TrackEventKind::Meta(MetaMessage::EndOfTrack)) {
            continue;
        } else if tick < span.start {
            if outlives_its_moment(&event.kind) {
                lands(0, &event.kind, tick);
                inherited.push(event.kind);
            }
        } else if tick < span.end {
            lands(tick - span.start, &event.kind, tick);
            inside.push((tick - span.start, event.kind));
        }
    }

    let mut events: Vec<(u32, TrackEventKind<'a>)> =
        inherited.into_iter().map(|kind| (0, kind)).collect();
    events.extend(inside);

    // The passage is as long as the Bars it names, even when nothing sounds in
    // the last of them: a Bar of silence at the end is part of the passage.
    let end = events
        .iter()
        .map(|&(tick, _)| tick)
        .max()
        .unwrap_or(0)
        .max(span.end - span.start);
    events.push((end, TrackEventKind::Meta(MetaMessage::EndOfTrack)));

    with_delta_times(events)
}

fn is_a_note(kind: &TrackEventKind) -> bool {
    matches!(
        kind,
        TrackEventKind::Midi {
            message: MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. },
            ..
        }
    )
}

/// Whether an event sets something that is still true after it — the state a
/// passage starting part way through the Take has to inherit to sound like
/// itself.
///
/// Tempo and time signature are what the passage is measured in. The rest
/// follow from the same rule and the boundary in `CHARTER.md`: a program
/// change, a controller, a pitch bend and a SysEx setup message are all in the
/// file, so they are the Piece, and a passage heard without them is heard as
/// something the Take does not say.
///
/// Left behind is everything belonging to a moment the passage does not
/// contain: notes, a note's aftertouch, an SMPTE offset for a start the passage
/// is not, sequencer-specific data no device ever hears, and the text events
/// that name a place — a marker reading "Chorus" is about the Bar it sits in,
/// not about every Bar after it. `Escape` is left behind too: it is raw bytes
/// escaping the format's own framing, so nothing can be said about what it
/// sets, and a guess about undefined bytes is a guess played into someone's
/// ears. Whatever the format grows next is left behind for the same reason.
fn outlives_its_moment(kind: &TrackEventKind) -> bool {
    match kind {
        TrackEventKind::Midi { message, .. } => matches!(
            message,
            MidiMessage::ProgramChange { .. }
                | MidiMessage::Controller { .. }
                | MidiMessage::PitchBend { .. }
                | MidiMessage::ChannelAftertouch { .. }
        ),
        TrackEventKind::SysEx(_) => true,
        TrackEventKind::Escape(_) => false,
        TrackEventKind::Meta(meta) => matches!(
            meta,
            MetaMessage::Tempo(_)
                | MetaMessage::TimeSignature(..)
                | MetaMessage::KeySignature(..)
                // What the track calls itself, which it goes on being called.
                | MetaMessage::TrackNumber(_)
                | MetaMessage::TrackName(_)
                | MetaMessage::InstrumentName(_)
                | MetaMessage::ProgramName(_)
                | MetaMessage::DeviceName(_)
                | MetaMessage::MidiChannel(_)
                | MetaMessage::MidiPort(_)
        ),
    }
}
