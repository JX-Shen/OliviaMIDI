//! What two tracks leave unordered — #26.
//!
//! A Rank orders events within one track (ADR-0008). A channel's state written
//! on one track and a strike of that channel on another have no order the file
//! states: a Type-1 player merges the tracks and the format does not say in
//! which order. So a Take can pass through `apply`, report the state the Edit
//! asked for, and sound under the one it had — measured at -16.3 dBFS when this
//! was found, and invisible to `inspect` and `diff`, which compare state and
//! see no difference at all.
//!
//! `apart()` is the Take these use: three tracks, of which track 1 carries C4
//! and E4 on channel 0 and track 2 carries that channel's programme, which is
//! where a DAW export puts it when somebody keeps a patch-change track.

mod common;

use common::{build_take_stating_apart, edit_set, mid, program_change};
use std::path::{Path, PathBuf};

/// Two Bars of 4/4 at 960 ticks to the quarter: C4 through Bar 1, E4 through
/// Bar 2, both on channel 0 of track 1, with track 2 stating the channel's
/// programme at whichever Ticks a test asks for.
fn apart(dir: &Path, setting: &[(u32, u8)]) -> PathBuf {
    let setting: Vec<_> = setting
        .iter()
        .map(|&(tick, program)| program_change(tick, program))
        .collect();
    apart_with(dir, &setting)
}

/// The same Take, with whatever the setting track states written out in full.
/// `apart` is this with the events spelled as programmes, which is every case
/// #26 needed and none of the ones a tempo or a bend needs.
fn apart_with(dir: &Path, setting: &[(u32, midly::TrackEventKind<'static>)]) -> PathBuf {
    build_take_stating_apart(
        &dir.join("apart.mid"),
        960,
        &[(0, 4, 4)],
        setting,
        &[(0, 1920, 60), (1920, 1920, 64)],
    )
}

/// Run an Edit Set against a Take, with whatever sites the run is answering
/// for, and hand back what happened.
fn applied(take: &Path, edits: &str, allowing: &[&str]) -> (bool, String, PathBuf) {
    let dir = take.parent().expect("a take in a directory");
    let out = dir.join(format!("out-{}.mid", edits.len()));
    let mut command = mid();
    command
        .arg("apply")
        .arg(take)
        .arg(edit_set(dir, "edits", edits));
    for site in allowing {
        command.arg("--allow-unranked").arg(site);
    }
    let outcome = command.arg("-o").arg(&out).output().expect("mid runs");
    (
        outcome.status.success(),
        String::from_utf8_lossy(&outcome.stderr).into_owned(),
        out,
    )
}

/// The reproduction, and the refusal it now gets: `apply` exited 0, `inspect`
/// reported the new Program, and the notes sounded on the old one.
#[test]
fn a_state_stated_where_another_track_strikes_that_channel_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(0, 40)]);
    let (ok, stderr, out) = applied(
        &take,
        r#"{ "kind": "set_program", "track": 2, "channel": 0, "tick": 1920, "program": 56 }"#,
        &[],
    );

    assert!(!ok, "the Edit Set was applied");
    assert!(!out.exists(), "a refused Edit Set left a Take behind");
    // Both tracks, because knowing only one of them does not tell you what to
    // do, and both remedies, because which is right is the author's to choose.
    assert!(stderr.contains("track 2"), "{stderr}");
    assert!(stderr.contains("track 1"), "{stderr}");
    assert!(stderr.contains("tick 1920"), "{stderr}");
    assert!(stderr.contains("carries those notes"), "{stderr}");
    assert!(stderr.contains("strikes nothing"), "{stderr}");
}

/// The remedy the refusal names works, and the notes at that Tick end up under
/// the state: one track, so the file states the order.
#[test]
fn the_same_edit_on_the_track_that_carries_the_notes_succeeds() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(0, 40)]);
    let (ok, stderr, out) = applied(
        &take,
        r#"{ "kind": "set_program", "track": 1, "channel": 0, "tick": 1920, "program": 56 }"#,
        &[],
    );

    assert!(ok, "{stderr}");
    let events = common::events_of_track(&out, 1);
    let strike = events
        .iter()
        .position(|(tick, kind)| *tick == 1920 && *kind == "strikes")
        .expect("E4 is struck");
    // The programme is on the notes' track now, so the file states the order,
    // and the placement rule put it in front of the note it governs.
    // The Take's own statement at Tick 0 stays where the author put it; the
    // Edit's lands on the notes' track, which is what the refusal asked for.
    assert_eq!(
        common::program_changes(&out),
        vec![(1920, 56), (0, 40)],
        "the programme was not stated on the notes' track"
    );
    assert!(strike > 0, "nothing precedes the strike it governs");
}

