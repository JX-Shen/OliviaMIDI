use std::path::PathBuf;

/// List every note in a Take, with the identity that names it.
///
/// An identity is derived from the note's content — track, channel, pitch, start
/// Tick, and an occurrence index for notes that collide on all four. It is what
/// an Edit Set refers to, and it survives an Edit that does not touch the note.
///
/// `--bars 5:8` narrows it to a passage: Bars are 1-indexed and the range
/// includes both ends, so that is four Bars. A note belongs to the Bar it
/// *starts* in, even when it sustains across the Bar line.
///
/// Above the notes it says which Program each channel of the passage is on — the
/// state the passage *begins* in, including one the Take set many Bars earlier —
/// and then every place the passage itself states another. A channel the Take
/// says nothing about is reported as stating none, never as being on program 0:
/// General MIDI's default makes those two sound identical and they are different
/// Pieces. The name beside a program number is General MIDI's, and is labelled
/// so, because which instrument that number *sounds* like depends on the bank,
/// and the bank is the Rig.
///
/// The listing is in the order the music happens, so that reading down it reads
/// down the passage: a chord's notes are adjacent and the Bar numbers only ever
/// go forwards. `--json` keeps the Take's own order instead — track by track,
/// note-on by note-on — because that order is what fixes the occurrence index in
/// every identity, and an agent consuming the payload is entitled to see it.
/// Every line carries its identity, so the two orders name the same notes.
///
/// Bar lines are derived from the time signature the Take states, which in an
/// ordinary export lives on a different track than the notes. One time signature
/// has to govern the whole Take: one that states none is refused rather than
/// assumed to be in 4/4, and so is one that states none until part way in, or
/// changes time signature part way through. The final Bar counts as a Bar even
/// when the Take stops part way inside it.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to list.
    take: PathBuf,

    /// The passage to list, as FIRST:LAST — 1-indexed, both ends included.
    #[arg(long, value_name = "FIRST:LAST")]
    bars: Option<battuta::BarRange>,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

/// What `inspect --json` hands an agent.
///
/// An object rather than the bare array of notes this printed before 0.1.1: a
/// Program belongs to a channel, not to a note, so there is nowhere in an array
/// of notes to put one that would not be a lie about what holds it.
///
/// No General MIDI name here, and no Controller spec name either, for the reason
/// #7 kept pitch names out of the payload: a name is a gloss for a human reading
/// a terminal, and an agent is entitled to one spelling of a fact. The number is
/// the fact.
///
/// A Controller's state carries `null` where the Take set nothing, which is what
/// `unstated` is on the terminal. An agent deciding whether to write a
/// `set_controller` needs 0 and nothing apart for the reason a human does.
#[derive(serde::Serialize)]
struct Listing {
    programs: Vec<battuta::Program>,
    stated_programs: Vec<battuta::StatedProgram>,
    controllers: Vec<battuta::Controller>,
    stated_controllers: Vec<battuta::StatedController>,
    notes: Vec<battuta::Note>,
}

pub fn run(args: Args) -> battuta::Result<()> {
    let take = battuta::Take::read(&args.take)?;
    let notes = take.notes_in(args.bars)?;
    let programs = take.programs_in(args.bars)?;
    let controllers = take.controllers_in(args.bars)?;

    if args.json {
        println!(
            "{}",
            crate::json(&Listing {
                programs: programs.programs,
                stated_programs: programs.stated,
                controllers: controllers.controllers,
                stated_controllers: controllers.stated,
                notes,
            })
        );
        return Ok(());
    }

    // Read once for the whole listing rather than per row: every Tick of a Take
    // is placed against the same Bar lines.
    let lines = take.bar_lines();

    // Two blocks above the notes, a blank line under each: what each channel is
    // on, and where the passage states another. Separate tables rather than one,
    // because a state row and an event row have different shapes, and sharing a
    // table would line the channel of one up under the Position of the other.
    //
    // A Take that states no Program at all gets one line saying so rather than a
    // row per channel saying it separately. Sixteen ways of saying nothing is not
    // more informative than one, and most Takes — `fixtures/olivia.mid` included
    // — are this case. It is still said, because the notes will sound on whatever
    // the bank defaults to, and that is the fact #12 exists to stop hiding;
    // `unstated` per channel appears where it tells one channel from another.
    //
    // A passage holding no channels at all has nothing to say nothing about, and
    // `no notes` below is the whole of the answer.
    // Both blocks speak, or neither does. A passage with no channels at all has
    // nothing to say nothing about — `no notes` below is the whole of the answer
    // — and a Controller line there would be answering about channels the
    // Program block had just declined to mention.
    let has_channels = !programs.programs.is_empty() || !programs.stated.is_empty();
    if has_channels {
        if programs.stated.is_empty()
            && programs
                .programs
                .iter()
                .all(|state| state.program.is_none())
        {
            println!("no programs stated");
        } else {
            let rows: Vec<Vec<String>> = programs
                .programs
                .iter()
                .map(crate::wording::programs)
                .collect();
            crate::wording::table(&rows);
        }
        if !programs.stated.is_empty() {
            println!();
            let rows: Vec<Vec<String>> = programs
                .stated
                .iter()
                .map(|stated| crate::wording::stated_program(lines, stated))
                .collect();
            crate::wording::table(&rows);
        }
        println!();
    }

    // What each channel holds for a Controller, under the Programs and above the
    // notes: it is channel state like a Program, and it reframes the notes the
    // same way — the same line played with the expression already at 100 is a
    // different passage. Its own table, for the reason the two above are two.
    // A passage holding nothing gets one line saying so rather than a row per
    // channel: a channel holds one Program and `unstated` beside it is a fact
    // about that channel, where a channel says nothing about a hundred and
    // twenty Controllers and printing that many non-statements informs nobody.
    // It is still said, because a reader told nothing cannot tell a Take with no
    // expression shaping from a tool that does not look — the invisibility #11
    // is about.
    if has_channels {
        if controllers.controllers.is_empty() {
            println!("no controllers stated");
        } else {
            let rows: Vec<Vec<String>> = controllers
                .controllers
                .iter()
                .map(|held| {
                    // The peak earns a cell only where the passage states
                    // something for this Controller. Where it states nothing the
                    // peak is the value already beside it, at the passage's own
                    // first Tick, and the cell would be one fact printed twice.
                    let stated = controllers.stated.iter().any(|stated| {
                        stated.channel == held.channel && stated.controller == held.controller
                    });
                    crate::wording::controller(
                        lines,
                        held,
                        stated.then_some((held.peak, held.peak_at)),
                    )
                })
                .collect();
            crate::wording::table(&rows);
        }
        if !controllers.stated.is_empty() {
            println!();
            let rows: Vec<Vec<String>> = controllers
                .stated
                .iter()
                .map(|stated| crate::wording::stated_controller(lines, stated))
                .collect();
            crate::wording::table(&rows);
        }
        println!();
    }

    if notes.is_empty() {
        println!("no notes");
        return Ok(());
    }

    // A stable sort, so notes sharing a Tick keep the Take's own order among
    // themselves and two runs on one Take agree.
    let mut notes = notes;
    notes.sort_by_key(|note| note.start);

    let rows: Vec<Vec<String>> = notes
        .iter()
        .map(|note| {
            let mut row = crate::wording::note(lines, note);
            // Last, and so never padded: it is the one thing on the line meant
            // to be copied rather than read.
            row.push(note.id.to_string());
            row
        })
        .collect();
    crate::wording::table(&rows);
    Ok(())
}
