# No implicit Rig fallback

`mid play` resolves its Rig as `--rig` > `BATTUTA_SOUNDFONT` > **fail**. There
is no fallback to a system soundfont, to FluidSynth's compiled-in default, or to
macOS's bundled GM bank, and none will be added.

The reasoning is in `CHARTER.md` under the Piece/Rig boundary; recorded here
because a future contributor will read this as a missing convenience and try to
"fix" it.

This is also the worked example of *Refuse rather than answer plausibly* in
`CHARTER.md`, and the one ADR-0004, ADR-0006 and ADR-0007 each argue from when
they refuse something of their own. The asymmetric-cost paragraph below is what
they are pointing at, so it is load bearing beyond the Rig.

## Consequences

The first `mid play` on any new machine fails. That is the intended shape of the
trade: setting an environment variable once costs thirty seconds, while an
audition conducted through a silently substituted soundfont costs an aesthetic
judgement that is wrong without anyone being able to tell. The costs are
asymmetric and only one of them is recoverable.

Because the failure is guaranteed rather than exceptional, its message carries
the weight a `doctor` subcommand otherwise would. Two distinct conditions, two
distinct messages: `fluidsynth` not on PATH says `brew install fluid-synth`; no
Rig configured says how to set `BATTUTA_SOUNDFONT`. Never one merged "playback
failed".

The flag is named `--rig`, not `--soundfont`, even though V0 accepts only a
single soundfont path as its value. The vocabulary is correct from the first
day, and V1's named Rigs will not need a rename.

`.asset` deliberately does not carry the Rig. `mid` is a global binary invoked
from inside music project directories, not from this repository; if it read
`.asset` its behaviour would become dependent on the working directory, and the
same command would produce different sound in different places. `.asset` is
repo-scoped and for humans; the Rig is machine-scoped and for the tool.
