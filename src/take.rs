use crate::error::{Error, Result};
use crate::note::{Note, NoteId};
use midly::{Format, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One `.mid` file — a version of the Piece that can be heard.
///
/// A Take *is* its bytes. Everything else — the header, the notes, the length —
/// is derived from them on demand, which is what lets `write` be a byte copy and
/// keeps the file the single source of truth.
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
        // Parse once here so an unreadable Take fails at `read`, naming the file,
        // rather than at whatever later call happened to touch it first.
        take.smf()?;
        Ok(take)
    }

    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Take {
        Take { path: None, bytes }
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, &self.bytes).map_err(|source| Error::Write {
            path: path.to_path_buf(),
            source,
        })
    }

    pub(crate) fn smf(&self) -> Result<Smf<'_>> {
        Smf::parse(&self.bytes).map_err(|source| Error::Malformed {
            path: self.described_path(),
            source,
        })
    }

    fn described_path(&self) -> PathBuf {
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

        let mut tempo: Option<(u32, u32)> = None; // (tick, micros per quarter)
        let mut time_signature: Option<(u32, TimeSignature)> = None;
        let mut unreadable_denominator: Option<u8> = None;
        let mut length_ticks = 0u32;

        for track in &smf.tracks {
            let mut tick = 0u32;
            for event in track {
                tick += event.delta.as_int();
                // Tempo and metre are read wherever they are, not where the notes
                // are: an ordinary export puts them on a conductor track of their
                // own. Earliest tick wins, ties broken by track order.
                match event.kind {
                    TrackEventKind::Meta(MetaMessage::Tempo(micros))
                        if tempo.is_none_or(|(at, _)| tick < at) =>
                    {
                        tempo = Some((tick, micros.as_int()));
                    }
                    TrackEventKind::Meta(MetaMessage::TimeSignature(num, denom_pow2, _, _))
                        if time_signature.is_none_or(|(at, _)| tick < at) =>
                    {
                        // The metre is stored as a power of two. A power this
                        // model cannot represent is reported as such: a
                        // denominator of 0 would be a wrong answer, and a wrong
                        // answer about the metre is worse than no answer.
                        match 1u8.checked_shl(denom_pow2 as u32) {
                            Some(denominator) => {
                                time_signature = Some((
                                    tick,
                                    TimeSignature {
                                        numerator: num,
                                        denominator,
                                    },
                                ));
                            }
                            None => unreadable_denominator = Some(denom_pow2),
                        }
                    }
                    _ => {}
                }
            }
            length_ticks = length_ticks.max(tick);
        }

        if let Some(power) = unreadable_denominator {
            return Err(Error::UnreadableTimeSignature {
                path: self.described_path(),
                power,
            });
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
            time_signature: time_signature.map(|(_, ts)| ts),
            length_ticks,
        })
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
}
