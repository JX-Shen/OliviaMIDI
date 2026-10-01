# Writing and keeping tests

A test here is evidence that a behaviour survives change. This file names the
few rules that decide whether a test is that evidence, and each one cites the
case that made it necessary. Distilled on 2026-10-01 from the test rules of the
owner's AgentOS project, keeping only what this repository had already met;
nothing here depends on that project.

## Through the boundary the behaviour is observable at

`mid`'s behaviour is observable at the process boundary, so that is where its
tests are: `tests/`, driving the built binary. The public `battuta` API is also
a boundary, and a test may drive it where the library states a fact `mid` only
words.

A test inside `src/` is the exception, and it says why it cannot be written at
the boundary. `src/track.rs` has the two this repository holds: one property
that only shows in a Take too strange to commit, and one check that fires only
when the code is already wrong, so watching it at the boundary would mean
shipping a broken `mid`. Its module comment is the pattern.

## The expected result does not come from battuta

The oracle is read from the files and the Edit Sets with `midly`, or written as
a literal over a small Take built for the test. It is never computed by asking
`battuta`, and never by reproducing its algorithm: a test that agrees with the
code by construction passes while both are wrong.

Another command is not an oracle either. `tests/combine.rs` says so in its
header: `mid diff` reports nothing at a reordered site, so a check built on it
would pass a combination that got the order wrong.

## A test asserts the behaviour it is named for

Every assertion in a test is a claim about the command the test is about. A
precondition stated about another command binds the test to that command's
behaviour: when that command changes legitimately, the test fails with nothing
wrong in the one it tests.

The reordered-strikes test in `tests/combine.rs` first asserted that `mid diff`
saw no difference between the two sides. That was true and is why the case
matters, but it was a claim about `diff`, which can change while `combine` does
not. The reason the case matters went into #51; the assertion came out.

## A test of behaviour that already exists shows that it can fail

A test written after its behaviour was green was never red, so nothing yet
shows it would notice the behaviour going away. Show it: break the behaviour in
the real code, watch the test go red, restore the code. Record the break and
the result in the commit message or the issue, so the claim can be checked.

The break can be wrong too. In #51 the first attempt to make side verification
blind to order within a Tick compared notes instead of events; the reordered
test stayed green, because note identities are themselves order-sensitive. Only
a comparison that genuinely ignored order within a Tick removed the behaviour,
and the test went red under it. A break that leaves the test green proves
nothing until it is shown to have removed what the test is about.

`src/track.rs` set the precedent: each defect its check is shown catching was
also reintroduced in the real code and caught there.

## Removing a test names its successor

A test is removed only by naming the test that still fails for the same defect,
or the change that retired the behaviour it protected. Shared setup or a
similar name is not that.

## What is not here

Lists and counts are read from their owner, never written beside it — that is
*Where a fact lives* in `CLAUDE.md`, and `tests/contract.rs` is its form.
Nothing here asks for a test lint, a coverage threshold, a plan table or a
record of every test kept: a rule that would not change what is written,
caught or rejected is not one this file carries.
