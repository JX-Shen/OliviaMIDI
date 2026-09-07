//! Preparing a passage keeps the orders the Take stated, or refuses — #34.
//!
//! Inheriting is a collapse. Everything the Take had already set arrives at the
//! passage's first Tick, from however many Ticks away, and two events the Take
//! ordered by *time* end up sharing an instant. Within one track a Rank still
//! orders them and orders them as they were. Across two tracks nothing does:
//! ADR-0008's rule runs within one track, so the passage would state no order
//! where the Take stated one, and the audition would be one of two things
//! depending on the player.
//!
//! That is the one thing `mid` will not do here. A site the *author* left open
//! is theirs (ADR-0003) and `inspect` reports it rather than refusing it; a site
//! `mid` would create, out of one that was closed, while preparing something
//! somebody is about to listen to and form an opinion from, is refused —
//! *refuse rather than answer plausibly*.

mod common;

use common::{fake_fluidsynth, mid};

/// One track's events in file order, program changes included.
///
/// `common::events_of_track` is silent about a program change, and a program
/// change is the whole of what this file is about: where it falls among the
/// events sharing its Tick is the fact under test.
fn order_of_track(path: &std::path::Path, track: usize) -> Vec<(u32, &'static str)> {
    let bytes = std::fs::read(path).expect("Take is readable");
    let smf = midly::Smf::parse(&bytes).expect("Take parses");
    let mut found = Vec::new();
    let mut tick = 0u32;
    for event in &smf.tracks[track] {
        tick += event.delta.as_int();
        found.push((
            tick,
            match event.kind {
                midly::TrackEventKind::Meta(midly::MetaMessage::EndOfTrack) => "end of track",
                midly::TrackEventKind::Meta(_) => "meta",
                midly::TrackEventKind::Midi { message, .. } => match message {
                    midly::MidiMessage::ProgramChange { .. } => "program",
                    midly::MidiMessage::Controller { .. } => "controller",
                    midly::MidiMessage::NoteOn { vel, .. } if vel.as_int() > 0 => "strikes",
                    midly::MidiMessage::NoteOff { .. } | midly::MidiMessage::NoteOn { .. } => {
                        "releases"
                    }
                    _ => "other",
                },
                _ => "other",
            },
        ));
    }
    found
}

/// Two Bars of 2/4 at 480 PPQ, so that Bar 2 begins at Tick 960 and a passage
/// can be asked for by Bar without arithmetic in every test.
const PPQ: u16 = 480;
const BAR_2: u32 = 960;

/// A passage whose inherited Program is on the same track as the notes it
/// governs keeps the order the Take gave them.
///
/// The Program is stated in Bar 1 and the passage begins at Bar 2, so it is
/// inherited to the passage's first Tick — where the first note of Bar 2 also
/// lands. Both are on the voice track, so a Rank orders them, and it orders
/// them as the Take did: the Program first, and the note sounds on the
/// instrument the Take names.
#[test]
fn an_inherited_program_on_the_notes_own_track_keeps_its_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("rig.sf2"));
    let take = common::build_take_setting(
        &dir.path().join("together.mid"),
        PPQ,
        &[(0, 2, 4)],
        &[
            common::program_change(0, 40),
            common::strike(0, 69),
            common::release(480, 69),
            common::strike(BAR_2, 71),
            common::release(BAR_2 + 480, 71),
        ],
        &[],
    );

    let output = mid()
        .args(["play"])
        .arg(&take)
        .args(["--bars", "2:2", "--rig"])
        .arg(&soundfont)
        .env("PATH", &fake.dir)
        .output()
        .expect("mid runs");
    assert!(output.status.success(), "play failed: {output:?}");

    assert_eq!(
        order_of_track(&fake.handed, 1),
        vec![
            (0, "program"),
            (0, "strikes"),
            (480, "releases"),
            (960, "end of track"),
        ],
        "the Program is met before the note it governs, as it was in the Take"
    );
}

/// A passage that would put an inherited Program and the notes it governs on
/// two tracks at one Tick is refused.
///
/// `build_take_stating_apart` writes the notes on track 1 and the Program on
/// track 2. In the Take they are ordered by Tick — the Program at 0, the note
/// at 960 — and nothing has to rank them. The passage would put both at its
/// first Tick, on two tracks, and a Rank does not run between tracks. That is
/// an order the Take stated and the passage would not.
#[test]
fn a_passage_that_would_lose_an_order_the_take_stated_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("rig.sf2"));
    let take = common::build_take_stating_apart(
        &dir.path().join("apart.mid"),
        PPQ,
        &[(0, 2, 4)],
        &[common::program_change(0, 40)],
        &[(0, 480, 69), (BAR_2, 480, 71)],
    );
    let before = std::fs::read(&take).expect("the Take is readable");

    let output = mid()
        .args(["play"])
        .arg(&take)
        .args(["--bars", "2:2", "--rig"])
        .arg(&soundfont)
        .env("PATH", &fake.dir)
        .output()
        .expect("mid runs");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");

    assert_eq!(
        output.status.code(),
        Some(1),
        "an ordering-safety refusal is a refusal, not an audition: {stderr}"
    );
    for expected in [
        "program change",
        "channel 0",
        "track 2",
        "track 1",
        "would state no order between them",
        "Nothing has been written",
    ] {
        assert!(
            stderr.contains(expected),
            "the refusal does not say {expected:?}: {stderr}"
        );
    }
    assert!(
        !stderr.contains("panicked"),
        "the refusal is a typed error, not a panic: {stderr}"
    );

    assert!(
        !fake.handed.exists(),
        "the refusal handed a passage to the synthesiser anyway"
    );
    assert!(
        !fake.log.exists(),
        "the refusal started the synthesiser anyway"
    );
    assert_eq!(
        std::fs::read(&take).expect("the Take is readable"),
        before,
        "the source Take was written to"
    );
    assert_eq!(
        std::fs::read_dir(dir.path())
            .expect("the scratch directory is readable")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|kind| kind == "mid"))
            .count(),
        1,
        "the refusal left a partial Take behind"
    );
}

