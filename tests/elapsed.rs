//! Elapsed time beside the Tick, where a Take has no Bars — #73.
//!
//! Every expected reading here is worked by hand from the tempos the Take is
//! built with, never asked of `battuta`.

mod common;

use common::{mid, tempo};
use midly::num::{u15, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use std::path::{Path, PathBuf};

const PPQ: u16 = 480;

fn note(start: u32, key: u8) -> Vec<(u32, TrackEventKind<'static>)> {
    let message = |message| TrackEventKind::Midi {
        channel: u4::new(0),
        message,
    };
    vec![
        (
            start,
            message(MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(80),
            }),
        ),
        (
            start + 120,
            message(MidiMessage::NoteOff {
                key: u7::new(key),
                vel: u7::new(0),
            }),
        ),
    ]
}

/// A Take of exactly these tracks, stating no time signature, so that it has
/// no Bars and every position is a Tick.
fn take(path: &Path, tracks: Vec<Vec<(u32, TrackEventKind<'static>)>>) -> PathBuf {
    take_at(path, PPQ, tracks)
}

fn take_at(path: &Path, ppq: u16, tracks: Vec<Vec<(u32, TrackEventKind<'static>)>>) -> PathBuf {
    let tracks = tracks
        .into_iter()
        .map(|mut events| {
            events.sort_by_key(|(tick, _)| *tick);
            let mut previous = 0;
            let mut written: Vec<TrackEvent> = events
                .into_iter()
                .map(|(tick, kind)| {
                    let event = TrackEvent {
                        delta: u28::new(tick - previous),
                        kind,
                    };
                    previous = tick;
                    event
                })
                .collect();
            written.push(TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            });
            written
        })
        .collect();
    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(ppq))),
        tracks,
    };
    smf.save(path).expect("built Take is writable");
    path.to_path_buf()
}

fn said(args: &[&str]) -> String {
    let output = mid().args(args).output().expect("mid runs");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).expect("stdout is UTF-8")
}

fn inspect(path: &Path) -> String {
    said(&["inspect", path.to_str().expect("a UTF-8 path")])
}

fn info(path: &Path) -> String {
    said(&["info", path.to_str().expect("a UTF-8 path")])
}

/// 120 bpm for two beats, then 60. Tick 960 is one second in; Tick 1200 is
/// half a beat at 60 further, so 1.5 s. Read at the opening tempo throughout it
/// would be 1.25 s.
#[test]
fn a_tempo_change_is_counted_from_where_it_is_stated() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut voice = note(0, 60);
    voice.extend(note(1200, 62));
    let path = take(
        &dir.path().join("slowing.mid"),
        vec![vec![tempo(0, 500_000), tempo(960, 1_000_000)], voice],
    );

    let listing = inspect(&path);
    assert!(listing.contains("tick 0 (0:00.00)"), "{listing}");
    assert!(listing.contains("tick 1200 (0:01.50)"), "{listing}");
}

/// Tick 640 at 120 bpm is 0.6666… s: a counter shows 0:00.66 there, and has not
/// reached 0:00.67.
#[test]
fn hundredths_are_truncated_never_rounded_up() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = take(
        &dir.path().join("third.mid"),
        vec![vec![tempo(0, 500_000)], note(640, 60)],
    );

    let listing = inspect(&path);
    assert!(listing.contains("tick 640 (0:00.66)"), "{listing}");
}

/// 4503.2 s at 60 bpm and 480 PPQ is Tick 2161536: seventy-five minutes, not an
/// hour and fifteen.
#[test]
fn minutes_run_past_sixty() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = take(
        &dir.path().join("long.mid"),
        vec![vec![tempo(0, 1_000_000)], note(2_161_536, 60)],
    );

    let listing = inspect(&path);
    assert!(listing.contains("tick 2161536 (75:03.20)"), "{listing}");
}

/// The last event is the note-off at 1320, which at 120 bpm is 1.375 s.
#[test]
fn info_gives_the_length_as_an_elapsed_time() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = take(
        &dir.path().join("short.mid"),
        vec![vec![tempo(0, 500_000)], note(1200, 60)],
    );

    let described = info(&path);
    assert!(
        described.contains("1320 ticks (0:01.37; bars need one stated time signature)"),
        "{described}"
    );
}

/// SMF would have a reader assume 120 bpm. The file does not say so, and #3
/// declined the same assumption for 4/4.
#[test]
fn a_take_stating_no_tempo_has_no_elapsed_time() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = take(&dir.path().join("untimed.mid"), vec![vec![], note(960, 60)]);

    let listing = inspect(&path);
    assert!(listing.contains("tick 960 "), "{listing}");
    assert!(!listing.contains("tick 960 ("), "{listing}");
    let described = info(&path);
    assert!(
        described.contains("1080 ticks (no elapsed time: no tempo stated;"),
        "{described}"
    );
}

/// Nothing says how fast Ticks 0–479 pass, and every later reading counts
/// through them.
#[test]
fn a_tempo_stated_late_gives_no_tick_an_elapsed_time() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = take(
        &dir.path().join("late.mid"),
        vec![vec![tempo(480, 500_000)], note(960, 60)],
    );

    let listing = inspect(&path);
    assert!(!listing.contains("tick 960 ("), "{listing}");
    let described = info(&path);
    assert!(
        described.contains("no elapsed time: no tempo stated until tick 480"),
        "{described}"
    );
}

