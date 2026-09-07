mod common;

use common::{build_take, edit_set, first_note_id, mid, set_velocity_edit_set, FIXTURE};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// A Take derived from the fixture by an Edit Set spelled out at the call site.
fn derived(dir: &Path, name: &str, edits: &str) -> PathBuf {
    derived_from(dir, FIXTURE, name, edits)
}

/// The same, from whichever Take the test names — `common::STACKED` where what
/// is under test only happens to a Take that collides.
fn derived_from(dir: &Path, take: &str, name: &str, edits: &str) -> PathBuf {
    let out = dir.join(format!("{name}.mid"));
    mid()
        .args(["apply", take])
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

/// Two Takes holding one note at one Tick, the second one having replaced it
/// with a different pitch.
///
/// The distance the tolerance bounds is zero here, so this is the case that
/// separates *matching by identity* from *matching by nearness*: identity says
/// two notes, nearness says one note transposed, and only the tolerance decides
/// which claim the diff is entitled to make.
fn substituted_by(dir: &Path, semitones: u8) -> (PathBuf, PathBuf) {
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
        &[(480, 240, 60 + semitones)],
    );
    (before, after)
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

/// The case a distance cannot reject, because the distance is zero.
///
/// Moving a note far enough puts it outside the tolerance and the bound does the
/// work. A note *replaced* at its own Tick is never outside any bound, so at
/// zero the first pass has to be the only pass — or the diff answers a caller
/// who asked it not to infer with an inference.
#[test]
fn a_tolerance_of_zero_does_not_pair_a_substituted_pitch_with_what_it_replaced() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (before, after) = substituted_by(dir.path(), 2);

    let diff = diff_json(&before, &after, Some("0"));
    assert_eq!(list(&diff, "changed").len(), 0);
    assert_eq!(list(&diff, "removed")[0]["id"], "t1:c0:p60:s480:n0");
    assert_eq!(list(&diff, "added")[0]["id"], "t1:c0:p62:s480:n0");
}

/// The other side of the boundary: the inference is still there to be asked
/// for, and asking for it is what a tolerance above zero does.
#[test]
fn the_same_substitution_at_the_default_tolerance_is_one_transposed_note() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (before, after) = substituted_by(dir.path(), 2);

    let diff = diff_json(&before, &after, None);
    assert_eq!(list(&diff, "added").len(), 0);
    assert_eq!(list(&diff, "removed").len(), 0);
    assert_eq!(
        changed(&diff),
        vec![(
            "t1:c0:p60:s480:n0".to_string(),
            "t1:c0:p62:s480:n0".to_string(),
            vec!["pitch".to_string()]
        )]
    );
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
    // the same reasoning as the Rig disclosure in ADR-0005.
    mid()
        .arg("diff")
        .arg(&before)
        .arg(&after)
        .assert()
        .success()
        .stderr(predicates::str::contains("120"));
}

/// A changed note reads as a description: which note it is, then what about it
/// is different, in the fixed order the library reports. Both facts of a note
/// that was transposed *and* softened are on the one line, because it is one
/// note.
///
/// The note is named the way `inspect` names it and the way an added or removed
/// note is described — position, track, pitch — because a row that named only a
/// position would be true of every note of a chord. See #14.
///
/// A pitch change reads as *transposed to*, for the reason a `start` change
/// reads as *moved to*: the row opens with the note as it was, so naming the
/// pitch it came from would put `A4` on the line twice.
///
/// The identities are still not here. They are what an Edit Set names, and
/// `--json` carries both of them; a human reading this wants to know what
/// happened to the music, and two forty-character identities per line is what
/// stops them.
#[test]
fn reads_as_a_description_of_what_changed() {
    let dir = tempfile::tempdir().expect("temp dir");
    let id = first_note_id();
    let after = derived(
        dir.path(),
        "transposed-and-softened",
        &format!(
            r#"{{ "kind": "transpose_note", "id": "{id}", "semitones": -2 }},
               {{ "kind": "set_velocity",   "id": "{id}", "velocity": 40 }}"#
        ),
    );
    assert_eq!(
        common::human_output(&["diff", FIXTURE, after.to_str().expect("a path")]),
        "changed  bar 1 beat 1  track 1  A4  transposed to G4, velocity 50 -> 40\n"
    );
}

