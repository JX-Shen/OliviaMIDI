# An absolute Tick is a u32, and a longer Take is refused

Every Tick `battuta` holds is a `u32`: a Note's start and duration, `Info`'s
length, the span a Bar range resolves to, every number the `--json` payloads
carry. A Take whose events accumulate past that is refused at `Take::read` and
at `Take::from_smf`, with `Error::TakeTooLong`.

This needed deciding because the format does not agree with the model. A delta
time is a `u28`, and SMF puts no limit on how many of them a track may hold, so
a file can be built entirely out of individually writable gaps whose running
total leaves `u32` behind. Such a file is well formed. It is not obscure to
construct: seventeen maximal deltas do it.

Before this decision the total was accumulated unchecked in five places, and the
same file gave two answers. A debug build panicked in `Take::info`. A release
build wrapped and reported the wrapped Tick as the Take's length — exit zero, a
plausible number, nothing to say it was false. That second one is the failure
this project is shaped to refuse: a wrong answer with nothing to reveal it, in
the same class as ADR-0006's refusal to assume 4/4.

## Considered Options

**Widening every absolute Tick to `u64`.** The stronger option in the abstract,
and it keeps every well-formed SMF readable. Rejected on what it costs and who
it costs it for. The width is not an implementation detail — it is in `Note`, in
`Info`, and so in the `--json` every agent driving this tool parses. Past 2^53 a
JSON number stops being exact in any consumer with a JavaScript-shaped reader,
so `u64` does not remove the boundary so much as move it somewhere no error
message is watching. And what it buys is the ability to describe a Take some
nine million quarter notes long. That is not a Piece. Paying a compatibility
cost on every Take anybody actually has, to accommodate a file no instrument
produced, is the wrong direction.

**Keeping `u32` and clamping.** Rejected outright: it is the release build's
current behaviour with the panic removed, which is to say the bug.

**Keeping `u32` and refusing.** Chosen. Both options that survive contact with
the problem need checked accumulation anyway — you cannot know a total has
exceeded the range without adding it up safely — so the real choice was only
what to do at the boundary, and refusing is what the rest of the crate already
does when a file is outside what it can describe truthfully.

## Consequences

The check is at the two places a Take comes into existence, not at the five that
accumulate a Tick. That is what licenses the plain `+=` in `info`,
`time_signatures`, `notes`, `Rewrite::of` and `passage`: they run on a Take that
has already been shown to fit, so their sums fit. The invariant is stated on
`Take` itself, because it is the reason those five are allowed to look careless.

`from_smf` pays for the invariant by parsing back what it just wrote. No Edit
can reach past the range today — each lands its Ticks through a `u32`
conversion, and a passage only ever moves events earlier — so the check is
currently provable by argument rather than needed. It is kept anyway: an
argument is what a later Edit breaks without noticing, and everything unchecked
downstream depends on this one being true.

The refusal is not in `--help`. A limit no Take anyone has will reach does not
belong in the primary product surface, where every line is read by everybody;
this ADR and the error message are where it is recorded. The error says the
likely cause, which is a corrupt delta time rather than very long music.

Refusing at `read` means one door for all five commands. A Take past the range
cannot be described, inspected, edited, diffed or played, and each of them says
so in the same words.
