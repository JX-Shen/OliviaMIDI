mod common;

use common::{
    edit_set, empty_edit_set, event_stream, first_note_id, fixture_bytes, mid,
    set_velocity_edit_set, write, FIXTURE,
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
fn an_unsupported_edit_fails_rather_than_being_skipped() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    let edits = dir.path().join("edits.json");
    write(
        &edits,
        r#"{ "edits": [ { "kind": "make_sadder", "id": "t1:c0:p69:s0:n0" } ] }"#,
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

/// The wire format used to spell this key `op`. An Edit Set still written that
/// way must fail rather than parse into something empty — a stale Edit Set that
/// silently does nothing is the failure mode this whole format is shaped to
/// avoid.
#[test]
fn the_superseded_op_key_is_rejected() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    let edits = dir.path().join("edits.json");
    write(
        &edits,
        r#"{ "edits": [ { "op": "set_velocity", "id": "t1:c0:p69:s0:n0", "velocity": 40 } ] }"#,
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

/// A note's key is on both of its events. An Edit that changed only the note-on
/// would leave a note struck and never released, and the Take would not be
/// readable at all — which is the cheapest demonstration that an Edit is not, in
/// general, one event.
#[test]
fn a_transposed_note_changes_pitch_and_nothing_else() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    let target = first_note_id(); // t1:c0:p69:s0:n0 — 1435 ticks at velocity 50

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "transpose",
            &format!(r#"{{ "kind": "transpose_note", "id": "{target}", "semitones": -2 }}"#),
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let after = common::notes(&common::inspect_json(&out));
    let moved = after
        .iter()
        .find(|note| note["id"] == "t1:c0:p67:s0:n0")
        .expect("the transposed note is two semitones down");
    assert_eq!(moved["start"], 0);
    assert_eq!(moved["duration"], 1435);
    assert_eq!(moved["velocity"], 50);

    // Every other identity is untouched, and the transposed one took the place
    // its own new content gives it.
    let expected: Vec<String> = common::note_ids(&common::inspect_json(Path::new(FIXTURE)))
        .into_iter()
        .map(|id| {
            if id == target {
                "t1:c0:p67:s0:n0".to_string()
            } else {
                id
            }
        })
        .collect();
    assert_eq!(common::note_ids(&common::inspect_json(&out)), expected);
}

/// Moving a note changes when *both* of its events happen, which changes the
/// delta times of everything after them. The assertion that bites is not the
/// moved note's own start but that nothing else moved: a delta re-encoded
/// wrongly would drag the whole rest of the track with it.
#[test]
fn a_moved_note_leaves_its_neighbours_where_they_were() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    let target = "t1:c0:p71:s1440:n0"; // 955 ticks at velocity 50

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "move",
            &format!(r#"{{ "kind": "move_note", "id": "{target}", "delta_ticks": -480 }}"#),
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let after = common::notes(&common::inspect_json(&out));
    let moved = after
        .iter()
        .find(|note| note["id"] == "t1:c0:p71:s960:n0")
        .expect("the moved note starts 480 ticks earlier");
    assert_eq!(moved["duration"], 955, "a move does not resize");
    assert_eq!(moved["velocity"], 50);

    let elsewhere = |notes: &[serde_json::Value]| -> Vec<(String, u64)> {
        notes
            .iter()
            .filter(|note| note["id"] != target && note["id"] != "t1:c0:p71:s960:n0")
            .map(|note| {
                (
                    note["id"].as_str().expect("id is a string").to_string(),
                    note["start"].as_u64().expect("start is a number"),
                )
            })
            .collect()
    };
    let before = common::notes(&common::inspect_json(Path::new(FIXTURE)));
    assert_eq!(elsewhere(&before), elsewhere(&after), "a neighbour moved");
}

/// Resizing moves the note-off and nothing else. Identity is derived from track,
/// channel, pitch and start — a note's length is not part of it — so a resized
/// note answers to the name it always had.
#[test]
fn a_resized_note_keeps_its_start_and_its_identity() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    let target = "t1:c0:p67:s2400:n0"; // 475 ticks

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "resize",
            &format!(r#"{{ "kind": "resize_note", "id": "{target}", "delta_ticks": 240 }}"#),
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let after = common::notes(&common::inspect_json(&out));
    let resized = after
        .iter()
        .find(|note| note["id"] == target)
        .expect("a resized note answers to the name it always had");
    assert_eq!(resized["start"], 2400, "a resize does not move a note");
    assert_eq!(resized["duration"], 715);

    let before = common::notes(&common::inspect_json(Path::new(FIXTURE)));
    assert_eq!(
        common::note_ids(&common::inspect_json(Path::new(FIXTURE))),
        common::note_ids(&common::inspect_json(&out))
    );
    let others = |notes: &[serde_json::Value]| -> Vec<u64> {
        notes
            .iter()
            .filter(|note| note["id"] != target)
            .map(|note| note["duration"].as_u64().expect("duration is a number"))
            .collect()
    };
    assert_eq!(
        others(&before),
        others(&after),
        "another note's length changed"
    );
}

/// Deleting takes both of a note's events out. Removing them from the list
/// would renumber every event after them, and any later Edit in the same Edit
/// Set is already holding one of those numbers — so a deleted slot is marked
/// dead and left where it is.
#[test]
fn a_deleted_note_takes_both_its_events_and_leaves_the_rest_numbered() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    let target = "t1:c0:p71:s1440:n0";

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "delete",
            &format!(r#"{{ "kind": "delete_note", "id": "{target}" }}"#),
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let before = common::note_ids(&common::inspect_json(Path::new(FIXTURE)));
    let after = common::note_ids(&common::inspect_json(&out));
    let expected: Vec<String> = before.into_iter().filter(|id| id != target).collect();
    assert_eq!(after.len(), 35);
    assert_eq!(after, expected, "deleting one note disturbed another");
}

/// A `delete_note` earlier in the Edit Set must not renumber a later Edit's
/// target. This is the case ADR-0002 was tightened to close, and the one the
/// fixture cannot reach by accident: the two Edits name notes far apart in the
/// track, and the second is only correct if the first left its number alone.
#[test]
fn a_delete_does_not_renumber_a_later_edits_target() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "delete-then-edit",
            r#"{ "kind": "delete_note", "id": "t1:c0:p69:s0:n0" },
               { "kind": "set_velocity", "id": "t1:c0:p67:s2400:n0", "velocity": 100 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let after = common::notes(&common::inspect_json(&out));
    let changed: Vec<(&str, u64)> = after
        .iter()
        .filter(|note| note["velocity"] != 50 && note["track"] == 1)
        .map(|note| {
            (
                note["id"].as_str().expect("id is a string"),
                note["velocity"].as_u64().expect("velocity is a number"),
            )
        })
        .collect();
    assert_eq!(
        changed,
        vec![("t1:c0:p67:s2400:n0", 100)],
        "the second Edit landed on a different note"
    );
}

/// Identities are resolved against the input Take, so a note an earlier Edit
/// deleted still resolves — it just has nowhere for a later effect to land. That
/// is refused rather than quietly done nothing about: an Edit Set that changes
/// less than it says is the failure mode the whole format is shaped to avoid.
#[test]
fn an_edit_naming_a_note_an_earlier_edit_deleted_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "delete-then-name",
            r#"{ "kind": "delete_note", "id": "t1:c0:p69:s0:n0" },
               { "kind": "set_velocity", "id": "t1:c0:p69:s0:n0", "velocity": 100 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .failure()
        .stderr(predicates::str::contains("t1:c0:p69:s0:n0"));
    assert!(!out.exists());
}

/// `add_note` is the only kind that creates an identity rather than naming one,
/// so the note it makes has to be derived the same way as every other: on a
/// later `inspect` it is indistinguishable from one that was always there.
#[test]
fn an_added_note_is_indistinguishable_from_one_that_was_always_there() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "add",
            r#"{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 62,
                 "start": 5760, "duration": 955, "velocity": 50 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let after = common::notes(&common::inspect_json(&out));
    assert_eq!(after.len(), 37);
    let added = after
        .iter()
        .find(|note| note["id"] == "t1:c0:p62:s5760:n0")
        .expect("the added note is named the way every other note is");
    assert_eq!(added["track"], 1);
    assert_eq!(added["channel"], 0);
    assert_eq!(added["pitch"], 62);
    assert_eq!(added["start"], 5760);
    assert_eq!(added["duration"], 955);
    assert_eq!(added["velocity"], 50);

    for id in common::note_ids(&common::inspect_json(Path::new(FIXTURE))) {
        assert!(
            common::note_ids(&common::inspect_json(&out)).contains(&id),
            "adding a note renamed {id}"
        );
    }
}

/// The collision ADR-0002's occurrence index exists for, created on purpose —
/// the fixture has none, so this is the first time that path is taken at all.
///
/// The note that was already there keeps `n0`. If an added note-on could be
/// placed ahead of it, an untouched note would silently change its name, and
/// two of this ticket's acceptance criteria would contradict each other.
#[test]
fn a_note_added_on_top_of_another_takes_the_next_occurrence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    // t2:c1:p54:s480:n0 is already there, 955 ticks long at velocity 38.

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "stack",
            r#"{ "kind": "add_note", "track": 2, "channel": 1, "pitch": 54,
                 "start": 480, "duration": 955, "velocity": 90 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    assert_eq!(
        stacked_at(&out, 2, 54, 480),
        vec![
            ("t2:c1:p54:s480:n0".to_string(), 955, 38),
            ("t2:c1:p54:s480:n1".to_string(), 955, 90),
        ],
        "the note that was already there did not keep its name and its length"
    );
}

/// The one overlap MIDI cannot survive. A note-off names a channel and a pitch,
/// not the note-on it ends, so a reader hands the first release to the earliest
/// note still sounding. Let one note of a channel and pitch finish *inside*
/// another and re-reading the Take gives each of them the other's length: an
/// `apply` that succeeded, a file that plays, and two lengths quietly swapped.
///
/// Worse, it cannot be undone. Deleting the added note afterwards would take the
/// release the reader gave to the *other* note, leaving the note that was always
/// there shorter than it started. Refused, therefore, rather than written.
#[test]
fn a_note_added_inside_another_of_the_same_pitch_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    // t2:c1:p54:s480:n0 sounds from tick 480 to tick 1435.

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "nested",
            r#"{ "kind": "add_note", "track": 2, "channel": 1, "pitch": 54,
                 "start": 720, "duration": 240, "velocity": 60 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .failure()
        .stderr(predicates::str::contains("pitch 54"));
    assert!(!out.exists());
}

