//! Rewriting one track's event list, once, for a whole Edit Set.
//!
//! `apply` began by reaching a single event at a stored address and changing
//! one byte of it. That works for exactly one kind of Edit. Every other kind
//! moves events, removes them or inserts them — and every one of those
//! invalidates an address some later Edit is still holding.
//!
//! The fix is to stop asking one number to mean three things. In an encoded
//! track, an event's place in the list is at once its *address* (how a resolved
//! Edit finds it), its *musical time* (delta times accumulate along it), and its
//! *Rank* among the events sharing that Tick (which of two notes struck together
//! is the first occurrence). Inserting an event has to change the second, so it
//! drags the first with it, and every address taken before the insertion is
//! wrong.
//!
//! Here they are three separate quantities:
//!
//! | | what carries it |
//! |---|---|
//! | address | a `Slot`'s index, which never moves |
//! | musical time | `Slot::tick`, an absolute Tick |
//! | Rank at one Tick | `Slot::rank` |
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

/// One event of a track, found by its index and placed by its Tick and Rank.
struct Slot<'a> {
    tick: u32,
    kind: TrackEventKind<'a>,
    /// Where this event falls among the events sharing its Tick — which of
    /// them a synthesiser meets first. Starts spaced by `RANK_SPACING` from an
    /// original's own index, rather than equal to it, so a Rank can always be
    /// found between any two original Ranks; a track no Edit touched still
    /// re-encodes to exactly what it was, because spacing them preserves every
    /// original's Rank relative to every other. See CONTEXT.md's **Rank**.
    rank: i64,
    /// Cleared by `remove`. A slot is never taken out of the list, because
    /// taking one out would renumber every slot after it and a later Edit is
    /// holding those numbers.
    alive: bool,
    /// Which of this Edit Set's writes to this track last touched the event,
    /// counting from 0 — Edits land in the order the Edit Set gave them, so it
    /// is that order. `None` for an event that arrived in the Take and no Edit
    /// has touched: ADR-0003 keeps those exactly where they were, and
    /// `stay_placed` asserts nothing about them. See #27.
    written: Option<usize>,
}

/// The distance between two originals' Ranks. Spacing them, rather than
/// numbering them 0, 1, 2, ..., is what leaves room to place an event between
/// any two of them — which a `Slot`'s index cannot do, because an index is an
/// address and never moves.
///
/// The size is not load bearing, only the room: any value above the number of
/// events one Edit Set can place at a single Tick would do, and this one leaves
/// 65535 of them. It is a power of two so that a Rank reads as an index and an
/// offset when one is printed while debugging.
const RANK_SPACING: i64 = 1 << 16;

/// What an event placed or re-placed at a Tick is — named by the caller, which
/// is the one place that knows, rather than guessed here from the event's own
/// kind. A kind arriving with no rule of its own would otherwise be a fresh
/// chance to guess one; naming it is how the next kind instead has to say what
/// it is. See #24.
#[derive(Clone, Copy)]
pub(crate) enum Placement {
    /// Starts a note. Placed after every event already at its Tick: occurrence
    /// indices are counted in note-on order, so a note arriving where others of
    /// its track, channel and pitch already begin has to arrive behind them and
    /// take the next index rather than one of theirs. See ADR-0002.
    Strike,
    /// Ends a note, by either spelling: a note-off, or a note-on at velocity
    /// zero. Placed before every event already at its Tick — a release landing
    /// exactly on the next strike of the same pitch would, placed last, silence
    /// the note that strike had just begun.
    Release,
    /// States what a channel is on or holds: a program change or a control
    /// change. ADR-0008 is the principle it is placed by — state before the
    /// strikes it governs — and `before_the_strikes` is the rule #20 settled
    /// from it, in the three clauses `mid apply --help` states.
    State(Statement),
}

