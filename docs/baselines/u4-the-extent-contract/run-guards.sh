#!/usr/bin/env bash
# **The guard proof for the extent contract (2026-10-13, plan §40; `LEDGER.md` C-192).**
#
# Two kinds of case, one rule: perturb **the guard's own subject** and show the bar bites, **by name**.
#
# * A **test case** perturbs a source and runs the named test or suite; the fault line the bar prints (the
#   assertion, the panic, the compiler error) is the name.
# * A **product case** perturbs a source and runs the **binary**; the named line in its own output flipping
#   to `= false` — with a non-zero exit — is the name. This pass's new evidence lines (`U4 one world, two
#   doors:`, `U4 region:`) and the U10 document guard live only in the run, so a case that ran only
#   `cargo test` could not show them biting at all.
#
# **The twelve cases §39 proved are re-expressed here on the bytes this pass holds** — and re-expressing
# them found three real things rather than three string edits:
#
# * the side rule's bar `a_flat_uniform_chunk_is_six_faces` **no longer exists** (the extent contract's own
#   first test replaced it), so the case is re-pointed at the test that now refuses a chunk with walls;
# * the columns-the-world-owns rule had **two owners** after the contract landed — `Footprint::over`'s refusal
#   and a shadow count in `faces_of` — and disabling either alone changed nothing observable. This pass
#   **removed the shadow count**: the contract is the one owner, and the case perturbs the contract's own
#   refusal, which now bites;
# * the footprint's own floor interior by declaration is **not observable from the module's flat test** (its
#   world holds solid ground below the footprint's floor), so that case is a **product** case, proved by the
#   independent recount disagreeing with the sweep.
#
# §39's own `run-guards.sh` is kept **unmodified** as the dated artifact of §39's bytes; this file is the one
# that runs on these.
#
# **One thing this header did not carry until the amendment audit asked for it.** The pass's own end-to-end
# check, `tests/u4_geometry.rs`, was **destroyed mid-pass** by a `write_file` and rebuilt from the pass's own
# earlier read of it. The `clean-room/` tree is untracked (`git ls-files clean-room` answers nothing; one
# branch, no stash), so nothing could restore it, and §39's bytes survive only as a digest (`575a1cb7…`). That
# is also why **this file can carry no witness for it**: every case here, and all twelve of §39's, perturbs a
# *source* the mesher owns — `src/mesh.rs`, `src/evidence.rs`, `src/gameplay.rs` — so no case can be re-pointed
# at a byte range inside the rebuilt test file. The incident is written up in plan §40 and `LEDGER.md` C-192
# §192.10; `digests.txt` names what the rebuilt file is, and the four `%TEMP%` copies (`47333a99…`, 319 lines,
# **two** tests) that are a *different*, older file rather than the source of the rebuild.
#
# **The ten new cases, each with its own subject:**
#
# 1. the reading reads the ring it declares (`build_from_fields`'s halo patch — the world is the chunk *plus*
#    `MESH_RING`);
# 2. `Footprint::halo_owned` tells the truth about a world that is only the chunk;
# 3. the footprint's own floor is **interior by declaration** (`solid_at`'s first answer);
# 4. the span's ceiling is a band strictly above the highest cell the columns hold;
# 5. a region is **one footprint over the rectangle** (`build_region`'s side), not one chunk's;
# 6. the cells' door meshes the chunk's **own** footprint, not its world's;
# 7. the two doors are compared over **the one world the reading reads** (the evidence's own choice of world);
# 8. both sides of a region's seam name the **same world plane** (the chunks' own boundary count);
# 9. a region's faces are the chunks' faces **summed** (the per-chunk footprint in that loop);
# 10. the declared `U4_QUICKSTART_DIGEST` is the document on disk (the read-in-place guard that refused this
#     pass's own document edit until the constant moved with it — the defect the cascade found by running).
#
# Every touched file is restored and its digest checked before the next case, and the script restores through
# a `trap`, so an interrupt cannot leave the tree perturbed. Three files in `FILES` — `src/systems.rs`
# (U4's status sentence), `src/world.rs` (a doc comment naming the mesh's digest) and `tests/u6_bridge.rs`
# (the budget fixture's own `1156`) — are **hashed and restored but never perturbed**: this pass moved a
# printed status line, a doc comment's digest and a test's declared column count, so they belong to the
# "every touched file" claim without carrying a guard of their own, and saying which is which is cheaper than
# implying that every listed file has a case.
#
# **The build is restored too**: `cargo test` and `cargo build` both rebuild the *binary*, so a case that
# runs while a source is perturbed leaves a perturbed build behind. Each case ends by rebuilding from the
# restored sources.
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
# Usage: `bash docs/baselines/u4-the-extent-contract/run-guards.sh` from anywhere. Exit 0 only when every
# case bit by name and every file came back byte-exact.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE="$ROOT/clean-room"
WORK="$(mktemp -d)"
BASELINE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$CRATE/target/debug/shploingus-clean-room"
FILES=(src/mesh.rs src/evidence.rs src/systems.rs src/world.rs src/gameplay.rs tests/u4_geometry.rs tests/u6_bridge.rs)

