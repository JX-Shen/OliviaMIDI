# OliviaMIDI — Project Charter

## What this is

A pair of humans and agents work on one piece of music together. The music lives
as MIDI files on disk. The agent can read those files precisely, change them
mechanically, explain what changed, and play the result back. The human decides
whether it is any good.

That is the whole product. It is not an AI music generator, and it is not a
smart MIDI editor.

> Do not optimise for *how much music the AI can produce*. Optimise for *how
> smoothly a human and an agent can make, understand, and compare individual
> creative decisions about the same musical object*.

That sentence is the spine. It rules out one-click composition, style prompts,
whole-song autocompletion, a composition DSL, AI self-evaluation loops, and
premature DAW-ification. It rules in selection, local edits, preservation, diff,
A/B, undo, audition, and explanation.

`battuta` is the Rust core. `mid` is the binary it ships. OliviaMIDI is the
product these belong to.

## The one boundary that is mechanically decidable

Everything else in this charter is a posture. This one is a rule:

> **What is inside a Take belongs to the Piece. What is required to turn a Take
> into air belongs to the Rig.**

The test is literal: *is it in the MIDI file?*

- Velocity is in the file → Piece.
- CC7 channel volume is in the file → Piece.
- Program change — whether this part is brass or strings — is in the file →
  Piece. **Orchestration is composition.**
- The soundfont, the synthesiser, the playback gain → not in the file → Rig.

This matters because "the brass sounds wrong" has two entirely different
diagnoses: the brass *part* is badly written (Piece), or the brass *samples* in
this soundfont are bad (Rig). A tool that blurs these lets a human and an agent
degrade a well-written line in order to compensate for a poor sample — and the
result will genuinely sound better, which is why the mistake is invisible.

Because orchestration is in the file, editing it is legitimate scope for this
tool. A program change is an ordinary mechanical Edit, and reading, changing and
diffing orchestration belongs on the Piece side. It is deliberately absent from
V0.1 and is the natural first step after it. Do not re-file it under the Rig:
what a program change *sounds like* is the Rig's business, but *which program is
selected* is the Piece's.

Consequences, held as hard rules:

- A diff never reports Rig differences, even when two Takes were heard through
  different Rigs.
- Every audition is attributable: `mid play` always states which Rig it used, on
  stderr for humans and in the payload for `--json`. There is no flag whose
  purpose is to hide it.
- The Rig is never chosen implicitly. No fallback to a system soundfont. An
  unnamed Rig cannot be referred to, and a Rig that cannot be referred to cannot
  serve as a control in a comparison.

## Design principles

**Human owns the taste; agent owns the execution bandwidth.** The human decides
what sounds good and what is wrong. The agent analyses, tries, edits, and
compares faster than a human can — but must not dress its judgement up as the
answer. The point is not human-in-the-loop; it is that the human remains the
authorial centre. If it becomes agent-writes, agent-reviews, agent-revises and
the human only clicks Accept, the product has already failed.

**Refuse rather than answer plausibly.** Where the tool cannot answer truthfully
it fails and says why. It does not substitute a soundfont it was not given,
assume 4/4 for a Take that states no time signature, truncate a Tick it cannot
hold, or compare two Takes that count Ticks differently. Each of those produces
an answer that looks right and carries nothing in it to reveal that it is not,
and the costs are asymmetric: setting an environment variable costs thirty
seconds, while an aesthetic judgement formed about the wrong thing costs
something nobody can later detect was spent. ADR-0003 is the worked example the
others argue from.

The limit matters as much as the rule. Refuse when the alternative is to assert
something false; do not refuse in order to protect a check that another surface
already provides. `Take::bar_lines` answers *no bars* rather than failing, so
that `mid inspect` can still list the Take a human most needs to look at, and
nothing caps how long an Edit Set may be in order to keep it readable — the diff
is what makes a long one auditable.

**Artifact-first, conversation-second.** The conversation orbits a real,
evolving MIDI artifact — the chat log is not the work. The right interaction is
"we are looking at bars 5–8, we just changed the left hand, this is the current
Take", not "generate another one based on what we discussed". The agent should
behave like a coding agent standing inside a repo, not a consultant offering
suggestions across a chat window.

**Edit, don't regenerate.** The default action is a local change, not a fresh
generation. When the human says "this is too busy", the first question is *which
part can be thinner?* — not *shall I rewrite four bars?* Continuity is what
makes a piece become *theirs*: old decisions get kept, revised and reinterpreted
rather than repeatedly replaced.

**Preserve intent by default.** Anything not explicitly under discussion stays
untouched. Discussing harmony is not licence to rewrite the melody; discussing
dynamics is not licence to change rhythm. This protects existing work, and it is
what gives an audition explanatory power — you can hear *which* change caused
the difference.

