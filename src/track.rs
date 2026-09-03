//! Rewriting one track's event list, once, for a whole Edit Set.
//!
//! `apply` began by reaching a single event at a stored address and changing
//! one byte of it. That works for exactly one kind of Edit. Four of the other
//! five move events, remove them or insert them — and every one of those
//! invalidates an address some later Edit is still holding.
//!
//! The fix is to stop asking one number to mean three things. In an encoded
//! track, an event's place in the list is at once its *address* (how a resolved
//! Edit finds it), its *musical time* (delta times accumulate along it), and its
//! *rank among simultaneous events* (which of two notes struck together is the
//! first occurrence). Inserting an event has to change the second, so it drags
//! the first with it, and every address taken before the insertion is wrong.
//!
//! Here they are three separate quantities:
//!
//! | | what carries it |
//! |---|---|
//! | address | a `Slot`'s index, which never moves |
//! | musical time | `Slot::tick`, an absolute Tick |
//! | rank at one Tick | `Slot::order` |
//!
//! Nothing shifts an index: removing an event marks its slot dead, adding one
//! pushes onto the end. Delta times are derived once, at the end, from the
//! ticks — so an Edit changes a Tick and never a delta, and every event it did
//! not name keeps the Tick it arrived with. That is ADR-0003's guarantee, held
//! by a different mechanism than the one that held it for `set_velocity`.
//!
//! A slot index is an address for the length of one `apply` and is never
//! written down. Across files a note is addressed by its `NoteId`, derived from
//! content (ADR-0002). The two schemes meet exactly once, where every identity
//! in an Edit Set is resolved before the first Edit lands.

use crate::error::{Error, Result};
use midly::num::{u28, u7};
use midly::{MetaMessage, MidiMessage, TrackEvent, TrackEventKind};

/// One event of a track, found by its index and placed by its Tick and rank.
struct Slot<'a> {
    tick: u32,
    kind: TrackEventKind<'a>,
    /// Rank among the events sharing this Tick. It starts as the event's own
    /// index, so a track no Edit touched re-encodes to exactly what it was.
    order: i64,
    /// Cleared by `remove`. A slot is never taken out of the list, because
    /// taking one out would renumber every slot after it and a later Edit is
    /// holding those numbers.
    alive: bool,
}

/// A track opened up so a whole Edit Set can be applied to it.
pub(crate) struct Rewrite<'a> {
    slots: Vec<Slot<'a>>,
    /// The next rank for a re-placed note-on, and the next for a re-placed
    /// note-off. Strikes count up from beyond every original index, so a
    /// re-placed one lands after every event already at its Tick; releases
    /// count down below zero, so a re-placed one lands before them.
    next_strike: i64,
    next_release: i64,
}

impl<'a> Rewrite<'a> {
    /// Open a track. Every event becomes a slot, end-of-track included, because
    /// a slot's index has to be the index a resolved Edit is holding.
    pub(crate) fn of(track: &[TrackEvent<'a>]) -> Rewrite<'a> {
        let mut slots = Vec::with_capacity(track.len());
        let mut tick = 0u32;
        for event in track {
            tick += event.delta.as_int();
            slots.push(Slot {
                tick,
                kind: event.kind,
                order: slots.len() as i64,
                alive: true,
            });
        }
        Rewrite {
            next_strike: slots.len() as i64,
            next_release: 0,
            slots,
        }
    }

    /// Put a new event into the track. It goes on the end of the list, where it
    /// cannot shift any index a resolved Edit is holding, and is placed among
    /// the events at its Tick by the same rule as any changed one.
    pub(crate) fn push(&mut self, tick: u32, kind: TrackEventKind<'a>) -> usize {
        self.slots.push(Slot {
            tick,
            kind,
            order: 0,
            alive: true,
        });
        let index = self.slots.len() - 1;
        self.place_again(index);
        index
    }

    /// Take an event out of the track, without moving anything.
    pub(crate) fn remove(&mut self, index: usize) {
        self.slots[index].alive = false;
    }

    /// Whether an event is still in the track. An Edit reaching one an earlier
    /// Edit removed has nothing to change, and is refused rather than ignored.
    pub(crate) fn holds(&self, index: usize) -> bool {
        self.slots[index].alive
    }

    /// The key a note event names, whichever of the two spellings it uses.
    /// `None` if the event does not carry a note at all.
    pub(crate) fn key(&self, index: usize) -> Option<u8> {
        match self.slots[index].kind {
            TrackEventKind::Midi {
                message: MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. },
                ..
            } => Some(key.as_int()),
            _ => None,
        }
    }

    /// Put a note event on another key, reporting the key it was on. `None`, and
    /// nothing changed, if the event does not carry a note.
    pub(crate) fn set_key(&mut self, index: usize, key: u8) -> Option<u8> {
        let TrackEventKind::Midi {
            message:
                MidiMessage::NoteOn { key: current, .. } | MidiMessage::NoteOff { key: current, .. },
            ..
        } = &mut self.slots[index].kind
        else {
            return None;
        };
        let previous = current.as_int();
        *current = u7::new(key);
        Some(previous)
    }

    /// Strike a note-on at another velocity, reporting the velocity it had.
    /// `None`, and nothing changed, if the event is not a note-on.
    pub(crate) fn set_velocity(&mut self, index: usize, velocity: u8) -> Option<u8> {
        let TrackEventKind::Midi {
            message: MidiMessage::NoteOn { vel, .. },
            ..
        } = &mut self.slots[index].kind
        else {
            return None;
        };
        let previous = vel.as_int();
        *vel = u7::new(velocity);
        Some(previous)
    }

    /// The channel and key a note-on strikes — everything a note-off is able to
    /// name, and so everything that decides which notes can be confused for one
    /// another. `None` if the event is not a note-on.
    pub(crate) fn struck(&self, index: usize) -> Option<(u8, u8)> {
        match self.slots[index].kind {
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOn { key, .. },
            } => Some((channel.as_int(), key.as_int())),
            _ => None,
        }
    }

