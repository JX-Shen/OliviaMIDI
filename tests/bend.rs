//! What `inspect` says a channel is bent to — ADR-0007's other half, #43.
//!
//! #32 gave pitch bend to `diff`. The record's terms are more than a
//! comparison: *every command reports the setting*, and `inspect` states what is
//! in force where a passage begins, including what was set many Bars earlier.
//! Until this, `mid diff` could name a difference `mid inspect` could not show.
//!
//! The clause the record spends its Why on is the one about a passage that
//! contains no bend event and is bent anyway. Everything else here is the shape
//! `controllers` and `stated_controllers` already set, followed rather than
//! reinvented.

mod common;

fn listing(take: &std::path::Path, bars: &str) -> String {
    common::human_output(&["inspect", take.to_str().expect("a path"), "--bars", bars])
}

fn payload(take: &std::path::Path, bars: &str) -> serde_json::Value {
    serde_json::from_str(&common::json_output(&[
        "inspect",
        take.to_str().expect("a path"),
        "--bars",
        bars,
        "--json",
    ]))
    .expect("the payload is JSON")
}

// Two independent state tracks, with no boundary strike to obscure value conflicts.
fn split_bends(
    dir: &std::path::Path,
    name: &str,
    left: &[(u32, i16)],
    right: &[(u32, i16)],
) -> std::path::PathBuf {
    use midly::{Format, Header, MetaMessage, Smf, Timing, TrackEvent, TrackEventKind};
    let mut tracks = vec![vec![
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(2, 2, 24, 8)),
        },
        TrackEvent {
            delta: 3840.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ]];
    for statements in [left, right] {
        let mut previous = 0;
        let mut events = Vec::new();
        for &(tick, value) in statements {
            events.push(TrackEvent {
                delta: (tick - previous).into(),
                kind: common::pitch_bend(tick, value).1,
            });
            previous = tick;
        }
        events.push(TrackEvent {
            delta: (3840 - previous).into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        tracks.push(events);
    }
    let path = dir.join(name);
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(480.into())),
        tracks,
    }
    .save(&path)
    .expect("write Take");
    path
}

#[test]
fn a_bend_conflict_is_inherited_until_a_determinate_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let take = split_bends(
        dir.path(),
        "conflict.mid",
        &[(480, -4000), (1920, 0)],
        &[(480, 2000)],
    );
    let middle = payload(&take, "2:2");
    assert_eq!(middle["bends"][0]["value"]["kind"], "indeterminate");
    assert_eq!(
        middle["bends"][0]["value"]["candidates"],
        serde_json::json!([
            {"track": 1, "tick": 480, "value": -4000},
            {"track": 2, "tick": 480, "value": 2000}
        ])
    );
    assert_eq!(middle["bends"][0]["extremes"]["kind"], "incomplete");
    assert_eq!(middle["bends"][0]["unranked"][0]["from"], 960);
    assert_eq!(middle["bends"][0]["unranked"][0]["until"], 1920);
    let said = listing(&take, "2:2");
    assert!(said.contains("indeterminate"), "{said}");
    assert!(
        said.contains("track 1") && said.contains("track 2"),
        "{said}"
    );
    let later = payload(&take, "3:3");
    assert_eq!(
        later["bends"][0]["value"],
        serde_json::json!({"kind":"determinate", "value":0})
    );
    assert_eq!(later["bends"][0]["unranked"], serde_json::json!([]));
}