**One hypothesis, one experiment.** When the human says "this feels off", the
agent must not change five things at once. Propose the possible causes, then
change one primary variable at a time. This makes the process a learning
process: the human is building a mapping between *what they hear* and *how the
music is built*, not just receiving a better result.

**Audition is the ultimate test.** Analysis, theory, diffs and agent
explanations are all supporting evidence. Listening is the arbiter. A change
that is theoretically better and sounds worse is rejected. The loop is
inspect → change → listen → react, not a loop of prose.

**Diff is a first-class creative object.** Version difference is not merely
engineering history; it is part of understanding the work. The human should be
able to answer "what actually differs between A and B?" — in musical terms, not
in bytes. Over time the diff becomes a way of learning one's own preferences.

It carries a second duty that the first one hides. **An Edit Set states what was
asked for; a diff states what happened.** The two come apart: a `move_note` can
land a note on top of another and renumber a note nobody named, and the Edit Set
that asked for the move cannot state that consequence while a diff can see it.
So the diff is also how the human checks the agent, and where the two disagree
the diff is what to trust.

That is what keeps an Edit Set free to stay dumb. It never has to grow a way of
summarising itself, and every way of giving it one — a selector naming a range
of events, a label saying what the batch was for — is a step towards the
composition DSL below. It also settles where a legibility debt is owed: when a
change is mechanically large but musically small, the command that must be made
to explain it is `diff`, not `edits.json`.

**Reversibility encourages exploration.** Any significant change must be easy to
compare, undo, and keep alongside alternatives. The human should never feel "if
I let the AI touch this, it might destroy the good thing I have". Lower the cost
of trying things without lowering the human's authority to decide.

**Use existing musical standards; invent only interaction semantics.** MIDI is
MIDI. Do not redefine musical representation and do not invent a composition
DSL. The thing genuinely worth designing is how a human and an agent *refer to,
locate, change and compare* music — not another language for notes and bars.
Stay inside the existing music ecosystem rather than building an agent-only
island.

**Agent behaviour should resemble a good collaborator, not an oracle.** A good
human collaborator hears "something's wrong here", looks at what is actually
happening, offers possibilities, tries a version, and is willing to say "I'm not
sure — maybe it's this, have a listen." Keep that epistemic humility, especially
on questions like "why does this passage work" — separate fact, theoretical
analysis, and aesthetic interpretation.

**Beginner language is a valid interface.** Nobody should have to learn the
vocabulary before being allowed to express a musical intention. "It's too
crowded here", "it feels like it drops", "I want it to come back later", "the
second time sounds copy-pasted" are all first-class input. Translating vague but
real perceptions into checkable musical hypotheses is part of the agent's value.
Terminology is an explanatory tool, not an entry ticket.

**Teach through consequence, not prerequisite.** No harmony/counterpoint/
orchestration course is required before starting. The best learning is "you
preferred B because the chord was inverted and the bass stopped landing on the
root." Concepts arrive *after* the human has already heard the difference.
Learning is a byproduct of creating, not an exam that precedes it.

**Keep the agent replaceable; keep the project persistent.** Claude, Codex and
whatever comes next are collaborators. The work, its version history, the
meaning of its changes, and the habits the human develops must not depend on any
one model. What persists is the piece and the person's taste — not an AI vendor.

**Progressive disclosure over feature abundance.** The risk is not too few
features; it is dragging someone into DAW-grade complexity immediately. Someone
who can only say "I don't like these four bars" must be able to start, and
tracks, velocity, voice leading and CC should surface only as they become
needed. Deep capability, thin default surface.

**The tool should increase agency over time, not dependency.** The goal is not
"I can't write without the agent" but a human moving from "this doesn't sound
good" to "I suspect the cadence is too complete" or "is the bass too
root-heavy". If after six months the human's musical understanding has not
grown and only their prompting has, the tool has half failed.

## Naming rules

These are standing rules, applied every time a term is introduced — not
historical decisions. They are repeated in `AGENTS.md` because agents will be
the ones tempted to break them.

**Music-domain meaning wins; development-domain meaning yields.** When a word
means one thing in music and another in software, the musical meaning has
priority and the software-side concept is renamed. This bites on *collisions*,
not on *etymology*: `diff`, `info` and `--json` have no musical meaning and are
kept.

Two collisions already resolved by this rule:

- **patch.** In synthesiser usage a patch is a sound preset. Our edit format is
  therefore an *Edit Set* (`edits.json`, `mid apply`), and `patch` is left to the
  Rig side, where V1 per-channel sound assignment can legitimately use it.
