use crate::error::Result;
use crate::note::{Note, NoteId};
use crate::take::Take;
use serde::Serialize;
use std::collections::HashMap;

/// What differs between two Takes, in the Piece's terms.
///
/// A diff never reports Rig differences. Two Takes heard through different
/// soundfonts are the same Take here, which is the point: "the brass sounds
/// wrong" has to stay answerable as either a Piece problem or a Rig problem.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diff {
    pub added: Vec<Note>,
    pub removed: Vec<Note>,
    pub velocity_changed: Vec<VelocityChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VelocityChange {
    pub id: NoteId,
    pub track: usize,
    pub channel: u8,
    pub pitch: u8,
    pub start: u32,
    pub from: u8,
    pub to: u8,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.velocity_changed.is_empty()
    }
}

/// Compare two Takes by exact identity only.
///
/// A note in one Take matches a note in the other only when track, channel,
/// pitch and start tick are identical — which is what an identity is. Nothing
/// here decides that a note *moved*; matching within a tolerance is ADR-0004's
/// second pass and is deliberately not in this scope, so a moved note reports
/// here as one Removed and one Added.
pub fn diff(before: &Take, after: &Take) -> Result<Diff> {
    let before_notes = before.notes()?;
    let after_notes = after.notes()?;

    let after_by_id: HashMap<&NoteId, &Note> = after_notes.iter().map(|n| (&n.id, n)).collect();
    let before_by_id: HashMap<&NoteId, &Note> = before_notes.iter().map(|n| (&n.id, n)).collect();

    let mut velocity_changed = Vec::new();
    let mut removed = Vec::new();
    for note in &before_notes {
        match after_by_id.get(&note.id) {
            None => removed.push(note.clone()),
            Some(other) if other.velocity != note.velocity => {
                velocity_changed.push(VelocityChange {
                    id: note.id.clone(),
                    track: note.track,
                    channel: note.channel,
                    pitch: note.pitch,
                    start: note.start,
                    from: note.velocity,
                    to: other.velocity,
                });
            }
            Some(_) => {}
        }
    }

    let added = after_notes
        .iter()
        .filter(|note| !before_by_id.contains_key(&note.id))
        .cloned()
        .collect();

    Ok(Diff {
        added,
        removed,
        velocity_changed,
    })
}
