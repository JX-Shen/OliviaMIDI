# Notes are content-addressed

A note's identity is derived from its content — track, channel, pitch and start
tick — with an occurrence index disambiguating notes that collide on all four.
Edit Sets and diffs refer to notes by that identity.

## Considered Options

**Synthetic ids assigned at parse time** (track index plus event order) are
stable for repeated parses of the same file, but drift the moment a Take is
edited: insert a note and every id after it shifts. Multi-step editing sessions
would silently address the wrong notes.

**Coordinate addressing** — no ids at all, an Edit naming `{track, bar, pitch}`
— reads well to a human but has the same failure with worse ergonomics: once a
note has moved, its coordinates no longer find it.

## Consequences

Content addressing is what makes an identity meaningful *across two files*,
which is the precondition for the two things this project actually cares about:
a diff that can say "this note moved" rather than "one deleted, one added", and
an Edit Set that can be inverted into its own undo. Both of those are load
bearing — see *Diff is a first-class creative object* and *Reversibility
encourages exploration* in `CHARTER.md`.

The cost is the disambiguation rule. Two notes genuinely identical in track,
channel, pitch and start tick — a doubled voice, a stacked layer — are separated
only by occurrence index, which is positional and therefore inherits a small
version of the drift problem. This is accepted because such collisions are rare
and local, where synthetic-id drift was global.
