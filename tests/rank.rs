//! Rank as the oracle a comparison asks — ADR-0008's amendment, #33.
//!
//! Every Take here holds the *same events at the same Ticks* as the one it is
//! compared with. That is the whole point: the layers `mid diff` already had —
//! notes, Programs, Controllers — all answer "no differences" to every pair
//! below, because the difference is not in what the file says but in the order
//! it says it. A synthesiser hears two different things.
//!
//! What is deliberately not here is a comparison of two event streams, which
//! ADR-0007 refuses by name. Nothing below reads an event's position for its own
//! sake; each reads the order of a pair one event of which governs the other,
//! which is the only order the rule makes a claim about.

mod common;

/// The tolerance disclosure, which every `mid diff` writes to stderr. Not part
/// of any assertion here, and named so that a reader of the output below is not
/// left wondering where it went.
fn diff(before: &std::path::Path, after: &std::path::Path) -> String {
    common::human_output(&[
        "diff",
        before.to_str().expect("a path"),
        after.to_str().expect("a path"),
    ])
}

fn diff_json(before: &std::path::Path, after: &std::path::Path) -> serde_json::Value {
    serde_json::from_str(&common::json_output(&[
        "diff",
        before.to_str().expect("a path"),
        after.to_str().expect("a path"),
        "--json",
    ]))
    .expect("the payload is JSON")
}

/// A Program met after the notes it governs is a different Piece from the same
/// Program met before them.
///
/// The counterexample #29 asks for by name. Both Takes state program 40 on
/// channel 0 at Tick 0 and strike A4 there; `programs` agrees, the note lists
/// agree, and the after Take's chord sounds on whatever the channel was on
/// before — which, here, is nothing the file names.
#[test]
fn a_program_met_after_the_notes_it_governs_is_a_difference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::program_change(0, 40),
            common::strike(0, 69),
            common::release(480, 69),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::program_change(0, 40),
            common::release(480, 69),
        ],
        &[],
    );

    assert_eq!(
        diff(&before, &after),
        "rank  bar 1 beat 1  channel 0  program before its notes  before follows the rule, \
         after does not; track 1 program 40 occurrence 0 vs strike t1:c0:p69:s0:n0: \
         state before note -> state after note\n"
    );
}

/// The same disagreement, the other way round, and everything the payload says
/// about it.
#[test]
fn the_payload_names_the_pair_and_says_which_take_follows_the_rule() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::program_change(0, 40),
            common::release(480, 69),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::program_change(0, 40),
            common::strike(0, 69),
            common::release(480, 69),
        ],
        &[],
    );

    let payload = diff_json(&before, &after);
    assert_eq!(
        payload["rank_disagreements"],
        serde_json::json!([{
            "tick": 0,
            "channel": 0,
            "pair": "program_before_strike",
            "before_is_correct": false,
            "after_is_correct": true,
            "relations": [{
                "statement": { "track": 1, "value": 40, "occurrence": 0 },
                "before_note": "t1:c0:p69:s0:n0",
                "after_note": "t1:c0:p69:s0:n0",
                "before_state_first": false,
                "after_state_first": true,
            }],
        }])
    );
    assert_eq!(payload["unranked_sites"], serde_json::json!([]));
    assert_eq!(
        diff(&before, &after),
        "rank  bar 1 beat 1  channel 0  program before its notes  after follows the rule, \
         before does not; track 1 program 40 occurrence 0 vs strike t1:c0:p69:s0:n0: \
         state after note -> state before note\n"
    );
}

/// A damper met before the notes it should not catch is a difference.
///
/// ADR-0008's second pair, and the one notation had already settled: a
/// pianist's `Ped.` under a beat means the foot comes down as the hand lifts.
/// The after Take's pedal arrives first and holds a note the Take ends there.
#[test]
fn a_damper_met_before_the_releases_at_its_tick_is_a_difference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::release(480, 69),
            common::control_change(480, 64, 127),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::control_change(480, 64, 127),
            common::release(480, 69),
        ],
        &[],
    );

    assert_eq!(
        diff(&before, &after),
        "rank  bar 1 beat 2  channel 0  damper after its releases  before follows the rule, \
         after does not; track 1 CC64 127 occurrence 0 vs release t1:c0:p69:s0:n0: \
         state after note -> state before note\n"
    );
}