#[test]
fn bend_conflicts_are_disclosed_in_self_comparison_and_track_rearrangement() {
    let dir = tempfile::tempdir().unwrap();
    let a = split_bends(dir.path(), "a.mid", &[(0, -4000)], &[(0, 2000)]);
    let b = split_bends(dir.path(), "b.mid", &[(0, 2000)], &[(0, -4000)]);
    for other in [&a, &b] {
        let json: serde_json::Value = serde_json::from_str(&common::json_output(&[
            "diff",
            a.to_str().unwrap(),
            other.to_str().unwrap(),
            "--json",
        ]))
        .unwrap();
        assert_eq!(json["bends"], serde_json::json!([]));
        assert_eq!(json["unranked_bends"][0]["from"], 0);
        assert_eq!(
            json["unranked_bends"][0]["before"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            json["unranked_bends"][0]["after"].as_array().unwrap().len(),
            2
        );
        let said = common::human_output(&["diff", a.to_str().unwrap(), other.to_str().unwrap()]);
        assert!(
            said.contains("no determinate differences") && said.contains("not compared"),
            "{said}"
        );
    }
}

#[test]
fn bend_readings_keep_same_track_order_and_agreeing_cross_track_values() {
    let dir = tempfile::tempdir().unwrap();
    for (name, left, right, expected) in [
        ("same-track.mid", vec![(0, 4000), (0, -2000)], vec![], -2000),
        ("agree.mid", vec![(0, -2000)], vec![(0, -2000)], -2000),
        (
            "lasts-agree.mid",
            vec![(0, 4000), (0, -2000)],
            vec![(0, -2000)],
            -2000,
        ),
    ] {
        let take = split_bends(dir.path(), name, &left, &right);
        let read = payload(&take, "2:2");
        assert_eq!(
            read["bends"][0]["value"],
            serde_json::json!({"kind":"determinate", "value":expected})
        );
        assert_eq!(read["bends"][0]["unranked"], serde_json::json!([]));
        let take = battuta::Take::read(&take).unwrap();
        let diff = battuta::diff::diff(&take, &take, None).unwrap();
        assert!(diff.is_empty());
        assert!(diff.unranked_bends.is_empty());
    }
}

#[test]
fn an_unranked_interval_interrupts_a_difference_and_a_later_overwrite_resumes_it() {
    let dir = tempfile::tempdir().unwrap();
    let a = split_bends(
        dir.path(),
        "a.mid",
        &[(0, 100), (480, -4000), (1920, 100)],
        &[(480, 2000)],
    );
    let b = split_bends(dir.path(), "b.mid", &[(0, 0)], &[]);
    for (before, after, before_conflicts) in [(&a, &b, true), (&b, &a, false)] {
        let diff = battuta::diff::diff(
            &battuta::Take::read(before).unwrap(),
            &battuta::Take::read(after).unwrap(),
            Some(0),
        )
        .unwrap();
        assert!(!diff.is_empty());
        assert_eq!(
            diff.bends
                .iter()
                .map(|span| (span.from, span.until))
                .collect::<Vec<_>>(),
            vec![(0, Some(480)), (1920, None)]
        );
        assert_eq!(diff.unranked_bends.len(), 1);
        let interval = &diff.unranked_bends[0];
        assert_eq!((interval.from, interval.until), (480, Some(1920)));
        assert_eq!(!interval.before.is_empty(), before_conflicts);
        assert_eq!(!interval.after.is_empty(), !before_conflicts);
        for span in &diff.bends {
            let side = if before_conflicts {
                &span.before
            } else {
                &span.after
            };
            assert_eq!(side.furthest_down, Some(100));
            assert_eq!(side.furthest_up, Some(100));
        }
    }
}

#[test]
fn an_unranked_bend_strike_relation_does_not_poison_the_continuing_value() {
    let dir = tempfile::tempdir().unwrap();
    let path = common::build_take_stating_apart(
        &dir.path().join("strike.mid"),
        480,
        &[(0, 2, 4)],
        &[common::pitch_bend(0, -2000)],
        &[(0, 240, 60), (1920, 240, 62)],
    );
    let take = battuta::Take::read(&path).unwrap();
    let diff = battuta::diff::diff(&take, &take, None).unwrap();
    assert!(diff.is_empty());
    assert!(diff.unranked_bends.is_empty());
    assert_eq!(diff.unranked_state_sites.len(), 1);
    assert!(diff.unranked_state_sites[0].in_before && diff.unranked_state_sites[0].in_after);
    assert_eq!(
        payload(&path, "2:2")["bends"][0]["value"],
        serde_json::json!({"kind":"determinate", "value":-2000})
    );
}

#[test]
fn a_conflict_inside_a_window_does_not_claim_complete_extremes() {
    let dir = tempfile::tempdir().unwrap();
    let take = split_bends(
        dir.path(),
        "inside.mid",
        &[(0, 0), (1200, -4000), (1440, 200)],
        &[(1200, 2000)],
    );
    let read = payload(&take, "2:2");
    assert_eq!(
        read["bends"][0]["value"],
        serde_json::json!({"kind":"determinate", "value":0})
    );
    assert_eq!(read["bends"][0]["extremes"]["kind"], "incomplete");
    assert_eq!(read["bends"][0]["unranked"][0]["from"], 1200);
    assert_eq!(read["bends"][0]["unranked"][0]["until"], 1440);
    let said = listing(&take, "2:2");
    assert!(said.contains("extremes not summarised"), "{said}");
    assert!(
        said.contains("1440") || said.contains("bar 2 beat 2"),
        "{said}"
    );
}

/// Four Bars of 2/4 at 480 PPQ: Bar N begins at Tick (N-1) * 960.
fn bent(dir: &std::path::Path, name: &str, events: &[(u32, i16)]) -> std::path::PathBuf {
    let mut setting: Vec<(u32, midly::TrackEventKind<'static>)> = events
        .iter()
        .map(|&(tick, value)| common::pitch_bend(tick, value))
        .collect();
    for bar in 0..4u32 {
        setting.push(common::strike(bar * 960, 69));
        setting.push(common::release(bar * 960 + 480, 69));
    }
    common::build_take_setting(&dir.join(name), 480, &[(0, 2, 4)], &setting, &[])
}

/// A passage that bends nothing is still bent, if the Take bent it earlier.
///
/// The whole of what makes this a state and not a listing. The bend is set in
/// Bar 1, the passage is Bars 3 to 4, and not one event that set it is inside —
/// and the notes of Bar 3 sound a bend below the pitch they are written at.
/// ADR-0007's Why, in the smallest shape that shows it.
#[test]
fn a_passage_that_states_no_bend_reports_the_one_set_before_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = bent(dir.path(), "earlier.mid", &[(0, -4000)]);

    assert_eq!(
        listing(&take, "3:4"),
        "\
no programs stated

no controllers stated

channel 0  bend  -4000

bar 3 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s1920:n0
bar 4 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s2880:n0
"
    );
}

/// A channel bent back to the centre is not a channel never bent.
///
/// Nought is a value. A Take that bends a channel to nought has said something
/// about it, and a synthesiser given the two files does the same thing with
/// them only because nought happens to be where a channel starts — which is not
/// a fact the second file states. The same distinction `Controller` draws
/// between holding 0 and holding nothing, and `diff` already draws.
#[test]
fn a_channel_bent_to_the_centre_is_not_a_channel_never_bent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let centred = bent(dir.path(), "centred.mid", &[(0, 0)]);
    let never = bent(dir.path(), "never.mid", &[]);

    assert!(
        listing(&centred, "3:4").contains("channel 0  bend  0"),
        "a channel bent to the centre is not reported"
    );
    assert!(
        !listing(&never, "3:4").contains("bend"),
        "a channel nothing ever bent is reported as bent"
    );
    assert_eq!(payload(&never, "3:4")["bends"], serde_json::json!([]));
    assert_eq!(
        payload(&centred, "3:4")["bends"],
        serde_json::json!([{
            "channel": 0,
            "value": {"kind": "determinate", "value": 0},
            "extremes": {
                "kind": "complete",
                "furthest_down": 0,
                "furthest_down_at": 1920,
                "furthest_up": 0,
                "furthest_up_at": 1920
            },
            "unranked": [],
        }])
    );
}

