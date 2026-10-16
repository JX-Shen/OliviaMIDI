//! A Reset All Controllers, and the Controller readings taken across one — #63.
//!
//! CC121 is a channel mode message and not a Controller, but it ends the value
//! in force of every Controller its channel holds. A reading across it reports
//! the reset, naming where it is, and asserts no value: what the channel
//! returns to is the synthesiser's.
//!
//! Every expected result is a literal over a Take built here, or read out of
//! `fixtures/expressive.mid` with `midly`.

mod common;

use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const RESET_ALL_CONTROLLERS: u8 = 121;

fn control(channel: u8, controller: u8, value: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: channel.into(),
        message: MidiMessage::Controller {
            controller: controller.into(),
            value: value.into(),
        },
    }
}

fn reset(channel: u8) -> TrackEventKind<'static> {
    control(channel, RESET_ALL_CONTROLLERS, 0)
}

fn strike(pitch: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: 0.into(),
        message: MidiMessage::NoteOn {
            key: pitch.into(),
            vel: 64.into(),
        },
    }
}

fn release(pitch: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: 0.into(),
        message: MidiMessage::NoteOff {
            key: pitch.into(),
            vel: 0.into(),
        },
    }
}

/// A Take of 4/4 at 480 PPQ: a conductor track stating the time signature, then
/// each of `tracks` as (Tick, event) in the order it is written, every one
/// ending at Bar 5.
fn take(dir: &Path, name: &str, tracks: &[Vec<(u32, TrackEventKind<'static>)>]) -> PathBuf {
    let end = 4 * 1920;
    let mut smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(480.into())),
        tracks: vec![vec![
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
            },
            TrackEvent {
                delta: end.into(),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            },
        ]],
    };
    for events in tracks {
        let mut track = Vec::new();
        let mut previous = 0;
        for &(tick, kind) in events {
            track.push(TrackEvent {
                delta: (tick - previous).into(),
                kind,
            });
            previous = tick;
        }
        track.push(TrackEvent {
            delta: (end - previous).into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        smf.tracks.push(track);
    }
    let path = dir.join(name);
    smf.save(&path).expect("the Take is writable");
    path
}

/// `fixtures/expressive.mid` with one event added: CC121 on channel 0, on
/// track 1 at Tick 3000 (Bar 3 beat 1+120 in its 3/4 at 480 PPQ), after any
/// event already at that Tick. The reproduction in #63's body.
fn expressive_with_a_reset(dir: &Path) -> PathBuf {
    let bytes = std::fs::read(common::EXPRESSIVE).expect("the fixture is readable");
    let mut smf = Smf::parse(&bytes).expect("the fixture parses");
    let events = &mut smf.tracks[1];
    let mut tick = 0;
    let mut at = events.len();
    for (index, event) in events.iter().enumerate() {
        if tick + event.delta.as_int() > 3000 {
            at = index;
            break;
        }
        tick += event.delta.as_int();
    }
    // The event that follows keeps its own Tick: what it waited is now split
    // between the reset and itself.
    let rest = events[at].delta.as_int() - (3000 - tick);
    events[at].delta = rest.into();
    events.insert(
        at,
        TrackEvent {
            delta: (3000 - tick).into(),
            kind: reset(0),
        },
    );
    let path = dir.join("reset.mid");
    smf.save(&path).expect("the Take is writable");
    path
}

/// What channel 0 of `fixtures/expressive.mid` last states for CC11 before Tick
/// 3000, read with `midly`: the value a reader of the old output was told is in
/// force across the reset.
fn expressive_cc11_before_the_reset() -> u8 {
    let bytes = std::fs::read(common::EXPRESSIVE).expect("the fixture is readable");
    let smf = Smf::parse(&bytes).expect("the fixture parses");
    let mut last = None;
    for track in &smf.tracks {
        let mut tick = 0;
        for event in track {
            tick += event.delta.as_int();
            if let TrackEventKind::Midi {
                channel,
                message: MidiMessage::Controller { controller, value },
            } = event.kind
            {
                if channel.as_int() == 0 && controller.as_int() == 11 && tick <= 3000 {
                    last = Some(value.as_int());
                }
            }
        }
    }
    last.expect("the fixture states CC11 on channel 0")
}

fn json(args: &[&str]) -> Value {
    serde_json::from_str(&common::json_output(args)).expect("--json is JSON")
}

fn controller(listing: &Value, channel: u64, number: u64) -> &Value {
    listing["controllers"]
        .as_array()
        .expect("the payload has controllers")
        .iter()
        .find(|held| held["channel"] == channel && held["controller"] == number)
        .unwrap_or_else(|| panic!("no CC{number} on channel {channel}: {listing:#}"))
}

/// The reproduction in #63's body. Bar 4 begins after the reset and states
/// nothing for CC11, so what the channel holds there is the reset — and no
/// value, least of all the one stated before it.
#[test]
fn reads_a_controller_as_reset_after_a_reset_all_controllers() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = expressive_with_a_reset(dir.path());
    let path = take.to_str().expect("a path");
    assert_eq!(expressive_cc11_before_the_reset(), 100);

    assert_eq!(
        common::human_output(&["inspect", path, "--bars", "4:4"]),
        "\
no programs stated

channel 0  CC11 (expression controller)          reset at bar 3 beat 1+120 (Reset All Controllers)
channel 1  CC64 (damper pedal on/off (sustain))  0

bar 4 beat 1  track 1  D5  velocity 70  duration 1440  t1:c0:p74:s4320:n0
bar 4 beat 1  track 2  F4  velocity 60  duration 480   t2:c1:p65:s4320:n0
bar 4 beat 1  track 3  G2  velocity 55  duration 1440  t3:c2:p43:s4320:n0
"
    );

    let listing = json(&["inspect", path, "--bars", "4:4", "--json"]);
    assert_eq!(
        controller(&listing, 0, 11)["value"],
        json!({"kind": "reset", "at": 3000}),
        "the reading carries its kind and the reset's Tick, and no value"
    );
    for extreme in ["peak", "lowest"] {
        assert_eq!(
            controller(&listing, 0, 11)[extreme],
            json!({"kind": "no_value"}),
            "a passage that holds no value for CC11 has no {extreme}"
        );
    }
}

