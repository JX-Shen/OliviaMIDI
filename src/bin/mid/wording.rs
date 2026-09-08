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
    BarLines, BendDifference, BendSide, Controller, ControllerDifference, ControllerSide, Note,
    Program, ProgramDifference, StatedController, StatedProgram, TempoDifference, TempoSide,
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

/// Which of the two Takes puts a ranked pair in the order the rule calls
/// correct.
///
/// Always one of them and never both: a disagreement is reported only where the
/// two readings differ, and each reading is one of the two orders. So the clause
/// can say which side is which rather than leaving the reader to work it out
/// from a pair of booleans.
///
/// It says *the rule* and not *right*. ADR-0008's rule is what a Take this
/// project writes obeys; a Take that arrived the other way round is the author's
/// (ADR-0003), and `mid` reports the disagreement rather than grading it.
pub fn rank_difference(difference: &battuta::RankDisagreement) -> String {
    if difference.before_is_correct {
        "before follows the rule, after does not".to_string()
    } else {
        "after follows the rule, before does not".to_string()
    }
}

/// Which of the two Takes leaves a site with no order to read.
pub fn unranked_site(site: &battuta::UnrankedSite) -> String {
    match (site.in_before, site.in_after) {
        (true, true) => "neither states an order between the tracks".to_string(),
        (true, false) => "before states no order between the tracks".to_string(),
        _ => "after states no order between the tracks".to_string(),
    }
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
/// Show a complete peak when the passage states another value; always disclose
/// incomplete coverage — #42.
pub fn controller(lines: Option<BarLines>, held: &Controller, inside: bool) -> Vec<String> {
    let mut row = vec![
        channel(held.channel),
        controller_number(held.controller),
        state_reading(&held.value, |value| value.to_string()),
    ];
    match held.peak {
        battuta::ControllerPeak::Complete { value, at: tick } if inside => {
            row.push(format!("peak {value} at {}", at(lines, tick)))
        }
        battuta::ControllerPeak::Incomplete => {
            row.push("peak not summarised across unranked intervals".to_string())
        }
        _ => {}
    }
    row
}

pub fn state_reading<T>(reading: &battuta::Reading<T>, format: impl Fn(&T) -> String) -> String {
    match reading {
        battuta::Reading::Unstated => "unstated".to_string(),
        battuta::Reading::Determinate { value } => format(value),
        battuta::Reading::Indeterminate { candidates } => {
            format!("indeterminate ({})", state_candidates(candidates, format))
        }
    }
}

pub fn state_candidates<T>(
    candidates: &[battuta::Candidate<T>],
    format: impl Fn(&T) -> String,
) -> String {
    candidates
        .iter()
        .map(|candidate| {
            format!(
                "{} from track {} at tick {}",
                format(&candidate.value),
                candidate.track,
                candidate.tick
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn tempo_value(tempo: &battuta::Tempo) -> String {
    format!(
        "{} bpm ({} us per quarter)",
        tempo.bpm, tempo.micros_per_quarter
    )
}

pub fn indeterminate_span<T>(
    lines: Option<BarLines>,
    address: String,
    interval: &battuta::UnrankedSpan<T>,
    format: impl Fn(&T) -> String,
) -> Vec<String> {
    vec![
        address,
        span(lines, interval.from, interval.until),
        format!(
            "indeterminate: {}",
            state_candidates(&interval.candidates, format)
        ),
    ]
}

pub fn unranked_comparison<T>(
    before_lines: Option<BarLines>,
    after_lines: Option<BarLines>,
    address: String,
    interval: &battuta::UnrankedComparison<T>,
    format: impl Fn(&T) -> String,
) -> Vec<Vec<String>> {
    [
        ("before", before_lines, &interval.before),
        ("after", after_lines, &interval.after),
    ]
    .into_iter()
    .filter(|(_, _, candidates)| !candidates.is_empty())
    .map(|(side, lines, candidates)| {
        vec![
            format!("unranked {address}"),
            side.to_string(),
            span(lines, interval.from, interval.until),
            format!("not compared: {}", state_candidates(candidates, &format)),
        ]
    })
    .collect()
}

/// How far a channel is bent where the passage begins, and how far the passage
/// takes it: `channel 0  bend  -2000 (down to -6000 at bar 2 beat 1)`.
///
/// `controller`'s row with the Controller's number left out, because a bend has
/// none: nothing sub-addresses it, so `bend` is the whole of what it is. The
/// word is still in the row rather than left implicit, so that a reader
/// scanning the column meets the same kind of word the Controller rows put
/// there.
///
/// Two excursions where a Controller has one. A Controller runs from nought
/// upwards; a bend is signed about a centre, so the two directions are two
/// facts and either alone can hide the other. Each is printed only where it
/// went somewhere `value` does not already say — a bend that only rose says
/// only that it rose.
///
/// The raw signed number and never semitones. How many semitones a bend is
/// worth is the synthesiser's bend range, which is not in the file, so naming
/// semitones would print a Rig fact from a command that reports the Piece.
pub fn bend(lines: Option<BarLines>, held: &battuta::Bend, inside: bool) -> Vec<String> {
    let starting = match &held.value {
        battuta::Reading::Determinate { value } => Some(*value),
        _ => None,
    };
    let mut cell = match &held.value {
        battuta::Reading::Unstated => "unstated".to_string(),
        battuta::Reading::Determinate { value } => value.to_string(),
        battuta::Reading::Indeterminate { candidates } => {
            format!("indeterminate ({})", bend_candidates(candidates))
        }
    };
    match held.extremes {
        battuta::BendExtremes::Incomplete => {
            cell.push_str("; extremes not summarised across unranked intervals")
        }
        battuta::BendExtremes::Complete {
            furthest_down,
            furthest_down_at,
            furthest_up,
            furthest_up_at,
        } if inside => {
            let mut went = Vec::new();
            if Some(furthest_down) != starting {
                went.push(format!(
                    "down to {} at {}",
                    furthest_down,
                    at(lines, furthest_down_at)
                ));
            }
            if Some(furthest_up) != starting {
                went.push(format!(
                    "up to {} at {}",
                    furthest_up,
                    at(lines, furthest_up_at)
                ));
            }
            if !went.is_empty() {
                cell = format!("{cell} ({})", went.join(", "));
            }
        }
        _ => {}
    }
    vec![channel(held.channel), "bend".to_string(), cell]
}

pub fn bend_candidates(candidates: &[battuta::Candidate<i16>]) -> String {
    state_candidates(candidates, |value| value.to_string())
}

pub fn unranked_bend(
    lines: Option<BarLines>,
    channel_number: u8,
    from: u32,
    until: Option<u32>,
    candidates: &[battuta::Candidate<i16>],
) -> Vec<String> {
    vec![
        "unranked bend".to_string(),
        span(lines, from, until),
        channel(channel_number),
        format!("not compared: {}", bend_candidates(candidates)),
    ]
}

/// One place the passage bends a channel, as an event: where it happens, which
/// track says it, and how far.
///
/// `stated_controller`'s row, and the track is here for its reason: this
/// describes an event somebody can go and look at, where the state above
/// describes what the channel is.
pub fn stated_bend(lines: Option<BarLines>, stated: &battuta::StatedBend) -> Vec<String> {
    vec![
        at(lines, stated.tick),
        format!("track {}", stated.track),
        channel(stated.channel),
        "bend".to_string(),
        stated.value.to_string(),
    ]
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

/// The rows one controller difference occupies: `controller`, the span, the
/// channel, the Controller, and what each Take holds for it across the span.
///
/// The same rows a tempo and a bend take, on the same one-row-or-three
/// threshold. `span`'s doc comment gives the reason the three share one
/// sentence for the stretch — a reader who has learnt to read one row has
/// learnt to read all three — and it holds for the sides as well as for the
/// span. Two readers' worth of habit for one kind of fact is what #45 is about.
///
/// One excursion where a tempo and a bend have two, and that is the right
/// number rather than a shortfall: a Controller runs from nought upwards and
/// has no meaningful low extreme, which is ADR-0007's own argument and the
/// reason the other two needed a second field. `ControllerSide` already carries
/// everything `Side` asks for, so `--json` comes out byte for byte as it was.
///
/// `unstated` on either side, in the same shape as the numbers, for the reason
/// `program_difference` prints it: a blank would read as nothing having been
/// said rather than as the Take saying nothing. It is not a reason to stop
/// reading — an `unstated` side still says the peak it reaches inside the span
/// and where the span leaves it. That silence is the fault #32 inherited from
/// this row and fixed one state over.
pub fn controller_rows(
    before_lines: Option<BarLines>,
    after_lines: Option<BarLines>,
    difference: &ControllerDifference,
) -> Vec<Vec<String>> {
    let side = |lines, side: &ControllerSide| Side {
        at_start: match side.at_start {
            None => "unstated".to_string(),
            Some(at_start) => at_start.to_string(),
        },
        excursions: extremes([
            side.peak.zip(side.peak_at).and_then(|(peak, peak_at)| {
                (Some(peak) != side.at_start)
                    .then(|| (peak_at, format!("{peak} at {}", at(lines, peak_at))))
            }),
            None,
        ]),
        at_end: side
            .at_end
            .filter(|&at_end| Some(at_end) != side.at_start)
            .map(|at_end| at_end.to_string()),
    };
    span_rows(
        "controller",
        span(before_lines, difference.from, difference.until),
        vec![
            channel(difference.channel),
            controller_number(difference.controller),
        ],
        side(before_lines, &difference.before),
        side(after_lines, &difference.after),
    )
}

/// Where a stretch of the Piece two Takes disagree over begins and ends.
///
/// One sentence for every state compared as a span — a Controller, a tempo, a
/// bend — because a reader who has learnt to read one row has learnt to read
/// all three, and three spellings of *until* would be three things to learn for
/// nothing. `until` is exclusive, so a stretch that never closes reads
/// *onwards* rather than naming a Tick it does not reach.
pub fn span(lines: Option<BarLines>, from: u32, until: Option<u32>) -> String {
    match until {
        None => format!("{} onwards", at(lines, from)),
        Some(until) => format!("{} until {}", at(lines, from), at(lines, until)),
    }
}

/// One side of a span difference, and everything it has to say: what it holds
/// where the span begins, every extreme it reaches inside, and what it holds
/// where the span ends.
///
/// The excursions are in time order rather than a fixed up-then-down, because a
/// reader follows a stretch forwards and two clauses in the order they happened
/// are two facts rather than a shape. Saying which of them moved furthest, or
/// calling the pair a rise or a fall, would be the inference ADR-0007 rejects;
/// this states the extremes and leaves the gesture to whoever is reading.
struct Side {
    at_start: String,
    excursions: Vec<(u32, String)>,
    at_end: Option<String>,
}

impl Side {
    /// Whether this side says anything beyond where the span begins. Where
    /// neither side of a difference does, the whole difference is one row.
    fn has_interior(&self) -> bool {
        !self.excursions.is_empty() || self.at_end.is_some()
    }

    fn in_full(mut self) -> String {
        self.excursions.sort_by_key(|&(tick, _)| tick);
        let mut said = self.at_start;
        for (_, excursion) in self.excursions {
            said.push_str(", ");
            said.push_str(&excursion);
        }
        if let Some(at_end) = self.at_end {
            said.push_str(", ends at ");
            said.push_str(&at_end);
        }
        said
    }
}

/// One side's two extremes as clauses, each already carrying its reading and the
/// Tick it was reached at: `up to 144 at bar 6 beat 1`.
///
/// Both verbs where there are genuinely two extremes, and neither where the two
/// are one reading. A direction is a claim about where the side started, and a
/// side that started nowhere — `unstated` — has both its extremes stated and no
/// point to have moved from, so a span containing a single value would otherwise
/// come out as *up to 4000, down to 4000*: two clauses about one fact, each
/// asserting a direction off a floor that is not there. `reaching` says what the
/// pair actually knows in that case.
///
/// One extreme alone is a different thing and keeps its verb: the other was
/// dropped for equalling where the span began, so there is a point to have moved
/// from and the direction is read off it.
fn extremes(reached: [Option<(u32, String)>; 2]) -> Vec<(u32, String)> {
    let [up, down] = reached;
    match (up, down) {
        (Some(up), Some(down)) if up == down => vec![(up.0, format!("reaching {}", up.1))],
        (up, down) => up
            .map(|(tick, said)| (tick, format!("up to {said}")))
            .into_iter()
            .chain(down.map(|(tick, said)| (tick, format!("down to {said}"))))
            .collect(),
    }
}

/// The rows one span difference occupies.
///
/// One row where neither side has anything to say beyond where the span begins
/// — `tempo  bar 1 beat 1 onwards  60 -> 120` — and three where either does: a
/// heading naming the span, then a line for each side saying the whole of what
/// it holds across it.
///
/// Three rows rather than a longer one because a side's full reading is four
/// clauses and two of them side by side on one line is a line nobody finishes
/// reading. ADR-0007's *collapse into the one difference that states it* is
/// about not reporting an accelerando as forty differences and is untouched:
/// this is one difference either way, and the rows are its layout. What is not
/// allowed is what the single row used to do — hold a projection of the reading
/// and drop the rest of it (#32).
fn span_rows(
    label: &str,
    span: String,
    subject: Vec<String>,
    before: Side,
    after: Side,
) -> Vec<Vec<String>> {
    if !before.has_interior() && !after.has_interior() {
        let mut row = vec![label.to_string(), span];
        row.extend(subject);
        row.push(format!("{} -> {}", before.at_start, after.at_start));
        return vec![row];
    }
    let mut heading = vec![label.to_string(), span];
    heading.extend(subject);
    vec![
        heading,
        vec!["  before".to_string(), before.in_full()],
        vec!["  after".to_string(), after.in_full()],
    ]
}

/// The rows one tempo difference occupies: `tempo`, the span, and what each Take
/// is doing with the tempo across it.
///
/// Beats per minute, because that is the number a musician holds and the one
/// `mid info` already prints; the microseconds the file carries are in `--json`
/// for anyone who needs them — except where the beats alone would print two of
/// this difference's tempos identically, which `tempo_reading` covers.
///
/// `unstated` where a Take states no tempo across the span, in the same shape as
/// the numbers, for the reason `program_difference` prints it: a Take that names
/// no tempo is not a Take at 120. It is not a reason to stop reading, either —
/// an `unstated` side still says every extreme reached inside the span, since a
/// span the other Take opens is a span this one may still move about in.
pub fn tempo_rows(
    before_lines: Option<BarLines>,
    after_lines: Option<BarLines>,
    difference: &TempoDifference,
) -> Vec<Vec<String>> {
    let micros = beats_alone_collide(difference);
    let side = |lines, side: &TempoSide| {
        let start = side.at_start.map(|start| start.micros_per_quarter);
        Side {
            at_start: match side.at_start {
                None => "unstated".to_string(),
                Some(at_start) => tempo_reading(at_start, micros),
            },
            excursions: extremes(
                [
                    (side.fastest, side.fastest_at),
                    (side.slowest, side.slowest_at),
                ]
                .map(|(reached, reached_at)| {
                    let (reached, reached_at) = (reached?, reached_at?);
                    (Some(reached.micros_per_quarter) != start).then(|| {
                        (
                            reached_at,
                            format!(
                                "{} at {}",
                                tempo_reading(reached, micros),
                                at(lines, reached_at)
                            ),
                        )
                    })
                }),
            ),
            at_end: side
                .at_end
                .filter(|at_end| Some(at_end.micros_per_quarter) != start)
                .map(|at_end| tempo_reading(at_end, micros)),
        }
    };
    span_rows(
        "tempo",
        span(before_lines, difference.from, difference.until),
        Vec::new(),
        side(before_lines, &difference.before),
        side(after_lines, &difference.after),
    )
}

/// A tempo as a musician says it, with the microseconds beside it where the
/// beats alone would not tell two of one difference's tempos apart.
///
/// The beats are a rounding — see `bpm` — and two tempos a hair apart in
/// microseconds round to the same whole beat. A row asserting two sides differ
/// and then printing the same number on both of them is the failure `TempoSide`
/// carries two extremes to avoid, arriving by another road: the reader is told
/// of a difference and shown none. So where the collision happens the number the
/// file actually carries goes beside every tempo in that difference, and where
/// it does not the row stays as short as it reads.
///
/// Beside it, not instead of it, and never only in `--json`: ADR-0004's
/// carve-out for a convention holds only while the number it is a convention
/// about stays in view.
fn tempo_reading(tempo: battuta::Tempo, micros: bool) -> String {
    match micros {
        false => bpm(tempo.bpm),
        true => format!("{} ({} us)", bpm(tempo.bpm), tempo.micros_per_quarter),
    }
}

/// Whether two tempos of one difference are different tempos that print as the
/// same number of beats.
fn beats_alone_collide(difference: &TempoDifference) -> bool {
    let mut readings = Vec::new();
    for side in [&difference.before, &difference.after] {
        for tempo in [side.at_start, side.at_end, side.fastest, side.slowest]
            .into_iter()
            .flatten()
        {
            readings.push((bpm(tempo.bpm), tempo.micros_per_quarter));
        }
    }
    readings.iter().any(|(beats, micros)| {
        readings
            .iter()
            .any(|(other_beats, other_micros)| beats == other_beats && micros != other_micros)
    })
}

/// A tempo as a musician says it. Whole beats where the file means whole beats:
/// the microseconds a tempo is stored as do not all divide evenly into a
/// minute, so a tempo written as 140 arrives back as 140.00014 and printing
/// that would be reporting the encoding rather than the Piece. 120 divides
/// exactly and never needed this; the tempos either side of it do.
fn bpm(bpm: f64) -> String {
    if (bpm - bpm.round()).abs() < 0.005 {
        format!("{}", bpm.round() as i64)
    } else {
        format!("{bpm:.2}")
    }
}

/// The rows one bend difference occupies: `bend`, the span, the channel, and how
/// far each Take bends it across the span.
///
/// The raw signed number, not semitones. How many semitones a bend is worth is
/// the synthesiser's bend range and is not in the file, so naming semitones here
/// would be reporting a Rig fact as a Piece one. `0` is the centre and is a
/// value: a channel bent back to nought is not a channel never bent, which is
/// what `unstated` says.
///
/// Both directions are said, in time order, which is the whole reason `BendSide`
/// carries two extremes: a bend is signed about a centre MIDI fixes, so a dive
/// below the note and a rise above it are two facts and neither stands in for
/// the other.
pub fn bend_rows(
    before_lines: Option<BarLines>,
    after_lines: Option<BarLines>,
    difference: &BendDifference,
) -> Vec<Vec<String>> {
    let side = |lines, side: &BendSide| Side {
        at_start: match side.at_start {
            None => "unstated".to_string(),
            Some(at_start) => at_start.to_string(),
        },
        excursions: extremes(
            [
                (side.furthest_up, side.furthest_up_at),
                (side.furthest_down, side.furthest_down_at),
            ]
            .map(|(reached, reached_at)| {
                let (reached, reached_at) = (reached?, reached_at?);
                (Some(reached) != side.at_start).then(|| {
                    (
                        reached_at,
                        format!("{reached} at {}", at(lines, reached_at)),
                    )
                })
            }),
        ),
        at_end: side
            .at_end
            .filter(|&at_end| Some(at_end) != side.at_start)
            .map(|at_end| at_end.to_string()),
    };
    span_rows(
        "bend",
        span(before_lines, difference.from, difference.until),
        vec![channel(difference.channel)],
        side(before_lines, &difference.before),
        side(after_lines, &difference.after),
    )
}

/// What one channel is on: the state a passage begins in.
pub fn programs(state: &Program) -> Vec<String> {
    vec![
        channel(state.channel),
        state_reading(&state.program, |value| program(state.channel, Some(*value))),
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

/// A place the Take states no order between two of its tracks.
///
/// One sentence rather than a table row. The other blocks answer a question the
/// reader asked — what is each channel on, where does the passage state another
/// — and this one answers a question they did not: it is a warning, and a
/// warning laid out as a table reads as data. It also has to say what to do,
/// which no row of four cells has room for.
///
/// It names both tracks, because knowing only one of them does not tell you
/// which way to move anything, and it spells the site the way
/// `--allow-unranked` takes it, so that a reader who decides to answer for it
/// can copy the argument out of the line.
pub fn unranked(lines: Option<BarLines>, row: &battuta::Unranked) -> String {
    let state = match (row.state, row.controller) {
        (battuta::State::Controller, Some(number)) => {
            format!("the control change for CC {number}")
        }
        (battuta::State::Controller, None) => "the control change".to_string(),
        (battuta::State::Program, _) => "the program change".to_string(),
        (battuta::State::Bend, _) => "the bend".to_string(),
        (battuta::State::Tempo, _) => "the tempo".to_string(),
    };
    let against = match row.against {
        battuta::Against::Notes => format!("notes of that channel on track {}", row.against_track),
        battuta::Against::Value => {
            format!("a different value for it on track {}", row.against_track)
        }
    };
    match row.channel {
        Some(channel) => format!(
            "{}  {} for channel {} on track {} has no order the file states against {}  \
             (t{}:c{}:s{})",
            at(lines, row.tick),
            state,
            channel,
            row.track,
            against,
            row.track,
            channel,
            row.tick,
        ),
        // No site to copy. `--allow-unranked` takes a track, a channel and a
        // tick, and no Edit in the contract writes a state that has no channel
        // — so there is nothing here for a caller to answer for, and a line
        // offering an argument the flag would refuse would be advertising a
        // grammar that does not exist. See #42.
        None => format!(
            "{}  {} on track {} has no order the file states against {}",
            at(lines, row.tick),
            state,
            row.track,
            against,
        ),
    }
}
