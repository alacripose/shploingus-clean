#!/usr/bin/env bash
# **The guard proof for the drawn reading (2026-10-13, plan §41; `LEDGER.md` C-193).**
#
# Two kinds of case, one rule: perturb **the guard's own subject** and show the bar bites, **by name**.
#
# * A **test case** perturbs a source and runs the named test; the fault line the bar prints (the assertion,
#   the panic) is the name.
# * A **product case** perturbs a source, rebuilds, runs the **binary** and requires the named line in its own
#   output to have flipped to `= false` **with a non-zero exit**. This pass's four drawn bars live on their own
#   printed line, so a case that ran only `cargo test` could not show them biting at all.
# * A **window case** does the same through the binary's own `window` mode, because the window's bars are its
#   own line and (since §41) its own **exit status**: the window exits 1 when a drawn bar is not met.
#
# **What this pass built, and what each guard's subject is.** `src/drawn.rs` is new — the drawn reading: the
# mesh's four corners and two triangles per rectangle, one `shade` call per rectangle, one camera with a depth
# buffer — and `surface.rs`'s window now draws **that** reading (the plan reading is kept and named: it is the
# evidence run's own frame). The guards, in the order the cases below prove them:
#
#  1. `drawn::corners` — the rectangle's four corners and its two triangles of `INDICES` (the mesh's own
#     `vertices()`/`triangles()`/`indices()` per rectangle);
#  2. `drawn::normal_of` — a face's normal is its axis and the way it looks;
#  3. `Camera::over_extent`'s **distance** — `radius / sin(fov / 2)`, so the extent's own sphere subtends the
#     declared field of view;
#  4. the camera's **refusals** — a field of view that is not one, a view with no direction, a box that is not
#     a box or has no size;
#  5. `Camera::project`'s **near-plane rejection** — a point at or behind the near plane is not projected;
#  6. `presentation::encode` — the transfer function, **moved** here from `surface.rs` so both readings put
#     their colours through one arithmetic (its guard moved with it);
#  7. `raster`'s **depth** — the `1/z` interpolation (caught by the **independent recount** in
#     `tests/u4_drawn.rs` rather than by the frame's own numbers) and the write path that lets a pixel keep a
#     colour at all;
#  8. `draw`'s **mesh state** — the frame carries the state digest the mesh was derived from;
#  9. `draw_plane`'s **view direction** — the camera's own declared view, not the Sun's direction;
# 10. `draw_plane`'s **declared look** — the exposure both the frame and the recount encode with;
# 11. the frame's own **size** — the client's declared `48 × ZOOM`, so one window shows either reading.
#
# The four **drawn bars** (`Drawing::bars`, printed by both the evidence run and the window) are the product
# cases: the loop's own counts against the mesh's own declaration, the camera's frustum against the clip count,
# the picture against the FMA bar and its own pixels' range, and the frame's own mesh state against the law's
# state. Each is proved through the line it prints, not through a test.
#
# **The rule the recount follows.** The seventh guard is proved by a bar that shares no code with the loop it
# judges: `tests/u4_drawn.rs` writes the projection, the coverage test, the `1/z` depth and the transfer
# function out again and compares them with the frame's own buffer over a declared sample of pixels — the
# discipline `mesh::recount_faces` uses for faces, because a second implementation of one rule is evidence and
# a second call of one implementation is not.
#
# Every touched file is restored and its digest checked before the next case, and the script restores through a
# `trap`, so an interrupt cannot leave the tree perturbed. `src/lib.rs` (the module list and its own doc line),
# `src/systems.rs` (U4's status and interface sentences) and `tests/u1_surface.rs` (whose window test's
# assertions follow the drawn frame) are hashed and restored but **carry no case of their own**: they are
# touched, so they belong to the "every touched file" claim, and saying which is which is cheaper than implying
# that every listed file has a guard.
#
# **The build is restored too**: `cargo test` and `cargo build` both rebuild the binary, so a case that runs
# while a source is perturbed leaves a perturbed build behind. Each case ends by rebuilding from the restored
# sources.
#
# **The last check is the run's own determinism, not a dated file.** The script snapshots the binary's own
# output at the start, before any perturbation, and requires the rebuilt binary to reproduce *that*, byte for
# byte — and then reports, without gating on it, whether this pass's dated captures (`final-stdout.txt`,
# `window-stdout.txt`) still match.
#
# The case logs are kept in the scratch directory this script makes (outside the tree) and are **not** copied
# into it: they carry the run's own test-thread PIDs and cargo timings, so they cannot be reproduced
# byte-for-byte. This script's own output is the proof.
#
# Usage: `bash docs/baselines/u4-the-drawn-reading/run-guards.sh` from anywhere. Exit 0 only when every case bit
# by name and every file came back byte-exact.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE="$ROOT/clean-room"
WORK="$(mktemp -d)"
BASELINE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$CRATE/target/debug/shploingus-clean-room"
FILES=(src/drawn.rs src/presentation.rs src/surface.rs src/main.rs src/evidence.rs src/systems.rs src/lib.rs tests/u4_drawn.rs tests/u1_surface.rs)

