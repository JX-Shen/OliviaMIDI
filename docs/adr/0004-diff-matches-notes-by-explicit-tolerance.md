# Diff matches notes by explicit tolerance

`mid diff` pairs notes between two Takes in two passes: exact matches on
(track, channel, pitch, start Tick) first, then a greedy nearest-neighbour pass
within the same track and channel, bounded by a Tick tolerance. Whatever remains
unpaired is Added or Removed. A pair that differs reports **every** way it
differs, in the fixed order pitch, start, duration, velocity.

The tolerance is a named, documented parameter with a stated default, never a
constant buried in the matching code. It is `--tolerance <TICKS>`, it defaults
to a sixteenth note — the Take's ticks per quarter note divided by four — and
every diff states the value it used.

## The second pass makes a claim ADR-0002 declined to make

An identity is derived from a note's content, and pitch and start Tick are part
of that content. So a note that moved or was transposed is called one thing in
the before Take and another in the after, and there is no single name to report
it under. Content addressing cannot say *this is the same note, moved*: that is
precisely what it gave up in exchange for identities that mean something across
two files.

The second pass says it anyway, and the tolerance is the whole of its evidence.
Two notes on the same track and channel whose starts are near enough are
asserted to be one note that moved. That assertion is a judgement, not a fact
read out of either file, which settles three things about how it is made:

- **It is bounded by a number the human sets.** *Human owns the taste* in
  `CHARTER.md` is what makes the tolerance a parameter rather than a constant. A
  diff whose grouping cannot be interrogated is an oracle.
- **It is stated with its answer.** Every diff carries the tolerance it matched
  with — `tolerance_ticks` in the payload, and a line on stderr for a human. A
  diff read next week whose grouping depended on a number nobody recorded has
  the same problem as an audition heard through an unnamed Rig, and gets the
  same remedy (ADR-0003, ADR-0009).
- **The exact-only reading stays reachable.** `--tolerance 0` runs the first
  pass alone. Anyone who does not want the claim made on their behalf can
  decline it and still get a diff.

A pair carries **both notes whole** rather than one identity and a set of
deltas. Either identity alone would leave a reader unable to find the note in
one of the two Takes they are being told about, and the deltas are arithmetic
anyone can do once both notes are in front of them.

## A change is a set, and length is one of its members

ADR-0004 named three classifications — PitchChanged, TimingChanged,
VelocityChanged — read in a fixed order. Two things were wrong with that and are
now settled.

**The order is a presentation order over a set, not a priority that stops at the
first hit.** A note that was moved *and* softened underwent two changes. Naming
only the first would hide the other inside the one command a human runs to find
out what changed, against *you can hear which change caused the difference* in
`CHARTER.md`.

**TimingChanged is two changes, not one.** It is split into `start` and
`duration`. A note's start moving is a rhythmic placement; its length changing
is articulation; they are different musical events and a human told only
"timing" cannot tell which they are looking at. Before the split, `duration` was
in no classification at all, so `resize_note` — one of the six Edit kinds — was
visible to `diff` as nothing whatsoever: `mid diff` answered "no differences"
after a note had been lengthened by 400 Ticks. That was issue #10, and it is
what a diff exists to prevent rather than to commit.

The order is therefore pitch, start, duration, velocity: `duration` takes its
place beside the `start` that TimingChanged used to cover, and the names are the
four fields `mid inspect` already prints, so a diff and an inspect say the same
word for the same number.

## Ticks are only comparable within one denomination

`diff` compares raw Ticks. Two Takes counting a different number of Ticks to the
quarter note are refused rather than compared: the same four quarter notes at
480 and at 96 share only the one starting at Tick 0, and everything else reports
as added plus removed. That is a wrong answer with nothing in it to reveal
itself.

Refused rather than converted, because *Ticks are the truth; bars, beats and
seconds are all derived views* (`CHARTER.md`) and V0.1 performs no conversion
anywhere. Converting would also have to round, and a rounded Tick is a Take
neither file states.

The tolerance is the same question one level down. It is spelled in Ticks,
because a Tick is what the flag can be checked against and what the matching
actually does — but a *default* in Ticks would be meaningless, since 120 Ticks
is a sixteenth note at 480 and longer than a quarter note at 96. So the default
is a note value and the Take turns it into Ticks. Both Takes agree on that
number, or there is no diff to produce.

A sixteenth is small enough not to pair two different notes of a dense texture
and large enough to cover the nudging a quantise or a humanise does. Anything
wider is a judgement about a particular Piece, and belongs to whoever is looking
at it.

## Greedy makes the iteration order a contract

Greedy nearest-neighbour is not optimal matching, and it makes the order
candidates are considered decide the pairing: two unmatched notes equidistant
from a candidate produce different diffs depending on which is reached first. So
the order is stated rather than left to whatever iteration happened to do.

Before-notes are considered in the order `Take::notes` fixes — track order, then
note-on order — which is already part of that function's contract because it is
what fixes the occurrence index in every identity. Among candidates, nearest in
Ticks wins; a tie goes to the smaller pitch distance, because a transposed note
sits at the same Tick as everything else that did not move; a remaining tie goes
to the after Take's own note order. Two runs on the same pair of Takes therefore
agree.

Pairing is bounded by track **and channel**. ADR-0004 originally said track
alone. No Edit changes a note's channel, so the extra constraint costs nothing
that can arise from editing, and without it two different parts sharing a track
would have their notes declared the same note.

## Considered Options

**A pitch bound as well as a Tick bound.** Rejected: it would be a second
number deciding the grouping, and either it is another named parameter — doubling
the surface the ticket exists to keep interrogable — or it is exactly the
buried constant this decision forbids. There is also no principled value: an
octave transposition is twelve semitones and entirely ordinary.

**Converting one Take's Ticks to the other's PPQ instead of refusing.** Rejected
above.

**Reporting a changed note under its before-identity alone, with deltas.**
Rejected: it names the note in one of the two Takes the diff is about and leaves
it unfindable in the other.

## Consequences

A pathological pair — dense material moved by roughly the tolerance — produces a
defensible but not minimal diff. Accepted for V0. If it becomes a real complaint
the fix is a proper assignment algorithm behind the same interface, which is why
the tolerance parameter, and not the algorithm, is the part being fixed here.

The sharpest form of that is two Takes with no shared ancestry: notes that
happen to share a track, a channel and a Tick will be paired and reported as
changed, however far apart in pitch. `mid diff` on two unrelated files answers
rather than fails, and the answer is total and deterministic rather than
minimal. This is the cost the tolerance buys, which is the second reason every
diff states the number it used and `--tolerance 0` is spelled.

`Diff` no longer has a `velocity_changed` field. A velocity change is now one
member of `changed`, with `changes: ["velocity"]`, and carries both notes. This
is a breaking change to the `--json` contract, made inside V0.1 and before any
consumer outside this repository exists.
