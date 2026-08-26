//! `mid` — the binary OliviaMIDI ships.
//!
//! Every command here does three things: resolve its arguments into Takes, ask
//! the library one question about them, and format the answer. The only branches
//! a command module may hold are formatting ones. Anything that decides
//! *behaviour* — the order a Rig is resolved in, the refusal to write a Take over
//! its own input — lives in `battuta`, even when that means the library grows a
//! function shaped like a command. That rule is what keeps `battuta` an honest
//! library rather than a folder the CLI happens to keep its code in.

mod apply;
mod diff;
mod info;
mod inspect;
mod play;

use clap::{Parser, Subcommand};

/// Read, change, compare and hear one MIDI file.
///
/// A Take is one `.mid` file. What is inside it belongs to the Piece; what is
/// needed to turn it into air — the soundfont, the synthesiser — belongs to the
/// Rig. `mid` never blurs the two: a diff reports only the Piece, and `play`
/// always says which Rig you heard.
///
/// Ticks are the truth. Positions and durations are in the file's own ticks,
/// relative to the PPQ that `mid info` reports.
#[derive(Parser)]
#[command(name = "mid", version, verbatim_doc_comment)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Info(info::Args),
    Inspect(inspect::Args),
    Apply(apply::Args),
    Diff(diff::Args),
    Play(play::Args),
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Info(args) => info::run(args),
        Command::Inspect(args) => inspect::run(args),
        Command::Apply(args) => apply::run(args),
        Command::Diff(args) => diff::run(args),
        Command::Play(args) => play::run(args),
    };

    if let Err(error) = result {
        eprintln!("mid: {error}");
        std::process::exit(1);
    }
}

/// Rendering JSON of types this crate defines cannot fail; treating it as a
/// runtime error would only add a branch that no input can reach.
pub(crate) fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string_pretty(value).expect("battuta's own types serialise")
}