/// Two note-ons of a chord written the other way round are not a difference.
///
/// Neither note governs the other, so the rule ranks the pair not at all and
/// their written order carries no claim about the music. Reporting it would be
/// reporting the byte order of a chord, which is the event-stream comparison
/// ADR-0007 refuses.
#[test]
fn two_strikes_of_one_tick_carry_no_order_to_disagree_about() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::strike(0, 72),
            common::release(480, 69),
            common::release(480, 72),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 72),
            common::strike(0, 69),
            common::release(480, 72),
            common::release(480, 69),
        ],
        &[],
    );

    assert_eq!(diff(&before, &after), "no differences\n");
    assert_eq!(
        diff_json(&before, &after)["rank_disagreements"],
        serde_json::json!([])
    );
}

/// A Controller that is not the damper carries no ordering claim either.
///
/// The rule ADR-0008 states for a damper is notation's, and it is about a
/// damper. An expression curve met on the other side of a note at one Tick is
/// left alone rather than ranked by analogy — the pairs are the two the record
/// names, and a third would be this project inventing one.
#[test]
fn an_expression_value_and_a_release_carry_no_order_to_disagree_about() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::release(480, 69),
            common::control_change(480, 11, 100),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::control_change(480, 11, 100),
            common::release(480, 69),
        ],
        &[],
    );

    assert_eq!(diff(&before, &after), "no differences\n");
}

/// A Take compared with itself has no ordering disagreement with itself.
#[test]
fn a_take_does_not_disagree_with_itself_about_rank() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_setting(
        &dir.path().join("take.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::program_change(0, 40),
            common::release(480, 69),
        ],
        &[],
    );

    assert_eq!(diff(&take, &take), "no differences\n");
}

/// A pair split across two tracks is reported as unranked and is not a
/// difference.
///
/// `build_take_stating_apart` writes the notes on one track and the Program on
/// another, which is exactly the place ADR-0008 says the file states no order
/// and none the rule can give. Both Takes leave it open, so neither can be
/// compared with the other — and `no differences` is still the answer, because
/// a site the question could not be put at is not an answer of "different".
#[test]
fn a_pair_split_across_two_tracks_is_reported_and_is_not_a_difference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_stating_apart(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[common::program_change(0, 40)],
        &[(0, 480, 69)],
    );
    let after = common::build_take_stating_apart(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[common::program_change(0, 40)],
        &[(0, 480, 69)],
    );

    assert_eq!(
        diff(&before, &after),
        "no determinate differences; some content was not compared\n\
         unranked  bar 1 beat 1  channel 0  program before its notes  neither states an order \
         between the tracks\n\
         before  bar 1 beat 1  the program change for channel 0 on track 2 has no order the file states against notes of that channel on track 1  (t2:c0:s0)\n\
         after   bar 1 beat 1  the program change for channel 0 on track 2 has no order the file states against notes of that channel on track 1  (t2:c0:s0)\n"
    );

    let payload = diff_json(&before, &after);
    assert_eq!(payload["rank_disagreements"], serde_json::json!([]));
    assert_eq!(
        payload["unranked_sites"],
        serde_json::json!([{
            "tick": 0,
            "channel": 0,
            "pair": "program_before_strike",
            "in_before": true,
            "in_after": true,
        }])
    );
}

/// A note difference and an ordering difference are two layers of one answer,
/// and a reader can tell which is which.
///
/// #29's mixed counterexample. The velocity change and the reordering are
/// independent, and neither hides the other.
#[test]
fn a_note_change_and_an_ordering_change_are_both_reported() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::program_change(0, 40),
            common::strike(0, 69),
            common::release(480, 69),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::program_change(0, 40),
            common::release(960, 69),
        ],
        &[],
    );

    assert_eq!(
        diff(&before, &after),
        "rank     bar 1 beat 1  channel 0  program before its notes  before follows the rule, \
         after does not; track 1 program 40 occurrence 0 vs strike t1:c0:p69:s0:n0: \
         state before note -> state after note\n\
         changed  bar 1 beat 1  track 1    A4                        duration 480 -> 960\n"
    );
}

