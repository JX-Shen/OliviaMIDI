//! Shared scaffolding for the process-boundary tests.
//!
//! There is one seam in this suite: tests build `mid` and run it. Several
//! already-specified behaviours — which stream the Rig disclosure goes to, exit
//! codes, the PATH lookup for FluidSynth — are not observable at any lower one.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The reference Take. Never written to by any test.
pub const FIXTURE: &str = "fixtures/olivia.mid";

pub fn mid() -> assert_cmd::Command {
    let mut command = assert_cmd::Command::cargo_bin("mid").expect("mid builds");
    // Nothing in this suite may be steered by whatever Rig the machine happens
    // to have configured.
    command.env_remove(battuta::rig::SOUNDFONT_ENV);
    command
}

pub fn fixture_bytes() -> Vec<u8> {
    std::fs::read(FIXTURE).expect("the fixture is readable")
}

pub fn write(path: &Path, contents: &str) {
    std::fs::write(path, contents).expect("scratch file is writable");
}

/// The Take's event stream, rendered so two Takes can be compared for identity.
///
/// This is the *event-level* reading of "identical" that ADR-0005 settles on:
/// same tracks, same events, same delta times. Byte-level encoding choices —
/// running status, how a delta time is packed into a varint — are the writer's,
/// not the Piece's.
pub fn event_stream(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("Take is readable");
    let smf = midly::Smf::parse(&bytes).expect("Take parses");
    format!("{:?}\n{:#?}", smf.header, smf.tracks)
}

pub fn note_ids(json: &str) -> Vec<String> {
    notes(json)
        .iter()
        .map(|note| note["id"].as_str().expect("id is a string").to_string())
        .collect()
}

pub fn notes(json: &str) -> Vec<serde_json::Value> {
    serde_json::from_str(json).expect("inspect --json is JSON")
}

pub fn inspect_json(path: &Path) -> String {
    let output = mid()
        .args(["inspect", "--json"])
        .arg(path)
        .output()
        .expect("mid runs");
    assert!(output.status.success(), "inspect failed on {path:?}");
    String::from_utf8(output.stdout).expect("stdout is UTF-8")
}

/// A stand-in for FluidSynth, placed on the child's PATH.
///
/// Playback is tested by watching what `mid` hands to the synthesiser, not by
/// adding a flag to the product that makes playback skippable.
pub struct FakeFluidsynth {
    pub dir: PathBuf,
    pub log: PathBuf,
}

pub fn fake_fluidsynth(dir: &Path) -> FakeFluidsynth {
    let bin_dir = dir.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("scratch dir is writable");
    let log = dir.join("fluidsynth-argv");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\nexit 0\n",
        log.display()
    );
    let path = bin_dir.join("fluidsynth");
    std::fs::write(&path, script).expect("fake is writable");
    make_executable(&path);
    FakeFluidsynth { dir: bin_dir, log }
}

fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path).expect("fake exists").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("fake is chmod-able");
}

/// An empty directory to use as PATH, so `fluidsynth` cannot be found at all.
pub fn empty_path(dir: &Path) -> PathBuf {
    let empty = dir.join("empty-bin");
    std::fs::create_dir_all(&empty).expect("scratch dir is writable");
    empty
}

/// A file standing in for a soundfont. Rig resolution only checks that the path
/// exists — FluidSynth is the fake, so nothing ever opens this.
pub fn fake_soundfont(path: &Path) -> PathBuf {
    std::fs::write(path, b"not a soundfont").expect("scratch file is writable");
    path.to_path_buf()
}

/// The identity of the first note in the fixture, read back out of `inspect`
/// rather than hardcoded: the tests refer to a note the way an agent would.
pub fn first_note_id() -> String {
    note_ids(&inspect_json(Path::new(FIXTURE)))
        .into_iter()
        .next()
        .expect("the fixture has notes")
}

/// An Edit Set with one `set_velocity` in it, written to `dir`.
pub fn set_velocity_edit_set(dir: &Path, id: &str, velocity: &str) -> PathBuf {
    let path = dir.join(format!("edits-{velocity}.json"));
    write(
        &path,
        &format!(
            r#"{{ "edits": [ {{ "kind": "set_velocity", "id": "{id}", "velocity": {velocity} }} ] }}"#
        ),
    );
    path
}

/// An Edit Set that asks for nothing.
pub fn empty_edit_set(dir: &Path) -> PathBuf {
    let path = dir.join("empty.json");
    write(&path, r#"{ "edits": [] }"#);
    path
}
