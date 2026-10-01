use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Make one Take from two Alternatives and the common Take both were made from.
///
/// Each `--side` is an Alternative and the Edit Set that made it from the
/// common Take, exactly two of them. `-o` is required and is never one of the
/// inputs; on any refusal nothing is written.
///
/// Each side's Edit Set is first replayed on the common Take, and what it makes
/// has to be event-for-event the Take supplied beside it — the same tracks, the
/// same events in the same order, at the same Ticks. Bytes are not compared.
/// A side that does not replay, or replays to something else, is refused before
/// anything is read from either side. The sides are checked in the order given.
///
/// What each side actually changed then becomes a demand on a note of the
/// common Take. This release takes up three changes to an existing note: its
/// velocity (`set_velocity`), its duration (`resize_note`) and its presence
/// (`delete_note`). An Edit Set holding any other kind is refused. Only where a
/// note ends up counts, not the steps on the way, and a request for the value a
/// note already has asks for nothing: it is reported as having no effect and is
/// never a demand, nor a promise that the note stays as it is. Both sides asking
/// the same thing of a note is one demand.
///
/// Where the two sides demand different values of one field of one note, or one
/// deletes a note the other changes, that is a conflict. Every conflict found is
/// reported together, with the note and both demands, and the whole combination
/// is refused: nothing is settled by arithmetic or by choosing a side. Narrow an
/// Edit Set and combine again.
///
/// Otherwise every demand is applied to the common Take as one Edit Set, through
/// the same path `mid apply` takes and refused by the same checks. A problem
/// found only there — two changes each sound alone and impossible together — is
/// reported as found, not as the complete list of what stands in the way.
///
/// Then the events sharing each Tick are put in order. A side that moved one
/// event of a pair — a resize moves a note's release, even one that brings it
/// back to where it was — and holds both at that Tick decides their order. A
/// pair neither side moved keeps the common Take's order. A pair that first
/// meets in the combined Take is ordered by where the notes they belong to begin
/// in the common Take. Where the two sides wrote a pair in opposite orders, the
/// combination is refused only if the order is one the file gives a meaning to,
/// which is exactly an order `mid diff` compares — see `mid diff --help` for
/// which those are. Any other such pair is ordered as a pair
/// that first meets is, and the site is disclosed, so that you can narrow an
/// Edit Set if you mean the other order.
///
/// A track's end moves only where a lengthened note now finishes past it, and
/// every end that moved is reported with where it was and where it is.
/// Deleting or shortening never moves one. A combination that changes nothing
/// at all is a success, and is reported as unchanged.
///
/// Without `--json`, what was verified and what the library states — requests
/// with no effect, disclosed sites, moved track ends, an unchanged result — are
/// lines on stderr, and the path written is the one line on stdout. A refusal
/// prints nothing on stdout.
///
/// With `--json`, stdout carries one document whether the combination succeeds
/// or not: `common`, `sides` (each `take`, `edits` and `verified` — true
/// where it replayed and matched, false for the side that failed, null for one
/// not yet checked), `output`, and then either `unchanged`, `no_effect`,
/// `disclosed_sites` and `track_ends`, or — with `output` null and exit status
/// 1 — a `refusal` whose `kind` is `source_replay_failed`,
/// `source_evidence_mismatch`, `conflict`, `order_contradiction` or
/// `core_invalid`. The refusal is also stated on stderr.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The common Take both Alternatives were made from. Opened read-only.
    common: PathBuf,

    /// An Alternative and the Edit Set that made it from the common Take.
    /// Given exactly twice.
    #[arg(
        long = "side",
        num_args = 2,
        value_names = ["TAKE", "EDITS"],
        action = clap::ArgAction::Append,
        required = true
    )]
    side: Vec<PathBuf>,

    /// Where to write the combined Take. Required, and never an input.
    #[arg(short = 'o', long = "output")]
    output: PathBuf,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> battuta::Result<()> {
    let sides: Vec<(PathBuf, PathBuf)> = args
        .side
        .chunks(2)
        .map(|pair| (pair[0].clone(), pair[1].clone()))
        .collect();
    let result = battuta::combine::combine_to_new_take(&args.common, &sides, &args.output);

    let refusal = match &result {
        Ok(_) => None,
        Err(error) => refusal(error),
    };
    // Which sides were verified: `Some(true)` replayed and matched,
    // `Some(false)` the side that failed, `None` not reached. The library
    // checks them in the order given, each completely, and a refusal naming a
    // side is the first that failed. Unchecked is not failed.
    let verified: Vec<Option<bool>> = (0..sides.len())
        .map(|side| match &result {
            Ok(_) => Some(true),
            Err(battuta::Error::SourceReplayFailed { side: failed, .. })
            | Err(battuta::Error::SourceEvidenceMismatch { side: failed, .. }) => {
                match side.cmp(failed) {
                    std::cmp::Ordering::Less => Some(true),
                    std::cmp::Ordering::Equal => Some(false),
                    std::cmp::Ordering::Greater => None,
                }
            }
            Err(_) if refusal.is_some() => Some(true),
            Err(_) => None,
        })
        .collect();

    let combined = match result {
        Ok(combined) => combined,
        Err(error) => {
            if let Some(refusal) = refusal {
                if args.json {
                    let mut document = described(&args.common, &sides, &verified);
                    document["output"] = Value::Null;
                    document["refusal"] = refusal;
                    println!("{}", crate::json(&document));
                }
                for line in refused_lines(&error) {
                    eprintln!("{line}");
                }
            }
            return Err(error);
        }
    };

    if args.json {
        let mut document = described(&args.common, &sides, &verified);
        document["output"] = json!(args.output.display().to_string());
        document["unchanged"] = json!(combined.unchanged);
        document["no_effect"] = json!(combined.no_effect);
        document["disclosed_sites"] = json!(combined.disclosed_sites);
        document["track_ends"] = json!(combined.track_ends);
        println!("{}", crate::json(&document));
    }

    let lines = combined.take.bar_lines();
    for (side, (take, edits)) in sides.iter().enumerate() {
        eprintln!("{}", crate::wording::verified_side(side, take, edits));
    }
    for entry in &combined.no_effect {
        eprintln!("{}", crate::wording::no_effect(entry));
    }
    for site in &combined.disclosed_sites {
        eprintln!("{}", crate::wording::disclosed_site(lines, site));
    }
    for end in &combined.track_ends {
        eprintln!("{}", crate::wording::track_end(lines, end));
    }
    if combined.unchanged {
        eprintln!("{}", crate::wording::unchanged());
    }
    if !args.json {
        println!("{}", args.output.display());
    }
    Ok(())
}

