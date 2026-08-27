use serde::{Deserialize, Serialize};
use std::fmt;

/// A note's identity, derived from its content: track, channel, pitch, start
/// tick, and an occurrence index separating notes that collide on all four.
///
/// The occurrence index is always present, even when nothing collides. Omitting
/// it while a note is unique would mean that adding a second note at the same
/// place silently renamed the first — and identity stability across an Edit Set
/// is exactly what ADR-0002 is protecting.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NoteId(String);

impl NoteId {
    pub(crate) fn new(track: usize, channel: u8, pitch: u8, start: u32, occurrence: u32) -> Self {
        NoteId(format!(
            "t{track}:c{channel}:p{pitch}:s{start}:n{occurrence}"
        ))
    }
}

impl fmt::Display for NoteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One note of a Take, in the Take's own units: ticks, not seconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Note {
    pub id: NoteId,
    pub track: usize,
    pub channel: u8,
    pub pitch: u8,
    pub start: u32,
    pub duration: u32,
    pub velocity: u8,

    /// Where the note-on lives in its track's event list. Carried so that an
    /// Edit changes the event it names and leaves every other byte of the Take
    /// alone; never part of the published contract.
    #[serde(skip)]
    pub(crate) on_event: usize,

    /// Where the note-off that ends it lives, in the same list. Carried so that
    /// restricting a Take to a passage can keep a note whole — both of its
    /// events or neither — without pairing the track a second time.
    #[serde(skip)]
    pub(crate) off_event: usize,
}
