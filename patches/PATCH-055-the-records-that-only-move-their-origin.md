# PATCH-055 — the records that only move their origin

The mission this pass was given, 2026-10-22:

> *"Under eight busy cores a stream-in frame still reaches 69–104 ms because `write_quads` re-serialises
> and copies all 57 175 records each time a chunk arrives (14–40 ms) instead of only what moved; make
> that upload incremental … so an arriving chunk costs its own records and not the world's. Treat the
> ordering risk as the real enemy: a partial record upload must never leave a frame drawing records
> from two different states, an index range pointing past what was sent, or a stale tail after a fade
> ends — prove that with a test that fails if a whole-array reserialisation or a torn range returns."*

# What the trace said, and what it corrected

Instrumented per upload (`WorldPipeline::quad_path`, `quad_split`, printed as `upload-split`), a
loaded run's 468 record uploads split 398 patched to 70 encoded. Between two consecutive scenes the
rectangles are **the same rectangles in a different order**: a chunk's quads are the chunk's quads,
and what moved is the index into the origin array, because the walk that built them visited the
chunks differently. So the value diff the upload used to run had nothing to skip — four bytes a record
did. 2 358 064 records were patched where 26 700 000 would have been serialised.

The premise the mission carried — that the arrival itself was the 14–40 ms — is only half true, and
the numbers say which half: `write_quads` measures **0.40 ms median, 1.92 ms mean, 24.0 ms peak** over
74 sampled intervals, and its own peak lands on the arrival frames that *do* change geometry (70
encodes, worst encode 25.4 ms). Those arrivals insert records mid-array; a prefix patch cannot carry
them, and the tail behind the insertion is re-serialised.

# What changed

| file | the change |
|---|---|
| `crates/voxel-client/src/world.rs` | `update_record_bytes` — the whole incremental upload: the longest leading run of records that kept its shape is **reused from the staging bytes**, the origin slot of each record whose chunk index moved is patched in place (`patch_chunk`, `CHUNK_SLOT`), the old tail is **truncated** (a fade that ends must not leave a record the draw could read), and only what is past that run is serialised and appended. `RecordPath` states the two outcomes; `Unchanged` is gone, because the caller only calls when the values differ, so a body arriving beside unchanged records is still an upload and was silently uncounted |
| `crates/voxel-client/src/world.rs` | `quad_path()`, `quad_split()` — patched-vs-encoded and encode-vs-transfer, so the cost is read rather than inferred from `draw`. `quad_staging` is the buffer the bytes are rebuilt in, taken and returned, so an upload allocates nothing |
| `crates/voxel-client/src/world.rs` | three pins: `a_patched_upload_is_the_encoded_array_byte_for_byte` (renumbered, grown, **shrunk**, edited rectangle, no shared prefix, and a body beside unchanged records), `a_patch_touches_the_origin_slot_and_nothing_else` (every byte a patch moves is inside the four-byte origin slot, which is what holds `CHUNK_SLOT` to the record's own field order), and `a_patched_record_buffer_draws_the_frame_an_encoded_one_draws` (the same scene sent both ways through the real pipeline, compared pixel for pixel) |
| `crates/shploingus/src/main.rs` | the `upload-split` line states uploads, bytes, growths, arrays re-encoded and records patched beside the encode and transfer times |

**Why the ordering risk is not bought off by a budget.** The corner upload is sliced across frames
because a 20 MB scene cannot be sent atomically; the record upload is not, and is not merely because
it is cheap. A sliced record array is two states in one draw — exactly the torn range the mission
names — so the array leaves in **one** `write_buffer`, which wgpu orders before the pass: a frame
draws the array it was handed or the one before it, never a mix. The bytes are correct by
construction (prefix reused byte for byte, tail truncated, remainder appended), and
`a_patched_record_buffer_draws_the_frame_an_encoded_one_draws` is the pin that would notice if they
were not.

# The numbers

Same machine, dev profile, 16 logical cores, `SHPLOINGUS_FRAMES=1 SHPLOINGUS_STALL_MS=50`, 80 s run
under 8 `yes` processes, **4 850 measured frames**. Before = the PATCH-054 build.

| | measured |
|---|---|
| record uploads | 468 uploads, 738.8 MB, **2 growths**, **70 re-encoded**, **2 358 064 records patched** |
| `write_quads` per sampled interval (74) | median **0.40 ms**, mean **1.92 ms**, max **24.04 ms** |
| last upload's own split | **encode 0.23 ms + transfer 0.19 ms**; corners **0.00 ms** of the last frame |
| stream-in frames over 50 ms | **227** (228 counting the 173.9 ms at t9808), worst **173.9 ms** — `draw` 80.1, `shell` 31.1, `judge` 21.3, `scene` 14.9, `camera` 14.6 |
| the same 228 frames, by phase (mean) | `draw` **24.6** \| `shell` **13.6** \| `judge` 6.8 \| `scene` 6.5 \| `camera` 4.0 \| `stream` 2.1 \| rest ≤ 0.5 |
| over 60 / 70 / 80 ms | 77 / 24 / 6 |
| last frame over the ceiling | t9822; **33 clean intervals after it** (~3 800 frames), settled mean **8.6 ms** of 115 frames (`shell` 5.2, `draw` 1.05) |
| world | **605/605**, 0 stale-but-drawn, 57 175 quads, digest **7c66a632336cc0a3**, 0 in flight |
| light | 0 hop(s) past a chunk not resident in all 74 intervals; settled window 0 measured / 0 behind / 0 queued |
| memory | mesh **132.0 of 256.0 MB**, scratch 0.0 MB, corner pairs **72.0 MB retained, 4 allocations since start** |

**What is left under the ceiling is not the upload.** On the frames that miss it, `draw` averages
24.6 ms against a record upload whose own median is 0.40 ms, and `shell` averages 13.6 — the GPU pass
and the interface, under eight cores competing for both. Making the remaining 70 arrivals incremental
would remove at most ~25 ms from 6 of the 80 frames over 80 ms; it would not bring the stream-in
distribution under 50.

# Verified

- `cargo test -p voxel-client --lib --offline`: **528 passed, 0 failed, 2 ignored** (the two `world`
  pins above included, and the GPU pin actually draws).
- `cargo build -p shploingus --offline`: green, with the unused `mut` at the boot `Graphics::new`
  removed (`main.rs`'s only warning).
- The run above is of that build (`C:/tmp/final2-loaded.log`); the digest, the chassis count and the
  corner-pair ledger are unchanged.

# What it does not do

- **An arrival that inserts records mid-array still serialises the tail** (70 encodes, worst 25.4 ms).
  The fix is per-chunk record slots — an array that appends a chunk's records at the end of the chunk
  region instead of in walk order — which also moves the cloud and body suffixes into their own
  buffers. That is a scene-layout change, not an upload one, and it is the next pass's.
- **The 50 ms ceiling is still missed 227 times during stream-in**, worst 173.9 ms, and the trace
  puts that on `draw` and `shell` rather than on either upload.
- **The record upload is not budgeted**, deliberately: one atomic write of ≤1.4 MB is the ordering
  guarantee, and slicing it would trade 0.4 ms for a torn frame.
