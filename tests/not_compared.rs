//! `mid diff` names the content it did not compare — #62, under ADR-0009.
//!
//! `no differences` is the strongest sentence `mid` prints, and it was printed
//! about Takes that differ in content `diff` never looks at. What is tested here
//! is that it is now printed only where it is true, and that everywhere else the
//! kinds that differ are named instead.
//!
//! The kinds are not written down in this file. They are read out of
//! `battuta::Uncompared`, the type that decides them, through `serde` naming
//! every variant it would have accepted — the way `tests/contract.rs` reads the
//! Edit kinds. What is written here is one example Take per kind, and a kind
//! without one fails `every_kind_not_compared_is_named_rather_than_no_differences`,
//! so a kind added to the type cannot go untested, and a kind `diff` starts
//! comparing fails the same test until it leaves the type.
//!
//! Each example is `fixtures/expressive.mid` with one event added or changed,
//! as the reproductions in #62 build them. The expected counts are literals over
//! that fixture, which `tests/common/mod.rs` describes: one time signature on
//! track 0, one channel mode message (CC123 on track 2), twelve notes each
//! released by a note-off, four tracks each ending at its last event, and none
//! of the rest.

mod common;

use midly::num::{u4, u7};
use midly::{MetaMessage, MidiMessage, TrackEventKind};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The kinds `diff` does not compare, from the type that decides them.
fn kinds() -> Vec<String> {
    let refusal = serde_json::from_str::<battuta::Uncompared>(r#""""#)
        .expect_err("an empty name is not a kind")
        .to_string();
    let (_, listed) = refusal
        .split_once("expected one of ")
        .expect("serde names the variants it would have taken");
    listed
        .split(&[',', ' '][..])
        .filter_map(|word| word.trim().strip_prefix('`'))
        .filter_map(|word| word.split('`').next())
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// A Take's events with absolute Ticks, one list per track.
type Tracks<'a> = Vec<Vec<(u32, TrackEventKind<'a>)>>;

/// Put an event on a track at a Tick: after anything already there, before the
/// end of track. A SysEx goes before them instead, as a setup message would. An
/// event past the end of track moves the end to it, so that the end stays last.
fn add<'a>(tracks: &mut Tracks<'a>, track: usize, tick: u32, kind: TrackEventKind<'a>) {
    let events = &mut tracks[track];
    let (end, _) = events.pop().expect("a track has an end");
    let at = if matches!(kind, TrackEventKind::SysEx(_)) {
        events.iter().position(|&(t, _)| t >= tick)
    } else {
        events.iter().position(|&(t, _)| t > tick)
    }
    .unwrap_or(events.len());
    events.insert(at, (tick, kind));
    events.push((end.max(tick), TrackEventKind::Meta(MetaMessage::EndOfTrack)));
}

fn on_channel_0(message: MidiMessage) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::new(0),
        message,
    }
}

