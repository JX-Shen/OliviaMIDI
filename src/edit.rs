use crate::error::{Error, Result};
use crate::note::{Note, NoteId};
use crate::take::Take;
use crate::track::Rewrite;
use midly::num::{u4, u7};
use midly::{MidiMessage, TrackEventKind};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// A batch of Edits, applied together. The on-disk form is `edits.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditSet {
    pub edits: Vec<Edit>,
}

/// One mechanical change to a Take.
///
/// Mechanical is the whole point: an Edit says which note and what number, never
/// what the change is *for*. Musical intent belongs to the agent holding it, and
/// this enum is where it would leak in if it ever did.
///
/// The discriminator is `kind`, not `op`: an Edit Set contains Edits, and a key
/// that called them operations would seed the word into every sentence written
/// about them. See `CONTEXT.md` under **Edit**.
///
/// Every number is `i64` on purpose. A field typed to what MIDI can hold would
/// turn an out-of-range value into a JSON parse failure that does not say which
/// number was wrong; kept wide, the number is reported as itself.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    SetVelocity {
        id: NoteId,
        velocity: i64,
    },
    TransposeNote {
        id: NoteId,
        semitones: i64,
    },
    MoveNote {
        id: NoteId,
        delta_ticks: i64,
    },
    ResizeNote {
        id: NoteId,
        delta_ticks: i64,
    },
    DeleteNote {
        id: NoteId,
    },
    AddNote {
        track: i64,
        channel: i64,
        pitch: i64,
        start: i64,
        duration: i64,
        velocity: i64,
    },
    /// Which Program a channel is on from a Tick: the first Edit that names no
    /// note at all.
    ///
    /// It states an address rather than naming an identity, because there may be
    /// nothing there yet to name — a Take that states no Program for this
    /// channel is the ordinary case, and it is the one where saying so matters
    /// most. ADR-0002's content addressing is for notes; this does not go
    /// through it and is not an exception to it.
    ///
    /// The track is stated rather than inferred. Which track carries a channel's
    /// program change is the author's arrangement of the file, and a tool that
    /// guessed would move somebody's orchestration onto a track they did not put
    /// it on.
    SetProgram {
        track: i64,
        channel: i64,
        tick: i64,
        program: i64,
    },
}

impl EditSet {
    pub fn read(path: &Path) -> Result<EditSet> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|source| Error::EditSetUnreadable {
            path: path.to_path_buf(),
            source,
        })
    }
}

/// What one Edit does to the note it named.
///
/// `add_note` is deliberately not here, and neither is `set_program`. Neither
/// names a note — one creates a note, the other states which Program a channel
/// is on — and keeping them apart is what lets every match below be exhaustive:
/// a kind fitting neither shape cannot be routed into the wrong one, because
/// there is nowhere for it to go until somebody says where. The kind after these
/// is already named too — `CHARTER.md` lists changing a CC among the Edits, and
/// a CC Edit names no note either.
enum Change {
    Velocity(i64),
    Transpose(i64),
    Move(i64),
    Resize(i64),
    Delete,
}

/// The note `add_note` states. Wide numbers, for the reason `Edit`'s are.
struct NewNote {
    track: i64,
    channel: i64,
    pitch: i64,
    start: i64,
    duration: i64,
    velocity: i64,
}

/// The Program `set_program` states. Wide numbers, for the reason `Edit`'s are.
struct NewProgram {
    track: i64,
    channel: i64,
    tick: i64,
    program: i64,
}

/// An Edit once it has been looked up against the input Take. Three shapes,
/// because there are three sorts of Edit: those naming a note, the one stating a
/// note, and the one stating what a channel plays.
enum Landing {
    Named(Change, Note),
    Stated(NewNote),
    Orchestrated(NewProgram),
}

/// Every identity in the Edit Set, resolved against the input Take before any
/// effect lands (ADR-0002).
///
/// This is the one place the six kinds are told apart. Five name a note and are
/// paired here with the note they named. `add_note` names none: it is the only
/// kind that creates an identity, so it states a note rather than naming one,
/// and it is the only kind with nothing to look up. A consequence falls straight
/// out — an Edit can never name a note an earlier `add_note` created, because
/// that note is not in the list being searched.
fn resolve(notes: &[Note], edits: &[Edit]) -> Result<Vec<Landing>> {
    let named = |id: &NoteId, change| {
        notes
            .iter()
            .find(|note| &note.id == id)
            .cloned()
            .map(|note| Landing::Named(change, note))
            .ok_or_else(|| Error::UnknownNote(id.to_string()))
    };
    edits
        .iter()
        .map(|edit| match *edit {
            Edit::SetVelocity { ref id, velocity } => named(id, Change::Velocity(velocity)),
            Edit::TransposeNote { ref id, semitones } => named(id, Change::Transpose(semitones)),
            Edit::MoveNote {
                ref id,
                delta_ticks,
            } => named(id, Change::Move(delta_ticks)),
            Edit::ResizeNote {
                ref id,
                delta_ticks,
            } => named(id, Change::Resize(delta_ticks)),
            Edit::DeleteNote { ref id } => named(id, Change::Delete),
            Edit::AddNote {
                track,
                channel,
                pitch,
                start,
                duration,
                velocity,
            } => Ok(Landing::Stated(NewNote {
                track,
                channel,
                pitch,
                start,
                duration,
                velocity,
            })),
            // Nothing to look up: the address may name a moment the Take says
            // nothing at, which is the case this kind exists for.
            Edit::SetProgram {
                track,
                channel,
                tick,
                program,
            } => Ok(Landing::Orchestrated(NewProgram {
                track,
                channel,
                tick,
                program,
            })),
        })
        .collect()
}

