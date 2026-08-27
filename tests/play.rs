mod common;

use common::{fake_fluidsynth, mid, FIXTURE};

#[test]
fn hands_the_take_and_the_rig_to_fluidsynth() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));

    let output = mid()
        .args(["play", FIXTURE, "--rig"])
        .arg(&soundfont)
        .env("PATH", &fake.dir)
        .output()
        .expect("mid runs");
    assert!(output.status.success(), "play failed: {output:?}");

    let argv = std::fs::read_to_string(&fake.log).expect("the fake was invoked");
    let argv: Vec<&str> = argv.lines().collect();
    assert_eq!(
        argv,
        vec![
            "-i",
            "-n",
            "-q",
            soundfont.to_str().expect("path is UTF-8"),
            FIXTURE
        ]
    );
}

/// Every audition is attributable. The disclosure goes to stderr so that it can
/// never contaminate data being piped somewhere.
#[test]
fn states_the_rig_on_stderr_and_never_on_stdout() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));

    let output = mid()
        .args(["play", FIXTURE, "--rig"])
        .arg(&soundfont)
        .env("PATH", &fake.dir)
        .output()
        .expect("mid runs");

    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains(soundfont.to_str().expect("path is UTF-8")),
        "stderr did not state the Rig: {stderr}"
    );
    assert!(
        String::from_utf8(output.stdout)
            .expect("stdout is UTF-8")
            .is_empty(),
        "the Rig disclosure leaked onto stdout"
    );
}

#[test]
fn the_json_payload_carries_the_rig() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));

    let output = mid()
        .args(["play", FIXTURE, "--json", "--rig"])
        .arg(&soundfont)
        .env("PATH", &fake.dir)
        .output()
        .expect("mid runs");
    assert!(output.status.success());

    let audition: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("play --json is JSON");
    assert_eq!(audition["rig"], soundfont.to_str().expect("path is UTF-8"));
    assert_eq!(audition["take"], FIXTURE);
    assert_eq!(
        audition["bars"],
        serde_json::Value::Null,
        "the whole Take was heard, so no passage is named"
    );
}

#[test]
fn the_environment_names_a_rig_when_the_flag_does_not() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("from-env.sf2"));

    mid()
        .args(["play", FIXTURE])
        .env("PATH", &fake.dir)
        .env(battuta::rig::SOUNDFONT_ENV, &soundfont)
        .assert()
        .success()
        .stderr(predicates::str::contains(
            soundfont.to_str().expect("path is UTF-8"),
        ));
}

#[test]
fn the_flag_wins_over_the_environment() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let from_flag = common::fake_soundfont(&dir.path().join("from-flag.sf2"));
    let from_env = common::fake_soundfont(&dir.path().join("from-env.sf2"));

    mid()
        .args(["play", FIXTURE, "--rig"])
        .arg(&from_flag)
        .env("PATH", &fake.dir)
        .env(battuta::rig::SOUNDFONT_ENV, &from_env)
        .assert()
        .success()
        .stderr(predicates::str::contains(
            from_flag.to_str().expect("path is UTF-8"),
        ));
}

/// No Rig means no playback. There is no fallback to a system soundfont, to
/// FluidSynth's compiled-in default, or to whatever Homebrew installed next to
/// it — see ADR-0003.
#[test]
fn refuses_to_play_with_no_rig_configured() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());

    mid()
        .args(["play", FIXTURE])
        .env("PATH", &fake.dir)
        .assert()
        .failure()
        .stderr(predicates::str::contains("BATTUTA_SOUNDFONT"));

    assert!(
        !fake.log.exists(),
        "fluidsynth was invoked without a Rig chosen"
    );
}

