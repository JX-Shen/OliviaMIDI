# Decision records

What this project has settled about how the code works, and why it works that
way. One line each; the file is the argument.

`CHARTER.md` holds the other kind of decision — postures, scope and obligations,
true of the project whatever the code does. Where the two disagree, the charter
is right. A decision that changes no code is a charter clause and not an ADR,
and an ADR found to be wrong is amended in place rather than superseded — which
is why no file here supersedes another. `AGENTS.md` argues both at length, and
is not shipped with the crate.

| | | |
| --- | --- | --- |
| [0001](./0001-rust-core-as-a-library-cli-as-a-thin-consumer.md) | Rust core as a library, CLI as a thin consumer | Why Rust over Python and Swift, why `battuta` is a library from the first commit, and why synthesis is somebody else's problem |
| [0002](./0002-notes-are-content-addressed.md) | Notes are content-addressed | A note is named by its track, channel, pitch and start Tick — so a name means something across two files, and cannot say that a note moved |
| [0003](./0003-no-implicit-rig-fallback.md) | No implicit Rig fallback | `--rig` > `BATTUTA_SOUNDFONT` > fail, and nothing after that. The worked example of *Refuse rather than answer plausibly* |
| [0004](./0004-diff-matches-notes-by-explicit-tolerance.md) | Diff matches notes by explicit tolerance | How `diff` decides two notes are one note that moved, why the evidence is a number the human sets, and why a change is a set rather than one label |
| [0005](./0005-a-take-round-trips-at-the-event-level.md) | A Take round-trips at the event level | What "an empty Edit Set changes nothing" means, when the bytes are allowed to differ |
| [0006](./0006-a-takes-bars-come-from-one-stated-time-signature.md) | A Take's bars come from one stated time signature | Four refusals, and why none of them is a missing convenience |
| [0007](./0007-a-passage-carries-the-state-that-precedes-it.md) | A passage carries the state that precedes it | What a temporary Take cut from bar 5 has to bring with it to sound like bar 5, and what it must leave behind |
| [0008](./0008-an-absolute-tick-is-a-u32-and-a-longer-take-is-refused.md) | An absolute Tick is a `u32` and a longer Take is refused | Where the number stops, and why it stops rather than wraps |
| [0009](./0009-the-library-owns-the-moment-of-disclosure-mid-owns-its-wording.md) | The library owns the moment of disclosure, `mid` owns its wording | An audition must be attributable; the English attributing it is the consumer's |
| [0010](./0010-catching-signals-needs-the-consumers-consent.md) | Catching signals needs the consumer's consent | `battuta` takes no signal until it is asked, because the host may have its own |
| [0011](./0011-a-pitch-is-named-with-sharps-counting-from-c4.md) | A pitch is named with sharps, counting from C4 | Naming a pitch settles two conventions a MIDI file does not carry, and why the number never goes away |

## How they hang together

`0001` is the root: it makes `battuta` a library, and `0009` and `0010` are that
boundary being defended for a consumer that does not exist yet.

`0002` is the other root. `0004` exists because content addressing deliberately
cannot say *this is the same note, moved*, and `0005` and `0011` both depend on
what an identity is derived from.

`0003` is cited by `0004`, `0006` and `0007` — not for anything about Rigs, but
as the case each of them argues from when it refuses something of its own.