/// What one event states about a channel: which channel, and which of its
/// states.
///
/// One type answers two questions that must not drift apart — where a state
/// event is *written* (the first clause of the placement rule looks for the
/// other statements of its own address) and which state event is *found* (a
/// lookup for the one in force means the last of them by Rank). They agree by
/// asking `is_stated_by`, rather than by two searches spelling out the same
/// match.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Statement {
    pub(crate) channel: u8,
    pub(crate) state: ChannelState,
}

/// Which state of a channel a `Statement` is about.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelState {
    /// Which Program the channel is on.
    Program,
    /// What the channel holds for one Controller, by its CC number.
    Controller(u8),
}

impl ChannelState {
    /// What to call this in a refusal. Enough to point at the event, not a
    /// Controller's name — `mid` names Controllers where it reports them, and a
    /// fault report is not the place to learn a second way to do it.
    pub(crate) fn named(self) -> String {
        match self {
            ChannelState::Program => "program change".to_string(),
            ChannelState::Controller(number) => format!("control change for CC {number}"),
        }
    }
}

impl Statement {
    /// The address a channel's Program is stated at.
    pub(crate) fn program(channel: u8) -> Self {
        Statement {
            channel,
            state: ChannelState::Program,
        }
    }

    /// The address one Controller of a channel is stated at.
    pub(crate) fn controller(channel: u8, number: u8) -> Self {
        Statement {
            channel,
            state: ChannelState::Controller(number),
        }
    }

    /// Whether this event strikes a note this statement governs: a strike of
    /// its own channel. The address is not asked about — a Program and a
    /// Controller are both set for the same notes.
    fn governs(self, kind: &TrackEventKind) -> bool {
        strikes(kind, self.channel)
    }

    /// Whether this event states exactly this address.
    fn is_stated_by(self, kind: &TrackEventKind) -> bool {
        let TrackEventKind::Midi {
            channel: on,
            message,
        } = kind
        else {
            return false;
        };
        on.as_int() == self.channel
            && match (self.state, message) {
                (ChannelState::Program, MidiMessage::ProgramChange { .. }) => true,
                (ChannelState::Controller(number), MidiMessage::Controller { controller, .. }) => {
                    controller.as_int() == number
                }
                _ => false,
            }
    }
}

