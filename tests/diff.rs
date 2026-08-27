mod common;

use common::{build_take, edit_set, first_note_id, mid, set_velocity_edit_set, FIXTURE};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// A Take derived from the fixture by an Edit Set spelled out at the call site.
fn derived(dir: &Path, name: &str, edits: &str) -> PathBuf {
    let out = dir.join(format!("{name}.mid"));
    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(dir, name, edits))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();
    out
}

/// `mid diff --json`, parsed. `tolerance` is passed through when a test states
/// one, so that the default is exercised by every test that does not.
fn diff_json(before: &Path, after: &Path, tolerance: Option<&str>) -> Value {
    let mut command = mid();
    command.arg("diff").arg(before).arg(after).arg("--json");
    if let Some(ticks) = tolerance {
        command.args(["--tolerance", ticks]);
    }
    let output = command.output().expect("mid runs");
    assert!(
        output.status.success(),
        "diff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("diff --json is JSON")
}

fn list<'a>(diff: &'a Value, field: &str) -> &'a Vec<Value> {
    diff[field]
        .as_array()
        .unwrap_or_else(|| panic!("{field} is a list"))
}

/// What a changed pairing says, flattened to what a test wants to assert on.
fn changed(diff: &Value) -> Vec<(String, String, Vec<String>)> {
    list(diff, "changed")
        .iter()
        .map(|change| {
            (
                change["before"]["id"].as_str().expect("an id").to_string(),
                change["after"]["id"].as_str().expect("an id").to_string(),
                change["changes"]
                    .as_array()
                    .expect("changes is a list")
                    .iter()
                    .map(|kind| kind.as_str().expect("a kind").to_string())
                    .collect(),
            )
        })
        .collect()
}

/// Two Takes holding the same one note, the second one having moved it.
///
/// Built rather than derived from the fixture because a test about the tolerance
/// wants to state the distance in Ticks at the call site, and 480 ticks per
/// quarter note is what makes the default a sixteenth note of 120.
fn moved_by(dir: &Path, ticks: u32) -> (PathBuf, PathBuf) {
    let before = build_take(
        &dir.join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[(480, 240, 60)],
    );
    let after = build_take(
        &dir.join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[(480 + ticks, 240, 60)],
    );
    (before, after)
}

#[test]
fn reports_one_velocity_change_and_nothing_else() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target = first_note_id();
    let out = dir.path().join("take-02.mid");
    mid()
        .args(["apply", FIXTURE])
        .arg(set_velocity_edit_set(dir.path(), &target, "40"))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let diff = diff_json(Path::new(FIXTURE), &out, None);
    assert_eq!(list(&diff, "added").len(), 0);
    assert_eq!(list(&diff, "removed").len(), 0);
    assert_eq!(
        changed(&diff),
        vec![(target.clone(), target, vec!["velocity".to_string()])]
    );
    assert_eq!(diff["changed"][0]["before"]["velocity"], 50);
    assert_eq!(diff["changed"][0]["after"]["velocity"], 40);
}

#[test]
fn a_take_does_not_differ_from_itself() {
    let diff = diff_json(Path::new(FIXTURE), Path::new(FIXTURE), None);
    assert_eq!(list(&diff, "added").len(), 0);
    assert_eq!(list(&diff, "removed").len(), 0);
    assert_eq!(list(&diff, "changed").len(), 0);
}

#[test]
fn fails_when_a_take_cannot_be_read() {
    mid()
        .args(["diff", FIXTURE, "fixtures/no-such-take.mid"])
        .assert()
        .failure();
}

#[test]
fn a_note_moved_within_the_tolerance_is_one_changed_note() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (before, after) = moved_by(dir.path(), 100);

    let diff = diff_json(&before, &after, None);
    assert_eq!(list(&diff, "added").len(), 0);
    assert_eq!(list(&diff, "removed").len(), 0);
    assert_eq!(
        changed(&diff),
        vec![(
            "t1:c0:p60:s480:n0".to_string(),
            "t1:c0:p60:s580:n0".to_string(),
            vec!["start".to_string()],
        )]
    );
    assert_eq!(diff["changed"][0]["before"]["start"], 480);
    assert_eq!(diff["changed"][0]["after"]["start"], 580);
}

