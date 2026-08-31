mod common;

use common::{mid, FIXTURE};
use std::collections::HashSet;

#[test]
fn lists_every_note_with_an_identity() {
    let json = common::inspect_json(std::path::Path::new(FIXTURE));
    let notes = common::notes(&json);
    assert_eq!(notes.len(), 36);

    let ids: HashSet<String> = common::note_ids(&json).into_iter().collect();
    assert_eq!(ids.len(), 36, "every identity is distinct");

    for note in &notes {
        for field in [
            "id", "track", "channel", "pitch", "start", "duration", "velocity",
        ] {
            assert!(note.get(field).is_some(), "note is missing {field}: {note}");
        }
    }
}

/// The fixture's durations are one tick short of a beat, two beats and a bar.
/// A model that tidied them up to 480 / 960 / 1440 would still sound plausible,
/// which is why this is asserted rather than eyeballed.
#[test]
fn reports_durations_as_they_are_written() {
    let json = common::inspect_json(std::path::Path::new(FIXTURE));
    let durations: HashSet<u64> = common::notes(&json)
        .iter()
        .map(|note| note["duration"].as_u64().expect("duration is a number"))
        .collect();
    assert_eq!(durations, HashSet::from([475, 955, 1435]));
}

#[test]
fn identities_are_stable_across_repeated_reads() {
    let first = common::note_ids(&common::inspect_json(std::path::Path::new(FIXTURE)));
    let second = common::note_ids(&common::inspect_json(std::path::Path::new(FIXTURE)));
    assert_eq!(first, second);
}

#[test]
fn fails_on_a_take_that_is_not_there() {
    mid()
        .args(["inspect", "fixtures/no-such-take.mid", "--json"])
        .assert()
        .failure();
}

/// `mid inspect --bars <range> --json`, as an Output so a test can assert on
/// either the notes it lists or the refusal it prints.
fn inspect_bars(path: &std::path::Path, range: &str) -> std::process::Output {
    mid()
        .args(["inspect", "--bars", range, "--json"])
        .arg(path)
        .output()
        .expect("mid runs")
}

/// The Ticks the notes of a Bar range start on, which is the whole of what
/// `--bars` has to get right.
fn starts_in_bars(path: &std::path::Path, range: &str) -> Vec<u64> {
    let output = inspect_bars(path, range);
    assert!(
        output.status.success(),
        "inspect --bars {range} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    common::notes(&json)
        .iter()
        .map(|note| note["start"].as_u64().expect("start is a number"))
        .collect()
}

/// The passage a musician would point at. 18 of the fixture's 36 notes start in
/// Bars 5–8, which in 3/4 at 480 PPQ is Ticks 5760–11519. A 4/4 assumption
/// anywhere in the boundary arithmetic gives a different span and fails here.
#[test]
fn a_bar_range_returns_the_notes_starting_in_it() {
    let starts = starts_in_bars(std::path::Path::new(FIXTURE), "5:8");
    assert_eq!(starts.len(), 18);
    for start in starts {
        assert!(
            (5760..=11519).contains(&start),
            "note starting at {start} is not in Bars 5-8"
        );
    }
}

/// A single Bar is a range whose ends are equal, and Bar 1 begins at Tick 0 —
/// the two places an off-by-one hides.
#[test]
fn a_single_bar_range_returns_that_bar_alone() {
    let fixture = std::path::Path::new(FIXTURE);

    let fifth = starts_in_bars(fixture, "5:5");
    assert_eq!(fifth.len(), 5);
    assert!(fifth.iter().all(|start| (5760..=7199).contains(start)));

    let first = starts_in_bars(fixture, "1:1");
    assert_eq!(first.len(), 4);
    assert!(first.iter().all(|start| *start < 1440));
    assert!(first.contains(&0), "Bar 1 starts at Tick 0");
}

/// The fixture stops four Ticks inside Bar 8: `length_ticks` is 11516 where
/// eight Bars of 1440 would end at 11520. A partial final Bar is still a Bar, so
/// Bar 8 is selectable and Bar 9 does not exist.
#[test]
fn the_final_bar_counts_even_when_the_take_stops_inside_it() {
    let fixture = std::path::Path::new(FIXTURE);

    let eighth = starts_in_bars(fixture, "8:8");
    assert_eq!(eighth.len(), 4);
    assert!(eighth.iter().all(|start| (10080..=11519).contains(start)));

    let past_the_end = inspect_bars(fixture, "9:9");
    assert!(!past_the_end.status.success());
    let complaint = String::from_utf8_lossy(&past_the_end.stderr).to_string();
    assert!(
        complaint.contains("8 Bars") && complaint.contains("Bar 9"),
        "the refusal says neither how long the Take is nor which Bar was asked for: {complaint}"
    );
}

/// A Take whose last event lands exactly on a Bar line fills whole Bars and
/// gains no empty one after them. This is the ordinary case for a Take a DAW
/// wrote, and it is where counting "the Bar containing the last Tick" goes
/// wrong: it would report three Bars here and let `--bars 3:3` succeed.
#[test]
fn a_take_that_ends_on_a_bar_line_gains_no_empty_bar() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("two-whole-bars.mid"),
        480,
        &[(0, 3, 4)],
        // Two Bars of 3/4, the second note running right up to the Bar line.
        &[(0, 1440, 60), (1440, 1440, 62)],
    );

    assert_eq!(starts_in_bars(&take, "2:2"), vec![1440]);
    assert!(!inspect_bars(&take, "3:3").status.success());
}