/// A refusal names an ordering problem in words a parse failure never uses.
///
/// #29 asks that a caller be able to tell one from the other. Both exit 1 —
/// which is this repository's whole vocabulary of exit codes, and says *a
/// refusal rather than an answer* — so the sentence is what separates them.
#[test]
fn an_ordering_refusal_does_not_read_like_a_parse_failure() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("rig.sf2"));
    let take = common::build_take_stating_apart(
        &dir.path().join("apart.mid"),
        PPQ,
        &[(0, 2, 4)],
        &[common::program_change(0, 40)],
        &[(0, 480, 69), (BAR_2, 480, 71)],
    );

    let output = mid()
        .args(["play"])
        .arg(&take)
        .args(["--bars", "2:2", "--rig"])
        .arg(&soundfont)
        .env("PATH", &fake.dir)
        .output()
        .expect("mid runs");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");

    for absent in ["is not a MIDI file", "cannot read", "cannot write"] {
        assert!(
            !stderr.contains(absent),
            "the refusal reads like an I/O or parse failure: {stderr}"
        );
    }
}

/// A pair the Take already left unranked is carried, not refused.
///
/// Here the Program and the note it governs share Tick 960 in the Take itself,
/// on two tracks. The file states no order between them and never did; the
/// passage takes nothing away, and what a Take arrived with is the author's
/// (ADR-0003). `mid inspect` is where that site is reported, and refusing to
/// play it would be `mid` declining to read an imperfect Take — which is the
/// one thing the reading side is for.
#[test]
fn a_site_the_take_already_left_open_is_played_rather_than_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("rig.sf2"));
    let take = common::build_take_stating_apart(
        &dir.path().join("open.mid"),
        PPQ,
        &[(0, 2, 4)],
        &[common::program_change(BAR_2, 40)],
        &[(0, 480, 69), (BAR_2, 480, 71)],
    );

    let output = mid()
        .args(["play"])
        .arg(&take)
        .args(["--bars", "2:2", "--rig"])
        .arg(&soundfont)
        .env("PATH", &fake.dir)
        .output()
        .expect("mid runs");
    assert!(
        output.status.success(),
        "a site the Take already left open was refused: {output:?}"
    );
}

/// A whole-Take audition is never refused, because nothing is prepared.
///
/// The refusal is a property of the preparation, not of the Take. `play` with
/// no `--bars` hands the file itself to the synthesiser, so a Take whose Bar 2
/// cannot be cut out safely still plays from the beginning.
#[test]
fn a_take_played_whole_is_not_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("rig.sf2"));
    let take = common::build_take_stating_apart(
        &dir.path().join("apart.mid"),
        PPQ,
        &[(0, 2, 4)],
        &[common::program_change(0, 40)],
        &[(0, 480, 69), (BAR_2, 480, 71)],
    );

    let output = mid()
        .args(["play"])
        .arg(&take)
        .args(["--rig"])
        .arg(&soundfont)
        .env("PATH", &fake.dir)
        .output()
        .expect("mid runs");
    assert!(output.status.success(), "play failed: {output:?}");
}

/// A Program on another track that governs a channel the passage's first Tick
/// does not strike is not a hazard.
///
/// The pair needs both halves. Track 2 states a Program for channel 1 and no
/// note of channel 1 lands at the passage's first Tick, so there is no order
/// to lose and nothing to refuse — the refusal is narrow enough to leave the
/// ordinary case alone.
#[test]
fn a_state_for_a_channel_the_first_tick_does_not_strike_is_not_a_hazard() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fake = fake_fluidsynth(dir.path());
    let soundfont = common::fake_soundfont(&dir.path().join("rig.sf2"));
    let take = common::build_take_stating_apart(
        &dir.path().join("other-channel.mid"),
        PPQ,
        &[(0, 2, 4)],
        &[(
            0,
            midly::TrackEventKind::Midi {
                channel: midly::num::u4::new(1),
                message: midly::MidiMessage::ProgramChange {
                    program: midly::num::u7::new(40),
                },
            },
        )],
        &[(0, 480, 69), (BAR_2, 480, 71)],
    );

    let output = mid()
        .args(["play"])
        .arg(&take)
        .args(["--bars", "2:2", "--rig"])
        .arg(&soundfont)
        .env("PATH", &fake.dir)
        .output()
        .expect("mid runs");
    assert!(
        output.status.success(),
        "a Program for a channel the first Tick does not strike was refused: {output:?}"
    );
}
