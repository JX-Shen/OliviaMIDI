//! ADR-0002's occurrence index, against a Take that arrived already collided.
//!
//! The reference fixture cannot produce a collision: all 36 of its notes are
//! distinct in track, channel, pitch and start Tick, so the disambiguation path
//! is never taken while reading it. Until now the only collisions in the suite
//! were ones `apply` had just built, which tests the placement rule but leaves
//! the *reading* rule resting on the same code that wrote the file.
//!
//! `fixtures/stacked.mid` is hand-written, so the order of two note-ons sharing
//! a Tick is a fact about the file rather than a habit of our own builder. That
//! is the case ADR-0002 records as unexercised: "a Take arriving from elsewhere,
//! written by a program that ordered its simultaneous events differently."
//!
//! What the fixture contains — 480 PPQ, 4/4, three tracks, eleven notes:
//!
//! | | address | velocity | duration |
//! | --- | --- | --- | --- |
//! | a doubled voice | `t1:c0:p60:s0:n0` | 40 | 240 |
//! | | `t1:c0:p60:s0:n1` | 100 | 960 |
//! | a triple stack | `t1:c0:p64:s960:n0` | 60 | 120 |
//! | | `t1:c0:p64:s960:n1` | 30 | 240 |
//! | | `t1:c0:p64:s960:n2` | 90 | 480 |
//! | overtaken by a shorter note | `t1:c0:p62:s1920:n0` | 80 | 960 |
//! | | `t1:c0:p66:s2160:n0` | 55 | 240 |
//! | same pitch and Tick, other channel | `t2:c1:p60:s0:n0` | 70 | 480 |
//! | | `t2:c2:p60:s0:n0` | 50 | 480 |
//! | same channel and Tick, other pitch | `t2:c1:p55:s960:n0` | 65 | 480 |
//! | | `t2:c1:p59:s960:n0` | 45 | 480 |
//!
//! **Every colliding note differs in velocity, and that is the whole design.**
//! An identity is derived from track, channel, pitch and start Tick, so two
//! notes that collide agree on all four by definition. If they also agreed on
//! everything else, a swapped occurrence index would be undetectable — and in
//! fact harmless, because nothing could tell the two notes apart afterwards
//! either. Velocity is the free variable: it is carried on the note-on, so the
//! file can assign it per note. Duration cannot be chosen independently, because
//! a note-off names only a channel and a pitch and is paired with the oldest
//! note open on them, so the first-written of a collision always takes the first
//! release.
//!
//! The last two rows are controls. Neither is a collision — one differs in
//! channel and one in pitch — and both are here because an occurrence index that
//! was counted too coarsely would number them 0 and 1 and nothing else in the
//! suite would notice.

mod common;

use common::{edit_set, mid, STACKED};
use std::path::Path;

/// Every note as (identity, velocity, duration), in the order `--json` lists
/// them — the Take's own order, track by track and then note-on by note-on,
/// which is the order the indices are counted in. `inspect`'s human listing is
/// chronological across tracks instead, so the two interleave differently.
fn identified(take: &Path) -> Vec<(String, u64, u64)> {
    common::notes(&common::inspect_json(take))
        .iter()
        .map(|note| {
            (
                note["id"].as_str().expect("id is a string").to_string(),
                note["velocity"].as_u64().expect("velocity is a number"),
                note["duration"].as_u64().expect("duration is a number"),
            )
        })
        .collect()
}

fn owned(rows: &[(&str, u64, u64)]) -> Vec<(String, u64, u64)> {
    rows.iter()
        .map(|&(id, velocity, duration)| (id.to_string(), velocity, duration))
        .collect()
}

/// The fixture as `mid` reads it, asserted whole.
///
/// This is the pinning test: a `.mid` is opaque, so what stops the fixture from
/// quietly becoming a different file is that its entire reading is written out
/// here in terms a person can check.
#[test]
fn the_fixture_reads_as_the_collisions_it_was_built_to_hold() {
    assert_eq!(
        identified(Path::new(STACKED)),
        owned(&[
            ("t1:c0:p60:s0:n0", 40, 240),
            ("t1:c0:p60:s0:n1", 100, 960),
            ("t1:c0:p64:s960:n0", 60, 120),
            ("t1:c0:p64:s960:n1", 30, 240),
            ("t1:c0:p64:s960:n2", 90, 480),
            ("t1:c0:p62:s1920:n0", 80, 960),
            ("t1:c0:p66:s2160:n0", 55, 240),
            ("t2:c1:p60:s0:n0", 70, 480),
            ("t2:c2:p60:s0:n0", 50, 480),
            ("t2:c1:p55:s960:n0", 65, 480),
            ("t2:c1:p59:s960:n0", 45, 480),
        ])
    );
}

