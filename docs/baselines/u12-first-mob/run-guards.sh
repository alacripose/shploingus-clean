#!/usr/bin/env bash
# **The guard proof for U12's first mob (2026-10-13, thirty-third pass).**
#
# Each new guard is perturbed **in its own subject** — the world's own water reading, the game's own flow
# step, the being's reach, the charter's floor, the stage comparison, the recording of a denial, the
# unsouled refusal, and the two refusal names themselves — and then the bar is run to show it **bites, and by
# name**. Every touched file is restored and its digest checked before the next case, and the script restores
# through a `trap`, so an interrupt cannot leave the tree perturbed.
#
# Usage: `bash docs/baselines/u12-first-mob/run-guards.sh` from anywhere. Exit 0 only when every case bit by
# name and every file came back byte-exact.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE="$ROOT/clean-room"
WORK="$(mktemp -d)"
FILES=(src/being.rs src/refusal.rs src/chain.rs src/evidence.rs tests/u12_being.rs)

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
    local out status log
    log="$WORK/$(printf '%s' "$name" | tr ' ()/' '____').test.log"
    set +e
    # shellcheck disable=SC2086 — the selector is meant to split: `--lib`, or `--test u12_being`.
    out="$(cd "$CRATE" && cargo test --offline -j 2 $selector "$filter" 2>&1)"
    status=$?
    set -e
    printf '%s\n' "$out" > "$log"
    if [ "$status" -eq 0 ]; then
        echo "   RESULT: BIT NOTHING — the bar passed a perturbed build"
        FAILED=1
    else
        # The first lines that name the fault: the assertion, the panic, or the compiler's error.
        printf '%s' "$out" | grep -E 'assertion .* failed|panicked at|^error' | head -3 | sed 's/^/   bit: /'
    fi
    restore
    if ! (cd "$CRATE" && sha256sum -c "$WORK/digests.txt" > /dev/null 2>&1); then
        echo "   RESULT: THE TREE DID NOT COME BACK BYTE-EXACT"
        FAILED=1
    fi
}

snapshot

# 1. The world's own water reading: a depth that is always zero stands no water anywhere, so the being's own
#    drive never starts — the subject is `WorldRead::depth`.
run_case "the world's own water read" src/being.rs \
    '(surface - ground).max(0.0)' '0.0_f64' \
    "--lib" "being::tests::the_world_s_own_law"

# 2. The game's own flow step: a walk that stays on its cell never reaches water, and the being refuses for
#    reach instead of drinking — the subject is the step along `d8_receivers`.
run_case "the game's own flow step" src/being.rs \
    'Some(next) => Act::Walk { from: cell, to: next },' 'Some(_) => Act::Walk { from: cell, to: cell },' \
    "--lib" "being::tests::the_first_mob"

# 3. The being's reach: a reach that never runs out turns the unmet need into a walk forever, so the stranded
#    being is not refused — the subject is the reach comparison.
run_case "the being's own reach" src/being.rs \
    '} else if self.steps() >= self.reach {' '} else if false {' \
    "--test" "u12_being"

# 4. The refusal's own name: the unmet need is recorded under another kind, so the test that reads the name
#    bites — the subject is `water_unreachable` itself.
run_case "the refusal's own name (water_unreachable)" src/being.rs \
    'RefusalKind::WaterUnreachable,
                format!(
                    "the being walked its whole reach' \
    'RefusalKind::StageBelowFloor,
                format!(
                    "the being walked its whole reach' \
    "--test" "u12_being"

# 5. The floor's own name: the act below the charter's floor is recorded under the other new kind — the subject
#    is `stage_below_floor` itself.
run_case "the refusal's own name (stage_below_floor)" src/being.rs \
    'RefusalKind::StageBelowFloor,' 'RefusalKind::WaterUnreachable,' \
    "--test" "u12_being"

# 6. The charter's floor: a floor the being's own act cannot reach refuses every act — the subject is the floor
#    the laya question declares before its answer exists.
run_case "the charter's declared floor" src/being.rs \
    'pub const ACT_FLOOR: &str = "verified-internal";' 'pub const ACT_FLOOR: &str = "approved";' \
    "--test" "u12_being"

# 7. The stage comparison: a comparison that never refuses takes an act the instrument read below the floor —
#    the subject is the guard the laya answer passes through.
run_case "the stage comparison" src/being.rs \
    'if reading != self.organisation.floor() {' 'if false {' \
    "--test" "u12_being"

# 8. The recording of a denial: a refusal appended under an act's own kind is not a recorded refusal — the
#    subject is `record_refusal`'s own event kind.
run_case "the denial recorded" src/being.rs \
    'Namespace::Record, EventKind::RefusalRecorded, payload' \
    'Namespace::Record, EventKind::BeingActed, payload' \
    "--test" "u12_being"

# 9. The unsouled refusal: a being with an empty claim would be built rather than refused — the subject is the
#    soul check in the one constructor.
run_case "the unsouled refusal" src/being.rs \
    'if body.trim().is_empty() {' \
    'if false && body.trim().is_empty() {' \
    "--lib" "being::tests::an_unsouled"

echo
if [ "$FAILED" -eq 0 ]; then
    echo "every guard bit by name, and every touched file came back byte-exact:"
    cat "$WORK/digests.txt"
    for log in "$WORK"/*.test.log; do
        echo "   kept: $(basename "$log")"
        cp "$log" "$(dirname "${BASH_SOURCE[0]}")/"
    done
else
    echo "A GUARD DID NOT BITE, OR THE TREE DID NOT COME BACK"
fi
exit "$FAILED"
