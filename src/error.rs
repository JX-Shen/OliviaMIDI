use std::path::PathBuf;

/// Everything `battuta` can fail at. One variant per condition a caller could
/// reasonably want to tell apart — the messages are the product surface, so
/// they say what to do next rather than only what went wrong.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{path} is not a MIDI file this tool can read: {source}")]
    Malformed {
        path: PathBuf,
        #[source]
        source: midly::Error,
    },

    #[error(
        "{path} uses SMPTE timecode division; battuta reads metrical (ticks-per-quarter) files only"
    )]
    NotMetrical { path: PathBuf },

    #[error(
        "{path} states a time signature whose denominator is 2^{power}, which is not a note value"
    )]
    UnreadableTimeSignature { path: PathBuf, power: u8 },

    #[error(
        "track {track} has a note on channel {channel}, pitch {pitch}, that is never released"
    )]
    UnterminatedNote {
        track: usize,
        channel: u8,
        pitch: u8,
    },

    #[error("{path} states 0 ticks per quarter note, so nothing in it can be placed in time")]
    ZeroPpq { path: PathBuf },

    #[error(
        "{path} states no time signature, so it has no Bars to select. battuta never \
         assumes 4/4: a Bar number derived from a time signature the Take does not state is a \
         wrong answer with nothing to reveal it. Inspect the whole Take, or state the time \
         signature in the file."
    )]
    NoTimeSignature { path: PathBuf },

    #[error(
        "{path} does not state a time signature until tick {at_tick}, so the Bars before that \
         one have none to derive Bar lines from. Inspect the whole Take, or state the time \
         signature from the start of the file."
    )]
    TimeSignatureStartsLate { path: PathBuf, at_tick: u32 },

    #[error(
        "{path} changes time signature from {from} to {to} at tick {at_tick}. battuta derives \
         Bar lines from one time signature governing the whole Take, so it cannot say which Bar \
         a tick is in here. Inspect the whole Take instead."
    )]
    TimeSignatureChanges {
        path: PathBuf,
        at_tick: u32,
        from: crate::take::TimeSignature,
        to: crate::take::TimeSignature,
    },

    #[error(
        "{path} states a time signature of 0/{denominator}, which describes no Bar. Inspect the \
         whole Take instead."
    )]
    TimeSignatureWithNoBeats { path: PathBuf, denominator: u8 },

    #[error(
        "{path} states {numerator}/{denominator}, whose Bar is not a whole number of ticks at \
         {ppq} ticks per quarter. battuta will not round a Bar line. Inspect the whole Take \
         instead."
    )]
    BarNotWholeTicks {
        path: PathBuf,
        numerator: u8,
        denominator: u8,
        ppq: u16,
    },

    #[error(
        "a Bar range is written FIRST:LAST, 1-indexed and inclusive of both ends, as in 5:8; \
         {0} is not that"
    )]
    BarRangeMalformed(String),

    #[error("Bars are 1-indexed: the first Bar of a Take is Bar 1, and there is no Bar 0")]
    BarZero,

    #[error(
        "the Bar range {first}:{last} runs backwards; the first Bar must not come after the last"
    )]
    BarRangeInverted { first: u32, last: u32 },

    #[error("{path} is {bars} Bars long, so there is no Bar {last} to select")]
    BarRangeBeyondTake { path: PathBuf, last: u32, bars: u32 },

    #[error("cannot read the Edit Set {path}: {source}")]
    EditSetUnreadable {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("no note in the input Take has the identity {0}")]
    UnknownNote(String),

    #[error("the edited Take could not be encoded as MIDI: {0}")]
    Encode(String),

    #[error("velocity {0} is out of range; a note velocity is 1-127")]
    VelocityOutOfRange(i64),

    #[error("apply never writes in place: -o {0} is the input Take")]
    WriteInPlace(PathBuf),

    #[error(
        "no Rig configured. Pass --rig <soundfont.sf2>, or set BATTUTA_SOUNDFONT to a soundfont path.\n\
         battuta never picks a soundfont for you: an audition heard through a Rig you did not choose \
         is a judgement you cannot trust."
    )]
    NoRig,

    #[error("the Rig's soundfont {0} does not exist")]
    RigMissing(PathBuf),

    #[error("fluidsynth is not on PATH. Install it — on macOS, `brew install fluid-synth`.")]
    NoFluidsynth,

    #[error("fluidsynth is on PATH but could not be started: {0}")]
    FluidsynthUnusable(#[source] std::io::Error),

    #[error("fluidsynth failed: {0}")]
    FluidsynthFailed(std::process::ExitStatus),
}

pub type Result<T> = std::result::Result<T, Error>;
