#!/usr/bin/env python3
"""**Export PATCH-078, the tree mob, as one `.patch`.**

    python patches/PATCH-078-export.py > patches/PATCH-078-the-tree-that-carries-its-own-origin.patch
    python patches/PATCH-078-export.py --base <dir>      # write the base tree it diffs against

The base is this tree with this change set reverted: `HEAD` for the files no other pass touched,
and otherwise the working copy minus the lines and blocks this pass added. The patch itself is a
`git diff` between two scratch commits — the base, and the base with this pass's files copied in,
`flora.rs` removed, and the two new files added — so its headers, modes and paths are git's own.

Proof, all three:

    (cd $BASE && git apply --check -p1 <patch>)                     # into a clean base
    (cd $APPLY && git -c core.autocrlf=false apply -p1 <patch> && cmp each file)
    git apply --check --reverse -C1 --ignore-whitespace <patch>     # the + side is this tree
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile

FROM_HEAD = [
    "crates/voxel-mesh/src/mesh.rs",
    "crates/voxel-shapers/src/chunk_fields.rs",
    "crates/voxel-shapers/src/lib.rs",
    "crates/voxel-shapers/src/spec.rs",
]

DELETED = ["crates/voxel-shapers/src/flora.rs"]

NEW_FILES = [
    "crates/voxel-shapers/src/gaia.rs",
    "crates/shploingus/src/forest.rs",
]

# Files whose `Mesh` literals gained an `origin:` line or block this pass. The base is the
# working copy with both forms removed, because a base `Mesh` had no origin field at all.
STRIPPED = [
    "crates/voxel-client/src/scene.rs",
    "crates/voxel-client/src/renderer.rs",
    "crates/voxel-client/src/world.rs",
    "crates/voxel-client/src/far_surface.rs",
    "crates/voxel-stream/src/resident.rs",
    "crates/voxel-client/examples/face-pattern.rs",
    "crates/voxel-client/examples/face-square.rs",
    "crates/voxel-client/examples/gpu-shadow-check.rs",
    "crates/voxel-client/examples/shadow-lattice.rs",
]

REVERTS = [
    ("crates/shploingus/src/main.rs", "\nmod forest;\n", ""),
    (
        "crates/shploingus/src/main.rs",
        "    /// **The tree mob**: Gaia's records drawn as bodies at their own origins (docs/trees.md).\n"
        "    forest: forest::Forest,\n",
        "",
    ),
    ("crates/shploingus/src/main.rs", "        forest: forest::Forest::new(),\n", ""),
    (
        "crates/shploingus/src/main.rs",
        "        // ---- the forest: Gaia's records, one body a frame ------------------\n"
        "        // A tree is a being with an id and an origin; its body is drawn from that record and\n"
        "        // placed at the tree's own corner, and a cell the eye has left loses its body.\n"
        "        {\n"
        "            let spec = *self.spec();\n"
        "            let eye_m = self.camera.position_m;\n"
        "            self.forest.update(&spec, eye_m);\n"
        "        }\n",
        "",
    ),
    ("crates/shploingus/src/main.rs", "                .chain(self.forest.bodies());\n", ""),
]

MODIFIED = FROM_HEAD + ["crates/shploingus/src/main.rs"] + STRIPPED


def read(path):
    with open(path, encoding="utf-8", newline="") as handle:
        return handle.read()


def head_of(path):
    return subprocess.run(
        ["git", "show", f"HEAD:{path}"], capture_output=True, check=True
    ).stdout.decode("utf-8")


def base_of(path):
    if path in FROM_HEAD:
        return head_of(path)
    text = read(path)
    for name, current, base in REVERTS:
        if name == path:
            text = text.replace(current, base)
    if path in STRIPPED:
        text = re.sub(
            r"(?m)^[ \t]*origin: \{\r?\n[ \t]*let o = [^\n]*\n[ \t]*\[o\.x, o\.y, o\.z\],[^\n]*\n[ \t]*\},\r?\n",
            "",
            text,
        )
        kept = []
        for line in text.splitlines(keepends=True):
            if (
                re.match(r"\s+origin: .*,\s*$", line)
                and kept
                and re.match(r"\s+chunk: .*,\s*$", kept[-1])
            ):
                continue
            kept.append(line)
        text = "".join(kept)
    return text


def stage(root):
    for path in MODIFIED + DELETED:
        base = base_of(path) if path in MODIFIED else head_of(path)
        target = os.path.join(root, path)
        os.makedirs(os.path.dirname(target), exist_ok=True)
        with open(target, "w", encoding="utf-8", newline="") as handle:
            handle.write(base)
    out = subprocess.run(["git", "init", "-q", "."], cwd=root, check=True)
    subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    subprocess.run(
        ["git", "-c", "user.name=export", "-c", "user.email=export@local", "commit", "-qm", "base"],
        cwd=root,
        check=True,
    )


def write_tree(root):
    for path in MODIFIED + NEW_FILES:
        target = os.path.join(root, path)
        os.makedirs(os.path.dirname(target), exist_ok=True)
        shutil.copyfile(path, target)
    for path in DELETED:
        subprocess.run(["git", "rm", "-q", path], cwd=root, check=True)


def main():
    dump = None
    if "--base" in sys.argv:
        dump = sys.argv[sys.argv.index("--base") + 1]
    with tempfile.TemporaryDirectory() as scratch:
        root = os.path.join(scratch, "repo")
        os.makedirs(root)
        stage(root)
        if dump:
            # **The text this pass started from, not the scratch copy.** `git add` may rewrite a
            # working tree's line endings as it touches it, and a base dumped afterwards would
            # then refuse hunks whose context is right.
            for path in MODIFIED + DELETED:
                # **The staged bytes**, which are the patch's own `a` side: `git add`
                # normalises a working tree's line endings into the index, and a base dumped
                # from the working copy instead would refuse hunks whose context is right.
                base = subprocess.run(
                    ["git", "show", f":{path}"], cwd=root, capture_output=True, check=True
                ).stdout
                target = os.path.join(dump, path)
                os.makedirs(os.path.dirname(target), exist_ok=True)
                with open(target, "wb") as handle:
                    handle.write(base)
        write_tree(root)
        subprocess.run(["git", "add", "-A"], cwd=root, check=True)
        run = subprocess.run(
            ["git", "diff", "--cached", "--no-color", "--no-ext-diff"],
            cwd=root,
            capture_output=True,
            check=True,
        )
        sys.stdout.buffer.write(run.stdout)


if __name__ == "__main__":
    main()
