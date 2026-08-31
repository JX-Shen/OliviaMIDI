use std::path::PathBuf;

/// List every note in a Take, with the identity that names it.
///
/// An identity is derived from the note's content — track, channel, pitch, start
/// Tick, and an occurrence index for notes that collide on all four. It is what
/// an Edit Set refers to, and it survives an Edit that does not touch the note.
///
/// `--bars 5:8` narrows it to a passage: Bars are 1-indexed and the range
/// includes both ends, so that is four Bars. A note belongs to the Bar it
/// *starts* in, even when it sustains across the Bar line.
///
/// The listing is in the order the music happens, so that reading down it reads
/// down the passage: a chord's notes are adjacent and the Bar numbers only ever
/// go forwards. `--json` keeps the Take's own order instead — track by track,
/// note-on by note-on — because that order is what fixes the occurrence index in
/// every identity, and an agent consuming the payload is entitled to see it.
/// Every line carries its identity, so the two orders name the same notes.
///
/// Bar lines are derived from the time signature the Take states, which in an
/// ordinary export lives on a different track than the notes. One time signature
/// has to govern the whole Take: one that states none is refused rather than
/// assumed to be in 4/4, and so is one that states none until part way in, or
/// changes time signature part way through. The final Bar counts as a Bar even
/// when the Take stops part way inside it.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to list.
    take: PathBuf,

    /// The passage to list, as FIRST:LAST — 1-indexed, both ends included.
    #[arg(long, value_name = "FIRST:LAST")]
    bars: Option<battuta::BarRange>,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> battuta::Result<()> {
    let take = battuta::Take::read(&args.take)?;
    let notes = take.notes_in(args.bars)?;

    if args.json {
        println!("{}", crate::json(&notes));
        return Ok(());
    }

    if notes.is_empty() {
        println!("no notes");
        return Ok(());
    }

    // A stable sort, so notes sharing a Tick keep the Take's own order among
    // themselves and two runs on one Take agree.
    let mut notes = notes;
    notes.sort_by_key(|note| note.start);

    // Read once for the whole listing rather than per note: every note of a
    // Take is placed against the same Bar lines.
    let lines = take.bar_lines();
    let rows: Vec<Vec<String>> = notes
        .iter()
        .map(|note| {
            let mut row = crate::wording::note(lines, note);
            // Last, and so never padded: it is the one thing on the line meant
            // to be copied rather than read.
            row.push(note.id.to_string());
            row
        })
        .collect();
    crate::wording::table(&rows);
    Ok(())
}
