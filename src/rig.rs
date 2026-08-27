use crate::bars::BarRange;
use crate::error::{Error, Result};
use crate::take::Take;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;

/// The apparatus a Take is heard through. In V0 that is one soundfont for the
/// whole Piece; the name is `Rig` rather than `Soundfont` because V1's named
/// Rigs will carry more and must not need a rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rig {
    pub soundfont: PathBuf,
}

/// The environment variable that names a Rig when `--rig` does not.
pub const SOUNDFONT_ENV: &str = "BATTUTA_SOUNDFONT";

impl Rig {
    /// `--rig`, then `BATTUTA_SOUNDFONT`, then fail (ADR-0003).
    ///
    /// There is no third step. Falling back to a system soundfont would make an
    /// audition unattributable, and an unattributable audition is an aesthetic
    /// judgement that is wrong without anyone being able to tell.
    pub fn resolve(flag: Option<PathBuf>) -> Result<Rig> {
        let soundfont = flag
            .or_else(|| std::env::var_os(SOUNDFONT_ENV).map(PathBuf::from))
            .filter(|path| !path.as_os_str().is_empty())
            .ok_or(Error::NoRig)?;
        if !soundfont.exists() {
            return Err(Error::RigMissing(soundfont));
        }
        Ok(Rig { soundfont })
    }

    /// The command line handed to FluidSynth — the whole of what playback does.
    /// The suite observes it through a fake `fluidsynth` on the child's PATH
    /// rather than through this function, so that the real lookup stays in the
    /// tested path.
    pub fn fluidsynth_args(&self, take: &Path) -> Vec<PathBuf> {
        vec![
            PathBuf::from("-i"), // no interactive shell
            PathBuf::from("-n"), // no MIDI input
            PathBuf::from("-q"), // no banner on stdout
            self.soundfont.clone(),
            take.to_path_buf(),
        ]
    }
}

/// One audition: which Take was heard, how much of it, and through which Rig.
///
/// Attribution is the whole point of this type. A comparison made today is only
/// still meaningful tomorrow if the Rig it was heard through is on the record —
/// and so is the passage, because an opinion formed about four Bars, filed
/// against the whole Piece, is a record of a judgement nobody made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Audition {
    pub take: PathBuf,
    /// The passage heard, when one was asked for; the whole Take otherwise.
    pub bars: Option<BarRange>,
    pub rig: PathBuf,
}

/// The whole of `mid play`: resolve the Rig, hand the Take to FluidSynth, and
/// state what was heard.
///
/// Synthesis is not this project's problem (ADR-0001), so `mid` locates
/// `fluidsynth` on PATH and shells out. Not finding it and not having a Rig are
/// two different failures with two different remedies, and are never merged
/// into one "playback failed".
pub fn play(
    take: &Path,
    bars: Option<BarRange>,
    rig: Option<PathBuf>,
    disclose: &mut dyn std::io::Write,
) -> Result<Audition> {
    // The passage is cut before the Rig is resolved. A Bar range that cannot be
    // honoured is a mistake in the command rather than in the machine, and it
    // is refused in the words `inspect --bars` refuses it in whether or not a
    // Rig happens to be configured here. Nothing about how a Rig is resolved or
    // disclosed changes: for a whole Take there is still nothing before it.
    let passage = bars
        .map(|bars| Take::read(take)?.passage(bars))
        .transpose()?;
    let rig = Rig::resolve(rig)?;

    // Written out only now that there is something to play it through.
    let passage = passage.map(hold_for_playback).transpose()?;
    let heard = passage.as_ref().map_or(take, NamedTempFile::path);

    let mut child = Command::new("fluidsynth")
        .args(rig.fluidsynth_args(heard))
        .spawn()
        // Not finding FluidSynth and failing to start the one that is there are
        // different problems with different remedies, and neither is a problem
        // with the Take — reporting either as a write failure would name the
        // wrong thing to go and fix.
        .map_err(|source| match source.kind() {
            std::io::ErrorKind::NotFound => Error::NoFluidsynth,
            _ => Error::FluidsynthUnusable(source),
        })?;

    // Disclosed only once FluidSynth has actually started, so the line says
    // which Rig was used rather than which one would have been.
    let _ = writeln!(disclose, "rig: {}", rig.soundfont.display());

    let status = child.wait().map_err(Error::FluidsynthUnusable)?;
    if !status.success() {
        return Err(Error::FluidsynthFailed(status));
    }

    // The Take named is the user's, never the temporary one: the passage is a
    // detail of how it was played, not a file anybody has. `passage` is dropped
    // on the way out of this function, and the temporary Take with it.
    Ok(Audition {
        take: take.to_path_buf(),
        bars,
        rig: rig.soundfont,
    })
}

/// The passage, put somewhere it can be handed to FluidSynth and then
/// forgotten.
///
/// FluidSynth plays a file from its beginning and has no range playback, so the
/// only way to hear four Bars is to give it four Bars. The file lives in the
/// temporary directory and is deleted when the returned handle drops: writing
/// it beside the user's Take would leave something that looks like a Take they
/// made, and it is not one.
fn hold_for_playback(passage: Take) -> Result<NamedTempFile> {
    let file = tempfile::Builder::new()
        .prefix("battuta-passage-")
        .suffix(".mid")
        .tempfile()
        .map_err(Error::PassageUnwritable)?;
    // Creating the file and filling it are one condition with one remedy, and
    // the user is never told the temporary file's name: it is not a file they
    // have, so it is not a file they can go and fix.
    passage.write(file.path()).map_err(|error| match error {
        Error::Write { source, .. } => Error::PassageUnwritable(source),
        // `Take::write` fails no other way; keeping the value rather than
        // renaming it is the safe direction if it ever grows one.
        other => other,
    })?;
    Ok(file)
}
