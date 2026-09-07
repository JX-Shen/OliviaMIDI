//! Where a channel's pitch is bent, and where the Take says so.
//!
//! The shape is `controller`'s, because ADR-0007 is one rule over every channel
//! state: what the channel holds is the answer, and the events that put it there
//! are where it came from. That record names pitch bend as the next state to
//! join on those terms, and this is it joining.
//!
//! A bend is not a Controller. It has no controller number, so nothing
//! sub-addresses it, and its value is fourteen bits rather than seven. What it
//! shares is the thing ADR-0007 cares about: a value that holds until something
//! says otherwise, so that a bend set before a passage is in force at its first
//! note whether or not the passage contains the event that set it.

use crate::error::Result;
use crate::take::Take;
use midly::{MidiMessage, TrackEventKind};
use serde::Serialize;

/// One place a Take bends a channel: which track says it, on which channel, at
/// which Tick, and by how much.
///
/// Not `PitchBend`, although that is the MIDI event's own name, for the reason
/// `StatedController` is not `ControlChange`: two types under one name in one
/// crate would read as one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StatedBend {
    pub track: usize,
    pub channel: u8,
    pub tick: u32,

    /// How far the pitch is bent, as MIDI's own signed reading of the event:
    /// nought is no bend, negative is down, positive is up, and the range is
    /// -8192 to 8191.
    ///
    /// Signed rather than the raw fourteen bits, because the centre is not a
    /// fact about this file that a reading could get wrong — MIDI fixes it at
    /// 0x2000 and `midly` is what converts. The raw number would make *no bend*
    /// read as 8192, and every question anybody asks of a bend is a question
    /// about a distance from the centre: which way, and how far.
    ///
    /// How many semitones a given distance is worth is not here, because the
    /// file does not say. It is the synthesiser's bend range — a Rig fact by the
    /// boundary in `AGENTS.md`, and two semitones only by convention.
    pub value: i16,
}

impl Take {
    /// Every place the Take bends a channel, earliest Tick first, ties broken by
    /// track order.
    ///
    /// Read from wherever they are rather than from the notes' track, for the
    /// reason `stated_controllers` is: nothing obliges an export to put a
    /// channel's bends on the track whose notes play through it.
    pub fn stated_bends(&self) -> Result<Vec<StatedBend>> {
        let smf = self.smf()?;
        let mut found = Vec::new();

        for (track, events) in smf.tracks.iter().enumerate() {
            let mut tick = 0u32;
            for event in events {
                tick += event.delta.as_int();
                if let TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::PitchBend { bend },
                } = event.kind
                {
                    found.push(StatedBend {
                        track,
                        channel: channel.as_int(),
                        tick,
                        value: bend.as_int(),
                    });
                }
            }
        }

        // Stable, so events sharing a Tick keep track order among themselves —
        // and where two tracks bend one channel at one Tick, the later track is
        // the one in force, as it is for the synthesiser. Within one track it
        // keeps file order, so the last bend at a Tick is still the last one
        // after this.
        found.sort_by_key(|found| found.tick);
        Ok(found)
    }
}
