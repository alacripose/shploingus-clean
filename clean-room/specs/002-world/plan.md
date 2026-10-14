# Plan 002 — U2 World model

## 1. Summary

One small, dependency-light module set that owns the world's nouns: a signed 32-bit coordinate
algebra (voxel → chunk → region, with the corrected 20-level ladder), a single immutable
`WorldIdentity` whose **truth** digest is a pure function of `(seed, declared period, tick)`, a
residency **cache** kept as a separate concern, a closed `Domain` set for deterministic sub-streams,
and one material registry with exact rational densities and sourced reflectances. Content identity
is the project's one hash, `sha2`.

## 2. Technical context

- **Language / edition:** Rust, edition 2021, `rust-version = 1.98`.
- **Dependencies:** **`sha2 = "0.10"`** — and the justification, beside the line: the design spec
  (§1.3) says the world is **rebuilt and reproduced with digests**, the repository's one accepted
  hash for content identity is `sha2`, and it is actually used (both `Digest::of_parts` and the
  registry digest call it). No other dependency. `--offline` resolves it from the local cache.
- **Persistence / transport:** none in this slice. The digest is the persisted identity that later
  systems write into a package.
- **Testing:** `cargo test --offline` (unit tests in each module); `cargo clippy --all-targets -- -D
  warnings`; `cargo run` prints the measured digests.
- **Target platform:** any (integer/fixed-point coordinate math plus one hash; no floating-point in
  the coordinate path).
- **Constraints:** refuse-never-default; determinism; one owner per concern; no legacy import.
- **Scale:** 32³ chunks, signed 32-bit coordinates, a 20-level ladder to 16 777 216 blocks, a 64-cell
  stored subvoxel lattice, a 256³ addressed detail lattice.

## 3. Constitution Check

| Principle | Status | Evidence / note |
|---|---|---|
| P1 Fresh code, declared provenance | PASS | each new module header names `docs/DESIGN-SPEC.md` §1–§5 and the LEDGER rulings it implements |
| P2 Refuse, never default | PASS | `Refusal`/`RefusalSet` with named kinds; no clamping anywhere (`refusal.rs`) |
| P3 Determinism is a contract | PASS | FR-005/FR-006/FR-007; SC-002/SC-003 measured by digest; the chain is a digest-linked ledger and its wire round-trips exactly |
| P4 Evidence over assertion | PASS | SC-001…SC-006 name a test or a printed digest |
| P5 One declared interface, two doors | PASS | one registry and one world owner; coordinates are the single algebra |
| P6 Smallest sufficient artifact | PASS | four source files; no stub (the U2 remainder is *named owed*, not scaffolded) |
| P7 Legacy is a witness, never a source | PASS | `clean-room/` remains its own workspace; no `path` dependency; no `crates/*` import |

## 4. Project structure

| File | Owns |
|---|---|
| `src/lib.rs` | the public API root: declares the modules, so the crate's surface is a real API (a library), not dead code in a bin |
| `src/digest.rs` | `Digest` — the one content identity (SHA-256) |
| `src/refusal.rs` | `Refusal`, `RefusalKind`, `RefusalSet` — the named-refusal vocabulary, collecting **all** |
| `src/coordinates.rs` | `VoxelPos`, `WithinChunk`, `ChunkKey`, `SubvoxelIndex`, `DetailIndex`, `RegionKey`, the ladder function, and the face-relations (`chunks_reached_by`) |
| `src/material.rs` | `MaterialId`, `Density`, `Reflectance`, `Material`, `MaterialRegistry`, `Digest`, the starter registry |
| `src/world.rs` | `DeclaredPeriod`, `WorldIdentity`, `Tick`, `Domain`, `World` (identity + truth digest + residency cache + `stage_seed`) |
| `src/profile.rs` | the shape laws: `Profile` (flat, cylinder, frustum/cone, spheroid, and the stalagmite — the `Da` law's exact consequences over a recorded elliptical flank), exact normals, the silhouette radius |
| `src/shaper.rs` | the two-kinds × two-levels contract: `ShaperKind`, `Level`, `StoredLattice`, `apply`, `ShaperDeclaration`, `ShaperSet` |
| `src/chain.rs` | the append-only ledger: `Namespace`, `EventKind`, `Event`, `Chain`, `Ticket`, and the wire form |
| `src/main.rs` | the thin entry point: a consumer of the library that prints the registry (now with per-system status) and the U1 and U2 measurements; exit code is the verdict |
| `src/systems.rs` | the U1–U8 registry (gains a `status` column) |

## 5. Complexity tracking

*(empty)* — no principle is violated. `sha2` is a dependency in service of P3 (determinism is a
contract) and P4 (a digest is the evidence); it is declared in `Cargo.toml` with its reason on the
line beside it, per the repository's dependency rule.
