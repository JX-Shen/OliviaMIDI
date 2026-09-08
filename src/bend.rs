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
use crate::reading::{Candidate, Reading, Timeline, UnrankedSpan};
use crate::take::Take;
use midly::{MidiMessage, TrackEventKind};
use serde::Serialize;

/// A channel's starting reading and the coverage of its interval summary — #42.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bend {
    pub channel: u8,
    pub value: Reading<i16>,
    pub extremes: BendExtremes,
    pub unranked: Vec<UnrankedSpan<i16>>,
}

/// A complete interval's extremes, or an explicitly incomplete summary — #42.
/// Incomplete summaries are accompanied by Bend::unranked intervals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BendExtremes {
    Complete {
        furthest_down: i16,
        furthest_down_at: u32,
        furthest_up: i16,
        furthest_up_at: u32,
    },
    Incomplete,
}

pub(crate) fn timeline(stated: &[StatedBend], channel: u8) -> Timeline<i16> {
    Timeline::read(
        stated
            .iter()
            .filter(|statement| statement.channel == channel)
            .map(|statement| Candidate {
                track: statement.track,
                tick: statement.tick,
                value: statement.value,
            }),
    )
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

        // Stable listing order preserves within-track order, not cross-track
        // precedence. The continuing reading is resolved by timeline — #42.
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

        let mut channels: Vec<_> = all
            .iter()
            .filter(|stated| stated.tick < span.end)
            .map(|stated| stated.channel)
            .collect();
        channels.sort_unstable();
        channels.dedup();
        let mut held = Vec::new();
        for channel in channels {
            let timeline = timeline(&all, channel);
            let value = timeline.at(span.start);
            let unranked = timeline.unranked(span.start, bars.map(|_| span.end));
            let extremes = if unranked.is_empty() {
                let mut values = value
                    .determinate()
                    .flatten()
                    .map(|value| (span.start, value))
                    .into_iter()
                    .chain(
                        all.iter()
                            .filter(|stated| {
                                stated.channel == channel
                                    && span.start < stated.tick
                                    && stated.tick < span.end
                            })
                            .map(|stated| (stated.tick, stated.value)),
                    );
                let (tick, first) = values
                    .next()
                    .expect("a listed channel states a bend before the window ends");
                let (mut low, mut low_at, mut high, mut high_at) = (first, tick, first, tick);
                for (tick, value) in values {
                    if value < low {
                        low = value;
                        low_at = tick;
                    }
                    if value > high {
                        high = value;
                        high_at = tick;
                    }
                }
                BendExtremes::Complete {
                    furthest_down: low,
                    furthest_down_at: low_at,
                    furthest_up: high,
                    furthest_up_at: high_at,
                }
            } else {
                BendExtremes::Incomplete
            };
            held.push(Bend {
                channel,
                value,
                extremes,
                unranked,
            });
        }
        let stated = all
            .into_iter()
            .filter(|stated| span.start < stated.tick && stated.tick < span.end)
            .collect();
        Ok(Bends {
            bends: held,
            stated,
        })
    }
}
