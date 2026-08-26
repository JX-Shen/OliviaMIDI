use std::path::PathBuf;

/// Apply an Edit Set to a Take, writing a new Take.
///
/// The input is never modified and `-o` is required: the Take you liked cannot
/// be lost by running this. An Edit Set naming a note that is not in the input
/// fails the whole run — no partial Take is written.
///
/// Every identity in the Edit Set is resolved against the input Take before the
/// first operation is applied, so operations cannot renumber each other's
/// targets. One consequence: an operation cannot name a note added by an earlier
/// operation in the same Edit Set.
///
/// The Edit Set is JSON, and one operation exists so far:
///
///     { "edits": [ { "op": "set_velocity", "id": "t1:c0:p69:s0:n0", "velocity": 40 } ] }
///
/// Identities come from `mid inspect`.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to change. Opened read-only.
    take: PathBuf,

    /// The Edit Set to apply, as JSON.
    edits: PathBuf,

    /// Where to write the new Take. Required, and never the input.
    #[arg(short = 'o', long = "output")]
    output: PathBuf,
}

pub fn run(args: Args) -> battuta::Result<()> {
    battuta::edit::apply_to_new_take(&args.take, &args.edits, &args.output)
}
