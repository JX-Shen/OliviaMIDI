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

/// Two values at one address: the last one written is what is in force, and
/// nothing folds the other away.
///
/// `EXPRESSIVE` states CC11 twice at Tick 1440 — 30, then 40 — and 40 is what
/// the channel holds. Which of the two that is is a fact about the order they
/// are written in the file rather than about our own builder, which is why this
/// one case needs a committed fixture; `stacked.mid` is here for the same reason
/// with two note-ons.
///
/// The peak in the same assertion is the wobble surviving: the curve reads 76,
/// 80, 78, 84, 90, 88, 96 through Bar 2 and the highest of them is 96, not the
/// end of the first stretch that happened to be rising.
#[test]
fn holds_the_last_of_two_values_written_at_one_address() {
    let json = common::json_output(&["inspect", common::EXPRESSIVE, "--bars", "2:2", "--json"]);
    let (controllers, _) = common::controllers(&json);

    assert!(
        controllers.contains(&serde_json::json!({
            "channel": 0,
            "controller": 11,
            "value": 40,
            "peak": 96,
            "peak_at": 2760,
        })),
        "the state does not hold the second of the two values: {controllers:#?}"
    );
}

/// A channel mode message is not a Controller, and no command mentions one.
///
/// `EXPRESSIVE` states CC123 — All Notes Off — inside Bar 4, so this passage
/// contains it and the silence is the rule at work rather than a Bar range
/// excluding it. Nothing is in force after an instruction happens, and every
/// reading this tool makes of channel state presupposes something in force, so
/// there is nothing here for it to say (ADR-0007).
///
/// The hole is deliberate and this is what keeps it deliberate: an untested hole
/// is one somebody eventually closes by accident.
#[test]
fn never_mentions_a_channel_mode_message() {
    assert_eq!(
        common::human_output(&["inspect", common::EXPRESSIVE, "--bars", "4:4"]),
        "\
no programs stated

channel 0  CC11  100
channel 1  CC64  0

bar 4 beat 1  track 1  D5  velocity 70  duration 1440  t1:c0:p74:s4320:n0
bar 4 beat 1  track 2  F4  velocity 60  duration 480   t2:c1:p65:s4320:n0
bar 4 beat 1  track 3  G2  velocity 55  duration 1440  t3:c2:p43:s4320:n0
"
    );
}

/// An Edit Set naming no Controller leaves every control change where it was —
/// the duplicate address and the mode message included.
///
/// The property rather than a snapshot: what has to hold is that the two Takes
/// *agree* at the event level (ADR-0003). A tool that quietly folded the two
/// values at Tick 1440 into one, or dropped the CC123 it has nothing to say
/// about, would pass every audition and have changed the Piece.
#[test]
fn an_empty_edit_set_keeps_every_control_change() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("take-02.mid");

    common::mid()
        .args(["apply", common::EXPRESSIVE])
        .arg(common::empty_edit_set(dir.path()))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    assert_eq!(
        common::event_stream(std::path::Path::new(common::EXPRESSIVE)),
        common::event_stream(&out)
    );
}

/// `set_controller` states an address rather than naming an identity, so it
/// works whether or not the Take says anything there yet.
///
/// The first Edit lands on an address that holds 40 and changes it; the second
/// lands where the Take says nothing and creates the statement. A Take holding
/// nothing for a Controller is the ordinary case and must not need a different
/// Edit — the reason `set_program` is shaped this way, and why neither goes
/// through ADR-0002's content addressing.
///
/// The track is stated, never inferred. Which track carries a channel's control
/// changes is the author's arrangement of the file, and a tool that guessed
/// would move somebody's expression onto a track they did not put it on.
#[test]
fn set_controller_states_an_address_that_need_not_exist_yet() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("crescendo.mid"),
        480,
        &[(0, 3, 4)],
        &[(1440, 11, 40)],
        &[
            (0, 480, 69),
            (1440, 480, 69),
            (2880, 480, 69),
            (4320, 480, 69),
        ],
    );
    let edits = common::edit_set(
        dir.path(),
        "expression",
        r#"{ "kind": "set_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440, "value": 90 },
           { "kind": "set_controller", "track": 1, "channel": 0, "controller": 11, "tick": 2880, "value": 20 }"#,
    );
    let out = dir.path().join("take-02.mid");

    common::mid()
        .arg("apply")
        .arg(&take)
        .arg(&edits)
        .arg("-o")
        .arg(&out)
        .assert()
        .success();

    let json = common::json_output(&[
        "inspect",
        out.to_str().expect("a path"),
        "--bars",
        "1:4",
        "--json",
    ]);
    let (_, stated) = common::controllers(&json);
    assert_eq!(
        stated,
        vec![
            serde_json::json!({
                "track": 1, "channel": 0, "controller": 11, "tick": 1440, "value": 90,
            }),
            serde_json::json!({
                "track": 1, "channel": 0, "controller": 11, "tick": 2880, "value": 20,
            }),
        ]
    );
}