/// A note that moved is reported as *moved*, in Bars and Beats, so that "it came
/// in a sixteenth early" is readable as that rather than as two Tick counts.
///
/// Where it was is the row's own position column, so the clause says only where
/// it went. Naming both would put `bar 1 beat 1` on the line twice.
#[test]
fn says_where_a_moved_note_went_in_bars_and_beats() {
    let dir = tempfile::tempdir().expect("temp dir");
    let id = first_note_id();
    let after = derived(
        dir.path(),
        "nudged",
        &format!(r#"{{ "kind": "move_note", "id": "{id}", "delta_ticks": 60 }}"#),
    );
    assert_eq!(
        common::human_output(&["diff", FIXTURE, after.to_str().expect("a path")]),
        "changed  bar 1 beat 1  track 1  A4  moved to bar 1 beat 1+60\n"
    );
}

/// The failure #14 was opened for: two notes of a chord share a position and a
/// track, so a row carrying only those two is true of both.
///
/// `fixtures/olivia.mid` has such a chord — an F#3 and an A3 struck together on
/// track 2, both at velocity 38 — so the row this used to print, `changed bar 7
/// beat 2 track 2 velocity 38 -> 60`, described either of them. The pitch is
/// what settles it, and a chord is common enough that this was the ordinary
/// case rather than an exotic one.
#[test]
fn names_the_note_of_a_chord_that_changed() {
    let dir = tempfile::tempdir().expect("temp dir");
    let after = derived(
        dir.path(),
        "one-of-a-chord",
        r#"{ "kind": "set_velocity", "id": "t2:c1:p54:s9120:n0", "velocity": 60 }"#,
    );
    assert_eq!(
        common::human_output(&["diff", FIXTURE, after.to_str().expect("a path")]),
        "changed  bar 7 beat 2  track 2  F#3  velocity 38 -> 60\n",
        "the row does not say which note of the chord changed"
    );
}

/// And where the notes genuinely collide, naming the pitch is not enough either
/// — so the row says which occurrence, in the identity's own spelling.
///
/// `fixtures/stacked.mid` holds three E4s at one address and a doubled C4 at
/// another. Nothing musical separates them: they agree on track, channel, pitch
/// and start Tick, which is what a collision is. `E4 n1` is the note `inspect`
/// lists as `t1:c0:p64:s960:n1`, so a human reading this row can find it.
#[test]
fn names_which_of_a_collision_changed() {
    let dir = tempfile::tempdir().expect("temp dir");
    let after = derived_from(
        dir.path(),
        common::STACKED,
        "one-of-a-stack",
        r#"{ "kind": "set_velocity", "id": "t1:c0:p64:s960:n1", "velocity": 99 },
           { "kind": "move_note",    "id": "t1:c0:p60:s0:n1",   "delta_ticks": 120 }"#,
    );
    assert_eq!(
        common::human_output(&["diff", common::STACKED, after.to_str().expect("a path")]),
        "\
changed  bar 1 beat 1  track 1  C4 n1  moved to bar 1 beat 1+120
changed  bar 1 beat 3  track 1  E4 n1  velocity 30 -> 99
",
        "the row does not say which note of the collision changed"
    );
}

/// A note that is alone at its address is named without an occurrence, in the
/// same Take that has collisions elsewhere.
///
/// The disambiguator is per address, not per Take: it appears where the music
/// cannot name a note and nowhere else, so an ordinary row does not pay for a
/// collision two Bars away.
#[test]
fn does_not_disambiguate_a_note_nothing_collides_with() {
    let dir = tempfile::tempdir().expect("temp dir");
    let after = derived_from(
        dir.path(),
        common::STACKED,
        "the-lonely-one",
        r#"{ "kind": "set_velocity", "id": "t1:c0:p62:s1920:n0", "velocity": 20 }"#,
    );
    assert_eq!(
        common::human_output(&["diff", common::STACKED, after.to_str().expect("a path")]),
        "changed  bar 2 beat 1  track 1  D4  velocity 80 -> 20\n"
    );
}

/// A note that arrived and one that left are described the same way a note is
/// listed by `inspect`, and in the same columns.
#[test]
fn describes_an_added_and_a_removed_note_as_notes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let id = first_note_id();
    let after = derived(
        dir.path(),
        "swapped",
        &format!(
            r#"{{ "kind": "delete_note", "id": "{id}" }},
               {{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 71,
                  "start": 2880, "duration": 480, "velocity": 60 }}"#
        ),
    );
    assert_eq!(
        common::human_output(&["diff", FIXTURE, after.to_str().expect("a path")]),
        "\
added    bar 3 beat 1  track 1  B4  velocity 60  duration 480
removed  bar 1 beat 1  track 1  A4  velocity 50  duration 1435
"
    );
}

