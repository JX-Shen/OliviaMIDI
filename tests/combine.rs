//! `mid combine` — one Take from two Alternatives and their common Take (#56,
//! the first slice of #51).
//!
//! The cases are the 27 the #51 research probe ran, re-expressed through the
//! released binary, plus what #56's acceptance names beyond them. Every
//! assertion reads events. `mid diff --tolerance 0` is deliberately not an
//! oracle anywhere here: the probe showed it reports nothing at a reordered
//! site, so a check built on it would pass a combination that got the order
//! wrong.
//!
//! The checks each candidate passes are the probe's five, computed here from
//! the files and the Edit Sets alone, without asking `battuta` how it built
//! anything:
//!
//! 1. D's events per track are exactly what the accounts predict, by Tick and
//!    content;
//! 2. every event no side relocated keeps the common Take's order;
//! 3. every pair a side established keeps that side's order, except at a site
//!    the report discloses;
//! 4. D's notes pair strike to release exactly as the common Take's did, less
//!    those deleted;
//! 5. `mid apply D empty.json` round-trips D at the event level (ADR-0003).
//!
//! and then the sides are exchanged, and D and the report have to come back the
//! same up to which side is which.

mod common;

use common::mid;
use midly::num::{u15, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

const BACH: &str = "fixtures/external/mutopia-bach-bwv208.mid";
const GROOVE: &str = "fixtures/external/groove-drummer8-funk.mid";

// ------------------------------------------------------------------ reading --

/// One event as these tests see it: its Tick, its content spelled out, and —
/// for a channel event — the channel and message, so a test can say what the
/// same strike would be at another velocity.
#[derive(Clone, Debug)]
struct Ev {
    tick: u32,
    key: String,
    midi: Option<(u8, MidiMessage)>,
}

/// A Take's header and its tracks, events in the order the file writes them.
struct Parsed {
    header: String,
    tracks: Vec<Vec<Ev>>,
}

fn key_of(kind: &TrackEventKind) -> String {
    format!("{kind:?}")
}

fn parse(path: &Path) -> Parsed {
    let bytes = std::fs::read(path).expect("Take is readable");
    let smf = Smf::parse(&bytes).expect("Take parses");
    let tracks = smf
        .tracks
        .iter()
        .map(|track| {
            let mut tick = 0u32;
            track
                .iter()
                .map(|event| {
                    tick += event.delta.as_int();
                    Ev {
                        tick,
                        key: key_of(&event.kind),
                        midi: match event.kind {
                            TrackEventKind::Midi { channel, message } => {
                                Some((channel.as_int(), message))
                            }
                            _ => None,
                        },
                    }
                })
                .collect()
        })
        .collect();
    Parsed {
        header: format!("{:?}", smf.header),
        tracks,
    }
}

/// ADR-0003's equality: the same header, and per track the same events in the
/// same order at the same Ticks. Never bytes.
fn events_equal(a: &Path, b: &Path) -> bool {
    let (a, b) = (parse(a), parse(b));
    let flat = |p: &Parsed| -> Vec<Vec<(u32, String)>> {
        p.tracks
            .iter()
            .map(|t| t.iter().map(|e| (e.tick, e.key.clone())).collect())
            .collect()
    };
    a.header == b.header && flat(&a) == flat(&b)
}

fn strike(e: &Ev) -> Option<(u8, u8, u8)> {
    match e.midi {
        Some((ch, MidiMessage::NoteOn { key, vel })) if vel.as_int() > 0 => {
            Some((ch, key.as_int(), vel.as_int()))
        }
        _ => None,
    }
}

fn release(e: &Ev) -> Option<(u8, u8)> {
    match e.midi {
        Some((ch, MidiMessage::NoteOff { key, .. })) => Some((ch, key.as_int())),
        Some((ch, MidiMessage::NoteOn { key, vel })) if vel.as_int() == 0 => {
            Some((ch, key.as_int()))
        }
        _ => None,
    }
}

/// A note of a Take, paired the way a reader pairs one — the first release of
/// a channel and pitch ends the oldest note of it still sounding — with the
/// two event positions that carry it.
#[derive(Clone, Debug)]
struct Note {
    id: String,
    track: usize,
    start: u32,
    end: u32,
    velocity: u8,
    strike: usize,
    release: usize,
}

fn notes_of(parsed: &Parsed) -> Vec<Note> {
    let mut notes: Vec<Note> = Vec::new();
    for (track, events) in parsed.tracks.iter().enumerate() {
        let mut open: HashMap<(u8, u8), Vec<usize>> = HashMap::new();
        for (at, e) in events.iter().enumerate() {
            if let Some((ch, pitch, vel)) = strike(e) {
                open.entry((ch, pitch)).or_default().push(notes.len());
                notes.push(Note {
                    id: format!("t{track}:c{ch}:p{pitch}:s{}", e.tick),
                    track,
                    start: e.tick,
                    end: u32::MAX,
                    velocity: vel,
                    strike: at,
                    release: usize::MAX,
                });
            } else if let Some(address) = release(e) {
                if let Some(queue) = open.get_mut(&address) {
                    if !queue.is_empty() {
                        let n = queue.remove(0);
                        notes[n].end = e.tick;
                        notes[n].release = at;
                    }
                }
            }
        }
    }
    let mut occurrence: HashMap<String, usize> = HashMap::new();
    for note in &mut notes {
        let n = occurrence.entry(note.id.clone()).or_insert(0);
        note.id = format!("{}:n{n}", note.id);
        *n += 1;
    }
    notes
}

// ------------------------------------------------------------------- edits --

/// One Edit of the subset, as an Edit Set spells it.
#[derive(Clone, Debug, PartialEq)]
enum E {
    Velocity(String, i64),
    Resize(String, i64),
    Delete(String),
}

fn vel(id: &str, velocity: i64) -> E {
    E::Velocity(id.to_string(), velocity)
}

fn resize(id: &str, delta: i64) -> E {
    E::Resize(id.to_string(), delta)
}

fn delete(id: &str) -> E {
    E::Delete(id.to_string())
}

fn write_edits(path: &Path, edits: &[E]) {
    let edits: Vec<Value> = edits
        .iter()
        .map(|e| match e {
            E::Velocity(id, v) => json!({ "kind": "set_velocity", "id": id, "velocity": v }),
            E::Resize(id, d) => json!({ "kind": "resize_note", "id": id, "delta_ticks": d }),
            E::Delete(id) => json!({ "kind": "delete_note", "id": id }),
        })
        .collect();
    std::fs::write(path, json!({ "edits": edits }).to_string()).expect("writable");
}

/// What one Edit Set asks of the notes of the common Take: terminal values.
#[derive(Default, Clone)]
struct Account {
    deleted: BTreeSet<String>,
    velocity: BTreeMap<String, i64>,
    duration: BTreeMap<String, i64>,
    relocated: BTreeSet<String>,
}

fn account(edits: &[E], notes: &HashMap<String, Note>) -> Account {
    let mut a = Account::default();
    for e in edits {
        match e {
            E::Velocity(id, v) => {
                a.velocity.insert(id.clone(), *v);
            }
            E::Resize(id, d) => {
                let had = i64::from(notes[id].end - notes[id].start);
                *a.duration.entry(id.clone()).or_insert(had) += d;
                // A step of 0 moves nothing and relocates nothing (#56,
                // Decision of 2026-10-01).
                if *d != 0 {
                    a.relocated.insert(id.clone());
                }
            }
            E::Delete(id) => {
                a.deleted.insert(id.clone());
            }
        }
    }
    a
}

/// The account of the combined Take, for a combination that succeeded: what
/// either side actually changed, and every note either side relocated.
fn combined_account(accounts: &[Account], notes: &HashMap<String, Note>) -> Account {
    let mut d = Account::default();
    for a in accounts {
        d.deleted.extend(a.deleted.iter().cloned());
        d.relocated.extend(a.relocated.iter().cloned());
        for (id, v) in &a.velocity {
            if *v != i64::from(notes[id].velocity) {
                d.velocity.insert(id.clone(), *v);
            }
        }
        for (id, v) in &a.duration {
            if *v != i64::from(notes[id].end - notes[id].start) {
                d.duration.insert(id.clone(), *v);
            }
        }
    }
    d
}

/// Per track, the (Tick, content, index in the common Take) a Take made from
/// the common Take by this account must hold — as a multiset.
fn expected(a: &Parsed, notes: &[Note], acc: &Account) -> Vec<Vec<(u32, String, usize)>> {
    let mut of_event: HashMap<(usize, usize), &Note> = HashMap::new();
    for n in notes {
        of_event.insert((n.track, n.strike), n);
        of_event.insert((n.track, n.release), n);
    }
    a.tracks
        .iter()
        .enumerate()
        .map(|(track, events)| {
            let mut row = events
                .iter()
                .enumerate()
                .filter_map(|(at, e)| {
                    let Some(n) = of_event.get(&(track, at)) else {
                        return Some((e.tick, e.key.clone(), at));
                    };
                    if acc.deleted.contains(&n.id) {
                        return None;
                    }
                    if at == n.strike {
                        if let (Some(v), Some((ch, MidiMessage::NoteOn { key, .. }))) =
                            (acc.velocity.get(&n.id), e.midi)
                        {
                            let kind = TrackEventKind::Midi {
                                channel: u4::new(ch),
                                message: MidiMessage::NoteOn {
                                    key,
                                    vel: u7::new(u8::try_from(*v).expect("a velocity")),
                                },
                            };
                            return Some((e.tick, key_of(&kind), at));
                        }
                    }
                    if at == n.release {
                        if let Some(d) = acc.duration.get(&n.id) {
                            let tick = u32::try_from(i64::from(n.start) + d).expect("a Tick");
                            return Some((tick, e.key.clone(), at));
                        }
                    }
                    Some((e.tick, e.key.clone(), at))
                })
                .collect::<Vec<_>>();
            // The End-of-Track stays where it was unless an event now falls
            // after it, and then it follows that event — `apply`'s rule, which
            // only ever extends a track.
            let last = row.len().saturating_sub(1);
            if let Some(end) = row.get(last).filter(|e| e.1.contains("EndOfTrack")) {
                let reached = row[..last].iter().map(|e| e.0).max().unwrap_or(0);
                row[last].0 = end.0.max(reached);
            }
            row
        })
        .collect()
}

/// Each event of `parsed`, in its own order, as (Tick, index in the common
/// Take) — or `None` where its events are not exactly the expected multiset.
fn map_to_a(
    parsed: &Parsed,
    expected: &[Vec<(u32, String, usize)>],
) -> Option<Vec<Vec<(u32, usize)>>> {
    if parsed.tracks.len() != expected.len() {
        return None;
    }
    let mut mapped = Vec::new();
    for (events, want) in parsed.tracks.iter().zip(expected) {
        if events.len() != want.len() {
            return None;
        }
        let mut pool: HashMap<(u32, &str), Vec<usize>> = HashMap::new();
        for (tick, key, at) in want {
            pool.entry((*tick, key.as_str())).or_default().push(*at);
        }
        let mut row = Vec::new();
        for e in events {
            let queue = pool.get_mut(&(e.tick, e.key.as_str()))?;
            if queue.is_empty() {
                return None;
            }
            row.push((e.tick, queue.remove(0)));
        }
        mapped.push(row);
    }
    Some(mapped)
}

// ----------------------------------------------------------------- building --

/// One event a built Take states beyond its notes: (Tick, sequence at that
/// Tick, event). Strikes are sequence 0 and releases 1, so an extra at
/// sequence 0 is written in front of the releases at its Tick.
type Extra = (u32, u8, TrackEventKind<'static>);

fn controller(tick: u32, number: u8, value: u8) -> Extra {
    (
        tick,
        0,
        TrackEventKind::Midi {
            channel: u4::new(0),
            message: MidiMessage::Controller {
                controller: u7::new(number),
                value: u7::new(value),
            },
        },
    )
}

fn program(tick: u32, seq: u8, number: u8) -> Extra {
    (
        tick,
        seq,
        TrackEventKind::Midi {
            channel: u4::new(0),
            message: MidiMessage::ProgramChange {
                program: u7::new(number),
            },
        },
    )
}

/// A single-track Take at 480 PPQ in 4/4, as the probe built them: each note
/// is (start, end, pitch, velocity, release velocity) — the release velocity
/// only so that releases of one pitch are told apart — and the track ends at
/// `tail`.
fn synth(path: &Path, notes: &[(u32, u32, u8, u8, u8)], extra: &[Extra], tail: u32) -> PathBuf {
    let mut events: Vec<(u32, u8, TrackEventKind<'static>)> = Vec::new();
    for &(start, end, pitch, velocity, released) in notes {
        events.push((
            start,
            0,
            TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOn {
                    key: u7::new(pitch),
                    vel: u7::new(velocity),
                },
            },
        ));
        events.push((
            end,
            1,
            TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOff {
                    key: u7::new(pitch),
                    vel: u7::new(released),
                },
            },
        ));
    }
    events.extend_from_slice(extra);
    events.sort_by_key(|&(tick, seq, _)| (tick, seq));
    let mut track = vec![
        TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
        },
        TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(midly::num::u24::new(500_000))),
        },
    ];
    let mut last = 0;
    for (tick, _, kind) in events {
        track.push(TrackEvent {
            delta: u28::new(tick - last),
            kind,
        });
        last = tick;
    }
    track.push(TrackEvent {
        delta: u28::new(tail - last),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    Smf {
        header: Header::new(Format::SingleTrack, Timing::Metrical(u15::new(480))),
        tracks: vec![track],
    }
    .save(path)
    .expect("built Take is writable");
    path.to_path_buf()
}

/// An Alternative: the Take `mid apply` makes of `a` with these Edits, and the
/// Edit Set beside it.
struct Side {
    take: PathBuf,
    edits_path: PathBuf,
    edits: Vec<E>,
}

fn side(dir: &Path, name: &str, a: &Path, edits: &[E]) -> Side {
    let edits_path = dir.join(format!("{name}.json"));
    write_edits(&edits_path, edits);
    let take = dir.join(format!("{name}.mid"));
    let out = mid()
        .arg("apply")
        .arg(a)
        .arg(&edits_path)
        .arg("-o")
        .arg(&take)
        .output()
        .expect("mid runs");
    assert!(
        out.status.success(),
        "building side {name}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Side {
        take,
        edits_path,
        edits: edits.to_vec(),
    }
}

/// What `mid combine --json` did.
struct Outcome {
    success: bool,
    document: Value,
    stderr: String,
    output: PathBuf,
}

fn combine(a: &Path, sides: [&Side; 2], output: &Path) -> Outcome {
    let out = mid()
        .arg("combine")
        .arg(a)
        .arg("--side")
        .arg(&sides[0].take)
        .arg(&sides[0].edits_path)
        .arg("--side")
        .arg(&sides[1].take)
        .arg(&sides[1].edits_path)
        .arg("-o")
        .arg(output)
        .arg("--json")
        .output()
        .expect("mid runs");
    let document = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
        panic!(
            "--json writes one document on stdout, success or refusal: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    Outcome {
        success: out.status.success(),
        document,
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        output: output.to_path_buf(),
    }
}

/// A combination that has to refuse, with this `refusal.kind`, and write
/// nothing.
fn refused(dir: &Path, a: &Path, sides: [&Side; 2], kind: &str) -> Value {
    let output = dir.join("D.mid");
    let outcome = combine(a, sides, &output);
    assert!(!outcome.success, "{} should refuse", kind);
    assert_eq!(outcome.document["output"], Value::Null);
    assert_eq!(
        outcome.document["refusal"]["kind"], kind,
        "{}",
        outcome.stderr
    );
    assert!(!output.exists(), "a refusal wrote {output:?}");
    outcome.document["refusal"].clone()
}

/// A combination that has to succeed: it passes the five checks, exchanging
/// the sides gives the same D and the same report up to side index, and where
/// one side asks for nothing, or both ask for the same, D is the other side
/// event for event. Returns the report and D's path.
fn candidate(dir: &Path, a: &Path, sides: [&Side; 2]) -> (Value, PathBuf) {
    let d = dir.join("D.mid");
    let outcome = combine(a, sides, &d);
    assert!(outcome.success, "refused: {}", outcome.stderr);
    let report = outcome.document;
    for (i, s) in sides.iter().enumerate() {
        assert_eq!(report["sides"][i]["verified"], true);
        assert_eq!(report["sides"][i]["take"], s.take.display().to_string());
    }
    assert_eq!(report["output"], d.display().to_string());

    five_checks(dir, a, sides, &outcome.output, &report);

    let swapped_d = dir.join("D-swapped.mid");
    let swapped = combine(a, [sides[1], sides[0]], &swapped_d);
    assert!(swapped.success, "refused swapped: {}", swapped.stderr);
    assert!(
        events_equal(&d, &swapped_d),
        "exchanging the sides changed D"
    );
    assert_eq!(report["unchanged"], swapped.document["unchanged"]);
    assert_eq!(
        report["disclosed_sites"],
        swapped.document["disclosed_sites"]
    );
    assert_eq!(report["track_ends"], swapped.document["track_ends"]);
    let exchanged = |entries: &Value| -> BTreeSet<String> {
        entries
            .as_array()
            .expect("an array")
            .iter()
            .map(|entry| {
                let mut entry = entry.clone();
                entry["side"] = json!(1 - entry["side"].as_u64().expect("a side"));
                entry.to_string()
            })
            .collect()
    };
    let plain = |entries: &Value| -> BTreeSet<String> {
        entries
            .as_array()
            .expect("an array")
            .iter()
            .map(Value::to_string)
            .collect()
    };
    assert_eq!(
        plain(&report["no_effect"]),
        exchanged(&swapped.document["no_effect"])
    );

    let empty: Vec<&&Side> = sides.iter().filter(|s| s.edits.is_empty()).collect();
    if empty.len() == 2 {
        assert!(events_equal(&d, a), "empty + empty is not the common Take");
        assert_eq!(report["unchanged"], true);
    } else if empty.len() == 1 || sides[0].edits == sides[1].edits {
        let other = sides.iter().find(|s| !s.edits.is_empty()).expect("a side");
        assert!(
            events_equal(&d, &other.take),
            "D is not event for event the side that asked for something"
        );
    }
    (report, d)
}

/// The probe's five checks.
fn five_checks(dir: &Path, a_path: &Path, sides: [&Side; 2], d_path: &Path, report: &Value) {
    let a = parse(a_path);
    let notes = notes_of(&a);
    let by_id: HashMap<String, Note> = notes.iter().map(|n| (n.id.clone(), n.clone())).collect();
    let accounts: Vec<Account> = sides.iter().map(|s| account(&s.edits, &by_id)).collect();
    let combined = combined_account(&accounts, &by_id);
    let d = parse(d_path);

    // 1. Exact multiset.
    let d_map = map_to_a(&d, &expected(&a, &notes, &combined))
        .expect("D holds exactly the events the accounts predict");

    let release_of = |id: &String| (by_id[id].track, by_id[id].release);
    let relocated_by: Vec<BTreeSet<(usize, usize)>> = accounts
        .iter()
        .map(|acc| acc.relocated.iter().map(release_of).collect())
        .collect();
    let all_relocated: BTreeSet<(usize, usize)> = relocated_by.iter().flatten().copied().collect();

    // 2. Untouched events keep the common Take's order.
    for (track, row) in d_map.iter().enumerate() {
        let untouched: Vec<usize> = row
            .iter()
            .map(|&(_, at)| at)
            .filter(|at| !all_relocated.contains(&(track, *at)))
            .collect();
        assert!(
            untouched.windows(2).all(|w| w[0] < w[1]),
            "track {track}: an event no side moved left the common Take's order"
        );
    }

    // 3. Every pair a side established keeps that side's order in D.
    let disclosed: BTreeSet<(u64, u64)> = report["disclosed_sites"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|s| (s["track"].as_u64().unwrap(), s["tick"].as_u64().unwrap()))
        .collect();
    let d_pos: HashMap<(usize, usize), (u32, usize)> = d_map
        .iter()
        .enumerate()
        .flat_map(|(track, row)| {
            row.iter()
                .enumerate()
                .map(move |(pos, &(tick, at))| ((track, at), (tick, pos)))
        })
        .collect();
    for (s, acc) in sides.iter().zip(&accounts) {
        let mapped = map_to_a(&parse(&s.take), &expected(&a, &notes, acc))
            .expect("a side holds what its own account predicts");
        let s_pos: HashMap<(usize, usize), (u32, usize)> = mapped
            .iter()
            .enumerate()
            .flat_map(|(track, row)| {
                row.iter()
                    .enumerate()
                    .map(move |(pos, &(tick, at))| ((track, at), (tick, pos)))
            })
            .collect();
        for x in acc.relocated.iter().map(release_of) {
            let Some(&(tx, px)) = s_pos.get(&x) else {
                continue;
            };
            if disclosed.contains(&(x.0 as u64, u64::from(tx))) {
                continue;
            }
            for (&y, &(ty, py)) in &s_pos {
                if y == x || y.0 != x.0 || ty != tx {
                    continue;
                }
                let (Some(dx), Some(dy)) = (d_pos.get(&x), d_pos.get(&y)) else {
                    continue;
                };
                if dx.0 != dy.0 {
                    continue;
                }
                assert_eq!(
                    px < py,
                    dx.1 < dy.1,
                    "track {} tick {tx}: D does not keep the order a side established",
                    x.0
                );
            }
        }
    }

    // 4. Note pairing re-derives.
    let d_notes = notes_of(&d);
    let paired: BTreeSet<(usize, usize, usize)> = d_notes
        .iter()
        .map(|n| {
            (
                n.track,
                d_map[n.track][n.strike].1,
                d_map[n.track][n.release].1,
            )
        })
        .collect();
    let wanted: BTreeSet<(usize, usize, usize)> = notes
        .iter()
        .filter(|n| !combined.deleted.contains(&n.id))
        .map(|n| (n.track, n.strike, n.release))
        .collect();
    assert_eq!(paired, wanted, "D's notes do not pair as the common Take's");

    // 5. An empty Edit Set round-trips D through the core.
    let empty = dir.join("empty.json");
    write_edits(&empty, &[]);
    let round = dir.join("D-roundtrip.mid");
    mid()
        .arg("apply")
        .arg(d_path)
        .arg(&empty)
        .arg("-o")
        .arg(&round)
        .assert()
        .success();
    assert!(events_equal(d_path, &round), "D does not round-trip");
}

/// Pitches of the releases at `tick` on track 0, in written order.
fn releases_at(path: &Path, tick: u32) -> Vec<u8> {
    parse(path).tracks[0]
        .iter()
        .filter(|e| e.tick == tick)
        .filter_map(|e| release(e).map(|(_, pitch)| pitch))
        .collect()
}

/// What track 0 writes at `tick`, in order, as short words.
fn written_at(path: &Path, tick: u32) -> Vec<String> {
    parse(path).tracks[0]
        .iter()
        .filter(|e| e.tick == tick)
        .filter_map(|e| match e.midi {
            Some((_, MidiMessage::Controller { controller, .. })) => {
                Some(format!("cc{}", controller.as_int()))
            }
            Some((_, MidiMessage::ProgramChange { .. })) => Some("program".to_string()),
            _ => strike(e)
                .map(|(_, p, _)| format!("on{p}"))
                .or_else(|| release(e).map(|(_, p)| format!("off{p}"))),
        })
        .collect()
}

// --------------------------------------------------- #47's counterexample --

const P60: &str = "t0:c0:p60:s0:n0";
const P62: &str = "t0:c0:p62:s50:n0";

/// p60 [0, 100] and p62 [50, 200]: lengthening p60 by 200 and p62 by 100 makes
/// their releases meet at 300, in the order the Edit Set wrote them.
fn meet_a(dir: &Path) -> PathBuf {
    synth(
        &dir.join("A.mid"),
        &[(0, 100, 60, 80, 10), (50, 200, 62, 90, 20)],
        &[],
        1000,
    )
}

fn x() -> E {
    resize(P60, 200)
}

fn y() -> E {
    resize(P62, 100)
}

#[test]
fn b_plus_empty_keeps_b_s_release_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = meet_a(dir.path());
    let b = side(dir.path(), "B", &a, &[x(), y()]);
    let e = side(dir.path(), "E", &a, &[]);
    let (_, d) = candidate(dir.path(), &a, [&b, &e]);
    assert_eq!(releases_at(&b.take, 300), vec![62, 60]);
    assert_eq!(releases_at(&d, 300), vec![62, 60]);
}