/// The foreign event order, pinned separately from what `mid` makes of it.
///
/// The point of a hand-written fixture is the order it puts two events sharing a
/// Tick in, and that order survives in the bytes alone — every assertion above
/// reads it through the very rule under test. Two orderings here are deliberate
/// and neither is what our own builder emits: the quiet half of the doubled
/// voice is struck first, and the release ending the loud half is written *after*
/// the three strikes that share its Tick.
#[test]
fn the_file_orders_its_simultaneous_events_the_way_it_was_written() {
    let events = common::note_events(Path::new(STACKED));
    assert_eq!(
        &events[..4],
        &[
            (0, "strikes", 60),
            (0, "strikes", 60),
            (240, "releases", 60),
            (960, "strikes", 64),
        ],
        "the doubled voice is not struck twice at Tick 0 before anything releases"
    );
    assert_eq!(
        &events[4..7],
        &[
            (960, "strikes", 64),
            (960, "strikes", 64),
            (960, "releases", 60)
        ],
        "the release at Tick 960 is not written behind the strikes sharing it"
    );
}

/// Which of two colliding notes is `n0` is decided by which note-on the file
/// wrote first, and by nothing else.
///
/// The fixture strikes the quiet note first, which is the order an encoder
/// listing "the double, then the melody" would produce and the opposite of the
/// one listing the melody first. Reading it any other way — loudest first,
/// longest first, or by which release came back soonest — gives a different
/// answer here, which is why the two differ in velocity at all.
#[test]
fn an_occurrence_index_follows_note_on_order_in_the_file() {
    let doubled: Vec<_> = identified(Path::new(STACKED))
        .into_iter()
        .filter(|(id, ..)| id.starts_with("t1:c0:p60:s0:"))
        .collect();

    assert_eq!(
        doubled,
        owned(&[("t1:c0:p60:s0:n0", 40, 240), ("t1:c0:p60:s0:n1", 100, 960)]),
        "the quieter note was written first and did not take n0"
    );
}

/// A stack of three numbers 0, 1, 2 — not 0, 1, 1, and not 0 and 1 with the
/// third silently taking one of their names.
///
/// The velocities are scrambled against the durations on purpose: 60/30/90 over
/// 120/240/480. Any rule that ordered the stack by either field would put a
/// different note in the middle.
#[test]
fn a_third_note_at_one_address_takes_a_third_index() {
    let stack: Vec<_> = identified(Path::new(STACKED))
        .into_iter()
        .filter(|(id, ..)| id.starts_with("t1:c0:p64:s960:"))
        .collect();

    assert_eq!(
        stack,
        owned(&[
            ("t1:c0:p64:s960:n0", 60, 120),
            ("t1:c0:p64:s960:n1", 30, 240),
            ("t1:c0:p64:s960:n2", 90, 480),
        ])
    );
}

/// Two notes sharing a track, pitch and Tick but not a channel are two
/// different notes, each `n0` of its own address.
#[test]
fn a_different_channel_at_the_same_pitch_and_tick_is_not_a_collision() {
    let ids = common::note_ids(&common::inspect_json(Path::new(STACKED)));
    assert!(ids.contains(&"t2:c1:p60:s0:n0".to_string()));
    assert!(ids.contains(&"t2:c2:p60:s0:n0".to_string()));
    assert!(
        !ids.contains(&"t2:c1:p60:s0:n1".to_string()),
        "two channels were counted as one address"
    );
}

/// And two notes sharing a track, channel and Tick but not a pitch — an
/// ordinary chord, which must not be numbered as a stack.
#[test]
fn a_chord_is_not_a_collision() {
    let ids = common::note_ids(&common::inspect_json(Path::new(STACKED)));
    assert!(ids.contains(&"t2:c1:p55:s960:n0".to_string()));
    assert!(ids.contains(&"t2:c1:p59:s960:n0".to_string()));
    assert!(
        !ids.contains(&"t2:c1:p55:s960:n1".to_string()),
        "two pitches were counted as one address"
    );
}

