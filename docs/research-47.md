# #47 research: preserving requests and checking their interactions

Research record, 2026-09-10. Baseline:
`7b5a0a894f4364962a2f49533c64bea8a31a08cb`.
These are experimental observations and proposals for discussion, not adopted
Decisions, new public terminology, a release claim or musical acceptance.
The [parent question](https://github.com/JX-Shen/OliviaMIDI/issues/47)
remains open; the [Charter](../CHARTER.md) leaves musical judgement with the human.

## Where the evidence now points

The first useful product experiment remains a bounded combination of explicit
common-base requests. The existing core can execute it; the missing work is
coordinating what should be retained, checking the complete result and stating
the limits of the supplied evidence. The overnight results refine that proposal:

| Question | Executed finding | Consequence, not an adopted rule |
| --- | --- | --- |
| Does checking every pair suffice? | Three requests pass individually and in every pair, but all six full orders fail note pairing. | Validate the whole candidate with existing core constraints. |
| Does commutation establish independence? | Pedal and RPN cases commute exactly but change the context of another request. | Explain established interactions separately from applicability. |
| Can an inverse retain later work? | Independent later fields survive; restoring an old value can erase a later explicit value. | Investigate reconstruction from retained A and known later requests first. |
| Does exact replay certify history? | Keeping a note and deleting/recreating it make identical bytes in one fixture. | Replay verifies consistency of a supplied account, not its historical uniqueness. |
| Are external edits uniformly unusable? | Known track permutation and exact PPQ scaling carry a request correctly; symmetric tracks leave two fitting maps. | Separate declared correspondence from inference. |
| Does the pilot survive performance data? | A MAESTRO Take with 629 notes and 518 Controller events yields a verified candidate. | Preservation works in this case; pedal-context explanation remains missing. |

The earlier restricted prototype and two notation-export cases are recorded in
[#49](https://github.com/JX-Shen/OliviaMIDI/issues/49#issuecomment-5604739145)
and [#48](https://github.com/JX-Shen/OliviaMIDI/issues/48#issuecomment-5604740685).
This note records new research, rather than becoming another live capability
list. No public contract, CLI name, persistent format or release obligation
changes here. Experiments write disposable scripts and Takes; this note is the
only repository change. All musical preference and listening acceptance remain
open.

## State interactions: an equal result in both orders can still change context

Two executed examples rule out using disjoint edit addresses plus commutation
as a certificate of musical independence. A pedal change can govern a release
that a separate duration change carries across it. Changing an RPN selector can
redirect a Data Entry event edited independently. In both cases all four
construction paths below produced identical parsed event streams.

This is evidence about a candidate's interactions, not a reason already adopted
for refusing it. Existing `mid apply` executes the requested mechanics correctly.
The question for #47 is what a future combination should disclose, and what it
should leave for audition or an explicit choice.

### Evidence and method

The source was built as `mid 0.1.2`; the source identifier above distinguishes
it from an older installed binary. Each case uses a synthetic Type-0, one-track
file at PPQ 480, with a 4/4 statement and an end at Tick 400. No external musical
work is included. Mido 1.3.3 independently reads the written events. An in-memory
FluidSynth 2.6.0 instance receives only Controller events in the RPN tests: no
audio driver, SoundFont, note, rendering or playback is used.

For original A, left request L, right request R:

- B = apply(A, L); C = apply(A, R).
- D = apply(A, L followed by R); E = apply(A, R followed by L).
- BC = apply(B, R); CB = apply(C, L).

All 24 applications across three cases and one control succeeded. Each case
asserts D = E = BC = CB over parsed file type, PPQ, track boundaries and ordered
events with absolute Ticks. It also checks A's bytes remain unchanged. Equality
here is parsed-event equality, not byte equality or equal sound. Current
`inspect --json` outputs and A-to-D `diff --tolerance 0 --json` outputs were
saved. The final run added explicit outcome assertions and zero-tolerance
reports to the earlier exploratory execution.

The experiment follows the existing [event preservation rule](adr/0003-a-take-round-trips-at-the-event-level.md)
and [placement rule](adr/0008-events-sharing-one-instant-are-heard-in-the-order-they-are-written.md).
It does not extend the [Controller reading](../CONTEXT.md) into a new public
RPN interpreter; that glossary explicitly keeps the constituent Controllers
separate.

### Case 1: a duration change crosses a new pedal interval

A contains one note, pitch 60, channel 0, strike at 0, release at 100, velocity
80, release velocity 17. CC64 is 0 at Ticks 0, 150 and 300.
L sets CC64 at 150 to 127. R extends the note by 100 Ticks.

| Take | Written release Tick | Last CC64 before that release |
| --- | ---: | ---: |
| A | 100 | 0 |
| B (pedal only) | 100 | 0 |
| C (duration only) | 200 | 0 |
| D (both) | 200 | 127 |

The table is an executed Mido scan of file events. Neither side alone has a
release under CC64=127; the candidate does. The MIDI Association identifies
CC64's switch threshold at 64. [Official Controller table](https://midi.org/midi-1-0-control-change-messages)

FluidSynth's note-release handler checks whether the channel is sustained
before advancing the voice to its release stage; its CC64 handler damps
sustained voices when the switch is released. Thus this NoteOff would take
the sustain path, with pedal-release handling at 300, provided the voice is
still active. This is a source-supported inference, not a measured audio
duration: sample decay, envelopes and other Rig behaviour were not measured.
[FluidSynth 2.6.0 voice handler](https://github.com/FluidSynth/fluidsynth/blob/v2.6.0/src/synth/fluid_voice.c#L1230),
[Controller handler](https://github.com/FluidSynth/fluidsynth/blob/v2.6.0/src/synth/fluid_synth.c#L1790)

### Case 2: the same Tick is a necessary countercheck

Change only R to extend the note by 50 Ticks, ending at 150. All construction
paths still agree. The actual event order at 150 is the release followed by
CC64=127, so the scan still reads 0 immediately before the release. This is
consistent with the current help: a resized release precedes events already
at its destination Tick.

A blanket warning based on `release_tick >= pedal_start_tick` would conflate
this case with Case 1. It must inspect the written same-Tick order. This does
not establish a general sustain model: repeated notes, other pedals,
cross-track ordering and carried-in unusual event orders were excluded.

### Case 3: different Controller numbers can address one operation

A has CC101=0 at Tick 0, CC100=0 at Tick 1, and CC6=2 at Tick 10, all channel 0.
L changes CC100 at Tick 1 to 2. R changes CC6 at Tick 10 to 12.
The two edited addresses differ in Controller number and Tick.

The official table defines CC101/100 as the parameter selection and CC6 as
Data Entry for that selection. RPN 0 controls Bend sensitivity; RPN 2 controls
coarse tuning, centred at 64. Their interaction is specified rather than
inferred from musical taste. [MIDI Association RPN table](https://midi.org/midi-1-0-control-change-messages)

| Take | Selected RPN at Data Entry | CC6 data | FluidSynth Bend range | FluidSynth coarse tuning |
| --- | ---: | ---: | ---: | ---: |
| A | 0 | 2 | 2 semitones | 0 |
| B | 2 | 2 | 2 semitones | -62 semitones |
| C | 0 | 12 | 12 semitones | 0 |
| D | 2 | 12 | 2 semitones | -52 semitones |

The last two columns are actual API readings after feeding the file's Controller
events to a fresh FluidSynth instance. The default 2 in B/D is that instance's
unchanged Bend range; those files do not explicitly set a 2-semitone range.
The extreme tuning values make redirection obvious; this is a boundary fixture,
not an arrangement proposal. The code paths and generator identifier used by
the probe are version-pinned. [FluidSynth RPN handling](https://github.com/FluidSynth/fluidsynth/blob/v2.6.0/src/synth/fluid_synth.c#L1923),
[generator identifiers](https://github.com/FluidSynth/fluidsynth/blob/v2.6.0/include/fluidsynth/gen.h#L39)

D preserves both requested Controller values, yet the value 12 that controlled
Bend range in C now controls another parameter. No write collision or execution
order disagreement is needed for this consequence.

### Control: the Data Entry change on another channel

Extend A with channel 1's own CC101=0 at Tick 2, CC100=0 at Tick 3 and CC6=2 at
Tick 11. Keep L on channel 0, but apply R's CC6=12 at Tick 11 on channel 1.
The candidate retains B's channel-0 readings (range 2, coarse tuning -62)
and C's channel-1 readings (range 12, coarse tuning 0) exactly.

This successful control prevents the stronger unsupported conclusion that
any nearby RPN and Data Entry edits necessarily interact. Channel matters.
It does not prove independence of arbitrary channels in a full MIDI device:
global messages and other protocols were not exercised.

### What this supports, and what remains open

A future explanation can distinguish the concrete mechanical facts “the
release now follows pedal-on” and “Data Entry now meets RPN 2” from the human
decision to keep the result. Both can be researched without declaring a
conflict or introducing a full synthesiser state model into the core.

These examples support examining the **finished candidate**, with its earlier
channel state and event order, after constructing it. Address overlap and
pairwise commutation answer different questions. The report does not prescribe
mandatory warnings, refusal thresholds or protocol coverage; adopting those
is an owner decision under the [Charter](../CHARTER.md).

No playback was performed, no preference established, no production code
changed, and no existing core-contract contradiction was found in these cases.
The RPN API probe is an implementation control for one named synthesiser,
not proof of every receiver's response.

## Whole-candidate constraints: all pairs succeed, the triple fails

The minimal source is Type 0, PPQ 480, one track/channel, 4/4, ending at Tick
1000. Its two notes are p60 at [0,450], velocity 80, NoteOff velocity 10, and
p62 at [100,500], velocity 90, NoteOff velocity 20. All targets below name A.

| Request | Existing native Edit |
| --- | --- |
| X | Transpose `t0:c0:p60:s0:n0` by +2 semitones |
| Y | Move that same note by +150 Ticks |
| Z | Resize `t0:c0:p62:s100:n0` by +200 Ticks |

All three individual applications succeed. All six ordered pairs succeed;
each pair's two orders agree in complete parsed events. All six permutations
of the full triple fail with no output Take. This is 15 distinct applications,
not an extrapolation from one execution order. A sixteenth positive control
changes only Z to +50 and succeeds.

In the failing triple, the original second note now starts at 100 and ends at
700, while the original first note is now p62 at [150,600]. Their same-pitch
releases finish in the opposite order to their strikes. The positive control
ends the earlier strike at 550, restoring the valid order. Mido confirms the
control's strikes `(100,90), (150,80)` and releases `(550,20), (600,10)`, all
pitch 62. The current core correctly refuses the other candidate; this is not
a newly discovered core bug. [Input binding and final checks](https://github.com/JX-Shen/OliviaMIDI/blob/7b5a0a894f4364962a2f49533c64bea8a31a08cb/src/edit.rs#L372),
[note-pairing check](https://github.com/JX-Shen/OliviaMIDI/blob/7b5a0a894f4364962a2f49533c64bea8a31a08cb/src/edit.rs#L460)

The dependency is higher-order in the **requests**, even though the violated
result constraint concerns two notes. This does not require a higher-order
graph implementation. Constructing and checking the complete result already
finds it. The older pilot excludes onset/pitch changes, so this is evidence
against a broader pairwise-only design, not a failure inside that pilot's domain.

The generator and its results file were kept only in a temporary directory and
are not retained. The complete source and request recipe above reproduces the case using current
`apply --help`; all permutations use a single A-relative Edit Set, not successive
stale identities across files.

## Selective withdrawal: preserve the later request, not automatically the old value

Four synthetic cases were executed using current `mid` and independent Mido
event comparisons. The source is Type 0, PPQ 100, 4/4, ending at 500. Target
p60 is [0,100], velocity 80, NoteOff velocity 37; another note p64 is [200,300],
velocity 70, NoteOff velocity 23. Text follows the target strike at 0; text
precedes and follows its release at 100. SysEx at 50 and CC11=99 at 170 are
untouched controls. A→B is the experiment to withdraw; B→C is the later work.

| Case | A→B; then B→C | Actual comparison |
| --- | --- | --- |
| Independent fields | Target velocity 40; then duration +50 | Restore velocity 80 on C and replay duration +50 on A produce identical events, duration 150 and original release payload 37. |
| Later value on the same field | Target velocity 40; then velocity 60 | Restoring 80 on C legally erases the later 60. Replaying the later request on A produces exactly C: withdrawal can have no visible effect. |
| Later change of address | Target velocity 40; then move +20 | An inverse naming the old identity fails with no output. In this unique-destination case, naming start 20 succeeds and equals replaying the move on A. |
| Deletion | Delete p60; then set p64 velocity 55 | Re-adding p60 to C and replaying only the p64 request on A give identical inspected Notes and an empty zero-tolerance diff, but different events. |

In the deletion case the new release velocity is 0 instead of 37. The new
strike follows text that originally followed the strike; the new release
precedes text that originally preceded the release. Existing add semantics
correctly create and place a new pair; they do not express historical release
payload or arbitrary Rank. Retaining A and replaying this known independent
later request preserves the original pair without broadening the Edit format.
[Current apply input and placement contract](https://github.com/JX-Shen/OliviaMIDI/blob/7b5a0a894f4364962a2f49533c64bea8a31a08cb/src/bin/mid/apply.rs),
[event preservation](adr/0003-a-take-round-trips-at-the-event-level.md),
[Rank](adr/0008-events-sharing-one-instant-are-heard-in-the-order-they-are-written.md)

A paired-source control changes only the deleted release velocity in A to 99.
The two originals produce identical B and identical C after deletion and later
editing, yet their source-preserving reconstructions require 37 and 99. Without
the original evidence, the required payload is not recoverable from those B/C
files. With A retained, this case is constructible. Missing information and
insufficient expression in a particular inverse request are different limits.

This suggests a separate withdrawal investigation using explicit A, the
experiment and later requests. It does not establish universal replay: later
requests may depend on an added note, on an intermediate value or on a changed
address. Whether to preserve a later request, its realised terminal value or
musical intention remains a human-owned question when they diverge.

The executed report, its generator (`probe.py`, run as
`WITHDRAWAL_MID=/path/to/mid python3 probe.py --output-root <new directory>`)
and the final assertions were kept only in a temporary directory and are not
retained. Temporary artifacts are not a permanent repository fixture; the
source recipe, inputs, expected outcomes and governing primary sources are
retained here.

## Snapshot evidence: consistency is not a unique history

Three cases plus controls used current `mid` and complete Mido event checks.
They follow [content identity](adr/0002-notes-are-content-addressed.md) and
[bounded inference](adr/0004-a-judgement-made-for-the-human-is-bounded-stated-and-declinable.md).
They do not establish that snapshot-based combinations must refuse; a declared
content convention can be useful without promising recovery of actual history.

- **Identical additions:** A has an empty musical track with a retained tail.
  Independently adding p60 at 120, duration 120, velocity 80 gives byte-identical
  B and C. Native candidates containing the addition once or twice both succeed.
  The files cannot say whether the user means one shared contribution or two
  distinct contributions. The multiplicity needs the promised semantics.
- **Complete replay still has more than one explanation:** A has one note,
  release velocity 0, and a track-name event. An empty Edit Set and an Edit Set
  deleting then recreating exactly that note produce byte-identical B files;
  both equal A at event level. Both accounts were actually executed. A replay
  check can validate either account's consistency, but cannot certify which
  account uniquely describes an unknown past. A control changing only release
  velocity to 23 keeps the Notes equal but changes events, confirming that the
  full event oracle still catches omitted payload.
- **Known external coordinates work, symmetric ones remain ambiguous:** For
  two distinct musical tracks (p60 and p67), explicitly swapping tracks 1/2 and
  doubling PPQ plus every event delta allows a velocity request on A's track 1
  to be carried to B's track 2. The native result equals independently applying
  that transformation to the already edited C. Direct A/B diff refuses unequal
  PPQ, as intended. If both complete tracks are identical, identity and swapped
  mappings after the same scaling produce byte-identical B, but carrying C's
  velocity 40 yields `(t1=40,t2=80)` or `(t1=80,t2=40)`. Neither track names nor
  any other file content breaks the symmetry, because all of it was equal.

The PPQ experiment uses exact integer metrical scaling only; it says nothing
about quantising exports, SMPTE timing or opaque payloads containing timing.
Preserving track index is a possible declared convention, not evidence of an
external editor's historical behaviour. Nothing here requires a provenance
authentication subsystem or changes Git's ownership of storage/history.

The generator (`probe.py`, run as `MID=/path/to/mid python3 probe.py` in a
disposable directory) and its asserted receipts were kept only in a temporary
directory and are not retained. Its README recorded the exact native commands and distinction between content policy and
historical claims. No existing core-contract contradiction was found.

## A real performance control: MAESTRO with recorded pedal data

The [official MAESTRO source](https://magenta.tensorflow.org/datasets/maestro)
describes Disklavier performance capture and supplies v3.0.0 under CC BY-NC-SA
4.0. This research downloaded its MIDI-only archive and verified the published
SHA256 `70470ee253295c8d2c71e6d9d4a815189e35c89624b76d22fce5a019d5dde12c`.
No dataset files or derived music are added to this repository or its MIT
fixtures. Dataset credit: Hawthorne et al., *Enabling Factorized Piano Music
Modeling and Generation with the MAESTRO Dataset*, ICLR 2019.

Selection was mechanical: the shortest metadata duration, 45.1552083333 seconds,
labelled Liszt, Transcendental Etude No. 1, “Preludio”, year 2015. Archive member:
`maestro-v3.0.0/2015/MIDI-Unprocessed_R1_D1-1-8_mid--AUDIO-from_mp3_06_R1_2015_wav--3.midi`.
MIDI SHA256: `c72281527ca6588836bd1e109de848a8fd2ac246bbf06e25a58939d3168468fe`.
These selection facts come from downloaded metadata and parsing, not a judgement
that this is a representative performance corpus.

Actual input: Type 1, PPQ 480, two tracks, 629 notes, 517 CC64 events and one
CC67 event. The reported 4/4 and constant 500000 μs/quarter are file coordinates;
they are not evidence of the performer's notated bar boundaries. All notes are
on track 1/channel 0; their releases use note-on velocity zero.

The control deliberately uses a Tick window rather than inferred musical parts:

- B reduces velocity by 12 for all 17 notes below pitch 60 starting before Tick
  3840, clamped at 1. This pitch partition is explicit; it does not claim to
  identify a hand or an accompaniment voice.
- C lengthens `t1:c0:p64:s2910:n0` by 470, changing its written release from
  2969 to 3439. This is a release-context experiment, not a proposed improvement.
- The existing common-base pilot constructs D, verifies both source requests
  against actual B/C and finds equal events for both coordinated request orders.
  Its independent preservation check passes. All 629 notes remain, all 518
  Controller events keep content/Tick/order, and the edited release retains its
  note-on-zero encoding. Only the 17 velocities and that duration are requested.

The last CC64 before the target release is 127 in A and 63 in D; the recorded
switch to 63 occurs at 3438. Therefore exact preservation of the Controller
stream does not mean the edited note keeps its old relationship to that stream.
The prototype explicitly does not assess that relationship. No sound was
rendered or heard for this case, and written duration is not substituted for
actual sounding duration.

The successful final receipt, with the source metadata, generator and initial
files beside it, was kept only in a temporary directory and is not retained.
The generator uses the earlier temporary common-base prototype and creates a
fresh output directory. An initial report step used the wrong diff flag;
current help established `--tolerance`, the script was corrected, and the full
fresh run including its zero-tolerance report completed successfully.
The archive and local prototype are research dependencies, not shipping assets.
This does not satisfy #35/#37 release-corpus or listening acceptance by proxy.

Independent final review did not import the pilot's verifier: a separate Mido
FIFO pairing and event reconstruction confirmed exactly those 17 velocity
changes, the single release move, both Controller counts, every other carried
event and the source hash. It did not independently rerun B/C replay or the
pilot's two construction paths. Another challenger reran all 16 whole-candidate
applications and reviewed the note's inference/policy boundaries without finding
an overclaim. These are bounded checks, not whole-suite or release gates.

## Proposed next work, with the choices still visible

The evidence supports one small product decision first: whether to combine
explicit original-relative requirements under a declared rule. Same terminal
requirements once and differing requirements without an automatic winner are
candidate semantics, not conclusions the experiments alone can choose. The
owner also needs to decide whether differing legal event orders are selectable
candidates or a refusal in the first scope. The current pilot's conservative
restrictions remain research restrictions.

If that meaning is adopted, a complete first slice can reuse ordinary Takes,
existing Edit Sets, current application constraints, independent preservation
checks, diff and audition. Verify supplied source accounts, coordinate the
requirements, construct a candidate, and check the entire result. Keep concrete
context interactions separate from failed requirements; neither a clean write
nor a clean diff certifies musical independence.

Selective withdrawal deserves its own later slice. The positive result is
guarded reconstruction from retained source plus known later work, with a
no-visible-change success case and a deleted-payload control. The negative
result is that a universal same-shape inverse applied to C is not justified.
Snapshot-only imports likewise need their own explicit content/correspondence
promise. These findings give no reason yet for a new durable format, generic
dependency graph, arbitrary-event editing surface or history manager.

## Reproduce the state experiments

The original script and its final run were kept only in a temporary directory
and are not retained. The complete script is embedded below so they need not
survive. Save it as `probe.py` in a disposable directory. With Mido installed,
build the named baseline into a Cargo target directory outside the tree, then
run:

```sh
cargo build --locked --offline --target-dir "$build" --bin mid
MID="$build/debug/mid" python3 probe.py
```

The script reads current `apply --help` before writing Edit Sets. Set
`FLUID_LIB` if the FluidSynth library is at a different path; rerunning against
another version is a new implementation observation. The script creates a
fresh run directory and writes inputs, requests, candidates, inspections,
diffs and a results receipt there. `GEN_COARSETUNE = 51` is the version-pinned
FluidSynth header identifier, not a MIDI Controller number.

```python
import ctypes as C
import json
import os
from pathlib import Path
import subprocess
import tempfile
import mido

MID = os.environ['MID']
ROOT = Path(tempfile.mkdtemp(prefix='run-', dir=Path(__file__).parent))
subprocess.run([MID, 'apply', '--help'], check=True, stdout=subprocess.DEVNULL)

def cc(t, n, v, ch=0):
    return t, mido.Message('control_change', channel=ch, control=n, value=v)

def source(path, events):
    f = mido.MidiFile(type=0, ticks_per_beat=480)
    track = mido.MidiTrack()
    f.tracks.append(track)
    track.append(mido.MetaMessage('time_signature', numerator=4, denominator=4))
    last = 0
    for t, msg in sorted(events, key=lambda e: e[0]):
        track.append(msg.copy(time=t-last))
        last = t
    track.append(mido.MetaMessage('end_of_track', time=400-last))
    f.save(path)

def events(path):
    f = mido.MidiFile(path)
    result = []
    for track in f.tracks:
        tick, items = 0, []
        for msg in track:
            tick += msg.time
            items.append((tick, msg.copy(time=0).dict()))
        result.append(items)
    return f.type, f.ticks_per_beat, result

def apply(folder, name, src, edits):
    request = folder / (name + '.json')
    request.write_text(json.dumps({'edits': edits}, indent=2))
    out = folder / (name + '.mid')
    subprocess.run([MID, 'apply', str(src), str(request), '-o', str(out)],
                   check=True, capture_output=True, text=True)
    inspection = subprocess.check_output([MID, 'inspect', str(out), '--json'], text=True)
    (folder / (name + '-inspect.json')).write_text(inspection)
    return out

def setter(n, tick, value, ch=0):
    return dict(kind='set_controller', track=0, channel=ch,
                controller=n, tick=tick, value=value)

def pedal(path):
    # Only read the last written CC64 before this fixture's sole NoteOff.
    value = None
    for tick, msg in events(path)[2][0]:
        if msg['type'] == 'control_change' and msg['control'] == 64:
            value = msg['value']
        if msg['type'] == 'note_off':
            return {'release_tick': tick, 'cc64_before_release': value}

lib = C.CDLL(os.environ.get('FLUID_LIB', '/opt/homebrew/lib/libfluidsynth.dylib'))
def signature(name, restype, *args):
    f = getattr(lib, name)
    f.restype, f.argtypes = restype, args
    return f
ptr, integer = C.c_void_p, C.c_int
version = signature('fluid_version_str', C.c_char_p)().decode()
new_settings = signature('new_fluid_settings', ptr)
new_synth = signature('new_fluid_synth', ptr, ptr)
send_cc = signature('fluid_synth_cc', integer, ptr, integer, integer, integer)
get_range = signature('fluid_synth_get_pitch_wheel_sens', integer, ptr, integer, C.POINTER(integer))
get_gen = signature('fluid_synth_get_gen', C.c_float, ptr, integer, integer)
delete_synth = signature('delete_fluid_synth', None, ptr)
delete_settings = signature('delete_fluid_settings', None, ptr)

def rpn(path):
    settings = new_settings()
    synth = new_synth(settings)
    assert settings and synth
    try:
        for _, msg in events(path)[2][0]:
            if msg['type'] == 'control_change':
                assert send_cc(synth, msg['channel'], msg['control'], msg['value']) == 0
        result = []
        for ch in (0, 1):
            value = integer()
            assert get_range(synth, ch, C.byref(value)) == 0
            result.append({'channel': ch, 'bend_range_semitones': value.value,
                           'coarse_tuning_semitones': get_gen(synth, ch, 51)})
        return result
    finally:
        delete_synth(synth)
        delete_settings(settings)

def case(name, base_events, left, right, observe):
    folder = ROOT / name
    folder.mkdir()
    a = folder / 'A.mid'
    source(a, base_events)
    before = a.read_bytes()
    paths = {'A': a}
    paths['B'] = apply(folder, 'B', a, left)
    paths['C'] = apply(folder, 'C', a, right)
    paths['D'] = apply(folder, 'D', a, left + right)
    paths['E'] = apply(folder, 'E', a, right + left)
    paths['BC'] = apply(folder, 'BC', paths['B'], right)
    paths['CB'] = apply(folder, 'CB', paths['C'], left)
    assert before == a.read_bytes()
    expected = events(paths['D'])
    assert all(events(paths[k]) == expected for k in ('E', 'BC', 'CB'))
    diff = subprocess.check_output([MID, 'diff', str(a), str(paths['D']), '--tolerance', '0', '--json'], text=True)
    (folder / 'diff.json').write_text(diff)
    return {'ordered_events_equal_for_D_E_BC_CB': True,
            'observations': {k: observe(p) for k, p in paths.items()}}

on = (0, mido.Message('note_on', note=60, velocity=80))
off = (100, mido.Message('note_off', note=60, velocity=17))
pedal_base = [cc(0,64,0), on, off, cc(150,64,0), cc(300,64,0)]
resize = lambda delta: dict(kind='resize_note', id='t0:c0:p60:s0:n0', delta_ticks=delta)
results = {'mid': MID, 'fluidsynth': version, 'cases': {}}
results['cases']['pedal_crossing'] = case('pedal_crossing', pedal_base,
    [setter(64,150,127)], [resize(100)], pedal)
results['cases']['pedal_same_tick'] = case('pedal_same_tick', pedal_base,
    [setter(64,150,127)], [resize(50)], pedal)
rpn_base = [cc(0,101,0), cc(1,100,0), cc(10,6,2)]
results['cases']['rpn_coupling'] = case('rpn_coupling', rpn_base,
    [setter(100,1,2)], [setter(6,10,12)], rpn)
control_base = rpn_base + [cc(2,101,0,1), cc(3,100,0,1), cc(11,6,2,1)]
results['cases']['rpn_other_channel_control'] = case('rpn_other_channel_control', control_base,
    [setter(100,1,2)], [setter(6,11,12,1)], rpn)
cross = results['cases']['pedal_crossing']['observations']
assert [cross[k]['cc64_before_release'] for k in ('A','B','C','D')] == [0,0,0,127]
assert cross['D']['release_tick'] == 200
same = results['cases']['pedal_same_tick']['observations']['D']
assert same == {'release_tick':150, 'cc64_before_release':0}
coupled = results['cases']['rpn_coupling']['observations']
assert coupled['C'][0]['bend_range_semitones'] == 12
assert coupled['D'][0]['bend_range_semitones'] == 2
assert coupled['D'][0]['coarse_tuning_semitones'] == -52
control = results['cases']['rpn_other_channel_control']['observations']
assert control['D'][0] == control['B'][0]
assert control['D'][1] == control['C'][1]
(ROOT / 'results.json').write_text(json.dumps(results, indent=2))
print(ROOT)
print(json.dumps(results, indent=2))
```

## Daytime continuation: evidence guards, later targets and prepared audition

2026-09-10, after the owner approved the mechanical follow-up and audition
preparation. Same source baseline as above; `cargo build --locked --offline
--target-dir "$build" --bin mid`, into a directory outside the tree, succeeded
before execution.
The live apply help was read. Production code and the earlier prototype were
unchanged. These observations do not adopt combination or withdrawal semantics.

### Source evidence and output boundaries

A new Type-0 / PPQ-480 control carries four distinct-pitch notes, a CC11 curve,
Program, tempo, time signature, two same-Tick text events and SysEx. The notes
are `(start,end,pitch,release velocity)` = `(0,100,60,17)`, `(200,300,62,23)`,
`(400,500,64,29)`, `(600,700,65,31)`, all at strike velocity 80. The track ends
at 1000. CC11 at 0/10/20/30/40 is 10/40/60/30/0.
One side sets p60's velocity to 55 and deletes p64; the other extends p62 by 35
Ticks and sets p65's velocity to 66.

Executed against supplied A/B/C and their retained requests:

- Changing B's intermediate CC11 value 40 to 50, its first release velocity to
  99, its two same-Tick text events' order, its PPQ to 960, or adding an extra
  text event each produces `source_evidence_mismatch` and no output.
- A wrong A (first strike velocity 81), swapped candidate files without their
  requests, and a stale request (55 changed to 54) are also refused without
  output. All supplied input bytes remain unchanged.
- Exact diff between expected B and the tampered file reports the intermediate
  CC change. It is empty for the release-payload, text-order and extra-text
  cases. The replay guard catches each despite those empty reports.
- A separate control writes every MIDI status byte explicitly, including the
  SMF length field for SysEx. Its bytes differ from B while its parsed events
  agree. The prototype accepts it and produces the same parsed candidate. The
  guard is event equality, not a byte-hash requirement.
- An existing output and an output symlink to A are refused; their existing
  bytes remain intact.

Malformed JSON (`{`) and a wrong top-level shape (`[]`) both exit nonzero and
leave no output, but escape the disposable CLI as `JSONDecodeError` and
`TypeError`, with no structured result on stdout. This is a diagnostic gap in
the research wrapper, not an observed production-core defect or a false
successful candidate. It was recorded, not repaired into an unadopted public
interface.

### Ordering, independent preservation and measured execution cost

All eight combinations of within-side request permutations and side exchange
produce the same parsed candidate. All 24 complete permutations of these four
requests also succeed through current apply and agree at parsed-event level.
This is one controlled fixture, not a proof of general permutation invariance.

Schumann, Burgmuller and MAESTRO were each reconstructed five times. Every
candidate agrees with its earlier retained result at complete parsed-event
level; repeated output bytes also happen to agree. A separate verifier, not
the prototype's `verify`, reconstructs expected event changes from the source,
checks each moved release's payload and Tick, and compares the remaining
ordered events. It confirms the original 26, 28 and 18 requested changes,
respectively, and preservation of the other events. All inputs stay unchanged.

The same three already-known disjoint request pairs were also applied directly
to their common source five times and independently verified. Median warm-run
times in this execution were:

| Case | Research wrapper | Direct apply |
| --- | ---: | ---: |
| Schumann | 114 ms | 5.7 ms |
| Burgmuller | 108 ms | 5.4 ms |
| MAESTRO | 272 ms | 8.5 ms |

The wrapper makes one inspect call and four apply calls: two source replays and
two candidate orders, in addition to Python parsing and verification. The
direct timing includes request-file writing and one apply; the separate
post-run verification is outside both timed regions. Direct apply does not
validate the supplied B/C evidence or compare construction orders. These are
different assurance levels, so the numbers establish execution overhead only,
not a fair speed contest or human productivity improvement. Human correction
effort remains unmeasured. The reusable work observed here is coordination and
repeated evidence checking; no new core representation was needed.

### Withdrawal must carry later correspondence, not merely later JSON

Reuse the three stacked p60 notes at Tick 0, duration 100, strike velocity 80,
distinguished for this experiment by release velocities 10/20/30. Delete
original n0 to make B. On B, set n1 to velocity 55: this reaches the original
third note, whose release velocity is 30.

Withdrawing the deletion by returning to A and replaying that later JSON
unchanged succeeds, but sets the original second note (release 20) to 55.
Explicitly mapping the later B.n1 to A.n2 from the known deletion instead
restores the first note and preserves the later change to the original third.
All three applications are legal. Thus retained source plus later requests is
insufficient unless the later targets are correctly bound to that source;
source reconstruction is not permission to reuse destination-relative IDs.
The release payloads identify event pairs for the probe only, not a proposed
public identity scheme. This refines the earlier reconstruction proposal; it
does not contradict the core's documented input-relative binding.

A genuinely dependent case adds p67 at [800,900] with velocity 80, then sets
that new note's velocity to 50. Replaying the later request on A while
withdrawing the creation correctly fails: A has no such target, and no output
is written. Withdrawing the creation requires explicitly discarding or
replacing the dependent request; retaining the changed new note retains part
of the earlier contribution. Neither interpretation was chosen for the user.

### Audio prepared; the performance boundary is a weak listening contrast

The configured `BATTUTA_SOUNDFONT` resolved to the previously used GeneralUser
GS. Complete A/B/C/D Takes for Burgmuller and Liszt were rendered before any
audio excerpting, retaining the full prior synthesis context. Settings were
FluidSynth 2.6.0, 44.1 kHz, stereo s16, gain 0.2, chorus/reverb on, one synthesis
core and no loudness normalisation. Rig path, SoundFont SHA256, input/audio
hashes and commands are in the audio manifest. Each complete A was rendered
again; its PCM agrees exactly with its first rendering.

All eight complete WAVs are non-silent, have zero clipped samples and a final
second with integer PCM RMS zero. Their complete lengths are 31.608 seconds
for Burgmuller and 50.932 seconds for Liszt. These are signal checks, not
listening acceptance.

- Burgmuller's A/B/C/D comparison uses the audio corresponding to source Ticks
  768:5376 (0.789472:5.526304 seconds), covering the changed file Bars 3:6 with
  preceding/following context. The four sections start at 0, 5.736832,
  11.473664 and 17.210496 seconds, with one-second gaps.
- Liszt's comparison uses the first ten seconds; sections start at 0/11/22/33.
  A/C isolates duration, and B/D isolates duration in the changed-velocity
  context. The first A/C PCM difference is at 3.583186 seconds. Its maximum
  absolute difference is only 5 s16 units, full-file difference RMS 0.07544,
  and 2.9:5-second-window difference RMS 0.37096. A measurable file difference
  here does not establish a useful audible contrast. The earlier 127-versus-63
  pedal reading must not be presented as evidence of a large sounding change.

Comparison excerpts use 20 ms presentation fades at their outer edges; full
WAVs are unchanged. MP3s were encoded and decoded successfully. Their metadata
states order, changes, Rig and source attribution. Burgmuller retains the
source's public-domain declaration; Liszt retains MAESTRO's CC BY-NC-SA 4.0
attribution and licence. No music was put in the repository. No listening or
remote delivery is claimed.

### Receipts and limits

Four scripts did this work: `probe.py` (input guards, repeatability,
preservation, timing and withdrawal correspondence), `boundaries.py`
(reencoding/output/malformed-input observations), `dependencies.py` (all 24
orders and the created-target dependency) and `render.py` (offline audio and
checks). They and their receipts — one results file per script, plus the audio
manifest and duration measurements — were kept only in a temporary directory
and are not retained; the constructions and measured conclusions above survive
independently of them. The reencoding generator initially
omitted the SMF SysEx length; inspection of Mido's writer identified and fixed
that fixture error before its successful control. MP3 export initially used
split metadata arguments instead of `key=value`; correcting those arguments
completed export using the existing verified WAVs. Neither failed preparation
was counted as a tested candidate or a product defect.

Only this research record changed in the repository. No production repair,
new behaviour Decision, full-suite run, CI/release acceptance, commit, issue
write or publication is claimed by this continuation.

### Publication

Later on 2026-09-10, the owner authorised publishing the results. The public
records are [mechanical follow-up in #49](https://github.com/JX-Shen/OliviaMIDI/issues/49#issuecomment-5611878806),
[real-file and audio measurements in #48](https://github.com/JX-Shen/OliviaMIDI/issues/48#issuecomment-5611879372),
and [the consolidated position and evidence index in #47](https://github.com/JX-Shen/OliviaMIDI/issues/47#issuecomment-5611884448).
All three published bodies were read back and matched the prepared text under
the JX-Shen account. Scripts, raw receipts and audio remain local; no audio
attachment, repository commit, adopted Decision or release accompanied the
publication. Future public research updates belong on those issues.

## Construction-order feasibility probe

2026-09-10, same baseline, after the owner preferred investigating a uniform
construction convention over rejecting every legal order difference. This
preference authorised validation, not adoption of a specific ordering rule.
Production code and the earlier common-base prototype remain unchanged.

A disposable probe coordinates final requirements, then orders requests by
field phase (deletion, velocity, duration), track and reverse original A event
index. For a duration request that index is its release event's index. Current
apply still performs all event placement: because a relocated release is
prepended at its destination, reverse source processing retains A's original
relative release order when those releases newly meet. This is one candidate
convention for the earlier restricted domain, not public Rank input or
normalisation of carried events on read.

Actual execution uses full source-replay checks, the unchanged prototype's
preservation verifier and existing core result constraints. For the synthetic
release cases, an additional independent scan removes only the relocated
releases and confirms exact equality of the remaining absolute-Tick event
streams. Input bytes remain unchanged.

### Positive result and counterexample

Use A with p60 at [0,100], velocity 80, release velocity 10, and p62 at
[50,200], velocity 90, release velocity 20. Type 0, PPQ 480, one track/channel,
4/4, tail 1000. X extends p60 by 200; Y extends p62 by 100. Both releases
finish at Tick 300.

- One side supplies X and the other Y. The proposed convention resolves the
  previously observed order difference, produces p60 then p62 releases, and
  gives identical complete events when left and right inputs are exchanged.
- B is produced by requests X then Y; its releases are p62 then p60. C is
  produced by Y then X; its releases are p60 then p62. Both source accounts
  replay exactly and make identical final note-field demands.
- Combining B with an empty side changes its release order to C's. Combining
  B with itself does the same. Combining C with an empty side or itself keeps
  C. All these outputs are legal, meet final field demands and preserve A's
  unedited events. Exact diffs against the corresponding existing candidates
  were saved; no audio equivalence was tested.
- Adding an unrelated p67 at [400,700] to A and having the other side change
  only its velocity to 50 still changes B's p60/p62 release order. Thus an
  empty-side shortcut alone would not address the preservation question.
- Schumann, Burgmuller and MAESTRO controls each construct successfully in
  both left/right orders and equal their earlier retained candidates at
  complete parsed-event level. Their preservation checks pass. This is bounded
  feasibility evidence, not coverage of arbitrary request families.

The precise limitation is on a construction that depends **only on A and
coordinated final field demands**: B and C give it the same information, so
that construction cannot also reproduce both event-distinct candidates in
all single-side or identical-side cases. A source-sensitive construction has
not been disproved by this probe. Preserving an existing candidate when the
other side contributes nothing, and preserving an identical pair of candidates,
are properties tested here, not previously adopted public behaviour.

The unresolved product question is whether already established ordering among
edited candidate events must be carried into combination, and where a uniform
convention may fill a newly created ordering gap. No special-case repair,
source-sensitive algorithm, new rejection policy or contract amendment was
implemented. The experiment stops at that semantic choice.

The generator and its successful receipt, with the source, replay and
candidate files and exact diffs, were kept only in a temporary directory and are
not retained. The source and request recipes
above preserve the central counterexample. No production change, full-suite
run, audio rendering or public issue update accompanied this probe.

## 0.2.x specification draft — owner discussion, 2026-09-10

The owner approved publication and then consolidation into a single entry
point. [#47](https://github.com/JX-Shen/OliviaMIDI/issues/47) now owns the
current 0.2.x series direction, including shared boundaries, release links,
unresolved research and evidence links. Its rewritten
body was read back and matched the prepared text under JX-Shen; it remains
open and labelled needs-triage.

The briefly separate [#50](https://github.com/JX-Shen/OliviaMIDI/issues/50)
is closed as a retired duplicate, with its historical draft retained and
marked as migrated. The [map comment](https://github.com/JX-Shen/OliviaMIDI/issues/47#issuecomment-5613766199)
records that consolidation. Subsequent discussion established a separate
release scope: [#51](https://github.com/JX-Shen/OliviaMIDI/issues/51) now owns
the 0.2.0 specification and acceptance, as a native child of #47. Both remain
open and labelled needs-triage. Keep release-specific updates in #51 and
shared direction in #47 rather than maintaining a second local body.

Earlier sections remain dated research observations. The owner accepted the
complete CLI workflow and real-performance collaboration/audition as the
minimum acceptance layers. Construction and precise interface/input boundaries
remain open. Relationship explanation is a research priority, especially for
effects introduced by combination; its exact release obligation awaits evidence
and an owner decision. The published release draft records these distinctions.
No production implementation, implementation-ready status or release is implied
by this documentation publication.
