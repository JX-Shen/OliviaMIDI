# A judgement made for the human is bounded, stated and declinable

Some of what `mid` reports is not read out of a file. It is inferred: two notes
on either side of a diff are *asserted* to be one note that moved, because their
starts are near enough. Wherever the tool makes an assertion of that kind on the
human's behalf, three things hold:

- **It is bounded by a number the human sets.** The evidence for the assertion
  is a named, documented parameter with a stated default — never a constant
  buried in the code that makes the inference.
- **It is stated with its answer.** Every output that depends on the parameter
  carries the value it used, in the payload and on stderr.
- **It is declinable.** There is a value of the parameter at which no inference
  is made, and the command still answers.

The founding case is `mid diff`'s tolerance: `--tolerance <TICKS>`, defaulting to
a sixteenth note, reported with every diff, and `--tolerance 0` runs the exact
pass alone. How that pass is built — two passes, greedy nearest-neighbour, the
stated iteration order, the refusal to compare two Takes that count Ticks
differently — is the decision recorded in #6 and #10, and the code cites those.

## Why an inference is treated differently from a fact

An identity is derived from a note's content (ADR-0002), and pitch and start
Tick are part of that content. So a note that moved is called one thing in the
before Take and another in the after, and no fact read out of either file says
they are the same note. The second matching pass says it anyway. That statement
is a judgement, and *Human owns the taste* in `CHARTER.md` is what makes a
judgement the tool's to propose and the human's to own.

A diff whose grouping cannot be interrogated is an oracle. A diff read next week
whose grouping depended on a number nobody recorded has the same problem as an
audition heard through an unnamed Rig, and gets the same remedy: the Rig is
disclosed with every audition, and the tolerance is disclosed with every diff.
The same cut as ADR-0005 decides who words that disclosure.

## What this does not cover

A **convention** is not an inference. `F#4` for pitch 66 is a reading of a
number under a rule the file does not state, but it asserts nothing about the
music that the number does not already say, and the number stays beside it. A
convention is chosen once, arbitrarily and uniformly, and is then never
mentioned again — the opposite of a parameter. Which conventions this tool has
chosen, and why they are not flags, is #7.

A **refusal** is not an inference either. Where the tool cannot answer
truthfully it fails and says why, rather than asserting something bounded by a
parameter. *Refuse rather than answer plausibly* in `CHARTER.md` decides which
side of that line a case falls on; the tolerance is on this side because a
diff with no second pass is still a true diff, only a coarser one.

## Considered Options

**A constant, chosen well.** A sixteenth note is a good default and would have
served the fixture. Rejected because the value is not the point: whatever it is,
a diff whose grouping depended on it and did not say so is a diff the human
cannot check. The parameter exists so the number can be interrogated, not so it
can be changed.

**A second bounding parameter** — a pitch bound beside the Tick bound.
Rejected: either it is another named parameter, doubling the surface this
decision exists to keep interrogable, or it is exactly the buried constant it
forbids. There is also no principled value: an octave transposition is twelve
semitones and entirely ordinary.

**Disclosing in the payload only.** Rejected. The human reading a diff without
`--json` is the reader the disclosure is for, and a shell pipeline redirecting
the payload must not be able to drop the attribution with it.

## Consequences

Any future inference inherits all three properties at once, and the first
candidate is already filed: summarising a controller curve (#13) is an inference
about what a run of CC events *is*, and it will have a parameter, a disclosure
and an exact reading, or it will not ship.

Every diff prints a line on stderr whether or not `--json` was asked for. An
agent driving `mid` therefore has to expect that line, which is a cost paid on
every diff so that no diff can be read without its evidence.

Where the parameter is a note value rather than a Tick count, the Take turns it
into Ticks, and two Takes that disagree about the conversion are refused before
any inference is made. A default in Ticks would be meaningless across Takes: 120
Ticks is a sixteenth at 480 PPQ and longer than a quarter note at 96.
