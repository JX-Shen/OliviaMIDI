use std::path::PathBuf;

/// Report what a Take is: format, tracks, PPQ, tempo, time signature and length.
///
/// Length is in Ticks, and so is everything else; the Bar count beside it is the
/// derived view. Bars need one time signature governing the whole Take, so a Take
/// that states none — or states one only part way in, or changes it — is reported
/// without a Bar count, and the length line says which.
///
/// Tempo and time signature are each read at their earliest stated Tick. Where
/// that is not Tick 0 the row says the Take states none until it, and where the
/// Take changes either later the row says how many times; it does not list the
/// changes. Restating what is already in force is not a change. Conflicting
/// cross-track final tempos are indeterminate and retain their candidates and
/// source locations.
///
/// In JSON, `tempo.kind` distinguishes unstated, determinate and indeterminate;
/// `tempo.value` or `tempo.candidates` carries the corresponding reading.
/// `tempo_stated_from` and `time_signature_stated_from` are the Tick of the
/// earliest statement, null when there is none; `tempo_changes` and
/// `time_signature_changes` count the changes after it.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to describe.
    take: PathBuf,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> battuta::Result<()> {
    let info = battuta::Take::read(&args.take)?.info()?;

    if args.json {
        println!("{}", crate::json(&info));
        return Ok(());
    }

    // Through the same column mechanism `inspect` and `diff` use, rather than a
    // second one made of hand-typed spaces. The labels are the widest thing in
    // their column, so this lays out exactly as the spaces did.
    crate::wording::table(&[
        vec!["format".to_string(), info.format.to_string()],
        vec!["tracks".to_string(), info.tracks.to_string()],
        vec!["ppq".to_string(), info.ppq.to_string()],
        vec![
            "tempo".to_string(),
            stated_course(
                crate::wording::state_reading(
                    None,
                    &info.tempo,
                    crate::wording::tempo_value,
                    crate::wording::tempo_value,
                ),
                info.tempo_stated_from,
                info.tempo_changes,
            ),
        ],
        vec![
            "time signature".to_string(),
            match info.time_signature {
                Some(ts) => stated_course(
                    format!("{}/{}", ts.numerator, ts.denominator),
                    info.time_signature_stated_from,
                    info.time_signature_changes,
                ),
                None => "unstated".to_string(),
            },
        ],
        vec![
            "length".to_string(),
            match info.length_bars {
                Some(bars) => format!("{} ticks ({bars} bars)", info.length_ticks),
                None => format!("{} ticks ({})", info.length_ticks, no_bars(info.no_bars)),
            },
        ],
    ]);
    Ok(())
}

/// Why a Take has no Bar count, in terms of the time signature row above it.
fn no_bars(reason: Option<battuta::NoBars>) -> &'static str {
    use battuta::NoBars;
    match reason {
        Some(NoBars::NoTimeSignature) => "bars need one stated time signature",
        Some(NoBars::TimeSignatureStartsLate) => "bars need a time signature stated at tick 0",
        Some(NoBars::TimeSignatureChanges) => "bars need one time signature throughout",
        Some(NoBars::TimeSignatureWithNoBeats | NoBars::BarNotWholeTicks) | None => {
            "bars cannot be derived; mid inspect --bars says why"
        }
    }
}

/// The earliest stated reading, preceded by where it is stated when that is not
/// the opening, and followed by how many times it changes after — #65.
fn stated_course(first: String, stated_from: Option<u32>, changes: usize) -> String {
    let first = match stated_from {
        Some(tick) if tick > 0 => format!("unstated until tick {tick}, then {first}"),
        _ => first,
    };
    match changes {
        0 => first,
        1 => format!("{first}, then 1 change"),
        n => format!("{first}, then {n} changes"),
    }
}
