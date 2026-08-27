//! Musical position: Bars, and the span of Ticks one covers.
//!
//! Ticks are the truth; a Bar is a derived view of them. Deriving it needs a
//! time signature, and nothing here invents one — see ADR-0006. Cutting the
//! passage a span selects out of a Take is `crate::passage`.

use crate::error::{Error, Result};
use crate::note::Note;
use crate::take::Take;
use std::str::FromStr;

/// A range of Bars: 1-indexed, and inclusive at both ends, so `5:8` is Bars
/// five through eight — the four Bars a musician would point at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarRange {
    first: u32,
    last: u32,
}

/// The Ticks a Bar range covers, as a half-open span: `[start, end)`.
///
/// Half-open because it makes "the note belongs to the Bar it starts in" a
/// single comparison, and because a Bar's end Tick is the next Bar's first —
/// naming it as belonging to both is how off-by-one Bars happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickSpan {
    pub start: u32,
    pub end: u32,
}

/// `FIRST:LAST`, and nothing else. The rules a range must satisfy live here
/// rather than in the command that accepts it: `mid play --bars` reads the same
/// syntax, and a second parser would be a second set of rules to keep in step.
impl FromStr for BarRange {
    type Err = Error;

    fn from_str(text: &str) -> Result<BarRange> {
        let malformed = || Error::BarRangeMalformed(text.to_string());
        let (first, last) = text.split_once(':').ok_or_else(malformed)?;
        let first: u32 = first.trim().parse().map_err(|_| malformed())?;
        let last: u32 = last.trim().parse().map_err(|_| malformed())?;

        if first == 0 || last == 0 {
            return Err(Error::BarZero);
        }
        if first > last {
            return Err(Error::BarRangeInverted { first, last });
        }
        Ok(BarRange { first, last })
    }
}

/// Written the way it is typed: `5:8`. `mid play --json` records the passage it
/// auditioned in the form the flag accepts, so nothing has to be reassembled to
/// ask for it again.
impl std::fmt::Display for BarRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.first, self.last)
    }
}

impl serde::Serialize for BarRange {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// How many Bars a Take of this length is.
///
/// The Take occupies `[0, length_ticks)`, so this is how many Bars it takes to
/// cover every Tick it reaches: a partial final Bar is a Bar. The fixture's
/// 11516 Ticks are eight Bars of 1440, the last of which the Take stops four
/// Ticks inside. One home for the rule, because `info` reports it and
/// `tick_span` measures a range against it.
pub(crate) fn bar_count(length_ticks: u32, bar_ticks: u32) -> u32 {
    length_ticks.div_ceil(bar_ticks)
}

impl Take {
    /// The notes of the Take, restricted to a Bar range when one is given.
    ///
    /// A note belongs to the Bar it *starts* in. One that sustains across a Bar
    /// line is still sounding in the next Bar but was played in this one, and an
    /// Edit naming it is an Edit to this passage.
    pub fn notes_in(&self, bars: Option<BarRange>) -> Result<Vec<Note>> {
        let Some(bars) = bars else {
            return self.notes();
        };
        let span = self.tick_span(bars)?;
        Ok(self
            .notes()?
            .into_iter()
            .filter(|note| span.start <= note.start && note.start < span.end)
            .collect())
    }

    /// The Ticks a Bar range covers in this Take.
    ///
    /// The one shared capability behind every `--bars`: `inspect` filters notes
    /// by this span, and playing a passage is built from it.
    pub fn tick_span(&self, bars: BarRange) -> Result<TickSpan> {
        let info = self.info()?;
        let bar_ticks = self.bar_ticks(info.ppq)?;
        let bar_count = bar_count(info.length_ticks, bar_ticks);
        if bars.last > bar_count {
            return Err(Error::BarRangeBeyondTake {
                path: self.described_path(),
                last: bars.last,
                bars: bar_count,
            });
        }
        // Unreachable past the guard above, and clamping rather than wrapping is
        // the safe direction if it ever became reachable.
        Ok(TickSpan {
            start: (bars.first - 1).saturating_mul(bar_ticks),
            end: bars.last.saturating_mul(bar_ticks),
        })
    }

    /// The length of one Bar, in Ticks. Never zero.
    ///
    /// Takes the PPQ rather than reading it, because `info` is one of its
    /// callers: `Info` carries the Bar count, so a `bar_ticks` that asked `info`
    /// for the PPQ would ask itself.
    pub(crate) fn bar_ticks(&self, ppq: u16) -> Result<u32> {
        if ppq == 0 {
            return Err(Error::ZeroPpq {
                path: self.described_path(),
            });
        }
        let stated = self.stated_time_signature()?;
        // PPQ is Ticks per quarter note, so a Bar is `numerator` notes of value
        // `denominator`: 3/4 at 480 PPQ is 1440 Ticks, and so is 6/8.
        if stated.numerator == 0 {
            return Err(Error::TimeSignatureWithNoBeats {
                path: self.described_path(),
                denominator: stated.denominator,
            });
        }
        let quarters_worth = u32::from(stated.numerator) * u32::from(ppq) * 4;
        let denominator = u32::from(stated.denominator);
        if quarters_worth % denominator != 0 {
            return Err(Error::BarNotWholeTicks {
                path: self.described_path(),
                numerator: stated.numerator,
                denominator: stated.denominator,
                ppq,
            });
        }
        Ok(quarters_worth / denominator)
    }
}