/// Bar lines follow the time signature the Take states. 6/8 is what makes this
/// bite: a
/// Bar length computed as numerator × PPQ, ignoring the denominator, is right
/// for both 3/4 and 4/4 and wrong here.
#[test]
fn bar_lines_follow_the_stated_time_signature() {
    let dir = tempfile::tempdir().expect("temp dir");

    // 4/4 at 480 PPQ: a Bar is 1920 Ticks, so Bar 2 is 1920–3839.
    let four_four = common::build_take(
        &dir.path().join("four-four.mid"),
        480,
        &[(0, 4, 4)],
        &[
            (0, 240, 60),
            (1440, 240, 62),
            (1920, 240, 64),
            (3360, 240, 65),
        ],
    );
    assert_eq!(starts_in_bars(&four_four, "2:2"), vec![1920, 3360]);

    // 6/8 at 480 PPQ: six eighths is 1440 Ticks, the same Bar length as 3/4.
    let six_eight = common::build_take(
        &dir.path().join("six-eight.mid"),
        480,
        &[(0, 6, 8)],
        &[(0, 240, 60), (1440, 240, 62)],
    );
    assert_eq!(starts_in_bars(&six_eight, "2:2"), vec![1440]);
}

/// A note belongs to the Bar it *starts* in, even when it sustains past the Bar
/// line. Not one of the fixture's 36 notes crosses a Bar line, so this rule is
/// unexercised by it and needs a Take built to state it.
#[test]
fn a_note_belongs_to_the_bar_it_starts_in() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("across-the-bar-line.mid"),
        480,
        &[(0, 3, 4)],
        // The first note starts in Bar 1 and is still sounding in Bar 2.
        &[(1200, 600, 60), (1440, 240, 62)],
    );

    assert_eq!(starts_in_bars(&take, "1:1"), vec![1200]);
    assert_eq!(starts_in_bars(&take, "2:2"), vec![1440]);
}

/// Every way of asking for Bars that do not exist, each with its own remedy and
/// each a non-zero exit code.
#[test]
fn a_bar_range_that_cannot_be_honoured_is_refused() {
    let fixture = std::path::Path::new(FIXTURE);
    for (range, expected) in [
        ("9:9", "8 Bars long"),
        ("1:1000", "8 Bars long"),
        ("0:4", "1-indexed"),
        ("8:5", "runs backwards"),
        ("5", "FIRST:LAST"),
        ("nonsense", "FIRST:LAST"),
    ] {
        let output = inspect_bars(fixture, range);
        assert!(!output.status.success(), "--bars {range} was accepted");
        let complaint = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(
            complaint.contains(expected),
            "--bars {range} was refused without saying {expected:?}: {complaint}"
        );
    }
}