    /// The program change this channel is given at exactly this Tick, if the
    /// track holds one.
    ///
    /// Exactly: a `set_program` naming a Tick changes what the Take says *there*
    /// and inserts a statement when it says nothing there. It does not reach the
    /// statement before it, which is about an earlier moment, nor the one after,
    /// which the Take still makes and a later `inspect` still reports.
    pub(crate) fn program_at(&self, channel: u8, tick: u32) -> Option<usize> {
        self.slots.iter().position(|slot| {
            slot.alive
                && slot.tick == tick
                && matches!(
                    slot.kind,
                    TrackEventKind::Midi {
                        channel: on,
                        message: MidiMessage::ProgramChange { .. },
                    } if on.as_int() == channel
                )
        })
    }

    /// Put a program change on another Program, reporting the one it carried.
    /// `None`, and nothing changed, if the event is not a program change.
    pub(crate) fn set_program(&mut self, index: usize, program: u8) -> Option<u8> {
        let TrackEventKind::Midi {
            message: MidiMessage::ProgramChange { program: current },
            ..
        } = &mut self.slots[index].kind
        else {
            return None;
        };
        let previous = current.as_int();
        *current = u7::new(program);
        Some(previous)
    }

    /// Where an event will end up: its Tick, and its rank among the events
    /// sharing that Tick. The pair the track is finally sorted by, so comparing
    /// two of them answers "which of these comes first" without sorting.
    pub(crate) fn place(&self, index: usize) -> (u32, i64) {
        let slot = &self.slots[index];
        (slot.tick, slot.order)
    }

    pub(crate) fn tick(&self, index: usize) -> u32 {
        self.slots[index].tick
    }

    pub(crate) fn set_tick(&mut self, index: usize, tick: u32) {
        self.slots[index].tick = tick;
    }

    /// Place a slot after every event already at its Tick — or, if it ends a
    /// note, before them.
    ///
    /// Every Edit that changes what a note's identity is derived from, and every
    /// Edit that creates a note, ends here. Placing a changed note-on last is
    /// what stops the notes already struck at that Tick from being renumbered:
    /// occurrence indices are counted in note-on order, so a note arriving at
    /// their Tick has to arrive behind them and take the next index rather than
    /// one of theirs. See ADR-0002.
    ///
    /// A note-off goes the other way, and the reason is audible rather than
    /// arithmetic. A release landing exactly on the next strike of the same
    /// pitch would, placed last, silence the note that strike had just begun: a
    /// synthesiser stops a pitch, not an identity.
    pub(crate) fn place_again(&mut self, index: usize) {
        let order = if goes_before_the_events_at_its_tick(&self.slots[index].kind) {
            self.next_release -= 1;
            self.next_release
        } else {
            let order = self.next_strike;
            self.next_strike += 1;
            order
        };
        self.slots[index].order = order;
    }

    /// The track as an event list again, in Tick order, with delta times.
    ///
    /// End-of-track is not sorted with the rest — it is where the track stops,
    /// so it is gathered up and re-appended after the last surviving event,
    /// wherever an Edit has left that.
    pub(crate) fn finish(mut self) -> Result<Vec<TrackEvent<'a>>> {
        let mut end = 0u32;
        self.slots.retain(|slot| {
            if !slot.alive {
                return false;
            }
            if matches!(slot.kind, TrackEventKind::Meta(MetaMessage::EndOfTrack)) {
                end = end.max(slot.tick);
                return false;
            }
            true
        });
        self.slots.sort_by_key(|slot| (slot.tick, slot.order));
        let end = end.max(self.slots.last().map_or(0, |slot| slot.tick));

        with_delta_times(
            self.slots
                .into_iter()
                .map(|slot| (slot.tick, slot.kind))
                .chain([(end, TrackEventKind::Meta(MetaMessage::EndOfTrack))]),
        )
    }
}

