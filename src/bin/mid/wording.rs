//! How `mid` writes down where a note is and what it is called.
//!
//! The library decides which Bar a Tick falls in and which note a pitch is; this
//! decides that they read as `bar 5 beat 1` and `F#4`. The same cut as ADR-0009,
//! applied twice more: the fact is `battuta`'s, the sentence is `mid`'s.
//!
//! `note` is the one description of a note, and `inspect` and `diff` both build
//! their lines out of it. They surround it differently — a listing ends with the
//! identity an Edit Set copies, a diff begins with what became of the note — but
//! the note itself is described in one place, in one order, so that the two
//! commands cannot drift into two vocabularies for the same thing.
//!
//! Not `render`: `CHARTER.md` reserves `mid render` for turning a Take into
//! audio, and a module in this directory taking that name would be sitting in
//! the seat its implementation will want. `wording` is ADR-0009's own word for
//! what this does.

use battuta::{BarLines, Note};

/// Where a Tick is: musically if the Take says enough to tell, and in Ticks
/// otherwise.
///
/// The fallback is not a lesser answer. Ticks are the truth and a Bar is the
/// derived view, so a Take that states no time signature is reported in the
/// units it actually has. What it must not do is refuse — see `Take::bar_lines`.
///
/// Not `position`, although a Position is what it usually returns. `CONTEXT.md`
/// admits **Position** past **Tick**'s avoid list on the grounds that a Position
/// is a reading of a Tick and never a Tick under another word — and this is the
/// one function that would have made that false, by answering `tick 8640` to a
/// question spelled `position`. `at` is true of both answers and claims neither.
pub fn at(lines: Option<BarLines>, tick: u32) -> String {
    match lines.map(|lines| lines.position_of(tick)) {
        Some(at) if at.ticks_into_beat == 0 => format!("bar {} beat {}", at.bar, at.beat),
        Some(at) => format!("bar {} beat {}+{}", at.bar, at.beat, at.ticks_into_beat),
        None => format!("tick {tick}"),
    }
}

/// A pitch by name: `F#4`, `A2`, `C-1`.
///
/// ASCII, not `♯`. This output is meant to be pasted — into an Edit Set beside
/// the identity on the same line, into a message to an agent, into an issue —
/// and a sharp sign survives fewer of those journeys than it is worth.
pub fn pitch(pitch: u8) -> String {
    let name = battuta::pitch_name(pitch);
    let accidental = if name.sharp { "#" } else { "" };
    format!("{}{}{}", name.letter, accidental, name.octave)
}

/// One note, described: where it is, whose part it is on, what it is called, how
/// hard it was struck and how long it lasts.
///
/// The channel is not here. It is in the identity on the same line — `t2:c1:` —
/// and a column repeating it would be nine characters per line spent on the one
/// fact a human reading music does not use: the part is the track. Nothing
/// becomes ambiguous, because two notes differing only in channel still differ
/// in their identities.
pub fn note(lines: Option<BarLines>, note: &Note) -> Vec<String> {
    vec![
        at(lines, note.start),
        format!("track {}", note.track),
        pitch(note.pitch),
        format!("velocity {}", note.velocity),
        format!("duration {}", note.duration),
    ]
}

/// Print rows as columns, each as wide as the widest thing in it.
///
/// A row's last cell sets no width, and is never padded. That is what lets rows
/// of different shapes share a table: `diff` describes a changed note in one
/// trailing sentence where it describes an added one in four aligned columns,
/// and the sentence does not stretch the columns the other rows line up on.
pub fn table(rows: &[Vec<String>]) {
    let mut widths: Vec<usize> = Vec::new();
    for row in rows {
        for (column, cell) in row.iter().enumerate().take(row.len().saturating_sub(1)) {
            if widths.len() <= column {
                widths.resize(column + 1, 0);
            }
            widths[column] = widths[column].max(cell.chars().count());
        }
    }

    for row in rows {
        let mut line = String::new();
        for (column, cell) in row.iter().enumerate() {
            if column > 0 {
                line.push_str("  ");
            }
            line.push_str(cell);
            if column + 1 < row.len() {
                let width = widths.get(column).copied().unwrap_or(0);
                for _ in cell.chars().count()..width {
                    line.push(' ');
                }
            }
        }
        println!("{line}");
    }
}