/// The refusal above is about notes finishing out of order, not about notes
/// sharing a pitch. A second note of the same pitch and channel that begins
/// after the first has finished is ordinary music and must go through.
#[test]
fn a_second_note_of_the_same_pitch_after_the_first_is_allowed() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    // Between t2:c1:p54:s480:n0 ending at 1435 and t2:c1:p54:s1920:n0 beginning.

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "after",
            r#"{ "kind": "add_note", "track": 2, "channel": 1, "pitch": 54,
                 "start": 1440, "duration": 400, "velocity": 60 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let after = common::notes(&common::inspect_json(&out));
    let added = after
        .iter()
        .find(|note| note["id"] == "t2:c1:p54:s1440:n0")
        .expect("the added note is there under its own name");
    assert_eq!(added["duration"], 400);
}

/// All six kinds in one Edit Set. Their effects are ordered; their targets were
/// all fixed against the input Take before the first of them ran.
///
/// The assertion is the whole of track 1, in order, and the whole of track 2
/// untouched: identity stability has to hold no matter which kinds an Edit Set
/// contains, and the only way to say that is to name every identity.
#[test]
fn every_kind_of_edit_can_be_combined_in_one_edit_set() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "all-six",
            r#"{ "kind": "delete_note",    "id": "t1:c0:p69:s0:n0" },
               { "kind": "move_note",      "id": "t1:c0:p71:s1440:n0", "delta_ticks": -480 },
               { "kind": "transpose_note", "id": "t1:c0:p67:s2400:n0", "semitones": 5 },
               { "kind": "resize_note",    "id": "t1:c0:p69:s2880:n0", "delta_ticks": -1000 },
               { "kind": "set_velocity",   "id": "t1:c0:p61:s9600:n0", "velocity": 90 },
               { "kind": "add_note", "track": 1, "channel": 0, "pitch": 74,
                 "start": 5760, "duration": 480, "velocity": 70 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let after = common::notes(&common::inspect_json(&out));
    let on_track = |notes: &[serde_json::Value], track: u64| -> Vec<String> {
        notes
            .iter()
            .filter(|note| note["track"] == track)
            .map(|note| note["id"].as_str().expect("id is a string").to_string())
            .collect()
    };
    assert_eq!(
        on_track(&after, 1),
        vec![
            "t1:c0:p71:s960:n0",  // moved 480 ticks earlier
            "t1:c0:p72:s2400:n0", // transposed up five semitones
            "t1:c0:p69:s2880:n0", // resized; a length is not part of an identity
            "t1:c0:p64:s4320:n0",
            "t1:c0:p62:s5280:n0",
            "t1:c0:p66:s5760:n0",
            "t1:c0:p74:s5760:n0", // added, and behind the note already at 5760
            "t1:c0:p64:s6720:n0",
            "t1:c0:p62:s7200:n0",
            "t1:c0:p64:s8640:n0",
            "t1:c0:p61:s9600:n0", // velocity changed; an identity does not carry it
            "t1:c0:p62:s10080:n0",
        ]
    );

    let before = common::notes(&common::inspect_json(Path::new(FIXTURE)));
    assert_eq!(
        on_track(&before, 2),
        on_track(&after, 2),
        "an Edit Set that never named track 2 renamed something on it"
    );

    let field = |id: &str, field: &str| -> u64 {
        after
            .iter()
            .find(|note| note["id"] == id)
            .unwrap_or_else(|| panic!("{id} is in the new Take"))[field]
            .as_u64()
            .expect("a number")
    };
    assert_eq!(
        field("t1:c0:p71:s960:n0", "duration"),
        955,
        "a move resized"
    );
    assert_eq!(field("t1:c0:p69:s2880:n0", "duration"), 435);
    assert_eq!(field("t1:c0:p61:s9600:n0", "velocity"), 90);
    assert_eq!(field("t1:c0:p74:s5760:n0", "duration"), 480);
    assert_eq!(field("t1:c0:p74:s5760:n0", "velocity"), 70);
}