/// The other remedy: a Tick where that channel strikes nothing, which is the
/// habit of putting the setup ahead of the music.
#[test]
fn a_tick_no_other_track_strikes_that_channel_at_is_not_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(0, 40)]);
    let (ok, stderr, _) = applied(
        &take,
        r#"{ "kind": "set_program", "track": 2, "channel": 0, "tick": 960, "program": 56 }"#,
        &[],
    );
    assert!(ok, "{stderr}");
}

/// A strike of a different channel decides nothing about this one, and refusing
/// on it would fire on every Take with more than one part.
#[test]
fn a_strike_of_another_channel_is_not_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(0, 40)]);
    let (ok, stderr, _) = applied(
        &take,
        r#"{ "kind": "set_program", "track": 2, "channel": 3, "tick": 1920, "program": 56 }"#,
        &[],
    );
    assert!(ok, "{stderr}");
}

/// Two tracks stating one channel two Programs at one Tick: the file does not
/// say which the channel ends up on.
#[test]
fn two_tracks_stating_one_channel_different_values_at_one_tick_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    // Tick 960 so that no note is struck there: this is about the two
    // statements, not about the notes.
    let take = apart(dir.path(), &[(960, 40)]);
    let (ok, stderr, out) = applied(
        &take,
        r#"{ "kind": "set_program", "track": 1, "channel": 0, "tick": 960, "program": 56 }"#,
        &[],
    );

    assert!(!ok, "the Edit Set was applied");
    assert!(!out.exists());
    assert!(stderr.contains("56"), "{stderr}");
    assert!(stderr.contains("40"), "{stderr}");
}

/// The same shape with one value: both orders leave the channel where the Take
/// says, so the order the file does not state decides nothing, and nothing is
/// said about it.
#[test]
fn two_tracks_stating_one_channel_the_same_value_is_not_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(960, 40)]);
    let (ok, stderr, _) = applied(
        &take,
        r#"{ "kind": "set_program", "track": 1, "channel": 0, "tick": 960, "program": 40 }"#,
        &[],
    );
    assert!(ok, "{stderr}");
}

/// The same condition from the notes' end. An `add_note` landing where another
/// track states its channel creates exactly the ambiguity a `set_program`
/// would, and which of the two an Edit Set moved is not what makes it one.
#[test]
fn a_note_added_where_another_track_states_its_channel_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(960, 40)]);
    let (ok, stderr, out) = applied(
        &take,
        r#"{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 67,
             "start": 960, "duration": 480, "velocity": 50 }"#,
        &[],
    );

    assert!(!ok, "the Edit Set was applied");
    assert!(!out.exists());
    // Addressed from the state event, which is the site somebody can answer
    // for, whichever end the Edit Set moved.
    assert!(stderr.contains("--allow-unranked t2:c0:s960"), "{stderr}");
}

// Note-side Bend safety — #42.
#[test]
fn a_note_added_or_moved_onto_another_tracks_bend_is_refused() {
    for edits in [
        r#"{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 67,
             "start": 960, "duration": 480, "velocity": 50 }"#,
        r#"{ "kind": "move_note", "id": "t1:c0:p64:s1920:n0", "delta_ticks": -960 }"#,
    ] {
        let dir = tempfile::tempdir().expect("temp dir");
        let take = apart_with(dir.path(), &[common::pitch_bend(960, 4096)]);
        let original = std::fs::read(&take).unwrap();
        let (ok, stderr, out) = applied(&take, edits, &[]);
        assert!(!ok, "a note landed on another track's Bend: {edits}");
        assert!(!out.exists(), "a refused Edit Set left a Take behind");
        assert_eq!(std::fs::read(&take).unwrap(), original);
        assert!(stderr.contains("bend"), "{stderr}");
        assert!(
            stderr.contains("track 1") && stderr.contains("track 2"),
            "{stderr}"
        );
        assert!(stderr.contains("--allow-unranked t2:c0:s960"), "{stderr}");
        assert!(!stderr.contains("this Edit Set put a bend"), "{stderr}");
    }
}

