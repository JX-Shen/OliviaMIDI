use std::path::PathBuf;

/// Say what differs between two Takes.
///
/// Only the Piece is compared. Two Takes heard through different Rigs are not
/// different Takes, and no Rig difference is ever reported here.
///
/// Matching is exact for now: a note pairs with a note in the other Take only
/// when track, channel, pitch and start Tick are identical. A note that moved
/// therefore reports as one Removed and one Added rather than as moved.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to compare from.
    before: PathBuf,

    /// The Take to compare to.
    after: PathBuf,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> battuta::Result<()> {
    let before = battuta::Take::read(&args.before)?;
    let after = battuta::Take::read(&args.after)?;
    let diff = battuta::diff::diff(&before, &after)?;

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
    for change in &diff.velocity_changed {
        println!("velocity  {}  {} -> {}", change.id, change.from, change.to);
    }
    if diff.is_empty() {
        println!("no differences");
    }
    Ok(())
}
