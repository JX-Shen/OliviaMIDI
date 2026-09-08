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
/// The Edit Set is JSON. These are the kinds, and none of them carries musical
/// intent. This list is the contract: nothing else describes it, and nothing
/// else is kept in sync with it.
///
///     {
///       "edits": [
///         { "kind": "move_note",      "id": "t1:c0:p69:s0:n0", "delta_ticks": -240 },
///         { "kind": "transpose_note", "id": "t1:c0:p69:s0:n0", "semitones": -2   },
///         { "kind": "resize_note",    "id": "t1:c0:p69:s0:n0", "delta_ticks": 120 },
///         { "kind": "set_velocity",   "id": "t1:c0:p69:s0:n0", "velocity": 40    },
///         { "kind": "delete_note",    "id": "t1:c0:p69:s0:n0" },
///         { "kind": "add_note", "track": 1, "channel": 0, "pitch": 69,
///                               "start": 5760, "duration": 955, "velocity": 50 },
///         { "kind": "set_program", "track": 1, "channel": 0, "tick": 0, "program": 40 },
///         { "kind": "set_controller",    "track": 1, "channel": 0, "controller": 11,
///                                        "tick": 1440, "value": 100 },
///         { "kind": "delete_controller", "track": 1, "channel": 0, "controller": 11,
///                                        "tick": 1440 },
///         { "kind": "move_controller",   "track": 1, "channel": 0, "controller": 11,
///                                        "tick": 1440, "delta_ticks": 240 }
///       ]
///     }
///
/// Identities come from `mid inspect`. `add_note` names none: it states a note,
/// and the identity of the note it makes is derived from where that note lands,
/// exactly as every other note's is.
///
/// `set_program` names no note either, and states an address rather than an
/// identity: there may be nothing there yet to name, and a Take saying nothing
/// about a channel is the ordinary case. It means *from this Tick, this channel
/// is on this Program* — so it changes the statement the Take makes at that Tick
/// and inserts one where the Take makes none. It does not reach a later
/// statement: the Take may switch again two Bars on, and that switch is still
/// the Take's to make. `mid inspect` is where you see what the passage ends up
/// on. The track is stated rather than inferred, because which track carries a
/// channel's program change is the author's arrangement of the file.
///
/// The three `*_controller` kinds reach what a channel holds for one Controller
/// at one Tick — CC number 0-119, value 0-127. Controllers 120-127 are channel
/// mode messages rather than Controllers, and no Edit reaches them.
///
/// `set_controller` states an address, as `set_program` does: it changes what the
/// channel holds at that Tick and creates a statement where the Take makes none.
/// `delete_controller` and `move_controller` *name* what is there instead —
/// there has to be something to take away or carry, and an address holding
/// nothing is an Edit Set written against a different Take. A move landing on an
/// address that already holds a value takes over from it. There is no Edit that
/// names a stretch: a curve is dozens of these, and that is what an Edit Set is
/// for.
///
/// Where a Take states one Controller twice at one address, the statement
/// written last is the one in force and so the one all three kinds mean. The
/// other is left exactly where it is: an Edit Set that did not name it may not
/// fold it away. A named Edit then means that event for the rest of the Edit
/// Set, so a second one asking to move it counts from wherever the first left
/// it, and one naming an event an earlier Edit deleted fails rather than turning
/// to the statement beside it.
///
/// A Tick is not an instant. It is a sequence carrying one timestamp, a
/// synthesiser meets its events one after another, and which of them it meets
/// first is audible. A channel-state event — a program change or a control
/// change — that an Edit places or changes at Tick T is written into its own
/// track:
///
///   1. after every other statement of its own address that remains at T,
///   2. then immediately before the first strike (a note-on with velocity above
///      zero) of its own channel, in that track, that follows that position,
///   3. or, where no such strike follows it, at the end of T.
///
/// Several placed at one such position by one Edit Set keep the order the Edit
/// Set gave them. So the notes struck at T sound under the state the Take now
/// states there, whether the Edit put that statement there or changed one that
/// was already there — and where the Take states an address twice, the value the
/// Edit asked for is the one left in force.
///
/// Every event no Edit named or landed on keeps the Tick it arrived with, its
/// content, and its Rank — its place among the events sharing its Tick —
/// relative to every other event that stayed put.
///
/// The limit, which is stated rather than hidden: where a track writes a release
/// behind a strike at one Tick, the position above falls before that release, so
/// a latching Controller placed there — a damper, a sostenuto — catches a note
/// the Take ends at T and holds it on. Such a Tick states no order between "as
/// these notes end" and "as these begin": a Program is decided only by the
/// strikes at its Tick, a damper only by the releases, and one point cannot be
/// on both sides of a pair written the wrong way round. The Program is
/// protected, because a note on the wrong instrument is a worse answer than a
/// note held too long. `mid` never writes that ordering itself and only ever
/// carries one in.
///
/// A note an Edit adds, moves or transposes is placed after every event already
/// at its Tick. So a note landing where another of the same track, channel and
/// pitch already starts takes the next occurrence index, and the note that was
/// there keeps the name it had.
///
/// A resize moves a note's release and nothing else, and a release is placed
/// before the events already at its Tick. So a note shortened or lengthened onto
/// the Tick where the next note of its pitch is struck releases first and leaves
/// that strike sounding; it does not renumber anything, a release having no
/// occurrence index of its own.
///
/// Refused rather than guessed at:
///
///   * a transpose leaving MIDI's 0-127, or a move landing before Tick 0
///   * a resize leaving a note 0 ticks long or shorter
///   * an add naming a track the Take does not have, a channel outside 0-15, a
///     pitch outside 0-127, a velocity outside 1-127 (0 is how the format spells
///     a note-off, not a note), a start before Tick 0, or no length at all
///   * a `set_program` naming a track the Take does not have, a channel outside
///     0-15, a program outside 0-127, or a Tick before 0
///   * a Controller Edit naming a track the Take does not have, a channel outside
///     0-15, a controller outside 0-119, a value outside 0-127, or a Tick before 0
///   * a `delete_controller` or `move_controller` naming an address the input
///     Take states nothing at — including one an earlier `set_controller` in the
///     same Edit Set created, because targets are fixed against the input Take
///   * an Edit leaving two notes of one track, channel and pitch finishing out
///     of the order they began in. MIDI has no way to tell such notes apart, so
///     re-reading the Take would give each of them the other's length.
///   * an Edit leaving a channel-state event on one track and, at its Tick, a
///     strike of that channel or a different value for its own address on
///     another. A Rank runs within a track. Two tracks have none between them
///     and the format does not say which a player merges first, so the file
///     would not state what the music is. See below.
///
/// This also checks the notes' side: adding or moving a note onto another
/// track's existing Bend at the same Tick and channel is refused, even though
/// the Bend itself was not edited. Move the strike to another Tick, or name the
/// Bend statement's site with `--allow-unranked`, not the note's track.
///
/// A Take that arrived that way keeps it: what the author wrote is the author's
/// (ADR-0003), `mid inspect` reports where a Take leaves an order unstated, and
/// only what this Edit Set would write is refused. Nor is anything said where
/// two tracks state one channel the *same* value at one Tick — both orders leave
/// the channel where the Take says, so the order decides nothing.
///
/// The two remedies are the two habits this ambiguity has always been avoided
/// by: state it on the track that carries those notes, or at a Tick where that
/// channel strikes nothing. Where neither is what you want — you know the player,
/// or the Take is a test — `--allow-unranked t2:c0:s1920` names the one site you
/// are answering for, spelled as a note's identity spells the same three facts.
/// It is per site and repeatable on purpose: there is no way to turn the check
/// off for a run.
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

    /// A site to leave in an order the file does not state, as `t2:c0:s1920`.
    /// Repeatable; each names one place you are answering for.
    #[arg(long = "allow-unranked", value_name = "SITE")]
    allow_unranked: Vec<battuta::Site>,
}

pub fn run(args: Args) -> battuta::Result<()> {
    battuta::edit::apply_to_new_take(&args.take, &args.edits, &args.output, &args.allow_unranked)
}
