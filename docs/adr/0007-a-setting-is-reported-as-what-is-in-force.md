# A setting is reported as what is in force, never as the events that set it

A Take carries settings — which Program a channel is on, what value it holds for
a controller, how far a channel is bent, what tempo is running — put there by
events scattered through the Take and holding until something says otherwise.
Every command reports the setting. `inspect` states what is in force where a
passage begins, including what was set many Bars earlier; `diff` compares what is
in force at each moment either Take says anything, and consecutive moments
agreeing on the same disagreement collapse into the one difference that states
it.

Most of these settings belong to a channel. A tempo does not: it has no channel
and governs the whole Take. It is here anyway, because every sentence of the
argument below is about a *setting in force* and none of it is about a channel —
see the amendment at the foot of this record.

The events are still listed, because an Edit has to name an address and a reader
has to be able to copy one. What is never done is deriving an answer from their
*shape*.

## Why not the events

Comparing two event streams answers a question nobody asked. It reports that a
byte moved between tracks, that an export re-stated the same Program at every
Bar, that one crescendo is forty differences. Each of those is a true statement
about the file and none of them is a statement about the Piece, which is what
`CHARTER.md` gives `diff` as its whole job.

State is also what is actually heard. A synthesiser's program is channel state,
so it is what `mid play` hands one; a controller value is in force at the first
note of a passage whether or not the passage contains the event that set it. A
report of state is therefore a report of the audition a human is about to form
an opinion from, and a report of events is a report of the file's clerical
history.

## Considered Options

**Comparing the event streams**, which is the shorter implementation and needs
no notion of *in force* at all. Rejected on the paragraph above: it is
faithful to the bytes and unreadable as music.

**Deriving a shape from the events** — segmenting a stretch of one controller
into a rise or a fall, and then matching one Take's shape against the other's.
This is the tempting middle, because a state comparison of a curve can still be
several rows. Rejected because it is an inference in ADR-0004's sense and
nothing here needs one: *which value is in force* and *where the highest one
falls* are both read straight out of the file, and they are enough to say that
the expression now reaches 100 by Bar 6 where it used to reach it by Bar 7.
Asserting on top of that which events *were* one gesture adds a claim without
adding an answer. The stress test that settled it is in #13.

**Reporting state and dropping the event listing.** Rejected: an address that
appears nowhere cannot be edited, and this project's legibility debts are owed
by `diff` rather than paid by hiding things from `inspect`.

## Consequences

**Everything else on ADR-0003's carry list joins on these terms.** Pitch bend
and channel aftertouch are the next two, and #11 already says they follow the
same pattern; what this record fixes is that the pattern is a state comparison,
not a cheaper event comparison chosen per feature. *These terms* is the whole of
them and not the comparison alone: a setting that reaches `diff` and not
`inspect` has joined half of this record, which is where pitch bend stands and
what #43 is for.

**This record says what a reading reports, and never which of two statements is
the reading.** Where two tracks state one setting at one Tick, which of them a
listener hears is not something the file says — that is ADR-0008, and
**Unranked** in `CONTEXT.md` is the word for where the question has no answer to
report. Nothing here decides it. A setting joining these terms does not thereby
inherit the answer another setting was given, either: a channel's state is ranked
against another track's writing of that channel, and a tempo, having no channel,
is ranked against another track's tempo. Same word, different thing each is
ranked against. See #42.

**A thing with no value in force falls outside every command by
construction.** MIDI's channel mode messages — All Notes Off and its neighbours
— ride on the control change event type and are instructions rather than
settings: they happen and are over. Nothing here has anything to say about them,
which is a hole rather than an oversight, and it is recorded and pinned by a
fixture in #13.

**ADR-0004 carries an amendment because of this record.** It had predicted that
summarising controller data would be an inference needing a parameter. The
prediction was wrong about the feature and right about the principle, and its
Consequences now say which.

The judgements arguing from this are #12, which made the choice for Programs;
#13, which made it for Controllers and found the second case that turned it into
a principle; and #32, which made it for tempo and pitch bend and is what
occasioned the amendment below.

## Amended by #32

Two sentences were wrong, and neither of them was the principle.

**It said *channel state*, and the principle is not about channels.** Written
with two applications in hand — a Program and a Controller, both a channel's —
the record took the word for the subject. Nothing in *Why not the events* or in
either rejected option depends on it: an accelerando written as forty tempo
statements is the crescendo argument with a different noun, and a tempo is more
plainly *what is actually heard* than a Program is. #32 was the judgement that
found a setting with no channel arguing from this record, so the scope was
corrected to the one the argument always had. A record whose title has to be
read past is a record that will be read past.

The correction is a widening and not a licence: the new Consequence above marks
the line, because *the word channel is not load-bearing here* and *a tempo is
like a Controller in every respect* are two different claims and only the first
is made.

**It said *collapse into the one row*, and a row is a layout.** The clause is
about not reporting an accelerando as forty differences, and it was written when
one difference happened to occupy one line of output. #32's human rows spend
three lines on a difference whose reading does not fit one, and reading this
record as a promise about line count would have made a typographic accident into
a principle. *One difference* is what was meant and is now what it says.