#[test]
fn b_plus_b_is_b() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = meet_a(dir.path());
    let b = side(dir.path(), "B", &a, &[x(), y()]);
    let b2 = side(dir.path(), "B2", &a, &[x(), y()]);
    let (_, d) = candidate(dir.path(), &a, [&b, &b2]);
    assert_eq!(releases_at(&d, 300), vec![62, 60]);
}

#[test]
fn c_plus_empty_keeps_c_s_release_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = meet_a(dir.path());
    let c = side(dir.path(), "C", &a, &[y(), x()]);
    let e = side(dir.path(), "E", &a, &[]);
    let (_, d) = candidate(dir.path(), &a, [&c, &e]);
    assert_eq!(releases_at(&c.take, 300), vec![60, 62]);
    assert_eq!(releases_at(&d, 300), vec![60, 62]);
}

/// Equal demands, opposite established orders, on a pair the rule does not
/// rank: the convention decides — p60's strike is first in A — and the site is
/// disclosed rather than refused (#51 Decision).
#[test]
fn equal_demands_in_opposite_orders_are_disclosed_not_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = meet_a(dir.path());
    let b = side(dir.path(), "B", &a, &[x(), y()]);
    let c = side(dir.path(), "C", &a, &[y(), x()]);
    let (report, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(
        report["disclosed_sites"],
        json!([{ "track": 0, "tick": 300, "channel": 0 }])
    );
    assert_eq!(releases_at(&d, 300), vec![60, 62]);
}

