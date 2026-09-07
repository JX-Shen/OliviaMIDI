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

## Where a fact lives

A decision has three homes and the boundary is about kind. A *fact* — the
minimum Rust version, how many Edit kinds there are, which version is published
— has one home, and the question is only ever which. What follows is an
ordering, best first. Going one step down needs a reason, and the reason has to
be that the step above is genuinely unavailable.

1. **Delete the copy.** Nothing left to drift. #17 did this for the Edit kinds
   — not by adding a check that the README's count was right, but by removing
   the count, because `mid apply --help` was already the exhaustive list and a
   second one could only ever agree or lie. Ask first whether the copy is
   telling the reader anything its owner would not.
2. **Derive it.** The copy is computed from its owner and cannot disagree.
   `mid --version` comes from the manifest. `tests/contract.rs` reads the list
   of Edit kinds out of the type, never from a list written down beside it.
3. **Gate it.** Only where a fact must genuinely live in several places, all of
   them necessary. A tag, a manifest and an installed binary are three such
   places, and the fresh-install job in `ci.yml` is what compares them.
4. **Remember it.** This is what 0.1.1 did, and #16 is the record of how that
   went. It is not a reason; it is the absence of one.

**A gate cannot fix a second writer.** It can only report the disagreement once
both copies exist, which is a worse place to stand than never having made the
copy. `README.md` naming a Rust version was not the MSRV gate's failure: the
manifest was corrected and the README was a writer nobody had counted.

**And a gate can only hold what has been named.** `stay_placed` is not a test
somebody thought to write — it is the mechanical form of ADR-0008, and it could
not exist before the rule did. Every one of the five Rank defects was found by
rendering audio and comparing it, because there was nothing to assert against.
The same is true of prose: `tests/contract.rs` is possible only because #17
settled which surface owns the list. Two surfaces both passing their own check
is a repository that lies with a green pipeline.

## When a decision is not the agent's

The Working Philosophy says to verify with a human on a contradiction. That is
the whole of it, and it was written for an agent working while somebody was
awake. Unattended — overnight, or under `/implement` on a set of tickets — it
needs a boundary that can be applied without first asking what the word means.

**Stop, and leave it for the human:**

- a contradiction between `CHARTER.md`, an ADR, a `decision` issue and the code,
  including one that only appears once the work is under way;
- a change that would widen the scope the issue asked for, or touch a file the
  issue did not name;
- anything irreversible or visible outside this repository: a tag, a publish, a
  force-push, a closed issue, a deleted branch;
- a trade-off between two defensible behaviours that the charter and the ADRs do
  not decide between. Choosing one and noting the choice is still choosing.

**Do not stop for:**

- a read-only probe, however wide;
- any job in `ci.yml`, or any `cargo` command that does not write to the tree;
- an unambiguous defect inside the slice the issue already named;
- a question the issue's acceptance criteria already answer.

**Uncertainty rounds up.** Where it is unclear whether something crosses the
boundary, it crosses. A wrong stop costs one morning's reading; a wrong
continuation puts a decision the human owns into the tree with a green tick on
top of it. The asymmetry decides this, not how confident the agent feels.

**Stopping means stopping somewhere it will be seen.** Comment on the issue with
what was found and what is left, leave the work uncommitted rather than
committing half a decision, and do not open a second issue to carry on through.
An agent that stops and then works around the thing it stopped at has not
stopped.

## When to stop repairing

**Two failed repairs of one root cause end the repair.** After the second, stop
changing code and re-examine the seam, the assumption, the contract, or the way
the work was sliced. A third attempt at the same level is rarely the one that
works and reliably the one that leaves a workaround behind.

**Name what changed before retrying anything.** Before a second attempt at a
failed action, state the new evidence or the changed precondition that makes
another attempt worth making. Running it again, rewording it, or handing it to a
different agent is none of those.

## How agents drive this tool

`mid` is meant to be driven by agents, and it is self-describing on purpose:

