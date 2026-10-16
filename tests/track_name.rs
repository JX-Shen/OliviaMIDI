//! The name a track states, beside the number `inspect` and `diff` print for it
//! — #61: every name, quoted as the file states it, in written order, and on a
//! track's row only, never on a channel's.
//!
//! Every expected name here is read from the Take with `midly` or written into
//! a Take built for the test, never asked of `battuta`.

mod common;

use midly::num::{u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use std::path::{Path, PathBuf};

const OVERTURE: &str = "fixtures/external/cc0-midis-overture.mid";

/// What each track of a Take names itself, read straight from the file: the
/// text of every track name meta event in it, in written order.
fn names_in(path: &Path) -> Vec<Vec<String>> {
    let bytes = std::fs::read(path).expect("read Take");
    let smf = Smf::parse(&bytes).expect("parse Take");
    smf.tracks
        .iter()
        .map(|track| {
            track
                .iter()
                .filter_map(|event| match event.kind {
                    TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                        Some(String::from_utf8(name.to_vec()).expect("a UTF-8 name"))
                    }
                    _ => None,
                })
                .collect()
        })
        .collect()
}

/// Every `track N` a line prints, with whatever follows the number.
fn tracks_on(line: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find("track ") {
        let after = &rest[at + "track ".len()..];
        let digits = after.chars().take_while(char::is_ascii_digit).count();
        if digits > 0 {
            found.push((after[..digits].parse().unwrap(), &after[digits..]));
        }
        rest = &after[digits..];
    }
    found
}

/// A Take built for the test: a conductor track in 4/4 at 480 PPQ, then each
/// voice track with the name it states, if any, and its events in the order
/// given.
fn build(path: &Path, voices: &[(Option<&'static [u8]>, Vec<Event>)]) -> PathBuf {
    let mut tracks = vec![vec![
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
        },
        TrackEvent {
            delta: 7680.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ]];
    for (name, events) in voices {
        let mut track = Vec::new();
        if let Some(name) = name {
            track.push(TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::TrackName(name)),
            });
        }
        let mut previous = 0;
        for &(tick, kind) in events {
            track.push(TrackEvent {
                delta: (tick - previous).into(),
                kind,
            });
            previous = tick;
        }
        track.push(TrackEvent {
            delta: (7680 - previous).into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        tracks.push(track);
    }
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(480.into())),
        tracks,
    }
    .save(path)
    .expect("write Take");
    path.to_path_buf()
}

type Event = (u32, TrackEventKind<'static>);

fn midi(tick: u32, channel: u8, message: MidiMessage) -> Event {
    (
        tick,
        TrackEventKind::Midi {
            channel: u4::new(channel),
            message,
        },
    )
}

/// A note on channel 0, as its strike and its release.
fn note(start: u32, duration: u32, key: u8) -> [Event; 2] {
    [
        midi(
            start,
            0,
            MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(64),
            },
        ),
        midi(
            start + duration,
            0,
            MidiMessage::NoteOff {
                key: u7::new(key),
                vel: u7::new(0),
            },
        ),
    ]
}

fn inspect(take: &Path) -> String {
    common::human_output(&["inspect", take.to_str().expect("a path")])
}

#[test]
fn inspect_quotes_each_named_tracks_name_beside_its_number() {
    let names = names_in(Path::new(OVERTURE));
    let listing = inspect(Path::new(OVERTURE));
    let mut seen = vec![false; names.len()];
    for line in listing.lines() {
        for (track, after) in tracks_on(line) {
            let [name] = names[track].as_slice() else {
                panic!("every overture track states one name: {:?}", names[track]);
            };
            assert!(
                after.starts_with(&format!(" \"{name}\"  ")),
                "track {track} should read `track {track} \"{name}\"`: {line}"
            );
            seen[track] = true;
        }
    }
    // Every track that writes anything `inspect` lists was checked, so the
    // loop above cannot pass by finding no rows.
    assert!(seen[1..].iter().all(|seen| *seen), "{seen:?}\n{listing}");
}

/// Every object in a payload that carries a track's number, with the key the
/// number is under: `track`, or `against_track` for the far side of an
/// unranked site.
fn track_bearing(value: &serde_json::Value, found: &mut Vec<(String, serde_json::Value)>) {
    match value {
        serde_json::Value::Object(object) => {
            for key in ["track", "against_track"] {
                if object.get(key).is_some_and(serde_json::Value::is_u64) {
                    found.push((key.to_string(), value.clone()));
                }
            }
            object
                .values()
                .for_each(|value| track_bearing(value, found));
        }
        serde_json::Value::Array(values) => {
            values.iter().for_each(|value| track_bearing(value, found))
        }
        _ => {}
    }
}