/// The line `inspect` prints for channel 0's CC11, out of the whole output.
fn controller_line(take: &Path, bars: &str) -> String {
    let said = common::human_output(&["inspect", take.to_str().expect("a path"), "--bars", bars]);
    said.lines()
        .find(|line| line.starts_with("channel 0  CC11"))
        .unwrap_or_else(|| panic!("no CC11 row in:\n{said}"))
        .to_string()
}

/// CC11 at 100 from Bar 1, a reset at Bar 2 and CC11 at `restated` from Bar 3,
/// all on one track: what is in force where each Bar begins is a literal.
fn reset_then_restated_to(dir: &Path, restated: u8) -> PathBuf {
    take(
        dir,
        &format!("restated-{restated}.mid"),
        &[vec![
            (0, control(0, 11, 100)),
            (0, strike(60)),
            (480, release(60)),
            (1920, reset(0)),
            (3840, control(0, 11, restated)),
        ]],
    )
}

/// A later statement of the Controller reads as that value again: the reset
/// holds until the Take says something else, and no longer.
#[test]
fn a_later_statement_reads_as_its_value_again() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = reset_then_restated_to(dir.path(), 70);

    assert_eq!(
        controller_line(&take, "2:2"),
        "channel 0  CC11 (expression controller)  reset at bar 2 beat 1 (Reset All Controllers)"
    );
    assert_eq!(
        controller_line(&take, "3:3"),
        "channel 0  CC11 (expression controller)  70"
    );
    assert_eq!(
        controller_line(&take, "4:4"),
        "channel 0  CC11 (expression controller)  70"
    );
}

/// A passage the reset falls inside says so on the Controller's row. Its
/// extremes are values and cannot: the row would otherwise read as 100 held
/// across the whole passage — the second half of #63's reproduction.
#[test]
fn a_reset_inside_the_passage_is_said_on_the_row() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = reset_then_restated_to(dir.path(), 70);
    let path = take.to_str().expect("a path");

    assert_eq!(
        controller_line(&take, "1:2"),
        "channel 0  CC11 (expression controller)  100  \
         reset at bar 2 beat 1 (Reset All Controllers)"
    );
    assert_eq!(
        controller_line(&take, "2:3"),
        "channel 0  CC11 (expression controller)  \
         reset at bar 2 beat 1 (Reset All Controllers)  \
         peak 70 at bar 3 beat 1, lowest 70 at bar 3 beat 1"
    );

    let listing = json(&["inspect", path, "--bars", "1:2", "--json"]);
    let held = controller(&listing, 0, 11);
    assert_eq!(held["value"], json!({"kind": "determinate", "value": 100}));
    assert_eq!(held["resets"], json!([1920]));
}

