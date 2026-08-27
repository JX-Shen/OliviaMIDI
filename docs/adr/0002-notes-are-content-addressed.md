# Notes are content-addressed

A note's identity is derived from its content — track, channel, pitch and start
tick — with an occurrence index disambiguating notes that collide on all four.
Edit Sets and diffs refer to notes by that identity, and every identity in an
Edit Set is resolved against the input Take before any Edit in it is applied.

An Edit that changes what an identity is derived from — a note's start Tick, or
its pitch — and an Edit that creates a note both place that note after every
event already at its Tick.

## Considered Options

**Synthetic ids assigned at parse time** (track index plus event order) are
stable for repeated parses of the same file, but drift the moment a Take is
edited: insert a note and every id after it shifts. Multi-step editing sessions
would silently address the wrong notes.

**Coordinate addressing** — no ids at all, an Edit naming `{track, bar, pitch}`
— reads well to a human but has the same failure with worse ergonomics: once a
note has moved, its coordinates no longer find it.

**Resolving each identity at the moment its Edit runs** is the shorter
implementation, and it is the natural reading of "Edits apply in the order
given". It is wrong wherever a collision exists. Deleting the first of two
colliding notes renumbers the second, so a later Edit in the same Edit Set
either fails or — worse — lands silently on a different note. Collisions are
rare enough that this would survive ordinary use and any fixture without one.

## Consequences

Content addressing is what makes an identity meaningful *across two files*,
which is the precondition for the two things this project actually cares about:
a diff that can say "this note moved" rather than "one deleted, one added", and
an Edit Set that can be inverted into its own undo. Both of those are load
bearing — see *Diff is a first-class creative object* and *Reversibility
encourages exploration* in `CHARTER.md`.

**What content addressing cannot do is say that two identities are the same
note.** A note that moved or was transposed changes content an identity is
derived from, so it is called one thing in the before Take and another in the
after, and the diff of "this note moved" is a claim these identities are unable
to make on their own. Nothing here is amended by that: it is the price of
identities that are stable under everything *else*, and it is paid deliberately.
Where the claim is made instead is `mid diff`'s second matching pass, on the
evidence of a stated tolerance, and ADR-0004 holds the whole of that decision —
including why the evidence is a parameter the human sets and why every diff
reports the value it used.

Resolving identities up front makes an Edit Set atomic with respect to identity.
"Operations apply in the order given" therefore means their *effects* are
ordered, while their *targets* were all fixed before the first one ran. The cost
is that an Edit cannot refer to a note created by an earlier `add_note` in
the same Edit Set.

**Where a changed note lands is a rule about identity rather than about Edits,
which is why it is recorded here.** The occurrence index is counted in note-on
order within a track. So a note arriving at a Tick where another of the same
track, channel and pitch already begins settles which of the two is `n0` purely
by where its note-on is written. Placed ahead, it takes `n0` and renames a note
nobody asked to touch. Placed behind, every identity already there survives and
the new note takes the next index. Only the second lets both of the things this
project promises hold at once: that a note `add_note` created is
indistinguishable from one that was always there, and that notes an Edit Set did
not name keep the identities they had.

It reaches `move_note` and `transpose_note` for the same reason it reaches
`add_note`, although only `add_note` looks like it makes something. A move
changes a note's start Tick and a transpose changes its pitch, and both are
content an identity is derived from — so both can land a note on top of another
and renumber it. `set_velocity` and `resize_note` change nothing an identity is
derived from, and so leave a note exactly where it sits.

A note-off is placed the other way, before the events already at its Tick, and
that is not an identity rule at all: occurrence indices are counted in note-on
order and a release has no say in them. The reason is audible instead. A release
landing exactly on the next strike of the same pitch would, placed last, silence
the note that strike had just begun — a synthesiser stops a pitch, not an
identity.

**This is also what closes the question left open below, for one of the two
cases it names.** Which of two simultaneous note-ons is written first is no
longer whatever the encoder happened to do; within a single `apply` it is this
rule, applied deliberately. The suite now builds a genuine collision on purpose
— `apply` is the first thing in this tool that can — and asserts that the note
already there keeps `n0` while the new one takes `n1`. What is still unexercised
is a Take arriving from elsewhere, written by a program that ordered its
simultaneous events differently.

The remaining cost is the disambiguation rule. Two notes genuinely identical in
track, channel, pitch and start tick — a doubled voice, a stacked layer — are
separated only by occurrence index, which is positional. Two things follow.
Within a single Edit Set the up-front binding above neutralises it. Across a
write and a re-read it does not: if serialisation reorders events sharing a
tick, the occurrence indices swap and nothing reports an error. Whether that can
happen is not a matter of judgement but of what the round-trip actually
guarantees, which is an open question until the first implementation answers it.
