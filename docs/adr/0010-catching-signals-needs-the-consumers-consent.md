# Catching signals needs the consumer's consent

`battuta` installs no signal handler until the process has called
`battuta::remove_temporary_takes_on_signals()`. `mid` calls it once in `main`.
A consumer that does not call it keeps whatever handlers it had.

Installation is still lazy on top of that: consent alone changes nothing, and
the handlers arrive at the first temporary Take. A process that consents and
then never auditions anything has its signals left alone as well.

## The problem this fixes

ADR-0007 needs `SIGINT`, `SIGTERM` and `SIGHUP` because a process killed by a
signal runs no destructor, and a temporary passage must not outlive the command
that wrote it. That is right for `mid`.

It was not right for anybody else. Signal dispositions are a *process-global*
resource, and `battuta` was taking one as a side effect of an ordinary-looking
call. Exactly one prior disposition was preserved, `SIG_IGN`; every other handler
the process had was overwritten, never chained to, and never restored, because
installation happened inside a `Once`. So a host application holding its own
`SIGTERM` handler for its own shutdown lost it — permanently — the first time it
auditioned a single passage, and afterwards died where it used to shut down
cleanly. `remove_and_die` re-raises with the default disposition restored, so the
consumer's handler was not merely bypassed: the process's behaviour on that
signal changed from "clean up" to "die".

A library may not do that to its consumer without being asked. ADR-0001 exists to
make `battuta` usable by something that is not `mid`, and this was a trap laid
for precisely that consumer.

## Considered Options

**Chaining to the handler that was already there.** Rejected on safety, not on
taste. It means storing a `sigaction` per signal and calling an arbitrary unknown
function from inside a signal handler. Whether that is even legal depends on what
the previous handler was, and `battuta` cannot know. It is a worse defect than
the one it fixes.

**Restoring on the way out.** Rejected because there is no way out. The handler
exists precisely for the path where no destructor runs, so there is no hook that
is guaranteed to fire — and with several auditions outstanding across threads,
"the last one out restores" is a race over a global.

**Opting out — the consumer says "leave my signals alone".** Rejected as the
weaker shape at the same price. `mid` would need no change, but a consumer that
does not know this behaviour exists is still bitten by it; an escape hatch only
helps whoever has already been hurt. Safe-by-default is worth more than
convenient-by-default here, because the failure is invisible until the day a
signal actually arrives.

**Documenting the cost and keeping the behaviour.** Rejected for the same reason:
it relies on the consumer reading a caveat before being harmed by it.

**Consenting — chosen.** The one shape where the consumer that has said nothing
is the consumer that is safe.

## Consequences

ADR-0007's promise is now conditional, and the condition is named there: the
temporary Take is removed by any route out, and by a signal in a process that has
agreed to have its signals caught. `mid` agrees, so nothing about `mid` changes.

Opt-in usually trades safety for fragility — the one caller who needs the
behaviour can forget to ask for it, and the failure is silent. That trade is not
being made here, because the three tests in `tests/play.rs` that send `SIGINT`,
`SIGTERM` and `SIGHUP` to a live `mid` all go red the moment the call in `main`
is removed. It was checked, by removing it. Those tests were written one commit
earlier for a different reason, and they are what makes this the safe direction
rather than the theoretically-nicer one.

The declining consumer still gets `Drop`: every route out that runs a destructor
removes the file, on success, on failure and on an unwinding panic. What it gives
up is the signal alone, for the seconds an audition is actually running.

`tests/signals.rs` holds one test and has to keep holding one. Both the consent
flag and the signal dispositions are process-global and consent is one-way, so a
second test in that binary that consented would settle this one by winning a
race. The consenting side is tested in `play.rs`, where the consumer is `mid` in
a process of its own.

Paths are still registered for removal whether or not the process has consented.
It costs a leaked C string per outstanding passage, it means consent arriving mid
flight covers the passages already open, and a registry nothing reads is
harmless.
