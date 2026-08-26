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
