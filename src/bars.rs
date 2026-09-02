//! Musical position: Bars, and the span of Ticks one covers.
//!
//! Ticks are the truth; a Bar is a derived view of them. Deriving it needs a
//! time signature, and nothing here invents one — see #3. Cutting the
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

/// Where a Tick falls in the Bars of a Take: which Bar, which Beat of that Bar,
/// and how far past that Beat it lands.
///
/// A derived view, like every other reading of a Tick. Nothing here is written
/// in the file — the Take says 5760, and this says that 5760 is the first Beat
/// of Bar 5 in a Take whose Bars are 1440 Ticks long.
///
/// It carries no wording. Whether that reads as `bar 5 beat 1` is the
/// consumer's, on the same cut as ADR-0005: the placement is a fact about the
/// Take, the sentence about it is `mid`'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// 1-indexed, the same counting as `--bars`.
    pub bar: u32,

    /// 1-indexed within the Bar. A Bar has as many Beats as the time
    /// signature's numerator says notes: three in 3/4, six in 6/8. That is the
    /// signature read literally, and reading it any other way — 6/8 felt in two
    /// — is an interpretation of the music, which a derived view of a Tick has
    /// no standing to make.
    pub beat: u32,

    /// Ticks past that Beat, so 0 when the Tick lands on it. Never rounded to
    /// the nearest Beat: two notes a sixteenth apart are in two places, and
    /// saying otherwise would report the Take as having said something it did
    /// not.
    pub ticks_into_beat: u32,
}

/// The lines a Take's Ticks are read against: one Bar's worth of them, and one
/// Beat's.
///
/// Derived once and then asked repeatedly, because placing every note of a
/// passage is the ordinary case and re-reading the Take's time signature per
/// note would be the same answer computed each time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarLines {
    bar_ticks: u32,
    beat_ticks: u32,
}

impl BarLines {
    /// Where a Tick falls. Total: every Tick a Take can hold is inside some Bar,
    /// including one past the end of the Take.
    pub fn position_of(&self, tick: u32) -> Position {
        // Both divisors are non-zero by construction in `Take::bar_lines`.
        let into_bar = tick % self.bar_ticks;
        Position {
            bar: tick / self.bar_ticks + 1,
            beat: into_bar / self.beat_ticks + 1,
            ticks_into_beat: into_bar % self.beat_ticks,
        }
    }
}

impl Take {
    /// The Bar and Beat lines this Take is measured by, or `None` when it states
    /// nothing to derive them from.
    ///
    /// `None` rather than an error, because a caller asking this is asking
    /// whether a musical position is available at all — and the answer for a
    /// Take that states no time signature, or states one only part way in, is
    /// that it is not. Every reason is #3's, and `inspect --bars` is where
    /// each one is reported with its remedy; a listing that refused the Take
    /// would refuse exactly the Take a human most needs to look at.
    ///
    /// The last reason is not #3's: a Bar has one Beat per numerator, and
    /// a Take whose Bar is not a whole number of Beats has none to be placed on.
    /// It takes a PPQ small enough that a Bar is fewer Ticks than it has Beats.
    pub fn bar_lines(&self) -> Option<BarLines> {
        let bar_ticks = self.bar_ticks(self.ppq().ok()?).ok()?;
        let beats = u32::from(self.stated_time_signature().ok()?.numerator);
        if beats == 0 || bar_ticks % beats != 0 {
            return None;
        }
        Some(BarLines {
            bar_ticks,
            beat_ticks: bar_ticks / beats,
        })
    }
}