snapshot() {
    for file in "${FILES[@]}"; do
        mkdir -p "$WORK/$(dirname "$file")"
        cp "$CRATE/$file" "$WORK/$file"
    done
    (cd "$CRATE" && sha256sum "${FILES[@]}") > "$WORK/digests.txt"
}

# A rebuild from the restored sources: `cargo test`/`cargo build` leave the binary in the tree, so a
# perturbed case would otherwise leave a perturbed binary behind for whoever runs it next.
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
# failed case rather than a silent no-op. This tree's sources are mixed: the files this pass touched are LF
# except `src/evidence.rs`, which is CRLF, so a multi-line pattern written with bare newlines (as the cases
# below are, for readability) is translated to the file's own line ending when that is what matches — and a
# case that matches neither is a failed case rather than a silent no-op.
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

# Restores and digest-checks, and reports whether the tree came back. Shared by both kinds of case.
settle() {
    restore
    if ! (cd "$CRATE" && sha256sum -c "$WORK/digests.txt" > /dev/null 2>&1); then
        echo "   RESULT: THE TREE DID NOT COME BACK BYTE-EXACT"
        FAILED=1
    fi
    rebuild
}

run_case() {
    local name="$1" file="$2" old="$3" new="$4" selector="$5" filter="$6"
    echo "== $name"
    echo "   subject: $file :: $old"
    if ! perturb "$CRATE/$file" "$old" "$new"; then
        echo "   RESULT: the perturbation did not apply"
        FAILED=1
        settle
        return
    fi
    local out status log
    log="$WORK/$(printf '%s' "$name" | tr ' ()/' '____').test.log"
    set +e
    # shellcheck disable=SC2086 — the selector is meant to split: `--lib`, or `--test u4_geometry`.
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
    settle
}

# The product kind: perturb, rebuild, run the binary, and require **the named line** to have flipped to
# `= false` with a non-zero exit. The needle is the guard's own printed clause, so a case cannot pass by
# some other line going false.
run_product_case() {
    local name="$1" file="$2" old="$3" new="$4" needle="$5"
    echo "== $name"
    echo "   subject: $file :: $old"
    if ! perturb "$CRATE/$file" "$old" "$new"; then
        echo "   RESULT: the perturbation did not apply"
        FAILED=1
        settle
        return
    fi
    if ! (cd "$CRATE" && cargo build --offline -j 2 > "$WORK/perturbed-build.log" 2>&1); then
        echo "   RESULT: THE PERTURBED BUILD DID NOT COMPILE"
        FAILED=1
        settle
        return
    fi
    local status
    set +e
    (cd "$CRATE" && "$BIN") > "$WORK/perturbed-stdout.txt" 2> "$WORK/perturbed-stderr.txt"
    status=$?
    set -e
    local line
    line="$(grep -F -- "$needle" "$WORK/perturbed-stdout.txt" | head -1 || true)"
    if [ "$status" -eq 0 ]; then
        echo "   RESULT: BIT NOTHING — the binary exited 0 on a perturbed build"
        FAILED=1
    elif [ -z "$line" ]; then
        echo "   RESULT: THE NAMED LINE IS NOT IN THE PERTURBED OUTPUT: $needle"
        FAILED=1
    else
        echo "   bit: $(printf '%s' "$line" | cut -c1-150)"
        echo "   exit: $status"
    fi
    settle
}

snapshot

# The snapshot is the output of **these** sources, so the binary is rebuilt from the snapshot before the
# start capture is taken (a binary left on disk by an older build is a difference that is real but not a
# defect of this run).
rebuild

