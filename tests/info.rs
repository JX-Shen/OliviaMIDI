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
