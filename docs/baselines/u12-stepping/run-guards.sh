#!/usr/bin/env bash
# **The guard proof for U12's stepping plane (2026-10-13, thirty-fourth pass).**
#
# Each new guard is perturbed **in its own subject** — the declared rain, the declared fraction a pit keeps,
# the one-hop transfer itself, the snapshot the pass is computed from, the water the stepped read carries, and
# the tick count the state advances by — and then the bar is run to show it **bites, and by name**. Every
# touched file is restored and its digest checked before the next case, and the script restores through a
# `trap`, so an interrupt cannot leave the tree perturbed.
#
# Usage: `bash docs/baselines/u12-stepping/run-guards.sh` from anywhere. Exit 0 only when every case bit by
# name and every file came back byte-exact.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE="$ROOT/clean-room"
WORK="$(mktemp -d)"
FILES=(src/automaton.rs src/being.rs src/lib.rs src/evidence.rs tests/u12_stepping.rs)

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
    # shellcheck disable=SC2086 — the selector is meant to split: `--lib`, or `--test u12_stepping`.
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

# 1. The declared rain: with no rain at all a dry pit never wets and the plane never moves towards its fixed
#    point — the subject is `RAIN_UM_PER_TICK`.
run_case "the declared rain" src/automaton.rs \
    'pub const RAIN_UM_PER_TICK: i64 = 200;' 'pub const RAIN_UM_PER_TICK: i64 = 0;' \
    "--lib" "automaton::tests::rain_wets"

# 2. The declared fraction a pit keeps: a pit that keeps everything never comes to rest — the subject is
#    `PIT_KEEP_PERCENT`.
run_case "the fraction a pit keeps" src/automaton.rs \
    'pub const PIT_KEEP_PERCENT: i64 = 90;' 'pub const PIT_KEEP_PERCENT: i64 = 100;' \
    "--lib" "automaton::tests::rain_wets"

# 3. The one-hop transfer itself: half a column's water moved downhill leaves the other half standing where it
#    was, so the per-column law and the conservation identity both go — the subject is the routing write.
run_case "the one-hop transfer" src/automaton.rs \
    'moved[to] += own;' 'moved[to] += own / 2;' \
    "--lib" "automaton::tests::one_tick_is_one_synchronous_hop"

# 4. The pit's own thinning: a pit that keeps nothing keeps no water, so the dry pit never wets and the law's
#    own fixed point never arrives — the subject is what a pit is allowed to hold.
run_case "the pit's own thinning" src/automaton.rs \
    'None => moved[cell] -= own * (100 - PIT_KEEP_PERCENT) / 100,' \
    'None => moved[cell] = 0,' \
    "--lib" "automaton::tests::rain_wets"

# 5. The water the stepped read carries: a read built over a plane of zeros is not the state the law produced,
#    so the being reads a world nobody stepped — the subject is `Automaton::read`.
run_case "the stepped read's own water" src/automaton.rs \
    '        base.stepped_from(&self.depth_um)' \
    '        base.stepped_from(&vec![0_i64; self.depth_um.len()])' \
    "--test" "u12_stepping"

# 6. The tick count the state advances by: a state that counts two ticks per pass is not the plane at the
#    tick the run prints — the subject is the counter itself.
run_case "the tick count" src/automaton.rs \
    '        self.ticks += 1;' '        self.ticks += 2;' \
    "--lib" "automaton::tests::the_law_is_a_pure_function"

# 7. The state the law starts from: a plane of zeros is not the read's own water, so the generator's filled
#    surface is thrown away instead of routed — the subject is `Automaton::over`.
run_case "the state the law starts from" src/automaton.rs \
    '            depth_um: (0..world.cells()).map(|cell| world.depth_um(cell)).collect(),' \
    '            depth_um: vec![0_i64; world.cells()],' \
    "--test" "u12_stepping"

# 8. The being's next step under a stepped plane is the record's own bar: with the being driven over the plane
#    it was placed in, the plane's own change cannot move its step — the subject is the one call site that hands
#    the being the law's state.
run_case "the being reads the law's state" tests/u12_stepping.rs \
    'drive(&stepped, &scratch("stepped"), start, TICKS)' \
    'drive(&world, &scratch("stepped"), start, TICKS)' \
    "--test" "u12_stepping"

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