/// Every field the payload had before this is still there, under its own name
/// and its own type. #29's compatibility constraint, as an assertion.
#[test]
fn the_two_new_payload_fields_are_additive() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_setting(
        &dir.path().join("take.mid"),
        480,
        &[(0, 4, 4)],
        &[common::strike(0, 69), common::release(480, 69)],
        &[],
    );

    let payload = diff_json(&take, &take);
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
            "added",
            // #32's two, which joined the inventory the same way and are
            // asserted here for the same reason: this list is the whole of the
            // payload, so a field added anywhere has to be added here too.
            "bends",
            "changed",
            "controllers",
            "programs",
            "rank_disagreements",
            "removed",
            "tempos",
            "tolerance_ticks",
            "unranked_bends",
            "unranked_controllers",
            "unranked_programs",
            "unranked_sites",
            "unranked_state_sites",
            "unranked_tempos",
        ]
    );
}

/// A strike sharing a Tick with a damper is not what the damper is ranked
/// against.
///
/// ADR-0008 ranks a damper against the notes *released* at its Tick, and
/// against nothing else. A Tick that ends one note and begins another is the
/// limit `mid apply --help` states rather than a rule: one point cannot be both
/// after the releases and before the strikes, and this reports the half the
/// record decides rather than inventing the other.
#[test]
fn a_strike_at_the_dampers_tick_is_not_what_it_is_ranked_against() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::release(480, 69),
            common::strike(480, 71),
            common::control_change(480, 64, 127),
            common::release(960, 71),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::release(480, 69),
            common::control_change(480, 64, 127),
            common::strike(480, 71),
            common::release(960, 71),
        ],
        &[],
    );

    assert_eq!(diff(&before, &after), "no differences\n");
}

/// A note-on at velocity zero is a release here too.
///
/// The format spells a release two ways and `fixtures/stacked.mid` uses both.
/// Counting the second spelling as a strike would leave a damper ranked against
/// nothing at a Tick where it is ranked against something — the same mistake the
/// placement rule avoids one layer down, made by the reading instead of by the
/// write.
#[test]
fn a_note_on_at_velocity_zero_is_a_release_for_the_damper_too() {
    fn released(tick: u32, key: u8) -> (u32, midly::TrackEventKind<'static>) {
        (
            tick,
            midly::TrackEventKind::Midi {
                channel: midly::num::u4::new(0),
                message: midly::MidiMessage::NoteOn {
                    key: midly::num::u7::new(key),
                    vel: midly::num::u7::new(0),
                },
            },
        )
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            released(480, 69),
            common::control_change(480, 64, 127),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 69),
            common::control_change(480, 64, 127),
            released(480, 69),
        ],
        &[],
    );

    assert_eq!(
        diff(&before, &after),
        "rank  bar 1 beat 2  channel 0  damper after its releases  before follows the rule, \
         after does not; track 1 CC64 127 occurrence 0 vs release t1:c0:p69:s0:n0: \
         state after note -> state before note\n"
    );
}

/// A pair only one of the two Takes carries is not an ordering difference.
///
/// The after Take states no Program at all, so there is nothing at that Tick
/// whose order could disagree with anything. What changed is the content, and
/// `programs` already says so; reporting it here as well would be one change
/// counted twice under two names, and would tell a reader that an ordering they
/// never wrote is wrong.
#[test]
fn a_pair_the_other_take_does_not_carry_is_not_an_ordering_difference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::program_change(0, 40),
            common::strike(0, 69),
            common::release(480, 69),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[common::strike(0, 69), common::release(480, 69)],
        &[],
    );

    assert_eq!(
        diff(&before, &after),
        "program  bar 1 beat 1  channel 0  40 (GM violin) -> unstated\n"
    );
    assert_eq!(
        diff_json(&before, &after)["rank_disagreements"],
        serde_json::json!([])
    );
}

