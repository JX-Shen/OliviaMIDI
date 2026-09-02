# The library decides the fact, the consumer decides the wording

`battuta` decides *what is so* and *when a consumer is told*. `mid` decides
what the English for it is and which stream it goes to. The library hands over
a value; it never formats one.

Three things are the library's and must not move to the consumer:

- **The fact.** Which Rig was used, which Bar a Tick falls in, which note a
  pitch is. These are decided once, in one place, so that every consumer agrees.
- **The moment.** *When* the fact is handed over is behaviour. The Rig is
  disclosed after FluidSynth has started and before the audio finishes, so that
  it names the Rig that *was* used and so that an audition somebody interrupts
  has still been disclosed.
- **The obligation.** Where disclosure is mandatory, a consumer cannot reach the
  result without having been handed the fact first. The callback is a parameter
  rather than a subscription.

And one thing is the consumer's and must not move to the library:

- **The wording.** The string `rig: `, the glyph `#` rather than `♯`, `bar 5
  beat 1`, which stream a line lands on. `battuta` exposes `Rig`, `Position`
  and `PitchName` as data with no `Display`, so a consumer cannot inherit
  `mid`'s sentences by accident.

## Why the cut is here

`src/bin/mid/main.rs` states the rule: "The only branches a command module may
hold are formatting ones. Anything that decides *behaviour* lives in
`battuta`." Disclosure is two things at once, and the rule cuts between them.
*That* an audition is disclosed, and *when*, is behaviour. The sentence is
formatting.

`CHARTER.md` is explicit about whose sentence it is:

> Every audition is attributable: **`mid play`** always states which Rig it
> used, on stderr for humans and in the payload for `--json`.

The guarantee is made about the binary. ADR-0001 exists to keep `battuta` an
honest library rather than a folder `mid` keeps its code in, and a library that
chose English on behalf of a consumer that may want none — a native Mac product,
a Chinese note name, a piano-roll cell — had stopped being one.

The founding case was the Rig disclosure (#8), where the library used to take a
`&mut dyn Write` and write the line itself. The same cut has since decided the
Bar and beat placement `inspect` prints, the pitch name beside every number
(#7), and the tolerance line every diff prints (#6). `src/bin/mid/wording.rs`
is the consumer's side of it, in one module, under that name.

## Considered Options

**Keeping the write in the library and recording the exception.** Cheapest, and
defensible: mandatory attribution is a Charter rule and the layering rule is a
posture. Rejected because it is the more expensive of the two in the long run —
the exception is permanent, has to be remembered by everyone who later reads the
rule in `main.rs`, and buys nothing that the callback does not.

**A two-phase API — resolve a Rig, then play through it — with `mid` disclosing
between them.** Rejected: it moves disclosure back before playback, which
reintroduces exactly the "would have been used" reading the moment was chosen to
avoid, and grows the public surface to do it.

**Having `mid` print after `play` returns.** Less machinery, and rejected because
it silently stops disclosing any interrupted audition. Ctrl-C part way through is
the ordinary way to stop a passage, not an edge case. Three seconds is long
enough to form an opinion; an opinion formed and not attributed is exactly what
the Charter rule exists to prevent.

**Returning a formatted `String` for a pitch name.** Rejected on the same
ground: it puts a glyph in `battuta` on behalf of a consumer that may want `♯`.
`PitchName` is three fields and no `Display` impl.

## Consequences

A consumer can pass a callback that does nothing, and so decline to disclose.
That is deliberate. The Charter's guarantee is about `mid`, and `mid`'s
compliance is asserted at the process boundary by
`states_the_rig_on_stderr_and_never_on_stdout` — a test that could not exist if
the guarantee were the library's, because it is a claim about which *stream* the
line reaches. A consumer building its own product answers for its own
attribution; what it cannot do is not be told.

`mid` chooses stderr, and chooses it visibly at each call site. Two reasons live
there rather than in the library: `--json` on stdout stays one parseable
document, and a shell pipeline redirecting the payload cannot drop the
attribution with it.

Every new fact the library learns to state arrives as a type without a
`Display`, and every new line `mid` prints is a function in `wording.rs`. A
`Display` impl on a library type is the smell to look for.