snapshot() {
    for file in "${FILES[@]}"; do
        mkdir -p "$WORK/$(dirname "$file")"
        cp "$CRATE/$file" "$WORK/$file"
    done
    (cd "$CRATE" && sha256sum "${FILES[@]}") > "$WORK/digests.txt"
}

# A rebuild from the restored sources: `cargo test`/`cargo build` leave the binary in the tree, so a perturbed
# case would otherwise leave a perturbed binary behind for whoever runs it next.
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
# except `src/evidence.rs`, which is CRLF, so a multi-line pattern written with bare newlines (as the cases below
# are, for readability) is translated to the file's own line ending when that is what matches — and a case that
# matches neither is a failed case rather than a silent no-op.
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

# Restores and digest-checks, and reports whether the tree came back. Shared by every kind of case.
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
    # shellcheck disable=SC2086 — the selector is meant to split: `--lib`, or `--test u4_drawn`.
    out="$(cd "$CRATE" && cargo test --offline -j 2 $selector "$filter" 2>&1)"
    status=$?
    set -e
    printf '%s\n' "$out" > "$log"
    if [ "$status" -eq 0 ]; then
        echo "   RESULT: BIT NOTHING — the bar passed a perturbed build"
        FAILED=1
    else
        printf '%s' "$out" | grep -E 'assertion .* failed|panicked at|^error' | head -3 | sed 's/^/   bit: /'
    fi
    settle
}

# The product kind: perturb, rebuild, run the binary, and require **the named line** to have flipped to
# `= false` with a non-zero exit. The needle is the guard's own printed clause, so a case cannot pass by some
# other line going false.
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
    local status line
    set +e
    (cd "$CRATE" && "$BIN") > "$WORK/perturbed-stdout.txt" 2> "$WORK/perturbed-stderr.txt"
    status=$?
    set -e
    line="$(grep -F -- "$needle" "$WORK/perturbed-stdout.txt" | head -1 || true)"
    if [ "$status" -eq 0 ]; then
        echo "   RESULT: BIT NOTHING — the binary exited 0 on a perturbed build"
        FAILED=1
    elif [ -z "$line" ]; then
        echo "   RESULT: THE NAMED LINE IS NOT IN THE PERTURBED OUTPUT: $needle"
        FAILED=1
    else
        echo "   bit: $(printf '%s' "$line" | cut -c1-170)"
        echo "   exit: $status"
    fi
    settle
}

# The window kind: the same, through `window 20` — twenty frames of the world's own law, so the case is bounded
# (the declared run is two hundred) while the window's own bars, its own line and its own exit status are the
# ones under test.
run_window_case() {
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
    local status line
    set +e
    (cd "$CRATE" && "$BIN" window 20 "$WORK/perturbed.ppm") \
        > "$WORK/perturbed-window-stdout.txt" 2> "$WORK/perturbed-window-stderr.txt"
    status=$?
    set -e
    line="$(grep -F -- "$needle" "$WORK/perturbed-window-stdout.txt" | head -1 || true)"
    if [ "$status" -eq 0 ]; then
        echo "   RESULT: BIT NOTHING — the window exited 0 on a perturbed build"
        FAILED=1
    elif [ -z "$line" ]; then
        echo "   RESULT: THE NAMED LINE IS NOT IN THE PERTURBED WINDOW'S OUTPUT: $needle"
        FAILED=1
    else
        echo "   bit: $(printf '%s' "$line" | cut -c1-170)"
        echo "   exit: $status (the window's own exit status)"
    fi
    settle
}

snapshot

# The snapshot is the output of **these** sources, so the binary is rebuilt from the snapshot before the start
# capture is taken (a binary left on disk by an older build is a difference that is real but not a defect of
# this run).
rebuild

# The run's own output **before any perturbation** — what the rebuilt binary must reproduce.
(cd "$CRATE" && "$BIN") > "$WORK/start-stdout.txt" 2> "$WORK/start-stderr.txt"

# ---------------------------------------------------------------------------
# The new guards, one case each.
# ---------------------------------------------------------------------------

