//! Where a Take states no order — the reading side of #26.
//!
//! A Rank orders events within one track (ADR-0008). A channel's state written
//! on one track and the notes of that channel on another have no order the file
//! states: a Type-1 player merges the tracks, and the format does not say in
//! which order. `apply` refuses to *write* such a place; this is what says one
//! is already there.
//!
//! Reporting rather than refusing, because the two are different acts. What a
//! Take arrived with is the author's (ADR-0003) and `mid` is the tool somebody
//! opens a file *in order to* diagnose it: a reader that declined to read an
//! imperfect Take would withhold the answer exactly where it is wanted. So
//! `apply` refuses what this project would write, and `inspect` says what the
//! file leaves open. Neither one guesses.
//!
//! Nothing is reported where the two possible readings agree. Two tracks
//! stating one channel the *same* Program at one Tick leave an order that
//! decides nothing — the channel ends up on that Program either way — and a
//! report of a difference that no listener can hear is a report of nothing.

use crate::error::Result;
use crate::take::Take;
use midly::{MidiMessage, TrackEventKind};

/// One place a Take leaves unordered, and what it is unordered against.
///
/// The channel-state event is the near side of every one of these, because it
/// is the thing an Edit Set would have to name to answer for it (`--allow-
/// unranked`), and because a reader looking for what to do next needs the
/// statement rather than one of the notes under it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Unranked {
    pub tick: u32,
    pub channel: u8,
    /// The CC number, or `null` where the event is a program change. The same
    /// spelling `inspect` uses elsewhere to tell one channel state from the
    /// other.
    pub controller: Option<u8>,
    /// The track carrying the channel-state event.
    pub track: usize,
    /// The track carrying what it is unordered against.
    pub against_track: usize,
    pub against: Against,
}

/// What a channel-state event is left unordered against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Against {
    /// Strikes of its own channel. Those notes sound under the state before it
    /// or the state it sets, depending on the player.
    Notes,
    /// Another statement of its own address, holding a different value. The
    /// channel ends up on one value or the other, depending on the player.
    Value,
}

/// One channel-state event, as this reading collects them.
struct Stated {
    track: usize,
    tick: u32,
    channel: u8,
    controller: Option<u8>,
    value: u8,
}

/// One strike, as this reading collects them.
struct Struck {
    track: usize,
    tick: u32,
    channel: u8,
}

impl Take {
    /// Every place this Take states no order between two of its tracks.
    ///
    /// Deduplicated to one row per (statement, other track, sort of clash): a
    /// programme on one track and a chord of eight notes on another is one
    /// thing to know and one thing to do about it, not eight.
    ///
    /// In the order a reader meets them — earliest Tick first, then by the
    /// track stating it — for the reason every other listing this project
    /// prints is in the order the music happens.
    ///
    /// Narrowed to a passage as `programs_in` is, and by the passage's own
    /// Ticks rather than by what is in force at its start: this is about one
    /// Tick where two tracks meet, so a Tick outside the passage is not
    /// something the passage leaves open.
    pub fn unranked(&self, bars: Option<crate::bars::BarRange>) -> Result<Vec<Unranked>> {
        let span = match bars {
            Some(bars) => self.tick_span(bars)?,
            None => crate::bars::TickSpan {
                start: 0,
                end: u32::MAX,
            },
        };
        let smf = self.smf()?;
        let mut stated: Vec<Stated> = Vec::new();
        let mut struck: Vec<Struck> = Vec::new();

        for (track, events) in smf.tracks.iter().enumerate() {
            let mut tick = 0u32;
            for event in events {
                tick += event.delta.as_int();
                let TrackEventKind::Midi { channel, message } = event.kind else {
                    continue;
                };
                let channel = channel.as_int();
                match message {
                    MidiMessage::ProgramChange { program } => stated.push(Stated {
                        track,
                        tick,
                        channel,
                        controller: None,
                        value: program.as_int(),
                    }),
                    MidiMessage::Controller { controller, value } => stated.push(Stated {
                        track,
                        tick,
                        channel,
                        controller: Some(controller.as_int()),
                        value: value.as_int(),
                    }),
                    MidiMessage::NoteOn { vel, .. } if vel.as_int() > 0 => struck.push(Struck {
                        track,
                        tick,
                        channel,
                    }),
                    _ => {}
                }
            }
        }

        let mut found: Vec<Unranked> = Vec::new();
        for state in &stated {
            let mut against = |against_track: usize, against: Against| {
                let row = Unranked {
                    tick: state.tick,
                    channel: state.channel,
                    controller: state.controller,
                    track: state.track,
                    against_track,
                    against,
                };
                if !found.contains(&row) {
                    found.push(row);
                }
            };
            for note in &struck {
                if note.track != state.track
                    && note.tick == state.tick
                    && note.channel == state.channel
                {
                    against(note.track, Against::Notes);
                }
            }
            for other in &stated {
                if other.track != state.track
                    && other.tick == state.tick
                    && other.channel == state.channel
                    && other.controller == state.controller
                    && other.value != state.value
                {
                    against(other.track, Against::Value);
                }
            }
        }

        found.retain(|row| span.start <= row.tick && row.tick < span.end);
        found.sort_by_key(|row| (row.tick, row.track, row.against_track));
        Ok(found)
    }
}