/// Absolute Ticks in, delta times out: the one rule for placing events back
/// into a track the format can hold.
///
/// Both places that re-derive delta times come through here — the track `apply`
/// rewrote, and the passage `play --bars` cut — because it is one question, and
/// `u28` offers two answers to it. `u28::try_from` refuses a gap too large to
/// encode; `u28::new` masks it, writing a smaller gap with nothing left to say
/// it had been. Masking is the worse of the two failures by a distance: the
/// command succeeds, the file is valid MIDI, and the music is wrong. Having the
/// rule in one place is what stops the two callers from making that choice
/// separately, which is how they came to disagree.
///
/// Events arrive in ascending Tick order, by construction at both call sites:
/// one has just sorted its slots, the other builds its list while walking a
/// track it reads in order. A violation would be a fault in `battuta` rather
/// than in anybody's Take, so it is reported as one instead of underflowing.
pub(crate) fn with_delta_times<'a>(
    events: impl IntoIterator<Item = (u32, TrackEventKind<'a>)>,
) -> Result<Vec<TrackEvent<'a>>> {
    let mut previous = 0u32;
    events
        .into_iter()
        .map(|(tick, kind)| {
            let gap = tick.checked_sub(previous).ok_or_else(|| {
                Error::Encode(format!("an event at tick {tick} follows one at {previous}"))
            })?;
            previous = tick;
            Ok(TrackEvent {
                delta: u28::try_from(gap).ok_or(Error::GapUnwritable(gap))?,
                kind,
            })
        })
        .collect()
}

/// Whether an event ends a note. Both spellings count: a note-on with velocity 0
/// is a note-off, and the format uses the two interchangeably.
/// Whether an event has to precede the events already at its Tick.
///
/// Two kinds do, for the same kind of reason — what a synthesiser does with the
/// events of one Tick depends on the order it meets them in:
///
/// A release, because a note-off landing exactly on the next strike of the same
/// pitch would, placed last, silence the note that strike had just begun.
///
/// A program change, because a note-on at that Tick has to sound on the Program
/// the Take now states there. Placed last, the new Program would arrive after
/// the note it was set for, and the note would sound on the one before it — an
/// `apply` that succeeded, a file that plays, and the change inaudible in the
/// one Bar it was asked for.
fn goes_before_the_events_at_its_tick(kind: &TrackEventKind) -> bool {
    releases_a_note(kind)
        || matches!(
            kind,
            TrackEventKind::Midi {
                message: MidiMessage::ProgramChange { .. },
                ..
            }
        )
}

fn releases_a_note(kind: &TrackEventKind) -> bool {
    match kind {
        TrackEventKind::Midi { message, .. } => match message {
            MidiMessage::NoteOff { .. } => true,
            MidiMessage::NoteOn { vel, .. } => vel.as_int() == 0,
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    //! This crate's tests are at the process boundary, in `tests/`, because that
    //! is where `mid`'s behaviour is observable. The encoding rule is the
    //! exception. The difference between refusing an unwritable gap and masking
    //! it into a smaller one shows up only in a Take sparse enough to need a
    //! delta of 268 million Ticks, and committing a fixture that strange — or
    //! generating one — to watch a single `u28` conversion would cost more than
    //! stating the property here does.

    use super::*;

    /// The largest delta time the format holds.
    const LARGEST: u32 = 268_435_455;

    /// An event with nothing to say about it. What is being encoded here is the
    /// distance between events, so which events they are does not matter.
    fn anything() -> TrackEventKind<'static> {
        TrackEventKind::Meta(MetaMessage::EndOfTrack)
    }

    #[test]
    fn absolute_ticks_become_the_gaps_between_them() {
        let events = with_delta_times([(0, anything()), (480, anything()), (500, anything())])
            .expect("ordinary Ticks are writable");
        let deltas: Vec<u32> = events.iter().map(|event| event.delta.as_int()).collect();
        assert_eq!(deltas, vec![0, 480, 20]);
    }

    #[test]
    fn the_largest_delta_the_format_holds_is_written() {
        let events = with_delta_times([(0, anything()), (LARGEST, anything())])
            .expect("the largest encodable gap is encodable");
        assert_eq!(events[1].delta.as_int(), LARGEST);
    }

    /// The defect this rule exists to make impossible. `u28::new` masks rather
    /// than refuses, so this gap would be written as 0 — an event moved to the
    /// front of the track, in a file that is valid MIDI and different music.
    #[test]
    fn a_gap_the_format_cannot_reach_is_refused_and_not_masked() {
        let too_far = LARGEST + 1;
        let refused = with_delta_times([(0, anything()), (too_far, anything())])
            .expect_err("a gap past the format's reach is not writable");
        match refused {
            Error::GapUnwritable(ticks) => assert_eq!(
                ticks, too_far,
                "the refusal named a gap other than the one asked for"
            ),
            other => panic!("refused for the wrong reason: {other}"),
        }
    }

    /// Ascending Tick order is this rule's one precondition, and both callers
    /// meet it. A caller that stopped meeting it is a fault in `battuta`, and is
    /// reported as one rather than underflowing into a nonsense gap.
    #[test]
    fn events_out_of_order_are_a_fault_and_not_an_underflow() {
        let refused = with_delta_times([(480, anything()), (0, anything())])
            .expect_err("events out of Tick order cannot be encoded");
        assert!(matches!(refused, Error::Encode(_)), "{refused}");
    }
}
