# Phase 1 — Quickstart: validating U3 (once it is built)

**Feature**: `003-u3-shapers` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**Read this first.** U3 is **built and measured** (2026-10-11): the declared roster, the noise
primitives, the cadence runtime and **thirteen of the fifteen stage bodies**. Scenarios 1, 3, 4 and 5
below carry pasted output; scenario 2 is **partial** (the print has the rung, reads and writes but not
the source), and the two stages that refuse do so **by decision** — `caves` (the dissolution field it
reads is not one of the declared fields) and `earthworks` (the record, not a generator). An unmeasured
line stays marked `unchecked`.

## Prerequisites

```bash
cd clean-room                # its own Cargo workspace, beside the legacy tree
cargo --version              # the toolchain the workspace pins
```

No network is required: the crate depends only on `sha2`, already resolved locally (`--offline`).

## Scenario 1 — one digest for one (seed, tick), whatever the worker count (SC-001, SC-002)

```bash
cargo run --offline
# measured 2026-10-11 (the binary takes no arguments; the seed and tick are the reference declaration's):
#   U3 declared roster: 15 stages, declaration digest 0e2f704345b47814c2fdbad3183a8290a60210bcefe711751c40824a1cc60cb3
#   U3 digest at 1 worker(s): 460214b1b503d3e6334acabd73a6a654a110394df2de8235c750cf6a331b2fc6
#   U3 digest at 2 worker(s): 460214b1b503d3e6334acabd73a6a654a110394df2de8235c750cf6a331b2fc6
#   U3 digest at 4 worker(s): 460214b1b503d3e6334acabd73a6a654a110394df2de8235c750cf6a331b2fc6
#   U3 digest at 8 worker(s): 460214b1b503d3e6334acabd73a6a654a110394df2de8235c750cf6a331b2fc6
#   U3 tally at tick 0: 13/15 stages ran, 2 refused
#   U3 tick 1 digest: 7da363a83442494e85baec7864fa3eaddd273e6bc7d115ea943ff3d0d2d8cae5 (moves the world: true)
#   U3 law readings: stream order 1-4; 275 columns hold water (deepest 0.303 m);
#     deposition 0.1362 m over 96 columns; blend 0.000-0.995; capacity up to 165.1 g m^-2 yr^-1;
#     flock up to 0.546
#   U3 conservation: hillslope flux sums to -8.674e-18 over a region of magnitude 3.217e0 (cancels: true)
```

**Passes when**: the four worker counts print one digest, the tick+1 digest differs, and the laws the
roster declares are *visible in the world* rather than asserted about it. **Measured:** all hold. The
tally is **13 of 15** because `caves` and `earthworks` refuse by decision; the legacy's `14 of 16` was
the same rule against the legacy's own count, so against this fifteen-stage roster the bar is `13 of
15` — met, and not by counting a refusal as a ran stage.

## Scenario 2 — the roster names its sources (SC-004)

```bash
cargo run --offline | grep -A 16 "declared roster"
# measured 2026-10-11:
#   U3   tectonics   rung 3 (detail  ) reads [] writes ["uplift", "plate_id"]
#   U3   fields      rung 3 (detail  ) reads [] writes ["lithology"]
#   ... fifteen lines, one per stage, with the rung on the closed four-rung ladder
```

**Passes when**: every stage prints a source, and a stage admitted without one is refused by name.
**Partially measured:** the roster prints (fifteen stages, each with its rung, reads and writes), and a
stage with no source is refused by name (`a_stage_without_a_source_is_refused_by_name`); the **source
is not on the printed line** yet — it is inside the declaration digest — so this stays `unchecked`.

## Scenario 3 — region laws at a site, and conservation (SC-005)

```bash
cargo test --offline region
# measured 2026-10-11:
#   field::tests::a_regions_only_law_asked_for_a_lone_site_is_refused_by_name ... ok
#   water::tests::a_lone_site_cannot_evaluate_a_law_about_a_relation ... ok
#   water::tests::hillslope_transport_conserves_over_a_closed_region_and_refuses_a_cliff ... ok
```

**Passes when**: the refusal is named *and* the conservation recount balances. **Measured:** both hold,
and the recount is taken on the **generated world** as well as over a fixture — `cargo run` prints the
hillslope flux summing to −8.674e−18 over a region of magnitude 3.217 (Scenario 3b). The equal-and-
opposite exchange is the divergence itself: `q` leaves the higher column of every unordered pair and
arrives at the lower, so the closed region sums to zero without a boundary term to account for.

## Scenario 4 — the tick is idempotent and non-rewinding (SC-006)

```bash
cargo test --offline tick
# expected: drive(N) twice ⇒ one receipt digest; drive(N-1) ⇒ refused by name (rewind)
```

**Measured:** tick 3 driven twice is one receipt (`a_tick_is_idempotent_and_a_rewind_is_refused_by_name`),
and tick 2 after tick 3 is refused as `rewind` (`rewind: tick 2 is earlier than tick 3 that was already
driven`).

## Scenario 3b — the laws are visible in the world (the in-stage debt of the first pass)

```bash
cargo run --offline | grep -E "law readings|conservation"
# measured 2026-10-11:
#   stream order 1-4; 275 columns hold water (deepest 0.303 m); deposition 0.1362 m over 96 columns;
#   blend 0.000-0.995; capacity up to 165.1 g m^-2 yr^-1; flock up to 0.546
#   hillslope flux sums to -8.674e-18 over a region of magnitude 3.217e0 (cancels: true)
```

**Passes when**: each reading is a fact about the generated world, not a unit test's fixture — a network
that branches (Strahler 1952), water standing where Priority-Flood (Barnes et al. 2014) found a basin,
a delta that fills it (Gilbert 1885; Shields 1936), a blend that crosses two domains (Shepard 1968),
a province that carries production (Lieth 1975) and agents (Reynolds 1987), and a hillslope half whose
exchanges cancel exactly over the closed region (Roering et al. 1999; SC-005). **Measured:** all hold,
and `check_u3_laws` refuses the binary if any of them stops being true of the reference world.

## Scenario 5 — the crate's own gates (SC-007, and today's status)

```bash
cargo test --offline                      # measured 2026-10-11: 112 passed; 0 failed
cargo clippy --offline --all-targets -- -D warnings   # measured: clean
cargo run --offline                       # measured: exit 0, U1/U2/U3 measurements printed
```

**Measured:** these three pass with U3 in the tree. The boundary check holds too: `cargo tree --offline`
lists `sha2` and its transitive crates and nothing else, and nothing under `../crates/` is `use`d or
pathed (P7).

## What is still owed here

- **The *state* half of `delta`** — avulsion (Slingerland & Smith 2004) is a decision about the channel
  over time, and `MISSING-SYSTEMS` §4.1 rules that a per-tick accumulator outside the record would be a
  second simulation, so it waits on the record rather than on a missing formula.
- **`caves`' dissolution field** — the occupancy answer is a closed vocabulary whose field is not
  declared; the binary prints that refusal with its reason.
- **Scenario 2's source per stage**, and `--roster` as a flag rather than the unconditional print
  (T023/T024).
- **The reference world's dials are a declaration, not a law**: the patch is 48 columns wide, its relief
  is 5 m, its plates carry no signed bias and its ladder rung is 0, so a column-scale soil law holds
  across the whole patch. Change them and the laws refuse where they should — that is the design, and
  `check_u3_laws` is the bar that notices.