/// Two Takes that differ in nothing say so, rather than printing nothing at all.
#[test]
fn says_when_two_takes_differ_in_nothing() {
    assert_eq!(
        common::human_output(&["diff", FIXTURE, FIXTURE]),
        "no differences\n"
    );
}

/// A Take with a bar of notes and whatever else the test puts on its voice
/// track, so that a comparison can be about one state and nothing else.
///
/// 480 Ticks to the quarter note and 4/4 throughout, which makes Bar 2 Tick
/// 1920 and every position in these tests readable. `build_take_setting`
/// already states 120 to the minute at Tick 0, so two Takes built this way
/// agree about the tempo until one of them says otherwise.
fn stating(dir: &Path, name: &str, setting: &[(u32, midly::TrackEventKind<'static>)]) -> PathBuf {
    common::build_take_setting(
        &dir.join(format!("{name}.mid")),
        480,
        &[(0, 4, 4)],
        setting,
        &[
            (0, 480, 60),
            (480, 480, 62),
            (960, 480, 64),
            (1440, 480, 65),
        ],
    )
}

/// The same Take, stating no tempo at all — not even the 120 at Tick 0 that
/// `stating` writes on the conductor track.
fn stating_no_tempo(
    dir: &Path,
    name: &str,
    setting: &[(u32, midly::TrackEventKind<'static>)],
) -> PathBuf {
    common::build_take_stating_no_tempo(
        &dir.join(format!("{name}.mid")),
        480,
        &[(0, 4, 4)],
        setting,
        &[
            (0, 480, 60),
            (480, 480, 62),
            (960, 480, 64),
            (1440, 480, 65),
        ],
    )
}

/// Every note-level list of a diff, so a test about one state can say that the
/// notes are not what it found.
fn notes_agree(diff: &Value) {
    assert!(list(diff, "added").is_empty(), "added: {:?}", diff["added"]);
    assert!(
        list(diff, "removed").is_empty(),
        "removed: {:?}",
        diff["removed"]
    );
    assert!(
        list(diff, "changed").is_empty(),
        "changed: {:?}",
        diff["changed"]
    );
}

/// The hole #32 names: until tempo was compared, `mid diff` answered "no
/// differences" and exited 0 about two Takes at different tempos.
#[test]
fn a_tempo_only_change_is_a_difference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating(dir.path(), "before", &[]);
    let after = stating(dir.path(), "after", &[common::tempo(1920, 428_571)]);

    let diff = diff_json(&before, &after, None);
    notes_agree(&diff);
    let tempos = list(&diff, "tempos");
    assert_eq!(tempos.len(), 1, "tempos: {tempos:?}");
    assert_eq!(tempos[0]["from"], 1920);
    assert_eq!(tempos[0]["until"], Value::Null);
    assert_eq!(
        tempos[0]["before"]["at_start"]["micros_per_quarter"],
        500_000
    );
    assert_eq!(
        tempos[0]["after"]["at_start"]["micros_per_quarter"],
        428_571
    );

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "tempo  bar 2 beat 1 onwards  120 -> 140\n"
    );
}

/// The same hole for the other state ADR-0007's Consequences name.
#[test]
fn a_pitch_bend_only_change_is_a_difference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating(dir.path(), "before", &[common::pitch_bend(960, 0)]);
    let after = stating(dir.path(), "after", &[common::pitch_bend(960, -2048)]);

    let diff = diff_json(&before, &after, None);
    notes_agree(&diff);
    let bends = list(&diff, "bends");
    assert_eq!(bends.len(), 1, "bends: {bends:?}");
    assert_eq!(bends[0]["channel"], 0);
    assert_eq!(bends[0]["from"], 960);
    assert_eq!(bends[0]["before"]["at_start"], 0);
    assert_eq!(bends[0]["after"]["at_start"], -2048);

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "bend  bar 1 beat 3 onwards  channel 0  0 -> -2048\n"
    );
}

