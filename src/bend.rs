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

use crate::bars::BarRange;
use crate::error::Result;
use crate::take::Take;
use midly::{MidiMessage, TrackEventKind};
use serde::Serialize;

/// How far one channel of a passage is bent: where the passage begins, and the
/// furthest each way inside it.
///
/// `Controller`'s shape, with one field where that has none. A Controller runs
/// from nought upwards, so one *peak* says everything about where it went; a
/// bend is signed about a centre MIDI fixes, so a phrase that dips below the
/// note and returns never rises above where it began and a single extreme is
/// blind to the whole of it. That is ADR-0007's own argument about what a
/// reading must not hide, applied to a state that needs two numbers to obey it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Bend {
    pub channel: u8,

    /// How far the channel is bent when the passage begins. `None` is a Take
    /// that bent it nowhere before this, and it is not nought: a channel bent
    /// back to the centre and a channel never bent are two different Pieces,
    /// the same distinction `Controller` draws between holding 0 and holding
    /// nothing.
    pub value: Option<i16>,

    /// The lowest and the highest the passage reaches, and where each is first
    /// reached. Seeded from `value` where the Take had already bent the
    /// channel, so a passage that only rises still reports what it rose from.
    pub furthest_down: i16,
    pub furthest_down_at: u32,
    pub furthest_up: i16,
    pub furthest_up_at: u32,
}

/// The bends of a passage: how far each of its channels is bent when it begins,
/// and where the passage bends one.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bends {
    pub bends: Vec<Bend>,
    pub stated: Vec<StatedBend>,
}

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

    /// How far each channel of a passage is bent when it begins, and where the
    /// passage bends one.
    ///
    /// `controllers_in`'s reading, and it answers ADR-0007's question rather
    /// than listing events: a channel bent in Bar 2 is still bent in Bar 6, so
    /// a passage beginning at Bar 6 reports the bend although not one of the
    /// events that set it is inside it. That clause is the whole of why this is
    /// a state and not a listing.
    ///
    /// A channel is reported only where the Take bends it somewhere — before
    /// the passage or inside it. Sixteen channels reporting *unstated* would be
    /// sixteen ways of saying nothing, and most Takes bend nothing at all.
    pub fn bends_in(&self, bars: Option<BarRange>) -> Result<Bends> {
        let span = match bars {
            Some(bars) => self.tick_span(bars)?,
            None => crate::bars::TickSpan {
                start: 0,
                end: u32::MAX,
            },
        };

        let all = self.stated_bends()?;

        let mut held: Vec<Bend> = Vec::new();
        for stated in all.iter().filter(|stated| stated.tick <= span.start) {
            match held.iter_mut().find(|held| held.channel == stated.channel) {
                Some(held) => {
                    held.value = Some(stated.value);
                    held.furthest_down = stated.value;
                    held.furthest_up = stated.value;
                }
                None => held.push(Bend {
                    channel: stated.channel,
                    value: Some(stated.value),
                    furthest_down: stated.value,
                    furthest_down_at: span.start,
                    furthest_up: stated.value,
                    furthest_up_at: span.start,
                }),
            }
        }

        let stated: Vec<StatedBend> = all
            .into_iter()
            .filter(|stated| span.start < stated.tick && stated.tick < span.end)
            .collect();

        // A channel the passage bends and nothing bent before it is listed too,
        // bent nowhere. Leaving it out would print its events under a block that
        // had just said no channel was bent, and a reader would have to decide
        // which half of the output to believe. `controllers_in` does the same
        // for the same reason.
        for stated in &stated {
            if !held.iter().any(|held| held.channel == stated.channel) {
                held.push(Bend {
                    channel: stated.channel,
                    value: None,
                    furthest_down: stated.value,
                    furthest_down_at: stated.tick,
                    furthest_up: stated.value,
                    furthest_up_at: stated.tick,
                });
            }
        }

        // Strictly beyond, so a value reached twice is reported at the first of
        // the two: where the passage *first* goes furthest is the fact, and a
        // later restatement of it changes nothing a listener hears.
        for held in held.iter_mut() {
            for stated in stated
                .iter()
                .filter(|stated| stated.channel == held.channel)
            {
                if stated.value < held.furthest_down {
                    held.furthest_down = stated.value;
                    held.furthest_down_at = stated.tick;
                }
                if stated.value > held.furthest_up {
                    held.furthest_up = stated.value;
                    held.furthest_up_at = stated.tick;
                }
            }
        }

        held.sort_by_key(|held| held.channel);

        Ok(Bends {
            bends: held,
            stated,
        })
    }
}