# The run's own output **before any perturbation** — what the rebuilt binary must reproduce.
(cd "$CRATE" && "$BIN") > "$WORK/start-stdout.txt" 2> "$WORK/start-stderr.txt"

# ---------------------------------------------------------------------------
# §39's twelve guards, re-expressed on the extent contract's bytes.
# ---------------------------------------------------------------------------

# 1. The side rule: a solid's side is a face where its neighbour is not solid — the subject is the visibility
#    test itself, which now takes the footprint the contract is made of. The bar is the extent contract's own
#    first test: §39's `a_flat_uniform_chunk_is_six_faces` no longer exists under that name, and a chunk whose
#    every side became a face is exactly what "a flat chunk is one rectangle" refuses.
run_case "the side rule (a neighbour that is not solid)" src/mesh.rs \
    '                    if solid_at(cube, footprint, x + dx, y + dy, z + dz)? {
                        continue;
                    }' '                    if false {
                        continue;
                    }' \
    "--lib" "mesh::tests::a_flat_worlds_chunk_is_one_plane_and_its_edges_are_the_rings_business"

# 2. The water's own surface rule: a column's water shows its top where the cell above holds none.
run_case "the water's own surface (the cell above holds none)" src/mesh.rs \
    'if cell.water_um > 0 && !water_at(cube, footprint, x, y, z + 1)? {' 'if cell.water_um > 0 && true {' \
    "--lib" "mesh::tests::two_cells_merge_only_when_layer_plane_and_material_agree"

# 3. The merge's group key: a rectangle can never cross a layer, an axis, a facing, a plane or a material.
run_case "the merge's group key (layer · axis · facing · plane · material)" src/mesh.rs \
    '.entry((face.layer, face.axis, face.facing, face.plane, face.material))' \
    '.entry((face.layer, face.axis, 0, face.plane, face.material))' \
    "--lib" "mesh::tests::a_step_is_a_wall_and_not_a_hole"

# 4. The merge's own tally: a rectangle that grew past a single face is counted as merged.
run_case "the merge counts only rectangles that grew" src/mesh.rs \
    'if (width as u64) * (depth as u64) > 1 {' 'if (width as u64) * (depth as u64) > 0 {' \
    "--lib" "mesh::tests::merge_tiles_every_group_exactly_whatever_the_pattern"

# 5. **The columns the world owns** — now the contract's own refusal, and the one owner of it: this pass
#    **removed** the second, shadowing count that `faces_of` carried (§39's subject), so `Footprint::over` is
#    the only place a footprint with no column of the world under it is refused, and this case perturbs *that*
#    refusal's own condition. §39's subject could not be proved on these bytes at all: with the contract's
#    refusal beside it, disabling the shadow count changed nothing observable, which is exactly why it went.
run_case "the columns the world owns (the contract's own refusal)" src/mesh.rs \
    'let (Some(lowest_ground), Some(highest_cell)) = (lowest_ground, highest_cell) else {' \
    'let (Some(lowest_ground), Some(highest_cell)) = (lowest_ground.or(Some(0)), highest_cell.or(Some(0))) else {' \
    "--lib" "mesh::tests::a_chunk_with_no_column_under_the_world_is_refused_by_name"

# 6. The key: a mesh of a cell world is named by that world's own identity.
run_case "the key is the cells' own identity" src/mesh.rs \
    'self.merge_into(GeometryKey::of_state(cube.digest(), chunk, level), faces, count, effort, done)' \
    'self.merge_into(GeometryKey::of_state(Digest::of(b"a fixed state"), chunk, level), faces, count, effort, done)' \
    "--lib" "mesh::tests::the_laws_own_step_re_keys_the_mesh_and_moves_no_ground_face"

# 7. The level rule: the render ladder does not coarsen a cell world — the subject is the **cells' door's**
#    own test, and `derive_region` repeats it, so the case names the arm that counts a refusal.
run_case "the render ladder does not coarsen a cell world" src/mesh.rs \
    'if level != Level::ALL[0] {
            ledger.refusals += 1;
            return Err(cell_level_refusal(level));
        }' 'if false {
            ledger.refusals += 1;
            return Err(cell_level_refusal(level));
        }' \
    "--lib" "mesh::tests::a_level_above_zero_on_a_cell_world_is_refused_by_name"

