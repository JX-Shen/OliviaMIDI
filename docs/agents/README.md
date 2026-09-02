# Agent skills documentation

These files configure how the engineering skills from
[`mattpocock/skills`](https://github.com/mattpocock/skills) operate in this
repository. Most of what is here came from that repository and is deliberately
kept close to its original form: adapting prose that already says the right
thing would only create a divergence to maintain.

| file | origin |
| --- | --- |
| `domain.md` | upstream, unchanged — its examples are generic and not about MIDI |
| `triage-labels.md` | upstream, with the label column filled in for this tracker |
| `issue-tracker.md` | this project's own, apart from the `gh` conventions |

The disciplines these skills encode — grilling a design before building it,
resolving domain terms before naming anything, recording decisions as ADRs and
amending them in place — are why `CHARTER.md`, `CONTEXT.md` and `docs/adr/`
exist at all. The credit for the method belongs upstream. What is this project's
own is the follow-through.

None of this directory ships in the `battuta` crate. It describes how the
repository is worked in, not what the crate is — see *Distribution* in
`CHARTER.md`.
