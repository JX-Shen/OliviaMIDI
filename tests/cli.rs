mod common;

use common::mid;
use std::os::unix::process::ExitStatusExt;

/// `mid help` and per-command help *are* the command reference — there is no
/// separate document that could drift from the binary. So the suite checks that
/// the contract is actually in there.
#[test]
fn help_lists_every_command() {
    let output = mid().arg("help").output().expect("mid runs");
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    for command in ["info", "inspect", "apply", "diff", "play", "render"] {
        assert!(
            help.contains(command),
            "`mid help` never mentions {command}"
        );
    }
}

#[test]
fn every_command_has_its_own_help() {
    for command in ["info", "inspect", "apply", "diff", "play", "render"] {
        mid().args([command, "--help"]).assert().success();
    }
}

#[test]
fn apply_help_states_the_edit_set_format_and_the_no_in_place_rule() {
    mid()
        .args(["apply", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("set_velocity"))
        .stdout(predicates::str::contains("never modified"));
}

#[test]
fn play_help_states_how_the_rig_is_resolved() {
    mid()
        .args(["play", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("BATTUTA_SOUNDFONT"))
        .stdout(predicates::str::contains("--rig"));
}

/// The tolerance is what decides whether a note "moved" or was "deleted and
/// re-added", and `mid diff --help` is the only place its default is documented.
#[test]
fn diff_help_states_the_tolerance_and_its_default() {
    mid()
        .args(["diff", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--tolerance"))
        // The default, as a note value and as the Ticks it comes to.
        .stdout(predicates::str::contains("sixteenth"))
        .stdout(predicates::str::contains("120"))
        // That the exact-only reading is still reachable.
        .stdout(predicates::str::contains("0 matches by identity alone"));
}

#[test]
fn an_unknown_command_fails() {
    mid().arg("reharmonize").assert().failure();
}

#[test]
fn every_command_needs_a_take() {
    for command in ["info", "inspect", "apply", "diff", "play", "render"] {
        mid().arg(command).assert().failure();
    }
}

#[test]
fn inspect_help_states_the_bar_semantics() {
    mid()
        .args(["inspect", "--help"])
        .assert()
        .success()
        // 1-indexed, both ends included.
        .stdout(predicates::str::contains("1-indexed"))
        .stdout(predicates::str::contains("both ends"))
        // Which Bar a note sustaining across a Bar line belongs to.
        .stdout(predicates::str::contains("starts"))
        // What happens when the time signature is missing or not one throughout.
        .stdout(predicates::str::contains("4/4"))
        .stdout(predicates::str::contains("changes time signature"))
        // That a partial final Bar is a Bar.
        .stdout(predicates::str::contains("final Bar"));
}

/// A reader that stops early — `| head`, which agents reach for constantly —
/// ends `mid` the way it ends any Unix filter: killed by `SIGPIPE`, with nothing
/// on stderr. It used to panic with exit status 101, which reads as a crash in
/// `mid` when nothing is wrong with the Take or the command. #74.
///
/// The Take is built large enough that `inspect` cannot fit its output in a
/// pipe buffer, so `mid` is still writing after the reader has gone whatever
/// the timing.
#[test]
fn a_reader_that_goes_away_ends_mid_quietly() {
    let dir = tempfile::tempdir().expect("temp dir");
    let notes: Vec<common::NoteSpec> = (0..4000u32)
        .map(|i| (i * 120, 110, 36 + (i % 60) as u8))
        .collect();
    let take = common::build_take(&dir.path().join("long.mid"), 480, &[(0, 4, 4)], &notes);

    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("mid"))
        .arg("inspect")
        .arg(&take)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("mid runs");
    // The reader goes away before reading a byte.
    drop(child.stdout.take());
    let output = child.wait_with_output().expect("mid exits");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "mid said something on its way out: {stderr}"
    );
    assert_eq!(
        output.status.signal(),
        Some(libc::SIGPIPE),
        "mid did not end by SIGPIPE: {:?}",
        output.status
    );
}
