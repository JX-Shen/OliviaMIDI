# A passage carries the state that precedes it

`mid play --bars 5:8` hands FluidSynth a temporary Take holding those Bars
alone. FluidSynth plays a file from its beginning and has no range playback, so
hearing four Bars means giving it four Bars.

That temporary Take holds:

- The notes that **start** inside the range, whole. A note belongs to the Bar it
  starts in — the rule `inspect --bars` already uses — so one that began earlier
  and is still sounding is not in the passage, and one struck inside it keeps its
  full length even where that runs past the last Bar line.
- Everything else that happens inside the range, where it happens in it.
- The state the Take had already set by the time the range began, gathered at
  Tick 0 in the order it was set: tempo, time signature and key signature;
  program change, controller, pitch bend and channel pressure; what the track
  calls itself — its number, and its track, instrument, program and device
  names — along with the channel and port it names; and SysEx.
- Nothing else. A note that merely ends there, a note's aftertouch, an SMPTE
  offset for a start the passage is not, sequencer-specific data no device ever
  hears, and the text events that name a place — a marker reading "Chorus" is
  about the Bar it sits in — all belong to a moment the passage does not
  contain. An `Escape` event is left behind too, on a different ground: it is
  raw bytes escaping the format's own framing, so nothing can be said about what
  it sets, and a guess about undefined bytes is a guess played into someone's
  ears. Whatever the format grows next is left behind for the same reason.

The passage begins at Tick 0, and is at least as long as the Bars it names, so
that a Bar with nothing in it is still heard as a Bar. One visible consequence:
`--bars 1:8` of `fixtures/olivia.mid` is 11520 Ticks where the Take itself ends
at 11516, because the Take stops four Ticks inside its last Bar and the passage
does not. Eight Bars is eight Bars.

It is never presented as a Take. It is written to the temporary directory rather
than beside the user's files; it is removed when the command ends, by any route
it can end by; and `mid play --json` names the user's Take and the passage of it
that was heard, never the temporary file.

"By any route" includes the interrupt, and that is the part that costs
something. `play` blocks for as long as the audio lasts, so Ctrl-C is the
ordinary way to stop a passage part way through rather than an edge case — and
a process killed by a signal runs no destructor. So `battuta` catches `SIGINT`,
`SIGTERM` and `SIGHUP`, unlinks the temporary Takes it has room to remember, and
re-raises the signal with the default disposition restored, so that a shell
still reports the command as interrupted rather than as having chosen some exit
code of its own.

In a process that has agreed to it. Signal dispositions are process-global, so
catching them is asked for rather than assumed: `mid` consents in `main`, and a
library consumer that does not keeps its own handlers and its cleanup by `Drop`.
That is ADR-0010, and it is the one qualification on "by any route" — for `mid`,
which is what this decision is about, nothing changes.

That means a signal handler and a global, in a crate that had neither. Both are
confined to `src/temporary.rs` and both are the minimum the mechanism admits: a
handler may not allocate, may not take a lock, and may not read a `Vec` another
thread might be growing, so the registry is a fixed array of atomic pointers to
leaked C strings, and the only call made from the handler is `unlink`, which
POSIX guarantees is safe there. The handlers are installed lazily, the first
time there is anything to clean up in a process that consented, so a consumer
that never writes a temporary Take never has its signals touched whether it
consented or not — and a signal already being ignored is left ignored, because
that is a decision the process made before `battuta` was called.

"Room to remember" is eight paths, and the number is part of the promise rather
than a detail behind it. `mid play` holds one; the other seven are for a library
consumer auditioning passages on several threads. A ninth outstanding at the
same moment is still removed by `Drop` on every ordinary route out, including a
failure — it is only a *signal* arriving while nine are open that leaves one
behind. The array is not grown to some larger number that would have the same
edge one step further out, and it is not made a `Vec`, because a handler may not
read one another thread might be growing. If a consumer ever needs more than
eight concurrent auditions, the fix is a registry designed for that and not a
bigger constant.

The temporary Take is also the one file in this crate that `Take::write` does
not write. That method writes a new file beside its destination and renames it
on, which is what keeps `apply` from ever writing through a file its input also
names — and it is wrong here, because the path is already on the handler's list
and the intermediate file would not be. A signal landing mid-write would remove
the registered path and leave the other one behind, which is the exact failure
this section exists to prevent. So the bytes go straight into the file that was
registered.

