//! Orchestration: which Program a channel is on — #12.
//!
//! `mid` carried program changes faithfully long before it mentioned one. It
//! inherited them into a passage (#4) and never dropped one from an `apply`
//! (ADR-0003), and said nothing about any of it. Preserved, invisible,
//! untouchable — and the middle one is the dangerous one, because a human who
//! cannot see that a part is on a horn will diagnose it as a badly written line.
//! These tests are about the seeing and the changing.
//!
//! `ORCHESTRATED` is the Take they mostly use: four Bars of 3/4, channel 0 on
//! program 40 from Tick 0, channel 1 switching to program 60 at Bar 3, and
//! channel 2 stating nothing while playing throughout.

mod common;

use common::{FIXTURE, ORCHESTRATED};

/// What each channel of the passage is on, above the notes and before them: the
/// state reframes every note under it, because the same notes on a horn are a
/// different passage.
///
/// Asserted whole. Which facts share a line, and that the note table below is
/// untouched by the block above it, is the point.
#[test]
fn states_which_program_each_channel_of_the_passage_is_on() {
    assert_eq!(
        common::human_output(&["inspect", ORCHESTRATED, "--bars", "1:2"]),
        "\
channel 0  program 40 (GM violin)
channel 1  unstated
channel 2  unstated

bar 1 beat 1  track 1  A4  velocity 70  duration 480  t1:c0:p69:s0:n0
bar 1 beat 1  track 2  D3  velocity 60  duration 480  t2:c1:p50:s0:n0
bar 1 beat 1  track 3  D2  velocity 55  duration 480  t3:c2:p38:s0:n0
bar 2 beat 1  track 1  A4  velocity 70  duration 480  t1:c0:p69:s1440:n0
bar 2 beat 1  track 2  D3  velocity 60  duration 480  t2:c1:p50:s1440:n0
bar 2 beat 1  track 3  D2  velocity 55  duration 480  t3:c2:p38:s1440:n0
"
    );
}

/// A Program set before the passage began is still what the passage is on.
///
/// Bar 4 states nothing. Channel 0 was set two Bars earlier and channel 1 one
/// Bar earlier, and both are in force here — which is the whole reason the state
/// is reported separately from the events. A passage listing only what happens
/// inside it would describe Bar 4 as having no instrument at all.
#[test]
fn reports_a_program_set_before_the_passage_began() {
    let listed = common::human_output(&["inspect", ORCHESTRATED, "--bars", "4:4"]);
    let (state, _) = listed.split_once("\n\n").expect("a block, then the notes");
    assert_eq!(
        state,
        "\
channel 0  program 40 (GM violin)
channel 1  program 60 (GM french horn)
channel 2  unstated"
    );
}

/// A switch inside the passage is an event, with a Position, and is not folded
/// into the state.
///
/// Bars 2 to 4 open with channel 1 on nothing and change at Bar 3. Reporting
/// only the opening state would hide a change a listener hears; reporting only
/// the event would say the passage had no instrument until Bar 3.
#[test]
fn says_where_the_passage_states_another_program() {
    assert_eq!(
        common::human_output(&["inspect", ORCHESTRATED, "--bars", "2:4"])
            .lines()
            .take(5)
            .collect::<Vec<_>>()
            .join("\n"),
        "\
channel 0  program 40 (GM violin)
channel 1  unstated
channel 2  unstated

bar 3 beat 1  track 2  channel 1  program 60 (GM french horn)"
    );
}

/// A Program stated at the passage's own first Tick is state, not event.
///
/// Bar 3 is where channel 1 switches, so `--bars 3:4` opens on program 60. It is
/// in force from the first Tick of the passage, which is what a listener hears
/// and what `play` hands the synthesiser — so it is said once, as the state, and
/// not again as something that happens during the passage.
#[test]
fn a_program_stated_at_the_first_tick_is_the_state_it_begins_in() {
    let listed = common::human_output(&["inspect", ORCHESTRATED, "--bars", "3:4"]);
    let (state, notes) = listed.split_once("\n\n").expect("a block, then the notes");
    assert_eq!(
        state,
        "\
channel 0  program 40 (GM violin)
channel 1  program 60 (GM french horn)
channel 2  unstated"
    );
    assert!(
        !notes.contains("channel 1  program 60"),
        "said once, as a state, not twice: {notes}"
    );
}

