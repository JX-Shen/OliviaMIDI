# A Take round-trips at the event level

"An empty Edit Set produces a Take identical to its input" means *event-level*
identity: the written Take has the same header, the same tracks, the same events
in the same order, with the same delta times. It does not promise the same
bytes.

The spec left this open deliberately and asked the first implementation to
answer it rather than let it stay ambiguous.

## Considered Options

**Byte-level identity** is the stronger claim and, as it happens, the one
`fixtures/olivia.mid` satisfies: parsed and written back through `midly` it is
byte-for-byte the file it started as, running status and all. It was still
rejected as the *promise*. Byte identity is a claim about MIDI's encoding
choices — whether a status byte is repeated, how a delta time is packed into a
varint — and those belong to whichever program wrote the file. Two encodings of
the same music are the same Take. Promising byte identity would make `mid` answer
for the writer's habits, and the first Take produced by a DAW that packs a varint
non-canonically would break a guarantee that never had anything to do with the
music.

**Event-level identity** is the claim that is actually load bearing. What a human
wants from `apply` is that nothing they did not ask about was rewritten — the
notes they liked, the tempo, the metre, the controllers, the tracks. That is a
statement about events. It is also the claim that keeps meaning once an Edit Set
is *not* empty, where byte identity has nothing to say at all.

**Preserving the input bytes when the Edit Set is empty** would satisfy both
readings, by copying the file rather than writing it. It was rejected as a lie:
it makes the round-trip test pass without the writer ever running, so the one
path the test exists to protect is the one path it stops covering.

## Consequences

The round-trip assertion re-parses both files and compares their event streams.
It is a direct assertion rather than a snapshot, because it states that two runs
*agree* — a relation no stored blob can express.

Byte identity for the fixture is an observation, not a guarantee, and is
deliberately not asserted anywhere. A test on an unpromised property would
constrain `midly`'s writer on this project's behalf, and would fail one day
without anything the project cares about having broken.

`apply` reaches the event it names and leaves the rest of the parsed Take
untouched, which is what makes the guarantee cheap: a `set_velocity` rewrites one
velocity byte's worth of model and nothing else. Any future Edit must keep
that shape. An implementation that rebuilt a Take from its notes would satisfy
the round-trip test on this fixture and quietly drop every event the note model
does not carry.

This does not settle ADR-0002's open question about occurrence indices surviving
a write and a re-read. Event-level identity says the events come back in the
order they went in, which is what that question needed — but only for Takes this
tool wrote. A Take that arrives from elsewhere and is re-read is still
unexercised, and still waits on a fixture with a genuine collision in it.

## Amended: the mechanism changed, the guarantee did not

The five Edits that followed `set_velocity` cannot reach one event and stop. A
transpose changes the key on *both* of a note's events; a move changes when both
happen, and with them the delta times of their neighbours; a delete takes two
events out and an add puts two in. So `apply` no longer edits the parsed event
list in place. It reads each track once into slots addressed by index, lets Edits
mark, move and add slots without any index ever shifting, and derives the delta
times again at the end from the Ticks.

What the paragraph above was protecting survives intact, and is what the
round-trip assertion still tests: an event no Edit named keeps the Tick it
arrived with, its content, and its order relative to every other event that
stayed put — so a track nothing touched re-encodes to exactly the events it came
in as. What has gone is the *means*. Read "reaches the event it names and leaves
the rest of the parsed Take untouched" as the promise it was making rather than
as the mechanism it happened to name.

The option rejected above is untouched and still rejected. This rebuilds a track's
*event list*, out of that track's own events, and never a Take out of its notes.
Every event the note model does not carry is still carried through, because it was
never taken out in the first place.
