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

/// The lowest control change number that is not a Controller.
///
/// 120 to 127 are MIDI's channel mode messages — All Sound Off, All Notes Off,
/// Reset All Controllers and their neighbours. They ride on the control change
/// event type and are instructions rather than settings: they happen and are
/// over, leaving nothing in force. Every reading this crate makes of channel
/// state presupposes a value that holds until changed, so for these there is
/// nothing to report rather than something dangerous to report.
///
/// The boundary is drawn by what those readings require and lands where MIDI's
/// own table breaks only because the specification's authors made the same
/// distinction. A Take's mode messages are carried untouched (ADR-0003) and
/// named by nothing; the hole is deliberate, and `fixtures/expressive.mid` pins
/// it. See ADR-0007 and #13.
pub const FIRST_CHANNEL_MODE: u8 = 120;

/// One place a Take states a Controller, and which event of its track says so.
///
/// The event index is what lets a named Controller Edit keep the statement it
/// resolved against for a whole Edit Set. An address cannot: where a Take states
/// one Controller twice at one address, asking the address again after an
/// earlier Edit has run gives back whichever statement is in force *now*, which
/// is the wrong one the moment an Edit has moved or removed the right one. See
/// #18. `Note` carries its two event indices for the same reason.
///
/// Crate-private, and a pairing rather than a field on `StatedController`: an
/// index into one track's event list means something for the length of one
/// `apply` and has no business in what the library hands a consumer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ControllerEvent {
    pub(crate) stated: StatedController,
    /// Counting every event of the track, meta events included — the numbering
    /// `Rewrite` opens a track into, and the one `Note` records a strike and a
    /// release by.
    pub(crate) event: usize,
}

impl Take {
    /// Every place the Take states a Controller, earliest Tick first, ties
    /// broken by track order.
    ///
    /// Read from wherever they are rather than from the notes' track: nothing
    /// obliges an export to put a channel's control changes on the track whose
    /// notes play through it.
    pub fn stated_controllers(&self) -> Result<Vec<StatedController>> {
        Ok(self
            .controller_events()?
            .into_iter()
            .map(|event| event.stated)
            .collect())
    }

    /// The same, each with the event that states it.
    ///
    /// The two readings walk one list the same way, which is why this is the one
    /// that walks it.
    pub(crate) fn controller_events(&self) -> Result<Vec<ControllerEvent>> {
        let smf = self.smf()?;
        let mut found = Vec::new();

        for (track, events) in smf.tracks.iter().enumerate() {
            let mut tick = 0u32;
            for (index, event) in events.iter().enumerate() {
                tick += event.delta.as_int();
                if let TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::Controller { controller, value },
                } = event.kind
                {
                    if controller.as_int() >= FIRST_CHANNEL_MODE {
                        continue;
                    }
                    found.push(ControllerEvent {
                        stated: StatedController {
                            track,
                            channel: channel.as_int(),
                            controller: controller.as_int(),
                            tick,
                            value: value.as_int(),
                        },
                        event: index,
                    });
                }
            }
        }

