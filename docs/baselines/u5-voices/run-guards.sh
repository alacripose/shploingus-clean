#!/usr/bin/env bash
# **The guard proof for U5's voices (2026-10-12, nineteenth pass).**
#
# Each new guard is perturbed **in its own subject** — a refusal's own name, the ladder the drift is read
# against, the rule that reads it, the normalisation the levels are bounded by, the calibration the digest
# is pinned to, and the table's own band arithmetic — and then the bar is run to show it **bites, and by
# name**. Every touched file is restored and its digest checked before the next case, and the script
# restores through a `trap`, so an interrupt cannot leave the tree perturbed.
#
# Usage: `bash docs/baselines/u5-voices/run-guards.sh` from anywhere. Exit 0 only when every case bit by
# name and every file came back byte-exact. (The last pass's harness was untracked, trap-less and
# line-anchored; this one is anchored on the value it perturbs, restores through a trap, and aggregates
# its own exit status.)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE="$ROOT/clean-room"
WORK="$(mktemp -d)"
FILES=(src/light.rs src/feeling.rs src/audio.rs tests/u5_measure.rs)

snapshot() {
    for file in "${FILES[@]}"; do
        mkdir -p "$WORK/$(dirname "$file")"
        cp "$CRATE/$file" "$WORK/$file"
    done
    (cd "$CRATE" && sha256sum "${FILES[@]}") > "$WORK/digests.txt"
}

restore() {
    for file in "${FILES[@]}"; do
        if [ -f "$WORK/$file" ]; then cp "$WORK/$file" "$CRATE/$file"; fi
    done
}
trap 'restore; rm -rf "$WORK"' EXIT

# A perturbation is one exact string, replaced once — no line numbers, and a match that is not unique is a
# failed case rather than a silent no-op.
perturb() {
    python - "$1" "$2" "$3" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(path, "r", encoding="utf-8", newline="").read()
count = text.count(old)
if count != 1:
    raise SystemExit(f"the perturbation matched {count} times, not once: {old!r}")
open(path, "w", encoding="utf-8", newline="").write(text.replace(old, new))
PY
}

FAILED=0
run_case() {
    local name="$1" file="$2" old="$3" new="$4" selector="$5" filter="$6"
    echo "== $name"
    echo "   subject: $file :: $old"
    if ! perturb "$CRATE/$file" "$old" "$new"; then
        echo "   RESULT: the perturbation did not apply"
        FAILED=1
        restore
        return
    fi
    local out status
    set +e
    # shellcheck disable=SC2086 — the selector is meant to split: `--lib`, or `--test u5_measure`.
    out="$(cd "$CRATE" && cargo test --offline -j 2 $selector "$filter" 2>&1)"
    status=$?
    set -e
    if [ "$status" -eq 0 ]; then
        echo "   RESULT: BIT NOTHING — the bar passed a perturbed build"
        FAILED=1
    else
        printf '   RESULT: bit by name — %s\n' \
            "$(printf '%s' "$out" | grep -m1 -E 'assertion .* failed|panicked at|^error' | sed 's/^[[:space:]]*//')"
    fi
    restore
    if ! (cd "$CRATE" && sha256sum -c "$WORK/digests.txt" > /dev/null 2>&1); then
        echo "   RESULT: THE TREE DID NOT COME BACK BYTE-EXACT"
        FAILED=1
    fi
}

THE_VOICE_TEST="the_voice_renders_a_bounded_buffer_that_the_wheel_and_the_weather_both_move"
THE_REFUSALS_TEST="a_feeling_that_was_never_declared_is_refused_by_name_rather_than_voiced"

snapshot
echo "the tree before any perturbation:"
cat "$WORK/digests.txt"
echo

run_case "feeling_absent is refused, and by its own name" \
    "src/audio.rs" \
    $'RefusalKind::FeelingAbsent,\n                "no feeling-wheel reading' \
    $'RefusalKind::FeelingUncalibrated,\n                "no feeling-wheel reading' \
    "--test u5_measure" "$THE_REFUSALS_TEST"

run_case "a word the wheel does not declare is refused, and by its own name" \
    "src/feeling.rs" \
    $'RefusalKind::FeelingUndeclared,\n                format!(' \
    $'RefusalKind::FeelingAbsent,\n                format!(' \
    "--test u5_measure" "$THE_REFUSALS_TEST"

run_case "the ladder is a guard, not a formality: one band wider accepts the wheel's own example" \
    "src/feeling.rs" \
    '        (-(band as i32), CHANNEL_COUNT as i32 - 1 - band as i32)' \
    '        (-(band as i32) - 1, CHANNEL_COUNT as i32 - 1 - band as i32)' \
    "--test u5_measure" "$THE_REFUSALS_TEST"

run_case "a blend of two families is refused, and by its own name" \
    "src/feeling.rs" \
    $'                    RefusalKind::FeelingContradictory,\n                    format!(' \
    $'                    RefusalKind::FeelingUncalibrated,\n                    format!(' \
    "--test u5_measure" "$THE_REFUSALS_TEST"

run_case "the levels are bounded by the signature's own normalisation" \
    "src/feeling.rs" \
    '                *weight /= total;' \
    '                *weight *= 1.0;' \
    "--lib" "audio::tests::the_voice_mixes_a_bed_and_a_rain_and_its_samples_stay_bounded"

run_case "the digest is the buffer's identity: a moved band tone moves it" \
    "src/audio.rs" \
    'pub const BAND_OCTAVE_STEPS: f64 = 0.25;' \
    'pub const BAND_OCTAVE_STEPS: f64 = 0.5;' \
    "--test u5_measure" "$THE_VOICE_TEST"

run_case "the table's own band arithmetic reads its bands half-open, lower end inside" \
    "src/light.rs" \
    'if !nm.is_finite() || nm < VISIBLE_LO_NM || nm >= VISIBLE_HI_NM {' \
    'if !nm.is_finite() || nm <= VISIBLE_LO_NM || nm >= VISIBLE_HI_NM {' \
    "--lib" "light::tests::a_wavelength_lands_in_exactly_one_band_and_outside_the_span_in_none"

echo
echo "the tree after every case:"
(cd "$CRATE" && sha256sum "${FILES[@]}")
if [ "$FAILED" -ne 0 ]; then
    echo "GUARD PROOF FAILED"
    exit 1
fi
echo "GUARD PROOF OK: every guard bit by name, and every touched file came back byte-exact"