/// A track opened up so a whole Edit Set can be applied to it.
pub(crate) struct Rewrite<'a> {
    slots: Vec<Slot<'a>>,
    /// How many writes this rewrite has made, which numbers the next one.
    writes: usize,
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
                rank: slots.len() as i64 * RANK_SPACING,
                alive: true,
                written: None,
            });
        }
        Rewrite { slots, writes: 0 }
    }

    /// Put a new event into the track. It goes on the end of the list, where it
    /// cannot shift any index a resolved Edit is holding, and is placed among
    /// the events at its Tick by the same rule as any changed one.
    pub(crate) fn push(
        &mut self,
        tick: u32,
        kind: TrackEventKind<'a>,
        placement: Placement,
    ) -> usize {
        self.slots.push(Slot {
            tick,
            kind,
            rank: 0,
            alive: true,
            written: None,
        });
        let index = self.slots.len() - 1;
        self.place_again(index, placement);
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
        key(&self.slots[index].kind)
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
        self.wrote(index);
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
        self.wrote(index);
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
    ///
    /// The *last* such event by Rank rather than the first, as `controller_at`
    /// finds the last control change by Rank. Two program changes at one
    /// address are both carried (ADR-0003 — an Edit Set naming neither may not
    /// fold them), and the one in force is the one last by Rank, which is what
    /// `inspect` reports and what a synthesiser plays. See #19.
    ///
    /// By Rank, and not by slot index: an event a resolved Edit re-places keeps
    /// the index it was given when the track was opened, so an index search
    /// answers "which of these was written first", not "which is in force" —
    /// the two agree only while nothing has been re-placed. See #24.
    pub(crate) fn program_at(&self, channel: u8, tick: u32) -> Option<usize> {
        self.in_force_at(tick, Statement::program(channel))
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
        self.wrote(index);
        Some(previous)
    }

    /// Where a Controller is stated on a channel at a Tick, or `None`.
    ///
    /// The *last* such event by Rank rather than the first, as `program_at`
    /// finds the last program change by Rank. Two control changes at one
    /// address are both carried (ADR-0003 — an Edit Set naming neither may not
    /// fold them), and the one in force is the one last by Rank, so that is the
    /// one an Edit naming the address means.
    ///
    /// By Rank, and not by slot index, for the reason `program_at` is: an
    /// index search answers which was written first, and stops agreeing with
    /// which is in force the moment anything has been re-placed. A Controller
    /// moved onto an address a duplicate already occupies is exactly that
    /// moment — the mover keeps the slot index it arrived with, which need not
    /// be the highest of the two. See #24.
    pub(crate) fn controller_at(&self, channel: u8, controller: u8, tick: u32) -> Option<usize> {
        self.in_force_at(tick, Statement::controller(channel, controller))
    }

    /// The alive slot stating this channel state at this Tick, last by Rank —
    /// "in force", by ADR-0008's rule. `None` if the track states none there.
    ///
    /// The one place `program_at` and `controller_at` ask "which is in force",
    /// so that the answer is the same question asked twice rather than two
    /// searches that could drift apart. It is also the question the placement
    /// rule's first clause asks, through the same `Statement`.
    fn in_force_at(&self, tick: u32, statement: Statement) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| {
                slot.alive && slot.tick == tick && statement.is_stated_by(&slot.kind)
            })
            .max_by_key(|(_, slot)| slot.rank)
            .map(|(index, _)| index)
    }

    /// Put a control change on another value, reporting the one it carried.
    /// `None`, and nothing changed, if the event is not a control change.
    pub(crate) fn set_controller(&mut self, index: usize, value: u8) -> Option<u8> {
        let TrackEventKind::Midi {
            message: MidiMessage::Controller { value: current, .. },
            ..
        } = &mut self.slots[index].kind
        else {
            return None;
        };
        let previous = current.as_int();
        *current = u7::new(value);
        self.wrote(index);
        Some(previous)
    }

    /// Every channel-state event the track still holds, as the cross-track check
    /// reads them.
    ///
    /// No Rank comes out. A Rank orders events within one track and the caller
    /// of this is comparing two, which is the whole reason that check exists —
    /// handing it a number it must not compare would be an invitation.
    pub(crate) fn stated(&self) -> impl Iterator<Item = Stated> + '_ {
        self.slots
            .iter()
            .filter(|slot| slot.alive)
            .filter_map(|slot| {
                Some(Stated {
                    tick: slot.tick,
                    statement: stated_by(&slot.kind)?,
                    value: valued(&slot.kind)?,
                    written: slot.written.is_some(),
                })
            })
    }

    /// Every strike the track still holds, as the cross-track check reads them.
    pub(crate) fn struck_notes(&self) -> impl Iterator<Item = Struck> + '_ {
        self.slots
            .iter()
            .filter(|slot| slot.alive)
            .filter_map(|slot| match slot.kind {
                TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::NoteOn { vel, .. },
                } if vel.as_int() > 0 => Some(Struck {
                    tick: slot.tick,
                    channel: channel.as_int(),
                    written: slot.written.is_some(),
                }),
                _ => None,
            })
    }

    /// Where an event will end up: its Tick, and its Rank among the events
    /// sharing that Tick. The pair the track is finally sorted by, so comparing
    /// two of them answers "which of these comes first" without sorting.
    pub(crate) fn place(&self, index: usize) -> (u32, i64) {
        let slot = &self.slots[index];
        (slot.tick, slot.rank)
    }

    pub(crate) fn tick(&self, index: usize) -> u32 {
        self.slots[index].tick
    }

    pub(crate) fn set_tick(&mut self, index: usize, tick: u32) {
        self.slots[index].tick = tick;
        self.wrote(index);
    }

    /// Give a slot the Rank its `Placement` calls for, against the track as it
    /// now stands.
    ///
    /// Which position each `Placement` asks for, and why, is written on its
    /// variant. Every Edit that changes what a note's identity is derived from,
    /// every Edit that creates a note, and every Edit that places or changes
    /// what a channel is on or holds ends here — and nothing here looks at the
    /// event to decide *which* rule to apply, because the caller has already
    /// said what it is placing.
    ///
    /// Read against the Tick rather than against a running counter: an Edit that
    /// ran earlier may have carried an event onto this Tick or off it, and each
    /// position is a claim about what is there now. Every position is derived
    /// from the Ranks already in use and steps clear of them, so no two live
    /// slots at a Tick share a Rank and a search for the last by Rank has one
    /// answer — for as many placements at one position as `RANK_SPACING` leaves
    /// room for.
    pub(crate) fn place_again(&mut self, index: usize, placement: Placement) {
        let rank = match placement {
            Placement::Strike => self.after_everything(index),
            Placement::Release => self.before_everything(index),
            Placement::State(statement) => self.before_the_strikes(index, statement),
        };
        self.slots[index].rank = rank;
        self.wrote(index);
    }

    /// Mark an event as one this Edit Set has written, and number the write.
    ///
    /// Every mutation here comes through it, not only the ones that set a Rank.
    /// `stay_placed` checks what this Edit Set wrote, and the defect it exists
    /// to catch is an event an Edit moved and *failed* to place — which reaches
    /// `set_tick` and never reaches `place_again`. Marking only the placements
    /// would leave the check blind to exactly the shape of bug it is for.
    fn wrote(&mut self, index: usize) {
        self.slots[index].written = Some(self.writes);
        self.writes += 1;
    }

    /// The alive slots sharing a Tick with the one being placed.
    ///
    /// Never the slot itself: it is alive and at that Tick already, carrying the
    /// stale Rank this placement is about to replace, and counting it would let
    /// an event be placed relative to where it used to be.
    ///
    /// The Tick is read here rather than asked for. Every caller was passing
    /// the placed slot's own — an argument that cannot disagree with its
    /// sibling and would be a defect if it ever did. See #41.
    fn sharing(&self, placing: usize) -> impl Iterator<Item = &Slot<'a>> + '_ {
        let tick = self.slots[placing].tick;
        self.slots
            .iter()
            .enumerate()
            .filter(move |(index, slot)| *index != placing && slot.alive && slot.tick == tick)
            .map(|(_, slot)| slot)
    }

    /// Behind every event already at the Tick.
    fn after_everything(&self, placing: usize) -> i64 {
        self.sharing(placing)
            .map(|slot| slot.rank)
            .max()
            .map_or(0, |last| last + RANK_SPACING)
    }

    /// In front of every event already at the Tick.
    fn before_everything(&self, placing: usize) -> i64 {
        self.sharing(placing)
            .map(|slot| slot.rank)
            .min()
            .map_or(0, |first| first - RANK_SPACING)
    }

    /// Where a channel-state event goes among the events sharing its Tick.
    ///
    /// ADR-0008 is the principle — state before the strikes it governs — and
    /// these are the three clauses #20 settled from it. Each is load bearing,
    /// and `mid apply --help` states all three so that an agent can predict a
    /// placement rather than discover it.
    fn before_the_strikes(&self, placing: usize, statement: Statement) -> i64 {
        // 1. After every other statement of its own address that remains here.
        //    Without this, a statement moved onto a Tick that already states its
        //    address can land behind the one it was meant to replace, and the
        //    Edit's value is not the one in force.
        let after = self
            .sharing(placing)
            .filter(|slot| statement.is_stated_by(&slot.kind))
            .map(|slot| slot.rank)
            .max();
        // 2. Then immediately before the first strike of its own channel that
        //    follows that position — the notes this state was set for. Searching
        //    forward from clause 1 is what keeps the two from contradicting each
        //    other: clause 1 fixes a position, clause 2 searches on from it.
        let strike = self
            .sharing(placing)
            .filter(|slot| statement.governs(&slot.kind))
            .map(|slot| slot.rank)
            .filter(|rank| after.is_none_or(|already| *rank > already))
            .min();
        // 3. Or, where no strike follows it, at the end of the Tick. Without
        //    this the placement is undefined wherever a Tick strikes nothing on
        //    the channel.
        let Some(strike) = strike else {
            return self.after_everything(placing);
        };
        // "Immediately before" is past everything that strike is already behind,
        // which is also what keeps several placed here in the order the Edit Set
        // asked for: each one becomes the last the next has to clear. A Rank is
        // free there because `RANK_SPACING` leaves room between any two.
        self.sharing(placing)
            .map(|slot| slot.rank)
            .filter(|rank| *rank < strike)
            .max()
            .map_or(strike - RANK_SPACING, |behind| behind + 1)
    }

    /// Refuse a track whose Ranks contradict the rule that placed them.
    ///
    /// A sibling to `stay_distinct`, and the same posture as `with_delta_times`
    /// reporting an event out of Tick order: a fault in `battuta` rather than a
    /// problem with anybody's Take. If `place_again` is right this can never
    /// fire, which is the point — every defect this project has shipped on
    /// Rank was found by rendering audio and comparing it sample by sample,
    /// which needs a Rig, needs CI to have a soundfont, and needs somebody to
    /// already suspect the answer. ADR-0008 made the alternative possible: the
    /// placement of every event this project writes is now decided by one
    /// rule, and a rule is a property of the finished track. See #27.
    ///
    /// Only what this Edit Set wrote. A Take carried in with a release behind
    /// a strike keeps it — ADR-0003 — so an event with no `written` is one this
    /// check has nothing to say about. The cross-track case is not here
    /// either: two tracks share no Rank, so there is nothing to compare. That
    /// one is `stay_rankable`, a sibling run over the same finished Take — this
    /// asserts the order the rule gave, that one refuses where the file gives
    /// none. See #26.
    ///
    /// Which rule each event answers to is read from the event, not from the
    /// `Placement` its caller named. `place_again` takes the caller's word on
    /// purpose, so that a kind arriving with no rule of its own cannot have one
    /// guessed for it (#24) — and a check that took the same word would agree
    /// with a wrong one. Asking the event is the second opinion: a release is a
    /// release whatever an Edit called it, and an Edit that moved one without
    /// placing it at all named no `Placement` to be checked against.
    pub(crate) fn stay_placed(&self, track: usize) -> Result<()> {
        for (index, slot) in self.slots.iter().enumerate() {
            let Some(written) = slot.written.filter(|_| slot.alive) else {
                continue;
            };
            if released(&slot.kind).is_some() {
                self.release_precedes_its_own_strike(track, index, slot)?;
            }
            if let Some(statement) = stated_by(&slot.kind) {
                self.state_precedes_the_strikes_it_governs(track, index, slot, statement)?;
                self.states_keep_the_order_they_were_written_in(
                    track, index, slot, statement, written,
                )?;
            }
            // A strike is asserted nothing about. It is placed behind everything
            // at its Tick, and both of the other rules place things in front of
            // what is there — so a release or a statement landing later is meant
            // to end up in front of it. "Behind everything" is true when it is
            // placed and need not be true afterwards, which makes it no property
            // of the finished track.
        }
        Ok(())
    }

    /// A release this Edit Set wrote is in front of every strike of its own
    /// channel and pitch at its Tick — #25's defect, as a property.
    fn release_precedes_its_own_strike(
        &self,
        track: usize,
        index: usize,
        slot: &Slot<'a>,
    ) -> Result<()> {
        let Some((channel, pitch)) = released(&slot.kind) else {
            return Ok(());
        };
        for behind in self.sharing(index) {
            if behind.rank < slot.rank
                && strikes(&behind.kind, channel)
                && key(&behind.kind) == Some(pitch)
            {
                return Err(Error::ReleaseBehindItsStrike {
                    track,
                    tick: slot.tick,
                    channel,
                    pitch,
                });
            }
        }
        Ok(())
    }

    /// A channel-state event this Edit Set wrote is in front of the strikes of
    /// its channel that it governs at its Tick — ADR-0008, as a property.
    ///
    /// It governs the strikes from the statement of its own address in front of
    /// it onwards. The strikes before *that* one belong to it, not to this: a
    /// Tick may state an address twice, and the first of them is what the
    /// strikes between them sound under.
    fn state_precedes_the_strikes_it_governs(
        &self,
        track: usize,
        index: usize,
        slot: &Slot<'a>,
        statement: Statement,
    ) -> Result<()> {
        let governs_from = self
            .sharing(index)
            .filter(|other| other.rank < slot.rank && statement.is_stated_by(&other.kind))
            .map(|other| other.rank)
            .max();
        for behind in self.sharing(index) {
            if behind.rank < slot.rank
                && statement.governs(&behind.kind)
                && governs_from.is_none_or(|from| behind.rank > from)
            {
                return Err(Error::StateBehindItsStrikes {
                    track,
                    tick: slot.tick,
                    channel: statement.channel,
                    state: statement.state.named(),
                });
            }
        }
        Ok(())
    }

    /// Two channel-state events this Edit Set wrote at one position are in the
    /// order the Edit Set gave them.
    ///
    /// One position, not one Tick: a strike between two of them is what decides
    /// their order instead, and either side of it they are answering to
    /// different notes.
    fn states_keep_the_order_they_were_written_in(
        &self,
        track: usize,
        index: usize,
        slot: &Slot<'a>,
        statement: Statement,
        written: usize,
    ) -> Result<()> {
        for (other, later) in self.slots.iter().enumerate() {
            if other == index || !later.alive || later.tick != slot.tick {
                continue;
            }
            let Some(order) = later.written else { continue };
            let Some(other_statement) = stated_by(&later.kind) else {
                continue;
            };
            if other_statement.channel != statement.channel || order < written {
                continue;
            }
            if later.rank > slot.rank
                || self.strikes_between(index, statement.channel, later.rank, slot.rank)
            {
                continue;
            }
            return Err(Error::StatesOutOfOrder {
                track,
                tick: slot.tick,
                channel: statement.channel,
                first: statement.state.named(),
                second: other_statement.state.named(),
            });
        }
        Ok(())
    }

    /// Whether a strike of this channel falls between two Ranks at a Tick.
    fn strikes_between(&self, placing: usize, channel: u8, low: i64, high: i64) -> bool {
        self.sharing(placing)
            .any(|slot| slot.rank > low && slot.rank < high && strikes(&slot.kind, channel))
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
        self.slots.sort_by_key(|slot| (slot.tick, slot.rank));
        let end = end.max(self.slots.last().map_or(0, |slot| slot.tick));

        with_delta_times(
            self.slots
                .into_iter()
                .map(|slot| (slot.tick, slot.kind))
                .chain([(end, TrackEventKind::Meta(MetaMessage::EndOfTrack))]),
        )
    }
}

