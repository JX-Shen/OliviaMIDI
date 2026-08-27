use std::path::PathBuf;

/// Apply an Edit Set to a Take, writing a new Take.
///
/// The input is never modified and `-o` is required: the Take you liked cannot
/// be lost by running this. An Edit Set that fails anywhere fails as a whole —
/// no partial Take is ever written.
///
/// Every identity in the Edit Set is resolved against the input Take before the
/// first Edit is applied, so Edits cannot renumber each other's targets. They
/// then apply in the order given, which makes their effects ordered while their
/// targets were all fixed in advance. Two things follow: an Edit cannot name a
/// note added by an earlier Edit in the same Edit Set, and an Edit naming a note
/// an earlier Edit deleted fails rather than quietly doing nothing.
///
/// The Edit Set is JSON. Six kinds, none of which carries musical intent:
///
///     {
///       "edits": [
///         { "kind": "move_note",      "id": "t1:c0:p69:s0:n0", "delta_ticks": -240 },
///         { "kind": "transpose_note", "id": "t1:c0:p69:s0:n0", "semitones": -2   },
///         { "kind": "resize_note",    "id": "t1:c0:p69:s0:n0", "delta_ticks": 120 },
///         { "kind": "set_velocity",   "id": "t1:c0:p69:s0:n0", "velocity": 40    },
///         { "kind": "delete_note",    "id": "t1:c0:p69:s0:n0" },
///         { "kind": "add_note", "track": 1, "channel": 0, "pitch": 69,
///                               "start": 5760, "duration": 955, "velocity": 50 }
///       ]
///     }
///
/// Identities come from `mid inspect`. `add_note` names none: it states a note,
/// and the identity of the note it makes is derived from where that note lands,
/// exactly as every other note's is.
///
/// A note an Edit adds, moves or transposes is placed after every event already
/// at its Tick. So a note landing where another of the same track, channel and
/// pitch already starts takes the next occurrence index, and the note that was
/// there keeps the name it had.
///
/// Refused rather than guessed at:
///
///   * a transpose leaving MIDI's 0-127, or a move landing before Tick 0
///   * a resize leaving a note 0 ticks long or shorter
///   * an add naming a track the Take does not have, a channel outside 0-15, a
///     pitch outside 0-127, a velocity outside 1-127 (0 is how the format spells
///     a note-off, not a note), a start before Tick 0, or no length at all
///   * an Edit leaving two notes of one track, channel and pitch finishing out
///     of the order they began in. MIDI has no way to tell such notes apart, so
///     re-reading the Take would give each of them the other's length.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to change. Opened read-only.
    take: PathBuf,

    /// The Edit Set to apply, as JSON.
    edits: PathBuf,

    /// Where to write the new Take. Required, and never the input.
    #[arg(short = 'o', long = "output")]
    output: PathBuf,
}

pub fn run(args: Args) -> battuta::Result<()> {
    battuta::edit::apply_to_new_take(&args.take, &args.edits, &args.output)
}