/// Each side lengthens one note: the releases meet only in D, nobody
/// established their order, and the convention orders them silently.
#[test]
fn a_pair_that_first_meets_in_d_is_ordered_by_the_convention() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = meet_a(dir.path());
    let b = side(dir.path(), "B", &a, &[x()]);
    let c = side(dir.path(), "C", &a, &[y()]);
    let (report, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(report["disclosed_sites"], json!([]));
    assert_eq!(releases_at(&d, 300), vec![60, 62]);
}

#[test]
fn b_plus_an_unrelated_velocity_change_keeps_b_s_release_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = synth(
        &dir.path().join("A.mid"),
        &[
            (0, 100, 60, 80, 10),
            (50, 200, 62, 90, 20),
            (400, 700, 67, 80, 30),
        ],
        &[],
        1000,
    );
    let b = side(dir.path(), "B", &a, &[x(), y()]);
    let c = side(dir.path(), "C", &a, &[vel("t0:c0:p67:s400:n0", 50)]);
    let (_, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(releases_at(&d, 300), vec![62, 60]);
}

// ------------------------------------------------------------ coordination --

/// Three notes collide on track, channel, pitch and start. Each side's
/// identity is bound on the common Take, not on the other side's result: B
/// deletes n0 and C softens n1, and in D the note C softened is the one that
/// was n1 in A, now n0 because the note before it is gone.
#[test]
fn stacked_occurrence_targets_bind_on_the_common_take() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = synth(
        &dir.path().join("A.mid"),
        &[
            (0, 100, 60, 80, 10),
            (0, 100, 60, 80, 20),
            (0, 100, 60, 80, 30),
        ],
        &[],
        1000,
    );
    let b = side(dir.path(), "B", &a, &[delete("t0:c0:p60:s0:n0")]);
    let c = side(dir.path(), "C", &a, &[vel("t0:c0:p60:s0:n1", 40)]);
    let (_, d) = candidate(dir.path(), &a, [&b, &c]);
    let velocities: Vec<u8> = notes_of(&parse(&d)).iter().map(|n| n.velocity).collect();
    assert_eq!(velocities, vec![40, 80]);
    assert_eq!(releases_at(&d, 100).len(), 2);
}

