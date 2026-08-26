mod common;

use common::mid;

/// `mid help` and per-command help *are* the command reference — there is no
/// separate document that could drift from the binary. So the suite checks that
/// the contract is actually in there.
#[test]
fn help_lists_every_command() {
    let output = mid().arg("help").output().expect("mid runs");
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    for command in ["info", "inspect", "apply", "diff", "play"] {
        assert!(
            help.contains(command),
            "`mid help` never mentions {command}"
        );
    }
}

#[test]
fn every_command_has_its_own_help() {
    for command in ["info", "inspect", "apply", "diff", "play"] {
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

#[test]
fn an_unknown_command_fails() {
    mid().arg("reharmonize").assert().failure();
}

#[test]
fn every_command_needs_a_take() {
    for command in ["info", "inspect", "apply", "diff", "play"] {
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
        // What happens when the metre is missing or not the same throughout.
        .stdout(predicates::str::contains("4/4"))
        .stdout(predicates::str::contains("changes time signature"))
        // That a partial final Bar is a Bar.
        .stdout(predicates::str::contains("final Bar"));
}