# 8. The accounting: the ledger's face count is the sweep's.
run_case "the ledger's face count is the sweep's" src/mesh.rs \
    'done.faces += count.total_faces();' 'done.faces += count.total_faces() + 1;' \
    "--lib" "mesh::tests::the_ledger_equals_an_independent_recount"

# 9. The water's own material: the ladder's declared water id, not a plausible rock.
run_case "the water face is the ladder's declared water" src/mesh.rs \
    '                        material: water,
                    });' '                        material: MaterialId(1),
                    });' \
    "--lib" "mesh::tests::two_cells_merge_only_when_layer_plane_and_material_agree"

# 10. The budget's quad ceiling: checked against the merge that really happened.
run_case "the quad ceiling is checked after the merge" src/mesh.rs \
    'if done.quads > effort.budget.max_quads {' 'if false {' \
    "--lib" "mesh::tests::a_budget_that_cannot_be_met_is_refused_with_its_numbers"

# 11. The surface's own finiteness: a NaN elevation is refused by name, with its field.
run_case "a NaN surface is refused with its own field" src/mesh.rs \
    'if let Some(value) = plane.iter().find(|value| !value.is_finite()) {' \
    'if let Some(value) = plane.iter().find(|value| false) {' \
    "--lib" "mesh::tests::a_stages_refusal_passes_through_the_mesh_seam"

# 12. The reading's own key state: U2's truth digest, so a moved world is rebuilt and never served.
run_case "the reading's key is the world's own truth" src/mesh.rs \
    'self.merge_into(GeometryKey::new(world, chunk, level), faces, count, effort, done)' \
    'self.merge_into(GeometryKey::of_state(Digest::of(b"a fixed state"), chunk, level), faces, count, effort, done)' \
    "--lib" "mesh::tests::a_stale_entry_is_rebuilt_and_never_served"

# ---------------------------------------------------------------------------
# §40's new guards.
# ---------------------------------------------------------------------------

# 13. **The reading reads the ring it declares.** The subject is the halo the reading builds its world over
#     (the chunk's footprint *plus* `MESH_RING`); a world that is only the chunk cannot own the halo, so the
#     reading's own produced-columns count must stop being `ring_columns()`.
run_case "the reading reads the ring it declares" src/mesh.rs \
    '        let halo = Patch::new(
            VoxelPos::new(origin.x - MESH_RING, origin.y - MESH_RING, origin.z),
            CHUNK_SIDE as u32 + 2 * MESH_RING as u32,
        )?;' '        let halo = Patch::new(
            VoxelPos::new(origin.x - MESH_RING, origin.y - MESH_RING, origin.z),
            CHUNK_SIDE as u32,
        )?;' \
    "--lib" "mesh::tests::a_flat_worlds_chunk_is_one_plane_and_its_edges_are_the_rings_business"

# 14. **`halo_owned` tells the truth.** The subject is the ring's membership answer; a declaration that
#     claims every world owns the halo must stop saying that a world which is only the chunk does not.
run_case "halo_owned tells the truth about a world that is only the chunk" src/mesh.rs \
    '                if !columns_hold(own, x, y) {
                    return false;
                }' '                if false {
                    return false;
                }' \
    "--lib" "mesh::tests::a_flat_worlds_chunk_is_one_plane_and_its_edges_are_the_rings_business"

# 15. **The footprint's own floor is interior by declaration.** The subject is `solid_at`'s first answer.
#     Its bar is the **reciprocal** guard rather than a test: the independent recount keeps the declaration, so
#     dropping it from the sweep makes the ledger's faces disagree with the recount's for the same cells
#     (`1 289` against `2 313` on this pass's bytes) and the named line flips to `= false`. The module's own
#     flat test does **not** notice — its world holds ground below the footprint's floor — which is worth
#     knowing: this declaration is measured by the recount, not by the fixture.
run_product_case "the footprint's own floor is interior by declaration" src/mesh.rs \
    '    if z == footprint.floor_z() - MESH_RING {
        return Ok(true);
    }' '    if false {
        return Ok(true);
    }' \
    'an independent sweep of the same cells = false'

