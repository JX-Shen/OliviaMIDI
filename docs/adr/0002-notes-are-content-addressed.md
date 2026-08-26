# Notes are content-addressed

A note's identity is derived from its content — track, channel, pitch and start
tick — with an occurrence index disambiguating notes that collide on all four.
Edit Sets and diffs refer to notes by that identity, and every identity in an
Edit Set is resolved against the input Take before any operation is applied.

## Considered Options

**Synthetic ids assigned at parse time** (track index plus event order) are
stable for repeated parses of the same file, but drift the moment a Take is
edited: insert a note and every id after it shifts. Multi-step editing sessions
would silently address the wrong notes.

**Coordinate addressing** — no ids at all, an Edit naming `{track, bar, pitch}`
— reads well to a human but has the same failure with worse ergonomics: once a
note has moved, its coordinates no longer find it.

**Resolving each identity at the moment its operation runs** is the shorter
implementation, and it is the natural reading of "operations apply in the order
given". It is wrong wherever a collision exists. Deleting the first of two
colliding notes renumbers the second, so a later operation in the same Edit Set
either fails or — worse — lands silently on a different note. Collisions are
rare enough that this would survive ordinary use and any fixture without one.

## Consequences

Content addressing is what makes an identity meaningful *across two files*,
which is the precondition for the two things this project actually cares about:
a diff that can say "this note moved" rather than "one deleted, one added", and
an Edit Set that can be inverted into its own undo. Both of those are load
bearing — see *Diff is a first-class creative object* and *Reversibility
encourages exploration* in `CHARTER.md`.

Resolving identities up front makes an Edit Set atomic with respect to identity.
"Operations apply in the order given" therefore means their *effects* are
ordered, while their *targets* were all fixed before the first one ran. The cost
is that an operation cannot refer to a note created by an earlier `add_note` in
the same Edit Set.

The remaining cost is the disambiguation rule. Two notes genuinely identical in
track, channel, pitch and start tick — a doubled voice, a stacked layer — are
separated only by occurrence index, which is positional. Two things follow.
Within a single Edit Set the up-front binding above neutralises it. Across a
write and a re-read it does not: if serialisation reorders events sharing a
tick, the occurrence indices swap and nothing reports an error. Whether that can
happen is not a matter of judgement but of what the round-trip actually
guarantees, which is an open question until the first implementation answers it.