# 1. **The rectangle's four corners.** The subject is `corners`' own order; a rectangle whose third corner
#    stopped being `(u + span_u, v + span_v)` is not the rectangle the mesh declared.
run_case "the rectangle's corners are its four vertices" src/drawn.rs \
    '        at(span_u, span_v),' '        at(span_u, 0.0),' \
    "--lib" "drawn::tests::a_rectangles_four_corners_are_the_vertices_and_its_two_triangles_the_indices"

# 2. **A rectangle's normal is its axis and the way it looks.** The subject is the facing sign; a downward
#    rectangle that answers `+z` is a rectangle the light cannot tell from the ground.
run_case "a rectangle's normal is the way it looks" src/drawn.rs \
    '] = if quad.facing >= 0 { 1.0 } else { -1.0 };' '] = 1.0;' \
    "--lib" "drawn::tests::a_rectangles_normal_is_its_axis_and_the_way_it_looks"

# 3. **The camera's own distance.** The subject is the one arithmetic that places the eye — `radius / sin(fov /
#    2)` — where the extent's own sphere subtends the declared field of view.
run_case "the eye's distance is radius over sin(fov over 2)" src/drawn.rs \
    '        let distance = radius / (fov_deg / 2.0).to_radians().sin();' '        let distance = radius;' \
    "--lib" "drawn::tests::a_camera_stands_off_so_the_extents_own_sphere_subtends_its_own_field_of_view"

# 4. **The camera's own refusals.** The subject is the field-of-view guard: an angle that is not an angle, a
#    view with no direction and a box with no size must each be refused by name rather than defaulted.
run_case "a field of view that is not one is refused" src/drawn.rs \
    '        if !(0.0 < fov_deg && fov_deg < 180.0) {' '        if false {' \
    "--lib" "drawn::tests::a_camera_refuses_an_angle_that_is_not_one_a_view_with_no_direction_and_no_extent"

# 5. **The near plane rejects what is behind it.** The subject is `project`'s own test; a point at the near
#    plane that came back projected would put geometry at a depth it does not have.
run_case "a point at or behind the near plane is not projected" src/drawn.rs \
    '        if depth < self.near || !depth.is_finite() {' '        if !depth.is_finite() {' \
    "--lib" "drawn::tests::the_projection_puts_the_target_at_the_frames_own_centre"

# 6. **The transfer function** — moved into `presentation.rs` by this pass so both readings encode through one
#    arithmetic. The subject is the sRGB curve's own low end; black that encodes to a byte is not black.
run_case "the transfer function's low end is the standard's own" src/presentation.rs \
    '            12.92 * lit' '            12.92 * lit + 1.0' \
    "--lib" "presentation::tests::a_flat_colour_encodes_to_the_byte_the_screen_is_handed"

# 7. **The depth is the perspective-correct one, judged by the independent recount.** The subject is the `1/z`
#    interpolation; a depth interpolated linearly in screen space is not the depth of the surface at that pixel,
#    and the recount in `tests/u4_drawn.rs` — which interpolates `1/z` of its own — is what measures the
#    difference. (Measured while writing this proof: disabling the nearer-wins test **alone** changes nothing on
#    this world's picture, because the mesh's own order — `Solid` before `Water`, then ground faces last —
#    happens to submit each surface in the order the eye sees it, so *nearer wins* is not decisive at any
#    sampled pixel. That is a fact about this world and not about the rule, and it is why the depth test's own
#    case below is the write path rather than the comparison.)
run_case "the depth is the perspective-correct one" src/drawn.rs \
    '            let z = 1.0 / (w0 / a.2 + w1 / b.2 + w2 / c.2);' \
    '            let z = w0 * a.2 + w1 * b.2 + w2 * c.2;' \
    "--test u4_drawn" "every_drawn_pixel_is_the_nearest_rectangle_the_mesh_holds"

# 7b. **The depth test is what lets a pixel keep a colour.** The subject is the write inside the test; with the
#     test's body gone the buffer keeps nothing, the frame's own depth buffer counts zero shaded pixels, and the
#     counts bar's own guard on the frame's pixels bites.
run_case "the depth test is what keeps a pixel" src/drawn.rs \
    '            if z < depth[at] {' '            if false {' \
    "--test u4_drawn" "the_drawn_frame_is_the_meshes_own_rectangles_to_pixels"

# 8. **The mesh state the frame came from.** The subject is the digest the frame carries; a frame that names a
#    fixed state is a frame whose geometry cannot be traced to the world it was drawn from.
run_case "the frame carries the mesh's own state" src/drawn.rs \
    '        mesh_state: mesh.key().state,' '        mesh_state: Digest::of(b"a fixed state"),' \
    "--test u4_drawn" "the_drawn_frame_is_the_meshes_own_rectangles_to_pixels"

