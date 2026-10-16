//! `mid diff` across two time signatures places every position in one frame,
//! the before Take's Bar lines, and says so — #64, under ADR-0009.
//!
//! Each Take used to be placed against its own Bar lines, so a note moved 480
//! Ticks between a 3/4 and a 4/4 Take read as moved to where it already was.
//! What is tested here is that every position printed, on both sides, is a
//! reading of its Tick in the before Take's Bar lines, that the output says so
//! where the two Takes' Bar lines differ, and that it says nothing new where
//! they agree.
//!
//! The Takes are `fixtures/expressive.mid` (480 PPQ, 3/4, so a Bar is 1440
//! Ticks and a Beat 480) and that fixture rewritten here. The expected
//! positions are literals in that 3/4: Tick 1920 is Bar 2 Beat 2, never Bar 2
//! Beat 1, which is where 1920 falls in 4/4.

mod common;

use common::{mid, EXPRESSIVE};
use midly::num::{u28, u7};
use midly::{MetaMessage, MidiMessage, TrackEventKind};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// A Take's events with absolute Ticks, one list per track.
type Tracks<'a> = Vec<Vec<(u32, TrackEventKind<'a>)>>;

/// `fixtures/expressive.mid`, changed by `change`, written to `dir/name.mid`.
/// Each track is put back in Tick order afterwards, keeping the written order
/// of events that share a Tick, so a change may move an event anywhere.
fn expressive_with(dir: &Path, name: &str, change: impl FnOnce(&mut Tracks)) -> PathBuf {
    let bytes = std::fs::read(EXPRESSIVE).expect("the fixture is readable");
    let smf = midly::Smf::parse(&bytes).expect("the fixture parses");
    let mut tracks: Tracks = smf
        .tracks
        .iter()
        .map(|track| {
            let mut tick = 0;
            track
                .iter()
                .map(|event| {
                    tick += event.delta.as_int();
                    (tick, event.kind)
                })
                .collect()
        })
        .collect();
    change(&mut tracks);
    let tracks = tracks
        .into_iter()
        .map(|mut events| {
            events.sort_by_key(|&(tick, kind)| {
                (
                    tick,
                    matches!(kind, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
                )
            });
            let mut previous = 0;
            events
                .into_iter()
                .map(|(tick, kind)| {
                    let delta = u28::new(tick - previous);
                    previous = tick;
                    midly::TrackEvent { delta, kind }
                })
                .collect()
        })
        .collect();
    let path = dir.join(format!("{name}.mid"));
    midly::Smf {
        header: smf.header,
        tracks,
    }
    .save(&path)
    .expect("the Take is written");
    path
}

/// The fixture's one time signature, `FF 58 04 03 02 18 08` on track 0 at Tick
/// 0, rewritten as `FF 58 04 04 02 18 08`: 4/4, so a Bar is 1920 Ticks.
fn in_four_four(tracks: &mut Tracks) {
    let stated = tracks[0]
        .iter_mut()
        .find(|(_, kind)| matches!(kind, TrackEventKind::Meta(MetaMessage::TimeSignature(..))))
        .expect("the fixture states a time signature");
    assert_eq!(
        *stated,
        (
            0,
            TrackEventKind::Meta(MetaMessage::TimeSignature(3, 2, 24, 8))
        ),
        "the fixture is in 3/4 from Tick 0"
    );
    stated.1 = TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8));
}

/// B4 on track 1 (`t1:c0:p71:s1440:n0`, Ticks 1440 to 2880) moved 480 Ticks
/// later, strike and release both — what #64's Edit Set does.
fn b4_moved_later(tracks: &mut Tracks) {
    let mut moved = 0;
    for (tick, kind) in tracks[1].iter_mut() {
        let key = match kind {
            TrackEventKind::Midi {
                message: MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. },
                ..
            } => *key,
            _ => continue,
        };
        if key == u7::new(71) && (*tick == 1440 || *tick == 2880) {
            *tick += 480;
            moved += 1;
        }
    }
    assert_eq!(moved, 2, "B4's strike and release are on track 1");
}

fn diff_stdout(before: &Path, after: &Path, extra: &[&str]) -> String {
    let output = mid()
        .arg("diff")
        .arg(before)
        .arg(after)
        .args(extra)
        .output()
        .expect("mid runs");
    assert!(
        output.status.success(),
        "diff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("diff prints UTF-8")
}

fn line_with<'a>(out: &'a str, needle: &str) -> &'a str {
    out.lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no line contains {needle:?} in:\n{out}"))
}

/// #64's reproduction, matched as a move. B4 left Tick 1440, Bar 2 Beat 1 in
/// the before Take's 3/4, for Tick 1920, Bar 2 Beat 2 in it.
#[test]
fn a_note_moved_across_time_signatures_reads_as_moved() {
    let dir = tempfile::tempdir().expect("temp dir");
    let after = expressive_with(dir.path(), "four-four-moved", |tracks| {
        in_four_four(tracks);
        b4_moved_later(tracks);
    });
    let out = diff_stdout(Path::new(EXPRESSIVE), &after, &["--tolerance", "480"]);
    let row = line_with(&out, "changed");
    assert!(
        row.contains("bar 2 beat 1") && row.ends_with("moved to bar 2 beat 2"),
        "the move is placed in the before Take's Bar lines on both sides:\n{out}"
    );
}

