use std::path::PathBuf;

/// Report what a Take is: format, tracks, PPQ, tempo, time signature and length.
///
/// Length is in Ticks, and so is everything else; the Bar count beside it is the
/// derived view. Bars need one time signature governing the whole Take, so a Take
/// that states none — or states one only part way in, or changes it — is reported
/// without a Bar count. `mid inspect --bars` says which of those it is.
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
            match info.tempo {
                Some(tempo) => format!(
                    "{} bpm ({} us per quarter)",
                    tempo.bpm, tempo.micros_per_quarter
                ),
                None => "unstated".to_string(),
            },
        ],
        vec![
            "time signature".to_string(),
            match info.time_signature {
                Some(ts) => format!("{}/{}", ts.numerator, ts.denominator),
                None => "unstated".to_string(),
            },
        ],
        vec![
            "length".to_string(),
            match info.length_bars {
                Some(bars) => format!("{} ticks ({bars} bars)", info.length_ticks),
                None => format!(
                    "{} ticks (bars need one stated time signature)",
                    info.length_ticks
                ),
            },
        ],
    ]);
    Ok(())
}
