# Diff matches notes by explicit tolerance

`mid diff` pairs notes between two Takes in two passes: exact matches on
(track, channel, pitch, start tick) first, then a greedy nearest-neighbour pass
within the same track, bounded by a tick tolerance. Whatever remains unpaired is
Added or Removed. Paired notes that differ are classified in a fixed order:
PitchChanged, TimingChanged, VelocityChanged.

## Consequences

The tolerance is a named, documented parameter with a stated default, not a
constant buried in the matching code. This is the point of the decision: the
tolerance is what decides whether a note "moved" or was "deleted and re-added",
and a human must be able to know why the tool said what it said. A diff whose
grouping cannot be interrogated is an oracle, and the human is supposed to own
the judgement here — see *Human owns the taste* in `CHARTER.md`.

Greedy nearest-neighbour is not optimal matching; a pathological Take — dense
material moved by roughly the tolerance — will produce a defensible but not
minimal diff. Accepted for V0. If it becomes a real complaint the fix is a
proper assignment algorithm behind the same interface, which is why the
tolerance parameter, not the algorithm, is the part being fixed here.
