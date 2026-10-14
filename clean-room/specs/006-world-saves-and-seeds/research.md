# Phase 0 — Research: U9 saves, loads and the world's own seed systems

**Feature**: `006-world-saves-and-seeds` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**What this phase had to resolve.** The spec names a written world, its seed systems in the plural, and
one refusal per way a save can be wrong. This file records the decisions that had to be *made* before a
design could be written, each with its rationale and the alternative rejected. All of them are
re-derived from documents this repository already holds — `docs/DESIGN-SPEC.md` §1.3 and §2 F1/SPEC-3,
`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16.1 U9 and §5 row 9, `LEDGER.md` C-96.1 and its H-7.20 line,
`docs/GAIA.md` §5c/§11, and the determinism test `docs/MISSING-SYSTEMS.md` §1 names — **no legacy
source was read** (P7), and no external source was needed.

## D1 — A save is **inputs**, never derived state

- **Decision**: a save carries the identity (`seed`, declared period, tick), the five declared
  sub-streams' own seeds, the configuration/ruleset identity, and the record. It does **not** carry the
  world truth digest, the resident set, or any geometry.
- **Rationale**: F1 rules that *replay inputs are recorded, not derived*, and the tick-digest test in
  `MISSING-SYSTEMS.md` §1 is the whole acceptance bar: *a ticked world and a rebuilt world produce one
  digest, whatever the worker count*. Derived state in a save is exactly what would let a loaded world
  disagree with the world it came from — C-46's class of defect, one level up.
- **Alternatives considered**: (a) snapshotting the resident set or the mesh — rejected: both are
  caches of a world that must be recomputed, not carried; (b) writing the truth digest as a checksum —
  rejected: the trailer already gives integrity, and a derived number in a save invites being trusted
  *over* the world it was derived from.

## D2 — The seed systems are five carried seeds, not one dial renamed

- **Decision**: `World::stream_seed(domain) -> u64` is added **at U2's owner** — the domain's own seed,
  a pure function of `(seed, declared period, tick, domain)`. A save carries **all five**, one per
  declared `Domain`, and a load checks each against the world's own.
- **Rationale**: the mission names the seed systems **in the plural** and §16.1 says why: a save that
  carried one seed and *called* the five streams derived would be a single dial with five names, which
  is precisely the state §16 was written to end. U2 already owns the closed `Domain` set, so the
  per-domain seed belongs there rather than in `save.rs` — a second answer here would be a second
  vocabulary (P5).
- **Alternatives considered**: (a) re-rooting `stage_seed(domain, pos)` on `stream_seed` so the per-
  position sample is *drawn from* the domain's seed — **rejected on blast radius**: `stage_seed` feeds
  the noise primitives, so re-rooting it would move U3's published digest  (`460214b1…2fc6`) and U4's
  (`44a682e1…4ffb` since §40, `2db62b78…b79c` since §39, `0e7358e4…4ed8` before it), which are the acceptance evidence of
  already-landed slices, for no acceptance
  value; (b) carrying one world seed and validating only it — rejected, that is the "one dial" failure.
- **What is claimed, exactly**: `stream_seed(domain)` is the domain's declared seed and
  `stage_seed(domain, pos)` is the per-position sample the shapers consume. They are two declared
  numbers over one identity; the save carries the first. No claim is made that one is computed from the
  other, because it is not.

## D3 — One exact, length-framed, little-endian layout, with a trailer

- **Decision**: fixed field order, little-endian integers, every variable-length part framed by its
  length, and a trailing SHA-256 (`Digest`) over every byte before it.
- **Rationale**: exactness is an FR — `save → load → save` must reproduce identical bytes — and a
  framed layout makes truncation a *refusal* rather than a misread. Little-endian integers and no
  floating point mean two platforms write one byte string.
- **Alternatives considered**: (a) extending the chain's tab-separated text wire over the whole save —
  rejected: the record already has one exact wire form, and embedding it as a framed blob keeps **one**
  wire form rather than inventing a second (P5); (b) JSON — rejected: a second parser and a data-format
  dependency for a format whose whole job is to be exact.

## D4 — Integrity and correspondence are different questions with different names

- **Decision**: the trailer digest is checked **first** and refuses `save_tampered`; the ruleset, the
  five streams and the record's declared head/length are checked **after** and refuse
  `identity_mismatch` or `record_diverged`.
- **Rationale**: "was this altered?" and "is this save of the world it claims to be?" are different
  faults, and one kind for both would tell a caller nothing about which it has. It also makes each
  check independently testable: a doctored stream seed with a **recomputed** trailer is not tampering —
  it is a save of a different world, and it must be named as such.
- **Alternatives considered**: one `digest_mismatch` refusal for every fault — rejected: it conflates a
  corrupt file with a wrong-world save, and P2's subject is *naming* what is wrong.

## D5 — A save names the configuration/ruleset identity, and a load under another is refused

- **Decision**: a save carries `Ruleset { ruleset, revision }`; `Save::load(expected)` compares the two
  and refuses `identity_mismatch` with both, naming each.
- **Rationale**: `GAIA.md` §5c/§11 states the reason the record keeps per-record ruleset identity at
  all — a save stays portable across mod changes **only because** what it was made under is written
  down, which makes history explainable. Loading under a different ruleset and calling the result the
  same world is the silent reinterpretation the one rule forbids.
- **Alternatives considered**: loading the save and *recording* the deviation — rejected: that is a
  second account of a ruleset change, and the ruleset's own versioning is not U9's to invent.

## D6 — The record is declared as well as carried

- **Decision**: the save carries the record's wire form **and** declares its head digest and length; a
  parsed chain whose head or length disagrees is refused `record_diverged`.
- **Rationale**: the trailer cannot catch a save that was *built* with a different, perfectly valid
  chain — a hand-built save recomputes its own trailer. Declaring what the record is supposed to be is
  what makes "this is the same record" checkable rather than assumed, and it is the one check that can
  catch a swapped record at all.
- **Alternatives considered**: trusting the parsed chain alone — rejected: a shorter valid chain would
  load silently as a shorter world.

## D7 — The door stays U6's

- **Decision**: `save` and `load` are the **library's** surface. The closed tool table stays at five
  verbs; adding `save`/`load` as verbs is a U6 change and is named owed.
- **Rationale**: P5 — one declared interface, one table. A second table here would be the second
  decision matrix the constitution refuses by name, and the mission's instruction for this slice is to
  keep the content gate (which the document form needs) out of it.
- **Alternatives considered**: adding the verbs now — rejected: the table is a closed, tested value
  (`bridge::TOOLS`) with a receipt contract behind it; smuggling two verbs in would change U6's
  published surface from inside U9.

## D8 — The evidence writes a real file

- **Decision**: `check_u9` writes the save to one fixed path under the system temp directory, reads it
  back, and prints the path and the byte length. Tests use their own distinct file names so a parallel
  run cannot collide.
- **Rationale**: the spec's first user story is *a world written down*; a buffer in one process is not
  written down, and the filesystem is the surface a caller actually uses. A test that renames its own
  file per case keeps the suite parallel-safe rather than serialised by a shared path.
- **Alternatives considered**: in-memory only — rejected as the weaker measurement; the file round trip
  is one line of evidence and it exercises the real surface.

## D9 — A stream is checked in **both** directions

- **Decision**: a save missing a domain the loader declares is refused `seed_undeclared` naming the
  domain; a save naming a domain the loader does not declare (or naming one twice) is refused the same
  name. Every problem is collected, not the first.
- **Rationale**: a roster that later shrinks or grows must not silently drop or absorb a stream's
  identity, and the failure mode of a one-directional check is exactly that silence.
- **Alternatives considered**: checking only for missing streams — rejected: a save declaring an
  unknown stream would then load, and the loaded world's identity would be a guess.
