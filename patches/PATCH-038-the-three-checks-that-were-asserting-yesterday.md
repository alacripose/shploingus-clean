---
patch_id: PATCH-038
title: The three checks that were asserting yesterday's tree — a fixture race and two stale premises
status: applied
operation: change
target: shploingus/test-integrity
base_ref: shploingus-clean@main
base_digest: sha256:759371d60917e73300f618ce5643997f70fcffc0b7bca65986e4e78674194c78
dependencies: [PATCH-022, PATCH-036, PATCH-037]
source: user-directed-conversation
created_at: 2026-10-10
approval: user-directed
live_mutation: false
---

# Change

Running every suite of the two crates this round touched — rather than the filtered tests that had
been standing in for them — found three checks that were not measuring what they claimed. **Two were
passing for the wrong reason and one was failing for the right one.**

**1. A fixture race in the placement tests, and it was my own, from this morning.** `patch.rs`'s
`a_tree_of_files()` built one directory, `std::env::temp_dir()/placement-<pid>`, and both placement
tests call it — then each removes it. Under the default test threading one test rewrites the tree
while the other walks it, so `placed_files` came back short by whatever the other test's rewrite had
removed mid-walk. It surfaced as `a_directory_the_record_names_places_the_files_under_it` failing
`placed_files 1 == 2`, intermittently: **it passed in four consecutive filtered runs and failed in
the full suite.** The fixture now takes a name (`space.rs`'s own convention, which I had not
followed), so each test owns its directory.

**2. `protocol.rs` hardcoded a count that a backwalk falsified.** The ownership test asserted
`ownership\t0 record(s)` for `crates/voxel-audio/src/lib.rs` — true when it was written, and false
since PATCH-036 named that file. The surface was answering correctly and the test was asserting a
memory. It now derives the expectation from the record set **by the tool's own rule** (a record that
names the path, or whose declared `target` names the crate segment), and its zero case is a crate
that does not exist, with that precondition asserted from the records rather than assumed — because
**every real crate is now named by some record**, which is itself a fact this round's placement work
established.

**3. `api_layer.rs` asserted that the tree refuses, and the tree had stopped.** It read the live
verdict and required `rejected()` to be non-empty — the *"this tree is mid-retrofit"* premise. When
`conformance/accepted-refusals.txt` landed on 2026-10-09 every one of the twelve refusals was
accepted by name, so `rejected()` became empty and the test failed on a door working exactly as
declared. The mechanism is what needs testing, so it is now driven against **the door with nothing
accepted** (`conformance::of(&census, &[], false)`), which is the state the mechanism exists for and
which no acceptance list can move.

# Acceptance

- `cargo test -j 2 -p voxel-package` — **321 lib tests plus every integration suite green**, and the
  placement filter run **four consecutive times** without a flake.
- `cargo test -j 2 -p voxel-mcp` — **8 (`api_layer`) + 10 (`protocol`) green.**
- No assertion was weakened to get there: the placement counts still add up per kind, the ownership
  count is now *stronger* (it checks the ids the surface prints, not just a total), and the api_layer
  test still asserts the tree refuses twelve cells.

# Evidence

- The failing run, unedited: `test patch::tests::a_directory_the_record_names_places_the_files_under_it`
  → `assertion left == right failed: both files are under the named directory, left: 1, right: 2`.
- The tool's own answer that killed the hardcoded zero:
  `path\tcrates/voxel-audio/src/lib.rs\ncrate\tvoxel-audio\npatch\tPATCH-036\tapplied\tshploingus/voxel-audio\nownership\t1 record(s) name this path`.
- The gate's conformance lines that killed the other premise: `conformance-refused\t12`,
  `conformance-accepted\t12`, `conformance-rejected\t0`, `conformance-accepted-list\tconformance/accepted-refusals.txt`.
- `grep -rl <crate> patches/` over all twelve crates: **every crate is named by 2–4 records** — the
  reason the zero case is now a crate that does not exist.

# Rollback

Three test-side changes, no production byte among them: restore the unnamed fixture, the hardcoded
`0 record(s)`, and `conformance::verdict(&root)`. Each rollback restores a check that fails when the
tree moves — which is what they did.

# Not done, and named

- **The suites were run with `-j 2`, not at the default parallelism, and that is not a preference.**
  At `-j` the same `cargo test -p voxel-package` run failed with `can't find crate for std`,
  `only metadata stub found for rlib dependency hashbrown`, and `error: internal compiler error: no
  resolution for an import` — with `C:` at **99 % full, 7.7 GiB free**, and the kache store
  restoring cache hits by **copy**. Those errors are eviction, not code: the tree and the cache each
  cost their own bytes and the volume cannot hold a fully-parallel build of it. `cargo space --floor 20`
  freed 933 MiB (`space-freed 978579346`) and the suites then ran green. **This is row 24's cost,
  measured on a real build rather than argued**, and the honest statement is that a parallel build of
  this tree does not fit in the free space this machine has today.
- **`ort_sys` cannot be built as `rlib`** in this environment: the first `cargo test -p voxel-mcp`
  of the round refused before running anything. It cleared once the cache restored the artefact, so
  the cause is not established here — but a dependency that reports *"required to be available in
  rlib format, but was not found in this form"* under disk pressure is the same story as the item
  above, not a separate defect.
- **Nothing in this record makes a build fit.** It names the condition; the repair is row 24's
  un-taken switch or a smaller tree, and it is the user's call.
