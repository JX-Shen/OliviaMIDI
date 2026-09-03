//! Which Program a channel is on, and where the Take says so.
//!
//! Two shapes, because a passage has two different things to be told about its
//! orchestration. What each channel is on when the passage *begins* is a state:
//! it is what `play` puts the synthesiser in before the first note sounds, and
//! the Take may have set it many Bars earlier. Where the passage itself states
//! another is an event: it happens at a Position, and it is audible as a change.
//! Reporting only the first would hide a switch a listener hears; reporting only
//! the second would describe a passage as having no instrument at all.
//!
//! A Program is held by the channel, not by the track. That is not a modelling
//! preference — a synthesiser's program is channel state, so it is what
//! `mid play` actually hands one, and two tracks writing the same channel are
//! writing one thing. The track still appears on a `StatedProgram`, because the
//! event lives in a track and an Edit has to name the one it means.

use crate::bars::BarRange;
use crate::error::Result;
use crate::take::Take;
use midly::{MidiMessage, TrackEventKind};
use serde::Serialize;

/// Which Program one channel is on.
///
/// `None` is a Take that states none for this channel, and it is not program 0.
/// General MIDI's default is program 0, so the two are indistinguishable by ear
/// on a General MIDI bank — which is exactly why they must be distinguishable
/// here. Writing 0 where the file said nothing would change the Piece and pass
/// every audition. See #12.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Program {
    pub channel: u8,
    pub program: Option<u8>,
}

/// One place a Take states a Program: which track says it, on which channel, at
/// which Tick, and which Program.
///
/// Not `ProgramChange`, although that is the MIDI event's own name.
/// `midly::MidiMessage::ProgramChange` is the event as the format holds it, and
/// two types under one name in one crate would read as one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StatedProgram {
    pub track: usize,
    pub channel: u8,
    pub tick: u32,
    pub program: u8,
}

/// The orchestration of a passage: what each of its channels is on when it
/// begins, and where the passage states another.
///
/// A Program stated at the passage's own first Tick belongs to the state, not to
/// the events: it is in force from the moment the passage starts, which is what
/// a listener hears and what `play` hands the synthesiser. So each fact appears
/// once, and the events are the changes that happen *during* the passage.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Programs {
    pub programs: Vec<Program>,
    pub stated: Vec<StatedProgram>,
}

impl Take {
    /// Every place the Take states a Program, earliest Tick first, ties broken
    /// by track order.
    ///
    /// Read from wherever they are rather than from the notes' track: nothing
    /// obliges an export to put a channel's program change on the track whose
    /// notes play through it.
    pub fn stated_programs(&self) -> Result<Vec<StatedProgram>> {
        let smf = self.smf()?;
        let mut stated = Vec::new();

        for (track, events) in smf.tracks.iter().enumerate() {
            let mut tick = 0u32;
            for event in events {
                tick += event.delta.as_int();
                if let TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::ProgramChange { program },
                } = event.kind
                {
                    stated.push(StatedProgram {
                        track,
                        channel: channel.as_int(),
                        tick,
                        program: program.as_int(),
                    });
                }
            }
        }

        // Stable, so events sharing a Tick keep track order among themselves —
        // and where two tracks state a Program for one channel at one Tick, the
        // later track is the one in force, as it is for the synthesiser.
        stated.sort_by_key(|stated| stated.tick);
        Ok(stated)
    }

    /// The Programs of a passage, or of the whole Take when no Bar range is
    /// given.
    ///
    /// Which channels are listed: those the passage has notes on, and those a
    /// Program is in force or stated for. A channel with notes and no Program is
    /// listed as stating none — that is the fact a reader most needs, since the
    /// notes will sound on whatever the bank defaults to. A channel with a
    /// Program and no notes is listed too: a switch on a silent channel is
    /// something done to the Piece. The other channels are left out rather than
    /// padded to sixteen rows of nothing.
    pub fn programs_in(&self, bars: Option<BarRange>) -> Result<Programs> {
        let span = match bars {
            Some(bars) => self.tick_span(bars)?,
            None => crate::bars::TickSpan {
                start: 0,
                end: u32::MAX,
            },
        };
        let all = self.stated_programs()?;

        // In force at the passage's first Tick: the last thing said at or before
        // it, in the order `stated_programs` fixes.
        let mut in_force: Vec<(u8, u8)> = Vec::new();
        for stated in all.iter().filter(|stated| stated.tick <= span.start) {
            match in_force
                .iter_mut()
                .find(|(channel, _)| *channel == stated.channel)
            {
                Some((_, program)) => *program = stated.program,
                None => in_force.push((stated.channel, stated.program)),
            }
        }

        let stated: Vec<StatedProgram> = all
            .into_iter()
            .filter(|stated| span.start < stated.tick && stated.tick < span.end)
            .collect();

        let mut channels: Vec<u8> = self
            .notes_in(bars)?
            .iter()
            .map(|note| note.channel)
            .chain(in_force.iter().map(|&(channel, _)| channel))
            .chain(stated.iter().map(|stated| stated.channel))
            .collect();
        channels.sort_unstable();
        channels.dedup();

        Ok(Programs {
            programs: channels
                .into_iter()
                .map(|channel| Program {
                    channel,
                    program: in_force
                        .iter()
                        .find(|&&(held, _)| held == channel)
                        .map(|&(_, program)| program),
                })
                .collect(),
            stated,
        })
    }
}

