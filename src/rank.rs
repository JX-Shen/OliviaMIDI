//! Rank as the oracle a comparison asks — ADR-0008's amendment, #33.
//!
//! The rule that decides which order is correct for *writing* decides which
//! order is correct for *comparing*. Two Takes may hold the same events at the
//! same Ticks and still be two different Pieces, because at one Tick one of them
//! meets a Program before the notes it governs and the other meets it after.
//! Nothing in `mid diff` could see that before this.
//!
//! Only a pair the rule ranks is compared. ADR-0008 names two, and each is a
//! case notation had already settled before MIDI flattened it: a Program is met
//! before the notes struck at its Tick, because a note has to sound on the
//! instrument the Take now names; a damper is met after the notes released at
//! its Tick, because that is what a pianist's `Ped.` under a beat means. Two
//! note-ons of different pitches at one Tick are not such a pair — neither
//! governs the other, their written order is no claim about the music, and
//! comparing it would report the byte order of a chord.
//!
//! Reading an order is not rewriting one. ADR-0008 refused normalising a Take's
//! event order on read and still refuses it: this reads what the file says and
//! reports it. A round trip through `apply` still produces the same events in
//! the same order (ADR-0003).

use crate::error::Result;
use crate::take::Take;
use midly::{MidiMessage, TrackEventKind};
use serde::Serialize;
use std::collections::BTreeMap;

/// The MIDI controller number of the damper pedal.
const DAMPER: u8 = 64;

/// A pair of events at one Tick whose order the rule makes a claim about.
///
/// Named for the order the rule calls correct, so that a reader who has the
/// name has the rule. Both are ADR-0008's own, in its own words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RankedPairKind {
    /// A program change and the strikes of its channel at its Tick. The Program
    /// is met first: a note has to sound on the instrument the Take names.
    ProgramBeforeStrike,
    /// A damper — CC64 — and the releases of its channel at its Tick. The
    /// damper is met last: the foot comes down as the hand lifts, and does not
    /// catch what has just ended.
    DamperAfterRelease,
}

impl RankedPairKind {
    /// What to call this where it is reported. `mid` owns the English
    /// (ADR-0005); this is the fact's own name, which the two consumers share.
    pub fn named(self) -> &'static str {
        match self {
            RankedPairKind::ProgramBeforeStrike => "program before its notes",
            RankedPairKind::DamperAfterRelease => "damper after its releases",
        }
    }
}

/// One Tick where the two Takes carry a ranked pair in different orders.
///
/// `before_is_correct` and `after_is_correct` rather than one verdict: which
/// Take follows the rule is the useful half of the answer, and stating both
/// leaves the reader holding what to do about it. They are never equal — two
/// Takes that agree are not a disagreement.
///
/// A Tick and a channel and no track. A Rank runs within one track (ADR-0008),
/// so a pair split across two carries no order at all, and that site is an
/// `UnrankedSite` rather than this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RankDisagreement {
    pub tick: u32,
    pub channel: u8,
    pub pair: RankedPairKind,
    pub before_is_correct: bool,
    pub after_is_correct: bool,
}

/// One Tick where the comparison could not be made, because a Take writes the
/// pair across two tracks and the file states no order between them.
///
/// Reported for transparency and never as a difference: `Diff::is_empty` does
/// not consult these. What a Take arrived with is the author's (ADR-0003), and
/// a site left open is a property of the file rather than a complaint about it
/// — the same posture `Take::unranked` takes for `inspect`.
///
/// `in_before` and `in_after` say which of the two Takes leaves it open. Both
/// are true where both do, which is what a Take compared with itself produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UnrankedSite {
    pub tick: u32,
    pub channel: u8,
    pub pair: RankedPairKind,
    pub in_before: bool,
    pub in_after: bool,
}

/// What one Take says about one pair at one Tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reading {
    /// The pair is on one track and in the order the rule calls correct.
    Follows,
    /// The pair is on one track and in the other order.
    Departs,
    /// The pair is split across two tracks. The file states no order, and none
    /// this rule can give it.
    Unranked,
}

/// Where each half of a pair falls in its own track's event list.
///
/// Positions within one track, which is what a Rank is: a track's event list is
/// the order a synthesiser meets them in. Never compared across tracks, because
/// two tracks share no Rank.
#[derive(Default)]
struct Sightings {
    /// (track, position in that track) of each governing event — the program
    /// change, or the damper.
    governing: Vec<(usize, usize)>,
    /// (track, position) of each event the governing one is ranked against —
    /// the strikes, or the releases.
    governed: Vec<(usize, usize)>,
}