#[test]
fn a_note_side_bend_allowance_names_exactly_the_statement_site() {
    for edits in [
        r#"{ "kind": "add_note", "track": 1, "channel": 0, "pitch": 67,
             "start": 960, "duration": 480, "velocity": 50 }"#,
        r#"{ "kind": "move_note", "id": "t1:c0:p64:s1920:n0", "delta_ticks": -960 }"#,
    ] {
        let dir = tempfile::tempdir().expect("temp dir");
        let take = apart_with(dir.path(), &[common::pitch_bend(960, 4096)]);
        for wrong in ["t1:c0:s960", "t2:c1:s960", "t2:c0:s1920"] {
            let (ok, stderr, out) = applied(&take, edits, &[wrong]);
            assert!(!ok, "{wrong} covered the Bend statement: {stderr}");
            assert!(!out.exists());
        }
        let (ok, stderr, out) = applied(&take, edits, &["t2:c0:s960"]);
        assert!(ok, "{stderr}");
        let written = battuta::Take::read(&out).unwrap();
        assert_eq!(
            written.stated_bends().unwrap(),
            battuta::Take::read(&take).unwrap().stated_bends().unwrap()
        );
        assert!(written.unranked(None).unwrap().iter().any(|site| {
            site.state == battuta::State::Bend
                && site.track == 2
                && site.against_track == 1
                && site.tick == 960
                && site.channel == Some(0)
                && site.against == battuta::Against::Notes
        }));
    }
}

#[test]
fn a_bend_does_not_refuse_a_note_on_its_own_track_or_another_channel_or_tick() {
    for (track, channel, start) in [(2, 0, 960), (1, 1, 960), (1, 0, 961)] {
        let dir = tempfile::tempdir().expect("temp dir");
        let take = apart_with(dir.path(), &[common::pitch_bend(960, 4096)]);
        let edits = format!(
            r#"{{ "kind": "add_note", "track": {track}, "channel": {channel},
                  "pitch": 67, "start": {start}, "duration": 480, "velocity": 50 }}"#
        );
        let (ok, stderr, out) = applied(&take, &edits, &[]);
        assert!(ok, "{stderr}");
        let written = battuta::Take::read(&out).unwrap();
        assert!(written.unranked(None).unwrap().is_empty());
        if track == 2 {
            let bytes = std::fs::read(&out).unwrap();
            let smf = midly::Smf::parse(&bytes).unwrap();
            let events = &smf.tracks[2];
            let bend = events
                .iter()
                .position(|event| {
                    matches!(
                        event.kind,
                        midly::TrackEventKind::Midi {
                            message: midly::MidiMessage::PitchBend { .. },
                            ..
                        }
                    )
                })
                .unwrap();
            let strike = events
                .iter()
                .position(|event| {
                    matches!(
                        event.kind,
                        midly::TrackEventKind::Midi {
                            message: midly::MidiMessage::NoteOn { .. },
                            ..
                        }
                    )
                })
                .unwrap();
            assert!(bend < strike, "the note must follow the carried Bend");
        }
    }
}

#[test]
fn a_carried_bend_site_survives_an_empty_edit_or_an_edit_elsewhere() {
    for edits in [
        "",
        r#"{ "kind": "set_velocity", "id": "t1:c0:p64:s1920:n0", "velocity": 40 }"#,
    ] {
        let dir = tempfile::tempdir().expect("temp dir");
        let take = apart_with(dir.path(), &[common::pitch_bend(0, -4096)]);
        let before = battuta::Take::read(&take).unwrap();
        let (ok, stderr, out) = applied(&take, edits, &[]);
        assert!(ok, "{stderr}");
        let after = battuta::Take::read(&out).unwrap();
        assert_eq!(
            after.unranked(None).unwrap(),
            before.unranked(None).unwrap()
        );
        assert_eq!(
            after.stated_bends().unwrap(),
            before.stated_bends().unwrap()
        );
        if edits.is_empty() {
            assert_eq!(common::event_stream(&out), common::event_stream(&take));
        }
    }
}