/// Two Takes differing only by a CC121 differ in a Controller: from the reset
/// on, one holds 100 and the other holds whatever the reset leaves, which is no
/// value this file states. #63's reproduction against the fixture it was made
/// from.
///
/// The reset event itself is still disclosed as a channel mode message not
/// compared (#62): what it does beyond the Controllers the Take had set is not
/// compared. The fixture holds one, its CC123, and the reproduction adds one.
#[test]
fn diff_reports_a_reset_as_a_controller_difference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = expressive_with_a_reset(dir.path());
    let path = take.to_str().expect("a path");

    assert_eq!(
        common::human_output(&["diff", common::EXPRESSIVE, path]),
        "controller  bar 3 beat 1+120 onwards  channel 0  CC11 (expression controller)  \
         100 -> reset at bar 3 beat 1+120 (Reset All Controllers)\n\
         not compared: channel mode message (before 1, after 2)\n"
    );

    let payload = json(&["diff", common::EXPRESSIVE, path, "--json"]);
    assert_eq!(
        payload["controllers"],
        json!([{
            "channel": 0,
            "controller": 11,
            "from": 3000,
            "until": null,
            "before": {
                "at_start": 100, "at_end": 100, "peak": 100, "peak_at": 3000,
                "lowest": 100, "lowest_at": 3000,
                "reset_at_start": null, "resets": [],
            },
            "after": {
                "at_start": null, "at_end": null, "peak": null, "peak_at": null,
                "lowest": null, "lowest_at": null,
                "reset_at_start": 3000, "resets": [],
            },
        }]),
        "a reset side carries where the reset is, and no value"
    );
}

/// A side that is reset inside a span and stated again says so. Without it
/// the after side below would read as 100 held throughout, every number on it
/// true — ADR-0009's founding case, arriving by a reset.
#[test]
fn a_reset_inside_a_span_is_said_on_its_side() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = take(
        dir.path(),
        "before.mid",
        &[vec![
            (0, control(0, 11, 90)),
            (0, strike(60)),
            (480, release(60)),
        ]],
    );
    let after = reset_then_restated_to(dir.path(), 100);

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path"),
        ]),
        "\
controller  bar 1 beat 1 onwards  channel 0  CC11 (expression controller)
  before    90
  after     100, reset at bar 2 beat 1 (Reset All Controllers), ends at 100
not compared: channel mode message (before 0, after 1)
"
    );
}

/// CC11 at 10 from Tick 300, a reset at Tick 600 and CC11 at 5 from Tick 900,
/// on one track: Bar 1 beat 1+300, beat 2+120 and beat 2+420 in 4/4 at 480 PPQ.
fn ten_reset_five(dir: &Path) -> PathBuf {
    take(
        dir,
        "ten-reset-five.mid",
        &[vec![
            (0, strike(60)),
            (300, control(0, 11, 10)),
            (480, release(60)),
            (600, reset(0)),
            (900, control(0, 11, 5)),
        ]],
    )
}

/// A side's clauses are stated in the order they happen, resets among the
/// extremes. And an extreme reached after a reset is not a fall or a rise from
/// the value the side began on, which the reset ended: it is named as the
/// lowest, with no direction — #69's review of #63 and #68.
#[test]
fn a_span_side_states_its_reset_in_time_order_and_no_fall_across_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = take(
        dir.path(),
        "twenty.mid",
        &[vec![
            (0, strike(60)),
            (300, control(0, 11, 20)),
            (480, release(60)),
        ]],
    );
    let after = ten_reset_five(dir.path());

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path"),
        ]),
        "\
controller  bar 1 beat 1+300 onwards  channel 0  CC11 (expression controller)
  before    20
  after     10, reset at bar 1 beat 2+120 (Reset All Controllers), lowest 5 at bar 1 beat 2+420, ends at 5
