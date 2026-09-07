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
            "value": 0,
            "furthest_down": 0,
            "furthest_down_at": 1920,
            "furthest_up": 0,
            "furthest_up_at": 1920,
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