/// Apply an Edit Set to a Take, producing a new one. The input is never touched.
///
/// Every identity is resolved before the first effect lands, so Edits apply in
/// the order given with their effects ordered while their targets were all fixed
/// in advance.
pub fn apply(take: &Take, edit_set: &EditSet) -> Result<Take> {
    let notes = take.notes()?;
    let resolved = resolve(&notes, &edit_set.edits)?;

    let mut smf = take.smf()?;
    // Every track is opened before the first Edit lands, and every Edit reaches
    // its events by the index its identity resolved to. Those indices stay valid
    // for the whole loop because nothing here ever shifts one — see `track`.
    let mut tracks: Vec<Rewrite> = smf.tracks.iter().map(|track| Rewrite::of(track)).collect();

    // Every note the finished Take will hold, named by the two slots that carry
    // it rather than by an identity: identities are what the finished Take is
    // read *for*, and this list is what says whether reading it will come back
    // with the notes that were put in.
    let mut sounding: Vec<NoteSlots> = notes
        .iter()
        .map(|note| NoteSlots {
            track: note.track,
            on: note.on_event,
            off: note.off_event,
        })
        .collect();

    for landing in resolved {
        match landing {
            Landing::Named(change, note) => change_note(&mut tracks[note.track], change, &note)?,
            Landing::Stated(new) => sounding.push(add(&mut tracks, new)?),
            Landing::Orchestrated(new) => set_program(&mut tracks, new)?,
        }
    }
    stay_distinct(&tracks, &sounding)?;

    smf.tracks = tracks
        .into_iter()
        .map(Rewrite::finish)
        .collect::<Result<_>>()?;
    Take::from_smf(&smf)
}

/// The two slots carrying one note of the Take being built.
struct NoteSlots {
    track: usize,
    on: usize,
    off: usize,
}

/// What a note-off can name, and so the group inside which two notes can be
/// mistaken for one another: a track, a channel and a pitch.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Addressed {
    track: usize,
    channel: u8,
    pitch: u8,
}

/// One note as this check sees it: where its strike lands among the events of
/// its Tick, and the Tick its release falls on.
///
/// The strike carries its rank because two notes struck together still begin in
/// an order. The release carries only its Tick, because two released together
/// hand back the same length whichever of them a reader takes first.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Sounding {
    strike: (u32, i64),
    released: u32,
}

/// Refuse a Take whose notes would not come back the way they went in.
///
/// A note-off names a channel and a pitch, never the note-on it ends, so a
/// reader hands the first release to the earliest note of that channel and pitch
/// still sounding. That is right only while such notes finish in the order they
/// began. Let one finish inside another and re-reading the Take gives each of
/// them the other's length — an `apply` that succeeded, a file that plays, and
/// two lengths quietly swapped. It cannot even be undone: a later `delete_note`
/// would take the release the reader had given to the *other* note.
///
/// Notes no Edit touched cannot reach this. They kept the slots they were paired
/// in, and pairing produced them in finishing order to begin with — so anything
/// caught here is something an Edit did.
fn stay_distinct(tracks: &[Rewrite], sounding: &[NoteSlots]) -> Result<()> {
    let mut grouped: BTreeMap<Addressed, Vec<Sounding>> = BTreeMap::new();
    for note in sounding {
        let track = &tracks[note.track];
        if !track.holds(note.on) {
            continue;
        }
        let Some((channel, pitch)) = track.struck(note.on) else {
            continue;
        };
        grouped
            .entry(Addressed {
                track: note.track,
                channel,
                pitch,
            })
            .or_default()
            .push(Sounding {
                strike: track.place(note.on),
                released: track.tick(note.off),
            });
    }

    for (addressed, mut notes) in grouped {
        notes.sort();
        // Struck in this order, they have to be released in it too: a reader
        // hands the first release to the earliest note still sounding, so one
        // that began first and ends last would be handed the shorter length.
        for pair in notes.windows(2) {
            if pair[0].released > pair[1].released {
                return Err(Error::NotesIndistinguishable {
                    track: addressed.track,
                    channel: addressed.channel,
                    pitch: addressed.pitch,
                    first: pair[0].strike.0,
                    second: pair[1].strike.0,
                });
            }
        }
    }
    Ok(())
}