#[test]
fn every_object_carrying_a_track_number_carries_its_name() {
    let names = names_in(Path::new(OVERTURE));
    let payload: serde_json::Value =
        serde_json::from_str(&common::json_output(&["inspect", OVERTURE, "--json"]))
            .expect("the payload is JSON");
    let mut found = Vec::new();
    track_bearing(&payload, &mut found);
    assert!(!found.is_empty());
    for (key, object) in &found {
        let track = object[key].as_u64().unwrap() as usize;
        assert_eq!(
            object[format!("{key}_names")],
            serde_json::json!(names[track]),
            "{object}"
        );
    }
}

fn payload(args: &[&str]) -> serde_json::Value {
    serde_json::from_str(&common::json_output(args)).expect("the payload is JSON")
}

fn program(tick: u32, number: u8) -> Event {
    midi(
        tick,
        0,
        MidiMessage::ProgramChange {
            program: u7::new(number),
        },
    )
}

fn struck(tick: u32, key: u8, velocity: u8) -> Event {
    midi(
        tick,
        0,
        MidiMessage::NoteOn {
            key: u7::new(key),
            vel: u7::new(velocity),
        },
    )
}

fn released(tick: u32, key: u8) -> Event {
    midi(
        tick,
        0,
        MidiMessage::NoteOff {
            key: u7::new(key),
            vel: u7::new(0),
        },
    )
}

#[test]
fn a_take_that_names_no_track_prints_no_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut events = note(0, 480, 60).to_vec();
    events.push(program(960, 0));
    let take = build(&dir.path().join("unnamed.mid"), &[(None, events)]);

    let listing = inspect(&take);
    let mut rows = 0;
    for line in listing.lines() {
        for (_, after) in tracks_on(line) {
            assert!(after.starts_with("  "), "{line}");
            rows += 1;
        }
    }
    // The Program's row and the note's.
    assert_eq!(rows, 2, "{listing}");
    assert!(!listing.contains('"'), "{listing}");

    let mut found = Vec::new();
    track_bearing(
        &payload(&["inspect", take.to_str().unwrap(), "--json"]),
        &mut found,
    );
    assert_eq!(found.len(), 2);
    for (key, object) in &found {
        // Present and empty: a Take stating no name says so, rather than
        // leaving the key out.
        assert_eq!(
            object.get(format!("{key}_names")),
            Some(&serde_json::json!([])),
            "{object}"
        );
    }
}

