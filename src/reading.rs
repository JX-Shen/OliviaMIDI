//! State readings retaining the sources of indeterminate values — #42.

use serde::Serialize;
use std::collections::BTreeMap;

/// A possible value after a Tick, from the last statement on this track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate<T> {
    pub track: usize,
    /// The names that track states, in written order; empty where it states
    /// none. See #61.
    pub track_names: Vec<String>,
    pub tick: u32,
    pub value: T,
}

/// What can be read at a Tick. Absence and uncertainty are distinct — #42.
///
/// `C` is what a candidate holds, which is the value's own type for every
/// reading but a Controller's. A Controller's candidates are `Option<u8>`, and
/// `None` is a track whose last word at that Tick is a Reset All Controllers.
///
/// `Reset` is a Controller after a Reset All Controllers on its channel, until
/// the Take states that Controller again. It carries where the reset is and no
/// value — #63. Only a Controller reading is ever one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reading<T, C = T> {
    Unstated,
    Determinate { value: T },
    Indeterminate { candidates: Vec<Candidate<C>> },
    Reset { at: u32 },
}

/// A determinate reading, as two readings are compared: two resets are equal
/// wherever each was sent — #63.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Settled<T> {
    Unstated,
    Value(T),
    Reset,
}

impl<T> Reading<T> {
    /// Convert the value's units without dropping the candidate sources.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Reading<U> {
        match self {
            Self::Unstated => Reading::Unstated,
            Self::Determinate { value } => Reading::Determinate { value: f(value) },
            Self::Indeterminate { candidates } => Reading::Indeterminate {
                candidates: candidates
                    .into_iter()
                    .map(|candidate| Candidate {
                        track: candidate.track,
                        track_names: candidate.track_names,
                        tick: candidate.tick,
                        value: f(candidate.value),
                    })
                    .collect(),
            },
            Self::Reset { at } => Reading::Reset { at },
        }
    }
}

impl<T: Copy, C> Reading<T, C> {
    /// Outer None means indeterminate; inner None means no value — unstated,
    /// or reset (#63).
    pub(crate) fn determinate(&self) -> Option<Option<T>> {
        self.settled().map(|settled| match settled {
            Settled::Value(value) => Some(value),
            Settled::Unstated | Settled::Reset => None,
        })
    }

    /// None means indeterminate.
    pub(crate) fn settled(&self) -> Option<Settled<T>> {
        match self {
            Self::Unstated => Some(Settled::Unstated),
            Self::Determinate { value } => Some(Settled::Value(*value)),
            Self::Indeterminate { .. } => None,
            Self::Reset { .. } => Some(Settled::Reset),
        }
    }

    pub(crate) fn candidates(&self) -> &[Candidate<C>] {
        match self {
            Self::Indeterminate { candidates } => candidates,
            _ => &[],
        }
    }
}

/// An indeterminate half-open interval, clipped to the requested window.
/// Candidate Ticks retain the original source locations. None ends with the Take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnrankedSpan<T> {
    pub from: u32,
    pub until: Option<u32>,
    pub candidates: Vec<Candidate<T>>,
}

impl<T> UnrankedSpan<T> {
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> UnrankedSpan<U> {
        UnrankedSpan {
            from: self.from,
            until: self.until,
            candidates: self
                .candidates
                .into_iter()
                .map(|candidate| Candidate {
                    track: candidate.track,
                    track_names: candidate.track_names,
                    tick: candidate.tick,
                    value: f(candidate.value),
                })
                .collect(),
        }
    }
}

/// An interval excluded from comparison. Empty candidates mean that side has no conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnrankedComparison<T> {
    pub from: u32,
    pub until: Option<u32>,
    pub before: Vec<Candidate<T>>,
    pub after: Vec<Candidate<T>>,
}

impl<T> UnrankedComparison<T> {
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> UnrankedComparison<U> {
        let mut candidates = |values: Vec<Candidate<T>>| {
            values
                .into_iter()
                .map(|candidate| Candidate {
                    track: candidate.track,
                    track_names: candidate.track_names,
                    tick: candidate.tick,
                    value: f(candidate.value),
                })
                .collect()
        };
        UnrankedComparison {
            from: self.from,
            until: self.until,
            before: candidates(self.before),
            after: candidates(self.after),
        }
    }
}

/// One address's continuing state after each Tick's statements — #42.
pub(crate) struct Timeline<T> {
    pub(crate) moments: Vec<(u32, Reading<T>)>,
}

impl<T: Copy + Eq> Timeline<T> {
    /// Input retains written order within each track and Tick.
    pub(crate) fn read(statements: impl IntoIterator<Item = Candidate<T>>) -> Self {
        let mut by_tick: BTreeMap<u32, BTreeMap<usize, Candidate<T>>> = BTreeMap::new();
        for statement in statements {
            by_tick
                .entry(statement.tick)
                .or_default()
                .insert(statement.track, statement);
        }
        let moments = by_tick
            .into_iter()
            .map(|(tick, tracks)| {
                let candidates: Vec<_> = tracks.into_values().collect();
                let value = candidates[0].value;
                let reading = if candidates.iter().all(|candidate| candidate.value == value) {
                    Reading::Determinate { value }
                } else {
                    Reading::Indeterminate { candidates }
                };
                (tick, reading)
            })
            .collect();
        Self { moments }
    }

    pub(crate) fn at(&self, tick: u32) -> Reading<T> {
        let index = self.moments.partition_point(|(at, _)| *at <= tick);
        index
            .checked_sub(1)
            .map(|index| self.moments[index].1.clone())
            .unwrap_or(Reading::Unstated)
    }

    /// How many times what is in force changes after it is first stated.
    ///
    /// A restatement of what is already in force changes nothing (ADR-0007), and
    /// neither does an indeterminate reading restated with the same candidates at
    /// a later Tick: candidates are compared by track and value, not by where
    /// they were written.
    pub(crate) fn changes(&self) -> usize {
        let same = |a: &Reading<T>, b: &Reading<T>| match (a, b) {
            (Reading::Determinate { value: a }, Reading::Determinate { value: b }) => a == b,
            (
                Reading::Indeterminate { candidates: a },
                Reading::Indeterminate { candidates: b },
            ) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|(a, b)| a.track == b.track && a.value == b.value)
            }
            _ => false,
        };
        self.moments
            .windows(2)
            .filter(|pair| !same(&pair[0].1, &pair[1].1))
            .count()
    }

    pub(crate) fn unranked(&self, from: u32, until: Option<u32>) -> Vec<UnrankedSpan<T>> {
        self.moments
            .iter()
            .enumerate()
            .filter_map(|(index, (at, reading))| {
                let Reading::Indeterminate { candidates } = reading else {
                    return None;
                };
                let end = self.moments.get(index + 1).map(|(at, _)| *at);
                let start = from.max(*at);
                let end = match (end, until) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (a, b) => a.or(b),
                };
                if end.is_some_and(|end| end <= start) {
                    return None;
                }
                Some(UnrankedSpan {
                    from: start,
                    until: end,
                    candidates: candidates.clone(),
                })
            })
            .collect()
    }
}