/// What `BendSide` carries two extremes for. The span opens on a difference of
/// one unit and contains a dive of four thousand; a side reporting only a peak
/// would print nothing about the dive, because a bend below where the span
/// began never rises above it.
#[test]
fn a_dive_inside_a_bend_span_is_reported_although_it_is_no_peak() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating(
        dir.path(),
        "before",
        &[
            common::pitch_bend(0, 0),
            common::pitch_bend(480, 100),
            common::pitch_bend(1920, 0),
        ],
    );
    let after = stating(
        dir.path(),
        "after",
        &[
            common::pitch_bend(0, 0),
            common::pitch_bend(480, 101),
            common::pitch_bend(960, -4096),
            common::pitch_bend(1440, 101),
            common::pitch_bend(1920, 0),
        ],
    );

    let diff = diff_json(&before, &after, None);
    let bends = list(&diff, "bends");
    assert_eq!(bends.len(), 1, "bends: {bends:?}");
    let side = &bends[0]["after"];
    assert_eq!(side["at_start"], 101);
    assert_eq!(side["furthest_up"], 101);
    assert_eq!(side["furthest_down"], -4096);
    assert_eq!(side["furthest_down_at"], 960);

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "bend      bar 1 beat 2 until bar 2 beat 1  channel 0\n\
         \x20 before  100\n\
         \x20 after   101, down to -4096 at bar 1 beat 3\n"
    );
}

/// #13's difficulty, in the tempo's terms: an accelerando is written as a run
/// of statements and is one difference, not forty.
#[test]
fn an_accelerando_is_one_difference_and_not_one_per_statement() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ramp: Vec<_> = (0..40)
        .map(|step| common::tempo(1920 + step * 24, 500_000 - step * 1_800))
        .collect();
    let before = stating(dir.path(), "before", &[]);
    let after = stating(dir.path(), "after", &ramp);

    let diff = diff_json(&before, &after, None);
    let tempos = list(&diff, "tempos");
    assert_eq!(tempos.len(), 1, "tempos: {tempos:?}");
    // Not 1920, where the ramp begins: its first statement restates the 120 the
    // Take was already at, and restating what is in force is not a difference.
    // The two Takes stop agreeing at the second statement.
    assert_eq!(tempos[0]["from"], 1944);
    // The fastest the ramp reaches, which is the smallest number of
    // microseconds in it — the one field that says the accelerando happened
    // rather than that the tempo changed once.
    assert_eq!(
        tempos[0]["after"]["fastest"]["micros_per_quarter"],
        500_000 - 39 * 1_800
    );
    assert_eq!(tempos[0]["after"]["fastest_at"], 1920 + 39 * 24);
}

/// Nought is a value. A channel bent back to the centre and a channel never
/// bent are two different Pieces, the distinction #12's sixth criterion draws
/// for a Controller.
#[test]
fn a_channel_bent_to_the_centre_differs_from_one_never_bent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating(dir.path(), "before", &[]);
    let after = stating(dir.path(), "after", &[common::pitch_bend(960, 0)]);

    let diff = diff_json(&before, &after, None);
    let bends = list(&diff, "bends");
    assert_eq!(bends.len(), 1, "bends: {bends:?}");
    assert_eq!(bends[0]["before"]["at_start"], Value::Null);
    assert_eq!(bends[0]["after"]["at_start"], 0);

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "bend  bar 1 beat 3 onwards  channel 0  unstated -> 0\n"
    );
}

/// The other side of the boundary: a Take carrying both new states still does
/// not differ from itself.
#[test]
fn a_take_stating_a_tempo_and_a_bend_does_not_differ_from_itself() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = stating(
        dir.path(),
        "take",
        &[common::tempo(960, 428_571), common::pitch_bend(1440, -3000)],
    );
    let path = take.to_str().expect("a path");

    let diff = diff_json(&take, &take, None);
    assert!(list(&diff, "tempos").is_empty(), "{:?}", diff["tempos"]);
    assert!(list(&diff, "bends").is_empty(), "{:?}", diff["bends"]);
    assert_eq!(
        common::human_output(&["diff", path, path]),
        "no differences\n"
    );
}

/// The other extreme of `TempoSide`, which nothing else asks for: a span that
/// slows down. Every other tempo test here either quickens or holds, so
/// `slowest` and `slowest_at` could both be `None` and the suite stayed green.
/// See #32.
#[test]
fn a_ritardando_reports_the_slowest_tempo_reached_inside_the_span() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating(dir.path(), "before", &[]);
    // 150 from Bar 1 Beat 3, dropping to 60 for a Beat and picking 150 back up,
    // so the span's two ends agree and the slowing is only visible inside it.
    let after = stating(
        dir.path(),
        "after",
        &[
            common::tempo(960, 400_000),
            common::tempo(1440, 1_000_000),
            common::tempo(1680, 400_000),
        ],
    );

    let diff = diff_json(&before, &after, None);
    let tempos = list(&diff, "tempos");
    assert_eq!(tempos.len(), 1, "tempos: {tempos:?}");
    assert_eq!(
        tempos[0]["after"]["slowest"]["micros_per_quarter"],
        1_000_000
    );
    assert_eq!(tempos[0]["after"]["slowest_at"], 1440);

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "tempo     bar 1 beat 3 onwards\n\
         \x20 before  120\n\
         \x20 after   150, down to 60 at bar 1 beat 4\n"
    );
}

