//! `battuta` — the core of OliviaMIDI.
//!
//! A Take is one `.mid` file. This crate reads one, names every note in it,
//! applies mechanical Edits to produce a new Take, says what differs between
//! two, and hands one to a Rig to be heard. It holds no musical intent: naming
//! what a change *means* is the agent's job, and this crate's job is to never
//! encode it.
//!
//! The `mid` binary is one consumer of this library, not the program itself.

pub mod diff;
pub mod edit;
mod error;
pub mod note;
pub mod rig;
pub mod take;

pub use diff::{Diff, VelocityChange};
pub use edit::{apply, Edit, EditSet};
pub use error::{Error, Result};
pub use note::{Note, NoteId};
pub use rig::{Audition, Rig};
pub use take::{Info, Take, Tempo, TimeSignature};
