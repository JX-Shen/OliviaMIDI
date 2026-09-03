//! How `mid` writes down where a note is and what it is called.
//!
//! The library decides which Bar a Tick falls in and which note a pitch is; this
//! decides that they read as `bar 5 beat 1` and `F#4`. The same cut as ADR-0005,
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
//! the seat its implementation will want. `wording` is ADR-0005's own word for
//! what this does.

use battuta::{
    BarLines, Controller, ControllerDifference, ControllerSide, Note, Program, ProgramDifference,
    StatedController, StatedProgram,
};

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
/// The channel is never on a note's line. It is a fact a human reading music
/// does not use — the part is the track — and two notes differing only in
/// channel do not collide, so nothing becomes ambiguous by leaving it out.
///
/// That argument is about notes and does not reach a Program, which is held by
/// the channel and has no other subject: `program 40` without a channel is a
/// sentence missing the thing it is about, and the track cannot stand in, since
/// three tracks may write one channel and one track may write three. So
/// `program` below names the channel and this does not, and the two are the same
/// rule — say what is needed to point at the thing — rather than two.
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

/// Which channel, as the format counts them.
///
/// Counted from zero, because that is the number in the file and the number in
/// every identity `inspect` prints. General Midi's documentation counts the same
/// channels from one, so its percussion channel is 10 there and 9 here; where
/// that matters it is said in words rather than by renumbering, since a number
/// that disagreed with the identity beside it would be worse than a number a
/// reader has to shift.
pub fn channel(channel: u8) -> String {
    format!("channel {channel}")
}

/// Which Program, with what General Midi calls it.
///
/// The label is load-bearing. A pitch name is a claim about the file's own
/// semantics — pitch 66 is F#4 in every Take — but a program name is a claim
/// about *which bank is loaded*, and the bank is the Rig. An unlabelled `violin`
/// would be a Rig fact printed by a command that reports only the Piece, which
/// is the confusion `CHARTER.md` opens by refusing. `GM violin` says whose word
/// it is; `program 40` is what the Piece actually says.
///
/// No name on the drum channel, where a Program selects a kit rather than an
/// instrument: program 40 there is not a violin, and General Midi's melodic list
/// is not a list of what it is. The number stands alone rather than being
/// glossed wrongly.
///
/// `None` is a Take that states no Program for this channel, which is never
/// printed as program 0 — see #12.
pub fn program(on_channel: u8, program: Option<u8>) -> String {
    match program {
        None => "unstated".to_string(),
        Some(program) => format!("program {}", numbered(on_channel, program)),
    }
}

/// A Program as a number and, where there is one to have, its General Midi
/// name: `40 (GM violin)`.
///
/// Without the word `program` in front, for the rows that have already said it.
fn numbered(on_channel: u8, program: u8) -> String {
    match battuta::gm_name(program) {
        Some(name) if on_channel != battuta::GM_PERCUSSION_CHANNEL => {
            format!("{program} (GM {name})")
        }
        _ => program.to_string(),
    }
}

/// The two Programs a channel is on in two Takes: `unstated -> 60 (GM french
/// horn)`.
///
/// Both sides in the same shape, including `unstated`, because that side is a
/// difference like any other and a blank there would read as nothing having been
/// said rather than as the Take saying nothing.
pub fn program_difference(difference: &ProgramDifference) -> String {
    let side = |program: Option<u8>| match program {
        None => "unstated".to_string(),
        Some(program) => numbered(difference.channel, program),
    };
    format!("{} -> {}", side(difference.before), side(difference.after))
}

/// What one channel holds for one Controller: the state a passage begins in.
///
/// `CC11` rather than `controller 11`: `CC` is MIDI's own shorthand, and it is
/// where the attribution for any name printed beside it sits.
/// Where the passage's highest value falls is passed in rather than read off
/// `held`, because whether it is worth a cell is the consumer's call: a passage
/// that states nothing for this Controller has a peak equal to the value beside
/// it, and printing it would be saying one fact twice.
pub fn controller(
    lines: Option<BarLines>,
    held: &Controller,
    peak: Option<(u8, u32)>,
) -> Vec<String> {
    let mut row = vec![
        channel(held.channel),
        controller_number(held.controller),
        match held.value {
            None => "unstated".to_string(),
            Some(value) => value.to_string(),
        },
    ];
    if let Some((peak, tick)) = peak {
        row.push(format!("peak {peak} at {}", at(lines, tick)));
    }
    row
}

/// One place the passage states a Controller, as an event: where it happens,
/// which track says it, and what it says.
///
/// The track is here where it is absent from the state above, for the reason it
/// is on a `StatedProgram` row: this row describes an event somebody can go and
/// change, and it is what a reader copies a `set_controller` address out of.
/// A Controller as MIDI's shorthand and, where its table names one, that name:
/// `CC64 (damper pedal on/off (sustain))`.
///
/// The `CC` prefix is the attribution. A name here depends on nothing but MIDI —
/// the way `pitch 66` is `F#4` — so unlike `GM violin` it needs no label saying
/// whose word it is, and `pitch_name` carries none either. Where the table names
/// nothing there is no parenthesis: *undefined* is the table saying nothing, and
/// a bare number says that faithfully.
pub fn controller_number(controller: u8) -> String {
    match battuta::spec_name(controller) {
        Some(name) => format!("CC{controller} ({})", trimmed(name)),
        None => format!("CC{controller}"),
    }
}

