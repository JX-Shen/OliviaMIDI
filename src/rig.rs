use crate::bars::BarRange;
use crate::error::{Error, Result};
use crate::take::Take;
use crate::temporary::TemporaryTake;
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
    /// `--rig`, then `BATTUTA_SOUNDFONT`, then fail. The Rig is never chosen implicitly (`CHARTER.md`).
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
/// disclose which Rig was used.
///
/// Synthesis is not this project's problem (ADR-0001), so `mid` locates
/// `fluidsynth` on PATH and shells out. Not finding it and not having a Rig are
/// two different failures with two different remedies, and are never merged
/// into one "playback failed".
///
/// `disclose` is handed the resolved Rig once, at the only moment it can
/// truthfully be handed it: after FluidSynth has started, so it names the Rig
/// that *was* used rather than the one that would have been, and before the
/// audio ends, so an audition somebody interrupts has still been disclosed.
///
/// That moment is the library's, and so is the obligation — a caller cannot
/// reach an `Audition` without being told what it was heard through. The
/// sentence is not: `CHARTER.md` gives it to the binary, in as many words
/// ("`mid play` always states which Rig it used"), and a consumer that is not
/// `mid` has its own product to answer for. See ADR-0005.
pub fn play(
    take: &Path,
    bars: Option<BarRange>,
    rig: Option<PathBuf>,
    disclose: &mut dyn FnMut(&Rig),
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

    // Written out only now that there is something to play it through, and
    // written where nothing will find it afterwards.
    let passage = passage
        .map(|passage| TemporaryTake::holding(&passage))
        .transpose()?;
    let heard = passage.as_ref().map_or(take, TemporaryTake::path);

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

    // Disclosed only once FluidSynth has actually started, so what the caller
    // is told is which Rig was used rather than which one would have been.
    disclose(&rig);

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