/// A mixed site can change its Program/strike relations while both Takes
/// depart from the placement rule. See #33.
#[test]
fn two_mixed_program_sites_can_disagree_about_which_strike_precedes_the_program() {
    let dir = tempfile::tempdir().expect("temp dir");
    let build = |name: &str, first, second| {
        common::build_take_setting(
            &dir.path().join(name),
            480,
            &[(0, 4, 4)],
            &[
                common::program_change(0, 0),
                common::strike(480, first),
                common::program_change(480, 40),
                common::strike(480, second),
                common::release(960, 60),
                common::release(960, 64),
            ],
            &[],
        )
    };
    let before = build("before.mid", 60, 64);
    let after = build("after.mid", 64, 60);
    for take in [&before, &after] {
        assert_eq!(diff(take, take), "no differences\n");
    }
    for (left, right) in [(&before, &after), (&after, &before)] {
        let payload = diff_json(left, right);
        assert_eq!(payload["changed"], serde_json::json!([]));
        assert_eq!(payload["programs"], serde_json::json!([]));
        assert_eq!(payload["rank_disagreements"].as_array().unwrap().len(), 1);
        let rank = &payload["rank_disagreements"][0];
        assert_eq!(rank["before_is_correct"], false);
        assert_eq!(rank["after_is_correct"], false);
        let first_pitch = if left == &before { 60 } else { 64 };
        let second_pitch = if left == &before { 64 } else { 60 };
        assert_eq!(
            rank["relations"],
            serde_json::json!([
                {
                    "statement": {"track": 1, "value": 40, "occurrence": 0},
                    "before_note": format!("t1:c0:p{first_pitch}:s480:n0"),
                    "after_note": format!("t1:c0:p{first_pitch}:s480:n0"),
                    "before_state_first": false, "after_state_first": true,
                },
                {
                    "statement": {"track": 1, "value": 40, "occurrence": 0},
                    "before_note": format!("t1:c0:p{second_pitch}:s480:n0"),
                    "after_note": format!("t1:c0:p{second_pitch}:s480:n0"),
                    "before_state_first": true, "after_state_first": false,
                }
            ])
        );
        let human = diff(left, right);
        assert!(human.contains("neither follows the rule"));
        assert!(human.contains(&format!(
            "strike t1:c0:p{first_pitch}:s480:n0: state after note -> state before note"
        )));
        assert!(human.contains(&format!(
            "strike t1:c0:p{second_pitch}:s480:n0: state before note -> state after note"
        )));
    }
}

/// A one-sided cross-track pair is still disclosed in either comparison
/// direction, even when the other Take carries no pair. See #33.
#[test]
fn a_one_sided_unranked_site_is_disclosed_in_both_comparison_directions() {
    let dir = tempfile::tempdir().expect("temp dir");
    let unranked = common::build_take_stating_apart(
        &dir.path().join("unranked.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::program_change(0, 40),
            common::program_change(480, 40),
        ],
        &[(480, 480, 69)],
    );
    let absent = common::build_take_stating_apart(
        &dir.path().join("absent.mid"),
        480,
        &[(0, 4, 4)],
        &[common::program_change(0, 40)],
        &[(480, 480, 69)],
    );
    for (left, right, in_before, in_after) in [
        (&unranked, &absent, true, false),
        (&absent, &unranked, false, true),
    ] {
        let payload = diff_json(left, right);
        assert_eq!(payload["rank_disagreements"], serde_json::json!([]));
        assert!(battuta::diff::diff(
            &battuta::Take::read(left).expect("before Take"),
            &battuta::Take::read(right).expect("after Take"),
            Some(0),
        )
        .expect("comparable Takes")
        .is_empty());
        assert_eq!(
            payload["unranked_sites"],
            serde_json::json!([{
                "tick": 480,
                "channel": 0,
                "pair": "program_before_strike",
                "in_before": in_before,
                "in_after": in_after,
            }])
        );
    }
}

/// Permuting strikes on one side of a Program changes no causal relation. #33.
#[test]
fn a_mixed_site_ignores_strike_permutations_that_do_not_cross_the_state() {
    let dir = tempfile::tempdir().unwrap();
    let build = |name: &str, first, second| {
        common::build_take_setting(
            &dir.path().join(name),
            480,
            &[(0, 4, 4)],
            &[
                common::strike(0, first),
                common::strike(0, second),
                common::program_change(0, 40),
                common::strike(0, 67),
                common::release(480, 60),
                common::release(480, 64),
                common::release(480, 67),
            ],
            &[],
        )
    };
    let before = build("before.mid", 60, 64);
    let after = build("after.mid", 64, 60);
    assert_eq!(diff(&before, &after), "no differences\n");
    assert_eq!(diff(&after, &before), "no differences\n");
}

