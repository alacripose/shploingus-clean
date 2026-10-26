# The upstream format: code first, baselines as identity, patches as narrative

`patches/` is a chronological corpus: one record per change, each carrying its own story, its own
acceptance and its own evidence. That is the right shape for review and the wrong shape for
**reproducing a tree**, because the thing a reader needs first — *what bytes does this code produce?* —
is buried at the end of the newest record and has to be reassembled from a chain of them.

`patches/upstream/` is the consuming format. A record here is keyed by **code seam**, and the byte
baseline is its identity. It does not replace the chronological records; it indexes and consumes
them, and every claim it makes cites a record that still exists.

## The order of a record, and why it is this order

1. **Baseline** — the measured readings a reader can reproduce: the command, the count, the digest,
   and the folder under `docs/baselines/` holding the captured output. First, because a claim without
   a reading is not evidence, and the reading is what a consumer checks against.
2. **Code** — the seam's files and what each owns, with the exported names. This is the primary key:
   a record is findable by *where the code is*, not by when the patch was written.
3. **Consumption** — the chronological patch records folded into this seam, with their statuses. A
   record naming a patch that does not exist is refused; a patch may appear in more than one seam
   only when it genuinely changed more than one.
4. **Boundaries** — the readings that differ from what a reader might assume, and every place the
   record deliberately stops. An absence is written as an absence, with the reason.
5. **Rollback** — the exact edits that undo the seam, named by file and by function.

## The rules

- **Sample, don't summarise.** Counts and digests are copied from a captured run in
  `docs/baselines/<seam>-<date>/`; a number with no capture folder is refused.
- **The baseline is reproducible from the record alone.** The digest rule is stated, not assumed. The
  current rule is `git ls-files -z | sort -z | xargs -0 sha256sum | sha256sum` over the tracked set.
- **One seam, one file.** `UPSTREAM-NNN-<seam>.md`. Two files naming the same seam are a defect of
  the index, not a variant.
- **Absence is stated at the seam it belongs to.** A record that stops short says where and why, and
  names the next slice — never a TODO without an owner seam.
- **The corpus index is generated from front matter.** `INDEX-NNN-*.md` reads `patch_id`, `title`,
  `status`, `operation`, `target`, `base_ref` from every `patches/PATCH-*.md`; a wrong row is a wrong
  front-matter field, which is the property the format exists to expose.
- **`display-packages` is a baseline like any other.** Any record that adds a graded artifact class
  re-pins it, recording the old and the new digest in the same entry.

## What is consumed so far

| entry | seam | patches |
|---|---|---|
| `INDEX-001-the-corpus-consumed.md` | every `target:` in the corpus, 76 records / 53 seams | all |
| `UPSTREAM-001-the-one-rig-and-the-shell.md` | light law, shell drawing, cell frames | PATCH-071, 074, 075, 076 |
