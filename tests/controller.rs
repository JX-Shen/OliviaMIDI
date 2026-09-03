//! Controller data: what a channel holds, and where it says so — #13.
//!
//! `mid` carried control changes faithfully long before it mentioned one, in
//! exactly the state #11 names: preserved, invisible, untouchable. The middle
//! one is the dangerous one here for the reason it was dangerous for a Program,
//! only worse — a human who cannot see the expression curve will diagnose a
//! badly shaped phrase as a badly written line and edit the notes, and `mid
//! diff` will faithfully report that they edited the notes.
//!
//! What is *not* here is any summary of a curve. ADR-0007 settles that channel
//! state is reported as what is in force rather than as the events that set it,
//! so nothing below groups events into a rise or a fall.

mod common;

/// The value a passage begins holding, when the events that set it are all
/// behind it.
///
/// The crescendo is over by the end of Bar 2 and the passage starts at Bar 3, so
/// not one of these events is inside it — and 100 is still what the channel
/// holds when the first note of the passage sounds. This is ADR-0007's whole
/// claim at its smallest: the answer is the state, and the events are where it
/// came from.
#[test]
fn states_what_each_channel_of_the_passage_holds() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("crescendo.mid"),
        480,
        &[(0, 3, 4)],
        &[(1440, 11, 40), (2160, 11, 70), (2760, 11, 100)],
        &[
            (0, 480, 69),
            (1440, 480, 69),
            (2880, 480, 69),
            (4320, 480, 69),
        ],
    );

    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path"), "--bars", "3:4"]),
        "\
no programs stated

channel 0  CC11  100

bar 3 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s2880:n0
bar 4 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s4320:n0
"
    );
}

/// A passage holding nothing for any Controller says so, rather than saying
/// nothing.
///
/// `FIXTURE` states not one control change, and the line is still printed. It is
/// the whole of what #11 asks for: a reader who is told nothing cannot tell a
/// Take with no expression shaping from a tool that does not look, and only one
/// of those two lets them stop suspecting the curve and go back to the notes.
///
/// One line for the block rather than a row per channel, which is where this
/// parts company with a Program. A channel holds exactly one Program, so
/// `unstated` beside it is a fact about that channel; a channel has a hundred
/// and twenty Controllers it says nothing about, and printing that many
/// non-statements is not more informative than printing one.
#[test]
fn says_when_the_passage_holds_nothing_for_any_controller() {
    assert_eq!(
        common::human_output(&["inspect", common::FIXTURE, "--bars", "5:6"]),
        "\
no programs stated

no controllers stated

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

/// Every place the passage itself states a Controller, under the state and
/// above the notes.
///
/// Listed rather than summarised. Forty events are forty rows, as thirty-six
/// notes are thirty-six rows: listing is what `inspect` does, and `--bars` is
/// how a reader narrows it. Where the debt for a mechanically large change is
/// owed is `diff` (ADR-0007, and #11).
///
/// The value in force at the passage's own first Tick is the state, not an
/// event, so the 40 stated at Bar 2 Beat 1 appears above and not below — the cut
/// `programs_in` makes, for the same reason: it is what a listener hears from
/// the moment the passage starts.
#[test]
fn lists_every_place_the_passage_states_a_controller() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("crescendo.mid"),
        480,
        &[(0, 3, 4)],
        &[(1440, 11, 40), (2160, 11, 70), (2760, 11, 100)],
        &[
            (0, 480, 69),
            (1440, 480, 69),
            (2880, 480, 69),
            (4320, 480, 69),
        ],
    );

    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path"), "--bars", "2:2"]),
        "\
no programs stated

channel 0  CC11  40  peak 100 at bar 2 beat 3+360

bar 2 beat 2+240  track 1  channel 0  CC11  70
bar 2 beat 3+360  track 1  channel 0  CC11  100

bar 2 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s1440:n0
"
    );
}

/// Where the passage's highest value for a Controller falls, beside what the
/// channel holds when it begins.
///
/// Two facts, not a summary of a shape: what is in force at the first note, and
/// the highest thing in force anywhere in the passage. Both are read out of the
/// file, which is why neither needs a parameter (ADR-0007, and the amendment it
/// put on ADR-0004).
///
/// The curve wobbles on purpose. A CC11 line recorded from a physical fader is
/// not monotone — it reads 76, 80, 78, 100, 96 — and this is the only assertion
/// that the peak is the passage's highest value rather than the end of some
/// stretch that happened to be rising.
#[test]
fn states_where_the_passages_highest_value_falls() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("wobble.mid"),
        480,
        &[(0, 3, 4)],
        &[
            (1440, 11, 40),
            (1680, 11, 76),
            (1920, 11, 80),
            (2160, 11, 78),
            (2400, 11, 100),
            (2640, 11, 96),
        ],
        &[(0, 480, 69), (1440, 480, 69), (2880, 480, 69)],
    );

    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path"), "--bars", "2:2"]),
        "\
no programs stated