/// The same Takes at the default tolerance, which is too tight to pair them:
/// the note that arrived and the note that left are at two different positions.
#[test]
fn an_added_and_a_removed_note_across_time_signatures_are_two_positions() {
    let dir = tempfile::tempdir().expect("temp dir");
    let after = expressive_with(dir.path(), "four-four-moved", |tracks| {
        in_four_four(tracks);
        b4_moved_later(tracks);
    });
    let out = diff_stdout(Path::new(EXPRESSIVE), &after, &[]);
    assert!(
        line_with(&out, "added").contains("bar 2 beat 2"),
        "the arrival is Tick 1920, Bar 2 Beat 2 in 3/4:\n{out}"
    );
    assert!(
        line_with(&out, "removed").contains("bar 2 beat 1"),
        "the departure is Tick 1440, Bar 2 Beat 1 in 3/4:\n{out}"
    );
}

/// A Controller difference across the two time signatures: the after Take is
/// in 4/4 and holds CC11 on channel 0 at 20 from Tick 2400 and 10 from Tick
/// 2520, where the before Take holds 84 and 90. Tick 2520 is Bar 2 Beat 3+120
/// in 3/4 and Bar 2 Beat 2+120 in 4/4. The span's heading and both of its sides
/// are one frame.
#[test]
fn a_span_row_reads_both_sides_in_one_frame() {
    let dir = tempfile::tempdir().expect("temp dir");
    let after = expressive_with(dir.path(), "four-four-softer", |tracks| {
        in_four_four(tracks);
        let mut changed = 0;
        for (tick, kind) in tracks[1].iter_mut() {
            if let TrackEventKind::Midi {
                message: MidiMessage::Controller { controller, value },
                ..
            } = kind
            {
                if *controller == u7::new(11) && (*tick == 2400 || *tick == 2520) {
                    *value = u7::new(if *tick == 2400 { 20 } else { 10 });
                    changed += 1;
                }
            }
        }
        assert_eq!(changed, 2, "CC11 is stated at Ticks 2400 and 2520");
    });
    let out = diff_stdout(Path::new(EXPRESSIVE), &after, &[]);
    assert!(
        line_with(&out, "controller").contains("bar 2 beat 3 until bar 2 beat 3+240"),
        "the span is Ticks 2400 to 2640 in 3/4:\n{out}"
    );
    assert!(
        line_with(&out, "  before").contains("up to 90 at bar 2 beat 3+120"),
        "{out}"
    );
    assert!(
        line_with(&out, "  after").contains("down to 10 at bar 2 beat 3+120"),
        "the after side's extreme is placed in the before Take's Bar lines:\n{out}"
    );
}

/// The frame is stated once, before the rows, where the two Takes' Bar lines
/// differ.
#[test]
fn the_frame_is_stated_before_the_rows_where_time_signatures_differ() {
    let dir = tempfile::tempdir().expect("temp dir");
    let after = expressive_with(dir.path(), "four-four-moved", |tracks| {
        in_four_four(tracks);
        b4_moved_later(tracks);
    });
    let out = diff_stdout(Path::new(EXPRESSIVE), &after, &["--tolerance", "480"]);
    assert_eq!(
        out,
        "\
time signatures differ; every position is in the before Take's Bar lines
changed  bar 2 beat 1  track 1  channel 0  B4  moved to bar 2 beat 2
not compared: time signature (before 1, after 1)
"
    );
}

/// Where the two Takes share a time signature nothing new is said: the same
/// move inside 3/4 prints exactly the one row.
#[test]
fn no_frame_is_stated_where_time_signatures_agree() {
    let dir = tempfile::tempdir().expect("temp dir");
    let after = expressive_with(dir.path(), "moved", b4_moved_later);
    let out = diff_stdout(Path::new(EXPRESSIVE), &after, &["--tolerance", "480"]);
    assert_eq!(
        out,
        "changed  bar 2 beat 1  track 1  channel 0  B4  moved to bar 2 beat 2\n"
    );
}

fn diff_json(before: &Path, after: &Path) -> Value {
    let out = diff_stdout(before, after, &["--tolerance", "480", "--json"]);
    serde_json::from_str(&out).expect("diff --json is JSON")
}

/// `--json` names the frame and says whether the Bar lines differ. Its Ticks
/// are the Ticks the files hold, as before.
#[test]
fn json_names_the_frame_and_keeps_its_ticks() {
    let dir = tempfile::tempdir().expect("temp dir");
    let across = expressive_with(dir.path(), "four-four-moved", |tracks| {
        in_four_four(tracks);
        b4_moved_later(tracks);
    });
    let within = expressive_with(dir.path(), "moved", b4_moved_later);
    let before = Path::new(EXPRESSIVE);

    let diff = diff_json(before, &across);
    assert_eq!(
        diff["positions"],
        json!({ "bar_lines": "before", "bar_lines_differ": true })
    );
    assert_eq!(diff["changed"][0]["before"]["start"], json!(1440));
    assert_eq!(diff["changed"][0]["after"]["start"], json!(1920));

    assert_eq!(
        diff_json(before, &within)["positions"],
        json!({ "bar_lines": "before", "bar_lines_differ": false })
    );
}

/// A before Take that states no time signature has no Bar lines, so the frame
/// is Ticks: the after Take's 3/4 does not place the moved note, and the line
/// says why every position is a Tick.
#[test]
fn a_before_take_with_no_bar_lines_puts_every_position_in_ticks() {
    let dir = tempfile::tempdir().expect("temp dir");
    let unstated = expressive_with(dir.path(), "unstated", |tracks| {
        tracks[0].retain(|(_, kind)| {
            !matches!(kind, TrackEventKind::Meta(MetaMessage::TimeSignature(..)))
        });
    });
    let after = expressive_with(dir.path(), "moved", b4_moved_later);
    let out = diff_stdout(&unstated, &after, &["--tolerance", "480"]);
    assert_eq!(
        out,
        "\
time signatures differ; every position is a Tick, as the before Take has no Bar lines
changed  tick 1440  track 1  channel 0  B4  moved to tick 1920
not compared: time signature (before 0, after 1)
"
    );
}
