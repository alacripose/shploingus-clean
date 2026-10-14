#!/usr/bin/env bash
# **The guard proof for the water law on the cubic world's own cells (2026-10-13, plan §38; `LEDGER.md` C-190).**
#
# Each guard this pass added or moved is perturbed **in its own subject** and the bar is run to show it
# **bites, and by name**: the law's own rain, the pit's declared fraction, the synchronous pass (a routing
# column keeps nothing of its own), the state's identity being the cells' and not the tick count, the tick
# count itself, the graph being the cells' own surfaces, the pour never writing a column's ground, the cell's
# own cap (`CELL_HOLD_UM` — **named unexercised by §37 and exercised here, because it now has a subject**),
# the mapping that keeps a cell outside the patch out of the patch's own columns (the defect this pass found
# by running: the sweeps indexed past the fields they were handed), the reading being the declared voxel of
# the generator's elevation at every column (the check §37's own question turns on), and the being's walk —
# its own water test and its own reach. Every touched file is restored and its digest checked before the next
# case, and the script restores through a `trap`, so an interrupt cannot leave the tree perturbed.
#
# **The build is restored too**, as §37's harness learned: `cargo test` rebuilds the *binary* as well as the
# test harnesses, so a case that runs while a source is perturbed leaves a perturbed build behind. Each case
# ends by rebuilding from the restored sources.
#
# **The last check is the run's own determinism, not a dated file.** The script snapshots the binary's own
# output at the start, before any perturbation, and requires the rebuilt binary to reproduce *that*, byte for
# byte — and then reports, without gating on it, whether this pass's dated capture `final-stdout.txt` still
# matches.
#
# The case logs are kept in the scratch directory this script makes (outside the tree) and are **not** copied
# into it: they carry the run's own test-thread PIDs and cargo timings, so they cannot be reproduced
# byte-for-byte. This script's own output is the proof.
#
# Usage: `bash docs/baselines/u2-water-law-on-the-cells/run-guards.sh` from anywhere. Exit 0 only when every
# case bit by name and every file came back byte-exact.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE="$ROOT/clean-room"
WORK="$(mktemp -d)"
BASELINE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$CRATE/target/debug/shploingus-clean-room"
FILES=(src/automaton.rs src/chunk.rs src/being.rs src/evidence.rs src/surface.rs src/systems.rs src/lib.rs src/main.rs tests/u12_stepping.rs tests/u12_being.rs tests/u1_surface.rs)

snapshot() {
    for file in "${FILES[@]}"; do
        mkdir -p "$WORK/$(dirname "$file")"
        cp "$CRATE/$file" "$WORK/$file"
    done
    (cd "$CRATE" && sha256sum "${FILES[@]}") > "$WORK/digests.txt"
}

# The binary is rebuilt from the restored sources: `cargo test` builds the binary too, so a perturbed case
# would otherwise leave a perturbed binary behind for whoever runs it next.
rebuild() {
    if ! (cd "$CRATE" && cargo build --offline -j 2 > "$WORK/rebuild.log" 2>&1); then
        echo "   RESULT: THE BINARY DID NOT COME BACK FROM THE RESTORED SOURCES"
        FAILED=1
        return
    fi
    restore
}

restore() {
    for file in "${FILES[@]}"; do
        if [ -f "$WORK/$file" ]; then cp "$WORK/$file" "$CRATE/$file"; fi
    done
}
trap 'restore; rebuild; rm -rf "$WORK"' EXIT

# A perturbation is one exact string, replaced once — no line numbers, and a match that is not unique is a
# failed case rather than a silent no-op. This tree's sources are CRLF, so a multi-line pattern written with
# bare newlines (as the cases below are, for readability) is translated to the file's own line ending when
# that is what matches — and a case that matches neither is a failed case rather than a silent no-op.
perturb() {
    python - "$1" "$2" "$3" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(path, "r", encoding="utf-8", newline="").read()
for old_try, new_try in ((old, new), (old.replace("\n", "\r\n"), new.replace("\n", "\r\n"))):
    count = text.count(old_try)
    if count == 1:
        open(path, "w", encoding="utf-8", newline="").write(text.replace(old_try, new_try))
        raise SystemExit(0)
    if count > 1:
        raise SystemExit(f"the perturbation matched {count} times, not once: {old!r}")
raise SystemExit(f"the perturbation matched 0 times: {old!r}")
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
        # The first lines that name the fault: the assertion, the panic with its own refusal, or a compiler error.
        printf '%s' "$out" | grep -E 'assertion .* failed|panicked at|^error' | head -3 | sed 's/^/   bit: /'
    fi
    restore
    if ! (cd "$CRATE" && sha256sum -c "$WORK/digests.txt" > /dev/null 2>&1); then
        echo "   RESULT: THE TREE DID NOT COME BACK BYTE-EXACT"
        FAILED=1
    fi
    rebuild
}