        // Stable, so events sharing a Tick keep track order among themselves —
        // and where two tracks state one Controller at one Tick, the later track
        // is the one in force, as it is for the synthesiser. Within one track it
        // keeps file order, so the last statement at an address is still the last
        // one after this.
        found.sort_by_key(|found| found.stated.tick);
        Ok(found)
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

/// What MIDI's own table calls a Controller, in lower case and in its own words.
///
/// The quotation, stored whole so that it can be checked a line at a time
/// against the MMA's Control Change list. How much of it is worth a table cell
/// is the consumer's decision, and `mid` trims two kinds of clause it has no use
/// for; the library hands over what the document says (ADR-0005).
///
/// `None` where the specification defines no name: the numbers it leaves
/// undefined, the LSB rows whose own control is undefined, and everything from
/// `FIRST_CHANNEL_MODE` up, which is not a Controller at all. *Undefined* is not
/// a name — it is the table saying nothing — and a caller printing nothing is
/// printing that faithfully. A number a vendor uses for its own purposes is
/// exactly this case, and a reader seeing no name has learned something true:
/// whatever the value means here was decided outside MIDI.
///
/// Quoted rather than improved on. CC64 is *damper pedal on/off (sustain)* in
/// the specification and *sustain pedal* to every pianist, and the friendlier
/// word would stop this being a quotation and start it being this tool's opinion
/// of what the control is for. The same rule keeps `modulation wheel or lever`
/// whole.
///
/// Unlike a GM name this needs no label where it is printed: which control a
/// number means depends on nothing but MIDI, the way pitch 66 is F#4 in every
/// Take, so the `CC` a consumer already writes is the whole of the attribution.
/// What it does *not* say is what the value will sound like, which is the Rig's
/// and is never claimed here.
///
/// A wrong string names the wrong control and would read as a plausible answer,
/// which is `gm_name`'s hazard over a shorter table. Checked against
/// <https://midi.org/midi-1-0-control-change-messages>; check any change the
/// same way, and against the document rather than against another tool.
pub fn spec_name(controller: u8) -> Option<&'static str> {
    const NAMES: [Option<&str>; 120] = [
        Some("bank select"),
        Some("modulation wheel or lever"),
        Some("breath controller"),
        None,
        Some("foot controller"),
        Some("portamento time"),
        Some("data entry msb"),
        Some("channel volume (formerly main volume)"),
        Some("balance"),
        None,
        Some("pan"),
        Some("expression controller"),
        Some("effect control 1"),
        Some("effect control 2"),
        None,
        None,
        Some("general purpose controller 1"),
        Some("general purpose controller 2"),
        Some("general purpose controller 3"),
        Some("general purpose controller 4"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some("lsb for control 0 (bank select)"),
        Some("lsb for control 1 (modulation wheel or lever)"),
        Some("lsb for control 2 (breath controller)"),
        None,
        Some("lsb for control 4 (foot controller)"),
        Some("lsb for control 5 (portamento time)"),
        Some("lsb for control 6 (data entry)"),
        Some("lsb for control 7 (channel volume, formerly main volume)"),
        Some("lsb for control 8 (balance)"),
        None,
        Some("lsb for control 10 (pan)"),
        Some("lsb for control 11 (expression controller)"),
        Some("lsb for control 12 (effect control 1)"),
        Some("lsb for control 13 (effect control 2)"),
        None,
        None,
        Some("lsb for control 16 (general purpose controller 1)"),
        Some("lsb for control 17 (general purpose controller 2)"),
        Some("lsb for control 18 (general purpose controller 3)"),
        Some("lsb for control 19 (general purpose controller 4)"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some("damper pedal on/off (sustain)"),
        Some("portamento on/off"),
        Some("sostenuto on/off"),
        Some("soft pedal on/off"),
        Some("legato footswitch"),
        Some("hold 2"),
        Some("sound controller 1 (default: sound variation)"),
        Some("sound controller 2 (default: timbre/harmonic intens.)"),
        Some("sound controller 3 (default: release time)"),
        Some("sound controller 4 (default: attack time)"),
        Some("sound controller 5 (default: brightness)"),
        Some("sound controller 6 (default: decay time - see mma rp-021)"),
        Some("sound controller 7 (default: vibrato rate - see mma rp-021)"),
        Some("sound controller 8 (default: vibrato depth - see mma rp-021)"),
        Some("sound controller 9 (default: vibrato delay - see mma rp-021)"),
        Some("sound controller 10 (default undefined - see mma rp-021)"),
        Some("general purpose controller 5"),
        Some("general purpose controller 6"),
        Some("general purpose controller 7"),
        Some("general purpose controller 8"),
        Some("portamento control"),
        None,
        None,
        None,
        Some("high resolution velocity prefix"),
        None,
        None,
        Some("effects 1 depth (default: reverb send level - see mma rp-023)"),
        Some("effects 2 depth (formerly tremolo depth)"),
        Some("effects 3 depth (default: chorus send level - see mma rp-023)"),
        Some("effects 4 depth (formerly celeste [detune] depth)"),
        Some("effects 5 depth (formerly phaser depth)"),
        Some("data increment (data entry +1)"),
        Some("data decrement (data entry -1)"),
        Some("non-registered parameter number (nrpn) - lsb"),
        Some("non-registered parameter number (nrpn) - msb"),
        Some("registered parameter number (rpn) - lsb"),
        Some("registered parameter number (rpn) - msb"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ];
    NAMES.get(usize::from(controller)).copied().flatten()
}