/// What a channel-state event states: its channel and its address. `None` if the
/// event states nothing about a channel.
///
/// The inverse of `Statement::is_stated_by`, which asks whether one event states
/// one known address. This asks an event which address it states, because a
/// check reading the finished track has no address in hand to ask about.
fn stated_by(kind: &TrackEventKind) -> Option<Statement> {
    let TrackEventKind::Midi { channel, message } = kind else {
        return None;
    };
    let state = match message {
        MidiMessage::ProgramChange { .. } => ChannelState::Program,
        MidiMessage::Controller { controller, .. } => ChannelState::Controller(controller.as_int()),
        _ => return None,
    };
    Some(Statement {
        channel: channel.as_int(),
        state,
    })
}

/// What a channel-state event states the channel is on or holds: a program
/// number, or a Controller's value. `None` if the event states nothing about a
/// channel.
///
/// The value, and not only the address, because two tracks stating one channel
/// the *same* value at one Tick leave an order that decides nothing: both ways
/// round the channel ends up where the Take says. See #26.
fn valued(kind: &TrackEventKind) -> Option<u8> {
    let TrackEventKind::Midi { message, .. } = kind else {
        return None;
    };
    match message {
        MidiMessage::ProgramChange { program } => Some(program.as_int()),
        MidiMessage::Controller { value, .. } => Some(value.as_int()),
        _ => None,
    }
}