/// A Take that states no time signature has no Bars. The MIDI spec says to
/// assume 4/4; battuta refuses instead, because a Bar number derived from a
/// time signature the Take never stated is a wrong answer with nothing to
/// reveal it.
/// See ADR-0006.
#[test]
fn a_take_that_states_no_time_signature_refuses_to_talk_about_bars() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("no-time-signature.mid"),
        480,
        &[],
        &[(0, 240, 60), (1440, 240, 62)],
    );

    // Everything that does not need Bar lines still works on it.
    assert_eq!(common::notes(&common::inspect_json(&take)).len(), 2);

    let output = inspect_bars(&take, "1:1");
    assert!(!output.status.success());
    let complaint = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        complaint.contains("no time signature") && complaint.contains("4/4"),
        "the refusal does not say what is missing or that 4/4 is not assumed: {complaint}"
    );
}

/// A Take that changes time signature part way through is refused too, and the
/// refusal names the Tick where it changes so the human can go and look at it.
#[test]
fn a_take_that_changes_time_signature_refuses_to_talk_about_bars() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("changes-time-signature.mid"),
        480,
        &[(0, 3, 4), (2880, 4, 4)],
        &[(0, 240, 60), (2880, 240, 62)],
    );

    let output = inspect_bars(&take, "1:1");
    assert!(!output.status.success());
    let complaint = String::from_utf8_lossy(&output.stderr).to_string();
    for expected in ["3/4", "4/4", "2880"] {
        assert!(
            complaint.contains(expected),
            "the refusal never mentions {expected}: {complaint}"
        );
    }
}

/// Restating the same time signature is what some exports do at every Bar. It
/// changes nothing, and must not be mistaken for a change.
#[test]
fn a_restated_time_signature_is_not_a_change() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("restated-time-signature.mid"),
        480,
        &[(0, 3, 4), (1440, 3, 4), (2880, 3, 4)],
        &[(0, 240, 60), (1440, 240, 62), (2880, 240, 64)],
    );

    assert_eq!(starts_in_bars(&take, "2:2"), vec![1440]);
}

/// A time signature stated at Tick 5000 says nothing about Ticks 0–4999. Using
/// it for those Bars would apply a time signature backwards to before the Take
/// stated it
/// — the same wrong answer as assuming 4/4, one step over — so it is refused.
#[test]
fn a_time_signature_that_starts_late_governs_nothing_before_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("late-time-signature.mid"),
        480,
        &[(5000, 3, 4)],
        &[(0, 240, 60), (5000, 240, 62)],
    );

    let output = inspect_bars(&take, "1:1");
    assert!(!output.status.success(), "Bar 1 was gridded from nothing");
    let complaint = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        complaint.contains("5000"),
        "the refusal does not say where the time signature starts: {complaint}"
    );
}

/// Time signatures whose Bar length cannot be worked out at all. Neither is a
/// Bar this tool will round or guess at, and each says so in its own terms.
#[test]
fn a_time_signature_that_yields_no_bar_length_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");

    // 0/4 counts no beats, so it describes no Bar.
    let no_beats = common::build_take(
        &dir.path().join("no-beats.mid"),
        480,
        &[(0, 0, 4)],
        &[(0, 240, 60)],
    );
    let output = inspect_bars(&no_beats, "1:1");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("0/4"));

    // 1/32 at 1 Tick per quarter is an eighth of a Tick: not a Bar length.
    let fractional = common::build_take(
        &dir.path().join("fractional-bar.mid"),
        1,
        &[(0, 1, 32)],
        &[(0, 1, 60)],
    );
    let output = inspect_bars(&fractional, "1:1");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("whole number of ticks"),
        "a fractional Bar was not refused as one"
    );
}

