mod common;

use common::{
    empty_edit_set, event_stream, first_note_id, fixture_bytes, mid, set_velocity_edit_set, write,
    FIXTURE,
};
use std::path::Path;

/// The property, not a snapshot: two runs have to *agree*, which is a relation a
/// stored blob cannot express. See ADR-0005 for what "identical" means here.
#[test]
fn an_empty_edit_set_round_trips_the_take() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", FIXTURE])
        .arg(empty_edit_set(dir.path()))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    assert_eq!(
        event_stream(Path::new(FIXTURE)),
        event_stream(&out),
        "an empty Edit Set changed the Take"
    );
}

#[test]
fn the_round_trip_keeps_durations_exactly() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    mid()
        .args(["apply", FIXTURE])
        .arg(empty_edit_set(dir.path()))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let before = common::notes(&common::inspect_json(Path::new(FIXTURE)));
    let after = common::notes(&common::inspect_json(&out));
    let durations = |notes: &[serde_json::Value]| -> Vec<u64> {
        notes
            .iter()
            .map(|n| n["duration"].as_u64().expect("duration is a number"))
            .collect()
    };
    assert_eq!(durations(&before), durations(&after));
    assert!(durations(&after).contains(&475));
    assert!(durations(&after).contains(&955));
    assert!(durations(&after).contains(&1435));
}

/// Identity is content-derived, and velocity is not part of the content it is
/// derived from. So a `set_velocity` must leave *every* identity alone — the one
/// it names included.
#[test]
fn identities_survive_an_edit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    let target = first_note_id();

    mid()
        .args(["apply", FIXTURE])
        .arg(set_velocity_edit_set(dir.path(), &target, "40"))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let before = common::inspect_json(Path::new(FIXTURE));
    let after = common::inspect_json(&out);
    assert_eq!(common::note_ids(&before), common::note_ids(&after));

    let changed: Vec<_> = common::notes(&before)
        .iter()
        .zip(common::notes(&after))
        .filter(|(b, a)| b["velocity"] != a["velocity"])
        .map(|(b, _)| b["id"].as_str().expect("id is a string").to_string())
        .collect();
    assert_eq!(changed, vec![target], "exactly one note changed");
}

#[test]
fn an_unknown_identity_fails_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", FIXTURE])
        .arg(set_velocity_edit_set(dir.path(), "t9:c9:p9:s9:n9", "40"))
        .arg("-o")
        .arg(&out)
        .assert()
        .failure()
        .stderr(predicates::str::contains("t9:c9:p9:s9:n9"));

    assert!(
        !out.exists(),
        "a failed Edit Set left a partial Take behind"
    );
}

#[test]
fn an_unsupported_operation_fails_rather_than_being_skipped() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    let edits = dir.path().join("edits.json");
    write(
        &edits,
        r#"{ "edits": [ { "op": "make_sadder", "id": "t1:c0:p69:s0:n0" } ] }"#,
    );

    mid()
        .args(["apply", FIXTURE])
        .arg(&edits)
        .arg("-o")
        .arg(&out)
        .assert()
        .failure();
    assert!(!out.exists());
}

#[test]
fn a_velocity_outside_midi_range_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target = first_note_id();
    for velocity in ["0", "128", "-4"] {
        let out = dir.path().join(format!("take-{velocity}.mid"));
        mid()
            .args(["apply", FIXTURE])
            .arg(set_velocity_edit_set(dir.path(), &target, velocity))
            .arg("-o")
            .arg(&out)
            .assert()
            .failure();
        assert!(!out.exists(), "velocity {velocity} produced a Take");
    }
}

#[test]
fn output_is_required() {
    let dir = tempfile::tempdir().expect("temp dir");
    mid()
        .args(["apply", FIXTURE])
        .arg(empty_edit_set(dir.path()))
        .assert()
        .failure();
}

#[test]
fn apply_refuses_to_write_over_its_input() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = dir.path().join("take-01.mid");
    std::fs::write(&take, fixture_bytes()).expect("scratch Take is writable");
    let before = std::fs::read(&take).expect("scratch Take is readable");

    mid()
        .arg("apply")
        .arg(&take)
        .arg(empty_edit_set(dir.path()))
        .arg("-o")
        .arg(&take)
        .assert()
        .failure();

    assert_eq!(before, std::fs::read(&take).expect("still readable"));
}

/// The reference Piece is a fixed input. Nothing this suite runs may edit it.
#[test]
fn the_fixture_is_never_written_to() {
    let before = fixture_bytes();
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    mid()
        .args(["apply", FIXTURE])
        .arg(set_velocity_edit_set(dir.path(), &first_note_id(), "40"))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();
    assert_eq!(before, fixture_bytes());
}
