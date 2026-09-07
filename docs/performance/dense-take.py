#!/usr/bin/env python3
"""Write a dense Take, deterministically, for the baseline in `baseline.md`.

Generated rather than committed. The file is about a megabyte and its only
content is repetition; what has to survive is the recipe, and the recipe is
short enough to read. Nothing here is random, so two runs on two machines
produce the same bytes — `shasum` is in `measure.sh` for exactly that.

The shape is a densely automated DAW export, which is the workload this
project's readings are most exposed to: `inspect` and `diff` both walk every
control change to answer a question about what is in force, so a Take whose
control changes outnumber its notes seven to one is where a reading that
scales badly says so.
"""

import os
import struct
import sys

# The dense workload, and the four dials a scaling probe turns. Overridable
# from the environment so that `measure.sh --scaling` can ask for the same file
# at four sizes without a second generator, and so that a run which changed one
# of them says which.
PPQ = 480
BEATS = 4
TRACKS = int(os.environ.get("DENSE_TRACKS", 4))
BARS = int(os.environ.get("DENSE_BARS", 200))
NOTES_PER_BAR = int(os.environ.get("DENSE_NOTES_PER_BAR", 8))
# One control change every 60 Ticks on every voice track: an eighth note at
# this PPQ, which is a fader recorded rather than a value typed in. Raise it to
# silence the curves and leave the notes, which is how the two are told apart.
CONTROL_EVERY = int(os.environ.get("DENSE_CONTROL_EVERY", 60))


def varint(value):
    out = [value & 0x7F]
    value >>= 7
    while value:
        out.append((value & 0x7F) | 0x80)
        value >>= 7
    return bytes(reversed(out))


def event(delta, payload):
    return varint(delta) + payload


def chunk(name, body):
    return name + struct.pack(">I", len(body)) + body


def track(events):
    """Absolute Ticks in, delta times out — the one rule everything here obeys."""
    events.sort(key=lambda pair: pair[0])
    out = b""
    previous = 0
    for tick, payload in events:
        out += event(tick - previous, payload)
        previous = tick
    out += event(0, b"\xff\x2f\x00")
    return chunk(b"MTrk", out)


def conductor():
    return track(
        [
            (0, b"\xff\x51\x03" + struct.pack(">I", 500000)[1:]),
            (0, b"\xff\x58\x04" + bytes([BEATS, 2, 24, 8])),
        ]
    )


def voice(index):
    channel = index - 1
    bar_ticks = PPQ * BEATS
    events = []
    for bar in range(BARS):
        base = bar * bar_ticks
        for note in range(NOTES_PER_BAR):
            start = base + note * (bar_ticks // NOTES_PER_BAR)
            pitch = 48 + ((bar * NOTES_PER_BAR + note + index * 7) % 25)
            events.append((start, bytes([0x90 | channel, pitch, 64])))
            events.append((start + 200, bytes([0x80 | channel, pitch, 0])))
    for tick in range(0, BARS * bar_ticks, CONTROL_EVERY):
        # A curve that reverses, because a reading built for a monotone rise is
        # a reading that has not met a fader.
        value = 64 + int(63 * ((tick // CONTROL_EVERY) % 40 - 20) / 20)
        events.append((tick, bytes([0xB0 | channel, 11, max(0, min(127, value))])))
    events.append((0, bytes([0xC0 | channel, 40 + index])))
    return track(events)


def main(path):
    header = chunk(b"MThd", struct.pack(">HHH", 1, TRACKS + 1, PPQ))
    body = header + conductor() + b"".join(voice(index) for index in range(1, TRACKS + 1))
    with open(path, "wb") as handle:
        handle.write(body)


if __name__ == "__main__":
    main(sys.argv[1])