- `mid help` and `mid <command> --help` are the CLI contract. There is no
  separate command reference to keep in sync with the binary.
- **`mid apply --help` is the Edit Set contract, and it is the only exhaustive
  list of the Edit kinds.** Run it before writing an `edits.json`. Do not infer
  the schema from this file, from `README.md`, from an issue body, or from
  memory: every one of those is a copy, and this repository has already shipped
  a release where the copies disagreed with the binary. No prose outside the
  help may enumerate the kinds or count them — this is *Where a fact lives*
  applied to the one fact this repository has already been burnt by, and a
  number is the cheapest fact to write down and the first one to go stale.
- `--json` on `info`, `inspect` and `diff` gives structured output. Prefer it
  over parsing human output.
- Edits are mechanical only. Musical intent is the agent's job to hold and the
  core's job to never encode. Do not add an Edit like `make_sadder`.
- **Notes, Programs and Controllers are all the Piece.** Each is in the file, so
  each is reported by `inspect`, compared by `diff`, and reachable by an Edit.
  Do not work around a gap by editing notes to stand in for expression or
  orchestration — that substitution is the one #11 exists to prevent.
- `mid apply` never writes in place. Always produce a new Take.
- `mid play` states which Rig it used, on stderr and in `--json`. If a Rig is
  not configured it fails rather than guessing; do not work around this by
  picking a soundfont.

## Releasing

0.1.1 was tagged and published from a commit whose `AGENTS.md` told agents not
to write the Edits that commit had just shipped, and nothing caught it because
nothing was looking. What follows is the mechanism that replaced remembering.
Do not release without it, and do not work around a red one.

**The gate is `.github/workflows/ci.yml`, and it is green or there is no
release.** Four jobs: everything decidable by reading the tree plus `cargo
package`, the tests on macOS, `check` and `test` under the manifest's own
`rust-version` with `--locked`, and a fresh install whose binary is asked for
`--version` and `apply --help`. On a `v*` tag the install job also refuses a tag
that disagrees with the binary.

**What each answers, so that a red one is read rather than retried:**

- `tests/contract.rs` — that `mid apply --help` lists exactly the Edit kinds the
  binary accepts, in both directions. The list of kinds is read out of the type,
  never written down beside it.
- `cargo package` — that the crate holds the files it needs. A file missing from
  the package is a fact about the commit, and finding it out while tagging is
  finding it out too late.
- the MSRV job — that `rust-version` is true. It was not, when this was written.
- the fresh install — that a `cargo install` presents what the repository says
  it does, which is the only place the tag, the manifest and the binary are
  compared against each other.

**`cargo publish` is not automated, and is not to be.** A green pipeline cannot
read a release narrative. #22 is the checklist, and it is run by a human.

**The test no CI can run, and it is worth more than the ones it can.** Start an
agent with no history of this project. Give it the repository and a Take
carrying a CC11 curve, and ask for `inspect` → a Controller Edit → `apply` →
`diff` → `play`. It passes when the agent does not say Controllers are
unsupported, does not read `src/` to guess the schema, is sent here and then to
`mid apply --help`, writes a legal Edit Set, can explain the diff, and the target
note is audibly governed by the Controller at its own Tick. Run it before a
release that changed anything an agent is told about.

## Git conventions

- **No `Co-Authored-By` trailer, for agents or tools.** A commit is authored by
  the human who decided it. Agents are collaborators, not co-authors — the same
  reasoning as *Keep the agent replaceable; keep the project persistent* in
  `CHARTER.md`. Do not add the trailer even when a tool's defaults suggest it.
- Subject line in the imperative, under ~72 characters. Body explains why the
  change was made, not what the diff already shows.
- **An agent does not commit to `main`.** Branch first, named for the issue the
  work came from, and leave the merge to the human. `ci.yml` runs on every
  branch, so this costs no signal: a branch gets the same four jobs. What it
  buys is that a disagreement lands somewhere other than the history everyone
  else reads.

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