/// Targets are resolved against the *input* Take, so a note an earlier
/// `add_note` created is not there to be found. Its identity is perfectly
/// well-formed and names nothing, which is exactly the case that must fail
/// rather than resolve — the cost ADR-0002 accepts for resolving up front.
#[test]
fn an_edit_naming_a_note_an_earlier_add_note_created_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "add-then-name",
            r#"{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 74,
                 "start": 5760, "duration": 480, "velocity": 70 },
               { "kind": "set_velocity", "id": "t1:c0:p74:s5760:n0", "velocity": 90 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .failure()
        .stderr(predicates::str::contains("t1:c0:p74:s5760:n0"));
    assert!(!out.exists());
}

/// Every number an Edit can carry that a Take cannot hold. Each is reported as
/// the number that was asked for — which is why the fields are `i64` and not
/// typed to MIDI's own range: a `pitch` typed `u8` would turn 128 into a JSON
/// parse failure that does not say which number was wrong.
#[test]
fn a_number_a_take_cannot_hold_fails_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cases = [
        (
            "transpose above the top of the keyboard",
            r#"{ "kind": "transpose_note", "id": "t1:c0:p71:s1440:n0", "semitones": 60 }"#,
            "131",
        ),
        (
            "transpose below the bottom",
            r#"{ "kind": "transpose_note", "id": "t1:c0:p71:s1440:n0", "semitones": -80 }"#,
            "-9",
        ),
        (
            "move before the start of the Take",
            r#"{ "kind": "move_note", "id": "t1:c0:p69:s0:n0", "delta_ticks": -1 }"#,
            "-1",
        ),
        (
            "resize to nothing",
            r#"{ "kind": "resize_note", "id": "t1:c0:p67:s2400:n0", "delta_ticks": -475 }"#,
            "0 ticks long",
        ),
        (
            "resize past nothing",
            r#"{ "kind": "resize_note", "id": "t1:c0:p67:s2400:n0", "delta_ticks": -500 }"#,
            "-25 ticks long",
        ),
        (
            "add to a track the Take does not have",
            r#"{ "kind": "add_note", "track": 9, "channel": 0, "pitch": 60,
                 "start": 0, "duration": 480, "velocity": 60 }"#,
            "track 9",
        ),
        (
            "add on a channel MIDI does not have",
            r#"{ "kind": "add_note", "track": 1, "channel": 16, "pitch": 60,
                 "start": 0, "duration": 480, "velocity": 60 }"#,
            "channel 16",
        ),
        (
            "add at a pitch MIDI does not have",
            r#"{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 128,
                 "start": 0, "duration": 480, "velocity": 60 }"#,
            "pitch 128",
        ),
        (
            "add at velocity 0, which is how the format spells a note-off",
            r#"{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 60,
                 "start": 0, "duration": 480, "velocity": 0 }"#,
            "velocity 0",
        ),
        (
            "add a note of no length",
            r#"{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 60,
                 "start": 0, "duration": 0, "velocity": 60 }"#,
            "0 ticks",
        ),
        (
            "add a note before the start of the Take",
            r#"{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 60,
                 "start": -1, "duration": 480, "velocity": 60 }"#,
            "tick -1",
        ),
    ];

    for (index, (what, edit, said)) in cases.iter().enumerate() {
        let out = dir.path().join(format!("take-{index}.mid"));
        mid()
            .args(["apply", FIXTURE])
            .arg(edit_set(dir.path(), &format!("case-{index}"), edit))
            .arg("-o")
            .arg(&out)
            .assert()
            .failure()
            .stderr(predicates::str::contains(*said));
        assert!(!out.exists(), "{what} produced a Take");
    }
}

