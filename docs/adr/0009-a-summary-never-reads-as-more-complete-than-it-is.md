# A summary never reads as more complete than it is

A summary may say only part of what is there. Everything it states is true, and
its form implies nothing it does not state: it never reads as more complete than
it is.

`CHARTER.md`'s *Refuse rather than answer plausibly* covers the case where the
tool cannot answer truthfully at all, and says to fail. This covers the ground
between refusing and saying everything. Most of what `mid` prints is a summary —
one span in place of forty events, two extremes in place of a curve, one line in
place of every comparison it made — and a summary is the ordinary, legitimate way
to be shorter than the file. What it may not do is let its shape say what its
numbers do not.

## The founding case

A Controller difference was a span with, on each side, the value at its start,
the value at its end and the highest value anywhere in it; where those agreed,
the side printed as one number. A Take holding CC11 at 80 that eases to 50 and
comes back to 70 inside such a span printed:

```
controller  bar 2 beat 2+240 until bar 3 beat 2  channel 2  CC11 (expression controller)  80 -> 70
```

Every number on that row is true. The row is still wrong, because one number on a
side is the form for a side that held one value, and this one did not: the reader
is told 70, the Take went down to 50, and nothing on the row says anything was
left out. That is #58, written down on 2026-10-16.

Neither the numbers nor the omission was the fault. A summary has to leave things
out, and the highest value is a true thing to report. What failed was the form,
and the layout did the lying. The row now reads:

```
controller  bar 2 beat 2+240 until bar 3 beat 2  channel 2  CC11 (expression controller)
  before    80
  after     70, down to 50 at bar 2 beat 3+240, ends at 70
```

The last clause is the same rule a second time. A side that has said `down to 50`
and then nothing reads as a side that stayed at 50, so a side that moves says where
it ends even when that is where it began; only a side that never moves stays a
bare number.

## Where it came from

The rule was named from #58, and it had been argued from without a name before
then. A tempo side carries two extremes so that a row asserting a difference
never prints two identical readings, and a bend side carries two so that a span
opening on one unit of difference does not read as one unit while containing a
dive (#32). Extremes across an interval the file leaves unranked are reported as
*incomplete* rather than as a number, and a diff that found no determinate
difference but could not compare everything says that some content was not
compared rather than *no differences* (#42). Each of those was a judgement about
one behaviour, and this is what they had in common. #58 was the case where a
judgement had been made the other way, and the abstraction is what showed it.

## Considered Options

**Keeping the highest alone, and documenting that a dip is not reported.** The
cheapest change and the second option #58 offered. Rejected because the
footnote does not reach the row. The one-number form still implies the side held
that value, and the correction lives somewhere a reader of the row is not looking.
Every reader would have to carry the footnote to every row they read, which is
remembering standing in for a mechanism.

**Reporting the whole shape in `diff`** — every value in force across the span,
or each rise and fall in turn. Complete, and rejected. `mid inspect --bars` over
the span already lists every statement on either Take, and `CHARTER.md`'s limit
on refusing holds for providing too: a surface does not duplicate what another
surface already provides. Each rise and fall would also be the segmenting ADR-0007
rejects.

**Saying only that the two sides differ, and pointing at `inspect`.** True, and
complete in what it claims. Rejected because `CHARTER.md` puts the legibility debt
on `diff`: a change that is mechanically large and musically small is `diff`'s to
explain, and a row saying *differs, see elsewhere* has handed the debt back.

## Consequences

**A summary's form is part of what it states.** One number where a side held one
value, three rows where it held more, *no differences* where everything was
compared — each layout is a claim, and is held to the standard the numbers in it
are. Where a shorter form would imply something the reading does not support, the
longer form is printed.

**Leaving something out is allowed; leaving no sign that something was left out
is not.** Every summary omits, which is what makes it one. What it owes is that a
reader can tell from the summary itself whether they are looking at all there is
or at what was chosen to be said. *Incomplete*, *not compared*, and a help text
saying where the rest is to be found are each that sign.

**Every summary not yet written answers to this from the start.** Each state that
joins ADR-0007's terms arrives with a summary across a span, and so does any
count or verdict `mid` comes to print in place of a list. None of them waits for
its own #58.
