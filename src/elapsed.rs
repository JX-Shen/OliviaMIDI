//! Elapsed time: where a Tick falls in seconds, by the tempos the Take states.
//!
//! The other derived view of a Tick beside a Position (`CONTEXT.md`). A
//! Position is read against the time signature; this is read against the
//! Tempo, and lines up with the time axis of a render. Where the Take does not
//! say enough to tell, there is no reading rather than a plausible one: no
//! default tempo is assumed, before the first one stated or at all — #73.

use std::time::Duration;

use crate::reading::Reading;
use crate::take::{tempo_timeline, Take};
use crate::Result;

/// Why some or all of a Take's Ticks have no Elapsed time.
///
/// Carries no wording, on ADR-0005's cut: `mid info` says which, once, and
/// the rows that lack a reading say nothing — #73.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoElapsedTime {
    /// The Take states no tempo anywhere.
    NoTempo,
    /// The first tempo is stated at this Tick, so nothing says how fast the
    /// Ticks before it pass, and every later reading depends on them.
    TempoStartsLate { at: u32 },
    /// From this Tick the tempos stated on different tracks contradict each
    /// other. Ticks before it still have their reading.
    TempoIndeterminate { from: u32 },
}

/// The tempo map a Take's Ticks are read against for Elapsed time.
///
/// Derived once and then asked repeatedly, as `BarLines` is. Two are equal
/// exactly when they read every Tick alike.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// How long after Tick 0 this Tick is heard, or `None` where the Take does
    /// not say. Truncated to the nanosecond, never rounded up.
    pub fn elapsed_at(&self, tick: u32) -> Option<Duration> {
        match self.unknown {
            Some(NoElapsedTime::NoTempo | NoElapsedTime::TempoStartsLate { .. }) => return None,
            Some(NoElapsedTime::TempoIndeterminate { from }) if tick >= from => return None,
            _ => {}
        }
        let index = self
            .segments
            .partition_point(|segment| segment.tick <= tick);
        let segment = self.segments.get(index.checked_sub(1)?)?;
        let scaled = segment.scaled_micros
            + u128::from(tick - segment.tick) * u128::from(segment.micros_per_quarter);
        let nanos = scaled * 1000 / u128::from(self.ppq);
        Some(Duration::from_nanos(u64::try_from(nanos).ok()?))
    }

    /// Why some or all Ticks have no reading, or `None` when every Tick has one.
    pub fn unknown(&self) -> Option<NoElapsedTime> {
        self.unknown
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
            // A tempo restated changes nothing (ADR-0007), and leaving it out
            // is what makes two maps equal exactly when they read every Tick
            // alike, which is what `diff` asks of them.
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