/// Two conditions, two remedies, two messages — never one merged "playback
/// failed".
#[test]
fn a_missing_fluidsynth_is_a_different_failure_from_a_missing_rig() {
    let dir = tempfile::tempdir().expect("temp dir");
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));

    let output = mid()
        .args(["play", FIXTURE, "--rig"])
        .arg(&soundfont)
        .env("PATH", common::empty_path(dir.path()))
        .output()
        .expect("mid runs");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains("fluidsynth"), "{stderr}");
    assert!(
        !stderr.contains("BATTUTA_SOUNDFONT"),
        "the two failures were merged: {stderr}"
    );
}

#[test]
fn a_rig_whose_soundfont_does_not_exist_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());

    mid()
        .args(["play", FIXTURE, "--rig"])
        .arg(dir.path().join("absent.sf2"))
        .env("PATH", &fake.dir)
        .assert()
        .failure()
        .stderr(predicates::str::contains("absent.sf2"));
}

/// A `fluidsynth` that exists but cannot be run is a third condition, and it is
/// not a problem with the Take. Reporting it as one would send the human to look
/// at their music.
#[test]
fn a_fluidsynth_that_cannot_be_started_names_fluidsynth() {
    let dir = tempfile::tempdir().expect("temp dir");
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).expect("scratch dir is writable");
    std::fs::write(bin.join("fluidsynth"), "#!/bin/sh\nexit 0\n").expect("fake is writable");
    let soundfont = common::fake_soundfont(&dir.path().join("chosen.sf2"));

    let output = mid()
        .args(["play", FIXTURE, "--rig"])
        .arg(&soundfont)
        .env("PATH", &bin)
        .output()
        .expect("mid runs");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains("fluidsynth"), "{stderr}");
    assert!(
        !stderr.contains(FIXTURE),
        "a Rig failure was reported against the Take: {stderr}"
    );
}

/// One playback watched by a fake FluidSynth: what `mid` did, the argv the
/// synthesiser was given, and a copy of the file it was handed.
///
/// The copy is the only way to see a passage at all. The temporary Take that
/// carries one is gone by the time `mid` returns, which is itself one of the
/// things this suite has to prove.
struct Playback {
    output: std::process::Output,
    /// What the synthesiser received; empty when it was never reached.
    argv: Vec<String>,
    handed: std::path::PathBuf,
    soundfont: std::path::PathBuf,
}

fn play(dir: &std::path::Path, args: &[&str]) -> Playback {
    let soundfont = common::fake_soundfont(&dir.join("chosen.sf2"));
    play_through(dir, args, Some(&soundfont))
}

/// The same with no Rig at all, for the failures that must not depend on one.
fn play_with_no_rig(dir: &std::path::Path, args: &[&str]) -> Playback {
    play_through(dir, args, None)
}

fn play_through(
    dir: &std::path::Path,
    args: &[&str],
    soundfont: Option<&std::path::Path>,
) -> Playback {
    let fake = fake_fluidsynth(dir);
    let mut command = mid();
    command.arg("play").args(args);
    if let Some(soundfont) = soundfont {
        command.arg("--rig").arg(soundfont);
    }
    let output = command.env("PATH", &fake.dir).output().expect("mid runs");
    Playback {
        argv: std::fs::read_to_string(&fake.log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect(),
        handed: fake.handed,
        soundfont: soundfont.unwrap_or(std::path::Path::new("")).to_path_buf(),
        output,
    }
}

impl Playback {
    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.output.stdout).expect("play --json is JSON")
    }

    fn rig(&self) -> &str {
        self.soundfont.to_str().expect("path is UTF-8")
    }
}

/// The Ticks the notes of the file FluidSynth was handed start on.
fn starts_handed(playback: &Playback) -> Vec<u64> {
    common::notes(&common::inspect_json(&playback.handed))
        .iter()
        .map(|note| note["start"].as_u64().expect("start is a number"))
        .collect()
}

