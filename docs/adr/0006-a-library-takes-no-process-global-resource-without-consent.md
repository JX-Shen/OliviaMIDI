# A library takes no process-global resource without consent

`battuta` changes nothing that belongs to the process as a whole — a signal
disposition, an environment variable, the working directory, a global handler
of any kind — until the process has asked it to, by name. What it needs for its
own work it does through values it owns and returns.

The founding case is the signal handlers that remove a temporary passage when
`mid play` is interrupted. `battuta` installs none until the process has called
`battuta::remove_temporary_takes_on_signals()`. `mid` calls it once in `main`.
A consumer that does not call it keeps whatever handlers it had, and still gets
every temporary Take removed by `Drop` on every route out that runs a
destructor.

Installation is still lazy on top of that: consent alone changes nothing, and
the handlers arrive at the first temporary Take. A process that consents and
then never auditions anything has its signals left alone as well.

## The problem this fixes

A temporary passage must not outlive the command that wrote it, and a process
killed by a signal runs no destructor, so `mid` needs `SIGINT`, `SIGTERM` and
`SIGHUP` caught (#4). That is right for `mid`.

It was not right for anybody else (#9). Signal dispositions are a
*process-global* resource, and `battuta` was taking one as a side effect of an
ordinary-looking call. Exactly one prior disposition was preserved, `SIG_IGN`;
every other handler the process had was overwritten, never chained to, and never
restored. So a host application holding its own `SIGTERM` handler for its own
shutdown lost it — permanently — the first time it auditioned a single passage,
and afterwards died where it used to shut down cleanly. The re-raise restores
the default disposition, so the consumer's handler was not merely bypassed: the
process's behaviour on that signal changed from "clean up" to "die".

A library may not do that to its consumer without being asked. ADR-0001 exists
to make `battuta` usable by something that is not `mid`, and this was a trap
laid for precisely that consumer. The signal case is the one that has arisen;
the rule is stated for the resource class because the next one — a locale, a
logger, an allocator, a working directory — is the same trap with a different
name.

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

Opt-in usually trades safety for fragility — the one caller who needs the
behaviour can forget to ask for it, and the failure is silent. That trade is not
being made here, because the three tests in `tests/play.rs` that send `SIGINT`,
`SIGTERM` and `SIGHUP` to a live `mid` all go red the moment the call in `main`
is removed. It was checked, by removing it.

`tests/signals.rs` holds one test and has to keep holding one. Both the consent
flag and the signal dispositions are process-global and consent is one-way, so a
second test in that binary that consented would settle this one by winning a
race. The consenting side is tested in `play.rs`, where the consumer is `mid` in
a process of its own.

Paths are still registered for removal whether or not the process has consented.
It costs a leaked C string per outstanding passage, it means consent arriving
mid-flight covers the passages already open, and a registry nothing reads is
harmless.

The consent function is the one place in the public API that admits to a
side effect on the process, and its name says what it will do. A second
process-global need gets a second such function, not a flag on this one.