/// What General Midi calls a Program, in lower case.
///
/// `None` above 127, which no Take can hold — a program byte is a `u7` and every
/// Edit lands through a range check — but which a caller can still ask about.
/// Total is not available here the way it is for `pitch_name`: a pitch name is a
/// claim about the file's own semantics, and this one is a claim about a
/// document outside the file.
///
/// General Midi numbers its instruments 1 to 128 and the byte in the file counts
/// 0 to 127, so this table is the specification's list shifted down by one:
/// program 40 is the 41st entry, `violin`. An off-by-one here would name the
/// wrong instrument in every command, and would sound like a plausible answer.
///
/// Lower case, and no name for a drum channel: both are decisions about what
/// this is a name *of*. See `mid`'s wording, and #12 for why the label saying
/// General Midi is part of the output rather than a decoration on it.
pub fn gm_name(program: u8) -> Option<&'static str> {
    const NAMES: [&str; 128] = [
        "acoustic grand piano",
        "bright acoustic piano",
        "electric grand piano",
        "honky-tonk piano",
        "electric piano 1",
        "electric piano 2",
        "harpsichord",
        "clavinet",
        "celesta",
        "glockenspiel",
        "music box",
        "vibraphone",
        "marimba",
        "xylophone",
        "tubular bells",
        "dulcimer",
        "drawbar organ",
        "percussive organ",
        "rock organ",
        "church organ",
        "reed organ",
        "accordion",
        "harmonica",
        "tango accordion",
        "acoustic guitar (nylon)",
        "acoustic guitar (steel)",
        "electric guitar (jazz)",
        "electric guitar (clean)",
        "electric guitar (muted)",
        "overdriven guitar",
        "distortion guitar",
        "guitar harmonics",
        "acoustic bass",
        "electric bass (finger)",
        "electric bass (pick)",
        "fretless bass",
        "slap bass 1",
        "slap bass 2",
        "synth bass 1",
        "synth bass 2",
        "violin",
        "viola",
        "cello",
        "contrabass",
        "tremolo strings",
        "pizzicato strings",
        "orchestral harp",
        "timpani",
        "string ensemble 1",
        "string ensemble 2",
        "synth strings 1",
        "synth strings 2",
        "choir aahs",
        "voice oohs",
        "synth voice",
        "orchestra hit",
        "trumpet",
        "trombone",
        "tuba",
        "muted trumpet",
        "french horn",
        "brass section",
        "synth brass 1",
        "synth brass 2",
        "soprano sax",
        "alto sax",
        "tenor sax",
        "baritone sax",
        "oboe",
        "english horn",
        "bassoon",
        "clarinet",
        "piccolo",
        "flute",
        "recorder",
        "pan flute",
        "blown bottle",
        "shakuhachi",
        "whistle",
        "ocarina",
        "lead 1 (square)",
        "lead 2 (sawtooth)",
        "lead 3 (calliope)",
        "lead 4 (chiff)",
        "lead 5 (charang)",
        "lead 6 (voice)",
        "lead 7 (fifths)",
        "lead 8 (bass + lead)",
        "pad 1 (new age)",
        "pad 2 (warm)",
        "pad 3 (polysynth)",
        "pad 4 (choir)",
        "pad 5 (bowed)",
        "pad 6 (metallic)",
        "pad 7 (halo)",
        "pad 8 (sweep)",
        "fx 1 (rain)",
        "fx 2 (soundtrack)",
        "fx 3 (crystal)",
        "fx 4 (atmosphere)",
        "fx 5 (brightness)",
        "fx 6 (goblins)",
        "fx 7 (echoes)",
        "fx 8 (sci-fi)",
        "sitar",
        "banjo",
        "shamisen",
        "koto",
        "kalimba",
        "bag pipe",
        "fiddle",
        "shanai",
        "tinkle bell",
        "agogo",
        "steel drums",
        "woodblock",
        "taiko drum",
        "melodic tom",
        "synth drum",
        "reverse cymbal",
        "guitar fret noise",
        "breath noise",
        "seashore",
        "bird tweet",
        "telephone ring",
        "helicopter",
        "applause",
        "gunshot",
    ];
    NAMES.get(usize::from(program)).copied()
}

/// The channel General Midi reserves for percussion, counted from zero as the
/// format counts it — channel 10 in every piece of documentation written for a
/// musician.
///
/// A Program selects a drum kit there rather than an instrument, so the melodic
/// name is not a name of it: program 40 on this channel is not a violin. The
/// number is still the number, which is why this suppresses a gloss rather than
/// a fact.
pub const GM_PERCUSSION_CHANNEL: u8 = 9;
