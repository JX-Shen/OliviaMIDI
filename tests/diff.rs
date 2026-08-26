mod common;

use common::{first_note_id, mid, set_velocity_edit_set, FIXTURE};
use std::path::Path;

/// A Take derived from the fixture by softening its first note.
fn derived_take(dir: &Path, velocity: &str) -> (std::path::PathBuf, String) {
    let target = first_note_id();
    let out = dir.join("take-02.mid");
    mid()
        .args(["apply", FIXTURE])
        .arg(set_velocity_edit_set(dir, &target, velocity))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();
    (out, target)
}

#[test]
fn reports_one_velocity_change_and_nothing_else() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (after, target) = derived_take(dir.path(), "40");

    let output = mid()
        .args(["diff", FIXTURE])
        .arg(&after)
        .arg("--json")
        .output()
        .expect("mid runs");
    assert!(output.status.success());
    let diff: serde_json::Value = serde_json::from_slice(&output.stdout).expect("diff is JSON");

    assert_eq!(diff["added"].as_array().expect("added is a list").len(), 0);
    assert_eq!(
        diff["removed"].as_array().expect("removed is a list").len(),
        0
    );
    let changed = diff["velocity_changed"]
        .as_array()
        .expect("velocity_changed is a list");
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0]["id"], target);
    assert_eq!(changed[0]["from"], 50);
    assert_eq!(changed[0]["to"], 40);
}

#[test]
fn a_take_does_not_differ_from_itself() {
    mid()
        .args(["diff", FIXTURE, FIXTURE, "--json"])
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""added": []"#))
        .stdout(predicates::str::contains(r#""removed": []"#))
        .stdout(predicates::str::contains(r#""velocity_changed": []"#));
}

#[test]
fn fails_when_a_take_cannot_be_read() {
    mid()
        .args(["diff", FIXTURE, "fixtures/no-such-take.mid"])
        .assert()
        .failure();
}