/// One change, applied to the track the note it names is on.
fn change_note(track: &mut Rewrite, change: Change, note: &Note) -> Result<()> {
    // Every identity was resolved against the input Take, so one an earlier Edit
    // deleted still resolves — it simply has nowhere left for an effect to land.
    // Refused rather than quietly skipped.
    if !track.holds(note.on_event) {
        return Err(Error::NoteAlreadyDeleted(note.id.to_string()));
    }
    let lost = || Error::NoteEventsLost(note.id.to_string());

    match change {
        Change::Velocity(velocity) => {
            let velocity = midi_value(velocity, 127)
                .filter(|velocity| *velocity >= 1)
                .ok_or(Error::VelocityOutOfRange(velocity))?;
            track
                .set_velocity(note.on_event, velocity)
                .ok_or_else(lost)?;
        }

        // A note's key is on both of its events. Changing only the note-on would
        // leave a note struck and never released, and the Take would stop being
        // readable at all.
        Change::Transpose(semitones) => {
            let landed = i64::from(track.key(note.on_event).ok_or_else(lost)?) + semitones;
            let pitch = midi_value(landed, 127).ok_or(Error::TransposeOutOfRange {
                id: note.id.to_string(),
                semitones,
                landed,
            })?;
            for index in [note.on_event, note.off_event] {
                track.set_key(index, pitch).ok_or_else(lost)?;
                track.place_again(index);
            }
        }

        // Both events move by the same amount, so the note keeps its length. The
        // delta times around them are re-derived at the end, from Ticks: nothing
        // here touches a neighbour.
        Change::Move(delta_ticks) => {
            for index in [note.on_event, note.off_event] {
                let landed = i64::from(track.tick(index)) + delta_ticks;
                let tick = u32::try_from(landed).map_err(|_| Error::MoveOutOfRange {
                    id: note.id.to_string(),
                    delta_ticks,
                    landed,
                })?;
                track.set_tick(index, tick);
                track.place_again(index);
            }
        }

        // The note-off alone moves, so the note keeps its start and with it its
        // identity — which is why it is deliberately not placed again. Placing a
        // note again is how an Edit that *changes* an identity keeps from
        // renumbering the notes already at a Tick, and a resize changes no
        // identity at all, so it has nothing to get out of the way of.
        Change::Resize(delta_ticks) => {
            let landed = i64::from(track.tick(note.off_event)) + delta_ticks;
            let duration = landed - i64::from(track.tick(note.on_event));
            let end = u32::try_from(landed)
                .ok()
                .filter(|_| duration >= 1)
                .ok_or_else(|| Error::ResizeOutOfRange {
                    id: note.id.to_string(),
                    delta_ticks,
                    duration,
                })?;
            track.set_tick(note.off_event, end);
        }

        // Both events go or neither does. A note-off left behind would release a
        // note nothing in the Take ever struck.
        Change::Delete => {
            track.remove(note.on_event);
            track.remove(note.off_event);
        }
    }
    Ok(())
}

/// The one kind that creates a note rather than naming one.
///
/// Nothing states the new note's identity. It falls out of where the note lands,
/// derived from track, channel, pitch and start exactly as every other note's is
/// — which is what makes it indistinguishable, on a later `inspect`, from a note
/// that was always there.
fn add(tracks: &mut [Rewrite], new: NewNote) -> Result<NoteSlots> {
    let index = usize::try_from(new.track)
        .ok()
        .filter(|index| *index < tracks.len())
        .ok_or(Error::NoSuchTrack {
            track: new.track,
            tracks: tracks.len(),
        })?;
    let channel = midi_value(new.channel, 15).ok_or(Error::ChannelOutOfRange(new.channel))?;
    let pitch = midi_value(new.pitch, 127).ok_or(Error::PitchOutOfRange(new.pitch))?;
    // Velocity 0 is refused with the rest of the out-of-range values, and for a
    // sharper reason than range: a note-on at velocity 0 is how the format
    // spells a note-off, so it would add a release rather than a note.
    let velocity = midi_value(new.velocity, 127)
        .filter(|velocity| *velocity >= 1)
        .ok_or(Error::VelocityOutOfRange(new.velocity))?;
    let start = u32::try_from(new.start).map_err(|_| Error::StartOutOfRange(new.start))?;
    let end = new
        .start
        .checked_add(new.duration)
        .filter(|_| new.duration >= 1)
        .and_then(|end| u32::try_from(end).ok())
        .ok_or(Error::DurationOutOfRange(new.duration))?;

    let on_channel = |message| TrackEventKind::Midi {
        channel: u4::new(channel),
        message,
    };
    let track = &mut tracks[index];
    Ok(NoteSlots {
        track: index,
        on: track.push(
            start,
            on_channel(MidiMessage::NoteOn {
                key: u7::new(pitch),
                vel: u7::new(velocity),
            }),
        ),
        off: track.push(
            end,
            on_channel(MidiMessage::NoteOff {
                key: u7::new(pitch),
                vel: u7::new(0),
            }),
        ),
    })
}