/// Bars 5–8 of the fixture and nothing else, moved to the front so that the
/// passage starts when playback does. FluidSynth plays a file from the
/// beginning and has no range playback, so a passage left at its original Ticks
/// would be four Bars of silence followed by the music.
#[test]
fn a_bar_range_hands_fluidsynth_that_passage_and_nothing_else() {
    let dir = tempfile::tempdir().expect("temp dir");
    let played = play(dir.path(), &[FIXTURE, "--bars", "5:8"]);
    assert!(
        played.output.status.success(),
        "play --bars failed: {}",
        String::from_utf8_lossy(&played.output.stderr)
    );

    let passage: Vec<u64> = common::notes(&common::inspect_json(std::path::Path::new(FIXTURE)))
        .iter()
        .map(|note| note["start"].as_u64().expect("start is a number"))
        .filter(|start| (5760..11520).contains(start))
        .map(|start| start - 5760)
        .collect();
    assert_eq!(passage.len(), 18, "the fixture's Bars 5-8 hold 18 notes");
    assert_eq!(starts_handed(&played), passage);
}

/// The passage is heard as the Take states it. One that lost its tempo would
/// play at FluidSynth's own default, and one that lost its time signature
/// would not be four Bars of anything.
#[test]
fn the_passage_carries_the_takes_own_tempo_and_time_signature() {
    let dir = tempfile::tempdir().expect("temp dir");
    let played = play(dir.path(), &[FIXTURE, "--bars", "5:8"]);
    assert!(played.output.status.success());

    let whole = common::info_json(std::path::Path::new(FIXTURE));
    let passage = common::info_json(&played.handed);
    assert_eq!(passage["ppq"], whole["ppq"]);
    assert_eq!(passage["tempo"], whole["tempo"]);
    assert_eq!(passage["time_signature"], whole["time_signature"]);
    assert_eq!(passage["length_bars"], 4, "Bars 5-8 is four Bars long");
}

/// The temporary Take is an implementation detail of playback. It does not
/// outlive the command, and it is never written where the user keeps their
/// music.
#[test]
fn the_temporary_take_does_not_survive_the_command() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::fixture_bytes();
    let beside_the_fixture = listing("fixtures");

    let played = play(dir.path(), &[FIXTURE, "--bars", "5:8"]);
    assert!(played.output.status.success());

    let handed = std::path::PathBuf::from(played.argv.last().expect("a file was handed over"));
    assert!(
        !handed.exists(),
        "the temporary Take outlived the command: {handed:?}"
    );
    let project = std::env::current_dir().expect("a working directory");
    assert!(
        !handed.starts_with(&project),
        "the temporary Take was written into the project: {handed:?}"
    );
    assert_eq!(listing("fixtures"), beside_the_fixture);
    assert_eq!(
        common::fixture_bytes(),
        before,
        "the Take itself was rewritten"
    );
}

