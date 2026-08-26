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
    let notes = battuta::Take::read(&args.take)?.notes_in(args.bars)?;

    if args.json {
        println!("{}", crate::json(&notes));
        return Ok(());
    }

    for note in &notes {
        println!(
            "{}  track {} channel {} pitch {} start {} duration {} velocity {}",
            note.id, note.track, note.channel, note.pitch, note.start, note.duration, note.velocity
        );
    }
    Ok(())
}
