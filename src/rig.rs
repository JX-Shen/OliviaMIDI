use crate::error::{Error, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// One audition: which Take was heard, and which Rig it was heard through.
///
/// Attribution is the whole point of this type. A comparison made today is only
/// still meaningful tomorrow if the Rig it was heard through is on the record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Audition {
    pub take: PathBuf,
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
    rig: Option<PathBuf>,
    disclose: &mut dyn std::io::Write,
) -> Result<Audition> {
    let rig = Rig::resolve(rig)?;
    let mut child = Command::new("fluidsynth")
        .args(rig.fluidsynth_args(take))
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

    Ok(Audition {
        take: take.to_path_buf(),
        rig: rig.soundfont,
    })
}
