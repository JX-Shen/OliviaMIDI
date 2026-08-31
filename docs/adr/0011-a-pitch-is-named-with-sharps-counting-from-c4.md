# A pitch is named with sharps, counting from C4

`mid inspect` and `mid diff` call a pitch by name — `F#4`, `A2`, `C-1` — beside
the number the file holds. Two conventions decide that name, and a MIDI file
states neither:

- **Sharps, never flats.** Pitch 66 is `F#4`. `Gb4` is not produced by anything.
- **Pitch 60 is C4.** Which puts pitch 0 in octave −1 and pitch 127 in octave 9.

`battuta` hands over a `PitchName { letter, sharp, octave }` and `mid` writes it
down in ASCII. The library chooses which note this is; the consumer chooses the
glyph. That is ADR-0009's cut applied a third time.

## Why name a pitch at all

`CONTEXT.md` is explicit that MIDI carries no notation: it is why a `.mid` file
is a Take rather than a Score. Enharmonic spelling is on the list of what it
does not carry. So naming a pitch is exactly the kind of claim this project
otherwise refuses to make on a file's behalf, and the refusal — printing `pitch
66` and letting the reader convert — was the shipped behaviour until ticket #7.

`CHARTER.md` decides it the other way, and not narrowly:

> The tool should increase agency over time, not dependency. [...] If after six
> months the human's musical understanding has not grown and only their
> prompting has, the tool has half failed.

A human who cannot read `mid inspect` cannot check the agent's work, and cannot
build the mapping between what they hear and how the Take is built. `pitch 66`
and `pitch 61` are two numbers; `F#4` and `C#4` are the third and the fifth of a
chord you can hear resolving. The second reading is the whole of what the
command is for.

What keeps that honest is that the number stays. Every identity on the line
still reads `p66`, `--json` is untouched, and no Edit is expressed in note names.
The name is a gloss on the truth, never a replacement for it.

## Sharps, because a MIDI file has no opinion and every alternative invents one

There is no correct answer here, only a stable one. The candidates:

**Deriving the spelling from the key signature.** A MIDI file may carry a key
signature meta event, and in D major pitch 66 is genuinely F♯ while in D♭ major
pitch 61 is genuinely D♭. Rejected: most exports state no key signature at all,
so the common case falls back to a default anyway; a key signature governs a
stretch of music rather than the whole file, which is the same problem ADR-0006
already answers for time signatures with a refusal; and a spelling that changes
when an unrelated meta event changes makes `F#4` a moving target across Takes,
which is precisely what a diff must not have.

**Naming a pitch by both spellings — `F#4/Gb4`.** Rejected: it doubles the width
of the column that the ticket exists to make readable, and it asserts nothing —
the reader is handed the same choice, one column later.

**Sharps.** Chosen. It is arbitrary, and being arbitrary it is at least uniform:
one pitch has one name in every Take, in every command, forever. The convention
is also the one every DAW's piano roll and every MIDI reference table uses, so
it is the spelling a beginner will already have seen next to a keyboard.

The cost is real and worth stating plainly. A Take in D♭ major will read as a
wall of sharps, and a human reading `mid inspect` on it is being told something
about the file that is not something about the music. The remedy is not a flag;
it is that the name is beside the number, and the number is what the Piece
actually says.

## C4, because middle C has to be somewhere

Pitch 60 is middle C in every convention; what differs is whether that octave is
called 3, 4 or 5. C4 is scientific pitch notation and what MIDI hardware,
trackers and DAWs overwhelmingly print. Choosing it makes `A4` come out as pitch
69, which is the 440 Hz A, so the two most memorable anchors in the system agree.

The edges are stated rather than avoided: pitch 0 is `C-1` and pitch 127 is `G9`.
`C-1` reads a little strangely; it is still the honest consequence of an octave
numbering that puts middle C at 4, and inventing an octave 0 floor to avoid a
minus sign would break the arithmetic everywhere else.

## Considered Options

**Leaving the pitch as a number.** Rejected above; it is the behaviour this
replaces, and the Charter's agency principle is what replaces it.

**Making the spelling a flag — `--flats`.** Rejected. It is a second way to
write down the same file, so two humans reading the same passage would describe
it differently, and a `diff` would have to state which spelling it used the way
it already states its tolerance. The whole point of choosing arbitrarily is to
be able to stop mentioning the choice.

**Returning a formatted `String` from the library.** Rejected by ADR-0009: it
puts a glyph in `battuta` on behalf of a consumer that may want `♯`, or a
Chinese note name, or a piano-roll cell. `PitchName` is three fields and no
`Display` impl, so a consumer cannot accidentally inherit `mid`'s wording.

## Consequences

`mid` prints `#` rather than `♯`. The output is meant to be pasted — into an
Edit Set, into a message to an agent, into an issue — and ASCII survives more of
those journeys than the typographically correct sign does. `battuta` does not
know either character, so a consumer that wants `♯` is one `match` away from it.

Nothing in the Edit vocabulary accepts a note name. `add_note` takes `"pitch":
69`, not `"pitch": "A4"`, and that stays true: a name is a reading of a number
and an Edit names things the file already says.
