#!/usr/bin/env python3
"""**Export PATCH-077 as one `.patch`, carrying this thread's changes and nothing else.**

    python patches/PATCH-077-export.py > patches/PATCH-077-the-horizon-that-reads-the-subvoxels.patch
    python patches/PATCH-077-export.py /dev/null --dump /tmp/base      # the pre-thread base, for checking

This checkout cannot be exported with `git diff`: `HEAD` is a backup from before several passes
(1 113 paths differ), and the four tracked files here carry other passes' work inline. So the
pre-thread text of **every region this thread touched** is written out below — each block is the
text that stood there before, which is why the base is exact rather than approximated — and the
script rebuilds the five files as they stood, diffs them against the tree, and refuses if any
anchor has moved or if a name this thread introduced survives into the rebuilt base.

`crates/voxel-client/src/horizon.rs` is untracked in this checkout, so it travels whole, as the
file this record describes.

Verification (neither writes; both must pass):

    python patches/PATCH-077-export.py > /tmp/p077.patch
    git apply --check --reverse -C1 --ignore-whitespace /tmp/p077.patch   # the + side is the tree
    git apply --check -p1 /tmp/p077.patch                                 # into a clean copy of the base
"""
import os
import subprocess
import sys
import tempfile

NEW_FILES = [
    "crates/voxel-client/src/horizon.rs",
    # The horizon's shader, given its own file this pass: the block below was cut out of
    # `shading.wgsl` verbatim and the base is reconstructed from it (see `SPLIT`).
    "crates/voxel-client/src/horizon.wgsl",
    # The far surface: the coarse level of a resident chunk, its own module.
    "crates/voxel-client/src/far_surface.rs",
]

# **A block that moved files.** The base of the target is the tree with the source's body put
# back where it stood — `skip` lines of the source's own header, before the anchor that followed
# the block. A moved block cannot be expressed as a region: its anchors are gone from the tree.
SPLIT = {
    "crates/voxel-client/src/shading.wgsl": ("crates/voxel-client/src/horizon.wgsl", 5, "struct Light {"),
}

# (start marker, end marker or "" for end-of-file, text that stood between them before this thread)
REGIONS = {
    # Moved block: reverted by `SPLIT`, so it carries no regions of its own.
    "crates/voxel-client/src/shading.wgsl": [],
    "crates/voxel-client/src/world.rs": [
        # The resident set the volume now prunes itself against.
        ("use std::collections::BTreeSet;", "use crate::skylight::", ""),
        # `horizon_retain` — the eviction half of streaming.
        (
            "    /// **Forget every chunk the resident set no longer holds**",
            "    /// Whether a chunk's cube still needs this version of its tree",
            "",
        ),
        # `wants`/`stamp` took a version; before this thread they took a position alone.
        (
            "    /// Whether a chunk's cube still needs this version of its tree",
            "    /// The world-voxel corner the cube is anchored on",
            """    /// Whether a chunk has yet to be stamped into the cube.
    pub fn horizon_wants(&self, pos: voxel_model::ChunkPos) -> bool {
        self.horizon.wants(pos)
    }

    /// **Stamp one resident chunk's subvoxel octree into the light horizon.**
    ///
    /// Returns the cells the chunk marked. A chunk the cube does not cover, or one already
    /// stamped, costs nothing and marks nothing.
    pub fn horizon_stamp(
        &mut self,
        pos: voxel_model::ChunkPos,
        tree: &voxel_mesh::SparseVoxelOctree,
    ) -> usize {
        self.horizon.stamp(pos, tree)
    }

""",
        ),
        # The test that proves the octree's own subvoxels, not one word per metre, reach the march.
        ("    /// **A dug quarter lets its light through", "", "}"),
    ],
    "crates/voxel-client/src/lib.rs": [
        # The crate's own export list, before this thread added the band and subvoxel constants.
        (
            "pub use horizon::{",
            "pub use measure::{",
            """pub use horizon::{
    HorizonStream, HORIZON_CELLS, HORIZON_CELL_M, HORIZON_REACH_M, SKY_RAYS,
};

""",
        ),
        # The far surface's own export list, absent from the base.
        ("pub use far_surface::{", "pub use field::{", ""),
    ],
    "crates/voxel-work/src/stream.rs": [
        # The residency counter a stamp is keyed by.
        (
            "    /// **The version of the tree a chunk holds**",
            "    /// **The same tree as a handle**",
            "",
        ),
    ],
    "crates/shploingus/src/main.rs": [
        # One chunk, one level: the fine mesh inside the handover ring, the coarse one outside.
        (
            "            let light: Option<&voxel_client::LightAt> = None;",
            "                eye_chunk,\n                light,",
            """            let light: Option<&voxel_client::LightAt> = None;\n            self.resident.sync(\n                self.streamer.as_ref().expect("checked above").drawn_meshes(),\n""",
        ),
        # Eight stamps a frame became the whole box, keyed by version, pruned by residency.
        (
            "        // **The frame's own occlusion field, moved with the eye",
            "        // ---- the frame's residency change",
            """        // **The frame's own occlusion field, moved with the eye and fed by the chunks the
        // streamer holds.** The cube is recentred on the eye's chunk; a recentre clears it, and
        // the loop below stamps the chunks that are resident and not yet represented, a bounded
        // number per frame. That is what "streamed" means here: the light the fragment marches
        // arrives with the world rather than in one build behind it, and a chunk that has not
        // landed yet reads as air for the frames it is absent (its direct shadow is still the
        // shadow map's, which is drawn from the geometry that is present).
        const HORIZON_CHUNKS_PER_FRAME: usize = 8;
        if let Some(graphics) = self.graphics.as_mut() {
            let eye = self.camera.position_m;
            graphics.world.horizon_recenter(eye);
            if let Some(streamer) = self.streamer.as_ref() {
                let mut budget = HORIZON_CHUNKS_PER_FRAME;
                for pos in streamer.tree_chunks() {
                    if budget == 0 {
                        break;
                    }
                    if !graphics.world.horizon_wants(pos) {
                        continue;
                    }
                    if let Some(tree) = streamer.tree_shared(pos) {
                        graphics.world.horizon_stamp(pos, &tree);
                        budget -= 1;
                    }
                }
            }
        }

""",
        ),
    ],
}

