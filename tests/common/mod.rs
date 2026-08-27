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

/// The program changes a Take states, as (tick, program). Read straight from
/// the file, like `event_stream`: no command reports them, and a passage has to
/// carry the ones set before it.
pub fn program_changes(path: &Path) -> Vec<(u32, u8)> {
    let bytes = std::fs::read(path).expect("Take is readable");
    let smf = midly::Smf::parse(&bytes).expect("Take parses");
    let mut stated = Vec::new();
    for track in &smf.tracks {
        let mut tick = 0u32;
        for event in track {
            tick += event.delta.as_int();
            if let midly::TrackEventKind::Midi {
                message: midly::MidiMessage::ProgramChange { program },
                ..
            } = event.kind
            {
                stated.push((tick, program.as_int()));
            }
        }
    }
    stated
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

/// `mid info --json`, parsed. The Take a passage was cut from and the passage
/// itself are described by the same command, so a test can ask both.
pub fn info_json(path: &Path) -> serde_json::Value {
    let output = mid()
        .args(["info", "--json"])
        .arg(path)
        .output()
        .expect("mid runs");
    assert!(output.status.success(), "info failed on {path:?}");
    serde_json::from_slice(&output.stdout).expect("info --json is JSON")
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
/// adding a flag to the product that makes playback skippable. The fake copies
/// the file it was handed as well as logging its argv, because a passage is
/// handed over as a temporary Take that is gone by the time `mid` returns.
pub struct FakeFluidsynth {
    pub dir: PathBuf,
    pub log: PathBuf,
    /// A copy of the Take the fake was given, taken while it still existed.
    pub handed: PathBuf,
}

pub fn fake_fluidsynth(dir: &Path) -> FakeFluidsynth {
    fake(dir, "exit 0")
}

/// A fake that keeps "playing" instead of returning, so that a test can catch
/// `mid` in the middle of an audition — which is where an interrupt lands.
pub fn lingering_fake_fluidsynth(dir: &Path) -> FakeFluidsynth {
    fake(dir, "/bin/sleep 30")
}

/// Absolute paths for `/bin/cp` and `/bin/sleep`: the child's PATH is the
/// fake's own directory alone, so that `fluidsynth` can only be the fake and
/// nothing else is reachable on it.
fn fake(dir: &Path, last_act: &str) -> FakeFluidsynth {
    let bin_dir = dir.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("scratch dir is writable");
    let log = dir.join("fluidsynth-argv");
    let handed = dir.join("handed.mid");
    let script = format!(
        "#!/bin/sh\n\
         for arg in \"$@\"; do last=\"$arg\"; done\n\
         /bin/cp \"$last\" \"{}\"\n\
         printf '%s\\n' \"$@\" > \"{}\"\n\
         {last_act}\n",
        handed.display(),
        log.display()
    );
    let path = bin_dir.join("fluidsynth");
    std::fs::write(&path, script).expect("fake is writable");
    make_executable(&path);
    FakeFluidsynth {
        dir: bin_dir,
        log,
        handed,
    }
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

/// A time signature a built Take states: (tick, numerator, denominator).
pub type StatedTimeSignature = (u32, u8, u8);

/// A note a built Take contains: (start, duration, pitch).
pub type NoteSpec = (u32, u32, u8);

/// A program change a built Take states on its voice track: (tick, program).
pub type StatedProgram = (u32, u8);

/// Write a small purpose-built Take, shaped like an ordinary export: a conductor
/// track carrying the time signature and a second track carrying the notes.
///
/// The fixture cannot exercise everything Bar semantics has to answer for — it
/// states one time signature at Tick 0, and not one of its notes crosses a Bar
/// line. Rather than
/// commit a second opaque `.mid`, a test that needs such a Take states it here
/// in readable terms and builds it.
pub fn build_take(
    path: &Path,
    ppq: u16,
    stated: &[StatedTimeSignature],
    notes: &[NoteSpec],
) -> PathBuf {
    build_take_with_programs(path, ppq, stated, &[], notes)
}

/// The same, with program changes on the voice track — the state a passage
/// beginning part way through the Take has to inherit to sound like itself.
pub fn build_take_with_programs(
    path: &Path,
    ppq: u16,
    stated: &[StatedTimeSignature],
    programs: &[StatedProgram],
    notes: &[NoteSpec],
) -> PathBuf {
    use midly::num::{u15, u24, u28, u4, u7};
    use midly::{
        Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
    };

    /// Absolute ticks in, delta times out.
    fn deltas(mut events: Vec<(u32, TrackEventKind<'static>)>) -> Vec<TrackEvent<'static>> {
        events.sort_by_key(|(tick, _)| *tick);
        let mut previous = 0u32;
        events
            .into_iter()
            .map(|(tick, kind)| {
                let event = TrackEvent {
                    delta: u28::new(tick - previous),
                    kind,
                };
                previous = tick;
                event
            })
            .collect()
    }

    let mut conductor: Vec<(u32, TrackEventKind<'static>)> = stated
        .iter()
        .map(|&(tick, numerator, denominator)| {
            let power = u8::try_from(denominator.trailing_zeros()).expect("a note value");
            (
                tick,
                TrackEventKind::Meta(MetaMessage::TimeSignature(numerator, power, 24, 8)),
            )
        })
        .collect();
    conductor.push((
        0,
        TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000))),
    ));
    let conductor_end = stated.iter().map(|&(tick, ..)| tick).max().unwrap_or(0);
    conductor.push((conductor_end, TrackEventKind::Meta(MetaMessage::EndOfTrack)));

    let mut voice: Vec<(u32, TrackEventKind<'static>)> = Vec::new();
    for &(tick, program) in programs {
        voice.push((
            tick,
            TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::ProgramChange {
                    program: u7::new(program),
                },
            },
        ));
    }
    for &(start, duration, pitch) in notes {
        voice.push((
            start,
            TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOn {
                    key: u7::new(pitch),
                    vel: u7::new(64),
                },
            },
        ));
        voice.push((
            start + duration,
            TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOff {
                    key: u7::new(pitch),
                    vel: u7::new(0),
                },
            },
        ));
    }
    let voice_end = notes
        .iter()
        .map(|&(start, duration, _)| start + duration)
        .chain(programs.iter().map(|&(tick, _)| tick))
        .max()
        .unwrap_or(0);
    voice.push((voice_end, TrackEventKind::Meta(MetaMessage::EndOfTrack)));

    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(ppq))),
        tracks: vec![deltas(conductor), deltas(voice)],
    };
    smf.save(path).expect("built Take is writable");
    path.to_path_buf()
}