`SIGQUIT` is deliberately not caught: its purpose is an abrupt abort with a core
dump, and one file in the temporary directory is a smaller harm than getting in
the way of somebody debugging. `SIGKILL` cannot be caught by anyone.

## Why the state travels

The acceptance criterion for the ticket names tempo and metre. Those are not two
special cases; they are two instances of one rule, which is that the passage has
to be heard as the Take states it.

The Piece/Rig boundary in `CHARTER.md` decides the rest, and it is mechanical: a
program change is in the MIDI file, so it is the Piece. A passage that dropped
one would be heard on an instrument the Take never names — the same unusable
audition ADR-0003 refuses when it will not pick a soundfont for you, arrived at
from the other side. Worse, in fact: the substituted Rig at least fails loudly
on a machine with no `BATTUTA_SOUNDFONT`, whereas a passage missing its program
change plays perfectly and sounds like a judgement about the music.

## Considered Options

**Leaving the passage at the Ticks it was found at.** Faithful, and it wastes
exactly the time the ticket exists to save: Bars 5–8 of `fixtures/olivia.mid`, at
60 bpm in 3/4, would open with twenty-four seconds of silence.

**Carrying only tempo and time signature** — the acceptance criteria and no more.
Rejected because those two are what the fixture happens to carry, not what the
rule is. A Take with a program change is an ordinary export, and the failure it
would produce here is inaudible *as* a failure: the passage plays, on the wrong
instrument, and the human forms an opinion anyway.

**Keeping only the last value of each piece of state.** Rejected as machinery for
nothing: re-emitting them all at Tick 0 in the order they were set leaves the
last one in force, which is the same result with no table to maintain and no rule
to write about when two controller messages count as the same state.

**Truncating a note that runs past the last Bar line.** Rejected: `inspect --bars
8:8` reports that note's whole duration, and a `play` that shortened it would
make two commands disagree about one note.

**Including notes that are sounding but started earlier.** This is the option
with a real cost, because a passage can now begin without the chord that is
holding underneath it. Rejected anyway: `CONTEXT.md` defines Bar membership by
where a note starts, and taking the other reading here would make `--bars` mean
one thing in `inspect` and another in `play`. One rule that is sometimes
inconvenient beats two rules that are each locally comfortable.

**Writing the temporary Take next to the user's, or leaving it behind.**
Rejected: what it leaves behind looks exactly like a Take the user made, and is
not one. Nothing in `mid` may produce a file that a human could mistake for a
version of their Piece.

## Consequences

Restricting a Take to a Bar range is a library capability, `Take::passage`, not
something `play` does on its way to FluidSynth. `inspect --bars` and `play
--bars` are built on the same `tick_span`, so a range one refuses the other
refuses in the same words — which the suite asserts directly rather than by
reading two similar messages.

`play` cuts the passage *before* it resolves the Rig, which is the one place
this ticket's two criteria pull against each other. A Bar range that does not
exist is a mistake in the command and a missing Rig is a fact about the machine;
reporting the machine first hides the mistake behind an unrelated piece of
setup, and then hands it back once the setup is fixed. Nothing about how a Rig
is resolved or disclosed changes: for a whole Take there is still nothing in
front of it.

`Note` carries the index of the note-off that ends it, alongside the note-on's
that it already carried. Keeping a note whole needs both, and the alternative was
pairing every track a second time: the same rule in two places is how the two
places start to disagree.

`Audition` names the passage as well as the Take and the Rig. An opinion formed
about four Bars and filed against the whole Piece is a record of a judgement
nobody made, which is the same failure of attribution the Rig disclosure exists
to prevent.

A Take that states no time signature governing the whole of it cannot be played
by Bar range at all, by ADR-0006. `mid play` without `--bars` still plays it.

The signal handling is a component rather than a patch on playback, which is why
it lives in `src/temporary.rs` and knows nothing about passages: what must not
be left behind is a fact about the file. Anything this tool writes for its own
use later — a render on the way to somewhere else, a preview — inherits the same
guarantee by being a `TemporaryTake`, and inherits it without installing a
second handler.

What is deliberately still open: state is gathered at Tick 0 as a set of values,
so a controller that was mid-sweep when the passage began arrives at its last
value rather than continuing to move. No Take has needed better, and pretending
otherwise would mean interpolating something the file does not say.