fn plain_a(dir: &Path) -> PathBuf {
    synth(
        &dir.join("A.mid"),
        &[(0, 100, 60, 80, 10), (200, 300, 64, 80, 20)],
        &[],
        1000,
    )
}

#[test]
fn agreement_on_a_duration_is_counted_once() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let b = side(dir.path(), "B", &a, &[resize(P60, 20)]);
    let c = side(dir.path(), "C", &a, &[resize(P60, 20)]);
    let (_, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(
        notes_of(&parse(&d))[0].end,
        120,
        "+20 twice is 120, not 140"
    );
}

#[test]
fn agreement_on_a_velocity_is_counted_once() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let b = side(dir.path(), "B", &a, &[vel(P60, 60)]);
    let c = side(dir.path(), "C", &a, &[vel(P60, 60)]);
    candidate(dir.path(), &a, [&b, &c]);
}

/// Two conflicts in one combination are reported together, each with both
/// demands, and the combination refuses as a whole.
#[test]
fn conflicts_are_reported_together_with_both_demands() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let b = side(
        dir.path(),
        "B",
        &a,
        &[vel(P60, 60), delete("t0:c0:p64:s200:n0")],
    );
    let c = side(
        dir.path(),
        "C",
        &a,
        &[vel(P60, 70), resize("t0:c0:p64:s200:n0", 50)],
    );
    let refusal = refused(dir.path(), &a, [&b, &c], "conflict");
    assert_eq!(
        refusal["conflicts"],
        json!([
            { "id": P60, "field": "velocity",
              "demands": [ { "side": 0, "value": 60 }, { "side": 1, "value": 70 } ] },
            { "id": "t0:c0:p64:s200:n0", "delete": [0],
              "changes": [ { "side": 1, "field": "duration", "value": 150 } ] }
        ])
    );
}

