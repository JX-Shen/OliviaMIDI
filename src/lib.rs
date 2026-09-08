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
pub mod bend;
pub mod controller;
pub mod diff;
pub mod edit;
mod error;
pub mod note;
pub mod passage;
pub mod program;
pub mod rank;
pub mod reading;
pub mod rig;
pub mod take;
mod temporary;
mod track;
pub mod unranked;

pub use bars::{BarLines, BarRange, Position, TickSpan};
pub use bend::{Bend, BendExtremes, Bends, StatedBend};
pub use controller::{
    spec_name, Controller, ControllerPeak, Controllers, StatedController, FIRST_CHANNEL_MODE,
};
pub use diff::{
    BendDifference, BendSide, Change, ControllerDifference, ControllerSide, Diff, NoteChange,
    ProgramDifference, TempoDifference, TempoSide, UnrankedBend, UnrankedController,
    UnrankedProgram, UnrankedStateSite,
};
pub use edit::{apply, apply_allowing, Edit, EditSet, Site};
pub use error::{Error, Result};
pub use note::{pitch_name, Note, NoteId, PitchName};
pub use program::{gm_name, Program, Programs, StatedProgram, GM_PERCUSSION_CHANNEL};
pub use rank::{RankDisagreement, RankedPairKind, UnrankedSite};
pub use reading::{Candidate, Reading, UnrankedComparison, UnrankedSpan};
pub use rig::{Audition, Rig};
pub use take::{Info, StatedTempo, Take, Tempo, TimeSignature};
pub use temporary::remove_temporary_takes_on_signals;
pub use unranked::{Against, State, Unranked};