/// `inspect` and the synthesiser agree about what the passage is on.
///
/// The strongest form of the criterion, and the reason it is worth a fake
/// FluidSynth: `play --bars` hands over a temporary Take carrying the state the
/// passage inherited, and what `inspect` prints has to be a description of *that
/// file*. Two readings of one passage that disagreed would make the listing
/// worthless for checking anything.
#[test]
fn agrees_with_what_play_hands_the_synthesiser() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = common::fake_fluidsynth(dir.path());
    let soundfont = dir.path().join("rig.sf2");
    common::write(&soundfont, "not really a soundfont");

    common::mid()
        .env("PATH", &fake.dir)
        .env(battuta::rig::SOUNDFONT_ENV, &soundfont)
        .args(["play", ORCHESTRATED, "--bars", "4:4"])
        .assert()
        .success();

    // The passage gathers its inherited state at Tick 0, in the order it was
    // set. Both channels' Programs travel, and they are the two `inspect` names.
    let handed: Vec<(u8, u8)> = common::stated_programs(&fake.handed)
        .into_iter()
        .map(|(_, tick, channel, program)| {
            assert_eq!(
                tick, 0,
                "inherited state is gathered at the passage's start"
            );
            (channel, program)
        })
        .collect();
    assert_eq!(handed, vec![(0, 40), (1, 60)]);
}

/// A Take that states no Program is described as stating none — never as being
/// on program 0.
///
/// The one criterion that fails silently. General MIDI's default is program 0,
/// so a Take relying on it sounds identical to one that says 0; `FIXTURE` is
/// exactly that Take, which is why it renders as a piano on a General MIDI bank.
/// A tool that printed 0 here would be describing a Piece the file does not
/// state, and every audition would agree with it.
#[test]
fn a_take_that_states_no_program_is_described_as_stating_none() {
    let listed = common::human_output(&["inspect", FIXTURE, "--bars", "1:1"]);
    assert!(
        listed.starts_with("no programs stated\n\n"),
        "said once, plainly: {listed}"
    );

    let (programs, stated) = common::programs(&common::inspect_json(std::path::Path::new(FIXTURE)));
    assert!(stated.is_empty());
    for state in &programs {
        assert!(
            state["program"].is_null(),
            "stating none is null, never 0: {state}"
        );
    }
    // Both channels the Take plays on are listed. A channel with notes and no
    // Program is the fact a reader most needs: those notes will sound on
    // whatever the bank defaults to.
    assert_eq!(programs.len(), 2);
}

/// The payload an agent consumes: the state per channel, and the events, as
/// numbers.
///
/// No General MIDI name in it, for the reason #7 kept pitch names out: a name is
/// a gloss for a human reading a terminal, and an agent is entitled to one
/// spelling of a fact. The number is the fact.
#[test]
fn json_carries_the_orchestration_as_numbers() {
    let json = common::inspect_json(std::path::Path::new(ORCHESTRATED));
    let (programs, stated) = common::programs(&json);

    assert_eq!(
        programs
            .iter()
            .map(|state| (state["channel"].as_u64(), state["program"].as_u64()))
            .collect::<Vec<_>>(),
        vec![(Some(0), Some(40)), (Some(1), None), (Some(2), None)]
    );
    assert_eq!(stated.len(), 1);
    assert_eq!(stated[0]["track"].as_u64(), Some(2));
    assert_eq!(stated[0]["channel"].as_u64(), Some(1));
    assert_eq!(stated[0]["tick"].as_u64(), Some(2880));
    assert_eq!(stated[0]["program"].as_u64(), Some(60));

    assert!(
        !json.to_lowercase().contains("violin"),
        "no gloss in the payload: {json}"
    );
}

/// The General MIDI name is labelled as General MIDI's, and is absent on the
/// drum channel.
///
/// Both are the same decision. Which instrument a program number *sounds* like
/// depends on the bank, and the bank is the Rig — so the name says whose word it
/// is. On channel 9 the number selects a kit rather than an instrument, so the
/// melodic list is not a list of what it is, and glossing it would be a wrong
/// answer rather than an unlabelled one.
#[test]
fn labels_the_general_midi_name_and_omits_it_on_the_drum_channel() {
    use midly::num::{u4, u7};
    let dir = tempfile::tempdir().expect("temp dir");
    let drums = |channel: u8| {
        (
            0u32,
            midly::TrackEventKind::Midi {
                channel: u4::new(channel),
                message: midly::MidiMessage::ProgramChange {
                    program: u7::new(40),
                },
            },
        )
    };
    let take = common::build_take_setting(
        &dir.path().join("kit.mid"),
        480,
        &[(0, 4, 4)],
        &[drums(9), drums(0)],
        &[(0, 240, 60)],
    );
    let listed = common::human_output(&["inspect", take.to_str().expect("a path")]);
    let (state, _) = listed.split_once("\n\n").expect("a block, then the notes");
    assert_eq!(
        state,
        "\
channel 0  program 40 (GM violin)
channel 9  program 40"
    );
}