#[test]
fn the_same_move_beyond_the_tolerance_is_added_plus_removed() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (before, after) = moved_by(dir.path(), 240);

    let diff = diff_json(&before, &after, None);
    assert_eq!(list(&diff, "changed").len(), 0);
    assert_eq!(list(&diff, "removed")[0]["id"], "t1:c0:p60:s480:n0");
    assert_eq!(list(&diff, "added")[0]["id"], "t1:c0:p60:s720:n0");
}

#[test]
fn the_tolerance_can_be_overridden_from_the_command_line() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (before, after) = moved_by(dir.path(), 240);

    let diff = diff_json(&before, &after, Some("240"));
    assert_eq!(list(&diff, "added").len(), 0);
    assert_eq!(list(&diff, "removed").len(), 0);
    assert_eq!(list(&diff, "changed").len(), 1);
}

#[test]
fn a_tolerance_of_zero_matches_by_identity_alone() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (before, after) = moved_by(dir.path(), 1);

    let diff = diff_json(&before, &after, Some("0"));
    assert_eq!(list(&diff, "changed").len(), 0);
    assert_eq!(list(&diff, "added").len(), 1);
    assert_eq!(list(&diff, "removed").len(), 1);
}

#[test]
fn a_resized_note_is_reported_as_a_duration_change() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target = first_note_id();
    let after = derived(
        dir.path(),
        "resize",
        &format!(r#"{{ "kind": "resize_note", "id": "{target}", "delta_ticks": 400 }}"#),
    );

    let diff = diff_json(Path::new(FIXTURE), &after, None);
    assert_eq!(list(&diff, "added").len(), 0);
    assert_eq!(list(&diff, "removed").len(), 0);
    assert_eq!(
        changed(&diff),
        vec![(target.clone(), target, vec!["duration".to_string()])]
    );
    assert_eq!(diff["changed"][0]["before"]["duration"], 1435);
    assert_eq!(diff["changed"][0]["after"]["duration"], 1835);
}

#[test]
fn a_note_that_both_moved_and_softened_reports_both_changes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target = first_note_id();
    let after = derived(
        dir.path(),
        "moved-and-softened",
        &format!(
            r#"{{ "kind": "move_note", "id": "{target}", "delta_ticks": 60 }},
               {{ "kind": "set_velocity", "id": "{target}", "velocity": 40 }}"#
        ),
    );

    let diff = diff_json(Path::new(FIXTURE), &after, None);
    assert_eq!(list(&diff, "added").len(), 0);
    assert_eq!(list(&diff, "removed").len(), 0);
    assert_eq!(
        changed(&diff),
        vec![(
            target,
            "t1:c0:p69:s60:n0".to_string(),
            vec!["start".to_string(), "velocity".to_string()],
        )]
    );
}

#[test]
fn changes_are_listed_in_a_fixed_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target = first_note_id();
    let after = derived(
        dir.path(),
        "all-four",
        &format!(
            r#"{{ "kind": "set_velocity",   "id": "{target}", "velocity": 40 }},
               {{ "kind": "resize_note",    "id": "{target}", "delta_ticks": 100 }},
               {{ "kind": "move_note",      "id": "{target}", "delta_ticks": 60 }},
               {{ "kind": "transpose_note", "id": "{target}", "semitones": -2 }}"#
        ),
    );

    let diff = diff_json(Path::new(FIXTURE), &after, None);
    // Asked for in the reverse of the order they must be reported in, so that a
    // classification following the Edit Set rather than the fixed order fails.
    assert_eq!(
        changed(&diff),
        vec![(
            target,
            "t1:c0:p67:s60:n0".to_string(),
            vec![
                "pitch".to_string(),
                "start".to_string(),
                "duration".to_string(),
                "velocity".to_string(),
            ],
        )]
    );
}

#[test]
fn matching_never_pairs_notes_across_tracks() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target = first_note_id();
    // The same channel and pitch, 60 Ticks away — inside the tolerance in every
    // respect but the one that matters.
    let after = derived(
        dir.path(),
        "other-track",
        &format!(
            r#"{{ "kind": "delete_note", "id": "{target}" }},
               {{ "kind": "add_note", "track": 2, "channel": 0, "pitch": 69,
                  "start": 60, "duration": 1435, "velocity": 50 }}"#
        ),
    );

    let diff = diff_json(Path::new(FIXTURE), &after, None);
    assert_eq!(list(&diff, "changed").len(), 0);
    assert_eq!(list(&diff, "removed")[0]["id"], target.as_str());
    assert_eq!(list(&diff, "added")[0]["id"], "t2:c0:p69:s60:n0");
}

