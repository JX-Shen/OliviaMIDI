//! That `docs/fixtures.md` describes the files in `fixtures/external/`, and
//! that it describes the ones that are actually there.
//!
//! The fixtures in `fixtures/external/` came from somewhere else, and what ties
//! a file to its origin is not in the bytes: a `.mid` is a `.mid`, and nothing
//! in it says which catalogue entry it was fetched from or under what licence.
//! `docs/fixtures.md` is where that tie is written, which makes the document
//! the kind of copy this repository has already been burnt by — true when it
//! was written, silently false the first time somebody refreshes a fixture and
//! leaves the prose alone (#39, and #16 for what that costs).
//!
//! So the digest is written once, in the document, and read from there. This
//! test is the same shape as `tests/contract.rs`: both directions, and the list
//! is never written down a second time beside the first.
//!
//! It cannot check a fixture against its upstream — no test can, because
//! upstream is a URL somebody else controls. Checking that is what the digest
//! in the document is *for*, and the reader who cares does it by hand. What
//! this test holds is the narrower thing a machine can hold: that the bytes in
//! the tree and the record of them have not drifted apart.

use std::collections::BTreeMap;
use std::path::Path;

const EXTERNAL: &str = "fixtures/external";
const RECORD: &str = "docs/fixtures.md";

/// Every `## <name>` heading in the record, with the digest its entry states.
///
/// An entry states its digest on one line, `- SHA-256: \`<hex>\``, so that
/// pulling it out needs no parser and breaking the shape is visible in a diff.
fn recorded() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(RECORD).expect("the record is there");
    let mut entries = BTreeMap::new();
    let mut current: Option<String> = None;

    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            current = Some(heading.trim().to_owned());
        } else if let Some(rest) = line.trim().strip_prefix("- SHA-256: ") {
            let digest = rest.trim().trim_matches('`').to_owned();
            let name = current
                .clone()
                .expect("a digest appears under a heading naming its file");
            assert!(
                entries.insert(name.clone(), digest).is_none(),
                "{RECORD} states a digest for `{name}` twice"
            );
        }
    }
    entries
}

/// The files actually sitting in `fixtures/external/`.
fn present() -> BTreeMap<String, std::path::PathBuf> {
    std::fs::read_dir(EXTERNAL)
        .expect("the sourced fixtures are there")
        .map(|entry| entry.expect("the directory is readable").path())
        .filter(|path| path.is_file())
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("a fixture's name is text")
                .to_owned();
            (name, path)
        })
        .collect()
}

/// Hexadecimal SHA-256 of a file, from whichever of the two tools is installed.
///
/// A dependency was the other way to do this, and it would have put six crates
/// in the lockfile to hash four files in one test. Both CI platforms have one
/// of these: `shasum` comes with perl, `sha256sum` with coreutils.
fn digest(path: &Path) -> String {
    let attempts = [("shasum", vec!["-a", "256"]), ("sha256sum", vec![])];

    for (tool, flags) in attempts {
        let run = std::process::Command::new(tool)
            .args(&flags)
            .arg(path)
            .output();
        if let Ok(output) = run {
            if output.status.success() {
                let stdout = String::from_utf8(output.stdout).expect("a digest is text");
                return stdout
                    .split_whitespace()
                    .next()
                    .expect("the digest comes first")
                    .to_owned();
            }
        }
    }
    panic!("neither `shasum` nor `sha256sum` is installed, so {path:?} cannot be checked");
}

/// Every fixture in `fixtures/external/` is the one the record describes.
#[test]
fn a_sourced_fixture_is_the_one_the_record_names() {
    let recorded = recorded();

    for (name, path) in present() {
        let stated = recorded.get(&name).unwrap_or_else(|| {
            panic!("`{name}` is in {EXTERNAL} with no entry in {RECORD}: where did it come from?")
        });
        assert_eq!(
            &digest(&path),
            stated,
            "`{name}` is not the file {RECORD} records — \
             the fixture was replaced and its provenance was not"
        );
    }
}

/// And the record describes no fixture that is not there.
///
/// The other direction, for the same reason `tests/contract.rs` checks both: an
/// entry for a file nobody can produce reads as evidence and is not.
#[test]
fn the_record_names_no_fixture_that_is_missing() {
    let present = present();

    for name in recorded().keys() {
        assert!(
            present.contains_key(name),
            "{RECORD} has an entry for `{name}`, which is not in {EXTERNAL}"
        );
    }
}

/// Each entry says what licence was read, and where, and when.
///
/// Not that the licence is acceptable — that is the human's, and #35 is where
/// it was decided. Only that the entry cannot be written without stating it,
/// because an entry that names a source and omits its terms is the one that
/// gets copied for the next fixture.
#[test]
fn every_entry_states_the_licence_it_was_read_under() {
    let text = std::fs::read_to_string(RECORD).expect("the record is there");

    for name in recorded().keys() {
        let entry = text
            .split_once(&format!("## {name}"))
            .expect("the heading is there")
            .1;
        let entry = entry
            .split_once("\n## ")
            .map(|(head, _)| head)
            .unwrap_or(entry);
        assert!(
            entry.contains("- Licence: "),
            "the entry for `{name}` names no licence"
        );
        assert!(
            entry.contains("Read 20"),
            "the entry for `{name}` does not say when its licence was read"
        );
    }
}