/// Where the passage bends the channel itself, the state says where it went and
/// the events say where each was said.
///
/// Two excursions, because a bend is signed about a centre: a phrase that dips
/// and returns never rises above where it began, so one extreme would be blind
/// to the dive. The channel opens the passage at -1000, dives to -6000 and
/// climbs to 500, and the row carries both.
#[test]
fn a_passage_that_bends_a_channel_says_how_far_it_goes_each_way() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = bent(
        dir.path(),
        "inside.mid",
        &[(0, -1000), (2400, -6000), (2880, 500)],
    );

    assert_eq!(
        listing(&take, "3:4"),
        "\
no programs stated

no controllers stated

channel 0  bend  -1000 (down to -6000 at bar 3 beat 2, up to 500 at bar 4 beat 1)

bar 3 beat 2  track 1  channel 0  bend  -6000
bar 4 beat 1  track 1  channel 0  bend  500

bar 3 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s1920:n0
bar 4 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s2880:n0
"
    );
}

/// A passage that only rises says only that it rose.
///
/// The clause is printed per direction, so the row says what the passage did
/// and not what it did not. `controller` withholds its peak on the same test —
/// a fact printed twice is a fact a reader has to check against itself.
#[test]
fn a_bend_that_only_rises_does_not_report_a_dive_it_did_not_take() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = bent(dir.path(), "rising.mid", &[(0, -1000), (2400, 3000)]);

    assert!(
        listing(&take, "3:4").contains("channel 0  bend  -1000 (up to 3000 at bar 3 beat 2)"),
        "the row says something about a direction the passage did not take: {}",
        listing(&take, "3:4")
    );
}