/// Repeated statements and colliding notes retain their own occurrences. #33.
#[test]
fn duplicate_notes_and_statements_keep_their_corresponding_relations() {
    let dir = tempfile::tempdir().unwrap();
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 60),
            common::program_change(0, 40),
            common::strike(0, 60),
            common::program_change(0, 40),
            common::release(480, 60),
            common::release(480, 60),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 60),
            common::program_change(0, 40),
            common::program_change(0, 40),
            common::strike(0, 60),
            common::release(480, 60),
            common::release(480, 60),
        ],
        &[],
    );
    let payload = diff_json(&before, &after);
    assert_eq!(payload["changed"], serde_json::json!([]));
    assert_eq!(
        payload["rank_disagreements"][0]["relations"],
        serde_json::json!([{
            "statement": {"track": 1, "value": 40, "occurrence": 1},
            "before_note": "t1:c0:p60:s0:n1", "after_note": "t1:c0:p60:s0:n1",
            "before_state_first": false, "after_state_first": true,
        }])
    );
    assert_eq!(diff(&before, &before), "no differences\n");
    assert_eq!(diff(&after, &after), "no differences\n");
}

/// Damper relations name the released note and its original start, rather
/// than the release's position in a list. #33.
#[test]
fn mixed_damper_sites_report_which_note_release_crossed_the_state() {
    let dir = tempfile::tempdir().unwrap();
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 60),
            common::strike(240, 64),
            common::release(480, 60),
            common::control_change(480, 64, 127),
            common::release(480, 64),
        ],
        &[],
    );
    let after = common::build_take_setting(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::strike(0, 60),
            common::strike(240, 64),
            common::release(480, 64),
            common::control_change(480, 64, 127),
            common::release(480, 60),
        ],
        &[],
    );
    let payload = diff_json(&before, &after);
    assert_eq!(payload["changed"], serde_json::json!([]));
    let rank = &payload["rank_disagreements"][0];
    assert_eq!(rank["pair"], "damper_after_release");
    assert_eq!(rank["before_is_correct"], false);
    assert_eq!(rank["after_is_correct"], false);
    assert_eq!(
        rank["relations"],
        serde_json::json!([
            { "statement": {"track": 1, "value": 127, "occurrence": 0},
              "before_note": "t1:c0:p60:s0:n0", "after_note": "t1:c0:p60:s0:n0",
              "before_state_first": false, "after_state_first": true },
            { "statement": {"track": 1, "value": 127, "occurrence": 0},
              "before_note": "t1:c0:p64:s240:n0", "after_note": "t1:c0:p64:s240:n0",
              "before_state_first": true, "after_state_first": false }
        ])
    );
    let human = diff(&before, &after);
    assert!(human.contains("release t1:c0:p60:s0:n0: state after note -> state before note"));
    assert!(human.contains("release t1:c0:p64:s240:n0: state before note -> state after note"));
    assert_eq!(diff(&before, &before), "no differences\n");
}

/// Rank uses the correspondence selected by diff's existing tolerance. #33.
#[test]
fn rank_reuses_note_matching_and_does_not_match_unrelated_statements() {
    let dir = tempfile::tempdir().unwrap();
    let before = common::build_take_setting(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[
            common::program_change(0, 40),
            common::strike(0, 60),
            common::release(480, 60),
        ],
        &[],
    );
    let build = |name: &str, value| {
        common::build_take_setting(
            &dir.path().join(name),
            480,
            &[(0, 4, 4)],
            &[
                common::strike(0, 64),
                common::program_change(0, value),
                common::release(480, 64),
            ],
            &[],
        )
    };
    let after = build("after.mid", 40);
    let changed_state = build("changed-state.mid", 41);
    let read = |path: &std::path::Path| battuta::Take::read(path).unwrap();
    let exact = battuta::diff::diff(&read(&before), &read(&after), Some(0)).unwrap();
    assert!(exact.rank_disagreements.is_empty());
    assert_eq!(exact.added.len(), 1);
    assert_eq!(exact.removed.len(), 1);
    let inferred = battuta::diff::diff(&read(&before), &read(&after), None).unwrap();
    assert_eq!(inferred.rank_disagreements.len(), 1);
    let relation = &inferred.rank_disagreements[0].relations[0];
    assert_eq!(relation.before_note.to_string(), "t1:c0:p60:s0:n0");
    assert_eq!(relation.after_note.to_string(), "t1:c0:p64:s0:n0");
    let human = diff(&before, &after);
    assert!(human.contains("t1:c0:p60:s0:n0 -> t1:c0:p64:s0:n0"));
    let unmatched = battuta::diff::diff(&read(&before), &read(&changed_state), None).unwrap();
    assert!(unmatched.rank_disagreements.is_empty());
    assert!(!unmatched.programs.is_empty());
}
