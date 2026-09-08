use std::collections::HashMap;
use std::path::PathBuf;

/// Say what differs between two Takes.
///
/// Only the Piece is compared. Two Takes heard through different Rigs are not
/// different Takes, and no Rig difference is ever reported here.
///
/// Notes are matched in two passes. First by identity — same track, channel,
/// pitch and start Tick — and then, among whatever is left over, by nearest
/// neighbour within the same track and channel, as long as the two starts are
/// no further apart than the tolerance. Whatever is still unmatched is Added or
/// Removed.
///
/// That second pass is the one that says a note *moved* rather than that one
/// note vanished and another appeared, and the tolerance is the whole of its
/// evidence: two notes it pairs have different identities, because a note's
/// pitch and start Tick are part of what names it. So the tolerance is stated
/// on every diff — on stderr, and in the payload under `--json`.
///
/// Which Program each channel is on is compared too, and reported as a state
/// rather than as an event: `program bar 3 beat 1 channel 1 unstated -> 60 (GM
/// french horn)` says this part is on a horn from Bar 3 where it was on nothing
/// the file named. A Take that states no Program and one that states program 0
/// are different Pieces here, although they sound identical on a General MIDI
/// bank. Which Program is selected is in the file and so is the Piece; what it
/// sounds like is the Rig, and is not compared.
///
/// Tempo and pitch bend are compared the same way, as what is in force rather
/// than as the events that set it. A tempo row covers the stretch the two Takes
/// are at different tempos and says the extreme each reaches inside it, so an
/// accelerando written as forty tempo events is one row and not forty. A bend
/// row is the same for one channel, in the raw signed units the file carries:
/// how many semitones a bend is worth is the synthesiser's bend range, which is
/// the Rig and is not compared. A Take that states no tempo and one that states
/// 120 are different Pieces here, as are a channel bent back to the centre and a
/// channel never bent.
/// Same-Tick ordering is compared too, and it is a layer under both of those.
/// A file is a sequence, so *the same Tick* means *no time between* rather than
/// *at once*, and two Takes holding the same events at the same Ticks can still
/// be two Pieces: `rank bar 3 beat 1 channel 1 program before its notes  after
/// follows the rule, before does not` says that one of them meets a Program
/// after the chord it governs, so that chord sounds on the instrument the Take
/// had before.
///
/// Only a pair one event of which governs the other is compared — a Program and
/// the strikes at its Tick, a damper and the releases at its Tick. Two note-ons
/// of a chord are not such a pair: neither governs the other, so the order they
/// are written in is no claim about the music and reordering them is not a
/// difference.
///
/// Where a Take writes such a pair across two tracks, the file states no order
/// at all and the comparison cannot be made. Those sites are reported as
/// `unranked` and are not themselves differences. With unranked content, an
/// otherwise empty comparison reports no determinate differences and states
/// that some content was not compared.
///
/// Conflicting cross-track Program, Controller, Tempo and Bend values are excluded from value comparison
/// until a determinate overwrite. Their intervals and candidate sources appear
/// under `unranked_programs`, `unranked_controllers`, `unranked_tempos` and
/// `unranked_bends` in JSON; Channel-state/strike sites appear separately under
/// `unranked_state_sites`. Determinate differences elsewhere remain reported.
///
/// A matched note reports everything about it that differs, in the fixed order
/// pitch, start, duration, velocity — a note that was both moved and softened
/// reports both. `--json` gives both of its identities, because a note that
/// moved is called one thing in the before Take and another in the after; the
/// human reading is a description of what happened to the music, and says where
/// the note is rather than what it is named.
///
/// The two Takes must count the same number of Ticks to the quarter note. Ticks
/// are the truth here and are never converted, so Takes at two denominations
/// are refused rather than compared as if a Tick meant the same thing in each.
#[derive(clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
    /// The Take to compare from.
    before: PathBuf,

    /// The Take to compare to.
    after: PathBuf,

    /// How far apart in Ticks two notes may start and still be the same note,
    /// moved. Defaults to a sixteenth note — the Take's ticks per quarter note
    /// divided by four, so 120 at the usual 480. 0 matches by identity alone.
    #[arg(long, value_name = "TICKS")]
    tolerance: Option<u32>,

    /// Emit structured output for an agent to consume.
    #[arg(long)]
    json: bool,
}

