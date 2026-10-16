//! What `diff` does not compare, and whether two Takes differ in it — #62.
//!
//! Everything in the file is the Piece, and `diff` compares some of it: notes,
//! Programs, Controllers, Tempo, Bend and the Rank of causal pairs. The rest is
//! not compared yet, and a diff that found nothing among what it compared is
//! not entitled to say *no differences* while the rest differs (ADR-0009).
//!
//! `uncompared` is the one place that says, for every event the format can
//! hold, whether `diff` compares it. Its match names every variant and has no
//! wildcard, so an event kind `midly` adds is a compile error here rather than
//! something silently filed on one side. The disclosure is read from it and
//! from `Uncompared`'s variants, so neither is written down anywhere else.

use crate::controller::FIRST_CHANNEL_MODE;
use crate::error::Result;
use crate::note::Note;
use crate::take::Take;
use midly::{MetaMessage, MidiMessage, TrackEventKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// A kind of Piece content `diff` does not compare.
///
/// Declared in the order a diff lists them: what governs the whole Take, then
/// what a channel carries, then what is written beside the music.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Uncompared {
    TimeSignature,
    KeySignature,
    ChannelPressure,
    PolyphonicKeyPressure,
    /// Control changes 120 to 127, which are not Controllers (#13). A Reset All
    /// Controllers is one too: what it does to a Controller the Take had set is
    /// compared, as that Controller's reading, and the event is not (#63).
    ChannelMode,
    /// The velocity a note-off carries. The release itself is compared, as the
    /// end of its note; this byte of it is not.
    ReleaseVelocity,
    #[serde(rename = "sysex")]
    SysEx,
    Lyric,
    /// Every text meta event that is not a lyric or a track name.
    Text,
    TrackName,
    /// Where a track ends, where that is later than the last event in it.
    EndOfTrack,
    /// Every other meta event, and escaped bytes.
    Other,
}

/// One kind of content `diff` did not compare, where the two Takes differ in
/// it, and how much of it is counted.
///
/// Only that they differ, never what differs: saying what would be comparing
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NotCompared {
    pub kind: Uncompared,
    #[serde(flatten)]
    pub count: Counted,
}

/// What a `NotCompared` counts, in its own unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Counted {
    /// How many events of the kind each Take holds.
    Events { before: usize, after: usize },
    /// How many matched notes differ in it: `ReleaseVelocity`, which is read
    /// through the notes rather than as events.
    Notes { notes_differing: usize },
}

/// Which kind of content `diff` does not compare an event is, or `None` where
/// `diff` compares it.
fn uncompared(kind: &TrackEventKind) -> Option<Uncompared> {
    match kind {
        TrackEventKind::Midi { message, .. } => match message {
            // Notes. A release's velocity is the one part of it that is not
            // compared, and it is read from the notes, below.
            MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. } => None,
            MidiMessage::Controller { controller, .. } => {
                (controller.as_int() >= FIRST_CHANNEL_MODE).then_some(Uncompared::ChannelMode)
            }
            MidiMessage::ProgramChange { .. } | MidiMessage::PitchBend { .. } => None,
            MidiMessage::Aftertouch { .. } => Some(Uncompared::PolyphonicKeyPressure),
            MidiMessage::ChannelAftertouch { .. } => Some(Uncompared::ChannelPressure),
        },
        TrackEventKind::SysEx(_) => Some(Uncompared::SysEx),
        TrackEventKind::Escape(_) => Some(Uncompared::Other),
        TrackEventKind::Meta(meta) => match meta {
            MetaMessage::Tempo(_) => None,
            MetaMessage::TimeSignature(..) => Some(Uncompared::TimeSignature),
            MetaMessage::KeySignature(..) => Some(Uncompared::KeySignature),
            MetaMessage::Lyric(_) => Some(Uncompared::Lyric),
            MetaMessage::TrackName(_) => Some(Uncompared::TrackName),
            MetaMessage::Text(_)
            | MetaMessage::Copyright(_)
            | MetaMessage::InstrumentName(_)
            | MetaMessage::Marker(_)
            | MetaMessage::CuePoint(_)
            | MetaMessage::ProgramName(_)
            | MetaMessage::DeviceName(_) => Some(Uncompared::Text),
            MetaMessage::EndOfTrack => Some(Uncompared::EndOfTrack),
            MetaMessage::TrackNumber(_)
            | MetaMessage::MidiChannel(_)
            | MetaMessage::MidiPort(_)
            | MetaMessage::SmpteOffset(_)
            | MetaMessage::SequencerSpecific(_)
            | MetaMessage::Unknown(..) => Some(Uncompared::Other),
        },
    }
}

