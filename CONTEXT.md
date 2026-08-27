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
One mechanical change to a Take: add, delete, move or resize a note, change a
velocity, change a CC. Never carries musical intent — no "make it sadder".
An **Edit Set** is a batch of them, applied together (`edits.json`).
_中文_: 改动
_Avoid_: patch, mutation, operation; 补丁

`operation` is avoided for register, not for collision. The music-wins rule does
not reach it — its musical sense belongs to transformational theory, not to
notation — but an operation sounds large enough to carry intent, and an Edit is
deliberately flatter than that. So the discriminator key in `edits.json` is
`kind`, never `op`.

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
happens when the Take states none or states two, is ADR-0006.
_中文_: 小节
_Avoid_: measure, battuta (taken as the crate name); 拍、节

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
the whole of it, is ADR-0006.
_中文_: 拍号
_Avoid_: metre, meter, time sig; 节拍、拍子

Not `metre`, although it is the more musical-sounding of the two. In theory the
metre is how beats are organised and the time signature is the sign that states
it, and the sign is what a file can carry and this tool can read. A Take with no
time signature event does not lack a metre — the music has one, it simply is not
written down — so `metre` would make every refusal in ADR-0006 a false statement
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
are the truth; bars, beats and seconds are all derived views of them.
_中文_: 刻度
_Avoid_: time, position, offset

**Tempo**:
How fast a Take's ticks pass, as carried by the MIDI tempo meta event:
microseconds per quarter note, which `mid info` also reports as beats per minute.
Part of the Piece, not of the Rig. Italian wins here by the rule in `CHARTER.md`,
which names tempo as a case where it genuinely does.
_中文_: 速度
_Avoid_: speed, rhythm; 节奏、节拍

**Velocity**:
How hard a note is struck, as carried by the MIDI note event. Part of the Piece,
not of the Rig.
_中文_: 力度
_Avoid_: volume, loudness, dynamics; 音量、强弱