/// The one kind that changes the orchestration rather than a note.
///
/// What it means is a state: from this Tick, this channel is on this Program. So
/// it changes the statement the Take makes at that Tick when there is one, and
/// inserts one when there is not — a Take that states no Program for a channel
/// is the ordinary case and must not need a different Edit.
///
/// What it deliberately does not do is reach a *later* statement. "From this
/// Tick" is true until the Take says otherwise, and the Take may say otherwise
/// two Bars later; silently deleting that would be an Edit changing something it
/// was not asked about, and an Edit stays mechanical. What the passage is
/// actually on afterwards is what `inspect` reports and `diff` compares, which
/// is where a reader finds out.
fn set_program(tracks: &mut [Rewrite], new: NewProgram) -> Result<()> {
    let index = usize::try_from(new.track)
        .ok()
        .filter(|index| *index < tracks.len())
        .ok_or(Error::NoSuchTrack {
            track: new.track,
            tracks: tracks.len(),
        })?;
    let channel = midi_value(new.channel, 15).ok_or(Error::ChannelOutOfRange(new.channel))?;
    let program = midi_value(new.program, 127).ok_or(Error::ProgramOutOfRange(new.program))?;
    let tick = u32::try_from(new.tick).map_err(|_| Error::ProgramTickOutOfRange(new.tick))?;

    let track = &mut tracks[index];
    match track.program_at(channel, tick) {
        Some(stated) => {
            // `program_at` found a program change, so the setter cannot decline
            // this one. Its `Option` is honesty about being handed any index at
            // all, not a case that arises here.
            let replaced = track.set_program(stated, program);
            debug_assert!(replaced.is_some(), "program_at found a program change");
        }
        None => {
            track.push(
                tick,
                TrackEventKind::Midi {
                    channel: u4::new(channel),
                    message: MidiMessage::ProgramChange {
                        program: u7::new(program),
                    },
                },
            );
        }
    }
    Ok(())
}

/// A number the format can hold in one of its restricted integers.
fn midi_value(value: i64, largest: u8) -> Option<u8> {
    u8::try_from(value).ok().filter(|value| *value <= largest)
}

/// The whole of `mid apply`: read the Take, read the Edit Set, and write a new
/// Take somewhere else.
///
/// "Somewhere else" is enforced here rather than left to the caller. `apply`
/// never writes in place — losing the Take you liked has to be impossible, not
/// merely discouraged — so an output naming the same file as the input is
/// refused before anything is read, whichever of its names it was asked for by.
///
/// The refusal is the message; it is not what makes the input safe. That is
/// `Take::write`, which writes a new file and renames it onto the output and so
/// never writes through a file the input also names. A check tells the user
/// what they did; the structure is what holds when a check is wrong.
pub fn apply_to_new_take(input: &Path, edit_set: &Path, output: &Path) -> Result<()> {
    if same_file(input, output) {
        return Err(Error::WriteInPlace(output.to_path_buf()));
    }
    let take = Take::read(input)?;
    let edit_set = EditSet::read(edit_set)?;
    apply(&take, &edit_set)?.write(output)
}

/// Whether two paths name the same file.
///
/// By identity — device and inode — wherever the filesystem can be asked,
/// because a pathname cannot answer this question. Two hard links to one Take
/// are one file under two canonical pathnames, so comparing names calls them
/// different and `apply` writes through one of them into the other.
///
/// The pathname comparison is kept for the ordinary case where the output does
/// not exist yet. A path naming nothing cannot be the input, which had to exist
/// to be read, so that comparison is answering a different and easier question.
fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    // `metadata` follows symlinks, which is what makes an output symlinked at
    // the input resolve to the file it points at rather than to itself.
    if let (Ok(a), Ok(b)) = (std::fs::metadata(a), std::fs::metadata(b)) {
        return a.dev() == b.dev() && a.ino() == b.ino();
    }
    match (resolved(a), resolved(b)) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}

/// A path with symlinks and `..` resolved as far as the filesystem allows. An
/// output file usually does not exist yet, so its directory is resolved and the
/// file name put back on the end.
fn resolved(path: &Path) -> Option<std::path::PathBuf> {
    if let Ok(canonical) = path.canonicalize() {
        return Some(canonical);
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty())?;
    Some(parent.canonicalize().ok()?.join(path.file_name()?))
}