/// The names of everything in a directory, sorted.
fn listing(dir: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("the directory is readable")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Rig resolution and disclosure do not change because a passage was asked for.
/// The record says which passage was heard, too: an audition of four Bars filed
/// against the whole Piece is a false record of what was judged.
#[test]
fn a_passage_is_disclosed_and_recorded_as_a_whole_take_is() {
    let dir = tempfile::tempdir().expect("temp dir");
    let played = play(dir.path(), &[FIXTURE, "--bars", "5:8", "--json"]);
    assert!(played.output.status.success());

    assert!(
        played.stderr().contains(played.rig()),
        "stderr did not state the Rig: {}",
        played.stderr()
    );

    let audition = played.json();
    assert_eq!(audition["rig"], played.rig());
    assert_eq!(audition["take"], FIXTURE, "the Take is named, not the copy");
    assert_eq!(audition["bars"], "5:8");
}

/// One parser, one set of rules, one message: a Bar range `play` cannot honour
/// is refused in exactly the words `inspect` refuses it in, and the synthesiser
/// is never reached.
///
/// Including on a machine with no Rig configured. A Bar range that does not
/// exist is a mistake in the command, and being told about the Rig instead
/// would hide it behind an unrelated piece of setup — then hand it back once
/// the setup was fixed.
#[test]
fn a_bar_range_that_cannot_be_honoured_is_refused_as_inspect_refuses_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fixture = std::path::PathBuf::from(FIXTURE);
    let no_time_signature = common::build_take(
        &dir.path().join("no-time-signature.mid"),
        480,
        &[],
        &[(0, 240, 60)],
    );

    for (take, range) in [
        (&fixture, "9:9"),
        (&fixture, "1:1000"),
        (&fixture, "0:4"),
        (&fixture, "8:5"),
        (&fixture, "5"),
        (&fixture, "nonsense"),
        (&no_time_signature, "1:1"),
    ] {
        let take = take.to_str().expect("path is UTF-8");
        let inspected = mid()
            .args(["inspect", take, "--bars", range])
            .output()
            .expect("mid runs");
        let refusal = String::from_utf8_lossy(&inspected.stderr).into_owned();

        for played in [
            play(dir.path(), &[take, "--bars", range]),
            play_with_no_rig(dir.path(), &[take, "--bars", range]),
        ] {
            assert!(
                !played.output.status.success(),
                "--bars {range} was accepted on {take}"
            );
            assert!(
                played.argv.is_empty(),
                "--bars {range} was refused and still reached the synthesiser"
            );
            assert_eq!(
                played.stderr(),
                refusal,
                "play and inspect refuse --bars {range} differently"
            );
        }
    }
}

/// A note belongs to the Bar it starts in. One that began before the passage is
/// not in it, even though it is still sounding when the passage starts —
/// `inspect --bars` says so, and `play` must not disagree about the same note.
#[test]
fn a_note_that_started_before_the_passage_is_not_played_in_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("across-the-bar-line.mid"),
        480,
        &[(0, 3, 4)],
        // The first note starts in Bar 1 and is still sounding in Bar 2.
        &[(1200, 600, 60), (1440, 240, 62)],
    );

    let played = play(
        dir.path(),
        &[take.to_str().expect("path is UTF-8"), "--bars", "2:2"],
    );
    assert!(played.output.status.success());

    let notes = common::notes(&common::inspect_json(&played.handed));
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0]["pitch"], 62);
    assert_eq!(notes[0]["start"], 0);
}

/// A note struck inside the passage keeps its whole length, even where that
/// runs past the passage's last Bar line. It was played in this passage, and
/// cutting it short would make `play` disagree with the duration `inspect`
/// reports for the same note.
#[test]
fn a_note_struck_in_the_passage_keeps_its_whole_length() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("sustained-past-the-passage.mid"),
        480,
        &[(0, 3, 4)],
        // Bar 2 is Ticks 1440-2879; this note runs on into Bar 3.
        &[(0, 240, 60), (1440, 2000, 62)],
    );

    let played = play(
        dir.path(),
        &[take.to_str().expect("path is UTF-8"), "--bars", "2:2"],
    );
    assert!(played.output.status.success());

    let notes = common::notes(&common::inspect_json(&played.handed));
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0]["duration"], 2000);
}

/// A passage inherits the state set before it. A program change is in the file,
/// so it belongs to the Piece; a passage that dropped it would be heard on an
/// instrument the Take never names, which is the same unusable audition as one
/// heard through a Rig nobody chose.
#[test]
fn the_passage_inherits_the_program_change_that_precedes_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_programs(
        &dir.path().join("with-a-program.mid"),
        480,
        &[(0, 3, 4)],
        &[(0, 42)],
        &[(0, 240, 60), (1440, 240, 62)],
    );

    let played = play(
        dir.path(),
        &[take.to_str().expect("path is UTF-8"), "--bars", "2:2"],
    );
    assert!(played.output.status.success());

    assert_eq!(
        common::program_changes(&played.handed),
        vec![(0, 42)],
        "the passage does not state the program the Take set before it"
    );
}
