use std::path::PathBuf;

/// Everything `battuta` can fail at. One variant per condition a caller could
/// reasonably want to tell apart — the messages are the product surface, so
/// they say what to do next rather than only what went wrong.
///
/// Open by construction, and it says so. `CHARTER.md` requires a refusal to name
/// what was wrong rather than fail vaguely, so every word the vocabulary gains
/// arrives with its own refusals: an Edit kind, an event kind the tool learns to
/// read, a Rig it learns to resolve. A closed enum would promise a list this
/// crate cannot stop adding to, and would break an exhaustive match inside a
/// version range Cargo calls compatible — which is a smaller version of the
/// project stating what it does not do. See #23.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
        "{before} counts {before_ppq} ticks to the quarter note and {after} counts {after_ppq}, \
         so a Tick in one names no place in the other and the two cannot be compared. battuta \
         compares Ticks and never converts them: the same four quarter notes at these two \
         denominations share only the one starting at Tick 0. Re-export one of the Takes at the \
         other's ticks per quarter note."
    )]
    PpqMismatch {
        before: PathBuf,
        before_ppq: u16,
        after: PathBuf,
        after_ppq: u16,
    },

    #[error(
        "{path} places an event past Tick 4294967295, the largest absolute Tick battuta holds. \
         Ticks accumulate along a track and a delta time is only 28 bits, so a file can be built \
         out of gaps that are each writable and whose running total runs further than this. A \
         Take reaching this far is some nine million quarter notes long, which is almost always a \
         corrupt delta time rather than music: check what wrote it."
    )]
    TakeTooLong { path: PathBuf },

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

    #[error(
        "transposing {id} by {semitones} semitones lands on {landed}, which is not a MIDI pitch; \
         pitches are 0-127"
    )]
    TransposeOutOfRange {
        id: String,
        semitones: i64,
        landed: i64,
    },

    #[error(
        "on track {track}, channel {channel}, pitch {pitch}, the note starting at tick {first} \
         would finish after the note starting at tick {second} — one note ending inside another, \
         on the same channel and pitch. A note-off names a channel and a pitch, never the note it \
         ends, so re-reading this Take would give each of the two the other's length. Put one on \
         another channel, or make sure neither finishes inside the other."
    )]
    NotesIndistinguishable {
        track: usize,
        channel: u8,
        pitch: u8,
        first: u32,
        second: u32,
    },

    #[error("track {track} is not in the Take; it has {tracks} tracks, numbered from 0")]
    NoSuchTrack { track: i64, tracks: usize },

    #[error("channel {0} is out of range; a MIDI channel is 0-15")]
    ChannelOutOfRange(i64),

    #[error("pitch {0} is out of range; a MIDI pitch is 0-127")]
    PitchOutOfRange(i64),

    #[error("a note cannot start at tick {0}; the first Tick of a Take is 0")]
    StartOutOfRange(i64),

    #[error("program {0} is out of range; a MIDI program is 0-127")]
    ProgramOutOfRange(i64),

    #[error("a Program cannot be stated at tick {0}; the first Tick of a Take is 0")]
    ProgramTickOutOfRange(i64),

    #[error(
        "controller {0} is out of range; a Controller is 0-119; 120-127 are channel mode messages"
    )]
    ControllerOutOfRange(i64),

    #[error("controller value {0} is out of range; a MIDI controller value is 0-127")]
    ControllerValueOutOfRange(i64),

    #[error("a Controller cannot be stated at tick {0}; the first Tick of a Take is 0")]
    ControllerTickOutOfRange(i64),

    #[error(
        "no Controller is stated on track {track} channel {channel} controller {controller} at tick {tick}"
    )]
    UnknownController {
        track: i64,
        channel: i64,
        controller: i64,
        tick: i64,
    },

    #[error(
        "a note cannot last {0} ticks; a note lasts at least 1 tick, and has to end at a Tick the \
         Take can hold"
    )]
    DurationOutOfRange(i64),

    #[error(
        "an earlier Edit in this Edit Set deleted {0}, so this one has nothing to change. Every \
         identity is resolved against the input Take, so a deleted note still answers to its name."
    )]
    NoteAlreadyDeleted(String),

    #[error(
        "an earlier Edit in this Edit Set deleted the Controller on track {track} channel \
         {channel} controller {controller} at tick {tick}, so this one has nothing to change. \
         Every target is resolved against the input Take, so a deleted Controller still answers \
         to the address it was stated at — and this Edit means that event, not the other \
         statement there."
    )]
    ControllerAlreadyDeleted {
        track: usize,
        channel: u8,
        controller: u8,
        tick: u32,
    },

    #[error(
        "moving {id} by {delta_ticks} ticks lands it at {landed}, which is not a Tick; the first \
         Tick of a Take is 0"
    )]
    MoveOutOfRange {
        id: String,
        delta_ticks: i64,
        landed: i64,
    },

    #[error(
        "resizing {id} by {delta_ticks} ticks leaves it {duration} ticks long; a note lasts at \
         least 1 tick"
    )]
    ResizeOutOfRange {
        id: String,
        delta_ticks: i64,
        duration: i64,
    },

    #[error(
        "{0} ticks separate two events, and a MIDI delta time cannot reach that far — 268435455 \
         ticks is as far as one goes, so this Take cannot be written. If an Edit moved a note, \
         move it a shorter distance. If a Bar range was asked for, the events left behind either \
         side of it are further apart than the format can write; ask for a range that does not \
         span this much silence."
    )]
    GapUnwritable(u32),

    #[error(
        "the events {0} was found at no longer carry a note. That is a fault in battuta rather \
         than in your Take or your Edit Set; nothing has been written."
    )]
    NoteEventsLost(String),

    #[error(
        "apply never writes in place: -o {0} names the input Take. One file has as many names as \
         something has given it, and a symlink or a second hard link to your input is still your \
         input. Choose an output that does not already name it."
    )]
    WriteInPlace(PathBuf),

    #[error(
        "no Rig configured. Pass --rig <soundfont.sf2>, or set BATTUTA_SOUNDFONT to a soundfont \
         path.\n\
         \n\
         No soundfont yet? GeneralUser GS, by S. Christian Collins, is the bank this project is \
         developed against: all 128 General MIDI melodic presets, and a real grand piano at \
         program 0. The bank is named rather than linked so that the recipe outlives any one \
         download page.\n\
         \n\
         Not the demo bank that arrives beside fluid-synth — it holds no piano at all, so a Take \
         stating no program renders as bells. It makes sound, which is exactly what makes it the \
         worse failure.\n\
         \n\
         battuta never picks a soundfont for you: an audition heard through a Rig you did not \
         choose is a judgement you cannot trust."
    )]
    NoRig,

    #[error("the Rig's soundfont {0} does not exist")]
    RigMissing(PathBuf),

    #[error(
        "the passage could not be written to a temporary file: {0}. A Bar range is played from \
         a temporary Take, never from one written beside your own files, so playing one needs a \
         writable temporary directory."
    )]
    PassageUnwritable(#[source] std::io::Error),

    #[error("fluidsynth is not on PATH. Install it — on macOS, `brew install fluid-synth`.")]
    NoFluidsynth,

    #[error("fluidsynth is on PATH but could not be started: {0}")]
    FluidsynthUnusable(#[source] std::io::Error),

    #[error("fluidsynth failed: {0}")]
    FluidsynthFailed(std::process::ExitStatus),
}

pub type Result<T> = std::result::Result<T, Error>;
