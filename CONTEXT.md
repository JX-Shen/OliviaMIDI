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

**Tick**:
The unit of musical time inside a MIDI file, relative to the file's PPQ. Ticks
are the truth; bars, beats and seconds are all derived views of them.
_中文_: 刻度
_Avoid_: time, position, offset

**Velocity**:
How hard a note is struck, as carried by the MIDI note event. Part of the Piece,
not of the Rig.
_中文_: 力度
_Avoid_: volume, loudness, dynamics; 音量、强弱
