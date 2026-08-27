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

## How agents drive this tool

`mid` is meant to be driven by agents, and it is self-describing on purpose:

- `mid help` and `mid <command> --help` are the CLI contract. There is no
  separate command reference to keep in sync with the binary.
- `--json` on `info`, `inspect` and `diff` gives structured output. Prefer it
  over parsing human output.
- Edits are mechanical only — add, delete, move, transpose, resize a note,
  change a velocity, change a CC. Musical intent is the agent's job to hold and the
  core's job to never encode. Do not add an Edit like `make_sadder`.
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
engineering skills these docs are written for. Enabling it only takes effect
where the marketplace is registered: clone `git@github-personal:JX-Shen/skills`
(a fork of `mattpocock/skills`) and add it to `extraKnownMarketplaces` in the
machine's `~/.claude/settings.json` as a `directory` source.

### Triage labels

The five canonical triage roles, each label string equal to its name. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` at the root plus `docs/adr/`. See `docs/agents/domain.md`.