#[test]
fn velocity_60_against_70_refuses() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let b = side(dir.path(), "B", &a, &[vel(P60, 60)]);
    let c = side(dir.path(), "C", &a, &[vel(P60, 70)]);
    let refusal = refused(dir.path(), &a, [&b, &c], "conflict");
    assert_eq!(
        refusal["conflicts"],
        json!([{ "id": P60, "field": "velocity",
                 "demands": [ { "side": 0, "value": 60 }, { "side": 1, "value": 70 } ] }])
    );
}

#[test]
fn delete_against_resize_refuses() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let b = side(dir.path(), "B", &a, &[delete(P60)]);
    let c = side(dir.path(), "C", &a, &[resize(P60, 20)]);
    let refusal = refused(dir.path(), &a, [&b, &c], "conflict");
    assert_eq!(
        refusal["conflicts"],
        json!([{ "id": P60, "delete": [0],
                 "changes": [ { "side": 1, "field": "duration", "value": 120 } ] }])
    );
}

/// A request for the value a note already has asks for nothing: it is
/// reported, and it is no demand — so C's 60 is the combined requirement.
#[test]
fn a_request_with_no_effect_is_reported_and_demands_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let b = side(dir.path(), "B", &a, &[vel(P60, 80)]);
    let c = side(dir.path(), "C", &a, &[vel(P60, 60)]);
    let (report, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(
        report["no_effect"],
        json!([{ "side": 0, "id": P60, "field": "velocity" }])
    );
    assert!(events_equal(&d, &c.take));
}

#[test]
fn empty_plus_empty_is_a_successful_no_change() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let b = side(dir.path(), "B", &a, &[]);
    let c = side(dir.path(), "C", &a, &[]);
    let (report, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(report["unchanged"], true);
    assert!(events_equal(&d, &a));
}

/// Each side alone is sound — both were made by `mid apply` — and together
/// they leave two notes of one pitch finishing out of the order they began.
/// Found building the whole candidate, and refused as found.
#[test]
fn a_whole_candidate_failure_whose_subsets_pass_refuses_as_core_invalid() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = synth(
        &dir.path().join("A.mid"),
        &[(0, 100, 60, 80, 10), (50, 200, 60, 90, 20)],
        &[],
        1000,
    );
    let b = side(dir.path(), "B", &a, &[resize("t0:c0:p60:s0:n0", 80)]);
    let c = side(dir.path(), "C", &a, &[resize("t0:c0:p60:s50:n0", -50)]);
    let refusal = refused(dir.path(), &a, [&b, &c], "core_invalid");
    assert!(
        refusal["detail"]
            .as_str()
            .expect("a detail")
            .contains("one note ending inside another"),
        "{refusal}"
    );
}

/// A side whose Edit Set does not make the Take supplied beside it is refused
/// before anything is read from it, and the side is named.
#[test]
fn a_tampered_side_refuses_as_source_evidence_mismatch() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let mut b = side(dir.path(), "B", &a, &[vel(P60, 60)]);
    let claimed = dir.path().join("claimed.json");
    write_edits(&claimed, &[vel(P60, 61)]);
    b.edits_path = claimed;
    let c = side(dir.path(), "C", &a, &[]);
    let output = dir.path().join("D.mid");
    let outcome = combine(&a, [&b, &c], &output);
    assert!(!outcome.success);
    assert_eq!(
        outcome.document["refusal"],
        json!({ "kind": "source_evidence_mismatch", "side": 0 })
    );
    assert_eq!(outcome.document["sides"][0]["verified"], false);
    // The other side was never reached: unchecked, which is not failed.
    assert_eq!(outcome.document["sides"][1]["verified"], Value::Null);
    assert!(!output.exists());

    // Named whichever side it is.
    let outcome = combine(&a, [&c, &b], &output);
    assert_eq!(outcome.document["refusal"]["side"], 1);
    assert_eq!(outcome.document["sides"][0]["verified"], true);
    assert_eq!(outcome.document["sides"][1]["verified"], false);
    assert!(!output.exists());
}

/// An Edit Set that does not apply to the common Take at all.
#[test]
fn a_side_that_does_not_replay_refuses_as_source_replay_failed() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let mut b = side(dir.path(), "B", &a, &[vel(P60, 60)]);
    let claimed = dir.path().join("claimed.json");
    write_edits(&claimed, &[vel("t0:c0:p61:s0:n0", 60)]);
    b.edits_path = claimed;
    let c = side(dir.path(), "C", &a, &[]);
    let output = dir.path().join("D.mid");
    let outcome = combine(&a, [&b, &c], &output);
    assert_eq!(outcome.document["sides"][0]["verified"], false);
    assert_eq!(outcome.document["sides"][1]["verified"], Value::Null);
    let refusal = refused(dir.path(), &a, [&b, &c], "source_replay_failed");
    assert_eq!(refusal["side"], 0);
    assert!(refusal["detail"]
        .as_str()
        .expect("a detail")
        .contains("t0:c0:p61:s0:n0"));
}

// --------------------------------------------------------- net-zero resize --

/// p60 [0, 100] with a CC11 written in front of its release at 100.
fn cc_a(dir: &Path, number: u8) -> PathBuf {
    synth(
        &dir.join("A.mid"),
        &[(0, 100, 60, 80, 10)],
        &[controller(100, number, 90)],
        1000,
    )
}

/// `+50` then `-50` is a relocation: the release comes back to 100 in front of
/// the CC11, and B + empty reproduces B event for event (#51, confirmed
/// 2026-09-30 19:02). Its duration request reports no effect.
#[test]
fn a_net_zero_resize_is_a_relocation_and_b_plus_empty_is_b() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = cc_a(dir.path(), 11);
    let b = side(dir.path(), "B", &a, &[resize(P60, 50), resize(P60, -50)]);
    let c = side(dir.path(), "C", &a, &[]);
    assert_eq!(written_at(&a, 100), vec!["cc11", "off60"]);
    let (report, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(written_at(&d, 100), vec!["off60", "cc11"]);
    assert_eq!(
        report["no_effect"],
        json!([{ "side": 0, "id": P60, "field": "duration" }])
    );
}

