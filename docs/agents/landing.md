# Landing branches on a linear main

`main` has no merge commits, and it is kept that way. A branch lands by being
rebased onto `main` and fast-forwarded; nothing lands any other way. This file
is the procedure, and it applies to one branch as much as to eight.

Promoted on 2026-10-01 from a working note drafted 2026-09-08, after the run that
landed #39, #36, #41, #33 and #34 in one sitting.

## The rule

```sh
git checkout "$b" && git rebase main
git checkout main && git merge --ff-only "$b"
git push origin main
```

`--ff-only` is the guard: it fails rather than quietly writing a merge commit.

**GitHub's merge buttons are not used — not *Merge*, not *Rebase and merge*, not
*Squash and merge*.** Each one writes commits on GitHub that no local tree
produced: a merge commit, or the same changes re-committed under a new SHA and
without their signatures. When a branch with an open pull request is
fast-forwarded and `main` is pushed, GitHub sees the head commit in `main` and
marks the pull request merged by itself. The pull request stays the record; the
landing happens here.

Merging is the human's, as *Git conventions* in `CLAUDE.md` says. An agent asked
to land a branch follows this file; it does not infer the method from history.
#57 is what inference produced: the earlier pull requests carried different SHAs
from `main` because they were rebased locally before landing, and that was read
as GitHub's rebase button.

## Landing several: the shape of the concurrency

The useful distinction is not "parallel work" versus "serial work". It is:

- **Probing fans out.** Every question about *whether* branches conflict is
  read-only, order-independent, and can be asked all at once. `git merge-tree
  --write-tree` answers it without touching the working tree or the index.
- **Landing serialises, always.** The moment one branch lands, `main` moves, and
  every remaining answer is stale. There is no version of this that batches.

So the work is one wide read, then a narrow ordered write. Landing one branch and
then probing, repeatedly, is what makes a stack of branches feel like a week.

## 1 — Survey, all at once

For each unmerged branch, three facts:

```sh
git merge-base main "$b"          # is it cut from main, or from another branch?
git rev-list --count main.."$b"   # how many commits of its own?
git diff --name-only main..."$b"  # which files?
```

The merge-base is the one people skip and the one that matters. Branches here
are often cut from other branches; on 2026-09-08 eight branches were really two
stacks and three independents, and no plan made without knowing that would have
survived.

```sh
git merge-base --is-ancestor "$other" "$b"   # is $b built on $other?
```

## 2 — Probe every pair, still all at once

```sh
git merge-tree --write-tree --name-only main "$b"   # exit 0 = clean
```

Requires git ≥ 2.38. It writes a tree to the object database and nothing else —
no checkout, no index, no stash — so it is safe against a dirty tree and safe to
run on every pair in a loop.

Probe each stack tip against `main`, then each stack tip against the others.
Every branch clean against `main` alone is the normal result and is not the
answer: **the conflict is created by the landing order, not carried by the
branch.**

## 3 — Read the conflict before ordering around it

Not all conflicts cost the same, and the classification is mechanical:

**Two additions at one assembly point.** Both sides append a new variant, field
or arm where every new case of its kind must pass. Neither modifies what the
other wrote; the resolution is "keep both", in either order, with no judgement
about the music. Any two branches that each add a difference kind collide at
`Diff` in `src/diff.rs`, its `is_empty`, its constructor and its renderer,
because ADR-0007 makes `Diff` the single report. That is the fixed cost of a
right decision, paid with ordering rather than refactoring. Do not open an issue
to make it go away.

**Two shapes of the same refactor.** Both sides restructure a shared helper the
same way, independently — for instance each adding a channel-taking variant of a
helper in `tests/common/mod.rs` with the old name delegating to it. Also "keep
both", also no judgement.

**A real disagreement about behaviour.** Two sides changing the same lines to say
different things. This is not a rebase problem and must not be resolved during
one. Stop: `CLAUDE.md` → *When a decision is not the agent's*.

The first two are the common case. Assuming the third and escalating early costs
a morning; assuming the first two and resolving a real disagreement inside a
rebase puts a decision the human owns into the tree under a green tick.

## 4 — Order

1. **Independents first**, in any order. Zero-overlap branches land free and
   shrink the problem.
2. **Then the stack that waits on nobody.**
3. **Last, the stack whose behaviour a human still has to decide.** Landing it
   first means rebasing it again once the decision arrives — paying the same
   conflict twice — and putting a holding position into `main`, where the next
   reader finds it as though it were an answer.

**Order by what a branch is waiting for, not by how big it is.** Size is a
rebase; a pending decision is a second rebase plus a wrong fact in `main`.

The file lists from step 1 are the input here, and reading past an overlap in a
list you generated yourself is easy. Diff the lists mechanically, not by eye.

## 5 — Land, one at a time

Use the rule above. Re-run step 2 after each landing if anything downstream
matters: `main` has moved and the old answers are void.

For a stacked branch, rebase the tip onto the new `main` after its base has
landed. Git drops the base's commits by patch-id and prints
`hint: use --reapply-cherry-picks to include skipped commits`. **That hint is
expected and means the deduplication worked.** Confirm it by counting commits,
not by reading the hint: `git log --oneline <before>..main`.

A branch whose base had moved lands with its subjects intact and new SHAs, so its
old tip is not an ancestor of `main` and `git merge-base --is-ancestor "$b" main`
says no for a branch that fully landed. Do not read that as missing work: compare
subjects, or `git cherry main "$b"`. Deleting the stale branch is on the stop
list in `CLAUDE.md`. Leave it.

## 6 — Gate, and do not trust the local one

Run the `ci.yml` jobs locally as a **pre-filter**:

```sh
export PATH="$HOME/.cargo/bin:$PATH"   # cargo is not on an agent shell's PATH here
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo package --locked
cargo +1.85 check --locked && cargo +1.85 test --locked
cargo install --path . --root "$(mktemp -d)/install" --locked   # then --version, then apply --help
```

`rustup` may not be on PATH, but `cargo +<rust-version>` reaches an installed
toolchain, so the MSRV job is runnable locally. Take the version from the
manifest's `rust-version`, not from this file.

**cargo's fingerprint goes stale in this repository and lies in both
directions.** It has reported `Finished` against a seven-hour-old binary that
`touch` did not invalidate; it has reported a field missing that the source
plainly declared (a false red); and in the same breath it has passed clippy
without compiling the bin's test target at all (a false green). `cargo clean -p
battuta` and a rebuild cleared each.

- **A red that contradicts the source is a cache result until proven otherwise.**
  Read the file. If the source is coherent, `cargo clean -p battuta` before
  changing a line. It does not count as one of the two repairs in *When to stop
  repairing*; nothing was repaired.
- **A local green is a pre-filter and never the gate.** A false red argues with
  you; a false green does not. `ci.yml` checks out into a fresh environment with
  no cache of ours, which is why `main` is pushed and watched rather than
  declared good locally.

## 7 — Push, then stop

Push `main` and watch the run. Then stop: closing the issues these commits
answer, tagging, publishing and deleting branches are all on the stop list, and
the release checklist (#22) is read by a human on purpose.

## If this becomes an executable workflow

Steps 1 and 2 are pure reads — safe to run wide and concurrently. Steps 3 and 4
are judgement. Steps 5 to 7 are strictly serial and each moves shared state. An
orchestrator should parallelise only the probe, hand the classification to
something that can read a conflict rather than count one, and never hold more
than one landing in flight. A workflow that fans out the landing produces the
merge commits this history does not have.
