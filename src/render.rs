//! A Take turned into a WAV file, with what it was made from written inside.
//!
//! `play` discloses the Rig at the moment it runs. A file has no such moment:
//! it outlives the command and the terminal, and may be heard anywhere, by
//! anyone, long after. So the attribution travels in the file — a RIFF
//! `LIST`/`INFO` chunk, in `ICMT` — and names what it was made from by digest
//! and version, never by a path that means something on one machine only.
//!
//! The attribution is a claim, not evidence. Any tool may strip it, and the
//! check is to render again from it and compare. What it does not cover is
//! stated in `mid render --help`, and why it does not is #70.

use crate::bars::BarRange;
use crate::edit::same_file;
use crate::error::{Error, Result};
use crate::rig::Rig;
use crate::take::Take;
use crate::temporary::{TemporaryFile, TemporaryTake};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// One rendering: which Take, how much of it, through which Rig, into which
/// file, and what that file says it was made from.
///
/// Not an `Audition`. Nobody has heard a rendered file until somebody does,
/// and the type is kept apart so that no caller can file one as the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rendering {
    pub take: PathBuf,
    /// The passage rendered, when one was asked for; the whole Take otherwise.
    pub bars: Option<BarRange>,
    pub rig: PathBuf,
    pub output: PathBuf,
    pub attribution: Attribution,
}

/// What a rendered file is attributed to, as written into it.
///
/// Every field is something another machine can check: a digest of a file it
/// can obtain, or a version it can install. The soundfont is named by its file
/// name beside its digest, because a name helps a reader find the bank and the
/// digest is what says it is the same one; the path it was read from is in
/// `Rendering::rig` and deliberately not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Attribution {
    /// The `battuta` that rendered it, which fixes the command line FluidSynth
    /// was handed and so every setting not otherwise named here.
    pub battuta: String,
    pub take_sha256: String,
    pub soundfont: String,
    pub soundfont_sha256: String,
    /// FluidSynth's runtime version, as it states it.
    pub fluidsynth: String,
}

/// The whole of `mid render`: resolve the Rig as `play` does, have FluidSynth
/// write the audio, write the attribution into it, and rename it onto
/// `output`.
///
/// `disclose` is handed the resolved Rig at the moment `play` hands it over:
/// once FluidSynth has started, so it names the Rig that *was* used.
///
/// `output` is never written through. The audio goes to a file beside it and is
/// renamed on, so it holds what it held before or the whole of the new file,
/// and a path that also names the Take or the soundfont is refused before
/// anything is rendered.
pub fn render(
    take: &Path,
    bars: Option<BarRange>,
    rig: Option<PathBuf>,
    output: &Path,
    disclose: &mut dyn FnMut(&Rig),
) -> Result<Rendering> {
    if same_file(take, output) {
        return Err(Error::RenderOverInput {
            output: output.to_path_buf(),
            input: take.to_path_buf(),
        });
    }
    // The passage is cut before the Rig is resolved, as `play` cuts it, so a
    // Bar range that cannot be honoured is refused in the same words whether or
    // not a Rig is configured here.
    let passage = bars
        .map(|bars| Take::read(take)?.passage(bars))
        .transpose()?;
    let rig = Rig::resolve(rig)?;
    if same_file(&rig.soundfont, output) {
        return Err(Error::RenderOverInput {
            output: output.to_path_buf(),
            input: rig.soundfont.clone(),
        });
    }

    let attribution = Attribution {
        battuta: env!("CARGO_PKG_VERSION").to_string(),
        take_sha256: sha256_of(take)?,
        soundfont: rig
            .soundfont
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        soundfont_sha256: sha256_of(&rig.soundfont)?,
        fluidsynth: fluidsynth_version()?,
    };

    let passage = passage
        .map(|passage| TemporaryTake::holding(&passage))
        .transpose()?;
    let rendered = passage.as_ref().map_or(take, TemporaryTake::path);

    let written = |source: std::io::Error| Error::Write {
        path: output.to_path_buf(),
        source,
    };
    let wav = TemporaryFile::beside(output, ".wav").map_err(written)?;

    let mut child = Command::new("fluidsynth")
        .args(rig.fluidsynth_render_args(rendered, wav.path()))
        .spawn()
        .map_err(|source| match source.kind() {
            std::io::ErrorKind::NotFound => Error::NoFluidsynth,
            _ => Error::FluidsynthUnusable(source),
        })?;
    disclose(&rig);
    let status = child.wait().map_err(Error::FluidsynthUnusable)?;
    if !status.success() {
        return Err(Error::FluidsynthFailed(status));
    }

    let rendering = Rendering {
        take: take.to_path_buf(),
        bars,
        rig: rig.soundfont,
        output: output.to_path_buf(),
        attribution,
    };

    let audio = std::fs::read(wav.path()).map_err(written)?;
    let attributed = with_attribution(audio, &rendering)
        .ok_or_else(|| Error::RenderNotWav(output.to_path_buf()))?;
    std::fs::write(wav.path(), attributed).map_err(written)?;
    {
        use std::os::unix::fs::PermissionsExt;
        // Created private to its owner, as every temporary file is; a rendered
        // file is meant to be opened by something else.
        std::fs::set_permissions(wav.path(), std::fs::Permissions::from_mode(0o644))
            .map_err(written)?;
    }
    wav.persist(output).map_err(written)?;
    Ok(rendering)
}