/// The same across a damper: the release moves in front of the CC64, which
/// is inherited from B with no refusal. `mid diff A D` is where the human sees
/// it — not as this suite's oracle, but as the surface the order is resolved
/// on.
#[test]
fn a_net_zero_resize_across_a_damper_is_inherited_and_diff_reports_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = cc_a(dir.path(), 64);
    let b = side(dir.path(), "B", &a, &[resize(P60, 50), resize(P60, -50)]);
    let c = side(dir.path(), "C", &a, &[]);
    let (_, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(written_at(&d, 100), vec!["off60", "cc64"]);
    let diff: Value = serde_json::from_str(&common::json_output(&[
        "diff",
        a.to_str().unwrap(),
        d.to_str().unwrap(),
        "--json",
    ]))
    .expect("JSON");
    assert_eq!(
        diff["rank_disagreements"][0]["pair"],
        "damper_after_release"
    );
}

/// A `resize_note` of 0 moves nothing, so it relocates nothing: it is a
/// request with no effect, carries no order claim, and cannot contradict a
/// side that did move the release across a damper. The pair the #56 stop
/// found (`.scratch/56-combine/zero-resize/`), as the owner ruled it in #56's
/// Decision of 2026-10-01: D is B.
#[test]
fn a_zero_resize_is_no_effect_and_claims_no_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = cc_a(dir.path(), 64);
    let b = side(dir.path(), "B", &a, &[resize(P60, 50), resize(P60, -50)]);
    let c = side(dir.path(), "C", &a, &[resize(P60, 0)]);
    assert!(events_equal(&c.take, &a), "a zero resize changed the Take");
    let (report, d) = candidate(dir.path(), &a, [&b, &c]);
    assert!(events_equal(&d, &b.take), "D is not B");
    assert_eq!(written_at(&d, 100), vec!["off60", "cc64"]);
    assert_eq!(report["disclosed_sites"], json!([]));
    assert!(
        report["no_effect"]
            .as_array()
            .expect("an array")
            .contains(&json!({ "side": 1, "id": P60, "field": "duration" })),
        "{}",
        report["no_effect"]
    );
}

/// p60 and p64 both [0, 100], CC11 in front of both releases.
fn two_a(dir: &Path, p64_end: u32) -> PathBuf {
    synth(
        &dir.join("A.mid"),
        &[(0, 100, 60, 80, 10), (0, p64_end, 64, 80, 20)],
        &[controller(100, 11, 90)],
        1000,
    )
}

/// Each side net-zeroes a different note at one Tick: each puts its own
/// release at the front, the two orders are opposite on a pair the rule does
/// not rank, and that is one disclosed site and no refusal.
#[test]
fn two_net_zero_resizes_at_one_tick_disclose_one_site() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = two_a(dir.path(), 100);
    let p64 = "t0:c0:p64:s0:n0";
    let b = side(dir.path(), "B", &a, &[resize(P60, 50), resize(P60, -50)]);
    let c = side(dir.path(), "C", &a, &[resize(p64, 50), resize(p64, -50)]);
    let (report, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(
        report["disclosed_sites"],
        json!([{ "track": 0, "tick": 100, "channel": 0 }])
    );
    assert_eq!(written_at(&d, 100), vec!["off60", "off64", "cc11"]);
}

#[test]
fn a_net_zero_resize_beside_a_shortening_onto_its_tick() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = two_a(dir.path(), 120);
    let b = side(dir.path(), "B", &a, &[resize(P60, 50), resize(P60, -50)]);
    let c = side(dir.path(), "C", &a, &[resize("t0:c0:p64:s0:n0", -20)]);
    let (report, _) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(report["disclosed_sites"], json!([]));
}

#[test]
fn a_net_zero_resize_beside_a_lengthening_away_from_its_tick() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = two_a(dir.path(), 100);
    let b = side(dir.path(), "B", &a, &[resize(P60, 50), resize(P60, -50)]);
    let c = side(dir.path(), "C", &a, &[resize("t0:c0:p64:s0:n0", 20)]);
    let (report, _) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(report["disclosed_sites"], json!([]));
}

// ------------------------------------------------------------- track ends --

/// Lengthening past the End-of-Track moves it, and the report says so with
/// the track and both Ticks — checked against the file written.
#[test]
fn a_track_end_extension_is_reported_and_matches_the_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = synth(
        &dir.path().join("A.mid"),
        &[(0, 100, 60, 80, 10), (200, 900, 64, 80, 20)],
        &[],
        1000,
    );
    let b = side(dir.path(), "B", &a, &[resize("t0:c0:p64:s200:n0", 300)]);
    let c = side(dir.path(), "C", &a, &[vel(P60, 50)]);
    let (report, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(
        report["track_ends"],
        json!([{ "track": 0, "before": 1000, "after": 1200 }])
    );
    let last = parse(&d).tracks[0].last().cloned().expect("an event");
    assert_eq!(last.tick, 1200);
    assert!(last.key.contains("EndOfTrack"));
}

/// Deleting and shortening never move a track's end, and nothing is reported.
#[test]
fn deletion_and_shortening_never_move_a_track_end() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = synth(
        &dir.path().join("A.mid"),
        &[(0, 100, 60, 80, 10), (200, 1000, 64, 80, 20)],
        &[],
        1000,
    );
    let b = side(dir.path(), "B", &a, &[delete("t0:c0:p64:s200:n0")]);
    let c = side(dir.path(), "C", &a, &[resize(P60, -50)]);
    let (report, d) = candidate(dir.path(), &a, [&b, &c]);
    assert_eq!(report["track_ends"], json!([]));
    assert_eq!(parse(&d).tracks[0].last().expect("an event").tick, 1000);
}

// --------------------------------------------------- order contradictions --

/// Within the subset no ranked pair can be contradicted: `apply` puts every
/// relocated release at the front of its Tick and moves nothing else, so two
/// sides that both relocated a release past a damper agree on its side of the
/// damper, and a Program is never moved at all (#51 Decision). This is that
/// claim tested rather than argued: every pairing of a spread of subset Edit
/// Sets over a Take carrying a damper and a Program, each written on the far
/// side of what it governs, and not one of them refuses on order.
#[test]
fn an_order_contradiction_is_unreachable_within_the_subset() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = synth(
        &dir.path().join("A.mid"),
        &[
            (0, 100, 60, 80, 10),
            (0, 100, 64, 80, 20),
            (100, 200, 67, 80, 30),
            (200, 300, 72, 80, 40),
        ],
        &[controller(100, 64, 127), program(200, 1, 40)],
        1000,
    );
    let (p60, p64, p67) = (P60, "t0:c0:p64:s0:n0", "t0:c0:p67:s100:n0");
    let sets: Vec<Vec<E>> = vec![
        vec![],
        vec![resize(p60, 50), resize(p60, -50)],
        vec![resize(p64, 50), resize(p64, -50)],
        vec![resize(p60, -50), resize(p60, 50)],
        vec![resize(p60, 100)],
        vec![resize(p64, 100), resize(p60, 100)],
        vec![resize(p67, -50), resize(p67, 50)],
        vec![resize(p67, 50)],
        vec![vel(p60, 30), resize(p64, 0)],
        vec![delete(p64)],
    ];
    let built: Vec<Side> = sets
        .iter()
        .enumerate()
        .map(|(i, edits)| side(dir.path(), &format!("S{i}"), &a, edits))
        .collect();
    let mut outcomes = BTreeMap::new();
    for (i, first) in built.iter().enumerate() {
        for (j, second) in built.iter().enumerate() {
            let output = dir.path().join(format!("D-{i}-{j}.mid"));
            let outcome = combine(&a, [first, second], &output);
            let kind = if outcome.success {
                "candidate".to_string()
            } else {
                outcome.document["refusal"]["kind"]
                    .as_str()
                    .expect("a refusal kind")
                    .to_string()
            };
            assert_ne!(
                kind, "order_contradiction",
                "S{i} + S{j} refused on order: {}",
                outcome.stderr
            );
            *outcomes.entry(kind).or_insert(0) += 1;
        }
    }
    // The spread reached both the agreeing and the refusing paths, so the
    // absence above is not an absence of attempts.
    assert!(outcomes.contains_key("candidate"), "{outcomes:?}");
    assert!(outcomes.contains_key("conflict"), "{outcomes:?}");
}

