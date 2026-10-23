//! Elapsed time: where a Tick falls in seconds, by the tempos the Take states.
//!
//! The other derived view of a Tick beside a Position (`CONTEXT.md`), read
//! against the Tempo rather than the time signature. No tempo is assumed where
//! the Take states none — #73.

use std::time::Duration;

use crate::reading::Reading;
use crate::take::{tempo_timeline, Take};
use crate::Result;

/// Why some or all of a Take's Ticks have no Elapsed time.
///
/// Carries no wording (ADR-0005) — #73.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoElapsedTime {
    /// The Take states no tempo anywhere.
    NoTempo,
    /// The first tempo is stated at this Tick, and no Tick has a reading.
    TempoStartsLate { at: u32 },
    /// From this Tick the tempos stated on different tracks contradict each
    /// other, and no Tick from it on has a reading.
    TempoIndeterminate { from: u32 },
}

/// The tempo map a Take's Ticks are read against for Elapsed time.
///
/// Derived once and then asked repeatedly, as `BarLines` is.
#[derive(Debug, Clone)]
pub struct ElapsedTimes {
    ppq: u16,
    /// Each determinate tempo in force from its Tick, with the elapsed time at
    /// that Tick kept exactly, as microseconds multiplied by the PPQ.
    segments: Vec<Segment>,
    unknown: Option<NoElapsedTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Segment {
    tick: u32,
    scaled_micros: u128,
    micros_per_quarter: u32,
}

impl ElapsedTimes {
    /// How long after Tick 0 this Tick is heard, or why the Take does not say.
    /// Truncated to the nanosecond, never rounded up.
    pub fn elapsed_at(&self, tick: u32) -> std::result::Result<Duration, NoElapsedTime> {
        match self.unknown {
            Some(reason @ (NoElapsedTime::NoTempo | NoElapsedTime::TempoStartsLate { .. })) => {
                return Err(reason)
            }
            Some(reason @ NoElapsedTime::TempoIndeterminate { from }) if tick >= from => {
                return Err(reason)
            }
            _ => {}
        }
        // The first segment starts at Tick 0 whenever none of the above holds.
        let index = self
            .segments
            .partition_point(|segment| segment.tick <= tick);
        let segment = self.segments[index - 1];
        let scaled = segment.scaled_micros
            + u128::from(tick - segment.tick) * u128::from(segment.micros_per_quarter);
        // At most 2^32 Ticks of 2^24 us each: the seconds fit a u64 and the
        // nanoseconds past them a u32.
        let per_second = 1_000_000 * u128::from(self.ppq);
        let seconds = (scaled / per_second) as u64;
        let nanos = (scaled % per_second * 1000 / u128::from(self.ppq)) as u32;
        Ok(Duration::new(seconds, nanos))
    }

    /// Whether every Tick reads the same in both, including where neither has
    /// a reading. Why a reading is missing does not count, only where.
    pub(crate) fn reads_alike(&self, other: &ElapsedTimes) -> bool {
        let unread_from = |times: &ElapsedTimes| match times.unknown {
            Some(NoElapsedTime::NoTempo | NoElapsedTime::TempoStartsLate { .. }) => Some(0),
            Some(NoElapsedTime::TempoIndeterminate { from }) => Some(from),
            None => None,
        };
        self.ppq == other.ppq
            && self.segments == other.segments
            && unread_from(self) == unread_from(other)
    }
}

impl Take {
    /// The tempo map this Take's Ticks are read against for Elapsed time.
    pub fn elapsed_times(&self) -> Result<ElapsedTimes> {
        let ppq = self.ppq()?;
        let timeline = tempo_timeline(&self.stated_tempos()?);
        let mut times = ElapsedTimes {
            ppq,
            segments: Vec::new(),
            unknown: None,
        };
        match timeline.moments.first() {
            None => times.unknown = Some(NoElapsedTime::NoTempo),
            Some(&(at, _)) if at != 0 => {
                times.unknown = Some(NoElapsedTime::TempoStartsLate { at })
            }
            Some(_) => {}
        }
        if times.unknown.is_some() {
            return Ok(times);
        }

        let mut previous: Option<Segment> = None;
        for (tick, reading) in &timeline.moments {
            let Reading::Determinate { value } = reading else {
                times.unknown = Some(NoElapsedTime::TempoIndeterminate { from: *tick });
                break;
            };
            // A tempo restated changes nothing (ADR-0007), so it starts no
            // segment, and `reads_alike` can compare segments directly.
            if previous.is_some_and(|previous| previous.micros_per_quarter == *value) {
                continue;
            }
            let scaled_micros = previous.map_or(0, |previous| {
                previous.scaled_micros
                    + u128::from(tick - previous.tick) * u128::from(previous.micros_per_quarter)
            });
            let segment = Segment {
                tick: *tick,
                scaled_micros,
                micros_per_quarter: *value,
            };
            times.segments.push(segment);
            previous = Some(segment);
        }
        Ok(times)
    }
}
