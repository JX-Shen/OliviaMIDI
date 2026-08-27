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

    println!("format          {}", info.format);
    println!("tracks          {}", info.tracks);
    println!("ppq             {}", info.ppq);
    match info.tempo {
        Some(tempo) => println!(
            "tempo           {} bpm ({} us per quarter)",
            tempo.bpm, tempo.micros_per_quarter
        ),
        None => println!("tempo           unstated"),
    }
    match info.time_signature {
        Some(ts) => println!("time signature  {}/{}", ts.numerator, ts.denominator),
        None => println!("time signature  unstated"),
    }
    match info.length_bars {
        Some(bars) => println!("length          {} ticks ({bars} bars)", info.length_ticks),
        None => println!(
            "length          {} ticks (bars need one stated time signature)",
            info.length_ticks
        ),
    }
    Ok(())
}