// ----------------------------------------------------------- real fixtures --

fn inspected(path: &str) -> Vec<Value> {
    common::notes(&common::inspect_json(Path::new(path)))
}

fn id(n: &Value) -> String {
    n["id"].as_str().expect("an id").to_string()
}

fn int(n: &Value, field: &str) -> i64 {
    n[field].as_i64().expect("a number")
}

/// Two sides each carrying one release of consecutive notes onto the start of
/// a later note on the same track: B takes note k, C note k+1, both ending
/// where note k+3 is struck. The probe's construction of a relation that
/// first appears in D.
fn meet_on_track(notes: &[Value], track: i64, which: usize) -> Vec<E> {
    let mut row: Vec<&Value> = notes.iter().filter(|n| int(n, "track") == track).collect();
    row.sort_by_key(|n| int(n, "start"));
    let mut edits = Vec::new();
    let mut k = 0;
    while edits.len() < 8 && k + 3 < row.len() {
        let (a, b, target) = (row[k + which], row[k + 1 - which], row[k + 3]);
        let crosses = row[k..k + 4].iter().any(|n| {
            int(n, "pitch") == int(a, "pitch")
                && int(a, "start") < int(n, "start")
                && int(n, "start") <= int(target, "start")
        });
        if int(a, "pitch") != int(b, "pitch")
            && int(target, "start") > int(a, "start").max(int(b, "start"))
            && !crosses
        {
            edits.push(resize(
                &id(a),
                int(target, "start") - (int(a, "start") + int(a, "duration")),
            ));
        }
        k += 4;
    }
    edits
}

/// The five recipes each real Take runs, from the parts of it they reach.
struct Recipes {
    path: &'static str,
    /// Notes to shorten and to lengthen.
    lead: Vec<Value>,
    /// Notes a second side changes in the same recipes.
    other: Vec<Value>,
    /// The track `meet_on_track` reaches.
    track: i64,
}

fn real_cases(recipes: Recipes) {
    let a = Path::new(recipes.path);
    let lead: Vec<&Value> = recipes.lead.iter().take(20).collect();
    let other: Vec<&Value> = recipes.other.iter().take(30).collect();
    let notes = inspected(recipes.path);

    let cases: Vec<(&str, Vec<E>, Vec<E>)> = vec![
        (
            "shortened + velocity",
            lead.iter()
                .map(|n| resize(&id(n), -(int(n, "duration") / 4).max(1)))
                .collect(),
            other.iter().map(|n| vel(&id(n), 70)).collect(),
        ),
        (
            "delete + soften",
            other.iter().map(|n| delete(&id(n))).collect(),
            lead.iter()
                .map(|n| vel(&id(n), (int(n, "velocity") - 30).max(1)))
                .collect(),
        ),
        (
            "two sides lengthen different notes",
            lead.iter().map(|n| resize(&id(n), 40)).collect(),
            other.iter().map(|n| resize(&id(n), 40)).collect(),
        ),
        (
            "lengthened + empty equals the side",
            lead.iter().map(|n| resize(&id(n), 40)).collect(),
            vec![],
        ),
        (
            "releases meet on a later strike",
            meet_on_track(&notes, recipes.track, 0),
            meet_on_track(&notes, recipes.track, 1),
        ),
    ];
    for (name, b_edits, c_edits) in cases {
        let dir = tempfile::tempdir().expect("temp dir");
        let b = side(dir.path(), "B", a, &b_edits);
        let c = side(dir.path(), "C", a, &c_edits);
        let (report, _) = candidate(dir.path(), a, [&b, &c]);
        assert_eq!(
            report["disclosed_sites"],
            json!([]),
            "{} {name}",
            recipes.path
        );
    }
}

/// Bach BWV 208: the melody is track 1 and the bass track 4.
#[test]
fn the_bach_fixture_runs_the_five_cases() {
    let notes = inspected(BACH);
    let on = |track: i64| -> Vec<Value> {
        notes
            .iter()
            .filter(|n| int(n, "track") == track)
            .cloned()
            .collect()
    };
    real_cases(Recipes {
        path: BACH,
        lead: on(1),
        other: on(4),
        track: 1,
    });
}

/// The Groove capture: one track of drums; kicks are 36, snares 38 and
/// closed hats 42.
#[test]
fn the_groove_fixture_runs_the_five_cases() {
    let notes = inspected(GROOVE);
    let pitched = |pitch: i64| -> Vec<Value> {
        notes
            .iter()
            .filter(|n| int(n, "pitch") == pitch)
            .take(40)
            .cloned()
            .collect()
    };
    real_cases(Recipes {
        path: GROOVE,
        lead: pitched(36),
        other: pitched(38),
        track: 0,
    });
    // And the probe's own delete-and-soften: closed hats against snares.
    let dir = tempfile::tempdir().expect("temp dir");
    let a = Path::new(GROOVE);
    let b = side(
        dir.path(),
        "B",
        a,
        &pitched(42)
            .iter()
            .map(|n| delete(&id(n)))
            .collect::<Vec<_>>(),
    );
    let c = side(
        dir.path(),
        "C",
        a,
        &pitched(38)
            .iter()
            .map(|n| vel(&id(n), (int(n, "velocity") - 30).max(1)))
            .collect::<Vec<_>>(),
    );
    candidate(dir.path(), a, [&b, &c]);
}

// ---------------------------------------------------------------- the CLI --

#[test]
fn combine_has_its_own_help() {
    mid()
        .args(["combine", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--side"))
        .stdout(predicates::str::contains("never an input"));
}

/// Without `--json`: what was verified and what the library states go to
/// stderr, one line each, and the path written is the one line on stdout.
#[test]
fn the_human_form_states_facts_on_stderr_and_the_path_on_stdout() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = two_a(dir.path(), 100);
    let p64 = "t0:c0:p64:s0:n0";
    let b = side(dir.path(), "B", &a, &[resize(P60, 50), resize(P60, -50)]);
    let c = side(dir.path(), "C", &a, &[resize(p64, 50), resize(p64, -50)]);
    let d = dir.path().join("D.mid");
    let out = mid()
        .arg("combine")
        .arg(&a)
        .arg("--side")
        .arg(&b.take)
        .arg(&b.edits_path)
        .arg("--side")
        .arg(&c.take)
        .arg(&c.edits_path)
        .arg("-o")
        .arg(&d)
        .output()
        .expect("mid runs");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("{}\n", d.display())
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(
        lines.iter().filter(|l| l.starts_with("verified: ")).count(),
        2
    );
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.starts_with("no effect: "))
            .count(),
        2
    );
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.starts_with("disclosed: "))
            .count(),
        1
    );
}