/// The payload gains two lists and moves nothing.
///
/// #29's compatibility constraint. A consumer reading `controllers` or `notes`
/// finds them under the same names, holding the same things.
#[test]
fn the_payload_gains_two_lists_and_moves_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = bent(dir.path(), "keys.mid", &[(0, -1000), (2400, 3000)]);
    let payload = payload(&take, "3:4");

    let mut keys: Vec<&str> = payload
        .as_object()
        .expect("the payload is an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "bends",
            "controllers",
            "notes",
            "programs",
            "stated_bends",
            "stated_controllers",
            "stated_programs",
            "unranked",
            "unranked_tempos",
        ]
    );
    assert_eq!(
        payload["stated_bends"],
        serde_json::json!([{
            "track": 1,
            "channel": 0,
            "tick": 2400,
            "value": 3000,
        }])
    );
}

/// A Take that bends nothing says nothing about bends.
///
/// Unlike the Program and Controller blocks, which answer even when the answer
/// is nothing. A reader can name a Controller and ask what a channel holds for
/// it; a bend has no address to have asked about, so silence leaves no question
/// unanswered — and every Take in this repository that bends nothing would
/// otherwise carry a line saying so.
#[test]
fn a_take_that_bends_nothing_prints_no_bend_block() {
    assert_eq!(
        common::human_output(&["inspect", common::FIXTURE, "--bars", "7:8"]),
        "\
no programs stated

no controllers stated

bar 7 beat 1  track 1  E4   velocity 50  duration 955   t1:c0:p64:s8640:n0
bar 7 beat 1  track 2  D2   velocity 45  duration 475   t2:c1:p38:s8640:n0
bar 7 beat 2  track 2  F#3  velocity 38  duration 955   t2:c1:p54:s9120:n0
bar 7 beat 2  track 2  A3   velocity 38  duration 955   t2:c1:p57:s9120:n0
bar 7 beat 3  track 1  C#4  velocity 50  duration 475   t1:c0:p61:s9600:n0
bar 8 beat 1  track 1  D4   velocity 50  duration 1435  t1:c0:p62:s10080:n0
bar 8 beat 1  track 2  D2   velocity 45  duration 475   t2:c1:p38:s10080:n0
bar 8 beat 2  track 2  F#3  velocity 38  duration 955   t2:c1:p54:s10560:n0
bar 8 beat 2  track 2  A3   velocity 38  duration 955   t2:c1:p57:s10560:n0
"
    );
}