/// `delete_controller` takes away one statement and leaves the rest.
///
/// Written after the implementation rather than before it: this kind and
/// `move_controller` below share one function and one branch of `Landing` with
/// `set_controller`, so they landed in its cycle. Said plainly here because the
/// loop is red first, and these two were not.
#[test]
fn delete_controller_takes_away_one_statement() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = applied(
        &dir,
        &[(1440, 11, 40), (2880, 11, 100)],
        r#"{ "kind": "delete_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440 }"#,
    );

    let json = common::json_output(&["inspect", out.to_str().expect("a path"), "--bars", "1:4", "--json"]);
    let (_, stated) = common::controllers(&json);
    assert_eq!(
        stated,
        vec![serde_json::json!({
            "track": 1, "channel": 0, "controller": 11, "tick": 2880, "value": 100,
        })]
    );
}

/// `move_controller` carries a statement to another Tick, and overwrites
/// whatever was there.
///
/// The 100 at Bar 3 moves back a Bar onto the 40 at Bar 2, and one value is
/// left. One address holds one value, and reading effects as ordered makes the
/// move the thing that happened last.
///
/// This is the kind that keeps "the brass swells too early" expressible. A
/// crescendo moved is thirty of these, which is what an Edit Set is for; saying
/// it as thirty deletions and thirty statements would leave the Edit Set unable
/// to say even that a move was asked for.
#[test]
fn move_controller_carries_a_statement_and_overwrites_the_destination() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = applied(
        &dir,
        &[(1440, 11, 40), (2880, 11, 100)],
        r#"{ "kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 2880, "delta_ticks": -1440 }"#,
    );

    let json = common::json_output(&["inspect", out.to_str().expect("a path"), "--bars", "1:4", "--json"]);
    let (_, stated) = common::controllers(&json);
    assert_eq!(
        stated,
        vec![serde_json::json!({
            "track": 1, "channel": 0, "controller": 11, "tick": 1440, "value": 100,
        })]
    );
}

/// An address holding nothing is an Edit Set written against a different Take,
/// and is refused rather than quietly doing nothing.
///
/// `set_controller` states an address and so needs none to exist; these two
/// *name* one. The distinction is the same one that lets `set_program` work on a
/// Take that states no Program while `delete_note` refuses an identity it cannot
/// find.
#[test]
fn refuses_to_move_a_controller_no_take_states() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("crescendo.mid"),
        480,
        &[(0, 3, 4)],
        &[(1440, 11, 40)],
        &[(0, 480, 69), (1440, 480, 69)],
    );
    let edits = common::edit_set(
        dir.path(),
        "nowhere",
        r#"{ "kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 2000, "delta_ticks": 480 }"#,
    );

    common::mid()
        .arg("apply")
        .arg(&take)
        .arg(&edits)
        .arg("-o")
        .arg(dir.path().join("take-02.mid"))
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "no Controller is stated on track 1 channel 0 controller 11 at tick 2000",
        ));
}

/// Build a Take stating `controllers`, apply `edits` to it, and hand back the
/// Take that came out.
fn applied(
    dir: &tempfile::TempDir,
    controllers: &[common::StatedController],
    edits: &str,
) -> std::path::PathBuf {
    let take = common::build_take_with_controllers(
        &dir.path().join("in.mid"),
        480,
        &[(0, 3, 4)],
        controllers,
        &[(0, 480, 69), (1440, 480, 69), (2880, 480, 69), (4320, 480, 69)],
    );
    let edits = common::edit_set(dir.path(), "edits", edits);
    let out = dir.path().join("take-02.mid");
    common::mid()
        .arg("apply")
        .arg(&take)
        .arg(&edits)
        .arg("-o")
        .arg(&out)
        .assert()
        .success();
    out
}
