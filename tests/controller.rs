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

channel 0  CC11 (expression controller)  100

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

channel 0  CC11 (expression controller)  40  peak 100 at bar 2 beat 3+360

bar 2 beat 2+240  track 1  channel 0  CC11 (expression controller)  70
bar 2 beat 3+360  track 1  channel 0  CC11 (expression controller)  100

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

channel 0  CC11 (expression controller)  40  peak 100 at bar 2 beat 3

bar 2 beat 1+240  track 1  channel 0  CC11 (expression controller)  76
bar 2 beat 2      track 1  channel 0  CC11 (expression controller)  80
bar 2 beat 2+240  track 1  channel 0  CC11 (expression controller)  78
bar 2 beat 3      track 1  channel 0  CC11 (expression controller)  100
bar 2 beat 3+240  track 1  channel 0  CC11 (expression controller)  96

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

channel 0  CC64 (damper pedal on/off (sustain))  unstated  peak 127 at bar 3 beat 2

bar 3 beat 2  track 1  channel 0  CC64 (damper pedal on/off (sustain))  127
bar 4 beat 1  track 1  channel 0  CC64 (damper pedal on/off (sustain))  0

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
        "controller  bar 6 beat 1 until bar 7 beat 1  channel 0  CC11 (expression controller)  70 -> 100\n"
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

channel 0  CC11 (expression controller)          100
channel 1  CC64 (damper pedal on/off (sustain))  0

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
        &[
            (0, 480, 69),
            (1440, 480, 69),
            (2880, 480, 69),
            (4320, 480, 69),
        ],
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

/// What MIDI's own table calls a Controller, printed beside the number, and
/// nothing printed where the table names none.
///
/// Quoted rather than improved on: CC64 is *damper pedal* in the specification
/// and *sustain pedal* to every pianist, and reaching for the friendlier word
/// would stop the parenthesis being a quotation and start it being this tool's
/// opinion of what the control is for.
///
/// The `CC` prefix is where the attribution sits, which is why there is no `GM`
/// style label here. A GM name depends on which bank is loaded and so is a Rig
/// fact needing one; which control a number *means* depends on nothing but
/// MIDI, like `pitch 66` being `F#4`, and `pitch_name` carries no label either.
///
/// CC20 is one of the numbers the specification leaves undefined, and it gets no
/// parenthesis at all. `undefined` is not a name — it is the table saying
/// nothing — and the honest way to print that is to print nothing, as an
/// unnamed Program prints `unstated` rather than 0.
///
/// Every name here was read against the MMA's own Control Change list rather
/// than against another tool's table.
#[test]
fn names_a_controller_the_way_midis_own_table_does() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("named.mid"),
        480,
        &[(0, 3, 4)],
        &[(1440, 11, 40), (1440, 20, 7)],
        &[(0, 480, 69), (1440, 480, 69)],
    );

    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path"), "--bars", "2:2"]),
        "\
no programs stated

channel 0  CC11 (expression controller)  40
channel 0  CC20                          7

bar 2 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s1440:n0
"
    );
}

/// The two clauses a name loses on the way to a table cell, and everything it
/// keeps.
///
/// Deleting some of a quotation is not replacing a word of it, and only two
/// kinds of clause are deleted — a cross reference to a paper the reader does
/// not have, and what a control used to be called. Each of the five rows here is
/// one case of that rule, read against the MMA's own list:
///
/// - CC7 is *channel volume (formerly main volume)*: the alias is the whole
///   parenthesis, so the whole parenthesis goes
/// - CC20 the specification leaves undefined, so there is no name to print
/// - CC39 is *lsb for control 7 (channel volume, formerly main volume)*: here
///   the alias is a clause inside a parenthesis whose rest *is* the name, so only
///   the clause goes
/// - CC74 is *sound controller 5 (default: brightness)* and keeps every word.
///   `default: brightness` is the whole of why CC74 is used as brightness, and a
///   trim that took it would leave a name that had stopped being informative
///   rather than one that had stopped being long
/// - CC75 adds *– see mma rp-021*, which points at a document the reader does
///   not have and says nothing about the control
#[test]
fn trims_a_cross_reference_and_an_alias_and_nothing_else() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("trims.mid"),
        480,
        &[(0, 3, 4)],
        &[
            (1440, 7, 90),
            (1440, 20, 7),
            (1440, 39, 3),
            (1440, 74, 64),
            (1440, 75, 20),
        ],
        &[(0, 480, 69), (1440, 480, 69)],
    );

    assert_eq!(
        common::human_output(&["inspect", take.to_str().expect("a path"), "--bars", "2:2"]),
        "\
no programs stated

channel 0  CC7 (channel volume)                             90
channel 0  CC20                                             7
channel 0  CC39 (lsb for control 7 (channel volume))        3
channel 0  CC74 (sound controller 5 (default: brightness))  64
channel 0  CC75 (sound controller 6 (default: decay time))  20

bar 2 beat 1  track 1  A4  velocity 64  duration 480  t1:c0:p69:s1440:n0
"
    );
}