/// What the Take carried in is the author's (ADR-0003). An Edit Set that
/// touches none of it is not answerable for it, and refusing here would mean a
/// Take with one such Tick could not be edited anywhere at all.
#[test]
fn an_ambiguity_the_take_arrived_with_does_not_refuse_an_edit_elsewhere() {
    let dir = tempfile::tempdir().expect("temp dir");
    // The Take already states the programme at Tick 0, where track 1 strikes C4.
    let take = apart(dir.path(), &[(0, 40)]);
    let (ok, stderr, _) = applied(
        &take,
        r#"{ "kind": "set_velocity", "id": "t1:c0:p64:s1920:n0", "velocity": 40 }"#,
        &[],
    );
    assert!(ok, "{stderr}");
}

/// Naming the site is how somebody takes responsibility for it — they know the
/// player, or the Take is a test. Per site, and there is no way to turn the
/// check off for a run.
#[test]
fn a_named_site_is_let_through() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(0, 40)]);
    let (ok, stderr, out) = applied(
        &take,
        r#"{ "kind": "set_program", "track": 2, "channel": 0, "tick": 1920, "program": 56 }"#,
        &["t2:c0:s1920"],
    );
    assert!(ok, "{stderr}");
    assert!(out.exists());
}

/// A site names one place. Naming another one does not cover this one.
#[test]
fn a_site_named_elsewhere_does_not_cover_this_one() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(0, 40)]);
    let (ok, _, _) = applied(
        &take,
        r#"{ "kind": "set_program", "track": 2, "channel": 0, "tick": 1920, "program": 56 }"#,
        &["t2:c0:s960"],
    );
    assert!(!ok, "a site named elsewhere let this one through");
}

/// The grammar is a note identity's, and a site that is not one is refused
/// rather than read as far as it parses.
#[test]
fn a_malformed_site_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(0, 40)]);
    for site in ["2:0:1920", "t2:c0", "t2:s1920:c0", "track2:c0:s1920"] {
        let (ok, stderr, _) = applied(
            &take,
            r#"{ "kind": "set_program", "track": 1, "channel": 0, "tick": 960, "program": 56 }"#,
            &[site],
        );
        assert!(!ok, "{site} was read as a site");
        assert!(stderr.contains(site), "{stderr}");
    }
}

/// The third outcome: what the Take arrived with is reported, not refused.
///
/// `inspect` is the tool somebody opens a Take *in order to* diagnose it, so a
/// reader that declined to read an imperfect file would withhold the answer
/// exactly where it is wanted.
#[test]
fn inspect_reports_what_the_take_leaves_unordered() {
    let dir = tempfile::tempdir().expect("temp dir");
    // The programme is on track 2 at Tick 0, where track 1 strikes C4 — the
    // shape a DAW export reaches when somebody keeps a patch-change track.
    let take = apart(dir.path(), &[(0, 40)]);
    let out = mid().arg("inspect").arg(&take).output().expect("mid runs");

    assert!(out.status.success());
    let human = String::from_utf8_lossy(&out.stdout);
    assert!(human.contains("no order the file states"), "{human}");
    assert!(human.contains("track 2"), "{human}");
    assert!(human.contains("track 1"), "{human}");
    // Spelled the way --allow-unranked takes it, so it can be copied out.
    assert!(human.contains("t2:c0:s0"), "{human}");
}

/// The same fact in the payload, and the four lists that were there before it
/// are still where a consumer left them.
#[test]
fn the_payload_carries_it_without_moving_anything_else() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(0, 40)]);
    let json: serde_json::Value =
        serde_json::from_str(&common::inspect_json(&take)).expect("inspect emits JSON");

    for existing in [
        "programs",
        "stated_programs",
        "controllers",
        "stated_controllers",
        "notes",
    ] {
        assert!(json.get(existing).is_some(), "{existing} left the payload");
    }
    assert_eq!(
        json["unranked"],
        serde_json::json!([{
            "tick": 0,
            "channel": 0,
            "controller": null,
            "track": 2,
            "against_track": 1,
            "against": "notes",
            "state": "program"
        }])
    );
}

/// A Take that leaves nothing open says nothing. A warning printed when there
/// is no warning is one a reader learns to skip.
#[test]
fn a_take_that_leaves_nothing_open_says_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    // Tick 960: the setup ahead of the music, which is the older of the two
    // habits this ambiguity has always been avoided by.
    let take = apart(dir.path(), &[(960, 40)]);
    let out = mid().arg("inspect").arg(&take).output().expect("mid runs");
    let human = String::from_utf8_lossy(&out.stdout);
    assert!(!human.contains("no order the file states"), "{human}");
}

