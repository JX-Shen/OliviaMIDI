# A Take's Bars come from one stated time signature

Bar lines are computed from a single time signature governing the whole Take,
read from wherever the Take states it. Four things follow, and all four are
refusals rather than answers:

- A Take that states no time signature has no Bars. It is refused, not assumed
  to be in 4/4.
- A Take that does not state one until part way in is refused, naming the Tick.
  A time signature stated at Tick 500 says nothing about Ticks 0–499.
- A Take that states a *different* one later is refused, naming the Tick where it
  changes. Restating the same one changes nothing and is not a change.
- The Take occupies `[0, length_ticks)`, and its Bar count is how many Bars it
  takes to cover that. A partial final Bar is a Bar.

A Bar is `numerator × PPQ × 4 / denominator` Ticks: 1440 at 480 PPQ in 3/4, and
also in 6/8. Two time signatures yield no Bar length at all and are refused
separately, because they are two different things to look at: `0/n` counts no
beats, and a Bar that does not come out as a whole number of Ticks would have to
be rounded, which would misplace every Bar line derived from it.

## Considered Options

**Assuming 4/4 when nothing is stated** is what SMF 1.0 says to do, so this
decision is a deliberate departure from the format's own convention rather than a
gap in it. It was rejected because the parent spec's Implementation Decisions name
a 4/4 assumption anywhere as a defect, and because the failure it produces cannot
be seen: `--bars 5:8` returns a passage, the passage looks plausible, and the
human forms an opinion about the wrong four Bars. The costs are asymmetric in
exactly the way ADR-0003 describes for the Rig, and for the same charter reason —
*Human owns the taste* only holds if the human is judging what they think they
are judging. Stating the time signature in the file costs a minute; an aesthetic
judgement made about the wrong passage costs something nobody can later detect
was spent.

**Assuming 4/4 but announcing it on stderr** is the same trade ADR-0003 already
adjudicated and lost. `mid` is driven by agents, and a warning on stderr in a
pipeline is read by nobody. The precedent is already in the product: `mid play`
states which Rig it used *and still refuses to choose one*. Disclosure is not a
substitute for refusal.

**A `--time-signature` override**, letting a human supply what the file does not
state, was rejected for now rather than on principle. It introduces a second
source of truth, and both ways of resolving the conflict are bad: allowing it to
override a Take that *does* state one lets someone silently re-grid music they
are looking at — and then `mid play --bars 5:8` auditions a passage that is not
the file's Bars 5–8, which is an unattributable audition — while forbidding that
makes one flag behave two different ways depending on the file. The decisive
point is that it is *additive later*: adding it reverses nothing decided here, so
building it now would be speculative surface, the same mistake as the `off_event`
field the first implementation removed.

**A time signature map** — Bar lines derived from the one in force at each Tick —
is the musically complete answer, and is where this goes when a Take needs it. It
was rejected now for three reasons. It looks like it removes the refusal and does
not: a change landing mid-Bar either starts a new Bar there, truncating the one in
progress, or is refused, so the ragged edge survives in a rarer and more obscure
place. Detecting a change costs nearly what handling it costs, so what is being
deferred is the mid-Bar policy and the fixtures it needs, not the scan. And the
upgrade is purely additive: on a Take governed by one time signature the two
functions agree everywhere, so nothing here has to be reversed or renamed.

**Reading the first time signature and ignoring later ones** is worse than
assuming 4/4. The file said, and the tool discarded what it said. **Reading the
first one and applying it to the Ticks before it** is the same fault wearing a
disguise, and it is the one an implementation falls into by accident: taking the
earliest stated time signature without asking whether it was stated at Tick 0.

**Counting Bars as "the Bar containing `length_ticks`"** — `floor + 1` — gives
the same eight Bars as `ceil` on `fixtures/olivia.mid`, whose 11516 Ticks stop
four Ticks inside Bar 8. It diverges on a Take whose last event lands exactly on
a Bar line, which is the ordinary output of a DAW when the music fills eight Bars
exactly: it reports nine Bars and lets `--bars 9:9` succeed on an empty one. A
wrong answer in the common case.

**Counting only whole Bars** amputates the fixture's Bar 8 along with four of its
notes, and contradicts the ticket's own criterion that `--bars 8:8` must work.

**Measuring the Take's extent from its notes** rather than from `length_ticks`
also gives eight Bars here. It was rejected for introducing a second notion of
length beside the one `info` reports, and because a final Bar carrying only
controller data, a program change or a marker would stop counting — which would
make non-note content stop being part of the Piece, against the charter's own test
of that boundary: *is it in the MIDI file?*

## Consequences

`mid inspect --bars` fails outright on whole classes of perfectly ordinary MIDI
file. That is the intended shape of the trade, and it is why every refusal carries
its remedy: one says what is missing and that 4/4 is not assumed, one names the
Tick where the time signature starts, one names the Tick where it changes. A
refusal that names a Tick is a lead, not a dead end.

`Info.time_signature` stays an `Option` and stays undefaulted. This decision is
what makes that `Option` load bearing rather than decorative: something now reads
it and refuses when it is absent.

The one shared capability is `Take::tick_span`, which turns a Bar range into a
half-open span of Ticks. `inspect --bars` filters notes by it, and playing a
passage will be built from it. Half-open because a Bar's end Tick is the next
Bar's first, and naming it as belonging to both is how off-by-one Bars happen.
Because both commands go through it, they refuse identically without sharing
anything else. It stops at the span: producing the temporary Take that playback
needs is the playback ticket's own work.

**`mid info` became stricter, which this ticket did not ask for.** Hoisting the
time-signature scan out of `info` so Bar arithmetic could share it also made an
unrepresentable denominator fatal wherever it appears, where before it was
ignored if a readable time signature had been stated earlier. That earlier
leniency was a latent wrong answer — `info` described a file as 3/4 while saying
nothing about the part of it that could not be read — and it is now specified and
tested rather than incidental. `Info.time_signature` itself is unchanged: it still
reports the earliest one the Take states, which is a true statement about the file
and a separate question from which one *governs* it.

A Take stating 0 Ticks per quarter note is refused by `info` itself, because every
derived view divides by it — and `bar_ticks` refuses it again on its own account,
so its promise never to return zero holds without depending on which caller got
there first.

A Take of zero length has zero Bars, so every `--bars` on it fails. Stated rather
than special-cased into one empty Bar: the formula has no exceptions in it.

`mid info` reports the Bar count beside the Tick count, and reports it as absent
when it cannot be derived. `info` describes a Take rather than diagnosing it, so
all four refusals collapse to one absence there: refusing to describe the Takes a
human most needs to look at would be the wrong trade, and so would printing four
different diagnoses in a payload. The reason and its remedy stay in exactly one
place — the refusal you get from `inspect --bars` when you actually ask for Bars —
which is the same arrangement ADR-0003 uses, where the failure message carries the
weight a `doctor` subcommand otherwise would. Both `--json` and the human line say
the same thing, so an agent reading the payload is not the one left without the
answer.

The fixture cannot exercise most of this. Not one of its 36 notes crosses a Bar
line, so it says nothing about a note belonging to the Bar it *starts* in, and it
states exactly one time signature at Tick 0, so it reaches none of the refusals.
Those tests build small Takes of their own instead of adding a second committed
`.mid` — a fixture has to be inventoried to be trustworthy, as the parent spec's
inventory of `fixtures/olivia.mid` shows, and a test that states its input in
readable terms needs no inventory.

What is deliberately still open: whether a time signature change may land anywhere
other than on a Bar line. The refusal keeps that question closed until a real Take
forces it, which is the point of refusing rather than guessing.