/// The text written into `ICMT`: one fact a line, each as `name value`.
///
/// The passage is always stated, `all` included: a line that is absent reads
/// as a field nobody filled in, and a whole Take is a fact about this file.
/// No timestamp, so that rendering one Take twice writes one file twice.
pub fn attribution_text(rendering: &Rendering) -> String {
    let attribution = &rendering.attribution;
    let bars = rendering
        .bars
        .map_or_else(|| "all".to_string(), |bars| bars.to_string());
    format!(
        "rendered by battuta {}\n\
         take sha256:{}\n\
         bars {}\n\
         soundfont {} sha256:{}\n\
         fluidsynth {}",
        attribution.battuta,
        attribution.take_sha256,
        bars,
        attribution.soundfont,
        attribution.soundfont_sha256,
        attribution.fluidsynth,
    )
}

/// `audio` with a `LIST`/`INFO` chunk after everything it already holds, and
/// the RIFF size made to count it. `None` when it is not a RIFF WAVE file.
///
/// Appended rather than inserted, so that every byte FluidSynth wrote stays
/// where FluidSynth wrote it. `ISFT` names the software, and is the field other
/// tools overwrite with their own; the attribution is in `ICMT`, which they
/// carry over.
fn with_attribution(mut audio: Vec<u8>, rendering: &Rendering) -> Option<Vec<u8>> {
    if audio.len() < 12 || &audio[0..4] != b"RIFF" || &audio[8..12] != b"WAVE" {
        return None;
    }
    // Every chunk starts on an even offset.
    if audio.len() % 2 == 1 {
        audio.push(0);
    }
    let mut info = b"INFO".to_vec();
    info.extend(info_field(
        b"ISFT",
        &format!("battuta {}", rendering.attribution.battuta),
    ));
    info.extend(info_field(b"ICMT", &attribution_text(rendering)));
    audio.extend(b"LIST");
    audio.extend((info.len() as u32).to_le_bytes());
    audio.extend(info);
    let riff_size = u32::try_from(audio.len() - 8).ok()?;
    audio[4..8].copy_from_slice(&riff_size.to_le_bytes());
    Some(audio)
}

/// One `INFO` sub-chunk: its text NUL-terminated, padded to an even length.
fn info_field(id: &[u8; 4], text: &str) -> Vec<u8> {
    let mut data = text.as_bytes().to_vec();
    data.push(0);
    let size = data.len() as u32;
    if data.len() % 2 == 1 {
        data.push(0);
    }
    let mut field = id.to_vec();
    field.extend(size.to_le_bytes());
    field.extend(data);
    field
}

/// A file's SHA-256, in lower-case hex: what `shasum -a 256` prints.
fn sha256_of(path: &Path) -> Result<String> {
    let read = |source: std::io::Error| Error::Read {
        path: path.to_path_buf(),
        source,
    };
    let mut file = std::fs::File::open(path).map_err(read)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(read)?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// The version FluidSynth states for itself at run time.
///
/// Asked of the `fluidsynth` on PATH, the one about to render, rather than of
/// anything installed beside it: the runtime version is the library that does
/// the synthesis.
fn fluidsynth_version() -> Result<String> {
    let answered = Command::new("fluidsynth")
        .arg("--version")
        .output()
        .map_err(|source| match source.kind() {
            std::io::ErrorKind::NotFound => Error::NoFluidsynth,
            _ => Error::FluidsynthUnusable(source),
        })?;
    let text = String::from_utf8_lossy(&answered.stdout).into_owned();
    text.lines()
        .find_map(|line| line.split_once("runtime version "))
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .map(str::to_string)
        .ok_or(Error::FluidsynthVersionUnreadable(text))
}