/// Two tracks state different tempos at Tick 960. Before it the reading is
/// whole; from it there is none, because every later Tick counts through it.
#[test]
fn contradicting_tempos_end_the_elapsed_time_where_they_are_stated() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut voice = note(480, 60);
    voice.extend(note(1440, 62));
    voice.push(tempo(960, 250_000));
    let path = take(
        &dir.path().join("contradicting.mid"),
        vec![vec![tempo(0, 500_000), tempo(960, 1_000_000)], voice],
    );

    let listing = inspect(&path);
    assert!(listing.contains("tick 480 (0:00.50)"), "{listing}");
    assert!(listing.contains("tick 1440 "), "{listing}");
    assert!(!listing.contains("tick 1440 ("), "{listing}");
    let described = info(&path);
    assert!(
        described.contains("elapsed time stops at tick 960: the tempo there is indeterminate"),
        "{described}"
    );
}

/// A Take with Bars is placed by them, as it was before #73.
#[test]
fn a_take_with_bars_is_placed_by_bar_and_beat_alone() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = common::build_take(
        &dir.path().join("barred.mid"),
        PPQ,
        &[(0, 4, 4)],
        &[(480, 120, 60)],
    );

    let listing = inspect(&path);
    assert!(listing.contains("bar 1 beat 2 "), "{listing}");
    assert!(!listing.contains("(0:"), "{listing}");
}

fn diff(before: &Path, after: &Path) -> String {
    let output = mid()
        .arg("diff")
        .arg(before)
        .arg(after)
        .output()
        .expect("mid runs");
    String::from_utf8(output.stdout).expect("stdout is UTF-8")
}

/// The after Take is at 60 where the before is at 120, so its added note at
/// Tick 960 is printed at the before Take's 0:01.00 and heard at 0:02.00 in its
/// own render. The diff says, once, whose Elapsed times these are.
#[test]
fn a_diff_across_tempos_says_whose_elapsed_times_it_prints() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = take(
        &dir.path().join("before.mid"),
        vec![vec![tempo(0, 500_000)], note(480, 60)],
    );
    let mut voice = note(480, 60);
    voice.extend(note(960, 62));
    let after = take(
        &dir.path().join("after.mid"),
        vec![vec![tempo(0, 1_000_000)], voice],
    );

    let out = diff(&before, &after);
    assert_eq!(
        out.lines().next(),
        Some("tempos differ; every elapsed time is the before Take's"),
        "{out}"
    );
    assert!(out.contains("tick 960 (0:01.00)"), "{out}");
}

/// A tempo restated at the value already in force changes no Elapsed time, so
/// there is nothing to say about whose they are.
#[test]
fn a_restated_tempo_is_not_a_tempo_that_differs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = take(
        &dir.path().join("before.mid"),
        vec![vec![tempo(0, 500_000)], note(480, 60)],
    );
    let mut voice = note(480, 60);
    voice.extend(note(960, 62));
    let after = take(
        &dir.path().join("after.mid"),
        vec![vec![tempo(0, 500_000), tempo(720, 500_000)], voice],
    );

    let out = diff(&before, &after);
    assert!(!out.contains("tempos differ"), "{out}");
    assert!(out.contains("tick 960 (0:01.00)"), "{out}");
}

/// 2,000,000,000 Ticks at one to the quarter note and the slowest tempo MIDI
/// can state, 16,777,215 us per quarter, is 33,554,430,000 s: longer than a
/// count of nanoseconds fits in 64 bits, and still a reading the file states.
/// The tempo is restated along the way only because no delta reaches that far.
#[test]
fn a_reading_too_long_for_nanoseconds_is_still_given() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut conductor: Vec<_> = (0..10u32)
        .map(|step| tempo(step * 200_000_000, 16_777_215))
        .collect();
    conductor.extend(note(2_000_000_000, 60));
    let path = take_at(&dir.path().join("aeons.mid"), 1, vec![conductor]);

    let listing = inspect(&path);
    assert!(
        listing.contains("tick 2000000000 (559240500:00.00)"),
        "{listing}"
    );
}

/// The frame line's fact is in `--json` as well, beside `bar_lines_differ`.
#[test]
fn diff_json_says_whether_elapsed_times_differ() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = take(
        &dir.path().join("before.mid"),
        vec![vec![tempo(0, 500_000)], note(480, 60)],
    );
    let slower = take(
        &dir.path().join("slower.mid"),
        vec![vec![tempo(0, 1_000_000)], note(480, 60)],
    );
    let restated = take(
        &dir.path().join("restated.mid"),
        vec![vec![tempo(0, 500_000), tempo(720, 500_000)], note(480, 60)],
    );

    let json = |after: &Path| -> serde_json::Value {
        let output = mid()
            .arg("diff")
            .arg(&before)
            .arg(after)
            .arg("--json")
            .output()
            .expect("mid runs");
        serde_json::from_slice(&output.stdout).expect("diff --json is JSON")
    };
    assert_eq!(json(&slower)["elapsed_times_differ"], true);
    assert_eq!(json(&restated)["elapsed_times_differ"], false);
}