#[test]
fn a_name_is_quoted_as_the_file_states_it() {
    let dir = tempfile::tempdir().unwrap();
    let take = build(
        &dir.path().join("odd.mid"),
        &[
            (Some(b""), note(0, 480, 60).to_vec()),
            (Some(b"control track"), note(0, 480, 62).to_vec()),
            (Some(b"violin"), note(0, 480, 64).to_vec()),
            (Some(b"violin"), note(0, 480, 65).to_vec()),
            (Some("小提琴 I".as_bytes()), note(0, 480, 67).to_vec()),
        ],
    );
    let listing = inspect(&take);
    for (track, quoted, pitch) in [
        (1, r#""""#, "C4"),
        (2, r#""control track""#, "D4"),
        (3, r#""violin""#, "E4"),
        (4, r#""violin""#, "F4"),
        (5, r#""小提琴 I""#, "G4"),
    ] {
        assert!(
            listing
                .lines()
                .any(|line| line.contains(&format!("track {track} {quoted}  "))
                    && line.contains(pitch)),
            "track {track} should read {quoted}:\n{listing}"
        );
    }

    let payload = payload(&["inspect", take.to_str().unwrap(), "--json"]);
    let names: Vec<_> = payload["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|note| (note["track"].clone(), note["track_names"].clone()))
        .collect();
    assert_eq!(
        names,
        [
            (1, ""),
            (2, "control track"),
            (3, "violin"),
            (4, "violin"),
            (5, "小提琴 I"),
        ]
        .map(|(track, name)| (serde_json::json!(track), serde_json::json!([name])))
    );
}

/// A track that states a Program, a Controller and a Bend inside Bar 1, named
/// so the name can be looked for on every row.
fn stating(dir: &Path, file: &str, value: u8) -> PathBuf {
    let mut events = vec![
        program(960, value),
        midi(
            960,
            0,
            MidiMessage::Controller {
                controller: u7::new(11),
                value: u7::new(value),
            },
        ),
        midi(
            960,
            0,
            MidiMessage::PitchBend {
                bend: midly::PitchBend::from_int(i16::from(value) * 10),
            },
        ),
    ];
    events.extend(note(1920, 480, 60));
    build(&dir.join(file), &[(Some(b"the strings"), events)])
}

#[test]
fn no_row_keyed_by_a_channel_carries_a_track_name() {
    let dir = tempfile::tempdir().unwrap();
    let before = stating(dir.path(), "before.mid", 20);
    let after = stating(dir.path(), "after.mid", 30);
    let (before, after) = (before.to_str().unwrap(), after.to_str().unwrap());

    let listing = common::human_output(&["inspect", before, "--bars", "1:2"]);
    let channel_rows: Vec<&str> = listing
        .lines()
        .filter(|line| line.starts_with("channel "))
        .collect();
    // A Program, a Controller and a Bend, each held by channel 0.
    assert_eq!(channel_rows.len(), 3, "{listing}");
    for line in channel_rows {
        assert!(!line.contains("strings"), "{line}");
    }
    // The events that set them are the track's, so they carry its name.
    let stated: Vec<&str> = listing
        .lines()
        .filter(|line| line.starts_with("bar ") && line.contains("channel 0"))
        .collect();
    assert_eq!(stated.len(), 3, "{listing}");
    for line in stated {
        assert!(line.contains(r#"track 1 "the strings"  "#), "{line}");
    }

    let diff = common::human_output(&["diff", before, after]);
    for label in ["program", "controller", "bend"] {
        assert!(
            diff.lines().any(|line| line.starts_with(label)),
            "no {label} row:\n{diff}"
        );
    }
    assert!(!diff.contains("strings"), "{diff}");
}

#[test]
fn diff_names_a_track_as_the_take_its_row_is_about_names_it() {
    let dir = tempfile::tempdir().unwrap();
    // One track, called one thing before and another after. A note the after
    // Take adds is on the after Take's track, one it loses is on the before
    // Take's, and a changed note is described as it was.
    let before = build(
        &dir.path().join("before.mid"),
        &[(
            Some(b"violin"),
            vec![
                struck(0, 60, 64),
                released(480, 60),
                struck(960, 64, 64),
                released(1440, 64),
            ],
        )],
    );
    let after = build(
        &dir.path().join("after.mid"),
        &[(
            Some(b"viola"),
            vec![
                struck(0, 60, 80),
                released(480, 60),
                struck(1920, 67, 64),
                released(2400, 67),
            ],
        )],
    );
    let (before, after) = (before.to_str().unwrap(), after.to_str().unwrap());

    let diff = common::human_output(&["diff", before, after]);
    for (verb, quoted) in [
        ("added", r#"track 1 "viola"  "#),
        ("removed", r#"track 1 "violin"  "#),
        ("changed", r#"track 1 "violin"  "#),
    ] {
        assert!(
            diff.lines()
                .any(|line| line.starts_with(verb) && line.contains(quoted)),
            "{verb} should read {quoted}:\n{diff}"
        );
    }

    let payload = payload(&["diff", before, after, "--json"]);
    let names = |value: &serde_json::Value| value["track_names"].clone();
    assert_eq!(names(&payload["added"][0]), serde_json::json!(["viola"]));
    assert_eq!(names(&payload["removed"][0]), serde_json::json!(["violin"]));
    let changed = &payload["changed"][0];
    assert_eq!(names(&changed["before"]), serde_json::json!(["violin"]));
    assert_eq!(names(&changed["after"]), serde_json::json!(["viola"]));
}

#[test]
fn every_track_an_unranked_reading_names_carries_its_name() {
    let dir = tempfile::tempdir().unwrap();
    // Two tracks stating channel 0 two Programs at one Tick, and the second
    // striking it there too: the file states neither order.
    let take = build(
        &dir.path().join("unranked.mid"),
        &[
            (Some(b"left"), vec![program(0, 10)]),
            (
                Some(b"right"),
                vec![program(0, 20), struck(0, 60, 64), released(480, 60)],
            ),
        ],
    );
    let listing = inspect(&take);
    assert!(
        listing.contains(r#"10 from track 1 "left" at tick 0; 20 from track 2 "right" at tick 0"#),
        "{listing}"
    );
    assert!(
        listing.contains(
            r#"the program change for channel 0 on track 1 "left" has no order the file states against notes of that channel on track 2 "right""#
        ),
        "{listing}"
    );

    let mut found = Vec::new();
    track_bearing(
        &payload(&["inspect", take.to_str().unwrap(), "--json"]),
        &mut found,
    );
    assert!(found.iter().any(|(key, _)| key == "against_track"));
    for (key, object) in &found {
        let expected = match object[key].as_u64().unwrap() {
            1 => "left",
            2 => "right",
            other => panic!("track {other}: {object}"),
        };
        assert_eq!(
            object[format!("{key}_names")],
            serde_json::json!([expected]),
            "{object}"
        );
    }
}

#[test]
fn a_rank_row_names_the_statements_track_as_each_take_names_it() {
    let dir = tempfile::tempdir().unwrap();
    let take = |file: &str, name: &'static [u8], events: Vec<Event>| {
        build(&dir.path().join(file), &[(Some(name), events)])
    };
    let before = take(
        "before.mid",
        b"piano",
        vec![program(0, 40), struck(0, 60, 64), released(480, 60)],
    );
    let kept = take(
        "kept.mid",
        b"piano",
        vec![struck(0, 60, 64), program(0, 40), released(480, 60)],
    );
    let renamed = take(
        "renamed.mid",
        b"keys",
        vec![struck(0, 60, 64), program(0, 40), released(480, 60)],
    );
    let (before, kept, renamed) = (
        before.to_str().unwrap(),
        kept.to_str().unwrap(),
        renamed.to_str().unwrap(),
    );

    let diff = common::human_output(&["diff", before, kept]);
    assert!(diff.contains(r#"; track 1 "piano" program 40"#), "{diff}");
    let diff = common::human_output(&["diff", before, renamed]);
    assert!(
        diff.contains(r#"; track 1 "piano" -> "keys" program 40"#),
        "{diff}"
    );

    // A side naming nothing is the one word `diff --help` gives it, here and
    // only here: a blank would not show that the name changed.
    let unnamed = build(
        &dir.path().join("unnamed.mid"),
        &[(
            None,
            vec![struck(0, 60, 64), program(0, 40), released(480, 60)],
        )],
    );
    let diff = common::human_output(&["diff", before, unnamed.to_str().unwrap()]);
    assert!(
        diff.contains(r#"; track 1 "piano" -> unnamed program 40"#),
        "{diff}"
    );

    let payload = payload(&["diff", before, renamed, "--json"]);
    let statement = &payload["rank_disagreements"][0]["relations"][0]["statement"];
    assert_eq!(
        statement["before_track_names"],
        serde_json::json!(["piano"])
    );
    assert_eq!(statement["after_track_names"], serde_json::json!(["keys"]));
}

#[test]
fn a_track_stating_two_names_is_quoted_with_both_in_written_order() {
    let dir = tempfile::tempdir().unwrap();
    // The second name is written part way in, after the first note: written
    // order, not only the names at the track's start.
    let mut events = note(0, 480, 60).to_vec();
    events.push((960, TrackEventKind::Meta(MetaMessage::TrackName(b"Vln I"))));
    events.extend(note(1920, 480, 62));
    let take = build(&dir.path().join("two.mid"), &[(Some(b"Violin"), events)]);

    let listing = inspect(&take);
    let rows: Vec<&str> = listing
        .lines()
        .filter(|line| line.contains("track 1"))
        .collect();
    assert_eq!(rows.len(), 2, "{listing}");
    for line in rows {
        assert!(line.contains(r#"track 1 "Violin" "Vln I"  "#), "{line}");
    }

    let payload = payload(&["inspect", take.to_str().unwrap(), "--json"]);
    for note in payload["notes"].as_array().unwrap() {
        assert_eq!(note["track_names"], serde_json::json!(["Violin", "Vln I"]));
    }
}

#[test]
fn an_unranked_site_stays_one_site_when_the_takes_name_its_track_differently() {
    let dir = tempfile::tempdir().unwrap();
    // A Program on track 1 and a strike of its channel on track 2 at one
    // Tick, in both Takes; only track 1's name differs.
    let take = |file: &str, name: &'static [u8]| {
        build(
            &dir.path().join(file),
            &[
                (Some(name), vec![program(0, 40)]),
                (Some(b"right"), vec![struck(0, 60, 64), released(480, 60)]),
            ],
        )
    };
    let before = take("before.mid", b"left");
    let after = take("after.mid", b"LEFT");
    let (before, after) = (before.to_str().unwrap(), after.to_str().unwrap());

    let payload = payload(&["diff", before, after, "--json"]);
    assert_eq!(
        payload["unranked_state_sites"],
        serde_json::json!([{
            "site": {
                "tick": 0,
                "channel": 0,
                "controller": null,
                "track": 1,
                "before_track_names": ["left"],
                "after_track_names": ["LEFT"],
                "against_track": 2,
                "before_against_track_names": ["right"],
                "after_against_track_names": ["right"],
                "against": "notes",
                "state": "program",
            },
            "in_before": true,
            "in_after": true,
        }]),
        "{payload}"
    );

    // Each side's row names the track as its own Take does.
    let diff = common::human_output(&["diff", before, after]);
    for (side, quoted) in [
        ("before", r#"on track 1 "left""#),
        ("after", r#"on track 1 "LEFT""#),
    ] {
        assert!(
            diff.lines()
                .any(|line| line.starts_with(side) && line.contains(quoted)),
            "{side} should read {quoted}:\n{diff}"
        );
    }
}
