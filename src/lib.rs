//! `battuta` — the core of OliviaMIDI.
//!
//! A Take is one `.mid` file. This crate reads one, names every note in it,
//! applies mechanical Edits to produce a new Take, says what differs between
//! two, and hands one to a Rig to be heard. It holds no musical intent: naming
//! what a change *means* is the agent's job, and this crate's job is to never
//! encode it.
//!
//! The `mid` binary is one consumer of this library, not the program itself.

pub mod bars;
pub mod controller;
pub mod diff;
pub mod edit;
mod error;
pub mod note;
pub mod passage;
pub mod program;
pub mod rig;
pub mod take;
mod temporary;
mod track;

pub use bars::{BarLines, BarRange, Position, TickSpan};
pub use controller::{Controller, Controllers, StatedController};
pub use diff::{Change, ControllerDifference, ControllerSide, Diff, NoteChange, ProgramDifference};
pub use edit::{apply, Edit, EditSet};
pub use error::{Error, Result};
pub use note::{pitch_name, Note, NoteId, PitchName};
pub use program::{gm_name, Program, Programs, StatedProgram, GM_PERCUSSION_CHANNEL};
pub use rig::{Audition, Rig};
pub use take::{Info, Take, Tempo, TimeSignature};
pub use temporary::remove_temporary_takes_on_signals;
