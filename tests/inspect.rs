mod common;

use common::{mid, FIXTURE};
use std::collections::HashSet;

#[test]
fn lists_every_note_with_an_identity() {
    let json = common::inspect_json(std::path::Path::new(FIXTURE));
    let notes = common::notes(&json);
    assert_eq!(notes.len(), 36);

    let ids: HashSet<String> = common::note_ids(&json).into_iter().collect();
    assert_eq!(ids.len(), 36, "every identity is distinct");

    for note in &notes {
        for field in [
            "id", "track", "channel", "pitch", "start", "duration", "velocity",
        ] {
            assert!(note.get(field).is_some(), "note is missing {field}: {note}");
        }
    }
}

/// The fixture's durations are one tick short of a beat, two beats and a bar.
/// A model that tidied them up to 480 / 960 / 1440 would still sound plausible,
/// which is why this is asserted rather than eyeballed.
#[test]
fn reports_durations_as_they_are_written() {
    let json = common::inspect_json(std::path::Path::new(FIXTURE));
    let durations: HashSet<u64> = common::notes(&json)
        .iter()
        .map(|note| note["duration"].as_u64().expect("duration is a number"))
        .collect();
    assert_eq!(durations, HashSet::from([475, 955, 1435]));
}

#[test]
fn identities_are_stable_across_repeated_reads() {
    let first = common::note_ids(&common::inspect_json(std::path::Path::new(FIXTURE)));
    let second = common::note_ids(&common::inspect_json(std::path::Path::new(FIXTURE)));
    assert_eq!(first, second);
}

#[test]
fn fails_on_a_take_that_is_not_there() {
    mid()
        .args(["inspect", "fixtures/no-such-take.mid", "--json"])
        .assert()
        .failure();
}
