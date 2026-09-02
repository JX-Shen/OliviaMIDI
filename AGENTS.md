## Working Philosophy: Reduce entropy

Optimize for long-term coherence with minimal machinery.

Work from evidence, not assumption.
Keep state aligned with reality.
Make decisions explicit, not ceremonial.
Prefer compactness over complexity: fewer concepts, fewer states, fewer moving parts, and fewer open loops.
Prefer extra information over stacking context.

Do not introduce unnecessary entropy.
Flag issues, but respect existing coherence.
Before changing assumptions, rules, or structure, revisit the project charter, think twice, and record the rationale.

If there is a contradiction, VERIFY WITH HUMAN.

Align.
Reproduce.
Slice.
Verify.
Record only what must survive.

## Project

OliviaMIDI is the product; `battuta` is the Rust core; `mid` is the binary it
ships. Read `CHARTER.md` before proposing anything, and `CONTEXT.md` before
naming anything.

The one boundary that is mechanically decidable: **what is inside a Take belongs
to the Piece, what is required to turn a Take into air belongs to the Rig.** The
test is literal — is it in the MIDI file? Program changes are; the soundfont is
not. A diff never reports Rig differences.

## Naming rules

Standing rules, applied every time a term is introduced. They are here, not only
in `CHARTER.md`, because agents are the ones most likely to break them.

- **Music-domain meaning wins; development-domain meaning yields.** Where a word
  means one thing in music and another in software, the musical meaning has
  priority and the software concept gets renamed. This applies to *collisions*,
  not to *etymology* — `diff`, `info` and `--json` have no musical meaning and
  are kept. Do not introduce `commit`, `branch`, `merge`, `patch` or `revision`
  as names for anything on the Piece side.
- **Italian wins where Italian is genuinely the lingua franca of notation** —
  tempo, dynamics, articulation, structural markings. It does not win merely by
  being Italian. Outside that scope: British English > Chinese > American
  English.
- **Every glossary term pins a Chinese equivalent**, with the same
  pick-one-and-list-the-rest discipline as the English term.
- Before adding a term to `CONTEXT.md`, check it against these three. Before
  reusing an existing term differently, stop — that is a contradiction, and the
  Working Philosophy says to verify with a human.

## Where a decision goes

Three homes, and the boundary is not about importance:

- **`CHARTER.md` holds postures, scope and obligations** — what is true of the
  project whatever the code happens to do. It cannot be proved false by reading
  code. *Human owns the taste* is a charter clause.
- **An ADR holds a principle the code is built on** — a rule that reading the
  code can prove false, and that code not yet written will have to obey. *The
  library decides the fact, the consumer decides the wording* is an ADR: open
  `rig::play`'s signature and you can check it, and the next fact the library
  learns to state is bound by it.
- **A judgement about one behaviour is closed in its issue.** Which events a
  passage carries, that a pitch is spelled with sharps, that a Tick is a `u32`:
  each was decided once, about one input, and each lives in the issue that
  asked for it. The issue's last comment is headed **Decision** and states the
  behaviour and the options rejected; the issue is labelled `decision` and
  locked; the code cites it as `#N`. Nothing is copied out of it anywhere else.

The charter derives the ADRs, and the ADRs are what each judgement argues from:
constitution, statute, case law. A judgement never becomes an ADR by being
important, and an ADR never becomes a charter clause by being general.

**Two tests decide whether a decision is an ADR, and it has to pass both.**
First: strip every command name, flag, type and event kind out of the sentence —
does it still say something? "A Tick is a `u32`" is empty without the type, so
it is a judgement. Second: is there code this project has not written yet that
would have to obey it? A principle constrains the future; a judgement settles
one case. A principle usually arrives when a *second* judgement is found arguing
from the first — that is the moment to write the ADR and have both cite it — or
when a decision on day one constrains a consumer or a feature that does not
exist. Do not open an ADR for a decision that has one application and
constrains nothing else, however hard it was to make: its issue is its record.

**A judgement's reasoning is written once, in the Decision comment.** Not in a
doc comment, not in the ADR it argues from, not in the README. A code comment
says what the code does and cites the issue for why; a sentence of reasoning
copied into code is a sentence that will be wrong one day with nothing to
say so. The Decision comment is dated and locked, so it cannot drift: it says
what was true when it was decided, and a later issue that changes the behaviour
carries its own Decision and cites the one it overturns.

**An ADR is amended in place.** Do not open a new ADR saying "supersedes
ADR-000N". Correct the one that is wrong and record inside it what changed and
why. A new number is for a subject that has genuinely split, never for a second
opinion about an old one. Every Decision comment names the ADR or charter clause
it applies, so `gh issue list --label decision --search ADR-0002` is the list
of a principle's applications; the ADR does not keep that list itself.

`docs/adr/README.md` indexes the principles, and maps the numbers this
directory used before 0.1.0 to where each one's content went. Add a line when
you add an ADR; it is the only place that answers "what has this project
already settled?" without opening six files.

## How agents drive this tool

`mid` is meant to be driven by agents, and it is self-describing on purpose:

- `mid help` and `mid <command> --help` are the CLI contract. There is no
  separate command reference to keep in sync with the binary.
- `--json` on `info`, `inspect` and `diff` gives structured output. Prefer it
  over parsing human output.
- Edits are mechanical only, and six kinds exist: move, transpose, resize, add
  and delete a note, and change a velocity. Musical intent is the agent's job to
  hold and the core's job to never encode. Do not add an Edit like `make_sadder`.
- **Changing a CC is a legitimate Edit and is not one of the six.** So is
  changing a program. Both are in the file and therefore the Piece, both are
  carried faithfully by `apply` and `play`, and neither is reported by `inspect`
  or `diff` or reachable by any Edit — see #11. An Edit Set naming one will fail
  to parse; do not write one, and do not work around the gap by editing notes
  instead, which is the substitution #11 exists to prevent.
- `mid apply` never writes in place. Always produce a new Take.
- `mid play` states which Rig it used, on stderr and in `--json`. If a Rig is
  not configured it fails rather than guessing; do not work around this by
  picking a soundfont.

## Git conventions

- **No `Co-Authored-By` trailer, for agents or tools.** A commit is authored by
  the human who decided it. Agents are collaborators, not co-authors — the same
  reasoning as *Keep the agent replaceable; keep the project persistent* in
  `CHARTER.md`. Do not add the trailer even when a tool's defaults suggest it.
- Subject line in the imperative, under ~72 characters. Body explains why the
  change was made, not what the diff already shows.

## Agent skills

### Issue tracker

Issues live as GitHub issues in this repo, driven via the `gh` CLI, under the
`JX-Shen` personal account. Account selection is automatic via `GH_CONFIG_DIR`,
but agent shells do not inherit it — verify `gh auth status --active` before any
`gh` write, and stop if it is not `JX-Shen`. See `docs/agents/issue-tracker.md`.

### Plugin

`.claude/settings.json` enables the `JFork@JFork` plugin, which supplies the
engineering skills these docs are written for. They come from
[`mattpocock/skills`](https://github.com/mattpocock/skills); this machine runs a
personal fork of it. Enabling the plugin only takes effect where the marketplace
is registered: clone either, and add it to `extraKnownMarketplaces` in the
machine's `~/.claude/settings.json` as a `directory` source.

### Triage labels

The five canonical triage roles, each label string equal to its name. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` at the root plus `docs/adr/`. See `docs/agents/domain.md`,
and *Where a decision goes* above for the gate an ADR has to pass here, which is
stricter than the skill's own.