/// The other half of "Edits apply in the order given". Their targets were fixed
/// against the input Take, but their *effects* compound: a second Edit on a note
/// an earlier one already changed starts from where that one left it, not from
/// where the note began.
#[test]
fn two_edits_on_one_note_compound() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "compound",
            r#"{ "kind": "move_note",      "id": "t1:c0:p71:s1440:n0", "delta_ticks": -480 },
               { "kind": "move_note",      "id": "t1:c0:p71:s1440:n0", "delta_ticks": -240 },
               { "kind": "transpose_note", "id": "t1:c0:p67:s2400:n0", "semitones": 2 },
               { "kind": "transpose_note", "id": "t1:c0:p67:s2400:n0", "semitones": 3 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let after = common::note_ids(&common::inspect_json(&out));
    assert!(
        after.contains(&"t1:c0:p71:s720:n0".to_string()),
        "two moves of -480 and -240 did not compound to -720: {after:?}"
    );
    assert!(
        after.contains(&"t1:c0:p72:s2400:n0".to_string()),
        "two transposes of 2 and 3 did not compound to 5: {after:?}"
    );
}

/// The placement rule is not `add_note`'s alone. A move changes a note's start
/// Tick, which is content an identity is derived from, so it can land a note on
/// top of another and renumber it. ADR-0002 says it must not, and this is the
/// collision issue #1 records as never having been exercised.
#[test]
fn a_note_moved_on_top_of_another_takes_the_next_occurrence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    // Both are 955 ticks long. The note that moves is the one written *earlier*
    // in the track, so without the rule its note-on would sort ahead of the one
    // already at tick 1920 and take that note's name.

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "move-onto",
            r#"{ "kind": "move_note",    "id": "t2:c1:p54:s480:n0", "delta_ticks": 1440 },
               { "kind": "set_velocity", "id": "t2:c1:p54:s480:n0", "velocity": 90 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    assert_eq!(
        stacked_at(&out, 2, 54, 1920),
        vec![
            ("t2:c1:p54:s1920:n0".to_string(), 955, 38),
            ("t2:c1:p54:s1920:n1".to_string(), 955, 90),
        ],
        "the note already at tick 1920 lost its name to the one that moved on to it"
    );
}