not compared: channel mode message (before 0, after 1)
"
    );

    // An extreme before the reset keeps its direction: the value it moved from
    // is still in force there.
    let dips_then_resets = take(
        dir.path(),
        "dips-then-resets.mid",
        &[vec![
            (0, strike(60)),
            (300, control(0, 11, 10)),
            (450, control(0, 11, 5)),
            (480, release(60)),
            (600, reset(0)),
            (900, control(0, 11, 50)),
        ]],
    );
    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            dips_then_resets.to_str().expect("a path"),
        ]),
        "\
controller  bar 1 beat 1+300 onwards  channel 0  CC11 (expression controller)
  before    20
  after     10, down to 5 at bar 1 beat 1+450, reset at bar 1 beat 2+120 (Reset All Controllers), peak 50 at bar 1 beat 2+420, ends at 50
not compared: channel mode message (before 0, after 1)
"
    );
}

/// `inspect`'s Controller row states its extremes and its resets in the order
/// they happen, through the same ordering as `diff`'s sides.
#[test]
fn an_inspect_row_states_its_reset_in_time_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = ten_reset_five(dir.path());

    assert_eq!(
        controller_line(&take, "1:1"),
        "channel 0  CC11 (expression controller)  unstated  \
         peak 10 at bar 1 beat 1+300, reset at bar 1 beat 2+120 (Reset All Controllers), \
         lowest 5 at bar 1 beat 2+420"
    );
}

/// Two resets are one state wherever each was sent, as two equal values are
/// wherever each was stated: a second reset over one already in force is no
/// Controller difference.
#[test]
fn a_reset_over_a_reset_is_no_controller_difference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let once = take(
        dir.path(),
        "once.mid",
        &[vec![(0, control(0, 11, 100)), (1920, reset(0))]],
    );
    let twice = take(
        dir.path(),
        "twice.mid",
        &[vec![
            (0, control(0, 11, 100)),
            (1920, reset(0)),
            (3840, reset(0)),
        ]],
    );

    let payload = json(&[
        "diff",
        once.to_str().expect("a path"),
        twice.to_str().expect("a path"),
        "--json",
    ]);
    assert_eq!(payload["controllers"], json!([]));
}

/// A reset ends only what is in force. A Controller nothing had set before it
/// is still unstated after it, and one stated after a reset at the same Tick —
/// how an export commonly opens a channel — holds the value stated.
#[test]
fn a_reset_ends_only_what_is_in_force() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = take(
        dir.path(),
        "opening.mid",
        &[vec![
            (0, reset(0)),
            (0, control(0, 7, 100)),
            (0, strike(60)),
            (480, release(60)),
            (1920, reset(0)),
            (3840, control(0, 11, 80)),
        ]],
    );
    let path = take.to_str().expect("a path");

    assert_eq!(
        common::human_output(&["inspect", path, "--bars", "1:1"])
            .lines()
            .nth(2),
        Some("channel 0  CC7 (channel volume)  100"),
        "the reset written before the statement at one Tick ends nothing"
    );

    let listing = json(&["inspect", path, "--bars", "2:3", "--json"]);
    assert_eq!(
        controller(&listing, 0, 7)["value"],
        json!({"kind": "reset", "at": 1920})
    );
    assert_eq!(
        controller(&listing, 0, 11)["value"],
        json!({"kind": "unstated"}),
        "a reset before a Controller is first stated leaves it unstated"
    );
    assert_eq!(controller(&listing, 0, 11)["resets"], json!([]));
}

/// A reset on one track and a statement on another, at one Tick, have no order
/// between them the file states (ADR-0008): one order leaves the value, the
/// other the reset. So the reading is indeterminate, and the reset is one of its
/// candidates, with no value.
#[test]
fn a_reset_unranked_against_a_statement_is_indeterminate() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = take(
        dir.path(),
        "unranked.mid",
        &[
            vec![
                (0, control(0, 11, 100)),
                (0, strike(60)),
                (480, release(60)),
                (1920, reset(0)),
            ],
            vec![(1920, control(0, 11, 60))],
        ],
    );
    let path = take.to_str().expect("a path");

    let listing = json(&["inspect", path, "--bars", "2:2", "--json"]);
    assert_eq!(
        controller(&listing, 0, 11)["value"],
        json!({"kind": "indeterminate", "candidates": [
            {"track": 1, "track_names": [], "tick": 1920, "value": null},
            {"track": 2, "track_names": [], "tick": 1920, "value": 60},
        ]})
    );
    let said = common::human_output(&["inspect", path, "--bars", "2:2"]);
    assert!(
        said.contains(
            "indeterminate (reset from track 1 at tick 1920; 60 from track 2 at tick 1920)"
        ),
        "{said}"
    );
}
