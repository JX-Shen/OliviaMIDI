# Rust core as a library, CLI as a thin consumer, playback shelled out

The core is a Rust crate, `battuta`, which is a library from day one; the `mid`
binary is one consumer of it rather than the program itself. Playback is not
implemented — `mid play` shells out to `fluidsynth`.

## Considered Options

Python was the obvious choice: `mido` and `pretty_midi` are mature, and the
first prototype would exist in an afternoon. It was rejected for distribution
(`~/bin/mid` as a single binary, with no virtualenv for an agent to get wrong)
and because the domain is a strongly typed pipeline — parse → normalise →
compare → classify → render — where algebraic data types carry real weight.

Swift was the tempting one. If this becomes a Mac tool used daily, CoreMIDI,
AVAudioEngine and a small SwiftUI surface are all first-class there. It was
rejected *for now* on the grounds that it invites exactly the wrong work: three
days into debugging an AudioUnit with not one bar of music changed. See the
acceptance criterion in `CHARTER.md`.

Go was the pragmatic middle and lost only narrowly, on data modelling.

## Consequences

The library/binary split is the concession to the Swift case: when a native Mac
product is genuinely wanted, it links `battuta` rather than reimplementing it.
Drawing that boundary now costs nothing; drawing it after the fact is expensive.

Shelling out to FluidSynth means `mid` has a runtime dependency it cannot
vendor, and playback failures surface as subprocess failures. That is accepted:
synthesis is not this project's problem, and CoreAudio output is FluidSynth's
supported configuration on macOS.

Python is not banished. `experiments/` stays, because the music-analysis
ecosystem — chord estimation, pitch-class distributions, `music21`, anything
ML-shaped — is unarguably Python's. Rewriting that ecosystem in Rust for purity
would be a much worse trade than keeping two languages with a clear boundary:
`battuta` is the deterministic core, Python is where questions get explored.
