use std::path::PathBuf;

/// List every note in a Take, with the identity that names it.
///
/// An identity is derived from the note's content — track, channel, pitch, start
/// Tick, and an occurrence index for notes that collide on all four. It is what
/// an Edit Set refers to, and it survives an Edit that does not touch the note.
///
/// There is no Bar filtering yet: this lists the whole Take.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to list.
    take: PathBuf,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> battuta::Result<()> {
    let notes = battuta::Take::read(&args.take)?.notes()?;

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
