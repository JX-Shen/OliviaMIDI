use crate::error::{Error, Result};
use crate::note::{Note, NoteId};
use crate::take::Take;
use midly::num::u7;
use midly::{MidiMessage, TrackEventKind};
use serde::Deserialize;
use std::path::Path;

/// A batch of Edits, applied together. The on-disk form is `edits.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditSet {
    pub edits: Vec<Edit>,
}

/// One mechanical change to a Take.
///
/// Mechanical is the whole point: an Edit says which note and what number, never
/// what the change is *for*. Musical intent belongs to the agent holding it, and
/// this enum is where it would leak in if it ever did.
///
/// The discriminator is `kind`, not `op`: an Edit Set contains Edits, and a key
/// that called them operations would seed the word into every sentence written
/// about them. See `CONTEXT.md` under **Edit**.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    SetVelocity {
        id: NoteId,
        /// Kept wide so an out-of-range value is reported as itself rather than
        /// as a JSON parse failure that does not say which number was wrong.
        velocity: i64,
    },
}

impl EditSet {
    pub fn read(path: &Path) -> Result<EditSet> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|source| Error::EditSetUnreadable {
            path: path.to_path_buf(),
            source,
        })
    }

    /// The note an Edit names. Every Edit names exactly one, which is what makes
    /// resolving a whole Edit Set up front possible at all.
    fn target(edit: &Edit) -> &NoteId {
        match edit {
            Edit::SetVelocity { id, .. } => id,
        }
    }
}

/// Apply an Edit Set to a Take, producing a new one. The input is never touched.
///
/// Every identity is resolved against the input Take *before* the first effect
/// lands (ADR-0002). Edits apply in the order given, so their effects are
/// ordered while their targets were all fixed in advance. With one Edit and no
/// collisions the two readings are indistinguishable, which is exactly why the
/// structure has to be right now rather than when it starts to matter.
pub fn apply(take: &Take, edit_set: &EditSet) -> Result<Take> {
    let notes = take.notes()?;

    let resolved: Vec<(&Edit, Note)> = edit_set
        .edits
        .iter()
        .map(|edit| {
            let target = EditSet::target(edit);
            notes
                .iter()
                .find(|note| &note.id == target)
                .cloned()
                .map(|note| (edit, note))
                .ok_or_else(|| Error::UnknownNote(target.to_string()))
        })
        .collect::<Result<_>>()?;

    let mut smf = take.smf()?;
    for (edit, note) in resolved {
        match *edit {
            Edit::SetVelocity { velocity, .. } => {
                let velocity = u8::try_from(velocity)
                    .ok()
                    .filter(|v| (1..=127).contains(v))
                    .ok_or(Error::VelocityOutOfRange(velocity))?;
                let event = &mut smf.tracks[note.track][note.on_event];
                if let TrackEventKind::Midi {
                    message: MidiMessage::NoteOn { vel, .. },
                    ..
                } = &mut event.kind
                {
                    *vel = u7::new(velocity);
                }
            }
        }
    }

    let mut bytes = Vec::new();
    smf.write(&mut bytes)
        .map_err(|source| Error::Encode(source.to_string()))?;
    Ok(Take::from_bytes(bytes))
}

/// The whole of `mid apply`: read the Take, read the Edit Set, and write a new
/// Take somewhere else.
///
/// "Somewhere else" is enforced here rather than left to the caller. `apply`
/// never writes in place — losing the Take you liked has to be impossible, not
/// merely discouraged — so an output path that resolves to the input is refused
/// before anything is read.
pub fn apply_to_new_take(input: &Path, edit_set: &Path, output: &Path) -> Result<()> {
    if same_file(input, output) {
        return Err(Error::WriteInPlace(output.to_path_buf()));
    }
    let take = Take::read(input)?;
    let edit_set = EditSet::read(edit_set)?;
    apply(&take, &edit_set)?.write(output)
}

/// Whether two paths name the same file, resolving symlinks and `..` where the
/// filesystem can. An output file usually does not exist yet, so its directory
/// is resolved and the file name compared.
fn same_file(a: &Path, b: &Path) -> bool {
    fn resolved(path: &Path) -> Option<std::path::PathBuf> {
        if let Ok(canonical) = path.canonicalize() {
            return Some(canonical);
        }
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty())?;
        Some(parent.canonicalize().ok()?.join(path.file_name()?))
    }
    match (resolved(a), resolved(b)) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}
