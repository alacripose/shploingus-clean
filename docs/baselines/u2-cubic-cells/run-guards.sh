#!/usr/bin/env bash
# **The guard proof for the cubic world's first slice (2026-10-13, plan §37; `LEDGER.md` C-189).**
#
# Each new guard is perturbed **in its own subject** — the named empty, a place outside the patch, the
# ceiling's own boundary, the fields-that-are-not-one-patch check, a class that cannot be formed, the voxel
# that contains the ground, the water read above the surface, the cell in the identity, the patch framed in
# the identity, a pour that is a set of the greater water, and the two lines of `WorldRead::over_cubic` that
# source the being's read from the cells — and then the bar is run to show it **bites, and by name**. Every
# touched file is restored and its digest checked before the next case, and the script restores through a
# `trap`, so an interrupt cannot leave the tree perturbed.
#
# **The build is restored too**, and this case was learned the hard way (§36 records it): `cargo test`
# rebuilds the *binary* as well as the test harnesses, so a case that runs while a source is perturbed leaves
# `target/debug/shploingus-clean-room` as a **perturbed build**. Each case therefore ends by rebuilding from
# the restored sources.
#
# **The last check is the run's own determinism, not a dated file.** §36's harness compared the rebuilt
# binary's output against its baseline's frozen capture; that check passes only while the capture is the
# current bytes, so the first later pass that moves a printed label turns it red for a reason that is not a
# defect. Here the script snapshots the **binary's own output at the start**, before any perturbation, and
# requires the rebuilt binary to reproduce *that*, byte for byte — and then reports, without gating on it,
# whether the dated capture `final-stdout.txt` still matches. A tree that came back byte-exact must run the
# same program; whether a dated capture still describes it is a question about the capture.
#
# The case logs are kept in the scratch directory this script makes and are **not** copied into the tree:
# they carry the run's own test-thread PIDs and cargo timings, so they cannot be reproduced byte-for-byte
# (§33 recorded that trap). This script's own output is the proof.
#
# Usage: `bash docs/baselines/u2-cubic-cells/run-guards.sh` from anywhere. Exit 0 only when every case bit
# by name and every file came back byte-exact.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE="$ROOT/clean-room"
WORK="$(mktemp -d)"
BASELINE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$CRATE/target/debug/shploingus-clean-room"
FILES=(src/chunk.rs src/being.rs src/evidence.rs src/surface.rs src/main.rs src/lib.rs src/systems.rs tests/u12_being.rs tests/u12_stepping.rs)

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
    # shellcheck disable=SC2086 — the selector is meant to split: `--lib`, or `--test u1_surface`.
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

# **The build comes first, and this harness caught that the hard way (run 4, 2026-10-13).** The start snapshot
# is what the final check compares against, so it must be the output of **these** sources: a binary left on
# disk by an earlier build of older bytes — a printed registry line had moved between them — made the final
# check fail on a difference that was real but not a defect of this run. `rebuild` restores the snapshot and
# builds from it, so the snapshot below is this tree's own program.
rebuild

# The run's own output **before any perturbation** — what the rebuilt binary must reproduce. (A dated
# capture is a measurement of a moment; this is a measurement of this run, and it cannot go stale.)
(cd "$CRATE" && "$BIN") > "$WORK/start-stdout.txt" 2> "$WORK/start-stderr.txt"

# 1. The named empty: a cell nobody filled is air, not a default material — the subject is the chunk's own
#    fill.
run_case "the named empty (air)" src/chunk.rs \
    'cells: vec![Cell::AIR; CHUNK_CELLS]' 'cells: vec![Cell::solid(MaterialId(0)); CHUNK_CELLS]' \
    "--lib" "chunk::tests::air_is_named_and_a_place_no_chunk_owns_is_refused_by_name"