/// `fixtures/expressive.mid`, changed by `change`, written to `dir/name.mid`.
fn expressive_with(dir: &Path, name: &str, change: impl FnOnce(&mut Tracks)) -> PathBuf {
    let bytes = std::fs::read(common::EXPRESSIVE).expect("the fixture is readable");
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
        .map(|events| {
            let mut previous = 0;
            events
                .into_iter()
                .map(|(tick, kind)| {
                    let delta = midly::num::u28::new(tick - previous);
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

/// One kind's example: what the human output calls it, how much of it is
/// counted, and the change that makes the after Take.
struct Example {
    words: &'static str,
    count: Count,
    change: fn(&mut Tracks),
}

/// How much of a kind is counted, in the unit the kind is counted in.
enum Count {
    /// How many events of it each Take holds.
    Events(usize, usize),
    /// How many matched notes differ in it.
    NotesDiffering(usize),
}

impl Count {
    fn json(&self, kind: &str) -> Value {
        match *self {
            Count::Events(before, after) => {
                json!({ "kind": kind, "before": before, "after": after })
            }
            Count::NotesDiffering(notes) => json!({ "kind": kind, "notes_differing": notes }),
        }
    }

    fn said(&self) -> String {
        match *self {
            Count::Events(before, after) => format!("before {before}, after {after}"),
            Count::NotesDiffering(1) => "1 matched note differs".to_string(),
            Count::NotesDiffering(notes) => format!("{notes} matched notes differ"),
        }
    }
}

/// The example for a kind, by the name the type gives it. `None` is a kind this
/// file has no example for, which is a failure, not a skip.
fn example(kind: &str) -> Option<Example> {
    let example = |words, before, after, change| {
        Some(Example {
            words,
            count: Count::Events(before, after),
            change,
        })
    };
    match kind {
        // `FF 58 04 04 02 18 08` on track 0 at Tick 2880. Track 0's end moves
        // with it, to the new last event.
        "time_signature" => example("time signature", 1, 2, |tracks| {
            add(
                tracks,
                0,
                2880,
                TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
            )
        }),
        // `D0 40` on track 1 at Tick 1440.
        "channel_pressure" => example("channel pressure", 0, 1, |tracks| {
            add(
                tracks,
                1,
                1440,
                on_channel_0(MidiMessage::ChannelAftertouch { vel: u7::new(64) }),
            )
        }),
        // `A0 47 40` on track 1 at Tick 1440.
        "polyphonic_key_pressure" => example("polyphonic key pressure", 0, 1, |tracks| {
            add(
                tracks,
                1,
                1440,
                on_channel_0(MidiMessage::Aftertouch {
                    key: u7::new(71),
                    vel: u7::new(64),
                }),
            )
        }),
        // `B0 7B 00`, All Notes Off, on track 1 at Tick 3000. The fixture's
        // own CC123 is the one before. Not #62's own reproduction, a Reset All
        // Controllers at the same place: since #63 that reset is also read as
        // ending the CC11 channel 0 holds, which is a determinate Controller
        // difference, while the event itself stays disclosed here.
        "channel_mode" => example("channel mode message", 1, 2, |tracks| {
            add(
                tracks,
                1,
                3000,
                on_channel_0(MidiMessage::Controller {
                    controller: u7::new(123),
                    value: u7::new(0),
                }),
            )
        }),
        // `F0 05 7E 7F 09 01 F7`, GM System On, on track 0 at Tick 0.
        "sysex" => example("SysEx", 0, 1, |tracks| {
            add(
                tracks,
                0,
                0,
                TrackEventKind::SysEx(&[0x7E, 0x7F, 0x09, 0x01, 0xF7]),
            )
        }),
        // `FF 59 02 02 00`, two sharps, major, on track 0 at Tick 0.
        "key_signature" => example("key signature", 0, 1, |tracks| {
            add(
                tracks,
                0,
                0,
                TrackEventKind::Meta(MetaMessage::KeySignature(2, false)),
            )
        }),
        // `FF 05 02 6C 61`, the lyric `la`, on track 1 at Tick 0.
        "lyric" => example("lyric", 0, 1, |tracks| {
            add(
                tracks,
                1,
                0,
                TrackEventKind::Meta(MetaMessage::Lyric(b"la")),
            )
        }),
        // `FF 01 02 68 69`, the text `hi`, on track 1 at Tick 0.
        "text" => example("text", 0, 1, |tracks| {
            add(tracks, 1, 0, TrackEventKind::Meta(MetaMessage::Text(b"hi")))
        }),
        // `FF 03 06 56 69 6F 6C 69 6E`, the track name `Violin`, on track 1 at
        // Tick 0.
        "track_name" => example("track name", 0, 1, |tracks| {
            add(
                tracks,
                1,
                0,
                TrackEventKind::Meta(MetaMessage::TrackName(b"Violin")),
            )
        }),
        // Track 1's first note-off, `80 45 00` at Tick 1440, given a release
        // velocity: `80 45 40`. One matched note now differs in it.
        "release_velocity" => Some(Example {
            words: "release velocity",
            count: Count::NotesDiffering(1),
            change: |tracks| {
                let (_, kind) = tracks[1]
                    .iter_mut()
                    .find(|(tick, kind)| {
                        *tick == 1440
                            && matches!(
                                kind,
                                TrackEventKind::Midi {
                                    message: MidiMessage::NoteOff { .. },
                                    ..
                                }
                            )
                    })
                    .expect("track 1 releases a note at Tick 1440");
                *kind = on_channel_0(MidiMessage::NoteOff {
                    key: u7::new(69),
                    vel: u7::new(64),
                });
            },
        }),
        // Track 3 ends a beat after its last note-off, where it ended at it:
        // two Takes of different lengths, alike in every other event.
        "end_of_track" => example("end of track", 0, 1, |tracks| {
            let (tick, _) = tracks[3].last_mut().expect("track 3 has an end");
            *tick += 480;
        }),
        // `FF 21 01 00`, a MIDI port, on track 1 at Tick 0.
        "other" => example("other event", 0, 1, |tracks| {
            add(
                tracks,
                1,
                0,
                TrackEventKind::Meta(MetaMessage::MidiPort(u7::new(0))),
            )
        }),
        _ => None,
    }
}

/// Every list in a diff's payload that holds a determinate difference.
const DETERMINATE: [&str; 8] = [
    "added",
    "removed",
    "changed",
    "programs",
    "controllers",
    "rank_disagreements",
    "tempos",
    "bends",
];

fn diff_json(before: &str, after: &Path) -> Value {
    serde_json::from_str(&common::json_output(&[
        "diff",
        before,
        after.to_str().expect("a path"),
        "--json",
    ]))
    .expect("diff --json is JSON")
}

#[test]
fn serde_gives_up_every_kind_not_compared() {
    let kinds = kinds();
    // Not a count and not a list, for `tests/contract.rs`'s reason: only that
    // the reading worked, so a `serde` whose wording changed fails here rather
    // than leaving the test below checking nothing.
    assert!(kinds.len() > 1, "read no kinds out of serde: {kinds:?}");
    assert!(
        kinds
            .iter()
            .all(|kind| kind.chars().all(|c| c.is_ascii_lowercase() || c == '_')),
        "read something that is not a kind: {kinds:?}"
    );
}

/// Each of #62's reproductions, and one for every other kind the type names:
/// the after Take differs from the fixture in that kind alone, and the diff
/// says so by name instead of saying `no differences`.
///
/// Both directions of the list's drift fail here. A kind with no example fails
/// on the `expect`. A kind `diff` has started comparing fails on the empty
/// determinate lists, because its example now produces a difference.
#[test]
fn every_kind_not_compared_is_named_rather_than_no_differences() {
    let dir = tempfile::tempdir().expect("temp dir");
    for kind in kinds() {
        let Example {
            words,
            count,
            change,
        } = example(&kind)
            .unwrap_or_else(|| panic!("`{kind}` is not compared and has no example here"));
        let take = expressive_with(dir.path(), &kind, change);

        let diff = diff_json(common::EXPRESSIVE, &take);
        for field in DETERMINATE {
            assert_eq!(
                diff[field],
                json!([]),
                "`{kind}` alone made a determinate difference in `{field}`, so it is compared"
            );
        }
        assert_eq!(
            diff["not_compared"],
            json!([count.json(&kind)]),
            "`{kind}` is not disclosed as it should be"
        );

        assert_eq!(
            common::human_output(&["diff", common::EXPRESSIVE, take.to_str().expect("a path")]),
            format!(
                "no determinate differences; not compared: {words} ({})\n",
                count.said()
            ),
            "`{kind}`"
        );
    }
}

/// `no differences` survives where it is true. The fixture holds kinds `diff`
/// does not compare — a time signature, a channel mode message, release
/// velocities — and a Take does not differ from itself in any of them.
#[test]
fn a_take_compared_with_itself_still_has_no_differences() {
    assert_eq!(
        common::human_output(&["diff", common::EXPRESSIVE, common::EXPRESSIVE]),
        "no differences\n"
    );
    let diff = diff_json(common::EXPRESSIVE, Path::new(common::EXPRESSIVE));
    assert_eq!(diff["not_compared"], json!([]));
}

/// A determinate difference does not hide the disclosure: the row is printed,
/// and the kinds not compared follow it. Two kinds here, in the order the type
/// declares them, which is the order `mid` lists them in.
#[test]
fn a_difference_and_a_kind_not_compared_are_both_shown() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = expressive_with(dir.path(), "both", |tracks| {
        // Track 3's first note, A2 on channel 2 at Tick 0, from velocity 55 to 70.
        let (_, kind) = tracks[3].first_mut().expect("track 3 strikes first");
        *kind = TrackEventKind::Midi {
            channel: u4::new(2),
            message: MidiMessage::NoteOn {
                key: u7::new(45),
                vel: u7::new(70),
            },
        };
        add(
            tracks,
            1,
            1440,
            on_channel_0(MidiMessage::ChannelAftertouch { vel: u7::new(64) }),
        );
        add(
            tracks,
            0,
            0,
            TrackEventKind::Meta(MetaMessage::KeySignature(2, false)),
        );
    });

    assert_eq!(
        common::human_output(&["diff", common::EXPRESSIVE, take.to_str().expect("a path")]),
        "\
changed  bar 1 beat 1  track 3  channel 2  A2  velocity 55 -> 70
not compared: key signature (before 0, after 1), channel pressure (before 0, after 1)
"
    );
    let diff = diff_json(common::EXPRESSIVE, &take);
    assert_eq!(diff["changed"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        diff["not_compared"],
        json!([
            { "kind": "key_signature", "before": 0, "after": 1 },
            { "kind": "channel_pressure", "before": 0, "after": 1 },
        ])
    );
}

/// The help no longer reads as if all of the Piece were compared.
#[test]
fn the_help_says_some_of_the_piece_is_not_compared() {
    let out = common::mid()
        .args(["diff", "--help"])
        .output()
        .expect("mid runs");
    assert!(out.status.success());
    let help = String::from_utf8_lossy(&out.stdout);
    assert!(
        help.contains("not compared:"),
        "the help does not say how a kind not compared is reported:\n{help}"
    );
}
