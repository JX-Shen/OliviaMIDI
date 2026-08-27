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
/// `--bars 5:8` hears a passage instead of the whole Take: 1-indexed, both ends
/// included, and the same Bar range `mid inspect --bars` reads, refused in the
/// same words when it cannot be honoured. The passage starts playing at once
/// rather than after the Bars in front of it, it is heard at the Take's own
/// tempo and time signature, and it carries whatever the Take had already set
/// by then — the program, the controllers. Your Take is not touched: what
/// FluidSynth is handed is a temporary file, and nothing is left behind when
/// the command ends, Ctrl-C included.
///
/// Playback is FluidSynth, found on PATH. "FluidSynth is missing" and "no Rig is
/// configured" are two different failures with two different remedies.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to hear.
    take: PathBuf,

    /// The passage to hear, as FIRST:LAST — 1-indexed, both ends included.
    #[arg(long, value_name = "FIRST:LAST")]
    bars: Option<battuta::BarRange>,

    /// The Rig to hear it through: a soundfont path.
    #[arg(long)]
    rig: Option<PathBuf>,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> battuta::Result<()> {
    // The library decides *that* the Rig is disclosed and when; this decides
    // what it says and where it goes. Stderr, so that `--json` on stdout stays
    // one JSON document, and so that a shell pipeline cannot drop the
    // attribution by redirecting the payload. See ADR-0009.
    let audition = battuta::rig::play(
        &args.take,
        args.bars,
        args.rig,
        &mut |rig: &battuta::Rig| {
            eprintln!("rig: {}", rig.soundfont.display());
        },
    )?;

    if args.json {
        println!("{}", crate::json(&audition));
    }
    Ok(())
}