/// One Edit kind changes which Program a channel is on, where the Take already
/// states one.
#[test]
fn one_edit_kind_changes_which_program_a_channel_is_on() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = dir.path().join("edits.json");
    common::write(
        &edits,
        r#"{"edits": [{"kind": "set_program", "track": 1, "channel": 0, "tick": 0, "program": 60}]}"#,
    );

    common::mid()
        .args(["apply", ORCHESTRATED])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    // The statement that was there is the statement that changed: one program
    // change on channel 0, still at Tick 0, still on track 1.
    assert_eq!(
        common::stated_programs(&output),
        vec![(1, 0, 0, 60), (2, 2880, 1, 60)]
    );
}

/// The same Edit states a Program where the Take stated none.
///
/// The ordinary case, and the reason `set_program` states an address rather than
/// naming an identity: there was nothing there to name. Channel 2 plays
/// throughout `ORCHESTRATED` and the file never says what it plays.
#[test]
fn states_a_program_where_the_take_stated_none() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = dir.path().join("edits.json");
    common::write(
        &edits,
        r#"{"edits": [{"kind": "set_program", "track": 3, "channel": 2, "tick": 2880, "program": 42}]}"#,
    );

    common::mid()
        .args(["apply", ORCHESTRATED])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        common::stated_programs(&output),
        vec![(1, 0, 0, 40), (2, 2880, 1, 60), (3, 2880, 2, 42)]
    );
}

/// An inserted program change precedes the notes at its Tick.
///
/// Audible rather than arithmetic. A note-on at that Tick has to sound on the
/// Program the Take now states there; a statement placed after it would leave
/// the note on the Program before, and the Edit would be inaudible in exactly
/// the Bar it was asked for — an `apply` that succeeded and a file that plays.
#[test]
fn an_inserted_program_change_precedes_the_notes_at_its_tick() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = dir.path().join("edits.json");
    common::write(
        &edits,
        r#"{"edits": [{"kind": "set_program", "track": 3, "channel": 2, "tick": 2880, "program": 42}]}"#,
    );

    common::mid()
        .args(["apply", ORCHESTRATED])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        events_of_track(&output, 3),
        vec![
            (0, "strikes"),
            (480, "releases"),
            (1440, "strikes"),
            (1920, "releases"),
            (2880, "program"),
            (2880, "strikes"),
            (3360, "releases"),
            (4320, "strikes"),
            (4800, "releases"),
        ]
    );
}

/// An Edit stating a Program does not reach the statement after it.
///
/// "From this Tick" is true until the Take says otherwise, and the Take may say
/// otherwise two Bars later. Deleting that would be an Edit changing something
/// it was not asked about, and an Edit stays mechanical. What the passage is on
/// afterwards is `inspect`'s answer to give.
#[test]
fn does_not_reach_a_later_statement() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = dir.path().join("edits.json");
    // Channel 1 is set at Bar 3. This states one for it at Tick 0, two Bars
    // before that, and the switch at Bar 3 still happens.
    common::write(
        &edits,
        r#"{"edits": [{"kind": "set_program", "track": 2, "channel": 1, "tick": 0, "program": 42}]}"#,
    );

    common::mid()
        .args(["apply", ORCHESTRATED])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        common::stated_programs(&output),
        vec![(1, 0, 0, 40), (2, 0, 1, 42), (2, 2880, 1, 60)]
    );
    // And the Take says so: Bars 1-2 on the new Program, Bar 3 onwards on the
    // one that was already there.
    let listed =
        common::human_output(&["inspect", output.to_str().expect("a path"), "--bars", "1:1"]);
    assert!(
        listed.starts_with("channel 0  program 40 (GM violin)\nchannel 1  program 42 (GM cello)"),
        "{listed}"
    );
}

/// An Edit Set that names no Program leaves every program change exactly where
/// it was.
///
/// ADR-0003's round trip, for the events this ticket has just made changeable.
/// The velocity Edit below touches one note-on byte, and the orchestration is
/// not collateral.
#[test]
fn an_edit_set_naming_no_program_leaves_the_orchestration_alone() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = dir.path().join("edits.json");
    common::write(
        &edits,
        r#"{"edits": [{"kind": "set_velocity", "id": "t1:c0:p69:s0:n0", "velocity": 99}]}"#,
    );

    common::mid()
        .args(["apply", ORCHESTRATED])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        common::stated_programs(&output),
        common::stated_programs(std::path::Path::new(ORCHESTRATED))
    );
}

