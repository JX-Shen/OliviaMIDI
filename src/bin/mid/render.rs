use std::path::PathBuf;

/// Render a Take to a WAV file, through a Rig you chose.
///
/// A rendered file has not been heard. It is what the Rig would play, written
/// down; nobody has listened to it until somebody does. Do not report a render
/// as a listen.
///
/// The Rig is resolved as `mid play` resolves it: `--rig`, then
/// `BATTUTA_SOUNDFONT`, then failure, with no fallback to a system soundfont.
/// The Rig used is stated on stderr, and in the payload under `--json`. No flag
/// suppresses it.
///
/// The file carries its own attribution, because it outlives this command. A
/// RIFF LIST/INFO chunk holds, in its comment field (ICMT):
///
///     rendered by battuta <version>
///     take sha256:<digest of the Take file>
///     bars <FIRST:LAST, or all>
///     soundfont <file name> sha256:<digest of the soundfont>
///     fluidsynth <runtime version>
///
/// The battuta version fixes what `mid` hands FluidSynth: exactly what `mid
/// play` hands it, plus the file to write. The soundfont's path is stated on
/// stderr but not written into the file, which is meant to leave this machine.
///
/// The attribution is a claim, not proof. Other tools may strip it — macOS's
/// `afconvert` does — and a file without it is unattributed, never attributed
/// to whatever Rig is configured now. To check one, render the same Take and
/// passage through the same soundfont and versions, and compare the files'
/// digests.
///
/// Not in the attribution: FluidSynth's own configuration files. FluidSynth
/// reads a user configuration file (`~/.fluidsynth`), and may read a system
/// one, whatever `mid` passes it, and either can change what is rendered. `mid`
/// does not record them; one that differs shows up as a differing digest when
/// the file is rendered again.
///
/// `--bars 5:8` renders a passage exactly as `mid play --bars` plays one: the
/// same Bar range, carrying whatever the Take had already set by then, and
/// refused in the same words — the collapse `mid play --help` describes
/// included.
///
/// `-o` is required. The file is written beside its destination and renamed
/// onto it, so the destination holds what it held before or the whole rendered
/// file, and never the Take or the soundfont. Nothing temporary is left behind,
/// Ctrl-C included. The output's path is printed on stdout.
///
/// Rendering is FluidSynth, found on PATH, as for `mid play`.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to render.
    take: PathBuf,

    /// Where to write the WAV file. Required.
    #[arg(short = 'o', long = "output")]
    output: PathBuf,

    /// The passage to render, as FIRST:LAST — 1-indexed, both ends included.
    #[arg(long, value_name = "FIRST:LAST")]
    bars: Option<battuta::BarRange>,

    /// The Rig to render it through: a soundfont path.
    #[arg(long)]
    rig: Option<PathBuf>,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> battuta::Result<()> {
    // Disclosed on stderr in `play`'s words, at the moment the library hands
    // it over. See ADR-0005.
    let rendering = battuta::render::render(
        &args.take,
        args.bars,
        args.rig,
        &args.output,
        &mut |rig: &battuta::Rig| {
            eprintln!("{}", crate::wording::rig(rig));
        },
    )?;

    eprintln!("{}", crate::wording::rendered(&rendering));
    if args.json {
        println!("{}", crate::json(&rendering));
    } else {
        println!("{}", rendering.output.display());
    }
    Ok(())
}
