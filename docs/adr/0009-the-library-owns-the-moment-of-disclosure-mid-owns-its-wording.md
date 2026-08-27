# The library owns the moment of disclosure, `mid` owns its wording

`battuta::rig::play` takes a `disclose: &mut dyn FnMut(&Rig)` and calls it once,
with the resolved Rig, at the moment FluidSynth has started. It does not format
anything. `mid` writes `rig: <path>` to stderr from inside that callback.

It used to take a `&mut dyn Write` and write the line itself.

## Why it moved

`src/bin/mid/main.rs` states the rule this sat against: "The only branches a
command module may hold are formatting ones. Anything that decides *behaviour* —
the order a Rig is resolved in, the refusal to write a Take over its own input —
lives in `battuta`." Disclosure is two things at once, and the rule cuts between
them. *That* an audition is disclosed is behaviour. The string `rig: ` is
formatting. The library was doing both.

`CHARTER.md` decides which side the sentence falls on, and it is explicit about
whose sentence it is:

> Every audition is attributable: **`mid play`** always states which Rig it
> used, on stderr for humans and in the payload for `--json`.

The guarantee is made about the binary. `Audition` already carries `rig:
PathBuf`, so the library was already handing over the fact; what it was
additionally doing was choosing English on behalf of a consumer that may not
want a line of it on a stream. ADR-0001 exists to keep `battuta` an honest
library rather than a folder `mid` keeps its code in, and this was one of the
places it had stopped being one.

## What did not move, and must not

The **moment**. The callback fires after FluidSynth has started, so it names the
Rig that *was* used rather than the one that would have been, and before the
audio finishes, so an audition somebody interrupts has still been disclosed.

That second half is the one an obvious simplification loses. Having `mid` print
after `play` returns would be less machinery and would silently stop disclosing
any interrupted audition — and Ctrl-C part way through is the ordinary way to
stop a passage (ADR-0007), not an edge case. Three seconds is long enough to form
an opinion; an opinion formed and not attributed is exactly what the Charter
rule exists to prevent.

The **obligation** also stays. A caller cannot reach an `Audition` without being
handed the Rig first, because the callback is a parameter rather than a
subscription.

## Considered Options

**Keeping the write in the library and recording the exception.** Cheapest, and
defensible: mandatory attribution is a Charter rule and the layering rule is a
posture. Rejected because it is the more expensive of the two in the long run —
the exception is permanent, has to be remembered by everyone who later reads the
rule in `main.rs`, and buys nothing that the callback does not.

**A two-phase API — `resolve` a Rig, then play through it — with `mid`
disclosing between them.** Rejected: it moves disclosure back before playback,
which reintroduces exactly the "would have been used" reading the current
placement was chosen to avoid, and grows the public surface to do it.

## Consequences

A consumer can pass a callback that does nothing, and so decline to disclose.
That is deliberate. The Charter's guarantee is about `mid`, and `mid`'s
compliance is asserted at the process boundary by
`states_the_rig_on_stderr_and_never_on_stdout` — a test that could not exist if
the guarantee were the library's, because it is a claim about which *stream* the
line reaches. A consumer building its own product answers for its own
attribution; what it cannot do is not be told.

`mid` chooses stderr, and now chooses it visibly. Two reasons live at that call
site rather than in the library: `--json` on stdout stays one parseable
document, and a shell pipeline redirecting the payload cannot drop the
attribution with it.