impl Sightings {
    /// What this Tick and channel says about the pair, or `None` where it does
    /// not carry one.
    ///
    /// A pair needs both halves. A Tick that states a Program and strikes
    /// nothing on that channel makes no ordering claim at all, and neither does
    /// a chord with no Program above it — so those are silence rather than a
    /// verdict.
    fn read(&self, governing_first: bool) -> Option<Reading> {
        if self.governing.is_empty() || self.governed.is_empty() {
            return None;
        }
        let tracks: Vec<usize> = self
            .governing
            .iter()
            .chain(self.governed.iter())
            .map(|&(track, _)| track)
            .collect();
        if tracks.iter().any(|track| *track != tracks[0]) {
            return Some(Reading::Unranked);
        }
        // The whole of one side against the whole of the other: a Take that
        // states a Program twice at a Tick has both of them before the strikes
        // when it follows the rule, and one of them after when it does not.
        let follows = if governing_first {
            self.governing.iter().map(|&(_, at)| at).max()
                < self.governed.iter().map(|&(_, at)| at).min()
        } else {
            self.governing.iter().map(|&(_, at)| at).min()
                > self.governed.iter().map(|&(_, at)| at).max()
        };
        Some(if follows {
            Reading::Follows
        } else {
            Reading::Departs
        })
    }
}

/// Every ordering claim one Take makes, keyed by the site it makes it at.
///
/// A `BTreeMap` so that the sites come out in the order a reader meets them —
/// earliest Tick first — for the reason every other listing this project prints
/// is in the order the music happens.
type Claims = BTreeMap<(u32, u8, RankedPairKind), Reading>;

fn claims(take: &Take) -> Result<Claims> {
    let smf = take.smf()?;
    let mut seen: BTreeMap<(u32, u8, RankedPairKind), Sightings> = BTreeMap::new();
    for (track, events) in smf.tracks.iter().enumerate() {
        let mut tick = 0u32;
        for (at, event) in events.iter().enumerate() {
            tick += event.delta.as_int();
            let TrackEventKind::Midi { channel, message } = event.kind else {
                continue;
            };
            let channel = channel.as_int();
            let mut sight = |pair: RankedPairKind, governing: bool| {
                let sightings = seen.entry((tick, channel, pair)).or_default();
                if governing {
                    sightings.governing.push((track, at));
                } else {
                    sightings.governed.push((track, at));
                }
            };
            match message {
                MidiMessage::ProgramChange { .. } => {
                    sight(RankedPairKind::ProgramBeforeStrike, true);
                }
                // A note-on above velocity zero is a strike; at velocity zero it
                // is the format's other spelling of a release, and counting it
                // as a strike is the mistake the placement rule avoids one layer
                // down.
                MidiMessage::NoteOn { vel, .. } if vel.as_int() > 0 => {
                    sight(RankedPairKind::ProgramBeforeStrike, false);
                }
                MidiMessage::NoteOff { .. } | MidiMessage::NoteOn { .. } => {
                    sight(RankedPairKind::DamperAfterRelease, false);
                }
                MidiMessage::Controller { controller, .. } if controller.as_int() == DAMPER => {
                    sight(RankedPairKind::DamperAfterRelease, true);
                }
                _ => {}
            }
        }
    }
    Ok(seen
        .into_iter()
        .filter_map(|(site, sightings)| {
            let governing_first = site.2 == RankedPairKind::ProgramBeforeStrike;
            sightings
                .read(governing_first)
                .map(|reading| (site, reading))
        })
        .collect())
}

/// Where the two Takes place a ranked pair in different orders, and where
/// neither order could be read.
///
/// Compared only at a site both Takes make a claim about. A pair that exists in
/// one Take and not the other is a difference of content — a Program that
/// arrived, a chord that left — and is already reported as one; calling it an
/// ordering difference as well would report one change twice under two names.
pub(crate) fn rank_differences(
    before: &Take,
    after: &Take,
) -> Result<(Vec<RankDisagreement>, Vec<UnrankedSite>)> {
    let before_claims = claims(before)?;
    let after_claims = claims(after)?;
    let mut disagreements = Vec::new();
    let mut sites = Vec::new();
    for (&site, &mine) in &before_claims {
        let Some(&theirs) = after_claims.get(&site) else {
            continue;
        };
        let (tick, channel, pair) = site;
        if mine == Reading::Unranked || theirs == Reading::Unranked {
            sites.push(UnrankedSite {
                tick,
                channel,
                pair,
                in_before: mine == Reading::Unranked,
                in_after: theirs == Reading::Unranked,
            });
            continue;
        }
        if mine != theirs {
            disagreements.push(RankDisagreement {
                tick,
                channel,
                pair,
                before_is_correct: mine == Reading::Follows,
                after_is_correct: theirs == Reading::Follows,
            });
        }
    }
    Ok((disagreements, sites))
}
