#!/usr/bin/env bash
# **The guard proof for U1's visible client (2026-10-13, §36).**
#
# Each new guard is perturbed **in its own subject** — the world's own water read, the level water surface,
# the terrain's declared relief, the colourless-material refusal's own kind, the zoom and column-count
# guards, the declared frame count, the transfer function's own byte order, the screenshot's own channel
# order, the client area's own size, the blit's own scan lines, and the read-back's own comparison — and then
# the bar is run to show it **bites, and by name**. Every touched file is restored and its digest checked
# before the next case, and the script restores through a `trap`, so an interrupt cannot leave the tree
# perturbed.
#
# **The build is restored too**, and this case was learned the hard way (§36 records it): `cargo test`
# rebuilds the *binary* as well as the test harnesses, so a case that runs while a source is perturbed leaves
# `target/debug/shploingus-clean-room` as a **perturbed build** — and every later run of that binary then
# reports whatever the perturbation did, which is how a guard case was once mistaken for a rendering defect.
# Each case therefore ends by rebuilding from the restored sources.
#
# **Repaired 2026-10-13 (§37), at the cause rather than by re-taking this baseline.** The last check used to
# compare that rebuilt binary's output against `final-stdout.txt`, this baseline's frozen capture. That check
# passes only while the capture *is* the current bytes, so the first later pass that moves one printed line
# turns it red for a reason that is not a defect — and §37 moved one: the label `world cells_digest` became
# `surface read digest` (`LEDGER.md` C-189, ruling 2), with every figure on the line unchanged. The check now
# snapshots the binary's own output **at the start of this script, before any perturbation**, and requires
# the rebuilt binary to reproduce *that* byte for byte; whether the dated capture still matches is
# **reported, not gated**. This baseline's captures are untouched, and the pair §37 took is
# `docs/baselines/u2-cubic-cells/final-stdout.txt` beside its own `pre-stdout.txt`.
#
# Two cases run the client's own window test, which opens a **real** window for two frames: those two cases
# therefore need a desktop session, and they fail loudly (rather than passing silently) on a machine without
# one — which is `LEDGER.md` C-40's own rule about the client.
#
# The case logs are kept in the scratch directory this script makes and are **not** copied into the tree:
# they carry the run's own test-thread PIDs and cargo timings, so they are the one artifact of this pass that
# cannot be reproduced byte-for-byte (§33 recorded that trap). This script's own output is the proof.
#
# Usage: `bash docs/baselines/u1-visible-client/run-guards.sh` from anywhere. Exit 0 only when every case bit
# by name and every file came back byte-exact.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE="$ROOT/clean-room"
WORK="$(mktemp -d)"
BASELINE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$CRATE/target/debug/shploingus-clean-room"
FILES=(src/surface.rs src/main.rs src/lib.rs src/refusal.rs src/reference.rs src/evidence.rs src/systems.rs tests/u1_surface.rs tests/u5_measure.rs)

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
        printf '%s' "$out" | grep -E 'assertion .* failed|panicked at|^error|platform_refused|no_reflectance' | head -3 | sed 's/^/   bit: /'
    fi
    restore
    if ! (cd "$CRATE" && sha256sum -c "$WORK/digests.txt" > /dev/null 2>&1); then
        echo "   RESULT: THE TREE DID NOT COME BACK BYTE-EXACT"
        FAILED=1
    fi
    rebuild
}

snapshot

# **The build comes first** (§37's repair, 2026-10-13 — the same defect the new harness caught for real: its
# start snapshot had been taken from the binary an *earlier* build left on disk, so one moved printed line made
# the final check fail on a real difference that was not a defect of the run). `rebuild` restores the snapshot
# and builds from it, so what is captured below is this tree's own program.
rebuild

# The run's own output **before any perturbation** — what the rebuilt binary must reproduce at the end. (A
# dated capture is a measurement of one moment; this is a measurement of this run, and it cannot go stale.)
"$BIN" > "$WORK/start-stdout.txt" 2> "$WORK/start-stderr.txt"

# 1. The world's own water read: a frame in which no column stands water draws the world's own water as its
#    terrain's material — the subject is the wet test the material is chosen by.
run_case "the world's own water read" src/surface.rs \
    'let wet = world.depth_um(cell) > 0;' 'let wet = false;' \
    "--test" "u1_surface" "the_material_of_a_column_is_the_ladder_s_own_mapping_and_water_is_its_own"

# 2. The level water surface: water drawn with a slope of its own is not the level surface the contract's own
#    normal names — the subject is the wet column's own normal.
run_case "the level water surface" src/surface.rs \
    '    if wet {
        return [0.0, 0.0, 1.0];
    }' '    if wet {
        return unit([0.0, 1.0, 0.0]);
    }' \
    "--test" "u1_surface" "the_material_of_a_column_is_the_ladder_s_own_mapping_and_water_is_its_own"