pub fn run(args: Args) -> battuta::Result<()> {
    let before = battuta::Take::read(&args.before)?;
    let after = battuta::Take::read(&args.after)?;
    let diff = battuta::diff::diff(&before, &after, args.tolerance)?;

    // The library decides *that* the tolerance is part of the answer — it is a
    // field of the `Diff` — and this decides what it says and where it goes.
    // Stderr, so that `--json` on stdout stays one JSON document and a pipeline
    // redirecting the payload cannot drop the disclosure with it. The same
    // reasoning as the Rig disclosure; see ADR-0005.
    eprintln!("tolerance: {} ticks", diff.tolerance_ticks);

    if args.json {
        println!("{}", crate::json(&diff));
        return Ok(());
    }

    // Each Take is placed against its own Bar lines. Two Takes being compared
    // are usually in the same time signature, but nothing here requires it, and
    // a note is where its own Take says it is.
    let before_lines = before.bar_lines();
    let after_lines = after.bar_lines();

    if diff.is_empty() {
        if diff.has_unranked() {
            println!("no determinate differences; some content was not compared");
        } else {
            println!("no differences");
        }
        // And then, if there are any, the sites where the ordering question
        // could not be put. They are not differences — `is_empty` does not
        // consult them — but a reader who has just been told the two Takes
        // agree is owed the places the comparison stopped short of. Suppressing
        // them here would make "no differences" the strongest sentence `mid`
        // prints and the least examined.
        crate::wording::table(&unranked_rows(before_lines, &diff.unranked_sites));
        crate::wording::table(&unranked_state_rows(before_lines, after_lines, &diff));
        return Ok(());
    }

    let mut rows = Vec::new();
    // Tempo above everything, because it is the only row that is about the whole
    // Take rather than a channel or a note. A Take that got faster reframes every
    // row beneath it, orchestration included.
    for difference in &diff.tempos {
        rows.extend(crate::wording::tempo_rows(
            before_lines,
            after_lines,
            difference,
        ));
    }
    // Orchestration first. A channel that changed instrument reframes every note
    // row under it — the same notes on a horn are a different passage — so it is
    // read before them rather than after.
    for difference in &diff.programs {
        rows.push(vec![
            "program".to_string(),
            // Placed against the before Take's Bar lines: the Tick is the moment
            // the two stop agreeing, and it is the Take the reader knows.
            match difference.until {
                Some(_) => crate::wording::span(before_lines, difference.at, difference.until),
                None => crate::wording::at(before_lines, difference.at),
            },
            crate::wording::channel(difference.channel),
            crate::wording::program_difference(difference),
        ]);
    }
    // Controller data next, and above the notes for the reason the orchestration
    // is: what the expression is doing reframes every note row under it. One row
    // per stretch the two Takes disagree over, never one per event — the whole
    // of #13's difficulty, settled by comparing what is in force (ADR-0007).
    for difference in &diff.controllers {
        rows.extend(crate::wording::controller_rows(
            before_lines,
            after_lines,
            difference,
        ));
    }
    // Bends beside the Controller rows, because they are the same kind of fact
    // about the same channel — what the expression is doing — and a reader
    // looking for it should not have to find it in two places.
    for difference in &diff.bends {
        rows.extend(crate::wording::bend_rows(
            before_lines,
            after_lines,
            difference,
        ));
    }
    // Then the ordering, which is the layer under both of those: the same
    // Program at the same Tick, met on the other side of the notes it governs.
    // Above the notes for their reason — a chord that sounds on the instrument
    // the Take names and a chord that does not are the same rows underneath.
    for difference in &diff.rank_disagreements {
        rows.push(vec![
            "rank".to_string(),
            crate::wording::at(before_lines, difference.tick),
            crate::wording::channel(difference.channel),
            difference.pair.named().to_string(),
            crate::wording::rank_difference(difference),
        ]);
    }
    for note in &diff.added {
        rows.push(described("added", after_lines, note));
    }
    for note in &diff.removed {
        rows.push(described("removed", before_lines, note));
    }
    // How many notes share each address, so that a row describing one of a
    // collision can say which. Read from the before Take, because that is the
    // one the row names a note in.
    let crowding = crowding(&before)?;
    for change in &diff.changed {
        let mut row = vec!["changed".to_string()];
        // The note as it was, not as it became: the reader knows the before
        // Take, and each clause says where its field went.
        row.extend(crate::wording::names(
            before_lines,
            &change.before,
            crowding_at(&crowding, &change.before),
        ));
        row.push(stated(change, after_lines));
        rows.push(row);
    }
    // Last, and after the differences rather than among them, because they are
    // not differences.
    rows.extend(unranked_rows(before_lines, &diff.unranked_sites));
    rows.extend(unranked_state_rows(before_lines, after_lines, &diff));
    crate::wording::table(&rows);
    Ok(())
}