snapshot

# The snapshot is the output of **these** sources, so the binary is rebuilt from the snapshot before the
# start capture is taken (a binary left on disk by an older build is a difference that is real but not a
# defect of this run — §37's harness caught that the hard way).
rebuild

# The run's own output **before any perturbation** — what the rebuilt binary must reproduce.
(cd "$CRATE" && "$BIN") > "$WORK/start-stdout.txt" 2> "$WORK/start-stderr.txt"

# 1. The law's own rain: the declared micrometres per tick are what a tick adds — the subject is the constant
#    in the arithmetic that adds it.
run_case "the law's own rain" src/automaton.rs \
    'let own = column.water_um + RAIN_UM_PER_TICK;' 'let own = column.water_um + RAIN_UM_PER_TICK + 1;' \
    "--lib" "automaton::tests::one_tick_is_one_synchronous_hop_by_the_declared_arithmetic_over_the_cells"

# 2. The pit's declared fraction: a pit keeps this share of what it holds and drains the rest — the subject is
#    the retention itself.
run_case "the pit's declared fraction" src/automaton.rs \
    'poured[cell] += own - own * (100 - PIT_KEEP_PERCENT) / 100;' 'poured[cell] += own;' \
    "--lib" "automaton::tests::rain_wets_a_dry_pit_and_the_plane_comes_to_rest_at_the_laws_own_fixed_point"

# 3. One synchronous pass: a column whose receiver is another keeps **nothing** of its own water in that tick
#    — the subject is the hop's own arm.
run_case "one synchronous pass (a hop keeps nothing of its own)" src/automaton.rs \
    'Some(to) => poured[to] += own,' 'Some(to) => { poured[to] += own; poured[cell] += own; }' \
    "--lib" "automaton::tests::one_tick_is_one_synchronous_hop_by_the_declared_arithmetic_over_the_cells"

# 4. The state's identity is the cells' and not the tick count: a world at rest at tick 200 and the same world
#    at tick 700 are one identity — the subject is the digest's own argument.
run_case "the state's identity is the cells' (not the count)" src/automaton.rs \
    '    pub fn digest(&self) -> Digest {
        self.world.digest()
    }' '    pub fn digest(&self) -> Digest {
        let ticks = self.ticks.to_le_bytes();
        Digest::of_parts(&[self.world.digest().as_bytes(), &ticks])
    }' \
    "--lib" "automaton::tests::the_plane_comes_to_rest_and_the_settled_world_is_not_the_transient"

# 5. The graph is **the cells' own surfaces**: the tick routes by what the cells return — the subject is the
#    elevation the running law hands its own routing function.
run_case "the graph is the cells' own surfaces" src/automaton.rs \
    'let elevation: Vec<f64> = columns.iter().map(|column| column.surface_z as f64).collect();' \
    'let elevation: Vec<f64> = columns.iter().map(|_column| 0.0).collect();' \
    "--lib" "automaton::tests::the_law_steps_the_cells_and_not_the_generator"

# 6. The law's own tick count: one tick is one count — the subject is the increment.
run_case "the law's own tick count" src/automaton.rs \
    'self.ticks += 1;' 'self.ticks += 2;' \
    "--lib" "automaton::tests::the_law_is_a_pure_function_of_the_cells_and_the_tick_count"

# 7. The pour never writes a column's ground: this law moves water and the ground is not its to move — the
#    subject is the sweep's own bound.
run_case "the pour never writes a column's ground" src/chunk.rs \
    'if z <= grounds[index] {
                    continue;
                }' 'if z < grounds[index] {
                    continue;
                }' \
    "--lib" "chunk::tests::the_pour_round_trips_the_water_it_is_handed"

