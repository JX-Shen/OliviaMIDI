# Sourced fixtures

Everything in `fixtures/external/` was produced by somebody else, on software
this project does not run. Every file there has an entry below, and the entry is
what makes the bytes checkable: a reader who wants to know that a fixture is
what this repository says it is fetches the source, takes its digest, and
compares. Nothing here is inferred from the bytes except where it says so.

A file in `fixtures/external/` with no entry below, or an entry naming no file,
is a fault, and `tests/fixtures.rs` is what says so. The digests are written
once, here, and that test reads them out of this file rather than out of a
second list beside it. The rest of `fixtures/` is this project's own work and is
not described here.

```
shasum -a 256 fixtures/external/groove-drummer8-funk.mid
```

Licences are recorded as the statement that was read and the date it was read,
not only as a name. A licence name is a copy of somebody else's page and can go
stale: the search result for one of the corpora below said CC BY 4.0 on the day
this file was written, and the repository's own `LICENSE.txt` said
CC BY-NC-SA 4.0.

`fixtures/external/` is not shipped in the crate. `CHARTER.md`'s *Distribution*
section says why.

---

## mutopia-beethoven-op61.mid

Beethoven, Concerto for Violin and Orchestra in D major, Op. 61, first
movement, typeset in LilyPond for the Mutopia Project from Breitkopf and
Härtel (1862–1865).

- Source: <https://www.mutopiaproject.org/ftp/BeethovenLv/O61/violin_concerto_1/violin_concerto_1.mid>
- SHA-256: `69fd077ebe5536528f1a6b92c51492b5b740f39bfbe77afe6389410bcb6401b1`
- Licence: Public Domain, stated per piece in the Mutopia catalogue entry for
  this movement. Read 2026-09-14 at
  <https://www.mutopiaproject.org/cgibin/make-table.cgi?searchingfor=Concerto&Composer=BeethovenLv>
- Production path: notation software. The file states its own producer in a
  text meta event on the control track: `GNU LilyPond 2.10.25`.

`mid info`: format 1, 14 tracks, ppq 384, 102.0000510000255 bpm, 4/4, 820608
ticks. `mid inspect`: 13 programs, 3055 stated Controllers (all CC7), 17380
notes.

This is the widest file here — fourteen tracks and a tempo the format cannot
spell exactly.

## mutopia-bach-bwv208.mid

J. S. Bach, *Schafe können sicher weiden*, from BWV 208, arranged for two
flutes, soprano and basso continuo, typeset in LilyPond for the Mutopia
Project.

- Source: <https://www.mutopiaproject.org/ftp/BachJS/BWV208/Sheep/Sheep.mid>
- SHA-256: `671d16499293e7911cbd717d68fbcc09f122d1195fb2349924c7b9409c4eec1f`
- Licence: Public Domain, stated per piece in the Mutopia catalogue entry for
  this piece. Read 2026-09-14 at
  <https://www.mutopiaproject.org/cgibin/make-table.cgi?searchingfor=Sheep>
- Production path: notation software, LilyPond, same corpus as the Beethoven.

`mid info`: format 1, 6 tracks, ppq 384, 60 bpm, 4/4, 61440 ticks.
`mid inspect`: 4 programs, 18 stated Controllers (all CC7), 1045 notes.

The same production path as the Beethoven at a sixteenth of the size, for a
test that should not read seventeen thousand notes to prove its point.

## cc0-midis-overture.mid

`overture-2021.mid`, from a collection of MIDI files its author placed in the
public domain. Twelve named tracks — `trumpets`, `french horns`, `violin`,
`viola` and so on.

- Source: <https://raw.githubusercontent.com/m-malandro/CC0-midis/HEAD/midis/overture-2021.mid>
- Repository: <https://github.com/m-malandro/CC0-midis>
- SHA-256: `40859ffbb9103157aaccf3b9b9e493ffce91a74a67198491dc232861615d2eba`
- Licence: CC0 1.0 Universal, as `LICENSE` in the repository above.
  Read 2026-09-14 at
  <https://raw.githubusercontent.com/m-malandro/CC0-midis/HEAD/LICENSE>
- Production path: **not recorded.** The repository does not say what wrote
  these files, and the bytes carry no producer meta event — only track names.
  A sequenced multi-track arrangement is what the file looks like; which
  software made it is unknown and is left unknown.

`mid info`: format 1, 12 tracks, ppq 960, 160 bpm, 4/4, 464640 ticks.
`mid inspect`: 11 programs, 2201 stated Controllers (CC1, CC10, CC64), 3867
notes.

The densest Controller writing of the four, and the only one here that uses
CC64 and CC10.

## groove-drummer8-funk.mid

`drummer8/session2/2_funk_92_beat_4-4.mid` from the Groove MIDI Dataset: a
professional drummer playing funk at 92 bpm to a click on a Roland TD-11
electronic kit, captured live. 367.46 seconds; the dataset's `train` split.

- Source: <https://storage.googleapis.com/magentadata/datasets/groove/groove-v1.0.0-midionly.zip>,
  member `groove/drummer8/session2/2_funk_92_beat_4-4.mid`. The digest below is
  the member's, not the archive's: unzip first, then take it.
- SHA-256: `5e30e71033c9b0a8d907fc2615ea00755f5bc6f97c39d7dc893e3ff43b9c3ceb`
- Licence: CC BY 4.0, as `LICENSE` in the archive. Read 2026-09-14 at
  <https://magenta.withgoogle.com/datasets/groove>
- Attribution, as the licence requires: Groove MIDI Dataset, Jon Gillick,
  Adam Roberts, Jesse Engel, Douglas Eck and David Bamman, Google Magenta,
  2019, <https://magenta.withgoogle.com/datasets/groove>. The file is carried
  here unmodified.
- Production path: live performance capture. Per-file metadata — drummer,
  session, style, bpm, beat type, time signature, split — is in the archive's
  `info.csv`.

`mid info`: **format 0**, 1 track, ppq 480, 91.99998773333498 bpm, 4/4, 270450
ticks. `mid inspect`: 1 program, 1964 stated Controllers (all CC4, the hi-hat
pedal), 3835 notes.

The only format 0 file this repository has. Every other Take here, sourced or
made, is format 1.