#[test]
fn pairs_a_moved_note_out_of_a_genuine_identity_collision() {
    // Two notes identical in track, channel, pitch and start — the doubled voice
    // ADR-0002's occurrence index exists for, and which the fixture cannot
    // produce. Moving one of them renumbers the other, so the identity a note
    // answers to on each side is not the one it had.
    let dir = tempfile::tempdir().expect("temp dir");
    let before = build_take(
        &dir.path().join("doubled.mid"),
        480,
        &[(0, 4, 4)],
        &[(480, 240, 60), (480, 240, 60)],
    );
    let after = build_take(
        &dir.path().join("one-moved.mid"),
        480,
        &[(0, 4, 4)],
        &[(480, 240, 60), (580, 240, 60)],
    );

    let diff = diff_json(&before, &after, None);
    assert_eq!(list(&diff, "added").len(), 0);
    assert_eq!(list(&diff, "removed").len(), 0);
    assert_eq!(
        changed(&diff),
        vec![(
            "t1:c0:p60:s480:n1".to_string(),
            "t1:c0:p60:s580:n0".to_string(),
            vec!["start".to_string()],
        )]
    );
}

#[test]
fn refuses_two_takes_denominated_in_different_ticks() {
    let dir = tempfile::tempdir().expect("temp dir");
    // The same four quarter notes, twice, at two PPQs.
    let fine = build_take(
        &dir.path().join("fine.mid"),
        480,
        &[(0, 4, 4)],
        &[
            (0, 475, 60),
            (480, 475, 62),
            (960, 475, 64),
            (1440, 475, 65),
        ],
    );
    let coarse = build_take(
        &dir.path().join("coarse.mid"),
        96,
        &[(0, 4, 4)],
        &[(0, 95, 60), (96, 95, 62), (192, 95, 64), (288, 95, 65)],
    );

    mid()
        .arg("diff")
        .arg(&fine)
        .arg(&coarse)
        .assert()
        .failure()
        .stderr(predicates::str::contains("480"))
        .stderr(predicates::str::contains("96"));
}

#[test]
fn two_takes_with_no_shared_ancestry_diff_rather_than_fail() {
    let dir = tempfile::tempdir().expect("temp dir");
    let stranger = build_take(
        &dir.path().join("stranger.mid"),
        480,
        &[(0, 3, 4)],
        &[(0, 240, 40), (960, 240, 41)],
    );

    let diff = diff_json(Path::new(FIXTURE), &stranger, None);

    // Every note on each side is accounted for exactly once. This is the whole
    // of what "defensible" can mean between two Takes that share nothing: the
    // answer is total and deterministic rather than minimal.
    assert_eq!(
        list(&diff, "removed").len() + list(&diff, "changed").len(),
        36
    );
    assert_eq!(list(&diff, "added").len() + list(&diff, "changed").len(), 2);

    // And it is not minimal, on purpose. The stranger's first note shares a
    // track, a channel and a Tick with the fixture's, so the greedy pass pairs
    // them across twenty-nine semitones and calls it a changed note. That cost
    // is why the tolerance is stated with every diff and why 0 is spelled.
    assert_eq!(
        changed(&diff),
        vec![(
            "t1:c0:p69:s0:n0".to_string(),
            "t1:c0:p40:s0:n0".to_string(),
            vec![
                "pitch".to_string(),
                "duration".to_string(),
                "velocity".to_string(),
            ],
        )]
    );
    assert_eq!(list(&diff, "added")[0]["id"], "t1:c0:p41:s960:n0");
}

#[test]
fn states_the_tolerance_it_matched_with() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (before, after) = moved_by(dir.path(), 100);

    // The default is a sixteenth note, which at this Take's 480 ticks per
    // quarter note is 120.
    let defaulted = diff_json(&before, &after, None);
    assert_eq!(defaulted["tolerance_ticks"], 120);

    let overridden = diff_json(&before, &after, Some("240"));
    assert_eq!(overridden["tolerance_ticks"], 240);

    // And on stderr, for a human, whether or not the payload was asked for —
    // the same reasoning as the Rig disclosure in ADR-0009.
    mid()
        .arg("diff")
        .arg(&before)
        .arg(&after)
        .assert()
        .success()
        .stderr(predicates::str::contains("120"));
}