/// A Controller stated at a Tick reaches the synthesiser before the notes there.
///
/// Audible rather than arithmetic, and the same claim a program change already
/// makes: a note-on at that Tick has to sound under the value the Take now holds
/// there. Placed after it, the value arrives after the note it was set for, the
/// note sounds under the old one, and the Edit is inaudible in exactly the Bar
/// it was asked for — an `apply` that succeeded, a file that plays, and
/// `inspect` reporting a state the passage does not begin in.
#[test]
fn an_inserted_control_change_precedes_the_notes_at_its_tick() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("plain.mid"),
        480,
        &[(0, 3, 4)],
        &[],
        &[(0, 480, 69), (1440, 480, 69)],
    );
    let output = dir.path().join("out.mid");
    let edits = common::edit_set(
        dir.path(),
        "state-it",
        r#"{"kind": "set_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440, "value": 100}"#,
    );

    common::mid()
        .arg("apply")
        .arg(&take)
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        events_of_track(&output, 1),
        vec![
            (0, "strikes"),
            (480, "releases"),
            (1440, "controller"),
            (1440, "strikes"),
            (1920, "releases"),
        ]
    );
}

/// A Controller moved onto a Tick reaches the synthesiser before the notes
/// there, as a stated one does.
///
/// The two Edits arrive at the same address by different routes, and a reader
/// who was told the value is in force from that Tick has no way to tell which
/// route was taken. Only one of the two may be audible there.
#[test]
fn a_moved_control_change_precedes_the_notes_at_its_tick() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("early.mid"),
        480,
        &[(0, 3, 4)],
        &[(960, 11, 100)],
        &[(0, 480, 69), (1440, 480, 69)],
    );
    let output = dir.path().join("out.mid");
    let edits = common::edit_set(
        dir.path(),
        "move-it",
        r#"{"kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 960, "delta_ticks": 480}"#,
    );

    common::mid()
        .arg("apply")
        .arg(&take)
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        events_of_track(&output, 1),
        vec![
            (0, "strikes"),
            (480, "releases"),
            (1440, "controller"),
            (1440, "strikes"),
            (1920, "releases"),
        ]
    );
}

/// Two Controllers stated at one Tick reach the synthesiser in the order the
/// Edit Set asked for.
///
/// An Edit Set is ordered, and for some Controllers the order *is* the meaning:
/// a Data Entry following the RPN it belongs to is a different message from the
/// two the other way round. This is not a claim about which state goes first
/// against the notes — it is the guard on the mechanism that answers that
/// question, because a fix that placed state events by counting downwards would
/// hand back every such pair reversed.
#[test]
fn controllers_stated_at_one_tick_keep_the_order_the_edit_set_asked_for() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_with_controllers(
        &dir.path().join("pair.mid"),
        480,
        &[(0, 3, 4)],
        &[],
        &[(0, 480, 69), (1440, 480, 69)],
    );
    let output = dir.path().join("out.mid");
    let edits = common::edit_set(
        dir.path(),
        "both",
        r#"{"kind": "set_controller", "track": 1, "channel": 0, "controller": 101, "tick": 1440, "value": 0},
           {"kind": "set_controller", "track": 1, "channel": 0, "controller": 100, "tick": 1440, "value": 0},
           {"kind": "set_controller", "track": 1, "channel": 0, "controller": 6,   "tick": 1440, "value": 2}"#,
    );

    common::mid()
        .arg("apply")
        .arg(&take)
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        controllers_at(&output, 1, 1440),
        vec![(101, 0), (100, 0), (6, 2)],
        "the Edit Set's order was not the order the file states them in"
    );
}