/// A span that moves both ways states both extremes, in the order they happened
/// — the reason `TempoSide` carries two of them rather than one. Swapping the
/// two verbs, or reporting either extreme alone, is a row that describes a
/// different shape of tempo change from the one in the file.
#[test]
fn a_tempo_span_that_moves_both_ways_states_both_extremes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating(dir.path(), "before", &[common::tempo(960, 250_000)]);
    // 120 where the span opens, up to 180, down to 60, and back to 120 so that
    // the span's ends agree and both excursions are what is left to report.
    let after = stating(
        dir.path(),
        "after",
        &[
            common::tempo(960, 500_000),
            common::tempo(1440, 333_333),
            common::tempo(1920, 1_000_000),
            common::tempo(2400, 500_000),
        ],
    );

    let diff = diff_json(&before, &after, None);
    let tempos = list(&diff, "tempos");
    assert_eq!(tempos.len(), 1, "tempos: {tempos:?}");
    assert_eq!(tempos[0]["after"]["fastest_at"], 1440);
    assert_eq!(tempos[0]["after"]["slowest_at"], 1920);

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "tempo     bar 1 beat 3 onwards\n\
         \x20 before  240\n\
         \x20 after   120, up to 180 at bar 1 beat 4, down to 60 at bar 2 beat 1\n"
    );
}

/// Where a span leaves each Take is not where it found it, and both states say
/// so: `at_end` was carried by `TempoSide` and `BendSide` and asserted by
/// nothing, so either could have been the span's opening value instead. See #32.
///
/// One Take for both, because the clause is one sentence of the reading shared
/// by every state compared as a span.
#[test]
fn a_span_that_ends_elsewhere_than_it_began_says_where_it_ends() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating(
        dir.path(),
        "before",
        &[common::tempo(960, 250_000), common::pitch_bend(960, 100)],
    );
    // Each state opens the span at one reading, reaches an extreme, and settles
    // on a third — so the value it ends at is neither the one it began at nor
    // either extreme, and only `at_end` can report it.
    let after = stating(
        dir.path(),
        "after",
        &[
            common::tempo(960, 500_000),
            common::tempo(1440, 333_333),
            common::tempo(1920, 400_000),
            common::pitch_bend(960, 200),
            common::pitch_bend(1440, 3000),
            common::pitch_bend(1920, 1000),
        ],
    );

    let diff = diff_json(&before, &after, None);
    let tempos = list(&diff, "tempos");
    assert_eq!(tempos.len(), 1, "tempos: {tempos:?}");
    assert_eq!(tempos[0]["after"]["at_end"]["micros_per_quarter"], 400_000);
    let bends = list(&diff, "bends");
    assert_eq!(bends.len(), 1, "bends: {bends:?}");
    assert_eq!(bends[0]["after"]["at_end"], 1000);

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "tempo     bar 1 beat 3 onwards\n\
         \x20 before  240\n\
         \x20 after   120, up to 180 at bar 1 beat 4, ends at 150\n\
         bend      bar 1 beat 3 onwards  channel 0\n\
         \x20 before  100\n\
         \x20 after   200, up to 3000 at bar 1 beat 4, ends at 1000\n"
    );
}

/// A bend is channel state, and the channel it is on is 3 as readily as 0.
/// Every bend the suite could build was on channel 0, so a comparison that
/// looked at that channel alone answered every test correctly and the Piece
/// wrongly.
#[test]
fn a_bend_on_a_channel_other_than_the_first_is_a_difference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating(dir.path(), "before", &[]);
    let after = stating(dir.path(), "after", &[common::pitch_bend_on(3, 960, -2048)]);

    let diff = diff_json(&before, &after, None);
    notes_agree(&diff);
    let bends = list(&diff, "bends");
    assert_eq!(bends.len(), 1, "bends: {bends:?}");
    assert_eq!(bends[0]["channel"], 3);
    assert_eq!(bends[0]["before"]["at_start"], Value::Null);
    assert_eq!(bends[0]["after"]["at_start"], -2048);

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "bend  bar 1 beat 3 onwards  channel 3  unstated -> -2048\n"
    );
}