/// One channel-state event, seen from outside its own track. See `stated`.
pub(crate) struct Stated {
    pub(crate) tick: u32,
    pub(crate) statement: Statement,
    pub(crate) value: u8,
    /// Whether this Edit Set wrote it. A Take carried one in is the author's,
    /// and ADR-0003 keeps it.
    pub(crate) written: bool,
}

/// One strike, seen from outside its own track. See `struck_notes`.
pub(crate) struct Struck {
    pub(crate) tick: u32,
    pub(crate) channel: u8,
    pub(crate) written: bool,
}

/// The channel and key a release ends, whichever of the two spellings it uses:
/// a note-off, or a note-on at velocity zero. `None` if the event ends nothing.
fn released(kind: &TrackEventKind) -> Option<(u8, u8)> {
    match kind {
        TrackEventKind::Midi {
            channel,
            message: MidiMessage::NoteOff { key, .. },
        } => Some((channel.as_int(), key.as_int())),
        TrackEventKind::Midi {
            channel,
            message: MidiMessage::NoteOn { key, vel },
        } if vel.as_int() == 0 => Some((channel.as_int(), key.as_int())),
        _ => None,
    }
}

/// The key a note event names, whichever of the two spellings it uses. `None`
/// if the event does not carry a note.
fn key(kind: &TrackEventKind) -> Option<u8> {
    match kind {
        TrackEventKind::Midi {
            message: MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. },
            ..
        } => Some(key.as_int()),
        _ => None,
    }
}