/// A named Controller Edit means the event it resolved against, for the whole
/// Edit Set.
///
/// Targets are fixed while effects are ordered, and a Controller Edit is not
/// exempt because it names an address rather than an identity. `EXPRESSIVE`
/// states CC11 twice at Tick 1440 — 30, then 40 — so 40 is the one in force and
/// the one both of these Edits mean. The first carries it clear of the address;
/// the second has to carry the *same* event further, not turn round and find the
/// 30 the first one left behind.
///
/// A move counts from where its event now is, as `move_note` does: a second
/// Edit on a moved target is asking for a further distance, not restating an
/// address that no longer holds anything.
#[test]
fn a_named_controller_edit_keeps_the_event_it_resolved_against() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = common::edit_set(
        dir.path(),
        "twice",
        r#"{"kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440, "delta_ticks": 30},
           {"kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440, "delta_ticks": 30}"#,
    );

    common::mid()
        .args(["apply", common::EXPRESSIVE])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        stated_controller(&output, 1, 0, 11, 1440),
        vec![30],
        "the Edit Set did not name the 30, and it was moved anyway"
    );
    assert_eq!(
        stated_controller(&output, 1, 0, 11, 1500),
        vec![40],
        "the event both Edits named did not arrive where the second one asked"
    );
}

/// A named Controller Edit still means its event after an earlier Edit moved it.
///
/// The second half of targets-fixed: the first Edit carries the 40 clear of the
/// address, and the delete has to follow it rather than take the 30 left behind.
/// Taking the 30 would be an Edit reaching an event the Edit Set never named,
/// which ADR-0003 forbids — and it would leave the one that *was* named sitting
/// where the Edit Set had just said to remove it from.
#[test]
fn a_delete_follows_the_event_an_earlier_move_carried_off() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = common::edit_set(
        dir.path(),
        "move-then-delete",
        r#"{"kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440, "delta_ticks": 30},
           {"kind": "delete_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440}"#,
    );

    common::mid()
        .args(["apply", common::EXPRESSIVE])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        stated_controller(&output, 1, 0, 11, 1440),
        vec![30],
        "the Edit Set named neither the 30 nor the address it sits at"
    );
    assert_eq!(
        stated_controller(&output, 1, 0, 11, 1470),
        Vec::<u8>::new(),
        "the event both Edits named survived the delete"
    );
}

/// A named Controller Edit whose event an earlier Edit deleted fails the whole
/// Edit Set.
///
/// The same refusal `change_note` makes for a note an earlier Edit removed. The
/// target resolved — it was in the input Take — and now has nowhere for an
/// effect to land, which is a different thing from never having existed and is
/// refused rather than quietly skipped. Turning instead to the other statement
/// at the address would be the failure this ticket is about, wearing a success
/// exit code.
#[test]
fn a_move_whose_event_an_earlier_edit_deleted_fails_the_edit_set() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = common::edit_set(
        dir.path(),
        "delete-then-move",
        r#"{"kind": "delete_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440},
           {"kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440, "delta_ticks": 30}"#,
    );

    common::mid()
        .args(["apply", common::EXPRESSIVE])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .failure();

    assert!(
        !output.exists(),
        "an Edit Set that failed left a Take behind"
    );
}

/// A `set_controller` and a later `move_controller` at one address reach the
/// same event.
///
/// `set_controller` states an address and changes what is in force there; a
/// `move_controller` naming that address means the same statement, now carrying
/// the value the first Edit gave it. Effects are ordered, so the move carries
/// 90 and not the 40 the Take arrived with.
#[test]
fn a_move_carries_the_value_an_earlier_set_gave_its_event() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = common::edit_set(
        dir.path(),
        "set-then-move",
        r#"{"kind": "set_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440, "value": 90},
           {"kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440, "delta_ticks": 30}"#,
    );

    common::mid()
        .args(["apply", common::EXPRESSIVE])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(stated_controller(&output, 1, 0, 11, 1440), vec![30]);
    assert_eq!(stated_controller(&output, 1, 0, 11, 1470), vec![90]);
}

/// A named Controller Edit cannot reach an address an earlier `set_controller`
/// created.
///
/// Targets are fixed against the *input* Take, so the address this move names
/// held nothing in the Take the Edit Set was written against — exactly as an
/// Edit cannot name a note an earlier `add_note` created. The refusal is the
/// same one a Take stating nothing there gets, because from the resolver's side
/// it is the same fact.
#[test]
fn a_move_cannot_name_an_address_an_earlier_set_created() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = common::edit_set(
        dir.path(),
        "set-new-then-move",
        r#"{"kind": "set_controller", "track": 1, "channel": 0, "controller": 11, "tick": 5000, "value": 50},
           {"kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 5000, "delta_ticks": 100}"#,
    );

    common::mid()
        .args(["apply", common::EXPRESSIVE])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "no Controller is stated on track 1 channel 0 controller 11 at tick 5000",
        ));
    assert!(!output.exists());
}

