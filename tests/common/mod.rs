//! Shared scaffolding for the process-boundary tests.
//!
//! There is one seam in this suite: tests build `mid` and run it. Several
//! already-specified behaviours — which stream the Rig disclosure goes to, exit
//! codes, the PATH lookup for FluidSynth — are not observable at any lower one.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The reference Take. Never written to by any test.
pub const FIXTURE: &str = "fixtures/olivia.mid";

/// The Take built to collide. Never written to by any test either.
///
/// `FIXTURE` cannot produce a collision — all 36 of its notes are distinct in
/// track, channel, pitch and start Tick — so ADR-0002's occurrence index was
/// only ever exercised on collisions `apply` had just created. This one arrives
/// already collided, and from outside: its events are hand-written, so the order
/// of two note-ons sharing a Tick is a fact about the file rather than about our
/// own builder. See `tests/stacked.rs`, which states its contents.
pub const STACKED: &str = "fixtures/stacked.mid";

/// The Take built to carry an orchestration. Never written to by any test.
///
/// Neither `FIXTURE` nor `STACKED` states a Program at all — both are the case
/// #12's sixth criterion is about, which makes them useless for the other six.
/// This one is four Bars of 3/4 at 480 PPQ, three voice tracks, and states:
///
/// - channel 0, Tick 0, program 40 — in force before the first note
/// - channel 1, Tick 2880 (Bar 3 Beat 1), program 60 — a switch part way in
/// - channel 2, nothing at all — a channel with notes and no Program
///
/// Each channel has one note per Bar, so every Bar range holds all three.
/// See `tests/program.rs`, which states its contents where it uses them.
pub const ORCHESTRATED: &str = "fixtures/orchestrated.mid";

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
/// This is the *event-level* reading of "identical" that ADR-0003 settles on:
/// same tracks, same events, same delta times. Byte-level encoding choices —
/// running status, how a delta time is packed into a varint — are the writer's,
/// not the Piece's.
pub fn event_stream(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("Take is readable");
    let smf = midly::Smf::parse(&bytes).expect("Take parses");
    format!("{:?}\n{:#?}", smf.header, smf.tracks)
}

/// Every program change a Take states, as (track, tick, channel, program), in
/// the order the file lists them.
///
/// Read straight from the file, so that a test asking whether `apply` left the
/// orchestration alone is not asking the same code that reports it. `mid inspect`
/// answers the state; this answers the events.
pub fn stated_programs(path: &Path) -> Vec<(usize, u32, u8, u8)> {
    let bytes = std::fs::read(path).expect("Take is readable");
    let smf = midly::Smf::parse(&bytes).expect("Take parses");
    let mut stated = Vec::new();
    for (index, track) in smf.tracks.iter().enumerate() {
        let mut tick = 0u32;
        for event in track {
            tick += event.delta.as_int();
            if let midly::TrackEventKind::Midi {
                channel,
                message: midly::MidiMessage::ProgramChange { program },
            } = event.kind
            {
                stated.push((index, tick, channel.as_int(), program.as_int()));
            }
        }
    }
    stated
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

/// Every note event of a Take, in the order the file lists them, as
/// (tick, whether it strikes or releases, pitch).
///
/// Read straight from the file, because nothing `mid` reports can see this.
/// Which of two events sharing a Tick comes first does not change what a reader
/// pairs the notes into — it hands a release to the oldest note of that channel
/// and pitch either way — and it is exactly what a synthesiser acts on.
pub fn note_events(path: &Path) -> Vec<(u32, &'static str, u8)> {
    let bytes = std::fs::read(path).expect("Take is readable");
    let smf = midly::Smf::parse(&bytes).expect("Take parses");
    let mut found = Vec::new();
    for track in &smf.tracks {
        let mut tick = 0u32;
        for event in track {
            tick += event.delta.as_int();
            let midly::TrackEventKind::Midi { message, .. } = event.kind else {
                continue;
            };
            // A note-on at velocity 0 is a note-off; the format uses the two
            // interchangeably and so does this.
            match message {
                midly::MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                    found.push((tick, "strikes", key.as_int()));
                }
                midly::MidiMessage::NoteOff { key, .. }
                | midly::MidiMessage::NoteOn { key, .. } => {
                    found.push((tick, "releases", key.as_int()));
                }
                _ => {}
            }
        }
    }
    found
}

pub fn note_ids(json: &str) -> Vec<String> {
    notes(json)
        .iter()
        .map(|note| note["id"].as_str().expect("id is a string").to_string())
        .collect()
}

/// The notes out of an `inspect --json` payload.
///
/// The payload is an object as of 0.1.1 — a Program belongs to a channel and
/// not to a note, so there was nowhere in a bare array of notes to put one.
/// `programs` reaches the other half.
pub fn notes(json: &str) -> Vec<serde_json::Value> {
    listing(json)["notes"]
        .as_array()
        .expect("the payload has notes")
        .clone()
}

/// What `inspect --json` says each channel is on, and where the passage states
/// another: the `programs` and `stated_programs` of the payload.
pub fn programs(json: &str) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let listing = listing(json);
    let array = |key: &str| {
        listing[key]
            .as_array()
            .unwrap_or_else(|| panic!("the payload has {key}"))
            .clone()
    };
    (array("programs"), array("stated_programs"))
}

