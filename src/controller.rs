//! What a channel holds for a Controller, and where the Take says so.
//!
//! The shape is `program`'s, because ADR-0007 is one rule over both: what a
//! channel holds is the answer, and the events that put it there are where it
//! came from. A value set many Bars before a passage is in force at its first
//! note, which is what `play` hands the synthesiser and so what a listener
//! hears.
//!
//! Nothing here segments a stretch of events into a rise or a fall. A curve is
//! how a musician reads controller data and is not a thing the file holds; every
//! number reported is read straight out of it.

use crate::bars::BarRange;
use crate::error::Result;
use crate::take::Take;
use midly::{MidiMessage, TrackEventKind};
use serde::Serialize;

/// What one channel holds for one Controller, and the highest it holds anywhere
/// in the passage.
///
/// Two readings rather than a summary of a shape. `value` is what is in force
/// when the passage begins — what a listener hears at its first note — and
/// `peak` is the highest value in force at any point in it, `peak_at` the Tick
/// it first reaches that. The starting value counts towards the peak, because it
/// is in force during the passage like any other; where nothing higher is
/// stated, the peak *is* it, at the passage's own first Tick.
///
/// Neither is bounded by a parameter, because neither is inferred: both are read
/// out of the file (ADR-0007, and the amendment it puts on ADR-0004). A peak
/// survives a curve that wobbles — a fader records 76, 80, 78, 100, 96 — where
/// anything reading a *stretch* of events as a rise or a fall would not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Controller {
    pub channel: u8,
    pub controller: u8,

    /// What the channel holds when the passage begins. `None` is a Take that set
    /// nothing for this Controller before it, and it is not 0: a synthesiser
    /// beginning a passage with the pedal up sounds like one that was never told,
    /// and the two are different Pieces. #12's sixth criterion, one level in.
    pub value: Option<u8>,
    pub peak: u8,
    pub peak_at: u32,
}

/// One place a Take states a Controller: which track says it, on which channel,
/// at which Tick, which Controller and what value.
///
/// Not `ControlChange`, although that is the MIDI event's own name, for the
/// reason `StatedProgram` is not `ProgramChange`: two types under one name in
/// one crate would read as one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StatedController {
    pub track: usize,
    pub channel: u8,
    pub controller: u8,
    pub tick: u32,
    pub value: u8,
}

/// The Controllers of a passage: what each of its channels holds when it begins,
/// and where the passage states another.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Controllers {
    pub controllers: Vec<Controller>,
    pub stated: Vec<StatedController>,
}

impl Take {
    /// Every place the Take states a Controller, earliest Tick first, ties
    /// broken by track order.
    ///
    /// Read from wherever they are rather than from the notes' track: nothing
    /// obliges an export to put a channel's control changes on the track whose
    /// notes play through it.
    pub fn stated_controllers(&self) -> Result<Vec<StatedController>> {
        let smf = self.smf()?;
        let mut stated = Vec::new();

        for (track, events) in smf.tracks.iter().enumerate() {
            let mut tick = 0u32;
            for event in events {
                tick += event.delta.as_int();
                if let TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::Controller { controller, value },
                } = event.kind
                {
                    stated.push(StatedController {
                        track,
                        channel: channel.as_int(),
                        controller: controller.as_int(),
                        tick,
                        value: value.as_int(),
                    });
                }
            }
        }

        // Stable, so events sharing a Tick keep track order among themselves —
        // and where two tracks state one Controller at one Tick, the later track
        // is the one in force, as it is for the synthesiser.
        stated.sort_by_key(|stated| stated.tick);
        Ok(stated)
    }

    /// The Controllers of a passage: what each channel holds when it begins, and
    /// where the passage states another.
    ///
    /// A value stated at the passage's own first Tick belongs to the state, not
    /// to the events: it is in force from the moment the passage starts, which
    /// is what a listener hears. So each fact appears once, and the events are
    /// the changes that happen *during* the passage — the cut `programs_in`
    /// makes, for the same reason.
    pub fn controllers_in(&self, bars: Option<BarRange>) -> Result<Controllers> {
        let span = match bars {
            Some(bars) => self.tick_span(bars)?,
            None => crate::bars::TickSpan {
                start: 0,
                end: u32::MAX,
            },
        };

        let all = self.stated_controllers()?;

        let mut held: Vec<Controller> = Vec::new();
        for stated in all.iter().filter(|stated| stated.tick <= span.start) {
            match held
                .iter_mut()
                .find(|held| held.channel == stated.channel && held.controller == stated.controller)
            {
                Some(held) => {
                    held.value = Some(stated.value);
                    held.peak = stated.value;
                }
                None => held.push(Controller {
                    channel: stated.channel,
                    controller: stated.controller,
                    value: Some(stated.value),
                    peak: stated.value,
                    peak_at: span.start,
                }),
            }
        }

        let stated: Vec<StatedController> = all
            .into_iter()
            .filter(|stated| span.start < stated.tick && stated.tick < span.end)
            .collect();

        // A Controller the passage names and nothing set before it is listed too,
        // holding nothing. Leaving it out would print its events under a block
        // that had just said no Controller was stated, and a reader would have to
        // decide which half of the output to believe. Its peak starts at the
        // first thing the passage says, so that a pair whose every value is 0 is
        // still reported at the Tick it was first said rather than at the
        // passage's own start.
        for stated in &stated {
            if !held
                .iter()
                .any(|held| held.channel == stated.channel && held.controller == stated.controller)
            {
                held.push(Controller {
                    channel: stated.channel,
                    controller: stated.controller,
                    value: None,
                    peak: stated.value,
                    peak_at: stated.tick,
                });
            }
        }

        // Strictly greater, so a value reached twice is reported at the first of
        // the two: where the passage *first* reaches its highest is the fact, and
        // a later restatement of it changes nothing a listener hears.
        for held in held.iter_mut() {
            for stated in stated.iter().filter(|stated| {
                stated.channel == held.channel && stated.controller == held.controller
            }) {
                if stated.value > held.peak {
                    held.peak = stated.value;
                    held.peak_at = stated.tick;
                }
            }
        }

        held.sort_by_key(|held| (held.channel, held.controller));

        Ok(Controllers {
            controllers: held,
            stated,
        })
    }
}
