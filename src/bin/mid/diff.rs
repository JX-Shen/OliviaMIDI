use std::collections::HashMap;
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
/// reports both. `--json` gives both of its identities, because a note that
/// moved is called one thing in the before Take and another in the after; the
/// human reading is a description of what happened to the music, and says where
/// the note is rather than what it is named.
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
    // reasoning as the Rig disclosure; see ADR-0005.
    eprintln!("tolerance: {} ticks", diff.tolerance_ticks);

    if args.json {
        println!("{}", crate::json(&diff));
        return Ok(());
    }

    if diff.is_empty() {
        println!("no differences");
        return Ok(());
    }

    // Each Take is placed against its own Bar lines. Two Takes being compared
    // are usually in the same time signature, but nothing here requires it, and
    // a note is where its own Take says it is.
    let before_lines = before.bar_lines();
    let after_lines = after.bar_lines();

    let mut rows = Vec::new();
    for note in &diff.added {
        rows.push(described("added", after_lines, note));
    }
    for note in &diff.removed {
        rows.push(described("removed", before_lines, note));
    }
    // How many notes share each address, so that a row describing one of a
    // collision can say which. Read from the before Take, because that is the
    // one the row names a note in.
    let crowding = crowding(&before)?;
    for change in &diff.changed {
        let mut row = vec!["changed".to_string()];
        // The note as it was, not as it became: the reader knows the before
        // Take, and each clause says where its field went.
        row.extend(crate::wording::names(
            before_lines,
            &change.before,
            crowding_at(&crowding, &change.before),
        ));
        row.push(stated(change, after_lines));
        rows.push(row);
    }
    crate::wording::table(&rows);
    Ok(())
}

/// How many notes share each address — track, channel, pitch and start Tick —
/// in a Take.
///
/// One is the ordinary answer and means the address names a note. More than one
/// is a collision, and is what `wording::names` needs to know before it can
/// claim to have pointed at a note rather than at a place several notes are.
fn crowding(take: &battuta::Take) -> battuta::Result<HashMap<(usize, u8, u8, u32), usize>> {
    let mut counted = HashMap::new();
    for note in take.notes()? {
        *counted
            .entry((note.track, note.channel, note.pitch, note.start))
            .or_insert(0) += 1;
    }
    Ok(counted)
}

fn crowding_at(counted: &HashMap<(usize, u8, u8, u32), usize>, note: &battuta::Note) -> usize {
    counted
        .get(&(note.track, note.channel, note.pitch, note.start))
        .copied()
        .unwrap_or(1)
}

/// A note that arrived or left, described the way `inspect` lists one — the
/// same cells in the same order, behind what became of it.
fn described(verb: &str, lines: Option<battuta::BarLines>, note: &battuta::Note) -> Vec<String> {
    let mut row = vec![verb.to_string()];
    row.extend(crate::wording::note(lines, note));
    row
}

/// Each change as the two things it went between. Formatting only: which
/// changes there are, and the order they come in, are the library's.
///
/// A `start` change reads as *moved to*, not as one position arrow to another:
/// the row already opens with where the note was, and repeating it would spend
/// the widest clause on the line saying the same thing twice. It is also the one
/// classification the V0.1 spec names in the human's own words — "a moved note
/// reported as moved" — so it is the one that must not come out as `changed`
/// alone.
fn stated(change: &battuta::diff::NoteChange, after_lines: Option<battuta::BarLines>) -> String {
    use battuta::diff::Change;
    change
        .changes
        .iter()
        .map(|&kind| match kind {
            // `transposed to`, not one pitch arrow to another, for the reason
            // `moved to` reads that way: the row already opens with the note as
            // it was, so naming the pitch it came from would say it twice.
            Change::Pitch => format!(
                "transposed to {}",
                crate::wording::pitch(change.after.pitch)
            ),
            Change::Start => format!(
                "moved to {}",
                crate::wording::at(after_lines, change.after.start)
            ),
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