/// Two tracks stating one channel the same value leave an order that decides
/// nothing, so there is nothing to report.
#[test]
fn agreeing_statements_on_two_tracks_are_not_reported() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart(dir.path(), &[(960, 40)]);
    let (ok, stderr, out) = applied(
        &take,
        r#"{ "kind": "set_program", "track": 1, "channel": 0, "tick": 960, "program": 40 }"#,
        &[],
    );
    assert!(ok, "{stderr}");

    let json: serde_json::Value =
        serde_json::from_str(&common::inspect_json(&out)).expect("inspect emits JSON");
    assert_eq!(json["unranked"], serde_json::json!([]));
}

// ---------------------------------------------------------------------------
// Tempo and bend join the reading — #42.
//
// A bend is a channel's state (#44), so it is the case above unchanged. A tempo
// has none: it governs the whole Take, which is why SMF Format 1 puts it on a
// conductor track, and two tracks stating one at one Tick is one conductor
// given two contradictory gestures rather than two players disagreeing.
// ---------------------------------------------------------------------------

/// Three tracks where both the voice and the setting track bend one channel at
/// one Tick. `apart()` cannot make this: it puts every setting on track 2, and
/// what is wanted here is a statement on each of two tracks.
fn both_bending(dir: &Path, first: i16, second: i16) -> PathBuf {
    stating_on_both(
        dir,
        common::pitch_bend(0, first),
        common::pitch_bend(0, second),
    )
}

/// The same Take with an arbitrary statement on each of the two tracks: `voice`
/// carries the notes and the first, `setting` carries the second.
fn stating_on_both(
    dir: &Path,
    on_voice: (u32, midly::TrackEventKind<'static>),
    on_setting: (u32, midly::TrackEventKind<'static>),
) -> PathBuf {
    use midly::num::{u15, u24, u28, u4, u7};
    use midly::{
        Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
    };

    fn deltas(mut events: Vec<(u32, TrackEventKind<'static>)>) -> Vec<TrackEvent<'static>> {
        events.sort_by_key(|(tick, _)| *tick);
        let mut previous = 0u32;
        events
            .into_iter()
            .map(|(tick, kind)| {
                let event = TrackEvent {
                    delta: u28::new(tick - previous),
                    kind,
                };
                previous = tick;
                event
            })
            .collect()
    }

    let conductor = vec![
        (
            0,
            TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
        ),
        (
            0,
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000))),
        ),
        (0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let voice = vec![
        on_voice,
        (
            0,
            TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOn {
                    key: u7::new(60),
                    vel: u7::new(64),
                },
            },
        ),
        (
            1920,
            TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOff {
                    key: u7::new(60),
                    vel: u7::new(0),
                },
            },
        ),
        (1920, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let setting = vec![
        on_setting,
        (0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];

    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(960))),
        tracks: vec![deltas(conductor), deltas(voice), deltas(setting)],
    };
    let path = dir.join("both-bending.mid");
    smf.save(&path).expect("a take is written");
    path
}

/// A bend on the setting track where the voice track strikes that channel at
/// the same Tick. The Program case verbatim, because a bend is held by the
/// channel exactly as a Program is (#44) — the note sounds bent or unbent
/// depending on which track the player merges first.
#[test]
fn a_bend_stated_where_another_track_strikes_that_channel_is_unranked() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart_with(dir.path(), &[common::pitch_bend(0, 4096)]);
    let json: serde_json::Value =
        serde_json::from_str(&common::inspect_json(&take)).expect("inspect emits JSON");

    assert_eq!(
        json["unranked"],
        serde_json::json!([{
            "tick": 0,
            "channel": 0,
            "controller": null,
            "track": 2,
            "against_track": 1,
            "against": "notes",
            "state": "bend"
        }])
    );
}

/// Two tracks bending one channel differently at one Tick. `Against::Value`,
/// which the reading has had since #26 and which a bend reaches without a new
/// code path once it is collected at all.
#[test]
fn two_tracks_bending_one_channel_differently_at_one_tick_are_unranked() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = both_bending(dir.path(), 4096, -4096);
    let json: serde_json::Value =
        serde_json::from_str(&common::inspect_json(&take)).expect("inspect emits JSON");

    let rows = json["unranked"].as_array().expect("an array").clone();
    assert!(
        rows.iter().any(|row| {
            row["state"] == "bend" && row["against"] == "value" && row["channel"] == 0
        }),
        "{rows:?}"
    );
}

