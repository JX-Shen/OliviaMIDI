# Decision records

The principles this project's code is built on, and why each one holds. One line
each; the file is the argument.

A principle here is a rule that reading the code can prove false, and that a
piece of code not yet written will have to obey. `CHARTER.md` holds the other
kind of decision — postures, scope and obligations, true of the project whatever
the code does — and where the two disagree, the charter is right. A judgement
made once, about one behaviour, is neither: it is settled in the issue that
asked for it, in a closing comment labelled `decision`, and the code cites the
issue by number. `AGENTS.md` says how to tell the three apart.

An ADR found to be wrong is amended in place rather than superseded, which is
why no file here supersedes another.

| | | |
| --- | --- | --- |
| [0001](./0001-rust-core-as-a-library-cli-as-a-thin-consumer.md) | Rust core as a library, CLI as a thin consumer | Why Rust over Python and Swift, why `battuta` is a library from the first commit, and why synthesis is somebody else's problem |
| [0002](./0002-notes-are-content-addressed.md) | Notes are content-addressed | A note is named by its track, channel, pitch and start Tick — so a name means something across two files, and cannot say that a note moved |
| [0003](./0003-a-take-round-trips-at-the-event-level.md) | A Take round-trips at the event level | What a Take *is*: its events, never its bytes. What "an empty Edit Set changes nothing" promises, and what it leaves to the writer |
| [0004](./0004-a-judgement-made-for-the-human-is-bounded-stated-and-declinable.md) | A judgement made for the human is bounded, stated and declinable | Anything `mid` infers rather than reads has a parameter the human sets, is reported with the value it used, and can be switched off |
| [0005](./0005-the-library-decides-the-fact-the-consumer-decides-the-wording.md) | The library decides the fact, the consumer decides the wording | `battuta` owns what is so, when it is told, and that it is told; `mid` owns the English and the stream |
| [0006](./0006-a-library-takes-no-process-global-resource-without-consent.md) | A library takes no process-global resource without consent | Signals, environment, working directory: nothing that belongs to the process is touched until the process asks by name |
| [0007](./0007-channel-state-is-reported-as-what-is-in-force.md) | Channel state is reported as what is in force, never as the events that set it | What a channel holds is the answer; the events that put it there are clerical history. Why a curve is not summarised into a shape, and why anything with nothing in force falls outside every command |

## How they hang together

`0001` is the root: it makes `battuta` a library, and `0005` and `0006` are that
boundary being defended for a consumer that does not exist yet.

`0002` is the other root. `0004` exists because content addressing deliberately
cannot say *this is the same note, moved*, and `0003` is what content addressing
needs from the writer.

`0007` is `0004` read from the other side, and it is why `0004` carries an
amendment: the claim `0004` could not make about a note is one `0007` never has
to make about a channel, so nothing it reports needs a parameter at all. It
leans on `0003` for the same thing `0002` does — a writer that returns what it
was given, so that the events nothing named come back untouched.

*Refuse rather than answer plausibly* in `CHARTER.md` is argued from more often
than any file here, and it is not here because it changes no code on its own:
each refusal is a judgement about one input, and each is recorded in its issue.

## Where the judgements went

Before the 0.1.0 release this directory held eleven records, and most of them
mixed a principle with the first judgement made under it. They were reorganised
once, before anything was published, and this is the only place that maps the
old numbers to where each one's content lives now. Issues in this repository
that cite an old number are dated records and were left as written.

| was | now |
| --- | --- |
| 0001 Rust core as a library | 0001, unchanged |
| 0002 Notes are content-addressed | 0002, with the test narrative moved to `tests/stacked.rs` |
| 0003 No implicit Rig fallback | *Refuse rather than answer plausibly* and the Piece/Rig boundary in `CHARTER.md`; the setup in `README.md` |
| 0004 Diff matches notes by explicit tolerance | 0004 holds the principle; #6 and #10 hold the matching |
| 0005 A Take round-trips at the event level | 0003 |
| 0006 A Take's Bars come from one stated time signature | #3 |
| 0007 A passage carries the state that precedes it | #4 |
| 0008 An absolute Tick is a `u32` | #15 |
| 0009 The library owns the moment of disclosure | 0005, generalised |
| 0010 Catching signals needs the consumer's consent | 0006, generalised |
| 0011 A pitch is named with sharps, counting from C4 | #7 |