/// `diff` reports a Program difference, as a state and with a Position.
///
/// The row opens with `program` rather than `changed`: a changed note is a note
/// that stayed and differs, and this is not about a note at all.
#[test]
fn diff_reports_a_program_change_as_a_difference() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = dir.path().join("out.mid");
    let edits = dir.path().join("edits.json");
    common::write(
        &edits,
        r#"{"edits": [{"kind": "set_program", "track": 1, "channel": 0, "tick": 0, "program": 60}]}"#,
    );
    common::mid()
        .args(["apply", ORCHESTRATED])
        .arg(&edits)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    assert_eq!(
        common::human_output(&["diff", ORCHESTRATED, output.to_str().expect("a path")]),
        "program  bar 1 beat 1  channel 0  40 (GM violin) -> 60 (GM french horn)\n"
    );
}

/// A Take compared with itself has no differences, orchestration included.
///
/// The other half of "never as a Rig difference": what a Program sounds like is
/// not in the file, so nothing about the Rig can reach this comparison. What is
/// in the file is compared, and agreeing files agree.
#[test]
fn a_take_does_not_differ_from_itself() {
    assert_eq!(
        common::human_output(&["diff", ORCHESTRATED, ORCHESTRATED]),
        "no differences\n"
    );
}

/// One row per disagreement, not one per moment it goes on being true.
///
/// A Take that switches to a Program the other never states disagrees from that
/// Tick onwards, however many times either file re-states it afterwards. Saying
/// so once is saying it; a row per statement would be the "forty differences"
/// reading #13 exists to avoid, arriving early.
#[test]
fn collapses_a_sustained_disagreement_into_one_row() {
    let dir = tempfile::tempdir().expect("temp dir");
    let before = common::build_take_with_programs(
        &dir.path().join("before.mid"),
        480,
        &[(0, 4, 4)],
        &[],
        &[(0, 240, 60), (1920, 240, 62), (3840, 240, 64)],
    );
    // Two statements of the same Program, one Bar apart. The disagreement with
    // `before` — which states none — starts at the first and never changes.
    let after = common::build_take_with_programs(
        &dir.path().join("after.mid"),
        480,
        &[(0, 4, 4)],
        &[(1920, 60), (3840, 60)],
        &[(0, 240, 60), (1920, 240, 62), (3840, 240, 64)],
    );

    assert_eq!(
        common::human_output(&[
            "diff",
            before.to_str().expect("a path"),
            after.to_str().expect("a path"),
        ]),
        "program  bar 2 beat 1  channel 0  unstated -> 60 (GM french horn)\n"
    );
}

/// A Program outside what MIDI holds is refused, and the number is reported as
/// itself rather than as a parse failure.
#[test]
fn refuses_a_program_outside_midi_range() {
    let dir = tempfile::tempdir().expect("temp dir");
    let edits = dir.path().join("edits.json");
    common::write(
        &edits,
        r#"{"edits": [{"kind": "set_program", "track": 1, "channel": 0, "tick": 0, "program": 128}]}"#,
    );
    common::mid()
        .args(["apply", ORCHESTRATED])
        .arg(&edits)
        .arg("--output")
        .arg(dir.path().join("out.mid"))
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "program 128 is out of range; a MIDI program is 0-127",
        ));
}

/// A track the Take does not have is refused, naming how many it has.
#[test]
fn refuses_a_track_the_take_does_not_have() {
    let dir = tempfile::tempdir().expect("temp dir");
    let edits = dir.path().join("edits.json");
    common::write(
        &edits,
        r#"{"edits": [{"kind": "set_program", "track": 9, "channel": 0, "tick": 0, "program": 40}]}"#,
    );
    common::mid()
        .args(["apply", ORCHESTRATED])
        .arg(&edits)
        .arg("--output")
        .arg(dir.path().join("out.mid"))
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "track 9 is not in the Take; it has 4 tracks, numbered from 0",
        ));
}

/// A Tick before the start of the Take is refused. There is no moment before
/// Tick 0 for a Program to be in force from.
#[test]
fn refuses_a_tick_before_the_take_begins() {
    let dir = tempfile::tempdir().expect("temp dir");
    let edits = dir.path().join("edits.json");
    common::write(
        &edits,
        r#"{"edits": [{"kind": "set_program", "track": 1, "channel": 0, "tick": -1, "program": 40}]}"#,
    );
    common::mid()
        .args(["apply", ORCHESTRATED])
        .arg(&edits)
        .arg("--output")
        .arg(dir.path().join("out.mid"))
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "a Program cannot be stated at tick -1; the first Tick of a Take is 0",
        ));
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
                midly::MidiMessage::ProgramChange { .. } => "program",
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
