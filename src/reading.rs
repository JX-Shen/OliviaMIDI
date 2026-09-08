//! State readings retaining the sources of indeterminate values — #42.

use serde::Serialize;
use std::collections::BTreeMap;

/// A possible value after a Tick, from the last statement on this track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate<T> {
    pub track: usize,
    pub tick: u32,
    pub value: T,
}

/// What can be read at a Tick. Absence and uncertainty are distinct — #42.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reading<T> {
    Unstated,
    Determinate { value: T },
    Indeterminate { candidates: Vec<Candidate<T>> },
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
                        tick: candidate.tick,
                        value: f(candidate.value),
                    })
                    .collect(),
            },
        }
    }
}

impl<T: Copy> Reading<T> {
    /// Outer None means indeterminate; inner None means unstated.
    pub(crate) fn determinate(&self) -> Option<Option<T>> {
        match self {
            Self::Unstated => Some(None),
            Self::Determinate { value } => Some(Some(*value)),
            Self::Indeterminate { .. } => None,
        }
    }

    pub(crate) fn candidates(&self) -> &[Candidate<T>] {
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