/// The kinds `diff` does not compare in which the two Takes differ.
///
/// Two Takes differ in a kind where its events differ at the event level —
/// track, Tick and content — regardless of their order within a Tick. A
/// release velocity is read through the note it ends instead, because the
/// release's track and Tick are the note's and are already compared: two
/// notes `diff` matched differ in it where their releases carry different
/// velocities, and a note added or removed is reported as that.
///
/// `matched_to` is `diff`'s matching, each before note's partner by position
/// among the after notes.
pub(crate) fn not_compared(
    before: &Take,
    after: &Take,
    before_notes: &[Note],
    after_notes: &[Note],
    matched_to: &[Option<usize>],
) -> Result<Vec<NotCompared>> {
    let before_smf = before.smf()?;
    let after_smf = after.smf()?;
    let before_events = events(&before_smf);
    let after_events = events(&after_smf);

    let mut found = BTreeMap::new();
    let events_of = |before, after| Counted::Events { before, after };
    for (&kind, mine) in &before_events {
        let theirs = after_events.get(&kind);
        if Some(mine) != theirs {
            found.insert(kind, events_of(count(mine), theirs.map_or(0, count)));
        }
    }
    for (&kind, theirs) in &after_events {
        if !before_events.contains_key(&kind) {
            found.insert(kind, events_of(0, count(theirs)));
        }
    }

    let release = |smf: &midly::Smf, note: &Note| match smf.tracks[note.track][note.off_event].kind
    {
        TrackEventKind::Midi {
            message: MidiMessage::NoteOff { vel, .. },
            ..
        } => Some(vel.as_int()),
        _ => None,
    };
    let notes_differing = before_notes
        .iter()
        .zip(matched_to)
        .filter_map(|(note, partner)| partner.map(|partner| (note, &after_notes[partner])))
        .filter(|(mine, theirs)| release(&before_smf, mine) != release(&after_smf, theirs))
        .count();
    if notes_differing > 0 {
        found.insert(
            Uncompared::ReleaseVelocity,
            Counted::Notes { notes_differing },
        );
    }

    Ok(found
        .into_iter()
        .map(|(kind, count)| NotCompared { kind, count })
        .collect())
}

/// How many of each event, by track and Tick, of each kind not compared.
type Events<'a> = HashMap<(usize, u32, TrackEventKind<'a>), usize>;

///
/// An end of track counts only where it falls after the last event before it.
/// One at that event's own Tick says nothing the event does not: a note moved
/// later takes the end of its track with it, and that is a note that moved, not
/// a track that lasts longer.
fn events<'a>(smf: &midly::Smf<'a>) -> HashMap<Uncompared, Events<'a>> {
    let mut found: HashMap<Uncompared, Events<'a>> = HashMap::new();
    for (track, events) in smf.tracks.iter().enumerate() {
        let mut tick = 0u32;
        for event in events {
            let delta = event.delta.as_int();
            tick += delta;
            if let Some(kind) = uncompared(&event.kind) {
                if kind == Uncompared::EndOfTrack && delta == 0 {
                    continue;
                }
                *found
                    .entry(kind)
                    .or_default()
                    .entry((track, tick, event.kind))
                    .or_insert(0) += 1;
            }
        }
    }
    found
}

fn count(events: &Events) -> usize {
    events.values().sum()
}