/// A refusal prints nothing on stdout without `--json`, and lists each
/// conflict on stderr before the refusal itself.
#[test]
fn a_human_refusal_prints_nothing_on_stdout() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let b = side(dir.path(), "B", &a, &[vel(P60, 60)]);
    let c = side(dir.path(), "C", &a, &[vel(P60, 70)]);
    let d = dir.path().join("D.mid");
    let out = mid()
        .arg("combine")
        .arg(&a)
        .arg("--side")
        .arg(&b.take)
        .arg(&b.edits_path)
        .arg("--side")
        .arg(&c.take)
        .arg(&c.edits_path)
        .arg("-o")
        .arg(&d)
        .output()
        .expect("mid runs");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("conflict: t0:c0:p60:s0:n0 velocity"),
        "{stderr}"
    );
    assert!(stderr.contains("mid: "), "{stderr}");
    assert!(!d.exists());
}

#[test]
fn combine_never_writes_over_an_input() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let b = side(dir.path(), "B", &a, &[vel(P60, 60)]);
    let c = side(dir.path(), "C", &a, &[]);
    let before = std::fs::read(&b.take).expect("readable");
    for output in [&a, &b.take, &c.edits_path] {
        mid()
            .arg("combine")
            .arg(&a)
            .arg("--side")
            .arg(&b.take)
            .arg(&b.edits_path)
            .arg("--side")
            .arg(&c.take)
            .arg(&c.edits_path)
            .arg("-o")
            .arg(output)
            .assert()
            .failure()
            .stderr(predicates::str::contains("never writes in place"));
    }
    assert_eq!(std::fs::read(&b.take).expect("readable"), before);
}

#[test]
fn combine_takes_exactly_two_sides() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let b = side(dir.path(), "B", &a, &[]);
    let d = dir.path().join("D.mid");
    mid()
        .arg("combine")
        .arg(&a)
        .arg("--side")
        .arg(&b.take)
        .arg(&b.edits_path)
        .arg("-o")
        .arg(&d)
        .assert()
        .failure()
        .stderr(predicates::str::contains("exactly two"));
    assert!(!d.exists());
}

/// Outside the subset is refused, not guessed at.
#[test]
fn an_edit_outside_the_subset_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = plain_a(dir.path());
    let edits = dir.path().join("moved.json");
    std::fs::write(
        &edits,
        json!({ "edits": [ { "kind": "move_note", "id": P60, "delta_ticks": 10 } ] }).to_string(),
    )
    .expect("writable");
    let moved = dir.path().join("moved.mid");
    mid()
        .arg("apply")
        .arg(&a)
        .arg(&edits)
        .arg("-o")
        .arg(&moved)
        .assert()
        .success();
    let c = side(dir.path(), "C", &a, &[]);
    let d = dir.path().join("D.mid");
    mid()
        .arg("combine")
        .arg(&a)
        .arg("--side")
        .arg(&moved)
        .arg(&edits)
        .arg("--side")
        .arg(&c.take)
        .arg(&c.edits_path)
        .arg("-o")
        .arg(&d)
        .assert()
        .failure()
        .stderr(predicates::str::contains("Edit 0"));
    assert!(!d.exists());
}

// ------------------------------------------------- the subset's contract --

/// Every Edit kind there is, read out of `battuta::EditSet` the way
/// `tests/contract.rs` reads it: from the variants `serde` would have taken.
fn every_kind() -> BTreeSet<String> {
    let refusal = serde_json::from_str::<battuta::EditSet>(r#"{ "edits": [ { "kind": "" } ] }"#)
        .expect_err("an empty kind is not a kind")
        .to_string();
    let (_, listed) = refusal
        .split_once("expected one of ")
        .expect("serde names the variants it would have taken");
    listed
        .split(&[',', ' '][..])
        .filter_map(|word| word.trim().strip_prefix('`'))
        .filter_map(|word| word.split('`').next())
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// `mid combine --help` names the kinds of Edit `combine` takes up, and
/// nothing else is allowed to: the refusal of any other kind points there.
/// This holds that sentence to the binary in both directions, the way
/// `tests/contract.rs` holds `apply --help`.
///
/// Neither side of the comparison is written down here. The kinds there are
/// come out of the `Edit` type; the example of each is the one
/// `fixtures/every-kind.json` holds (which `tests/contract.rs` keeps one per
/// kind); and whether `combine` takes a kind up is found by driving that one
/// Edit through `mid combine` as an Alternative and seeing whether it is
/// refused as a kind combine does not take up. An example that does not apply
/// to the common Take as written is tried with its `delta_ticks` negated — the
/// fixture's `move_note` moves the first note before Tick 0 — and a kind that
/// still cannot be driven fails here rather than being skipped.
#[test]
fn combine_help_names_exactly_the_kinds_combine_takes_up() {
    let dir = tempfile::tempdir().expect("temp dir");
    let common = Path::new(common::EXPRESSIVE);
    let text = std::fs::read_to_string("fixtures/every-kind.json").expect("the fixture is there");
    let examples: Vec<Value> = serde_json::from_str::<Value>(&text).expect("JSON")["edits"]
        .as_array()
        .expect("an Edit list")
        .clone();
    let empty = dir.path().join("empty.json");
    write_edits(&empty, &[]);

    let mut taken = BTreeSet::new();
    let mut driven = BTreeSet::new();
    for example in examples {
        let kind = example["kind"].as_str().expect("a kind").to_string();
        let mut negated = example.clone();
        if let Some(delta) = example["delta_ticks"].as_i64() {
            negated["delta_ticks"] = json!(-delta);
        }
        let edits = dir.path().join(format!("{kind}.json"));
        let take = dir.path().join(format!("{kind}.mid"));
        let applies = [example, negated].into_iter().any(|edit| {
            std::fs::write(&edits, json!({ "edits": [edit] }).to_string()).expect("writable");
            mid()
                .arg("apply")
                .arg(common)
                .arg(&edits)
                .arg("-o")
                .arg(&take)
                .output()
                .expect("mid runs")
                .status
                .success()
        });
        assert!(applies, "the example of `{kind}` applies to no common Take");

        let out = mid()
            .arg("combine")
            .arg(common)
            .arg("--side")
            .arg(&take)
            .arg(&edits)
            .arg("--side")
            .arg(common)
            .arg(&empty)
            .arg("-o")
            .arg(dir.path().join(format!("D-{kind}.mid")))
            .output()
            .expect("mid runs");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.contains("does not apply to the common Take")
                && !stderr.contains("is not what its Edit Set makes"),
            "`{kind}` never reached the subset check: {stderr}"
        );
        if !stderr.contains("combine does not take up") {
            taken.insert(kind.clone());
        }
        driven.insert(kind);
    }
    assert_eq!(
        driven,
        every_kind(),
        "every kind was driven through combine"
    );

    let help = String::from_utf8(
        mid()
            .args(["combine", "--help"])
            .output()
            .expect("mid runs")
            .stdout,
    )
    .expect("UTF-8");
    let named: BTreeSet<String> = every_kind()
        .into_iter()
        .filter(|kind| help.contains(&format!("`{kind}`")))
        .collect();
    assert_eq!(
        named, taken,
        "`mid combine --help` names other kinds than combine takes up"
    );
}