/// A move onto an address states one value there and leaves the other statement.
///
/// The Edit Set named neither of the two the Take states at Tick 1440, so
/// neither may be folded away for being redundant (ADR-0003) — but the mover has
/// to end up in force there, because reading effects as ordered makes it the
/// thing that happened last. Both hold: the statement that *was* in force gives
/// way to the mover, and the one under it stays exactly where it was.
#[test]
fn a_move_onto_a_duplicate_address_gives_way_only_to_the_statement_in_force() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = common::edit_set(
        dir.path(),
        "onto-1440",
        r#"{"kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1560, "delta_ticks": -120}"#,
    );

    common::mid()
        .args(["apply", common::EXPRESSIVE])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        stated_controller(&output, 1, 0, 11, 1440),
        vec![30, 48],
        "the 40 was in force and gave way; the 30 was named by nothing and stays"
    );
    assert_eq!(
        stated_controller(&output, 1, 0, 11, 1560),
        Vec::<u8>::new(),
        "the mover was left behind at the address it came from"
    );
}

/// The refusal names the address the Edit Set wrote, not where the event stands.
///
/// An earlier Edit may have carried the target somewhere else before the one
/// that fails. Naming the Tick it has reached would hand back a number appearing
/// nowhere in the Edit Set, and the reader's job is to find the Edit that is
/// wrong.
#[test]
fn a_refused_controller_edit_names_the_address_the_edit_set_wrote() {
    let dir = tempfile::tempdir().expect("temp dir");
    let edits = common::edit_set(
        dir.path(),
        "move-delete-move",
        r#"{"kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440, "delta_ticks": 30},
           {"kind": "delete_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440},
           {"kind": "move_controller", "track": 1, "channel": 0, "controller": 11, "tick": 1440, "delta_ticks": 30}"#,
    );

    common::mid()
        .args(["apply", common::EXPRESSIVE])
        .arg(&edits)
        .arg("--output")
        .arg(dir.path().join("out.mid"))
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "deleted the Controller on track 1 channel 0 controller 11 at tick 1440",
        ))
        .stderr(predicates::function::function(|stderr: &str| {
            !stderr.contains("1470")
        }));
}

/// One track's events in file order, as (tick, what it does) — enough to see
/// which of two events sharing a Tick the synthesiser meets first.
fn events_of_track(path: &std::path::Path, track: usize) -> Vec<(u32, &'static str)> {
    let bytes = std::fs::read(path).expect("Take is readable");
    let smf = midly::Smf::parse(&bytes).expect("Take parses");
    let mut found = Vec::new();
    let mut tick = 0u32;
    for event in &smf.tracks[track] {
        tick += event.delta.as_int();
        let midly::TrackEventKind::Midi { message, .. } = event.kind else {
            continue;
        };
        found.push((
            tick,
            match message {
                midly::MidiMessage::Controller { .. } => "controller",
                midly::MidiMessage::NoteOn { vel, .. } if vel.as_int() > 0 => "strikes",
                midly::MidiMessage::NoteOff { .. } | midly::MidiMessage::NoteOn { .. } => {
                    "releases"
                }
                _ => continue,
            },
        ));
    }
    found
}

/// The control changes one track states at one Tick, as (controller, value), in
/// the order the file lists them.
fn controllers_at(path: &std::path::Path, track: usize, at: u32) -> Vec<(u8, u8)> {
    let bytes = std::fs::read(path).expect("Take is readable");
    let smf = midly::Smf::parse(&bytes).expect("Take parses");
    let mut found = Vec::new();
    let mut tick = 0u32;
    for event in &smf.tracks[track] {
        tick += event.delta.as_int();
        if let midly::TrackEventKind::Midi {
            message: midly::MidiMessage::Controller { controller, value },
            ..
        } = event.kind
        {
            if tick == at {
                found.push((controller.as_int(), value.as_int()));
            }
        }
    }
    found
}

/// The values one track states for one Controller of one channel at one Tick,
/// in file order. Plural because a Take may state two at one address, and which
/// of them an Edit moved is the whole question.
fn stated_controller(
    path: &std::path::Path,
    track: usize,
    channel: u8,
    controller: u8,
    at: u32,
) -> Vec<u8> {
    let bytes = std::fs::read(path).expect("Take is readable");
    let smf = midly::Smf::parse(&bytes).expect("Take parses");
    let mut found = Vec::new();
    let mut tick = 0u32;
    for event in &smf.tracks[track] {
        tick += event.delta.as_int();
        if let midly::TrackEventKind::Midi {
            channel: on,
            message:
                midly::MidiMessage::Controller {
                    controller: number,
                    value,
                },
        } = event.kind
        {
            if tick == at && on.as_int() == channel && number.as_int() == controller {
                found.push(value.as_int());
            }
        }
    }
    found
}