# 2. A place outside the patch: the reading answers the patch's own columns and refuses the rest — the
#    subject is the patch's own bound, whole. (This case's **first** cut perturbed only the two *near* clauses
#    and **bit nothing**: the test probes the patch's *far* edge, which the other two clauses still caught — the
#    finding is kept here because a case that perturbs the wrong half of its subject is a case that proves
#    nothing, and the harness is what caught it.)
run_case "a place outside the patch" src/chunk.rs \
    'if at.x < origin.x
            || at.y < origin.y
            || at.x >= origin.x + side
            || at.y >= origin.y + side
        {' 'if false {' \
    "--lib" "chunk::tests::a_place_outside_the_patch_is_refused_by_name"

# 3. The ceiling the world spans: the world ends on a chunk boundary above its highest water — the subject
#    is the ceiling's own arithmetic.
run_case "the world ends on a chunk boundary" src/chunk.rs \
    'let ceiling_z = (top_chunk + 1) * CHUNK_SIDE;' 'let ceiling_z = (top_chunk + 1) * CHUNK_SIDE - 1;' \
    "--lib" "chunk::tests::the_world_spans_whole_chunks_over_the_surface_it_was_filled_from"

# 4. The fields must describe one patch: three arrays of one length, or no world — the subject is the check
#    itself.
run_case "one patch's own fields" src/chunk.rs \
    'if elevation.len() != cells || water_level.len() != cells || classes.len() != cells {' 'if false {' \
    "--lib" "chunk::tests::fields_that_do_not_describe_one_patch_are_refused_by_name"

# 5. A class that cannot be formed: a non-finite lithology is refused by name — the subject is the finiteness
#    test.
run_case "a class that cannot be formed" src/chunk.rs \
    'if !class.is_finite() {' 'if false {' \
    "--lib" "chunk::tests::a_class_that_cannot_be_formed_is_refused_by_name"

# 6. The voxel that contains the ground: the surface the reading returns is inside one voxel of the ground —
#    the subject is the quantisation.
run_case "the voxel that contains the ground" src/chunk.rs \
    'let surface_z = elevation[cell].floor() as i32;' 'let surface_z = elevation[cell].floor() as i32 - 1;' \
    "--lib" "chunk::tests::a_column_reads_back_the_water_the_declared_field_stands"

# 7. The water above the surface: the reading sums the cells above the top solid one — the subject is that
#    sum.
run_case "the water above the surface" src/chunk.rs \
    'water_um += self.cell(VoxelPos::new(at.x, at.y, z))?.water_um;' 'water_um += 0;' \
    "--lib" "chunk::tests::a_column_reads_back_the_water_the_declared_field_stands"

# 8. The cell in the identity: one micrometre in one cell is another world — the subject is the cell's own
#    bytes.
run_case "one cell moves the identity" src/chunk.rs \
    'bytes.extend_from_slice(&cell.water_um.to_le_bytes());' 'bytes.extend_from_slice(&0_i64.to_le_bytes());' \
    "--lib" "chunk::tests::the_identity_is_the_cells_and_moves_with_any_of_them"

# 9. A reading and the field it replaces describe the same column at the same index: the subject is the
#    patch's own column order. (This case replaces one that **bit nothing** and was kept as the finding it
#    is: `the patch is framed in the identity`, whose subject — zeroing the origin's framing — left the two
#    worlds different anyway, because a patch one column over moves every cell's own bytes inside its chunk.
#    The framing is therefore named as **unexercised** in C-189 §189.8 rather than claimed to be proved, and
#    the test it aimed at is renamed for what it measures: `a_patch_one_column_over_is_another_world`.)
run_case "the patch's own column order" src/chunk.rs \
    '            let x = origin.x + (cell % side as usize) as i32;
            let y = origin.y + (cell / side as usize) as i32;
            out.push(self.column(VoxelPos::new(x, y, 0))?);' \
    '            let x = origin.x + (cell / side as usize) as i32;
            let y = origin.y + (cell % side as usize) as i32;
            out.push(self.column(VoxelPos::new(x, y, 0))?);' \
    "--lib" "chunk::tests::a_column_reads_back_the_water_the_declared_field_stands"

# 10. A pour is a set of the greater water: a second, smaller pour does not shrink what stands — the subject
#     is the cell's own write.
run_case "a pour is a set of the greater water" src/chunk.rs \
    'if water_um > cell.water_um {' 'if true {' \
    "--lib" "chunk::tests::a_second_pour_does_not_shrink_the_water_of_a_cell"

# 11. The being's read is sourced from the cells (the water): a reading re-derived from the fields the world
#     was built from could not follow a cell — the subject is `over_cubic`'s own water line.
run_case "the reading's water is the cells' (being.rs)" src/being.rs \
    '.map(|column| column.water_um as f64 / 1_000_000.0)' '.map(|_column| 0.0)' \
    "--lib" "chunk::tests::the_reading_a_being_takes_is_sourced_from_the_cells"

# 12. The being's read is sourced from the cells (the surface): the same, for the ground's own line.
run_case "the reading's surface is the cells' (being.rs)" src/being.rs \
    '.map(|column| column.surface_um as f64 / 1_000_000.0)' '.map(|column| column.surface_um as f64 / 1_000_000.0 + 1.0)' \
    "--lib" "chunk::tests::the_reading_a_being_takes_is_sourced_from_the_cells"

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
