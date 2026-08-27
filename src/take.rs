use crate::error::{Error, Result};
use crate::note::{Note, NoteId};
use midly::{Format, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One `.mid` file — a version of the Piece that can be heard.
///
/// A Take holds its file's bytes and derives everything else — the header, the
/// notes, the length — from them on demand, which keeps the file the single
/// source of truth and lets a Take nothing edited go back out as it arrived.
///
/// Holding the bytes is not the same as *being* them. Two encodings of the same
/// events are the same Take: what a Take is, is its event stream, and how a
/// status byte or a varint got packed belongs to whichever program wrote the
/// file. That is ADR-0005, and it is the reading `apply`'s round-trip guarantee
/// is stated in.
///
/// Every Tick in a Take fits a `u32`. Both constructors refuse one whose delta
/// times accumulate past that, which is what lets the rest of the crate add a
/// delta to a Tick and know the sum is a Tick — see `within_tick_range`.
#[derive(Debug, Clone)]
pub struct Take {
    path: Option<PathBuf>,
    bytes: Vec<u8>,
}

/// What a Take is, before anything in it is looked at.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Info {
    /// SMF format: 0 single track, 1 parallel tracks, 2 sequential tracks.
    pub format: u8,
    pub tracks: usize,
    /// Ticks per quarter note.
    pub ppq: u16,
    /// Absent when the Take never states a tempo. Not defaulted: a Take that
    /// does not say is different from one that says 120.
    pub tempo: Option<Tempo>,
    /// Absent when the Take never states a time signature.
    pub time_signature: Option<TimeSignature>,
    /// The largest tick any track reaches, end-of-track included.
    pub length_ticks: u32,
    /// The same length in Bars, when the Take states one time signature
    /// governing the whole of it. Absent rather than guessed when it does not —
    /// `mid inspect --bars` is where the reason and its remedy are.
    pub length_bars: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Tempo {
    pub micros_per_quarter: u32,
    pub bpm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

impl std::fmt::Display for TimeSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

impl Take {
    pub fn read(path: &Path) -> Result<Take> {
        let bytes = std::fs::read(path).map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let take = Take {
            path: Some(path.to_path_buf()),
            bytes,
        };
        // Parsed here so an unreadable Take fails at `read`, naming the file,
        // rather than at whatever later call happened to touch it first. The
        // same pass settles the Tick range.
        take.within_tick_range()?;
        Ok(take)
    }

    /// A Take from an event stream, encoded back to bytes.
    ///
    /// The only way a Take is made from anything but a file, and so the only
    /// place the encoder is called: `apply` hands it an edited stream and
    /// `passage` a restricted one. Neither has a path, because neither is a
    /// file until somebody writes it.
    pub(crate) fn from_smf(smf: &Smf) -> Result<Take> {
        let mut bytes = Vec::new();
        smf.write(&mut bytes)
            .map_err(|source| Error::Encode(source.to_string()))?;
        let take = Take { path: None, bytes };
        // Checked here as well as in `read`, at the cost of parsing back what
        // was just written. No Edit can reach past the range — every one of them
        // lands its Ticks through a `u32` conversion, and a passage only ever
        // moves events earlier — so this holds today by argument. The invariant
        // is worth more than the argument: it is what every unchecked `+=` in
        // this crate rests on, and an argument is a thing a later Edit breaks
        // silently.
        take.within_tick_range()?;
        Ok(take)
    }

    /// Refuse a Take whose events accumulate past the largest absolute Tick this
    /// model holds.
    ///
    /// A delta time is a `u28` and a track may carry any number of them, so a
    /// well-formed file can be built out of gaps that are each writable and
    /// whose running total leaves `u32` behind. Every Tick in `battuta` is a
    /// `u32` — a Note's start and duration, `Info`'s length, the span a Bar range
    /// resolves to, every number a `--json` payload carries — so the total is
    /// checked once, here, at the only two places a Take comes into existence.
    ///
    /// Once, rather than at each of the five places that accumulate a Tick; and
    /// checked, rather than left to wrap. Unchecked, one file gave two answers:
    /// a debug build panicked, and a release build reported a wrapped Tick as
    /// though it were the length. Refusing at the boundary is what makes the
    /// two agree and what licenses the plain `+=` everywhere downstream.
    ///
    /// The alternative was widening every public Tick to `u64`; ADR-0008 records
    /// why refusing was chosen over paying that on every Take anyone has.
    fn within_tick_range(&self) -> Result<()> {
        for track in &self.smf()?.tracks {
            let mut tick = 0u32;
            for event in track {
                tick =
                    tick.checked_add(event.delta.as_int())
                        .ok_or_else(|| Error::TakeTooLong {
                            path: self.described_path(),
                        })?;
            }
        }
        Ok(())
    }

    /// Write this Take to a path, as a new file there rather than into the file
    /// that path already names.
    ///
    /// The bytes go to a temporary file beside the destination and are renamed
    /// onto it. Two things follow, and both are load bearing.
    ///
    /// Nothing is ever written *through* the destination. A path can name a file
    /// the input also names — a second hard link to a Take canonicalises to a
    /// different pathname and the same inode — and an in-place write through
    /// such a path would edit the Take it was reading. `apply` refuses that path
    /// before it reads anything, but that refusal is a check and this is a
    /// structure: the input keeps its bytes even if the check is ever wrong.
    ///
    /// And the destination holds either the Take it held before or the whole of
    /// this one. An in-place write that ran out of disk part way through would
    /// leave a truncated file that is not a Take at all, under a name its owner
    /// is about to trust.
    pub fn write(&self, path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let failed = |source: std::io::Error| Error::Write {
            path: path.to_path_buf(),
            source,
        };
        // Beside the destination, because a rename across two filesystems is
        // not a rename and not atomic. Named so that one left behind by a
        // process that died mid-write does not read as somebody's Take.
        let directory = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut file = tempfile::Builder::new()
            .prefix(".battuta-writing-")
            .tempfile_in(directory)
            .map_err(failed)?;
        std::io::Write::write_all(&mut file, &self.bytes).map_err(failed)?;
        // A temporary file is created private to its owner, and a Take is not a
        // secret — it is a file somebody is about to open in something else. Set
        // the mode rather than let it depend on which route wrote the file.
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o644))
            .map_err(failed)?;
        file.persist(path).map_err(|unrenamed| Error::Write {
            path: path.to_path_buf(),
            source: unrenamed.error,
        })?;
        Ok(())
    }

    /// The Take's bytes, for the one writer that must not use `write`.
    ///
    /// `temporary` writes into a file it has already registered for removal, so
    /// it cannot have the new-file-and-rename that `write` performs — see the
    /// reason at the call site.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn smf(&self) -> Result<Smf<'_>> {
        Smf::parse(&self.bytes).map_err(|source| Error::Malformed {
            path: self.described_path(),
            source,
        })
    }

    pub(crate) fn described_path(&self) -> PathBuf {
        self.path.clone().unwrap_or_else(|| PathBuf::from("<take>"))
    }

    pub fn info(&self) -> Result<Info> {
        let smf = self.smf()?;
        let ppq = match smf.header.timing {
            Timing::Metrical(ppq) => ppq.as_int(),
            Timing::Timecode(..) => {
                return Err(Error::NotMetrical {
                    path: self.described_path(),
                })
            }
        };
        if ppq == 0 {
            return Err(Error::ZeroPpq {
                path: self.described_path(),
            });
        }

        let mut tempo: Option<(u32, u32)> = None; // (tick, micros per quarter)
        let mut length_ticks = 0u32;

        for track in &smf.tracks {
            let mut tick = 0u32;
            for event in track {
                tick += event.delta.as_int();
                // Tempo is read wherever it is, not where the notes are: an
                // ordinary export puts it on a conductor track of its own.
                // Earliest tick wins, ties broken by track order.
                if let TrackEventKind::Meta(MetaMessage::Tempo(micros)) = event.kind {
                    if tempo.is_none_or(|(at, _)| tick < at) {
                        tempo = Some((tick, micros.as_int()));
                    }
                }
            }
            length_ticks = length_ticks.max(tick);
        }

        Ok(Info {
            format: match smf.header.format {
                Format::SingleTrack => 0,
                Format::Parallel => 1,
                Format::Sequential => 2,
            },
            tracks: smf.tracks.len(),
            ppq,
            tempo: tempo.map(|(_, micros)| Tempo {
                micros_per_quarter: micros,
                bpm: 60_000_000.0 / micros as f64,
            }),
            time_signature: self.time_signatures()?.first().map(|&(_, ts)| ts),
            length_ticks,
            // Every reason a Bar length cannot be derived means the same thing
            // here — no Bar count — so they collapse to `None`. `info` describes
            // a Take rather than diagnosing it, and refusing to describe the
            // Takes a human most needs to look at would be the wrong trade.
            length_bars: self
                .bar_ticks(ppq)
                .ok()
                .map(|bar_ticks| crate::bars::bar_count(length_ticks, bar_ticks)),
        })
    }

    /// Every time signature the Take states, in the order they take effect.
    ///
    /// Read from wherever they are rather than from the notes' track: an
    /// ordinary export carries them on a conductor track of its own.
    fn time_signatures(&self) -> Result<Vec<(u32, TimeSignature)>> {
        let smf = self.smf()?;
        let mut stated = Vec::new();

        for track in &smf.tracks {
            let mut tick = 0u32;
            for event in track {
                tick += event.delta.as_int();
                let TrackEventKind::Meta(MetaMessage::TimeSignature(num, power, _, _)) = event.kind
                else {
                    continue;
                };
                // The denominator is stored as a power of two. A power this
                // model cannot represent is reported as such: a denominator of 0
                // would be a wrong answer, and a wrong answer about the time
                // signature is worse than no answer.
                let Some(denominator) = 1u8.checked_shl(power as u32) else {
                    return Err(Error::UnreadableTimeSignature {
                        path: self.described_path(),
                        power,
                    });
                };
                stated.push((
                    tick,
                    TimeSignature {
                        numerator: num,
                        denominator,
                    },
                ));
            }
        }

        // Earliest tick first, ties broken by track order.
        stated.sort_by_key(|&(tick, _)| tick);
        Ok(stated)
    }

    /// The one time signature that governs this whole Take.
    ///
    /// Three refusals rather than an answer, because they are three different
    /// things to go and look at: a Take that says nothing, a Take that does not
    /// say until part way in, and a Take that says two different things. See
    /// ADR-0006 for why none of them is answered with 4/4.
    pub(crate) fn stated_time_signature(&self) -> Result<TimeSignature> {
        let stated = self.time_signatures()?;
        let Some(&(at_tick, first)) = stated.first() else {
            return Err(Error::NoTimeSignature {
                path: self.described_path(),
            });
        };
        // A time signature stated at Tick 500 says nothing about Ticks 0-499.
        // Gridding those Bars from it would be applying a time signature
        // backwards to before the Take stated it.
        if at_tick != 0 {
            return Err(Error::TimeSignatureStartsLate {
                path: self.described_path(),
                at_tick,
            });
        }
        // Restating the same time signature is what some exports do at every
        // Bar; it changes nothing. Stating a different one moves every later Bar
        // line, which is what this cannot answer for.
        if let Some(&(at_tick, changed)) = stated.iter().find(|(_, stated)| *stated != first) {
            return Err(Error::TimeSignatureChanges {
                path: self.described_path(),
                at_tick,
                from: first,
                to: changed,
            });
        }
        Ok(first)
    }

    /// Every note in the Take, in track order and then in note-on event order.
    ///
    /// The order is what fixes the occurrence index in each identity, so it is
    /// part of the contract rather than an implementation convenience.
    pub fn notes(&self) -> Result<Vec<Note>> {
        let smf = self.smf()?;
        let mut pairs: Vec<Pairing> = Vec::new();

        for (track_index, track) in smf.tracks.iter().enumerate() {
            // Note-ons waiting for their release, oldest first per (channel, pitch).
            let mut open: HashMap<(u8, u8), Vec<OpenNote>> = HashMap::new();
            let mut tick = 0u32;

            for (event_index, event) in track.iter().enumerate() {
                tick += event.delta.as_int();
                let TrackEventKind::Midi { channel, message } = event.kind else {
                    continue;
                };
                let channel = channel.as_int();
                match message {
                    MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                        open.entry((channel, key.as_int()))
                            .or_default()
                            .push(OpenNote {
                                on_event: event_index,
                                start: tick,
                                velocity: vel.as_int(),
                            });
                    }
                    // A note-on with velocity 0 is a note-off; the two spellings
                    // are interchangeable in the format and must be here too.
                    MidiMessage::NoteOff { key, .. } | MidiMessage::NoteOn { key, .. } => {
                        let pitch = key.as_int();
                        let Some(pending) = open.get_mut(&(channel, pitch)) else {
                            continue;
                        };
                        if pending.is_empty() {
                            continue;
                        }
                        let open_note = pending.remove(0);
                        pairs.push(Pairing {
                            track: track_index,
                            channel,
                            pitch,
                            start: open_note.start,
                            duration: tick - open_note.start,
                            velocity: open_note.velocity,
                            on_event: open_note.on_event,
                            off_event: event_index,
                        });
                    }
                    _ => {}
                }
            }

            if let Some((&(channel, pitch), _)) = open.iter().find(|(_, v)| !v.is_empty()) {
                return Err(Error::UnterminatedNote {
                    track: track_index,
                    channel,
                    pitch,
                });
            }
        }

        // Pairing emits a note when it *ends*, so put the notes back into note-on
        // order before numbering them: the occurrence index has to be a function
        // of where a note starts, not of which of two collided notes ended first.
        pairs.sort_by_key(|p| (p.track, p.on_event));

        let mut occurrences: HashMap<(usize, u8, u8, u32), u32> = HashMap::new();
        Ok(pairs
            .into_iter()
            .map(|p| {
                let occurrence = occurrences
                    .entry((p.track, p.channel, p.pitch, p.start))
                    .or_insert(0);
                let note = Note {
                    id: NoteId::new(p.track, p.channel, p.pitch, p.start, *occurrence),
                    track: p.track,
                    channel: p.channel,
                    pitch: p.pitch,
                    start: p.start,
                    duration: p.duration,
                    velocity: p.velocity,
                    on_event: p.on_event,
                    off_event: p.off_event,
                };
                *occurrence += 1;
                note
            })
            .collect())
    }
}

/// A note-on waiting for the note-off that ends it.
struct OpenNote {
    on_event: usize,
    start: u32,
    velocity: u8,
}

/// A matched note-on/note-off pair, before it has been given an identity.
struct Pairing {
    track: usize,
    channel: u8,
    pitch: u8,
    start: u32,
    duration: u32,
    velocity: u8,
    on_event: usize,
    off_event: usize,
}