# 16. **The span's ceiling is a band strictly above the highest cell.** The subject is the ceiling rule in
#     `Footprint::over`; a span that stops at the highest cell's own band hides the surface it was derived
#     from (and the water's top face with it).
run_case "the span's ceiling is a band above the highest cell" src/mesh.rs \
    'let ceiling_z = (highest_cell.div_euclid(CHUNK_SIDE) + 1) * CHUNK_SIDE;' \
    'let ceiling_z = highest_cell.div_euclid(CHUNK_SIDE) * CHUNK_SIDE;' \
    "--test u4_geometry" "one_footprint_stitches_chunks_and_its_faces_ignore_the_world_beyond_the_ring"

# 17. **A region is one footprint over the rectangle.** The subject is the side `build_region` declares; one
#     chunk's side must stop the region's ledger from sweeping every chunk's columns once.
run_case "a region is one footprint over the rectangle" src/mesh.rs \
    'let side = CHUNK_SIDE as u32 * chunks;' 'let side = CHUNK_SIDE as u32;' \
    "--test u4_geometry" "one_footprint_stitches_chunks_and_its_faces_ignore_the_world_beyond_the_ring"

# 18. **The cells' door meshes the chunk's own footprint.** The subject is the footprint `faces_of` sweeps;
#     a world-patch footprint must stop the cells' door from drawing what the reading draws of one world.
run_case "the cells' door meshes the chunk's own footprint" src/mesh.rs \
    'let footprint = Footprint::over(cube, footprint)?;' 'let footprint = Footprint::over(cube, cube.patch())?;' \
    "--lib" "mesh::tests::a_step_is_a_wall_and_not_a_hole"

# 19. **The two doors are compared over the one world the reading reads.** The subject is the extent one of
#     the doors reads — here the reading's own halo, the same subject case 13 perturbs, exercised through the
#     second bar: with a world that is only the chunk on the reading's side and the chunk + ring on the cells'
#     side, the named `U4 one world, two doors:` line must flip to `= false` and the binary must exit non-zero.
#     (Two cases over one subject is deliberate here: `derive` and `derive_cubic` are two doors, and the run's
#     own line is the only place the two are compared **face for face** rather than counted.)
run_product_case "the two doors are compared over the reading's own world" src/mesh.rs \
    '        let halo = Patch::new(
            VoxelPos::new(origin.x - MESH_RING, origin.y - MESH_RING, origin.z),
            CHUNK_SIDE as u32 + 2 * MESH_RING as u32,
        )?;' '        let halo = Patch::new(
            VoxelPos::new(origin.x - MESH_RING, origin.y - MESH_RING, origin.z),
            CHUNK_SIDE as u32,
        )?;' \
    'reading reads through both doors = false'

# 20. **Both sides of a region's seam name the same world plane.** The subject is the chunks' own boundary
#     count; a plane one chunk over admits nothing, so `boundary_agrees` must flip while the region's own
#     count stays.
run_product_case "both sides of the seam name the same world plane" src/evidence.rs \
    '            .filter(|quad| {
                (quad.axis == Axis::X && quad.plane == seam_x)
                    || (quad.axis == Axis::Y && quad.plane == seam_y)
            })' '            .filter(|quad| {
                (quad.axis == Axis::X && quad.plane == seam_x + CHUNK_SIDE)
                    || (quad.axis == Axis::Y && quad.plane == seam_y + CHUNK_SIDE)
            })' \
    'on those planes = false'

# 21. **A region's faces are the chunks' faces summed.** The subject is the per-chunk mesh in that loop:
#     meshing one chunk four times must stop the sum from being the region's own one-sweep area.
run_product_case "a region's faces are the chunks' faces summed" src/evidence.rs \
    'derive_cubic(
            &region_world,
            at,' 'derive_cubic(
            &region_world,
            chunk,' \
    'the region 8013 = false'

# 22. **The declared document digest is the document on disk.** The subject is `gameplay.rs`'s declared
#     `U4_QUICKSTART_DIGEST`; this pass's dated note moved that document's bytes, and the read-in-place guard
#     refused the edit until the constant moved with it (plan §40's found defect).
run_case "the declared quickstart digest is the document on disk" src/gameplay.rs \
    'pub const U4_QUICKSTART_DIGEST: &str = "ca00f1be' 'pub const U4_QUICKSTART_DIGEST: &str = "ba00f1be' \
    "--lib" "gameplay::tests::the_documents_the_rows_cite_are_the_ones_on_the_disk"

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
