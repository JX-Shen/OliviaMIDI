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
case "${1:-}" in
    ""|--scaling) ;;
    *) echo "usage: $0 [--scaling]" >&2; exit 2 ;;
esac
[ "$#" -le 1 ] || { echo "usage: $0 [--scaling]" >&2; exit 2; }

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cd "$root"
export LC_ALL=C
fail() { echo "$*" >&2; exit 1; }
hash() { shasum -a 256 "$1" | awk '{print $1}'; }
command_line() {
    printf 'command'
    for arg do
        printf " '%s'" "$(printf '%s' "$arg" | sed "s/'/'\\\\''/g")"
    done
    printf '\n'
}
workload() {
    printf 'workload %s: %s bytes, sha256 %s\n' "$1" \
        "$(wc -c < "$2" | tr -d ' ')" "$(hash "$2")"
}
generate() {
    command_line env DENSE_TRACKS=4 "DENSE_BARS=$1" \
        "DENSE_NOTES_PER_BAR=$2" "DENSE_CONTROL_EVERY=$3" \
        python3 docs/performance/dense-take.py "$4"
    DENSE_TRACKS=4 DENSE_BARS=$1 DENSE_NOTES_PER_BAR=$2 DENSE_CONTROL_EVERY=$3 \
        python3 docs/performance/dense-take.py "$4"
}

printf 'date UTC       %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
printf 'commit         %s\n' "$(git rev-parse HEAD)"
printf 'tree status (empty means clean):\n'
git status --short
printf 'tracked changes sha256 %s\n' "$(git diff HEAD --binary | shasum -a 256 | awk '{print $1}')"
printf 'machine        %s\n' "$(uname -srm)"
printf 'CPU            %s\n' "$(sysctl -n machdep.cpu.brand_string)"
sw_vers
cargo --version
rustc -Vv
printf 'measure.sh sha256 %s\n' "$(hash docs/performance/measure.sh)"
printf 'generator sha256  %s\n' "$(hash docs/performance/dense-take.py)"
printf 'Cargo.lock sha256 %s\n' "$(hash Cargo.lock)"

# A fresh, explicit build directory prevents an old or environment-selected
# binary from being measured. Build time is outside every measurement.
command_line cargo build --release --locked --quiet --target-dir "$work/build"
cargo build --release --locked --quiet --target-dir "$work/build"
mid="$work/build/release/mid"
"$mid" --version
printf 'binary sha256  %s\n' "$(hash "$mid")"
"$mid" apply --help > "$work/apply-help"

sparse="fixtures/olivia.mid"
dense="$work/dense.mid"

# A fake synthesiser, so that `play` measures the preparation and not
# FluidSynth. It reads the file it is handed and returns; `mid` still writes
# the passage, still resolves the Rig, and still cleans up after itself.
mkdir -p "$work/bin"
cat > "$work/bin/fluidsynth" <<'SH'
#!/bin/sh
set -eu
for arg do last=$arg; done
test -s "${last:?missing MIDI argument}"
/bin/cat "$last" > /dev/null
printf '%s\n' "$last" >> "${MID_MEASURE_SYNTH_LOG:?missing invocation log}"
SH
chmod +x "$work/bin/fluidsynth"
printf 'not a soundfont' > "$work/rig.sf2"

printf '{"edits":[]}\n' > "$work/empty-edits.json"
workload 'no-op Edit Set' "$work/empty-edits.json"
export MID_MEASURE_SYNTH_LOG="$work/synth.log"
export MID_MEASURE_STDERR="$work/stderr"

measure() {
    command_line "$@"
    # `/usr/bin/time -l` is BSD's; the last two fields wanted are real seconds
    # and maximum resident set size in bytes.
    # Keep child stderr separate from timing data and preserve the exit status.
    if /usr/bin/time -l /bin/sh -c 'exec "$@" 2>"$MID_MEASURE_STDERR"' sh "$@" \
        > /dev/null 2> "$work/time"; then
        :
    else
        status=$?
        printf 'measurement failed (exit %s)\n' "$status" >&2
        cat "$work/stderr" "$work/time" >&2
        exit "$status"
    fi
    real=$(awk '$2 == "real" && $1 ~ /^[0-9]+([.][0-9]+)?$/ {print $1}' "$work/time")
    peak=$(awk '/maximum resident set size/ && $1 ~ /^[0-9]+$/ {printf "%.2f", $1 / 1048576}' "$work/time")
    [ -n "$real" ] && [ -n "$peak" ] || {
        cat "$work/time" >&2
        fail 'measurement lacks wall time or peak resident memory'
    }
}
result() { printf 'result %-38s %10s s %10s MiB\n' "$1" "$real" "$peak"; }
run() {
    label=$1
    shift
    measure "$@"
    result "$label"
}

if [ "${1:-}" = "--scaling" ]; then
    # Vary notes and controls separately. The notes probe retains one CC per
    # voice at Tick 0; it is not CC-free. Every path uses measure's status check.
    for bars in 200 400 800; do
        for shape in notes controls; do
            if [ "$shape" = notes ]; then
                generate "$bars" 8 100000000 "$work/probe.mid"
            else
                generate "$bars" 0 60 "$work/probe.mid"
            fi
            workload "$bars bars, $shape varied" "$work/probe.mid"
            run "$bars bars, $shape varied, inspect" "$mid" inspect "$work/probe.mid"
            run "$bars bars, $shape varied, diff" "$mid" diff "$work/probe.mid" "$work/probe.mid"
        done
    done
    exit 0
fi

generate 200 8 60 "$dense"
for pair in "sparse:$sparse" "dense:$dense"; do
    kind=${pair%%:*}
    take=${pair#*:}
    workload "$kind" "$take"
    run "$kind inspect"       "$mid" inspect "$take"
    run "$kind inspect --json" "$mid" inspect "$take" --json
    measure "$mid" apply "$take" "$work/empty-edits.json" --output "$work/out-$kind.mid"
    [ -s "$work/out-$kind.mid" ] || fail 'no-op apply produced no Take'
    if ! "$mid" info "$work/out-$kind.mid" --json > /dev/null 2> "$work/validation"; then
        cat "$work/validation" >&2
        fail 'no-op apply produced an unreadable Take'
    fi
    result "$kind apply (no-op)"
    run "$kind diff (self)"   "$mid" diff "$take" "$take"
    rm -f "$MID_MEASURE_SYNTH_LOG"
    printf 'play environment: PATH prefix=%s BATTUTA_SOUNDFONT=%s\n' "$work/bin" "$work/rig.sf2"
    PATH="$work/bin:$PATH" BATTUTA_SOUNDFONT="$work/rig.sf2" \
        measure "$mid" play "$take" --bars 1:4
    [ -s "$MID_MEASURE_SYNTH_LOG" ] || fail 'play did not invoke the fake synthesiser'
    result "$kind play --bars 1:4"
done
