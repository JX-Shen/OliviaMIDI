//! The Edit Set contract, checked against the type that defines it — #21.
//!
//! `mid apply --help` is the only exhaustive list of the Edit kinds, and
//! `AGENTS.md` sends every agent to it before writing an `edits.json`. That
//! makes it a promise, and 0.1.1 shipped with it broken: the help, the README
//! and `AGENTS.md` each held their own copy of the list and the binary held a
//! fourth. #17 deleted the copies. This is what stops the last one drifting.
//!
//! The list of kinds is not written down here either. It is read out of the
//! `Edit` type itself, through the one thing that already knows it exhaustively:
//! `serde` naming every variant it would have accepted when it is handed one it
//! does not recognise. So a kind added to the enum joins this test's expectation
//! the moment it compiles, and there is no second list for somebody to forget.

mod common;

use common::mid;

/// The kinds the Edit Set actually accepts, from `Edit`'s own definition.
///
/// Through a deliberate parse failure, because `serde`'s "expected one of" is
/// generated from the variants and cannot fall behind them. A macro or a
/// hand-written array here would be a fourth copy of the fact this ticket exists
/// to stop copying.
fn kinds() -> Vec<String> {
    let refusal = serde_json::from_str::<battuta::EditSet>(r#"{ "edits": [ { "kind": "" } ] }"#)
        .expect_err("an empty kind is not a kind")
        .to_string();
    let (_, listed) = refusal
        .split_once("expected one of ")
        .expect("serde names the variants it would have taken");
    listed
        .split(&[',', ' '][..])
        .filter_map(|word| word.trim().strip_prefix('`'))
        .filter_map(|word| word.split('`').next())
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// Read once: the help is what the binary presents, not what the source says.
fn help() -> String {
    let out = mid().args(["apply", "--help"]).output().expect("mid runs");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn serde_gives_up_every_kind_there_is() {
    let kinds = kinds();
    // Not a count and not a list — either would be the copy this test refuses to
    // make. Only that the reading worked at all, so that a `serde` whose wording
    // changed fails here rather than silently checking nothing.
    assert!(kinds.len() > 1, "read no kinds out of serde: {kinds:?}");
    assert!(
        kinds
            .iter()
            .all(|kind| kind.chars().all(|c| c.is_ascii_lowercase() || c == '_')),
        "read something that is not a kind: {kinds:?}"
    );
}

/// Every kind the binary accepts is one the help tells you about. The direction
/// that matters most: a capability nobody is told about is the one #11 and #17
/// are both about.
#[test]
fn every_kind_the_binary_accepts_is_in_the_help() {
    let help = help();
    for kind in kinds() {
        assert!(
            help.contains(&kind),
            "`{kind}` is an Edit kind the help does not mention"
        );
    }
}

/// And the other direction: the help does not promise a kind the binary would
/// refuse. A stale name left in prose sends an agent to write an Edit Set that
/// cannot run.
#[test]
fn the_help_promises_no_kind_the_binary_would_refuse() {
    let help = help();
    let kinds = kinds();
    // Only the block that spells the kinds out as JSON, so that ordinary English
    // in the rest of the help is not read as a promise.
    for line in help.lines().filter(|line| line.contains("\"kind\"")) {
        let named = line
            .split_once("\"kind\":")
            .map(|(_, rest)| rest.trim().trim_start_matches('"'))
            .and_then(|rest| rest.split('"').next())
            .expect("a kind is named on this line");
        assert!(
            kinds.iter().any(|kind| kind == named),
            "the help offers `{named}`, which the Edit Set would refuse"
        );
    }
}

/// The fixture holds one minimal example of every kind, and parses.
///
/// It is what a reader can copy from and what a change to the shape of any kind
/// has to be made against: a field renamed under an existing kind is a break the
/// two tests above cannot see, because the kind's *name* is still there.
#[test]
fn the_fixture_holds_one_of_every_kind_and_parses() {
    let text = std::fs::read_to_string("fixtures/every-kind.json").expect("the fixture is there");
    let edits: battuta::EditSet = serde_json::from_str(&text).expect("the fixture parses");

    for kind in kinds() {
        assert!(
            text.contains(&format!("\"{kind}\"")),
            "`{kind}` has no example in fixtures/every-kind.json"
        );
    }
    assert_eq!(
        edits.edits.len(),
        kinds().len(),
        "the fixture holds one example per kind and no more"
    );
}
