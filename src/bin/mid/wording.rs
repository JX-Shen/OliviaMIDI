//! How `mid` writes down where a note is and what it is called.
//!
//! The library decides which Bar a Tick falls in and which note a pitch is; this
//! decides that they read as `bar 5 beat 1` and `F#4`. The same cut as ADR-0009,
//! applied twice more: the fact is `battuta`'s, the sentence is `mid`'s.
//!
//! `names` says which note, and `note` is that plus how hard it was struck and
//! how long it lasts. Every line `inspect` and `diff` print is built out of one
//! or the other, so the two commands cannot drift into two vocabularies for the
//! same thing. They surround it differently — a listing ends with the identity
//! an Edit Set copies, a diff opens with what became of the note — and a diff
//! row describing a change stops at `names`, because the fields it goes on to
//! talk about are the ones `note` would have printed.
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

/// The cells that *name* a note: where it is, whose part it is on, and what it
/// is called. Everything a musician would say to point at one note and no other.
///
/// These are exactly the components of the identity that have a musical
/// rendering. Track and start Tick and pitch do; channel and occurrence index do
/// not. Velocity and duration are not part of an identity at all, which is why
/// they are not here — they are properties of the note a diff talks *about*,
/// not part of pointing at it.
///
/// `among` is how many notes share this one's address in the Take being
/// described. Where it is more than one the three cells above cannot name a
/// note, because the notes collide on every one of them, and the occurrence
/// index is all that separates them. It has no musical name, so it is spelled
/// the way the identity spells it: `E4 n1` is the note `inspect` lists as
/// `t1:c0:p64:s960:n1`.
///
/// The channel is never here. It is a fact a human reading music does not use —
/// the part is the track — and two notes differing only in channel do not
/// collide, so nothing becomes ambiguous by leaving it out.
pub fn names(lines: Option<BarLines>, note: &Note, among: usize) -> Vec<String> {
    let mut called = pitch(note.pitch);
    if among > 1 {
        called.push_str(&format!(" n{}", note.occurrence));
    }
    vec![
        at(lines, note.start),
        format!("track {}", note.track),
        called,
    ]
}

/// One note, described: what names it, then how hard it was struck and how long
/// it lasts.
///
/// A listing never needs the disambiguator `names` can add, because `inspect`
/// ends every line with the identity itself, and `diff` says of an added or
/// removed note how loud and how long it is — which is what tells two notes at
/// one address apart when they differ at all.
pub fn note(lines: Option<BarLines>, note: &Note) -> Vec<String> {
    let mut row = names(lines, note, 1);
    row.push(format!("velocity {}", note.velocity));
    row.push(format!("duration {}", note.duration));
    row
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
