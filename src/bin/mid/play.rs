use std::path::PathBuf;

/// Hear a Take, through a Rig you chose.
///
/// The Rig is resolved as `--rig`, then `BATTUTA_SOUNDFONT`, then failure. There
/// is no fallback to a system soundfont and there never will be: forming an
/// opinion about your music through a soundfont you did not pick is a mistake
/// you cannot see afterwards.
///
/// The Rig used is always stated — on stderr, and in the payload under `--json`.
/// No flag suppresses it.
///
/// Playback is FluidSynth, found on PATH. "FluidSynth is missing" and "no Rig is
/// configured" are two different failures with two different remedies.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to hear.
    take: PathBuf,

    /// The Rig to hear it through: a soundfont path.
    #[arg(long)]
    rig: Option<PathBuf>,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> battuta::Result<()> {
    let audition = battuta::rig::play(&args.take, args.rig, &mut std::io::stderr())?;

    if args.json {
        println!("{}", crate::json(&audition));
    }
    Ok(())
}