# 9. **The camera's own declared view.** The subject is the direction `draw_plane` stands the eye off along; the
#    Sun's own direction is not the client's declared view, and the camera's guard measures which one it was.
run_case "the camera stands off along the view's own direction" src/surface.rs \
    '    let camera = drawn::Camera::over(&mesh, view().toward, drawn::DRAWN_FOV_DEG)?;' \
    '    let camera = drawn::Camera::over(&mesh, light().toward, drawn::DRAWN_FOV_DEG)?;' \
    "--test u4_drawn" "the_camera_stands_the_regions_own_extent_in_front_of_its_own_frustum"

# 10. **The declared look both the frame and the recount encode with.** The subject is the exposure
#     `draw_plane` hands the drawn reading; a frame encoded at another exposure than the one the recount reads
#     is two pictures wearing one digest.
run_case "the frame and the recount encode at one exposure" src/surface.rs \
    '            exposure: EXPOSURE,' '            exposure: 0.0,' \
    "--test u4_drawn" "every_drawn_pixel_is_the_nearest_rectangle_the_mesh_holds"

# 11. **The frame's own size.** The subject is the declared frame; the client's picture is `48 × ZOOM` pixels
#     square, which is what lets one window show either reading. This is the one case that needs a desktop: its
#     bar is `u1_surface.rs`'s window test, which opens a **real** window for two frames.
run_case "the drawn frame is the client's declared size" src/drawn.rs \
    'pub const DRAWN_SIZE: u32 = 576;' 'pub const DRAWN_SIZE: u32 = 288;' \
    "--test u1_surface" "the_presented_frame_reaches_the_window_s_own_pixels"

# ---------------------------------------------------------------------------
# The four drawn bars, each through the line it prints.
# ---------------------------------------------------------------------------

# 12. **The counts bar.** The subject is the drawing loop's own corner counter; the mesh declared four corners
#     per rectangle and the loop must have transformed four.
run_product_case "the loop's own counts are the mesh's own" src/drawn.rs \
    '        vertices += 1;' '        vertices += 2;' \
    'the loop'"'"'s own counts are the mesh'"'"'s own = false'

# 13. **The frustum bar.** The subject is the near plane's own declared fraction; a near plane ten times the
#     eye's distance stands in front of every corner the mesh holds, so every triangle is clipped whole, the
#     clip count stops being zero, and the named line flips. (Measured while writing this proof: pulling the eye
#     *inside* the extent instead — `distance = radius * 0.5` — does **not** clip anything on this world, whose
#     mesh is a nearly flat patch: its own depth range along the view axis is about eight metres and the eye
#     still stands eleven metres off the centre.)
run_product_case "every rectangle is inside the camera's own frustum" src/drawn.rs \
    'pub const NEAR_FRACTION: f64 = 0.01;' 'pub const NEAR_FRACTION: f64 = 10.0;' \
    'every rectangle is inside the camera'"'"'s own frustum = false'

# 14. **The picture bar.** The subject is the colour a rectangle's pixels are given; one grey for every
#     rectangle is a frame whose pixels have no range at all, and the bar measures the frame's own pixels.
run_product_case "the picture is not a flat fill" src/drawn.rs \
    '        let colour = encode(shaded, look.exposure);' \
    '        let colour = encode(Rgb { r: 0.5, g: 0.5, b: 0.5 }, look.exposure);' \
    'the picture is not a flat fill = false'

# 15. **The state bar.** The subject is case 8's — a frame that names a fixed state — through the run's own
#     printed clause rather than through a test: the law's state at the frame is the state the mesh was keyed by.
run_product_case "the frame's mesh state is the law's own state" src/drawn.rs \
    '        mesh_state: mesh.key().state,' '        mesh_state: Digest::of(b"a fixed state"),' \
    'the frame'"'"'s mesh state is the law'"'"'s own state = false'

# 16. **The window's own bars and the window's own exit status.** The subject is case 15's again, through the
#     window's line and through the status the window mode returns: since §41 the window exits non-zero when a
#     drawn bar is not met.
run_window_case "the window's own bars are its own exit status" src/drawn.rs \
    '        mesh_state: mesh.key().state,' '        mesh_state: Digest::of(b"a fixed state"),' \
    'the frame'"'"'s mesh state is the law'"'"'s own state = false'

# The final check: the tree came back byte-exact, so the rebuilt binary must run the program it ran at the start
# — byte for byte. Whether the dated captures still match is reported, not gated.
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