- **score.** A score implies notation — key signatures, enharmonic spelling,
  articulation, expression marks. A MIDI file carries none of these, so calling
  one a Score claims musical semantics it does not have. The work is a *Piece*;
  a single file is a *Take*.

**Italian wins where Italian is genuinely the lingua franca of notation** —
tempo, dynamics, articulation, structural markings. It does not win merely by
being Italian: `pezzo` and `ripresa` appear on no score and would be
play-acting, which is the same failure as `score` above. Outside that scope the
order is British English > Chinese > American English.

Note that `battuta` — Italian for *bar* — is already taken as the crate name, so
the musical unit is `bar`. This is deliberate; do not "restore" it to Italian.

**Every glossary term pins a Chinese equivalent.** Not decoration: the human and
the agent converse in Chinese, and without a pinned term the agent cannot tell
which side of the Piece/Rig boundary a word like 音色 refers to.

## V0.1

Five commands:

```
mid info    song.mid
mid inspect song.mid --bars 5:8 [--json]
mid apply   song.mid edits.json -o take-03.mid
mid diff    before.mid after.mid
mid play    song.mid --bars 5:8
```

Acceptance criterion, deliberately brutal:

> Before the end of the first weekend: the human says one sentence in natural
> language → the agent inspects the MIDI → a few notes change → `mid diff` →
> `mid play` → the human hears the result.

Nothing may be made elegant before that loop closes. The failure mode to fear is
learning Rust beautifully, writing an immaculate MIDI parser, and never having
changed sixteen bars of music together with an agent.

## Distribution

`battuta` is published to crates.io under MIT, the repository is public, and
`cargo install battuta` is the supported way to get `mid`. Publishing is a
one-way door: a version can be yanked but never withdrawn, and the name is held
for good. It is recorded here rather than assumed because nothing about the code
requires it.

**Holding the documents back was considered and rejected.** The case for it was
that this file, the glossary and the decision records took more thought than the
code did. The case against is evidence. A third of `src/` is comment, and
thirty-three of those comments name an ADR or this file outright, several of them
restating the option that was rejected and why — so withholding the documents
would not have withheld the reasoning. It would only have left thirty-three
references pointing at nothing, which advertises an omission without achieving
it.

The deciding argument is what the scarce thing actually is. Twenty-one recorded
rejections across ten ADRs, and two ADRs amended in place rather than superseded,
are a record nobody can reconstruct after the fact — including whoever wrote it.
That kind of record is worth nothing held and something read.

**Not shipped is not the same as not public.** `AGENTS.md` and `docs/agents/` are
excluded from the crate because they describe how this repository is worked in
rather than what the crate is, and a package should be honest about its own
scope. They stay readable in the repository. `exclude` is not a privacy
mechanism and must never be used as one — anything that would actually harm
someone by being public has to be removed or rewritten, not merely left out of
the tarball.

Prebuilt binaries, a Homebrew tap and Rigs referable by name are all downstream
of this, and none of them is V0.

## Non-goals

- **A composition DSL, or any musical semantics in the edit format.** Edits are
  mechanical: add / delete / move / resize a note, change velocity, change CC.
  Never `make_sadder`, `reharmonize`, `increase_tension`, `olivia_style`.
  Semantics belong to the agent; execution belongs to the core.
- **Owning version history.** `mid apply` never writes in place; it always emits
  a new Take. Naming and keeping Takes is the filesystem's and git's job. A tool
  that manages Take history has started becoming a DAW.
- **More than one degree of freedom in the Rig, for V0.** One soundfont for the
  whole Piece. Per-channel sound assignment is a property of the Rig and belongs
  to V1's named Rigs (`--rig concert-grand`), not to V0.
- **Rendering to audio (`mid render`).** V1. It also raises a real question this
  charter does not yet answer: a `.wav` outlives the terminal, so its Rig
  attribution has to be persisted with it.
- **Swift, CoreMIDI, AVAudioEngine, any GUI.** Deferred until there is a real
  demand for a native Mac product. `battuta` is a library from day one precisely
  so that day does not require a rewrite.
- **Music in this repository.** OliviaMIDI tracks the tool. The actual musical
  work lives elsewhere; `.asset` is a gitignored, machine-local pointer to it.

## Honest byproduct

Chinese musical terminology is currently unstable — partly Japanese-derived
compounds, partly whatever each DAW's localisers chose, so that 音色 / 音源 /
音色库 routinely denote the same thing. Pinning Chinese equivalents in
`CONTEXT.md` is done because the interaction requires it, and it may incidentally
produce a small, coherent Chinese vocabulary for this corner of the domain.

That is a byproduct, stated honestly. It is not a product goal, it does not
justify any feature, and it must not be used to argue for scope.
