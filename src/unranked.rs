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
use midly::{MetaMessage, MidiMessage, TrackEventKind};

/// One place a Take leaves unordered, and what it is unordered against.
///
/// The channel-state event is the near side of every one of these, because it
/// is the thing an Edit Set would have to name to answer for it (`--allow-
/// unranked`), and because a reader looking for what to do next needs the
/// statement rather than one of the notes under it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Unranked {
    pub tick: u32,
    /// The channel whose state this is, or `null` where the state has no
    /// channel. A tempo governs the whole Take and is written on a conductor
    /// track, so it has none — the same spelling `controller` uses for a state
    /// that has no CC number. See #42.
    pub channel: Option<u8>,
    /// The CC number, or `null` where the state has no CC number. That is a
    /// program change, a bend or a tempo — which `state` is what tells them
    /// apart. Until #42 there were two states and `null` happened to mean
    /// "program change"; that was a coincidence of there being two, never what
    /// the field said.
    pub controller: Option<u8>,
    /// The track carrying the channel-state event.
    pub track: usize,
    /// The track carrying what it is unordered against.
    pub against_track: usize,
    pub against: Against,
    /// Which state this is. Appended, and nothing above it moved: a consumer
    /// reading the rows #26 shipped finds every field where it left it. See
    /// #42.
    pub state: State,
}

/// Which of a Take's states an unranked site is about.
///
/// The values are the glossary's own terms. The type ratifies prose this
/// repository was already writing — `error.rs` calls this a `state` and so does
/// `wording::unranked` — rather than coining a word, which is the argument
/// **Rank** makes about itself in `CONTEXT.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Program,
    Controller,
    Bend,
    Tempo,
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
    state: State,
    channel: Option<u8>,
    controller: Option<u8>,
    /// Wide enough for every state's own units: a program or a CC value is
    /// 0-127, a bend is -8192 to 8191, and a tempo is microseconds to the
    /// quarter. Only ever compared for equality, never read as a quantity.
    value: i32,
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
                // A tempo before the channel states, because it is the one
                // that is not a channel event at all: it rides on a meta event
                // and governs the whole Take.
                if let TrackEventKind::Meta(MetaMessage::Tempo(micros)) = event.kind {
                    stated.push(Stated {
                        track,
                        tick,
                        state: State::Tempo,
                        channel: None,
                        controller: None,
                        value: i32::try_from(micros.as_int()).expect("a 24-bit tempo"),
                    });
                    continue;
                }
                let TrackEventKind::Midi { channel, message } = event.kind else {
                    continue;
                };
                let channel = channel.as_int();
                match message {
                    MidiMessage::ProgramChange { program } => stated.push(Stated {
                        track,
                        tick,
                        state: State::Program,
                        channel: Some(channel),
                        controller: None,
                        value: i32::from(program.as_int()),
                    }),
                    MidiMessage::Controller { controller, value } => stated.push(Stated {
                        track,
                        tick,
                        state: State::Controller,
                        channel: Some(channel),
                        controller: Some(controller.as_int()),
                        value: i32::from(value.as_int()),
                    }),
                    MidiMessage::PitchBend { bend } => stated.push(Stated {
                        track,
                        tick,
                        state: State::Bend,
                        channel: Some(channel),
                        controller: None,
                        value: i32::from(bend.as_int()),
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
                    state: state.state,
                };
                if !found.contains(&row) {
                    found.push(row);
                }
            };
            for note in &struck {
                if note.track != state.track
                    && note.tick == state.tick
                    && state.channel == Some(note.channel)
                {
                    against(note.track, Against::Notes);
                }
            }
            for other in &stated {
                if other.track != state.track
                    && other.tick == state.tick
                    && other.state == state.state
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
