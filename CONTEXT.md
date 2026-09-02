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
change a velocity, change a CC. Never carries musical intent — no "make it sadder".
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
are the truth; bars, beats and seconds are all derived views of them.
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
`time` in a MIDI tool is read as a moment, and a moment is what a Tick is. The
word was a synonym; a Position is a reading.

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