# 8. The cell's own cap (§37 named `CELL_HOLD_UM` unexercised; this pass gave it a subject): a cell holds at
#    most one voxel's worth and the rest stands in the cells above — the subject is the clamp's own bound.
run_case "the cell's own cap (CELL_HOLD_UM)" src/chunk.rs \
    'cell.water_um = (water_um[index] - offset * CELL_HOLD_UM).clamp(0, CELL_HOLD_UM);' \
    'cell.water_um = (water_um[index] - offset * CELL_HOLD_UM).clamp(0, CELL_HOLD_UM * 2);' \
    "--lib" "chunk::tests::a_depth_past_one_voxels_worth_spills_into_the_next_cell"

# 9. A cell the world owns but the patch does not name is **not a column of this patch** — the defect this
#    pass found by running: the world spans whole chunks, so a sweep that indexed by the patch's own order
#    without that bound walked past the fields it was handed (a panic, not a wrong number). The subject is
#    that bound.
run_case "a cell outside the patch is not one of its columns" src/chunk.rs \
    'if x < origin.x || y < origin.y || x >= origin.x + side || y >= origin.y + side {
        return None;
    }' 'if false {
        return None;
    }' \
    "--lib" "chunk::tests::the_pour_round_trips_the_water_it_is_handed"

# 10. The reading is the **declared voxel** of the generator's elevation at every column — the check §37's own
#     question turns on: a reading that quantised wrongly would still decompose into three tidy voxel causes,
#     so the fidelity check is measured before the decomposition. The subject is the build's own quantisation.
run_case "the reading is the declared voxel" src/chunk.rs \
    'let surface_z = elevation[cell].floor() as i32;' \
    'let surface_z = elevation[cell].floor() as i32 + 1;' \
    "--lib" "automaton::tests::every_route_difference_is_the_voxel_and_none_is_unexplained"

# 11. The walk's own water test: a being's walk stops where the world's own water stands — the subject is the
#     walk's own check.
run_case "the walk's own water" src/being.rs \
    'if world.water_stands(at) {
            return Some((step, at));
        }' 'if false {
            return Some((step, at));
        }' \
    "--test u12_stepping" "a_rained_on_column_changes_the_beings_next_step"

# 12. The walk's own arithmetic: the walk counts its own steps from one hop, and the count it answers with is
#     the count the being's own acts carry — the subject is the loop's own bound.
run_case "the walk counts its own hops" src/being.rs \
    'for step in 1..=reach {' 'for step in 0..=reach {' \
    "--test u12_stepping" "a_rained_on_column_changes_the_beings_next_step"

# The final check: the tree came back byte-exact, so the rebuilt binary must run the program it ran at the
# start — byte for byte. Whether the dated capture still matches is reported, not gated.
echo
echo "== the rebuilt binary, against the run's own start"
(cd "$CRATE" && "$BIN") > "$WORK/rebuilt-stdout.txt" 2> "$WORK/rebuilt-stderr.txt"
rebuilt_status=$?
echo "   exit: $rebuilt_status"
if [ "$rebuilt_status" -ne 0 ] \
    || ! cmp -s "$WORK/rebuilt-stdout.txt" "$WORK/start-stdout.txt" \
    || ! cmp -s "$WORK/rebuilt-stderr.txt" "$WORK/start-stderr.txt"; then
    echo "   RESULT: THE REBUILT BINARY'S OWN OUTPUT IS NOT THE OUTPUT OF THE SOURCES IT WAS BUILT FROM"
    FAILED=1
else
    echo "   its stdout and stderr are the start-of-run bytes byte for byte"
    sha256sum "$WORK/rebuilt-stdout.txt" | sed 's|^|   |'
    if cmp -s "$WORK/rebuilt-stdout.txt" "$BASELINE/final-stdout.txt"; then
        echo "   and the dated capture final-stdout.txt still describes these bytes exactly"
    else
        echo "   (the dated capture final-stdout.txt describes older bytes: a later pass moved a printed line —"
        echo "    that is the capture's business, not this check's)"
    fi
fi

echo
if [ "$FAILED" -eq 0 ]; then
    echo "every guard bit by name, and every touched file came back byte-exact:"
    cat "$WORK/digests.txt"
    echo "case logs kept in the scratch directory (not copied into the tree): $(basename "$WORK")"
else
    echo "A GUARD DID NOT BITE, OR THE TREE DID NOT COME BACK"
fi
exit "$FAILED"