/// Whether an event strikes a note on this channel: a note-on above velocity
/// zero.
///
/// Velocity rather than status byte, because the format spells a release two
/// ways and a note-on at velocity zero is the second of them. Counting one as a
/// strike would put a state event in front of a release — which is the whole
/// mistake, made one layer down.
fn strikes(kind: &TrackEventKind, channel: u8) -> bool {
    matches!(
        kind,
        TrackEventKind::Midi {
            channel: on,
            message: MidiMessage::NoteOn { vel, .. },
        } if on.as_int() == channel && vel.as_int() > 0
    )
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

#[cfg(test)]
mod tests {
    //! This crate's tests are at the process boundary, in `tests/`, because that
    //! is where `mid`'s behaviour is observable. Two rules are the exception.
    //!
    //! The encoding rule, because the difference between refusing an unwritable
    //! gap and masking it into a smaller one shows up only in a Take sparse
    //! enough to need a delta of 268 million Ticks, and committing a fixture
    //! that strange — or generating one — to watch a single `u28` conversion
    //! would cost more than stating the property here does.
    //!
    //! `stay_placed`, because it can only fire when `place_again` is wrong: a
    //! test at the process boundary would have to ship a broken `mid` to watch
    //! it. Each defect it is shown catching below was also reintroduced in the
    //! real code and caught there, which is the evidence that it works; these
    //! are what stop the check itself from being quietly disabled later by an
    //! edit that inverts a comparison.

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

    use midly::num::u4;

    /// One event, at a distance from the one before it.
    fn event(delta: u32, kind: TrackEventKind<'static>) -> TrackEvent<'static> {
        TrackEvent {
            delta: u28::new(delta),
            kind,
        }
    }

    /// A note-on above velocity zero, on channel 0.
    fn strike(pitch: u8) -> TrackEventKind<'static> {
        TrackEventKind::Midi {
            channel: u4::new(0),
            message: MidiMessage::NoteOn {
                key: u7::new(pitch),
                vel: u7::new(64),
            },
        }
    }

    /// A note-off, on channel 0.
    fn release(pitch: u8) -> TrackEventKind<'static> {
        TrackEventKind::Midi {
            channel: u4::new(0),
            message: MidiMessage::NoteOff {
                key: u7::new(pitch),
                vel: u7::new(0),
            },
        }
    }

    /// A control change, on channel 0.
    fn control(number: u8, value: u8) -> TrackEventKind<'static> {
        TrackEventKind::Midi {
            channel: u4::new(0),
            message: MidiMessage::Controller {
                controller: u7::new(number),
                value: u7::new(value),
            },
        }
    }

    /// The address a control change on channel 0 states.
    fn holding(number: u8) -> Placement {
        Placement::State(Statement::controller(0, number))
    }

    /// #25's defect, in the smallest shape that produces it: a release carried
    /// onto the Tick of a strike of its own pitch, and not placed again. A
    /// synthesiser would strike that note and stop it in the same instant.
    #[test]
    fn a_release_written_behind_its_own_strike_is_a_fault() {
        let events = vec![event(0, strike(60)), event(480, release(60))];
        let mut track = Rewrite::of(&events);
        track.set_tick(1, 0);
        let refused = track
            .stay_placed(0)
            .expect_err("a release behind its own strike silences it");
        assert!(
            matches!(refused, Error::ReleaseBehindItsStrike { .. }),
            "{refused}"
        );
    }

    /// #20's defect: a channel-state event written after the note-ons already
    /// at its Tick. Named here as a `Strike` to place it, which is the mistake —
    /// and the check reads the event rather than the name it was placed under,
    /// so it catches a caller that named the wrong rule as readily as a rule
    /// that was implemented wrongly.
    #[test]
    fn a_state_written_behind_a_note_it_governs_is_a_fault() {
        let events = vec![event(0, strike(60))];
        let mut track = Rewrite::of(&events);
        track.push(0, control(64, 127), Placement::Strike);
        let refused = track
            .stay_placed(0)
            .expect_err("a state behind the notes it governs does not reach them");
        assert!(
            matches!(refused, Error::StateBehindItsStrikes { .. }),
            "{refused}"
        );
    }

    /// Two statements an Edit Set placed at one position, come out reversed. The
    /// Edit Set asked for the second one last, so the second one is what the
    /// channel should hold; reversed, it holds the first.
    #[test]
    fn two_states_written_out_of_the_order_asked_for_are_a_fault() {
        let events = vec![event(0, strike(60))];
        let mut track = Rewrite::of(&events);
        let first = track.push(0, control(100, 1), holding(100));
        let second = track.push(0, control(101, 1), holding(101));
        track.slots[second].rank = track.slots[first].rank - 1;
        let refused = track
            .stay_placed(0)
            .expect_err("two statements reversed leave the wrong one in force");
        assert!(
            matches!(refused, Error::StatesOutOfOrder { .. }),
            "{refused}"
        );
    }

    /// The other half of the rule, and the one that keeps this check honest: a
    /// Take that arrived with a release behind a strike of its own pitch keeps
    /// it. ADR-0003 — the order an event arrived in is the author's, and no Edit
    /// named these.
    #[test]
    fn a_release_that_arrived_behind_its_strike_is_left_where_it_was() {
        let events = vec![event(0, strike(60)), event(0, release(60))];
        let track = Rewrite::of(&events);
        track
            .stay_placed(0)
            .expect("carried-in order is the author's, whatever it is");
    }
}