fn unranked_state_rows(
    before_lines: Option<battuta::BarLines>,
    after_lines: Option<battuta::BarLines>,
    diff: &battuta::Diff,
) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for interval in &diff.unranked_tempos {
        rows.extend(crate::wording::unranked_comparison(
            before_lines,
            after_lines,
            "tempo".to_string(),
            interval,
            crate::wording::tempo_value,
        ));
    }
    for item in &diff.unranked_programs {
        rows.extend(crate::wording::unranked_comparison(
            before_lines,
            after_lines,
            format!("program channel {}", item.channel),
            &item.interval,
            |value| value.to_string(),
        ));
    }
    for item in &diff.unranked_controllers {
        rows.extend(crate::wording::unranked_comparison(
            before_lines,
            after_lines,
            format!("controller channel {} CC{}", item.channel, item.controller),
            &item.interval,
            |value| value.to_string(),
        ));
    }
    for interval in &diff.unranked_bends {
        for (name, lines, candidates) in [
            ("before", before_lines, &interval.before),
            ("after", after_lines, &interval.after),
        ] {
            if candidates.is_empty() {
                continue;
            }
            let mut row = crate::wording::unranked_bend(
                lines,
                interval.channel,
                interval.from,
                interval.until,
                candidates,
            );
            row.insert(1, name.to_string());
            rows.push(row);
        }
    }
    for site in &diff.unranked_state_sites {
        for (name, lines, present) in [
            ("before", before_lines, site.in_before),
            ("after", after_lines, site.in_after),
        ] {
            if present {
                rows.push(vec![
                    name.to_string(),
                    crate::wording::unranked(lines, &site.site),
                ]);
            }
        }
    }
    rows
}

/// The sites where the ordering question could not be put, as rows.
///
/// Built in one place because they are printed in two — beneath "no
/// differences", and at the foot of a table that has some. The reader meets the
/// same row either way.
fn unranked_rows(
    lines: Option<battuta::BarLines>,
    sites: &[battuta::UnrankedSite],
) -> Vec<Vec<String>> {
    sites
        .iter()
        .map(|site| {
            vec![
                "unranked".to_string(),
                crate::wording::at(lines, site.tick),
                crate::wording::channel(site.channel),
                site.pair.named().to_string(),
                crate::wording::unranked_site(site),
            ]
        })
        .collect()
}

/// How many notes share each address — track, channel, pitch and start Tick —
/// in a Take.
///
/// One is the ordinary answer and means the address names a note. More than one
/// is a collision, and is what `wording::names` needs to know before it can
/// claim to have pointed at a note rather than at a place several notes are.
fn crowding(take: &battuta::Take) -> battuta::Result<HashMap<(usize, u8, u8, u32), usize>> {
    let mut counted = HashMap::new();
    for note in take.notes()? {
        *counted
            .entry((note.track, note.channel, note.pitch, note.start))
            .or_insert(0) += 1;
    }
    Ok(counted)
}

fn crowding_at(counted: &HashMap<(usize, u8, u8, u32), usize>, note: &battuta::Note) -> usize {
    counted
        .get(&(note.track, note.channel, note.pitch, note.start))
        .copied()
        .unwrap_or(1)
}

/// A note that arrived or left, described the way `inspect` lists one — the
/// same cells in the same order, behind what became of it.
fn described(verb: &str, lines: Option<battuta::BarLines>, note: &battuta::Note) -> Vec<String> {
    let mut row = vec![verb.to_string()];
    row.extend(crate::wording::note(lines, note));
    row
}

/// Each change as the two things it went between. Formatting only: which
/// changes there are, and the order they come in, are the library's.
///
/// A `start` change reads as *moved to*, not as one position arrow to another:
/// the row already opens with where the note was, and repeating it would spend
/// the widest clause on the line saying the same thing twice. It is also the one
/// classification the V0.1 spec names in the human's own words — "a moved note
/// reported as moved" — so it is the one that must not come out as `changed`
/// alone.
fn stated(change: &battuta::diff::NoteChange, after_lines: Option<battuta::BarLines>) -> String {
    use battuta::diff::Change;
    change
        .changes
        .iter()
        .map(|&kind| match kind {
            // `transposed to`, not one pitch arrow to another, for the reason
            // `moved to` reads that way: the row already opens with the note as
            // it was, so naming the pitch it came from would say it twice.
            Change::Pitch => format!(
                "transposed to {}",
                crate::wording::pitch(change.after.pitch)
            ),
            Change::Start => format!(
                "moved to {}",
                crate::wording::at(after_lines, change.after.start)
            ),
            Change::Duration => format!(
                "duration {} -> {}",
                change.before.duration, change.after.duration
            ),
            Change::Velocity => format!(
                "velocity {} -> {}",
                change.before.velocity, change.after.velocity
            ),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
