mod common;

use common::{mid, FIXTURE};

#[test]
fn reports_what_the_take_is() {
    let output = mid()
        .args(["info", FIXTURE, "--json"])
        .output()
        .expect("mid runs");
    assert!(output.status.success());
    let info: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("info --json is JSON");

    assert_eq!(info["format"], 1);
    assert_eq!(info["tracks"], 3);
    assert_eq!(info["ppq"], 480);
    assert_eq!(info["tempo"]["bpm"], 60.0);
    assert_eq!(info["time_signature"]["numerator"], 3);
    assert_eq!(info["time_signature"]["denominator"], 4);
    assert_eq!(info["length_ticks"], 11516);
}

/// The fixture's conductor track carries the tempo and the metre and holds no
/// notes, so reading them at all is the assertion: a `mid` that looked for them
/// beside the notes would find neither.
#[test]
fn reads_tempo_and_metre_from_the_conductor_track() {
    mid()
        .args(["info", FIXTURE])
        .assert()
        .success()
        .stdout(predicates::str::contains("tempo           60 bpm"))
        .stdout(predicates::str::contains("time signature  3/4"));
}

#[test]
fn fails_on_a_take_that_is_not_there() {
    mid()
        .args(["info", "fixtures/no-such-take.mid"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("no-such-take.mid"));
}

#[test]
fn fails_on_a_file_that_is_not_midi() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("not-midi.mid");
    common::write(&path, "this is not a MIDI file");
    mid().arg("info").arg(&path).assert().failure();
}

/// A metre this model cannot represent is an error, not a `3/0`. A wrong answer
/// about the metre would silently misplace every Bar computed from it.
#[test]
fn refuses_a_time_signature_it_cannot_represent() {
    use midly::{Format, Header, MetaMessage, Smf, Timing, TrackEvent, TrackEventKind};

    let mut smf = Smf::new(Header::new(Format::Parallel, Timing::Metrical(480.into())));
    smf.tracks.push(vec![
        TrackEvent {
            delta: 0.into(),
            // Denominator 2^8: a note value no MIDI file should claim.
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 8, 24, 8)),
        },
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ]);

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("impossible-metre.mid");
    smf.save(&path).expect("scratch Take is writable");

    mid()
        .arg("info")
        .arg(&path)
        .assert()
        .failure()
        .stderr(predicates::str::contains("time signature"));
}

/// A Take stating 0 Ticks per quarter note places nothing in time: every
/// duration, every Bar line and every tempo reading derived from it would be a
/// division by zero. Refused where the division is read, not where it bites.
#[test]
fn refuses_a_take_with_no_ticks_per_quarter() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("no-ppq.mid"),
        0,
        &[(0, 3, 4)],
        &[(0, 1, 60)],
    );

    mid()
        .arg("info")
        .arg(&take)
        .assert()
        .failure()
        .stderr(predicates::str::contains("ticks per quarter"));
}

/// An unrepresentable denominator is refused wherever it appears, including
/// after a time signature that *could* be read. Before Bar semantics existed,
/// `info` reported the readable one and said nothing about the other; a Take
/// that states something this tool cannot read is not a Take it should describe
/// as if it had.
///
/// Built by hand rather than with `build_take`, which only speaks real note
/// values — 2^8 is not one, which is the whole point of the test.
#[test]
fn refuses_an_unreadable_denominator_stated_after_a_readable_one() {
    use midly::{Format, Header, MetaMessage, Smf, Timing, TrackEvent, TrackEventKind};

    let meta = |delta: u32, message: MetaMessage<'static>| TrackEvent {
        delta: delta.into(),
        kind: TrackEventKind::Meta(message),
    };
    let mut smf = Smf::new(Header::new(Format::Parallel, Timing::Metrical(480.into())));
    smf.tracks.push(vec![
        meta(0, MetaMessage::TimeSignature(3, 2, 24, 8)),
        meta(1440, MetaMessage::TimeSignature(4, 8, 24, 8)),
        meta(0, MetaMessage::EndOfTrack),
    ]);

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("readable-then-not.mid");
    smf.save(&path).expect("scratch Take is writable");

    mid()
        .arg("info")
        .arg(&path)
        .assert()
        .failure()
        .stderr(predicates::str::contains("time signature"));
}

/// How long the Take is in the unit a musician talks in. `length_ticks` stays
/// the truth; this is the derived view of it, and it agrees with what
/// `inspect --bars` will select: eight Bars, the last one partial.
#[test]
fn reports_how_long_the_take_is_in_bars() {
    let output = mid()
        .args(["info", FIXTURE, "--json"])
        .output()
        .expect("mid runs");
    let info: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("info --json is JSON");
    assert_eq!(info["length_bars"], 8);

    mid()
        .args(["info", FIXTURE])
        .assert()
        .success()
        .stdout(predicates::str::contains("11516 ticks (8 bars)"));
}

/// `info` answers on every readable Take, including one whose Bars cannot be
/// derived — it is the command for finding out what you are holding, so it must
/// not refuse the Takes you most need to ask about. The count is absent rather
/// than guessed, and both outputs say so the same way: `info` reports the fact,
/// and `inspect --bars` is where the reason and its remedy live.
#[test]
fn reports_no_bar_count_when_the_bars_cannot_be_derived() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take(
        &dir.path().join("no-time-signature.mid"),
        480,
        &[],
        &[(0, 240, 60)],
    );

    let output = mid()
        .args(["info", "--json"])
        .arg(&take)
        .output()
        .expect("mid runs");
    assert!(output.status.success(), "info refused a readable Take");
    let info: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("info --json is JSON");
    assert!(info["length_bars"].is_null());
    assert!(info["length_ticks"].as_u64().expect("a length") > 0);

    mid()
        .arg("info")
        .arg(&take)
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "(bars need one stated time signature)",
        ));
}

/// The same input has to give the same answer whichever profile `mid` was built
/// in, and a refusal is the answer.
///
/// A delta time is 28 bits and a track may hold any number of them, so a
/// well-formed file can accumulate past the largest absolute Tick `battuta`
/// holds. That total was added up unchecked: a debug build panicked at it, and a
/// release build wrapped and reported the wrapped Tick as the length — one file,
/// two results, neither of them true. Exit 1 is the refusal, 101 the panic and 0
/// the false answer, so the code alone tells the three apart.
///
/// Asserted through `info` alone because the check is at `Take::read`, which is
/// the one door every command comes through. Five commands would be five tests
/// of one boundary.
#[test]
fn a_take_past_the_tick_range_is_refused_rather_than_answered() {
    let dir = tempfile::tempdir().expect("temp dir");
    let take = common::build_take_past_the_tick_range(&dir.path().join("past-the-range.mid"));

    let output = mid().arg("info").arg(&take).output().expect("mid runs");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(1),
        "a Take past the Tick range is a refusal, not a panic and not an answer: {stderr}"
    );
    assert!(
        stderr.contains("largest absolute Tick"),
        "the refusal does not say what was exceeded: {stderr}"
    );
    assert!(
        !stderr.contains("panicked"),
        "the refusal is a typed error, not a panic: {stderr}"
    );
}

/// The whole block, because the layout is the thing under test: which facts are
/// reported, in what order, and lined up so the values form a column.
#[test]
fn reads_as_a_summary_of_what_the_take_is() {
    assert_eq!(
        common::human_output(&["info", FIXTURE]),
        "\
format          1
tracks          3
ppq             480
tempo           60 bpm (1000000 us per quarter)
time signature  3/4
length          11516 ticks (8 bars)
"
    );
}
