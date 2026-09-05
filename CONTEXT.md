# OliviaMIDI

A human and an agent work on one piece of music together, held as MIDI files on
disk. This glossary pins the vocabulary they use to refer to it.

Terms are deliberately opinionated. Where music and software disagree on a word,
the musical meaning wins and the software concept is renamed — see the naming
rules in `CHARTER.md`. Each term pins a Chinese equivalent, because the human and
the agent converse in Chinese and an unpinned term drifts.

## The work

**Piece**:
The musical work being created, persisting across every version of it. Not a
file.
_中文_: 作品
_Avoid_: score, song, project; 曲子、项目

**Take**:
One concrete `.mid` file — a version of the Piece that can be heard.
_中文_: 条 (as in a studio's 再来一条)
_Avoid_: version, revision, draft; 版本、稿

**Edit**:
One mechanical change to a Take: add, delete, move, transpose or resize a note,
change a velocity, change which Program a channel is on, change a CC. Never
carries musical intent — no "make it sadder".
An **Edit Set** is a batch of them, applied together (`edits.json`).
_中文_: 改动
_Avoid_: patch, mutation, operation; 补丁

`operation` is avoided for register, not for collision. The music-wins rule does
not reach it — its musical sense belongs to transformational theory, not to
notation — but an operation sounds large enough to carry intent, and an Edit is
deliberately flatter than that. So the discriminator key in `edits.json` is
`kind`, never `op`.

**Transpose**:
Moving a note by a number of semitones and changing nothing else — its start,
its length and its velocity are untouched. One kind of Edit.
_中文_: 移调
_Avoid_: modulate, shift; 转调、变调

Not 转调. A modulation is a piece changing key, which is a reading of the music
and not a thing done to one note — and an Edit never carries a reading. Moving
one note down two semitones is not a modulation, and would still not be one if
every note in the Take were moved with it. 变调 is vaguer again: in ordinary
Chinese use it is whatever the button on a karaoke machine does, which is
sometimes this and sometimes a change of speed.

**Orchestration**:
Which instrument plays which part of the Piece. In the file, and so the Piece —
`CHARTER.md` puts it plainly: orchestration is composition. What each instrument
*sounds* like is the Rig, and the two are never re-filed into one another.
_中文_: 配器
_Avoid_: arrangement, instrumentation; 编曲、配置

Not `instrumentation`, although in theory that is the narrower and more accurate
word — instrumentation is which instruments, orchestration is what is done with
them. `CHARTER.md` has said Orchestration since before this file existed, and a
glossary that renamed the charter's term would leave the project with two words
for one boundary. 编曲 is avoided because in ordinary Chinese use it covers the
whole arrangement, groove and production included, most of which is not in the
file at all.

## Hearing it

**Rig**:
The apparatus a Take is heard through — synthesiser, soundfont, playback
settings. Never part of the Piece, and never reported by a diff.
_中文_: 音源装置
_Avoid_: patch, timbre, realization, sound, voice; 音色、音色库、音源

Note that `patch` is reserved for its synthesiser meaning — a sound preset —
and so belongs on this side, never to an Edit Set.

## Musical position

**Bar**:
A measure of the Piece. Bars are 1-indexed, and ranges include both ends:
`--bars 5:8` is bars five through eight, four bars in total.

Bars are counted from Tick 0 of the Take, and the last one counts even when the
Take stops part way inside it — `fixtures/olivia.mid` is eight bars of 3/4, not
seven and a bit. A note belongs to the bar it *starts* in, even when it sustains
across the bar line. Which time signature the bar lines come from, and what
happens when the Take states none or states two, is #3.
_中文_: 小节
_Avoid_: measure, battuta (taken as the crate name); 拍、节

**Beat**:
One of the equal divisions a Bar is counted in. A Bar has as many Beats as its
time signature's numerator says: three in 3/4, six in 6/8. Beats are 1-indexed
within their Bar, and a note falling between two of them is placed by the Beat it
follows and the Ticks it is past it — never by the nearest one, which would put
two different notes in the same place.
_中文_: 拍
_Avoid_: pulse, count; 节拍、拍子

Six Beats in 6/8 is the time signature read literally, and a literal reading is
the only one available here. A musician conducting 6/8 beats it in two, and is
right — but that is a reading of the music, and a derived view of a Tick has no
standing to make one. It is the line drawn under **Time signature** between the
sign and the metre, one level further down.

节拍 and 拍子 are avoided for the reasons they are already avoided there: 节拍
drags speed along with it, and 拍子 names the feel rather than the count. **Bar**
lists 拍 among the words to avoid, which is this term claiming it rather than a
contradiction — 小节 is the Bar, 拍 is what it is counted in.

**Position**:
Where a Tick falls once the Bars are derived: which Bar, which Beat of it, and
how far past that Beat. What `mid inspect` prints in place of a raw Tick, and
what a Take with no derivable Bars has none of — there the Tick is printed as
itself.
_中文_: 位置
_Avoid_: location, place, spot; 位点、坐标

**Tick** lists `position` among the words to avoid, and this does not overturn
that; why it does not is argued there, next to the list it narrows. The short of
it is that a Tick is the truth and a Position is one reading of it, and the
section they both sit in has been called Musical position since before either
term existed.

**Passage**:
The stretch of a Take a Bar range names — what `--bars 5:8` selects, for
`inspect` to list and `play` to sound. Under no obligation to be a unit of the
music: a passage is wherever the two of us are pointing, which may be the second
half of one phrase and the start of the next.
_中文_: 片段
_Avoid_: section, excerpt, clip, range; 乐段、乐句、段落

The Chinese is 片段 rather than 乐段 or 乐句 because both of those name formal
units — a period and a phrase — and a Bar range is not obliged to be either;
段落 has the same problem one level up. In English, `section` is taken twice
over, by musical form and by software, `excerpt` and `clip` both suggest
something lifted out to keep, and `range` names the argument rather than the
music it selects.

**Time signature**:
How many notes of what value make one bar, as carried by the MIDI time signature
meta event. Part of the Piece, not of the Rig. Every bar line is derived from it
and from nothing else; what happens when a Take states none, or none that governs
the whole of it, is #3.
_中文_: 拍号
_Avoid_: metre, meter, time sig; 节拍、拍子

Not `metre`, although it is the more musical-sounding of the two. In theory the
metre is how beats are organised and the time signature is the sign that states
it, and the sign is what a file can carry and this tool can read. A Take with no
time signature event does not lack a metre — the music has one, it simply is not
written down — so `metre` would make every refusal in #3 a false statement
about the music, where each is a true one about what the file *says*. Only a sign
can be the object of saying. `time` on its own is genuinely musical — *in 3/4
time* — but is already avoided for Tick.

The Chinese is 拍号 rather than 节拍 because 号 is a mark: it names the thing
written in the file, which is exactly what a meta event is. 节拍 drags speed along
with it — 节拍器 is a metronome — and would leave the time signature sharing a
root with the tempo. 拍子 names the feel rather than the mark, and goes vague on
the one question this project needs answered: 3/4 and 6/8 have the same bar
length and different time signatures.

**Tick**:
The unit of musical time inside a MIDI file, relative to the file's PPQ. Ticks
are the truth; bars, beats and seconds are all derived views of them. The finest
time a file counts in, and not an indivisible one: several events may share a
Tick, and the file still writes them in an order. See **Rank**.
_中文_: 刻度
_Avoid_: time, position, offset

Avoided as names *for a Tick*. A Tick is the root, and naming the root after one
of its readings would make the truth sound derived.

**Position** then takes one of the three, which needs saying because this entry
is where it would be checked. An avoid list bans a word as a synonym for its own
term; it does not retire the word. `CHARTER.md` settles the same question for
`patch`, which **Rig** and **Edit** both avoid and which the Rig side may
"legitimately use" for a sound preset. Read the other way — the word retired —
this file would already contradict itself, since **Bar** avoids 拍 and **Beat**
is 拍.

The test is whether the claimant is the banned term wearing a different word. A
Position is not: it cannot be stored in a file, no Edit accepts one, and a Take
that states no time signature has none at all while still having every Tick it
ever had. What `position` may never name is the count in the file.

The near case is under **Time signature**, which declined `time` for itself on
the strength of this entry. It was right to: *in 3/4 time* is genuine, but a
`time` in a MIDI tool is read as the moment something happens, and that is what
a Tick names. The word was a synonym; a Position is a reading.

Read *moment* there as *when*, not as *indivisible*. A Tick is the finest time
the file counts in, and several events still share one — which is **Rank**, and
which this entry did not have to know about, because a Position reads a Tick and
cannot see inside it either.

**Rank**:
Where an event falls among the events sharing its Tick — which of them a
synthesiser meets first. A Tick is the truth about *when*; a Rank is the truth
about *which first*, and it is as audible: a Program met after the note it was
set for leaves that note on the instrument before it, and a damper met after the
note it was set for catches nothing.

A Rank is derived, never stated. No Edit accepts one and no command prints one.
An Edit names a Tick, and where its event falls among that Tick's is settled by
one rule rather than by the Edit — because a Take gives every event a Rank
whether anybody chose it or not, and a tool that let an Edit Set state one would
be addressing below the Tick, one step from the selector `CHARTER.md` forbids.
Two events at one Tick have one Position between them and a Rank each.
_中文_: 次序
_Avoid_: order, precedence, priority; 顺序、优先级

**Rank** takes a word an organ builder uses for a row of pipes. `AGENTS.md`
gives the musical meaning priority where two genuinely collide, and these do
not: nothing here names a pipe, and neither does MIDI's own vocabulary. What
settled it is that the code had been saying it for three versions — `track.rs`
calls an event's place at its Tick a rank in seven sentences written before this
term existed — while the field carrying it was called `order`, which this file
cannot claim, because the repository already spends that word loosely in a
hundred and fifty places. The term ratifies the prose and renames the field.

`order` is avoided as a *name for a Rank*, by the rule argued under **Position**:
an avoid list bans a word as a synonym for its own term and does not retire it.
*The order they arrived in* stays good English and stays in use.

**Tempo**:
How fast a Take's ticks pass, as carried by the MIDI tempo meta event:
microseconds per quarter note, which `mid info` also reports as beats per minute.
Part of the Piece, not of the Rig. Italian wins here by the rule in `CHARTER.md`,
which names tempo as a case where it genuinely does.
_中文_: 速度
_Avoid_: speed, rhythm; 节奏、节拍

**Pitch**:
Which key is struck, as the MIDI note number the file carries: 0 to 127, one
semitone apart, with 60 as middle C. Part of the Piece, not of the Rig.

Its **pitch name** (音名) is what `mid` prints beside it — `F#4`, `A2` — under
two conventions the file does not state: sharps rather than flats, and middle C
in octave 4. A name is a gloss on the number and never a replacement: an identity
still reads `p66`, `--json` still carries the number, and no Edit accepts a name.
See #7.
_中文_: 音高 (the name: 音名)
_Avoid_: note, tone, key; 音符、音调
_Avoid for the name_: note name, note letter; 唱名

唱名 is the other naming system entirely — do, re, mi — which is a degree of a
scale rather than a pitch, and moves when the key does. A pitch name does not
move: `F#4` is pitch 66 in every Take.

Not `note`. A note is the whole event — pitch, start, length and velocity — so
spending the word on one of its four fields would leave the event with nothing to
be called. 音调 is vaguer still: in ordinary Chinese it is as often a tone of
voice or the key a song is in as it is this number.

**Velocity**:
How hard a note is struck, as carried by the MIDI note event. Part of the Piece,
not of the Rig.
_中文_: 力度
_Avoid_: volume, loudness, dynamics; 音量、强弱

**Channel**:
One of the sixteen paths a MIDI file addresses a synthesiser through, counted
from 0 as the file counts them. What a **Program** is held by, and part of what
names a note — but not what a musician calls a part. That is the track.
_中文_: 通道
_Avoid_: port, bus; 声道、频道、声部

声道 is the audio one, left and right, which is downstream of everything this
project calls the Piece and belongs to the **Rig** if it belongs anywhere. 频道
is a television station. 声部 is already the track: a channel is not a part, and
the two come apart in both directions — three tracks may write one channel, and
one track may write three.

A channel is not printed on a note's line, and is printed on a Program's. The
rule behind both is one rule: say what is needed to point at the thing. Two notes
differing only in channel do not collide, so a note is pointed at without it; a
Program has no subject but the channel, so `program 40` on its own points at
nothing.

**Program**:
Which of a bank's instruments a channel is set to play, as the MIDI program
change event carries it: 0 to 127. Part of the Piece, not of the Rig — *which*
instrument is selected is in the file; what that selection sounds like is not.

Held by the channel, not by the track: a synthesiser's program is channel state,
so that is what `mid play` actually hands one. A Take that states no program is
described as stating none, never as being on program 0 — the two are
indistinguishable by ear on a General MIDI bank and are different Pieces.

Its **GM name** (GM 名) is what `mid` may print beside the number — `program 40
(GM violin)` — and the label is not decoration. A pitch name is a claim about the
file's own semantics; a program name is a claim about *which bank is loaded*, so
an unlabelled `violin` would be a Rig fact printed by a command that reports only
the Piece. The number is what the Piece says. See #12.
_中文_: 乐器号
_Avoid_: patch, instrument, voice, timbre; 音色、音色库、程序
_Avoid for the name_: program name, instrument name; 音色名

音色 is the first word on **Rig**'s avoid list and cannot be borrowed back here,
which is not a collision but this project's central line seen from one side: 音色
is what a sound is like, and a program number is which instrument the file asks
for. 号 is a mark written in the file, the same reason **Time signature** is 拍号
rather than 节拍.

Not `instrument`. MIDI carries an `InstrumentName` meta event — a piece of text a
track calls itself, which a passage inherits — so the word already names
something else in the same file. `patch` is reserved for the Rig by
`CHARTER.md`, where a patch is a sound preset. 程序 is the literal translation
and is a software word with no musical sense at all.

The name is `GM name` rather than `program name` for the same reason: MIDI's
`ProgramName` meta event is a different thing, and the gloss has to say whose
convention it is.

**Controller**:
One of the numbered controls a channel carries, as the MIDI control change event
holds it: a number naming the control, and a value 0 to 127 it is set to which
stays in force until something changes it. Part of the Piece, not of the Rig —
the expression a phrase is shaped with is in the file; what that shaping
*sounds* like is not. Held by the channel, as a Program is.

Not every control change message is one. Numbers 120 to 127 are channel mode
messages — All Notes Off and its neighbours — which ride on the same event type
and are instructions rather than settings: they happen and are over, and leave
no value in force to be reported, and reporting what is in force is the whole of
what this tool does with channel state (ADR-0007). A Take's own are preserved
untouched and named by nothing this tool prints, which is a hole left
deliberately — see #13.

Musically it is a curve — one crescendo is dozens of events — but the curve is
not a thing the file holds, and so is not a thing this project names. Nothing
here segments a stretch of events into a gesture: what a reader is told is which
value is in force and where the highest one falls, both read straight out of the
file. See #13.

Its **spec name** is what `mid` may print beside the number — `CC64 (damper
pedal on/off (sustain))`. Quoted from MIDI's own table and never improved on:
*sustain pedal* is what a pianist calls CC64 and *damper pedal on/off (sustain)*
is what the specification calls it, and a gloss that reached for the friendlier
word would have stopped being a quotation and started being this tool's opinion
of what the control is for. Some of a quotation may be dropped where a table
cell cannot hold it — a cross reference to another document, or what a control
used to be called — but no word of it is ever exchanged for a better one. The
`CC` prefix is where the attribution sits, which is why the name needs no `GM`
style label of its own — unlike a GM name, it does not depend on which bank is
loaded. The convention that a value of 64 or more means a switch is *on* comes
from that same table, and is a gloss rather than a reading the library makes.
_中文_: 控制器 (the name: 规范名)
_Avoid_: CC message, control change, automation, envelope; 控制信号、自动化、包络

自动化 and 包络 both belong to a DAW rather than to a MIDI file: an automation
lane is a thing an editor draws and an envelope is a thing a synthesiser has,
and neither is what a control change event is. 控制信号 is the transport reading
— a signal is what travels down a cable — where what is in the file is a written
value.

Not paired into 14-bit values. MIDI's fine controllers are a second event on a
second Controller number, and many exports write only the coarse one; combining
them is an inference about what an export meant, and a Take that round-trips at
the event level (ADR-0003) has no room to make it. Two Controllers stated are
two Controllers reported.