/// The passage as a musician would point at it: Bar and Beat first, then the
/// note by name, and the identity last because that is what an Edit Set copies.
/// In the order the music happens rather than track by track, so the Bar numbers
/// only go forwards and the two notes of a chord sit next to each other.
///
/// Asserted whole. The columns and the order are the point — a `contains` on one
/// line could tell neither whether the lines line up nor which line came first.
#[test]
fn lists_a_passage_by_bar_beat_and_note_name() {
    assert_eq!(
        common::human_output(&["inspect", FIXTURE, "--bars", "5:6"]),
        "\
bar 5 beat 1  track 1  F#4  velocity 50  duration 955   t1:c0:p66:s5760:n0
bar 5 beat 1  track 2  A2   velocity 45  duration 475   t2:c1:p45:s5760:n0
bar 5 beat 2  track 2  A3   velocity 38  duration 955   t2:c1:p57:s6240:n0
bar 5 beat 2  track 2  C#4  velocity 38  duration 955   t2:c1:p61:s6240:n0
bar 5 beat 3  track 1  E4   velocity 50  duration 475   t1:c0:p64:s6720:n0
bar 6 beat 1  track 1  D4   velocity 50  duration 1435  t1:c0:p62:s7200:n0
bar 6 beat 1  track 2  A2   velocity 45  duration 475   t2:c1:p45:s7200:n0
bar 6 beat 2  track 2  A3   velocity 38  duration 955   t2:c1:p57:s7680:n0
bar 6 beat 2  track 2  C#4  velocity 38  duration 955   t2:c1:p61:s7680:n0
"
    );
}

/// A Take with no Bars to be positioned in is still listed, in the Ticks that
/// are the truth underneath them. `info` reports the missing Bar count and
/// `inspect --bars` refuses the range; listing every note refuses nothing,
/// because the Take is exactly the one you most need to look at.
#[test]
fn falls_back_to_ticks_when_the_bars_cannot_be_derived() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("no-time-signature.mid"),
        480,
        &[],
        &[(0, 240, 60), (960, 240, 62)],
    );
    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path")]),
        "\
tick 0    track 1  C4  velocity 64  duration 240  t1:c0:p60:s0:n0
tick 960  track 1  D4  velocity 64  duration 240  t1:c0:p62:s960:n0
"
    );
}

/// A note that does not land on a Beat is placed by the Beat it follows and the
/// Ticks it is past it — never rounded to the nearest one, which would put two
/// different notes in the same place and say the Take had said so.
#[test]
fn places_a_note_that_does_not_land_on_a_beat() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("upbeat.mid"),
        480,
        &[(0, 3, 4)],
        &[(240, 240, 66)],
    );
    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path")]),
        "bar 1 beat 1+240  track 1  F#4  velocity 64  duration 240  t1:c0:p66:s240:n0\n"
    );
}

/// The two conventions a MIDI file does not state, at the ends where they show:
/// pitch 60 is middle C and middle C is called C4, so pitch 0 is C-1 and pitch
/// 127 is G9. See ADR-0011.
#[test]
fn names_middle_c_and_both_ends_of_the_pitch_range() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("range.mid"),
        480,
        &[(0, 4, 4)],
        &[(0, 240, 0), (480, 240, 60), (960, 240, 127)],
    );
    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path")]),
        "\
bar 1 beat 1  track 1  C-1  velocity 64  duration 240  t1:c0:p0:s0:n0
bar 1 beat 2  track 1  C4   velocity 64  duration 240  t1:c0:p60:s480:n0
bar 1 beat 3  track 1  G9   velocity 64  duration 240  t1:c0:p127:s960:n0
"
    );
}

/// A Take bigger than the fixture keeps its columns: every column is as wide as
/// its widest value, so a Bar in three figures or a duration in four does not
/// shunt the rest of its line out of line with the others.
#[test]
fn widens_a_column_to_its_widest_value() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("wide.mid"),
        480,
        &[(0, 4, 4)],
        &[(0, 240, 60), (1920, 1920, 61), (46080, 96, 127)],
    );
    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path")]),
        "\
bar 1 beat 1   track 1  C4   velocity 64  duration 240   t1:c0:p60:s0:n0
bar 2 beat 1   track 1  C#4  velocity 64  duration 1920  t1:c0:p61:s1920:n0
bar 25 beat 1  track 1  G9   velocity 64  duration 96    t1:c0:p127:s46080:n0
"
    );
}

/// A passage with nothing in it says so. Silence would read as a command that
/// had not run.
#[test]
fn says_when_a_passage_holds_no_notes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("gap.mid"),
        480,
        &[(0, 4, 4)],
        &[(0, 240, 60), (3840, 240, 62)],
    );
    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path"), "--bars", "2:2"]),
        "no notes\n"
    );
}