/// `mid diff --help`'s claim, tested: a Take that states no tempo and one that
/// states 120 are different Pieces. Nothing could hold the first of them until
/// `build_take_stating_no_tempo` existed, so defaulting a tempo-less Take to 120
/// reversed the promise and broke no test.
#[test]
fn a_take_stating_no_tempo_differs_from_one_stating_120() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating_no_tempo(dir.path(), "before", &[]);
    let after = stating(dir.path(), "after", &[]);

    let diff = diff_json(&before, &after, None);
    notes_agree(&diff);
    let tempos = list(&diff, "tempos");
    assert_eq!(tempos.len(), 1, "tempos: {tempos:?}");
    assert_eq!(tempos[0]["from"], 0);
    assert_eq!(tempos[0]["before"]["at_start"], Value::Null);
    assert_eq!(
        tempos[0]["after"]["at_start"]["micros_per_quarter"],
        500_000
    );

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "tempo  bar 1 beat 1 onwards  unstated -> 120\n"
    );
}

/// An `unstated` side is not a side with nothing to say. The span belongs to the
/// other Take, and this one may move about inside it: where it began is unknown,
/// and both extremes and where it ends are not.
#[test]
fn an_unstated_tempo_side_still_reports_the_interior_of_the_span() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating_no_tempo(
        dir.path(),
        "before",
        &[common::tempo(960, 200_000), common::tempo(1440, 1_200_000)],
    );
    let after = stating(dir.path(), "after", &[]);

    let diff = diff_json(&before, &after, None);
    let tempos = list(&diff, "tempos");
    assert_eq!(tempos.len(), 1, "tempos: {tempos:?}");
    let side = &tempos[0]["before"];
    assert_eq!(side["at_start"], Value::Null);
    assert_eq!(side["fastest"]["micros_per_quarter"], 200_000);
    assert_eq!(side["fastest_at"], 960);
    assert_eq!(side["slowest"]["micros_per_quarter"], 1_200_000);
    assert_eq!(side["slowest_at"], 1440);
    assert_eq!(side["at_end"]["micros_per_quarter"], 1_200_000);

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "tempo     bar 1 beat 1 onwards\n\
         \x20 before  unstated, up to 300 at bar 1 beat 3, down to 50 at bar 1 beat 4, ends at 50\n\
         \x20 after   120\n"
    );
}

/// A row that asserts a difference and then prints the same number on both
/// sides has told the reader nothing. Beats are a rounding, so two tempos four
/// microseconds apart print as one number, and where that happens the
/// microseconds go beside every tempo in the difference.
#[test]
fn two_tempos_that_print_as_the_same_beats_disclose_their_microseconds() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating(dir.path(), "before", &[]);
    let after = stating(dir.path(), "after", &[common::tempo(960, 499_996)]);

    let diff = diff_json(&before, &after, None);
    let tempos = list(&diff, "tempos");
    assert_eq!(tempos.len(), 1, "tempos: {tempos:?}");

    let said = common::human_output(&[
        "diff",
        before.to_str().expect("a path"),
        after.to_str().expect("a path"),
    ]);
    assert!(
        !said.contains("120 -> 120"),
        "the row shows the reader no difference: {said}"
    );
    assert_eq!(
        said,
        "tempo  bar 1 beat 3 onwards  120 (500000 us) -> 120 (499996 us)\n"
    );
}

/// A change to one state is not read as a change to the other, and both reach
/// the caller when both moved.
#[test]
fn a_tempo_and_a_bend_that_both_changed_are_two_rows() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = stating(dir.path(), "before", &[common::pitch_bend(960, 0)]);
    let after = stating(
        dir.path(),
        "after",
        &[common::pitch_bend(960, 4096), common::tempo(1920, 400_000)],
    );

    let diff = diff_json(&before, &after, None);
    notes_agree(&diff);
    assert_eq!(list(&diff, "tempos").len(), 1);
    assert_eq!(list(&diff, "bends").len(), 1);

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path")
        ]),
        "\
tempo  bar 2 beat 1 onwards  120 -> 150
bend   bar 1 beat 3 onwards  channel 0  0 -> 4096
"
    );
}