channel 0  CC11  40  peak 100 at bar 2 beat 3

bar 2 beat 1+240  track 1  channel 0  CC11  76
bar 2 beat 2      track 1  channel 0  CC11  80
bar 2 beat 2+240  track 1  channel 0  CC11  78
bar 2 beat 3      track 1  channel 0  CC11  100
bar 2 beat 3+240  track 1  channel 0  CC11  96

bar 2 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s1440:n0
"
    );
}

/// A Controller the passage names but nothing set beforehand is described as
/// stating none, never as holding 0.
///
/// The pedal goes down in Bar 3 and the Take says nothing about CC64 before
/// that. Zero and unstated are two different Pieces — a synthesiser starting a
/// passage with the pedal up sounds the same as one that was never told, which
/// is exactly why the two must be told apart here rather than by ear. It is
/// #12's sixth criterion, one level in.
///
/// `unstated` is per Controller here where the block-level line is per passage,
/// and the two do not contradict: the passage has named CC64, so there is one
/// thing to say nothing about rather than a hundred and twenty.
#[test]
fn says_unstated_for_a_controller_the_passage_names_and_nothing_set() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("pedal.mid"),
        480,
        &[(0, 3, 4)],
        &[(3360, 64, 127), (4320, 64, 0)],
        &[
            (0, 480, 69),
            (1440, 480, 69),
            (2880, 480, 69),
            (4320, 480, 69),
        ],
    );

    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path"), "--bars", "3:4"]),
        "\
no programs stated

channel 0  CC64  unstated  peak 127 at bar 3 beat 2

bar 3 beat 2  track 1  channel 0  CC64  127
bar 4 beat 1  track 1  channel 0  CC64  0

bar 3 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s2880:n0
bar 4 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s4320:n0
"
    );
}

/// What an agent is handed: the state and every event, and no gloss on either.
///
/// The state carries `null` rather than 0 where the Take set nothing, which is
/// the payload's version of `unstated` — an agent deciding whether to write a
/// `set_controller` needs the two apart for the reason a human reading the
/// terminal does.
///
/// Asserted whole, so that a spec name arriving in the payload later would fail
/// here. A name is a gloss for somebody reading a terminal, and an agent is
/// entitled to one spelling of a fact — the same cut #7 made for pitch names.
#[test]
fn hands_an_agent_the_state_and_every_event() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("pedal.mid"),
        480,
        &[(0, 3, 4)],
        &[(3360, 64, 127), (4320, 64, 0)],
        &[
            (0, 480, 69),
            (1440, 480, 69),
            (2880, 480, 69),
            (4320, 480, 69),
        ],
    );

    let json = common::json_output(&[
        "inspect",
        take.to_str().expect("a path"),
        "--bars",
        "3:4",
        "--json",
    ]);
    let (controllers, stated) = common::controllers(&json);

    assert_eq!(
        controllers,
        vec![serde_json::json!({
            "channel": 0,
            "controller": 64,
            "value": null,
            "peak": 127,
            "peak_at": 3360,
        })]
    );
    assert_eq!(
        stated,
        vec![
            serde_json::json!({
                "track": 1,
                "channel": 0,
                "controller": 64,
                "tick": 3360,
                "value": 127,
            }),
            serde_json::json!({
                "track": 1,
                "channel": 0,
                "controller": 64,
                "tick": 4320,
                "value": 0,
            }),
        ]
    );
}

/// The difference #13 was written to make sayable: the expression now reaches
/// 100 by Bar 6 where it used to reach it by Bar 7.
///
/// One row, not forty. It is a span over what is *in force* — from the Tick the
/// two Takes stop agreeing to the Tick they agree again — and every number in it
/// is read out of one file or the other. Nothing here decides that these forty
/// events and those forty events were the same crescendo; that claim would need
/// a parameter under ADR-0004, and it is not needed to say the sentence
/// (ADR-0007).
#[test]
fn reports_a_controller_difference_as_a_span_over_what_is_in_force() {
    let dir = tempfile::tempdir().expect("temp dir");
    let notes = &[(5760, 480, 69), (7200, 480, 69), (8640, 480, 69)];
    let before = common::build_take_with_controllers(
        &dir.path().join("before.mid"),
        480,
        &[(0, 3, 4)],
        &[(5760, 11, 40), (7200, 11, 70), (8640, 11, 100)],
        notes,
    );
    let after = common::build_take_with_controllers(
        &dir.path().join("after.mid"),
        480,
        &[(0, 3, 4)],
        &[(5760, 11, 40), (7200, 11, 100), (8640, 11, 100)],
        notes,
    );

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path"),
        ]),
        "controller  bar 6 beat 1 until bar 7 beat 1  channel 0  CC11  70 -> 100\n"
    );
}
