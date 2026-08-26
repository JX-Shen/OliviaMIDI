use std::path::PathBuf;

/// Report what a Take is: format, tracks, PPQ, tempo, metre and length.
///
/// Length is in Ticks, and so is everything else. Bars are not reported yet, so
/// work them out from the PPQ and the time signature: in 3/4 at 480 PPQ a Bar is
/// 1440 Ticks.
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
    println!("length          {} ticks", info.length_ticks);
    Ok(())
}