/// The specification's name with the two clauses a table cell has no use for
/// taken out, and nothing else touched.
///
/// Deleting some of a quotation is not the same as replacing a word of it, and
/// only these two are deleted:
///
/// - a cross reference to another document — `sound controller 6 (default: decay
///   time - see mma rp-021)` loses ` - see mma rp-021`. It points at a paper the
///   reader does not have and says nothing about the control.
/// - a historical alias — `channel volume (formerly main volume)` loses the
///   whole parenthesis and `lsb for control 7 (channel volume, formerly main
///   volume)` loses only the clause, because the rest of that parenthesis is the
///   name. What a control used to be called is not what it is.
///
/// What is deliberately *kept* is everything that says what the control does:
/// `on/off`, which is how the specification says a control is a switch, and
/// `default: brightness`, which is the whole of why CC74 is used as brightness.
/// Trimming those would leave a name that had stopped being informative rather
/// than one that had stopped being long.
///
/// This is `mid`'s decision, not `battuta`'s: the library hands over the
/// quotation whole and a consumer with room for all of it may print all of it
/// (ADR-0005).
fn trimmed(name: &str) -> String {
    let mut name = name.to_string();
    // The cross reference runs to the end of whatever clause holds it.
    if let Some(start) = name.find(" - see ") {
        let end = name[start..]
            .find(')')
            .map(|offset| start + offset)
            .unwrap_or(name.len());
        name.replace_range(start..end, "");
    }
    // The alias, as a whole parenthesis where it is the whole of one and as a
    // clause where it is not.
    if let Some(start) = name.find(", formerly ") {
        let end = name[start..]
            .find(')')
            .map(|offset| start + offset)
            .unwrap_or(name.len());
        name.replace_range(start..end, "");
    } else if let Some(start) = name.find(" (formerly ") {
        let end = name[start..]
            .find(')')
            .map(|offset| start + offset + 1)
            .unwrap_or(name.len());
        name.replace_range(start..end, "");
    }
    name
}

pub fn stated_controller(lines: Option<BarLines>, stated: &StatedController) -> Vec<String> {
    vec![
        at(lines, stated.tick),
        format!("track {}", stated.track),
        channel(stated.channel),
        controller_number(stated.controller),
        stated.value.to_string(),
    ]
}

/// The stretch a controller difference covers: `bar 6 beat 1 until bar 7 beat
/// 1`.
///
/// `until` names the Tick the two Takes agree again, so the stretch reads as
/// half-open — up to that Bar and Beat, not through it. A span that never closes
/// says so in words rather than borrowing the last Tick either Take happens to
/// hold, which would read as a moment the two came back together.
pub fn controller_span(lines: Option<BarLines>, difference: &ControllerDifference) -> String {
    match difference.until {
        None => format!("{} onwards", at(lines, difference.from)),
        Some(until) => format!("{} until {}", at(lines, difference.from), at(lines, until)),
    }
}

/// What the two Takes hold for a Controller across the span: `70 -> 100`.
///
/// Each side is what it holds where the span begins, which is the fact a
/// listener starting there hears. Where a side reaches something higher inside
/// the span it says so — `70 (peak 85 at bar 6 beat 2) -> 100` — and where it
/// does not, the clause is left off rather than restating the number beside it.
///
/// `unstated` on either side, in the same shape as the numbers, for the reason
/// `program_difference` prints it: a blank would read as nothing having been
/// said rather than as the Take saying nothing.
pub fn controller_difference(
    before_lines: Option<BarLines>,
    after_lines: Option<BarLines>,
    difference: &ControllerDifference,
) -> String {
    let side = |lines, side: &ControllerSide| match side.at_start {
        None => "unstated".to_string(),
        Some(value) => match (side.peak, side.peak_at) {
            (Some(peak), Some(peak_at)) if peak > value => {
                format!("{value} (peak {peak} at {})", at(lines, peak_at))
            }
            _ => value.to_string(),
        },
    };
    format!(
        "{} -> {}",
        side(before_lines, &difference.before),
        side(after_lines, &difference.after)
    )
}

/// What one channel is on: the state a passage begins in.
pub fn programs(state: &Program) -> Vec<String> {
    vec![
        channel(state.channel),
        program(state.channel, state.program),
    ]
}

/// One place the Take states a Program, as an event: where it happens, which
/// track says it, and what it says.
///
/// The track is here where it is absent from the state above, and for the reason
/// the state has no use for it: this row describes an event somebody can go and
/// change, and `set_program` names the track it lands in. The row is what a
/// reader copies that argument out of.
pub fn stated_program(lines: Option<BarLines>, stated: &StatedProgram) -> Vec<String> {
    vec![
        at(lines, stated.tick),
        format!("track {}", stated.track),
        channel(stated.channel),
        program(stated.channel, Some(stated.program)),
    ]
}
