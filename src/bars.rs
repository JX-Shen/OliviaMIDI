//! Musical position: Bars, and the span of Ticks one covers.
//!
//! Ticks are the truth; a Bar is a derived view of them. Deriving it needs a
//! time signature, and nothing here invents one — see ADR-0006.

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
        let bar_ticks = self.bar_ticks()?;
        let bar_count = self.bar_count(bar_ticks)?;
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

    /// How many Bars the Take is long.
    ///
    /// The Take occupies `[0, length_ticks)`, so this is how many Bars it takes
    /// to cover every Tick it reaches: a partial final Bar is a Bar. The
    /// fixture's 11516 Ticks are eight Bars of 1440, the last of which the Take
    /// stops four Ticks inside.
    fn bar_count(&self, bar_ticks: u32) -> Result<u32> {
        Ok(self.info()?.length_ticks.div_ceil(bar_ticks))
    }

    /// The length of one Bar, in Ticks.
    ///
    /// Never zero, which is what keeps `bar_count` from dividing by it: `info`
    /// refuses a Take stating 0 Ticks per quarter, and a Bar that would not come
    /// out as a whole number of Ticks is refused below rather than rounded.
    fn bar_ticks(&self) -> Result<u32> {
        let info = self.info()?;
        let stated = self.stated_time_signature()?;
        // PPQ is Ticks per quarter note, so a Bar is `numerator` notes of value
        // `denominator`: 3/4 at 480 PPQ is 1440 Ticks, and so is 6/8.
        if stated.numerator == 0 {
            return Err(Error::TimeSignatureWithNoBeats {
                path: self.described_path(),
                denominator: stated.denominator,
            });
        }
        let quarters_worth = u32::from(stated.numerator) * u32::from(info.ppq) * 4;
        let denominator = u32::from(stated.denominator);
        if quarters_worth % denominator != 0 {
            return Err(Error::BarNotWholeTicks {
                path: self.described_path(),
                numerator: stated.numerator,
                denominator: stated.denominator,
                ppq: info.ppq,
            });
        }
        Ok(quarters_worth / denominator)
    }
}