# Names and literals this thread introduced; a rebuilt base must contain none of them.
MARKERS = [
    "HORIZON_SUBVOXEL_SIDE",
    "HORIZON_SUBVOXEL_M",
    "HORIZON_FAR_CELL_M",
    "HORIZON_FAR_REACH_M",
    "HORIZON_MAX_STEPS",
    "HORIZON_SUB_STEPS",
    "CHUNK_SIDE_CELLS",
    "FAR_MEASURED",
    "CHUNK_SLOTS",
    "horizon_march",
    "horizon_walk",
    "horizon_stamped",
    "horizon_mask",
    "horizon_subvoxel_solid",
    "horizon_partial_blocked",
    "horizon_transmittance",
    "horizon_dome",
    "horizon_retain",
    "tree_version",
    "horizon_version",
    "to_far",
]


def eol_of(text):
    return "\r\n" if "\r\n" in text else "\n"


def rebuild(path, regions):
    with open(path, encoding="utf-8", newline="") as handle:
        current = handle.read()
    eol = eol_of(current)
    base = current
    if path in SPLIT:
        source, skip, anchor = SPLIT[path]
        with open(source, encoding="utf-8", newline="") as handle:
            moved = handle.read()
        # The source's own line endings, not the target's: a file that is partly CRLF splits
        # on `\r\n` alone into one piece, which would silently move nothing.
        src_eol = eol_of(moved)
        block = src_eol.join(moved.split(src_eol)[skip:]).replace(src_eol, eol)
        if anchor not in base:
            raise SystemExit(f"{path}: the moved block's anchor is gone: {anchor}")
        base = base.replace(anchor, block + anchor, 1)
        return base
    for start, end, old in regions:
        if start not in base:
            raise SystemExit(f"{path}: the start anchor is not in the file: {start.strip()[:70]}")
        begin = base.index(start)
        if end:
            if end not in base[begin:]:
                raise SystemExit(f"{path}: the end anchor is not after the start: {end.strip()[:70]}")
            finish = begin + base[begin:].index(end)
        else:
            finish = len(base)
        base = base[:begin] + old.replace("\n", eol) + base[finish:]
    for marker in MARKERS:
        if marker in base:
            line = next(item for item in base.splitlines() if marker in item)
            raise SystemExit(f"{path}: the rebuilt base still says {marker}: {line.strip()[:80]}")
    return base


def unified(base_path, path):
    # **Bytes end to end.** Decoding the diff would fold a carriage return into a line feed and
    # rewrite the two files this patch is about; a CRLF source then no longer matches its own
    # context and `git apply` refuses it.
    run = subprocess.run(
        ["diff", "-u", "--label", f"a/{path}", "--label", f"b/{path}", base_path, path],
        capture_output=True,
    )
    if run.returncode not in (0, 1):
        raise SystemExit(f"diff {path}: {run.stderr.decode(errors='replace').strip()}")
    return run.stdout


def main():
    dump = sys.argv[2] if len(sys.argv) > 2 else None
    # **Bytes, not text**: on Windows a text stream rewrites every line feed as a carriage-return
    # pair, which changes the patch's own context. `git apply` then refuses hunks that are exactly
    # right. A new file's additions survive that rewrite; a modification's context does not.
    out = sys.stdout.buffer
    for new_file in NEW_FILES:
        new = subprocess.run(
            ["git", "diff", "--no-index", "--", os.devnull, new_file], capture_output=True
        )
        if new.returncode not in (0, 1):
            raise SystemExit(f"git diff {new_file}: {new.stderr.decode(errors='replace').strip()}")
        out.write(new.stdout)
    with tempfile.TemporaryDirectory() as scratch:
        for path, regions in REGIONS.items():
            base = rebuild(path, regions)
            base_path = os.path.join(scratch, os.path.basename(path) + ".base")
            with open(base_path, "w", encoding="utf-8", newline="") as handle:
                handle.write(base)
            # A `diff --git` header per file, so the export reads as one patch rather than four
            # unlabelled diffs; git derives nothing from it that `---`/`+++` did not already say.
            out.write(f"diff --git a/{path} b/{path}\n".encode("utf-8"))
            out.write(unified(base_path, path))
            if dump:
                target = os.path.join(dump, path.replace("/", os.sep))
                os.makedirs(os.path.dirname(target), exist_ok=True)
                with open(target, "w", encoding="utf-8", newline="") as handle:
                    handle.write(base)


if __name__ == "__main__":
    main()