/// And a transpose changes a note's pitch, which is the other half of what an
/// identity is derived from — so it can collide in exactly the same way.
#[test]
fn a_note_transposed_on_top_of_another_takes_the_next_occurrence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");
    // Both are 955 ticks long. Again the note that changes is written earlier in
    // the track than the one it lands on, so the rule is what decides this.

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "transpose-onto",
            r#"{ "kind": "transpose_note", "id": "t2:c1:p54:s480:n0", "semitones": 3 },
               { "kind": "set_velocity",   "id": "t2:c1:p54:s480:n0", "velocity": 90 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    assert_eq!(
        stacked_at(&out, 2, 57, 480),
        vec![
            ("t2:c1:p57:s480:n0".to_string(), 955, 38),
            ("t2:c1:p57:s480:n1".to_string(), 955, 90),
        ],
        "the note already at pitch 57 lost its name to the one transposed on to it"
    );
}

/// The delta time between two events is not a `u32` in the file; the format
/// packs it into 28 bits. `midly` masks a larger one rather than refusing, which
/// would write a smaller gap than was asked for and say nothing about it.
#[test]
fn a_move_too_far_to_write_as_a_delta_time_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", FIXTURE])
        .arg(edit_set(
            dir.path(),
            "far",
            r#"{ "kind": "move_note", "id": "t1:c0:p62:s10080:n0", "delta_ticks": 300000000 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .failure()
        .stderr(predicates::str::contains("268435455"));
    assert!(!out.exists());
}

/// The notes stacked at one track, pitch and start, in the order `inspect`
/// reports them — which is the order their occurrence indices are counted in.
///
/// Velocity is reported alongside the length because the fixture's stacked notes
/// are otherwise identical: every one of them is 955 ticks at velocity 38, so
/// without something telling them apart an assertion about *which* of the two
/// took `n0` would hold however they were ordered.
fn stacked_at(take: &Path, track: u64, pitch: u64, start: u64) -> Vec<(String, u64, u64)> {
    common::notes(&common::inspect_json(take))
        .iter()
        .filter(|note| note["track"] == track && note["pitch"] == pitch && note["start"] == start)
        .map(|note| {
            (
                note["id"].as_str().expect("id is a string").to_string(),
                note["duration"].as_u64().expect("duration is a number"),
                note["velocity"].as_u64().expect("velocity is a number"),
            )
        })
        .collect()
}
