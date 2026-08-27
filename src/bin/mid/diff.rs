use std::path::PathBuf;

/// Say what differs between two Takes.
///
/// Only the Piece is compared. Two Takes heard through different Rigs are not
/// different Takes, and no Rig difference is ever reported here.
///
/// Notes are matched in two passes. First by identity — same track, channel,
/// pitch and start Tick — and then, among whatever is left over, by nearest
/// neighbour within the same track and channel, as long as the two starts are
/// no further apart than the tolerance. Whatever is still unmatched is Added or
/// Removed.
///
/// That second pass is the one that says a note *moved* rather than that one
/// note vanished and another appeared, and the tolerance is the whole of its
/// evidence: two notes it pairs have different identities, because a note's
/// pitch and start Tick are part of what names it. So the tolerance is stated
/// on every diff — on stderr, and in the payload under `--json`.
///
/// A matched note reports everything about it that differs, in the fixed order
/// pitch, start, duration, velocity — a note that was both moved and softened
/// reports both. Both of its identities are given, because a note that moved is
/// called one thing in the before Take and another in the after.
///
/// The two Takes must count the same number of Ticks to the quarter note. Ticks
/// are the truth here and are never converted, so Takes at two denominations
/// are refused rather than compared as if a Tick meant the same thing in each.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to compare from.
    before: PathBuf,

    /// The Take to compare to.
    after: PathBuf,

    /// How far apart in Ticks two notes may start and still be the same note,
    /// moved. Defaults to a sixteenth note — the Take's ticks per quarter note
    /// divided by four, so 120 at the usual 480. 0 matches by identity alone.
    #[arg(long, value_name = "TICKS")]
    tolerance: Option<u32>,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> battuta::Result<()> {
    let before = battuta::Take::read(&args.before)?;
    let after = battuta::Take::read(&args.after)?;
    let diff = battuta::diff::diff(&before, &after, args.tolerance)?;

    // The library decides *that* the tolerance is part of the answer — it is a
    // field of the `Diff` — and this decides what it says and where it goes.
    // Stderr, so that `--json` on stdout stays one JSON document and a pipeline
    // redirecting the payload cannot drop the disclosure with it. The same
    // reasoning as the Rig disclosure; see ADR-0009.
    eprintln!("tolerance: {} ticks", diff.tolerance_ticks);

    if args.json {
        println!("{}", crate::json(&diff));
        return Ok(());
    }

    for note in &diff.added {
        println!("added     {}  velocity {}", note.id, note.velocity);
    }
    for note in &diff.removed {
        println!("removed   {}  velocity {}", note.id, note.velocity);
    }
    for change in &diff.changed {
        println!(
            "changed   {} -> {}  {}",
            change.before.id,
            change.after.id,
            stated(change)
        );
    }
    if diff.is_empty() {
        println!("no differences");
    }
    Ok(())
}

/// Each change as the two numbers it went between. Formatting only: which
/// changes there are, and the order they come in, are the library's.
fn stated(change: &battuta::diff::NoteChange) -> String {
    use battuta::diff::Change;
    change
        .changes
        .iter()
        .map(|&kind| match kind {
            Change::Pitch => format!("pitch {} -> {}", change.before.pitch, change.after.pitch),
            Change::Start => format!("start {} -> {}", change.before.start, change.after.start),
            Change::Duration => format!(
                "duration {} -> {}",
                change.before.duration, change.after.duration
            ),
            Change::Velocity => format!(
                "velocity {} -> {}",
                change.before.velocity, change.after.velocity
            ),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