/// Two tracks bending one channel the *same* way decide nothing, so nothing is
/// reported — the rule the entry states for a Program, holding for a bend.
#[test]
fn two_tracks_bending_one_channel_the_same_way_are_not_reported() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = both_bending(dir.path(), 4096, 4096);
    let json: serde_json::Value =
        serde_json::from_str(&common::inspect_json(&take)).expect("inspect emits JSON");

    let rows = json["unranked"].as_array().expect("an array");
    assert!(
        !rows.iter().any(|row| row["against"] == "value"),
        "{rows:?}"
    );
}

/// A tempo on the setting track against the conductor's own at Tick 0. No
/// channel, because a tempo has none: it is reported with `channel` null, which
/// is the same spelling `controller` has always used for a state that has no
/// such slot.
#[test]
fn two_tracks_stating_a_tempo_at_one_tick_are_unranked_with_no_channel() {
    let dir = tempfile::tempdir().expect("temp dir");
    // The conductor states 500_000 at Tick 0; track 2 contradicts it.
    let take = apart_with(dir.path(), &[common::tempo(0, 400_000)]);
    let json: serde_json::Value =
        serde_json::from_str(&common::inspect_json(&take)).expect("inspect emits JSON");

    assert_eq!(
        json["unranked"],
        serde_json::json!([{
            "tick": 0,
            "channel": null,
            "controller": null,
            "track": 0,
            "against_track": 2,
            "against": "value",
            "state": "tempo"
        }, {
            "tick": 0,
            "channel": null,
            "controller": null,
            "track": 2,
            "against_track": 0,
            "against": "value",
            "state": "tempo"
        }])
    );
}

/// The same tempo twice decides nothing.
#[test]
fn two_tracks_stating_the_same_tempo_at_one_tick_are_not_reported() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart_with(dir.path(), &[common::tempo(0, 500_000)]);
    let json: serde_json::Value =
        serde_json::from_str(&common::inspect_json(&take)).expect("inspect emits JSON");

    assert_eq!(json["unranked"], serde_json::json!([]));
}

/// A tempo row names no site. `--allow-unranked` takes a track, a channel and a
/// tick, and no Edit in the contract writes a tempo — so there is nothing to
/// answer for, and a line offering a site the flag would refuse would be
/// advertising a grammar that does not exist.
#[test]
fn a_tempo_row_offers_no_site_to_copy() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = apart_with(dir.path(), &[common::tempo(0, 400_000)]);
    let out = mid().arg("inspect").arg(&take).output().expect("mid runs");
    let human = String::from_utf8_lossy(&out.stdout);

    let rows: Vec<&str> = human
        .lines()
        .filter(|line| line.contains("no order the file states"))
        .collect();
    assert_eq!(rows.len(), 2, "{human}");
    for row in rows {
        assert!(row.contains("the tempo"), "{row}");
        assert!(
            !row.contains("for channel"),
            "a tempo row named a channel: {row}"
        );
        assert!(!row.contains("(t"), "a tempo row offered a site: {row}");
    }
}

/// A program change and a bend both have no CC number, so `controller: null`
/// cannot tell them apart. Without the state being part of the address, two
/// tracks stating one and the other at one Tick would be compared as one
/// address and reported as a disagreement that is not one.
#[test]
fn a_program_on_one_track_and_a_bend_on_another_are_not_one_address() {
    let dir = tempfile::tempdir().expect("temp dir");
    // The bend rides with the notes on track 1, the programme sits on track 2.
    // Both have no CC number and their values differ, so without the state
    // being part of the address they would be read as one address disagreeing.
    let take = stating_on_both(
        dir.path(),
        common::pitch_bend(0, 4096),
        program_change(0, 40),
    );
    let json: serde_json::Value =
        serde_json::from_str(&common::inspect_json(&take)).expect("inspect emits JSON");

    let rows = json["unranked"].as_array().expect("an array");
    assert!(
        !rows.iter().any(|row| row["against"] == "value"),
        "a program and a bend were read as one address: {rows:?}"
    );
}