/// An index is a function of where a note *starts*, not of which note came back
/// first — so the listing is in note-on order even where a note is overtaken.
///
/// `t1:c0:p66:s2160` starts later than `t1:c0:p62:s1920` and ends sooner, so
/// pairing emits it first. Nothing here collides; what is under test is the sort
/// that makes the numbering safe when something does.
#[test]
fn notes_are_listed_in_note_on_order_and_not_in_the_order_they_end() {
    let tail: Vec<_> = identified(Path::new(STACKED))
        .into_iter()
        .filter(|(id, ..)| id.starts_with("t1:c0:p62:") || id.starts_with("t1:c0:p66:"))
        .collect();

    assert_eq!(
        tail,
        owned(&[
            ("t1:c0:p62:s1920:n0", 80, 960),
            ("t1:c0:p66:s2160:n0", 55, 240)
        ]),
        "the note that ended first was listed first"
    );
}

/// The question ADR-0002 leaves open: whether a write and a re-read keep the
/// indices pointing at the notes they pointed at.
///
/// "If serialisation reorders events sharing a tick, the occurrence indices swap
/// and nothing reports an error." Nothing could — both files parse, both are
/// valid, and every identity still resolves. The failure would be silent and
/// would land an Edit Set written against the first file on the wrong note of
/// the second. It is asserted on the reading rather than on the bytes, because
/// ADR-0003 lets the encoding differ and it does: `midly` writes running status
/// where the fixture spells every status byte out.
#[test]
fn a_collision_survives_a_write_and_a_re_read() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", STACKED])
        .arg(common::empty_edit_set(dir.path()))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    assert_eq!(
        identified(&out),
        identified(Path::new(STACKED)),
        "a Take that changed nothing came back with its notes renamed"
    );
    assert_eq!(
        common::event_stream(&out),
        common::event_stream(Path::new(STACKED)),
        "the events sharing a Tick were reordered by the round trip"
    );
}

/// And that it survives being written repeatedly, which is what a session
/// actually does — every `apply` reads the Take the last one produced.
///
/// A reordering that is stable would pass the single round trip above while
/// still drifting one place per write.
#[test]
fn repeated_writes_do_not_drift() {
    let dir = tempfile::tempdir().expect("temp dir");
    let empty = common::empty_edit_set(dir.path());
    let mut current = Path::new(STACKED).to_path_buf();

    for generation in 1..=5 {
        let next = dir.path().join(format!("take-{generation:02}.mid"));
        mid()
            .arg("apply")
            .arg(&current)
            .arg(&empty)
            .arg("-o")
            .arg(&next)
            .assert()
            .success();
        assert_eq!(
            identified(&next),
            identified(Path::new(STACKED)),
            "the notes were renamed by write {generation}"
        );
        current = next;
    }
}