/// What `inspect --json` says each channel holds for a Controller, and where the
/// passage states another: the `controllers` and `stated_controllers` of the
/// payload.
pub fn controllers(json: &str) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let listing = listing(json);
    let array = |key: &str| {
        listing[key]
            .as_array()
            .unwrap_or_else(|| panic!("the payload has {key}"))
            .clone()
    };
    (array("controllers"), array("stated_controllers"))
}

fn listing(json: &str) -> serde_json::Value {
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

/// An Edit Set holding whatever Edits a test spells out, written to `dir` under
/// `name`. The kinds are written at the call site rather than assembled by a
/// helper per kind, so a test reads like the `edits.json` an agent would write.
pub fn edit_set(dir: &Path, name: &str, edits: &str) -> PathBuf {
    let path = dir.join(format!("{name}.json"));
    write(&path, &format!(r#"{{ "edits": [ {edits} ] }}"#));
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
    use midly::num::{u4, u7};
    let setting: Vec<(u32, midly::TrackEventKind<'static>)> = programs
        .iter()
        .map(|&(tick, program)| {
            (
                tick,
                midly::TrackEventKind::Midi {
                    channel: u4::new(0),
                    message: midly::MidiMessage::ProgramChange {
                        program: u7::new(program),
                    },
                },
            )
        })
        .collect();
    build_take_setting(path, ppq, stated, &setting, notes)
}

/// A Controller a built Take states on its voice track: (tick, controller,
/// value).
pub type StatedController = (u32, u8, u8);

/// The same, with control changes on the voice track — the value a passage
/// beginning part way through the Take has to inherit to be described as it
/// sounds (ADR-0007).
pub fn build_take_with_controllers(
    path: &Path,
    ppq: u16,
    stated: &[StatedTimeSignature],
    controllers: &[StatedController],
    notes: &[NoteSpec],
) -> PathBuf {
    use midly::num::{u4, u7};
    let setting: Vec<(u32, midly::TrackEventKind<'static>)> = controllers
        .iter()
        .map(|&(tick, controller, value)| {
            (
                tick,
                midly::TrackEventKind::Midi {
                    channel: u4::new(0),
                    message: midly::MidiMessage::Controller {
                        controller: u7::new(controller),
                        value: u7::new(value),
                    },
                },
            )
        })
        .collect();
    build_take_setting(path, ppq, stated, &setting, notes)
}

/// The same again, with whatever events a test names on the voice track.
///
/// #4's carry/leave-behind rule is one statement about a dozen event
/// kinds, so the kinds are a list here rather than a builder parameter each.
pub fn build_take_setting(
    path: &Path,
    ppq: u16,
    stated: &[StatedTimeSignature],
    setting: &[(u32, midly::TrackEventKind<'static>)],
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

    let mut voice: Vec<(u32, TrackEventKind<'static>)> = setting.to_vec();
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
        .chain(setting.iter().map(|&(tick, _)| tick))
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

/// A Take whose delta times are each writable and whose running total is not.
///
/// A delta time is 28 bits and a track may hold any number of them, so a file
/// can be built entirely out of encodable gaps and still run past the largest
/// absolute Tick `battuta` holds. Assembled byte by byte rather than through
/// `build_take`, because every builder places its events at absolute Ticks
/// first and so cannot express this file at all.
pub fn build_take_past_the_tick_range(path: &Path) -> PathBuf {
    /// The largest delta time the format holds, as the variable-length bytes
    /// the format writes it in.
    const LARGEST_DELTA: [u8; 4] = [0xff, 0xff, 0xff, 0x7f];

    let mut track = Vec::new();
    // Seventeen maximal gaps. Sixteen of them still fit a `u32`; the
    // seventeenth is what takes the total past it.
    for _ in 0..16 {
        track.extend_from_slice(&LARGEST_DELTA);
        track.extend_from_slice(&[0xff, 0x01, 0x01, b'x']); // a one-byte text event
    }
    track.extend_from_slice(&LARGEST_DELTA);
    track.extend_from_slice(&[0xff, 0x2f, 0x00]); // end of track

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&6u32.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes()); // format 0
    bytes.extend_from_slice(&1u16.to_be_bytes()); // one track
    bytes.extend_from_slice(&480u16.to_be_bytes()); // ticks per quarter note
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(
        &u32::try_from(track.len())
            .expect("a small track")
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&track);

    std::fs::write(path, bytes).expect("built Take is writable");
    path.to_path_buf()
}

/// The stdout of a `mid` run that must succeed.
///
/// Human output is asserted whole rather than a line at a time: what is under
/// test is the layout — which facts are on a line, in what order, and how the
/// lines line up with each other — and a `contains` on one line of it cannot
/// see any of that.
pub fn human_output(args: &[&str]) -> String {
    stdout(args)
}

/// The stdout of a `mid ... --json` run that must succeed.
///
/// The same run as `human_output`, under the name that says which of the two
/// surfaces is under test: one is a layout a person reads, the other a payload
/// an agent parses, and a test should say which it is asserting.
pub fn json_output(args: &[&str]) -> String {
    stdout(args)
}

fn stdout(args: &[&str]) -> String {
    let output = mid().args(args).output().expect("mid runs");
    assert!(
        output.status.success(),
        "mid {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout is UTF-8")
}