# 3. The terrain's declared relief: with no exaggeration every land column's normal is level, so the world's
#    own slope no longer reaches the frame — the subject is the multiplier the normal is read with.
run_case "the declared relief" src/surface.rs \
    'pub const RELIEF_EXAGGERATION: f64 = 8.0;' 'pub const RELIEF_EXAGGERATION: f64 = 0.0;' \
    "--test" "u1_surface" "the_material_of_a_column_is_the_ladder_s_own_mapping_and_water_is_its_own"

# 4. The colourless material's own refusal name: a material that derives no colour recorded under another
#    kind — the subject is `no_reflectance` itself.
run_case "the refusal's own name (no_reflectance)" src/surface.rs \
    'RefusalKind::NoReflectance,
                    format!(
                        "material' 'RefusalKind::ReflectanceOutOfRange,
                    format!(
                        "material' \
    "--test" "u1_surface" "a_material_that_derives_no_colour_refuses_the_frame_by_name"

# 5. The zoom guard: a zoom of no pixels would draw a frame of nothing — the subject is the guard itself.
run_case "the zoom guard" src/surface.rs \
    'if zoom == 0 {' 'if false {' \
    "--test" "u1_surface" "a_zoom_of_none_and_a_column_count_that_is_not_the_world_s_are_refused_by_name"

# 6. The column-count guard: fewer material classes than columns would draw a frame the world did not ask
#    for — the subject is the guard itself.
run_case "the column-count guard" src/surface.rs \
    'if classes.len() != cells {' 'if false {' \
    "--test" "u1_surface" "a_zoom_of_none_and_a_column_count_that_is_not_the_world_s_are_refused_by_name"

# 7. The declared frame count: a client whose frame is a tick only reaches the plane's own rest if its count
#    is that horizon — the subject is the declared count.
run_case "the declared frame count" src/surface.rs \
    'pub const DECLARED_FRAMES: u32 = SETTLE_TICKS as u32;' 'pub const DECLARED_FRAMES: u32 = 1;' \
    "--test" "u1_surface" "the_declared_frame_count_is_the_horizon_the_run_settles_at"

# 8. The transfer function's own byte order: the buffer the platform is handed is BGRA, so red belongs in the
#    third byte — the subject is the encoding's own order.
run_case "the encoding's own byte order" src/surface.rs \
    '[channel(colour.b), channel(colour.g), channel(colour.r), 255]' \
    '[channel(colour.r), channel(colour.g), channel(colour.b), 255]' \
    "--lib" "a_flat_colour_encodes_to_the_byte_the_screen_is_handed"

# 9. The screenshot's own channel order: the P6 file the bar asks for is the presented frame in RGB — the
#    subject is the writer's own order.
run_case "the screenshot's own channel order" src/surface.rs \
    'bytes.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);' \
    'bytes.extend_from_slice(&[pixel[0], pixel[1], pixel[2]]);' \
    "--test" "u1_surface" "the_frames_own_bytes_are_the_screenshot_the_bar_asks_for"

# 10. The client area is the frame's own size: a window made one pixel wider than the frame is not the
#     picture, and the client refuses it — the subject is the client-area check.
run_case "the client area is the frame's" src/surface.rs \
    '|| client.right - client.left != width' '|| client.right - client.left != width + 1' \
    "--test" "u1_surface" "the_presented_frame_reaches_the_window_s_own_pixels"

# 11. The blit's own answer: the platform's own blit is checked against the frame's own height, so a blit
#     that copied anything else is a picture that did not reach the screen whole — the subject is that check.
run_case "the blit's own answer" src/surface.rs \
    'if lines != self.height {
                return Err(Refusal::new(
                    RefusalKind::PlatformRefused,
                    format!(
                        "the platform'\''s own blit copied' \
    'if lines != self.height + 1 {
                return Err(Refusal::new(
                    RefusalKind::PlatformRefused,
                    format!(
                        "the platform'\''s own blit copied' \
    "--test" "u1_surface" "the_presented_frame_reaches_the_window_s_own_pixels"

# 12. The screen holds the frame: a comparison that never agrees is a window that does not hold the surface's
#     own bytes, and the client refuses by name rather than reporting a frame nobody saw — the subject is the
#     read-back's own comparison.
run_case "the screen holds the frame" src/surface.rs \
    'if self.pixels[at..at + 3] == surface.pixels()[at..at + 3] {' \
    'if false && self.pixels[at..at + 3] == surface.pixels()[at..at + 3] {' \
    "--test" "u1_surface" "the_presented_frame_reaches_the_window_s_own_pixels"

# The last check is end to end rather than per file: the binary the restored tree builds is run, and its own
# output is compared against **the output the same binary produced at the start of this script** (§37's
# repair). A tree that came back byte-exact and a build that did not would pass every check above and still be
# the wrong program — and a dated capture cannot tell those two apart once any later pass moves a line.
echo "== the binary the restored tree builds"
set +e
"$BIN" > "$WORK/rebuilt-stdout.txt" 2> "$WORK/rebuilt-stderr.txt"
rebuilt_status=$?
set -e
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
        echo "    that is the capture's business, not this check's; §37 records the move and the repair)"
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