/// The failure ADR-0002 rejected lazy resolution for, run against a real
/// collision.
///
/// Deleting `n0` of the doubled voice renumbers `n1` to `n0`. An Edit Set that
/// deletes the first and then edits the second must still reach the note it
/// named — under resolution-at-run-time the second Edit would either fail to
/// find `n1` or, worse, land on the note the first Edit had just renamed.
#[test]
fn deleting_the_first_of_a_collision_does_not_move_a_later_edits_target() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", STACKED])
        .arg(edit_set(
            dir.path(),
            "kill-n0",
            r#"{ "kind": "delete_note",  "id": "t1:c0:p60:s0:n0" },
               { "kind": "set_velocity", "id": "t1:c0:p60:s0:n1", "velocity": 50 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let doubled: Vec<_> = identified(&out)
        .into_iter()
        .filter(|(id, ..)| id.starts_with("t1:c0:p60:s0:"))
        .collect();
    assert_eq!(
        doubled,
        // The survivor is the 960-tick note, and it is now the only one at this
        // address, so it is `n0`. Its velocity is the evidence: 50 says the
        // second Edit reached the note that was `n1`, and 100 would say it was
        // never touched.
        owned(&[("t1:c0:p60:s0:n0", 50, 960)])
    );
}

/// The same Edit Set the other way round. Effects are ordered; targets were
/// fixed before the first Edit ran, so the order cannot change which notes
/// they reach.
#[test]
fn the_order_of_edits_does_not_move_their_targets() {
    let dir = tempfile::tempdir().expect("temp dir");
    let forwards = dir.path().join("forwards.mid");
    let backwards = dir.path().join("backwards.mid");

    mid()
        .args(["apply", STACKED])
        .arg(edit_set(
            dir.path(),
            "forwards",
            r#"{ "kind": "delete_note",  "id": "t1:c0:p60:s0:n0" },
               { "kind": "set_velocity", "id": "t1:c0:p60:s0:n1", "velocity": 50 }"#,
        ))
        .arg("-o")
        .arg(&forwards)
        .assert()
        .success();

    mid()
        .args(["apply", STACKED])
        .arg(edit_set(
            dir.path(),
            "backwards",
            r#"{ "kind": "set_velocity", "id": "t1:c0:p60:s0:n1", "velocity": 50 },
               { "kind": "delete_note",  "id": "t1:c0:p60:s0:n0" }"#,
        ))
        .arg("-o")
        .arg(&backwards)
        .assert()
        .success();

    assert_eq!(identified(&forwards), identified(&backwards));
}

/// Deleting the middle of three closes the gap rather than leaving a hole.
///
/// An index is positional, so after this the survivors are `n0` and `n1` — and
/// which is which is the assertion: the velocities say the two that remain are
/// the outer pair, in the order they were struck.
#[test]
fn deleting_the_middle_of_a_stack_renumbers_what_is_left() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", STACKED])
        .arg(edit_set(
            dir.path(),
            "kill-middle",
            r#"{ "kind": "delete_note", "id": "t1:c0:p64:s960:n1" }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let stack: Vec<_> = identified(&out)
        .into_iter()
        .filter(|(id, ..)| id.starts_with("t1:c0:p64:s960:"))
        .collect();
    assert_eq!(
        stack,
        owned(&[
            ("t1:c0:p64:s960:n0", 60, 120),
            ("t1:c0:p64:s960:n1", 90, 480)
        ]),
        "a deleted note left a hole in the numbering instead of closing it"
    );
}

/// Three Edits, one per member of a stack, each reaching its own note.
///
/// This is the disambiguation rule doing the only job it exists for. The
/// velocities written in are distinct and are matched against the durations,
/// which no Edit here touches — so the assertion is that the three landed on
/// the three notes rather than that three notes changed.
#[test]
fn each_member_of_a_stack_takes_its_own_edit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", STACKED])
        .arg(edit_set(
            dir.path(),
            "all-three",
            r#"{ "kind": "set_velocity", "id": "t1:c0:p64:s960:n0", "velocity": 11 },
               { "kind": "set_velocity", "id": "t1:c0:p64:s960:n1", "velocity": 22 },
               { "kind": "set_velocity", "id": "t1:c0:p64:s960:n2", "velocity": 33 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let stack: Vec<_> = identified(&out)
        .into_iter()
        .filter(|(id, ..)| id.starts_with("t1:c0:p64:s960:"))
        .collect();
    assert_eq!(
        stack,
        owned(&[
            ("t1:c0:p64:s960:n0", 11, 120),
            ("t1:c0:p64:s960:n1", 22, 240),
            ("t1:c0:p64:s960:n2", 33, 480),
        ]),
        "an Edit named one member of the stack and reached another"
    );
}

/// A note landing on an address that already holds three takes the fourth
/// index, and the three keep theirs.
///
/// `apply`'s placement rule is exercised elsewhere against a collision of two.
/// What is new here is that the notes it must not disturb are distinguishable:
/// in the built fixtures the stacked notes are identical, so an assertion that
/// they kept their names would hold however they were ordered.
#[test]
fn a_note_added_to_a_full_stack_takes_the_next_index() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", STACKED])
        .arg(edit_set(
            dir.path(),
            "onto-the-stack",
            r#"{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 64,
                 "start": 960, "duration": 720, "velocity": 77 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let stack: Vec<_> = identified(&out)
        .into_iter()
        .filter(|(id, ..)| id.starts_with("t1:c0:p64:s960:"))
        .collect();
    assert_eq!(
        stack,
        owned(&[
            ("t1:c0:p64:s960:n0", 60, 120),
            ("t1:c0:p64:s960:n1", 30, 240),
            ("t1:c0:p64:s960:n2", 90, 480),
            ("t1:c0:p64:s960:n3", 77, 720),
        ])
    );
}

/// A note moved and transposed onto the full stack in one Edit Set arrives
/// behind it, for the same reason an added one does.
///
/// Both Edits name the same note, and both change something its identity is
/// derived from. It is the sharpest form of the placement rule: the note is not
/// new, it is written earlier in the track than every note it lands among, and
/// only placing it last leaves their names alone.
#[test]
fn a_note_moved_and_transposed_onto_a_full_stack_arrives_behind_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", STACKED])
        .arg(edit_set(
            dir.path(),
            "onto-by-edit",
            r#"{ "kind": "move_note",      "id": "t1:c0:p62:s1920:n0", "delta_ticks": -960 },
               { "kind": "transpose_note", "id": "t1:c0:p62:s1920:n0", "semitones": 2 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let stack: Vec<_> = identified(&out)
        .into_iter()
        .filter(|(id, ..)| id.starts_with("t1:c0:p64:s960:"))
        .collect();
    assert_eq!(
        stack,
        owned(&[
            ("t1:c0:p64:s960:n0", 60, 120),
            ("t1:c0:p64:s960:n1", 30, 240),
            ("t1:c0:p64:s960:n2", 90, 480),
            ("t1:c0:p64:s960:n3", 80, 960),
        ]),
        "the notes already at this address were renumbered by one arriving on it"
    );
}

/// `diff` pairs by identity first, and inside a stack that is the only pass
/// that can run: three notes share a track, channel, pitch and start, so
/// nearest-neighbour has nothing to separate them by.
#[test]
fn diff_reaches_one_member_of_a_stack() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", STACKED])
        .arg(edit_set(
            dir.path(),
            "soften-the-middle",
            r#"{ "kind": "set_velocity", "id": "t1:c0:p64:s960:n1", "velocity": 99 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let output = mid()
        .args(["diff", "--json", STACKED])
        .arg(&out)
        .output()
        .expect("mid runs");
    let diff: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("diff --json is JSON");

    assert_eq!(diff["added"].as_array().expect("added is a list").len(), 0);
    assert_eq!(
        diff["removed"].as_array().expect("removed is a list").len(),
        0
    );
    let changed = diff["changed"].as_array().expect("changed is a list");
    assert_eq!(
        changed.len(),
        1,
        "a stack of three reported as more than one change"
    );
    assert_eq!(changed[0]["before"]["id"], "t1:c0:p64:s960:n1");
    assert_eq!(changed[0]["before"]["velocity"], 30);
    assert_eq!(changed[0]["after"]["velocity"], 99);
}

/// Two Takes that collide identically report no difference — the case where a
/// swapped index would show up as two changes that cancel out.
#[test]
fn a_take_does_not_differ_from_its_own_round_trip() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", STACKED])
        .arg(common::empty_edit_set(dir.path()))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let output = mid()
        .args(["diff", "--json", STACKED])
        .arg(&out)
        .output()
        .expect("mid runs");
    let diff: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("diff --json is JSON");

    assert_eq!(diff["added"].as_array().expect("added is a list").len(), 0);
    assert_eq!(
        diff["removed"].as_array().expect("removed is a list").len(),
        0
    );
    assert_eq!(
        diff["changed"].as_array().expect("changed is a list").len(),
        0
    );
}

/// The fixture is an input, like `fixtures/olivia.mid`, and nothing in the
/// suite may write to it.
#[test]
fn the_fixture_is_left_untouched() {
    let before = std::fs::read(STACKED).expect("the fixture is readable");
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    mid()
        .args(["apply", STACKED])
        .arg(edit_set(
            dir.path(),
            "anything",
            r#"{ "kind": "set_velocity", "id": "t1:c0:p64:s960:n1", "velocity": 99 }"#,
        ))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    assert_eq!(
        std::fs::read(STACKED).expect("the fixture is readable"),
        before
    );
}
