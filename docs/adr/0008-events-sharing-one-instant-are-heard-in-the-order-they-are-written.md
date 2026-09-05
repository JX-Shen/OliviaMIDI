# Events sharing one instant are heard in the order they are written

Where several events share one instant, the order they are written in is the
order they are heard. One rule decides that order; nothing states it.

MIDI has no simultaneity. A file is a sequence, and *the same Tick* means *no
time between*, not *at once* — a synthesiser meets those events one after
another and acts on each as it arrives. So a Tick is not an instant but a
sequence carrying one timestamp, and every event in it has a **Rank**.

A score does not have to answer this. Notation has two dimensions: a state lives
above, below or to the left of the staff — a key signature, a tempo marking, a
`Ped.` — and the notes live on it. A state written at a moment governs that
moment, and *which is read first* never arises, because the state is not in the
queue at all. MIDI has one dimension and everything is in one queue. Flattening
two dimensions into one is lossy, and the loss is exactly this: a question
notation never had to answer, which a file must answer anyway, every time.

So the answer is not a matter of taste to be settled per event kind. It is one
rule, and where notation has already settled the case, the rule translates it
rather than inventing: a Program is met before the notes struck at its Tick,
because a note has to sound on the instrument the Take now names; a damper is
met after the notes released at its Tick, because that is what a pianist's
`Ped.` under a beat means — the foot comes down as the hand lifts, and does not
catch what has just ended.

## Considered Options

**Leaving it to each Edit**, which is what the code did until this was written.
The order was decided in five separate places — by event kind, by Edit arm, by
which end of a list a lookup started from, by when a target was resolved, and by
which track an event sat on — each by whoever wrote that branch, none against a
rule, because there was no rule and the concept had no name. It produced five
defects of one shape: a command that succeeded, a file that was valid, a reading
that reported one thing and a synthesiser that produced another. Not one was
caught by a test. Every one was found by rendering audio and comparing it.

**Letting an Edit state a Rank.** It would answer every case exactly and it is
refused. An Edit Set that addresses below the Tick is addressing the encoding
rather than the music, and it is one step from the selector `CHARTER.md` forbids
by name. A Rank is derived from this rule; it is never an input.

**Normalising a Take's event order on read**, so that every Tick arrives in a
shape the rule handles cleanly. Refused twice over. It contradicts ADR-0003 in
that ADR's own words — the same events *in the same order* — so an empty Edit
Set would stop being empty, and `mid diff`, which compares state, would report
no difference between two files that differ. And it is not the harmless
tidying it appears to be: the same reordering is bit-identical on one Take and
32 dB apart on another, because whether it is audible depends on what else
shares the Tick. Taken seriously, the criterion that licenses it also licenses
deleting an overridden duplicate statement, which ADR-0003 forbids.

**A table of event kinds**, each with its own placement. It is where the code
already was, and it makes *from this Tick* mean different things depending on
which controller number an Edit names. One rule that a reader can apply is worth
more than a table that is right more often.

## Consequences

**One rule, applied in one place.** A branch that adds an event says what kind
of thing it is placing; it does not choose a number. The placement is decided
once, when the track is re-encoded — which it must be, because an Edit that runs
later can still move a note onto the Tick in question.

**Every lookup asks the same question the rule answers.** Where a Take states
one thing twice at an address, *the one in force* means the last by Rank. A
lookup that finds it by position in an array is asking a different question, and
will answer differently as soon as anything is re-ranked.

**A Rank is checkable without a Rig.** The rule is a property of the written
track: state before the strikes it governs, a release before a strike of its own
pitch, and each of one Edit Set's insertions in the order it asked for. That is
an assertion over the finished track, not a listening test — and it would have
caught all five defects that produced this record.

**Where the file cannot express the answer, `mid` refuses.** A Rank orders
events within one track. A channel's state written on one track and its notes on
another have no order the file states, and none this rule can give them, so an
Edit that would depend on one is refused rather than answered plausibly
(`CHARTER.md`). Everything a Tick *does* determine is documented instead, in
`mid apply --help`, so that an agent can predict the placement rather than
discover it.

**Carried-in order is the author's.** Where a Take was written with a release
behind a strike at one Tick, that is a fact about the file and ADR-0003 keeps it.
The rule places what an Edit puts there; it does not rearrange what arrived.
