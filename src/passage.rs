//! Cutting a passage out of a Take: the Bars a `--bars` range names, and the
//! state the Take had already set by the time they began.
//!
//! `crate::bars` decides which Ticks a Bar range covers. This is the surgery
//! that turns that span into a Take of its own, which `mid play --bars` needs
//! because FluidSynth plays a file from its beginning and has no range
//! playback. What travels and what is left behind is decided in #4.

use crate::bars::{BarRange, TickSpan};
use crate::error::{Error, Result};
use crate::rank::{role, Role};
use crate::take::Take;
use crate::track::with_delta_times;
use midly::{MetaMessage, MidiMessage, TrackEvent, TrackEventKind};
use std::collections::HashSet;

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
/// Only the events that take part in an ordering claim — ADR-0008's ranked
/// pairs — because they are the only ones whose order says anything about the
/// music. Two notes arriving at Tick 0 from two Ticks is what a passage *is*.
struct Landing {
    track: usize,
    channel: u8,
    role: Role,
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
    for state in landings.iter().filter(|landing| landing.role.governing) {
        for note in landings.iter().filter(|landing| !landing.role.governing) {
            if state.role.pair != note.role.pair
                || state.channel != note.channel
                || state.track == note.track
                || state.from == note.from
            {
                continue;
            }
            return Err(Error::PassageWouldLoseAnOrder {
                stating: state.track,
                sounding: note.track,
                from: state.from,
                at: note.from,
                channel: state.channel,
                state: state.role.pair.governing_named(),
                against: state.role.pair.governed_named(),
            });
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
        let TrackEventKind::Midi { channel, message } = kind else {
            return;
        };
        if let Some(role) = role(message) {
            landings.push(Landing {
                track: index_of_track,
                channel: channel.as_int(),
                role,
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
