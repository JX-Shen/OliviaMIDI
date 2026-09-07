#!/bin/sh
# Measure the baseline recorded in `baseline.md`. Read that file first; this
# script produces the numbers, and that file is where they mean something.
#
# Wall clock and peak resident memory for each command path, on a sparse
# workload and a dense one. `play` is measured through a fake `fluidsynth`, so
# what is timed is the passage `mid` prepares rather than somebody's soundfont.
#
# Run from the repository root. Everything it writes goes in one temporary
# directory and is removed on the way out.
set -eu

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cd "$root"
cargo build --release --quiet
mid="$root/target/release/mid"

python3 docs/performance/dense-take.py "$work/dense.mid"
sparse="fixtures/olivia.mid"
dense="$work/dense.mid"

# A fake synthesiser, so that `play` measures the preparation and not
# FluidSynth. It reads the file it is handed and returns; `mid` still writes
# the passage, still resolves the Rig, and still cleans up after itself.
mkdir -p "$work/bin"
printf '#!/bin/sh\nfor arg in "$@"; do last="$arg"; done\n/bin/cat "$last" > /dev/null\n' \
    > "$work/bin/fluidsynth"
chmod +x "$work/bin/fluidsynth"
printf 'not a soundfont' > "$work/rig.sf2"

printf '{}\n' > "$work/empty.json"
cat > "$work/empty-edits.json" <<'JSON'
{ "name": "no-op", "edits": [] }
JSON

echo "commit         $(git rev-parse --short HEAD)"
echo "mid --version  $("$mid" --version)"
echo "machine        $(uname -sm), $(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo unknown)"
echo "dense.mid      $(wc -c < "$dense" | tr -d ' ') bytes, sha256 $(shasum -a 256 "$dense" | cut -c1-16)"

run() {
    label=$1
    shift
    # `/usr/bin/time -l` is BSD's; the last two fields wanted are real seconds
    # and maximum resident set size in bytes.
    measured=$( { /usr/bin/time -l "$@" > /dev/null 2>"$work/time"; } 2>&1 || true )
    real=$(awk '/real/ { print $1 }' "$work/time" | head -1)
    peak=$(awk '/maximum resident set size/ { printf "%.1f", $1 / 1048576 }' "$work/time")
    printf '%-26s %10s %10s\n' "$label" "$real" "$peak"
}

if [ "${1:-}" != "--scaling" ]; then
    echo
    printf '%-26s %10s %10s\n' 'path' 'wall (s)' 'peak (MB)'
fi

if [ "${1:-}" = "--scaling" ]; then
    # Where the answer to "does this scale" is. Two dials, turned one at a
    # time: the notes alone, then the control changes alone, at two sizes each.
    # A path that doubles its work when its input doubles is linear; one that
    # takes four times as long is not, and the pair of runs says which dial it
    # is on.
    echo
    printf '%-40s %10s %10s\n' 'scaling probe' 'inspect' 'diff'
    for bars in 200 400 800; do
        for shape in notes controls; do
            if [ "$shape" = notes ]; then
                DENSE_BARS=$bars DENSE_CONTROL_EVERY=100000000 \
                    python3 docs/performance/dense-take.py "$work/probe.mid"
            else
                DENSE_BARS=$bars DENSE_NOTES_PER_BAR=0 \
                    python3 docs/performance/dense-take.py "$work/probe.mid"
            fi
            inspect=$( { /usr/bin/time -p "$mid" inspect "$work/probe.mid" >/dev/null; } 2>&1 \
                | awk '/real/ { print $2 }' )
            compare=$( { /usr/bin/time -p "$mid" diff "$work/probe.mid" "$work/probe.mid" >/dev/null; } 2>&1 \
                | awk '/real/ { print $2 }' )
            printf '%-40s %10s %10s\n' "$bars bars, $shape only" "$inspect" "$compare"
        done
    done
    exit 0
fi

for pair in "sparse:$sparse" "dense:$dense"; do
    kind=${pair%%:*}
    take=${pair#*:}
    run "$kind inspect"       "$mid" inspect "$take"
    run "$kind inspect --json" "$mid" inspect "$take" --json
    run "$kind apply (no-op)" "$mid" apply "$take" "$work/empty-edits.json" --output "$work/out-$kind.mid"
    run "$kind diff (self)"   "$mid" diff "$take" "$take"
    PATH="$work/bin:$PATH" BATTUTA_SOUNDFONT="$work/rig.sf2" \
        run "$kind play --bars 1:4" "$mid" play "$take" --bars 1:4
done
