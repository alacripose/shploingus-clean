# PATCH-056 — the code that was paying for itself

The mission this pass was given, 2026-10-22:

> *"Twelve passes have added instrumentation, one-off diagnostic examples and dead code around a
> frame that is now settled at 8.6 ms — including the zero-caller `draw_merged` compute pipeline
> (unsafe to call as written), the unused `hold`, and any report lines or examples that nothing
> reads. Delete what does not earn its bytes, and where something is kept, say why in one line; if
> removing the dead compute pipeline leaves the mission's GPU clause unserved rather than merely
> unimplemented, say that plainly instead of letting the code imply otherwise."*

# Removed

| what | lines | why it was dead |
|---|---|---|
| `WorldPipeline::draw_merged` (`world.rs`) | 97 | **zero callers**, and wrong as written: it passed the *vertex* bytes as the quad storage buffer (`&scene.vertex_bytes()` under a `// TODO: use actual quad data`), bound the record-format bind group to a compute pipeline whose layout it never declared (`layout: None`), created the merge pipeline *per frame*, and cleared the colour target that `draw` fills with sky. Nothing could have called it and got a picture |
| `merge.wgsl` | 167 | the compute shader above's only reader, and it merged nothing: it wrote to the same storage buffer it read, dispatched over garbage |
| `WorldPipeline::upload` (`world.rs`) | 17 | one-shot buffer creation, and its only three callers were inside `draw_merged` |
| `hold` + `grown` (`world.rs`) | 27 + 9 | `hold` was the arena's old growth path and had no caller (`cargo build` said `function \`hold\` is never used`); `grown` had none but `hold`'s, and a pin that tested only arithmetic about a function nothing ran |
| `a_held_buffer_grows_by_doubling_and_never_shrinks` | 21 | asserted that arithmetic and nothing else; both pairs now size themselves once at construction (PATCH-054), so the doubling rule has no caller left to describe |
| `examples/msaa-probe.rs` | 58 | a one-off adapter format-feature probe; no document, patch or test cites it, and the pipeline's sample count is asserted by the renderer's own tests |
| the sentence naming the merge pass in `PRESENTING`'s doc | 1 | it described a pass that no longer exists (now: the wireframe and the absent cloud pass only) |

**Net: 2 files and 391 lines fewer** (`world.rs` 2533 → 2353, `scene.rs` +14 for the repaired pin
below, `merge.wgsl` −167, `msaa-probe.rs` −58). Build and `cargo test -p voxel-client --lib` green;
`cargo build -p voxel-client --examples -j 1` builds all 19 remaining examples.

# Kept, and why in one line each

- `corner_traffic`, `corner_allocations`, `corner_retained_bytes`, `quad_traffic`, `quad_path`,
  `quad_split`, `upload_split`, `upload_pending` — every one is read by the `geometry`,
  `frame-work` or `upload-split` line the run is judged on, or by a pin.
- `examples/light-budget.rs`, `light-shot.rs`, `shell-shot.rs`, `face-*.rs`, `pattern-flicker.rs`,
  `measured-light.rs` — each is cited by the patch whose evidence it is (0 references for
  `light-budget.rs` in docs, but it is the frame budget's own tool and is tracked in git).
- `CornerPair::ensure_vertices` / `ensure_indices` — they report *whether they allocated*, which is
  the one thing a frame must never do and the number `corner_allocations` counts.

# One pin repaired rather than deleted

`scene::tests::the_fades_cost_is_the_second_trees_records_and_nothing_else` failed in the full suite
(526 passed, 1 failed, twice) and passed alone. Measured: **32.4 ns/record alone against 36.8 for a
cross-fade** — a copy; **65.5 against 171.1 inside the suite**. The fade's 80 000-record allocation
lands on fresh pages when the suite has fragmented the heap, so the assertion was measuring the
allocator, not the claim. The claim — "a cross-fade appends the second tree's own records, it does not
re-mesh it" — is now asserted as **bytes**: every record past the first tree's must be that tree's
own `(face, material, slice, uv, extent)` rebased onto the first tree's origins. A re-mesh fails it;
an allocator cannot. The timing is still printed on every run.

# Verified

- `cargo build -p shploingus --offline`: green, census/conformance/prose-frame/missing-docs pins hold.
- `cargo test -p voxel-client --lib --offline`: **527 passed, 0 failed, 2 ignored**.
- Loaded run, final build, 8 `yes` processes, `SHPLOINGUS_STALL_MS=50`, 80 s — `C:/tmp/final3-loaded.log`:

| | before (PATCH-055) | after |
|---|---|---|
| settled | 8.6 ms of 115 frames, 0 stalls | **8.8 ms of 112 frames, 0 stalls** |
| frames over 50 ms | 230 (worst 178.8, boot) | **5** (boot 142.8 + four at t9673–t9677, 50.7–64.4) |
| world | 605/605, 57 175 quads, digest `7c66a632336cc0a3` | **605/605, 57 175 quads, digest `7c66a632336cc0a3`** (30 report lines) |
| memory | mesh 132.0 / 256.0 MB, corner pairs 72.0 MB | **mesh 131.0 / 256.0 MB, corner pairs 72.0 MB** |
| light | 0 hops past a non-resident chunk; settled 0 measured / 0 queued | **same**: 0 hops in every interval, 56 222 cells measured at peak |
| record upload | 468 uploads, 70 re-encoded, median 0.40 ms | **543 uploads, median 0.42 ms, max 5.06 ms** |

The stall count moving from 230 to 5 is **not** attributable to this pass — the deleted code never
ran; the two runs differ by machine state, and the settled mean, residency, digest and memory are the
numbers that must not move, and they do not.

# What is still owed, stated plainly

**The GPU merge clause is now unserved, not merely unimplemented.** There is no GPU-side quad merge
in this tree at all: `mesh.rs`'s greedy merge (rows then columns, over leaves) runs on the CPU in
`cloud_cubes`, and the shader that was supposed to merge on the device never merged anything and was
never called. Any statement that this project merges geometry on the GPU is now false, and the
deletion is the statement of that.