/// The part of the document that is the same whatever the outcome.
fn described(common: &Path, sides: &[(PathBuf, PathBuf)], verified: &[Option<bool>]) -> Value {
    json!({
        "common": common.display().to_string(),
        "sides": sides
            .iter()
            .enumerate()
            .map(|(side, (take, edits))| json!({
                "take": take.display().to_string(),
                "edits": edits.display().to_string(),
                "verified": verified[side],
            }))
            .collect::<Vec<_>>(),
    })
}

/// The refusal object of the document, for the errors that are refusals of a
/// combination rather than failures to read its inputs. `None` for the rest,
/// which are reported as every other command reports an error.
fn refusal(error: &battuta::Error) -> Option<Value> {
    use battuta::Error;
    Some(match error {
        Error::SourceReplayFailed { side, source, .. } => json!({
            "kind": "source_replay_failed",
            "side": side,
            "detail": source.to_string(),
        }),
        Error::SourceEvidenceMismatch { side, .. } => json!({
            "kind": "source_evidence_mismatch",
            "side": side,
        }),
        Error::Conflicts(conflicts) => json!({
            "kind": "conflict",
            "conflicts": conflicts,
        }),
        Error::OrderContradiction(contradictions) => json!({
            "kind": "order_contradiction",
            "contradictions": contradictions,
        }),
        Error::CoreInvalid(source) => json!({
            "kind": "core_invalid",
            "detail": source.to_string(),
        }),
        _ => return None,
    })
}

/// One line per thing a refusal lists, ahead of the refusal itself.
fn refused_lines(error: &battuta::Error) -> Vec<String> {
    match error {
        battuta::Error::Conflicts(conflicts) => {
            conflicts.iter().map(crate::wording::conflict).collect()
        }
        battuta::Error::OrderContradiction(contradictions) => contradictions
            .iter()
            .map(crate::wording::order_contradiction)
            .collect(),
        _ => Vec::new(),
    }
}
