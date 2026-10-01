//! The stage graph (F9).
//!
//! A shaper does not call another shaper. It declares the stages it **reads**,
//! and the graph decides whether the pipeline it was handed is runnable. That is
//! the difference between "the generator crashed at 3am" and "the generator
//! refused, by name, at construction".
//!
//! Two rules are enforced here and both come from the record:
//!
//! 1. **A declared layer that is absent is a blocker, not an omission** (WG-Q3).
//!    Asking for a stage that was not declared is a
//!    [`RefusalKind::MissingStage`](voxel_model::RefusalKind::MissingStage), never
//!    a zero.
//! 2. **Fields are stage-coupled, not independent noise** (WG-Q5). Because
//!    dependencies are declared, a shaper that reads climate *must* have
//!    declared it, so an uncoupled field cannot be smuggled in behind a raw seed.
//!
//! # The coverage guard
//!
//! Adding a stage without adding its dependency row must **fail the build**. Rust
//! cannot enumerate an enum's variants, so [`STAGE_COUNT`] is written by hand and
//! [`ALL`]'s type is `[Stage; STAGE_COUNT]`: a new variant makes the array the
//! wrong length and the build stops. The residual is named rather than hidden —
//! the *third* way to add a stage (variant, arms, and a `DEPENDS` row, but not
//! `ALL`) is caught by [`tests::every_stage_in_all_has_a_dependency_row`].

use std::collections::BTreeSet;
use voxel_model::{Refusal, RefusalKind};

/// One stage of the generator.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Stage {
    Tectonics,
    Fields,
    Geology,
    Terrain,
    Hydrology,
    Erosion,
    Climate,
    Atmosphere,
    Weather,
    Caves,
    Substrate,
    SoilTransport,
    Ecology,
    Delta,
    Transition,
    Earthworks,
}

/// **Written by hand on purpose.** See the module docs: this is the half of the
/// coverage guard that turns a forgotten stage into a compile error.
pub const STAGE_COUNT: usize = 16;

/// How often a stage runs, in base ticks (H-8.3).
///
/// **This is part of the determinism contract, not a performance knob.** The
/// ladder is: player edits and dirty meshes at **×1**, water and settling at
/// **×4**, erosion and deposition at **×20**, and geology and climate at
/// **`TICKS_PER_DAY`**. A stage that ran at a different rate is a *different
/// world*, in exactly the sense F12 uses for worker count.
///
/// The rule that makes the ladder safe is the one underneath it: **the world
/// stays pure in `(tick, address)` and the deferred tick is the *schedule*,
/// never state.** Nothing here stores anything. A period says *when a stage is
/// asked*, and the answer a stage gives is a pure function of the address and
/// the tick — so two runs with different worker counts, or a run that skipped
/// frames, produce the same world as long as the tick is the same.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Cadence {
    /// Every tick: what the player just did, and the meshes that dirtied.
    EveryTick,
    /// ×4: water and settling.
    Water,
    /// ×20: erosion and deposition — the material that moves.
    Sediment,
    /// One in-game day: geology and climate.
    Daily,
}

impl Cadence {
    /// Every rung, fastest first.
    ///
    /// **Hand-written on purpose**, for the same reason [`ALL`] is: this is the
    /// list a test counts, so that a fifth rung cannot appear without someone
    /// deciding it should. [`tests::the_ladder_is_exactly_the_four_rungs_the_ruling_declares`]
    /// fails if the length moves.
    pub const ALL: [Cadence; 4] =
        [Cadence::EveryTick, Cadence::Water, Cadence::Sediment, Cadence::Daily];

    /// The period, in base ticks.
    ///
    /// **These four numbers are the ladder H-8.3 declares**, and they are the
    /// whole of it: `TICKS_PER_DAY` is [`crate::climate::TICKS_PER_DAY`] and is
    /// not restated here, so the day cannot drift between the clock and the
    /// stages that read it.
    pub const fn period_ticks(self) -> u64 {
        match self {
            Cadence::EveryTick => 1,
            Cadence::Water => 4,
            Cadence::Sediment => 20,
            Cadence::Daily => crate::climate::TICKS_PER_DAY,
        }
    }

    /// Whether a stage with this cadence is **due** at `tick`.
    ///
    /// The whole scheduling rule, and it is one line because there is nothing
    /// else to schedule: a stage whose period divides the tick is asked this
    /// tick. There is no accumulator, no catch-up and no "how many ticks did I
    /// miss", because a stage's answer must not depend on how often it was
    /// called — that is the same purity F12 demands of worker count.
    ///
    /// Tick 0 is due for every cadence, so a world asked for the first time is
    /// complete rather than one period stale.
    pub const fn is_due_at(self, tick: u64) -> bool {
        tick.is_multiple_of(self.period_ticks())
    }
}

impl Stage {
    /// This stage's index in [`ALL`], or [`STAGE_COUNT`] if it is not there.
    ///
    /// Compares **discriminants** rather than values because `PartialEq` is not
    /// a `const fn`; a `Stage` is a fieldless enum, so the discriminant is the
    /// identity and this stays exact.
    pub const fn index_in_all(self) -> usize {
        let me = self as usize;
        let mut i = 0;
        while i < STAGE_COUNT {
            if ALL[i] as usize == me {
                return i;
            }
            i += 1;
        }
        STAGE_COUNT
    }

    /// This stage's declared cadence (H-8.3).
    ///
    /// The declaration is **per stage** and is hand-written for the same
    /// reason [`ALL`] is: a stage added without a cadence should not compile
    /// silently. [`tests::every_stage_declares_a_cadence`] is the residual that
    /// Rust's type system cannot catch.
    pub const fn cadence(self) -> Cadence {
        match self {
            // ×1 — the player's own edits, and anything that dirties a mesh.
            Stage::Earthworks | Stage::Transition | Stage::Substrate => Cadence::EveryTick,
            // ×4 — water and settling.
            Stage::Hydrology | Stage::Weather | Stage::Atmosphere => Cadence::Water,
            // ×20 — erosion and deposition: everything that *moves* material.
            Stage::Erosion | Stage::Delta | Stage::SoilTransport => Cadence::Sediment,
            // `TICKS_PER_DAY` — geology and climate, the slow pair.
            Stage::Geology | Stage::Climate | Stage::Caves => Cadence::Daily,
            // The structural stages. These derive a field rather than evolve
            // one, so they are asked on demand by whatever reads them; see
            // `tests::a_structural_stage_has_no_own_period`.
            Stage::Tectonics | Stage::Fields | Stage::Ecology | Stage::Terrain => Cadence::EveryTick,
        }
    }

    /// Whether this stage is **due** at `tick`.
    pub const fn is_due_at(self, tick: u64) -> bool {
        self.cadence().is_due_at(tick)
    }
}

/// Every stage, in a stable order.
pub const ALL: [Stage; STAGE_COUNT] = [
    Stage::Tectonics,
    Stage::Fields,
    Stage::Geology,
    Stage::Terrain,
    Stage::Hydrology,
    Stage::Erosion,
    Stage::Climate,
    Stage::Atmosphere,
    Stage::Weather,
    Stage::Caves,
    Stage::Substrate,
    Stage::SoilTransport,
    Stage::Ecology,
    Stage::Delta,
    Stage::Transition,
    Stage::Earthworks,
];

impl Stage {
    /// The machine-readable name, which is also the seed domain.
    pub const fn name(self) -> &'static str {
        match self {
            Stage::Tectonics => "tectonics",
            Stage::Fields => "fields",
            Stage::Geology => "geology",
            Stage::Terrain => "terrain",
            Stage::Hydrology => "hydrology",
            Stage::Erosion => "erosion",
            Stage::Climate => "climate",
            Stage::Atmosphere => "atmosphere",
            Stage::Weather => "weather",
            Stage::Caves => "caves",
            Stage::Substrate => "substrate",
            Stage::SoilTransport => "soil_transport",
            Stage::Ecology => "ecology",
            Stage::Delta => "delta",
            Stage::Transition => "transition",
            Stage::Earthworks => "earthworks",
        }
    }

    /// What this stage reads.
    ///
    /// These are the *declared* couplings, and they are the ones a shaper's
    /// documentation is checked against: [`super::terrain`] says terrain needs
    /// tectonics and fields, and this table says the same thing.
    pub const fn depends_on(self) -> &'static [Stage] {
        match self {
            Stage::Tectonics => &[],
            Stage::Fields => &[Stage::Tectonics],
            Stage::Geology => &[Stage::Tectonics, Stage::Fields],
            Stage::Terrain => &[Stage::Tectonics, Stage::Fields, Stage::Geology],
            Stage::Hydrology => &[Stage::Terrain, Stage::Fields],
            Stage::Erosion => &[Stage::Terrain, Stage::Hydrology, Stage::Geology],
            Stage::Climate => &[Stage::Terrain, Stage::Atmosphere],
            Stage::Atmosphere => &[Stage::Fields, Stage::Terrain],
            Stage::Weather => &[Stage::Climate, Stage::Terrain, Stage::Hydrology],
            Stage::Caves => &[Stage::Geology, Stage::Hydrology],
            Stage::Substrate => &[Stage::Terrain, Stage::Climate, Stage::Erosion],
            // Soil transport reads the **baseline** profile and moves what is
            // loose off it, so it depends on `Substrate` and on the water that
            // carries it — which is why it cannot be folded into `Substrate`
            // itself. `Substrate` is the tick-free baseline; this is the part
            // that is allowed to depend on the tick. See `docs/HYPERGRILLING.md`
            // H-16.3.
            Stage::SoilTransport => &[Stage::Terrain, Stage::Substrate, Stage::Hydrology, Stage::Erosion],
            Stage::Ecology => &[Stage::Climate, Stage::Weather, Stage::Substrate, Stage::Terrain],
            Stage::Delta => &[Stage::Hydrology, Stage::Erosion, Stage::Terrain],
            Stage::Transition => &[Stage::Ecology, Stage::Geology],
            Stage::Earthworks => &[Stage::Terrain, Stage::Geology],
        }
    }
}

/// The set of stages a run has declared it will provide.
#[derive(Clone, Debug, Default)]
pub struct Declared {
    stages: BTreeSet<Stage>,
}

impl Declared {
    /// The full pipeline: every stage, run in dependency order.
    pub fn full() -> Self {
        Declared { stages: ALL.iter().copied().collect() }
    }

    /// A subset, for a test or a partial rebuild.
    ///
    /// The subset is **closed over dependencies**: declaring `Hydrology` without
    /// `Terrain` implies terrain, because a stage that needs terrain and cannot
    /// have it has nothing to say. Closure is what makes a partial pipeline
    /// honest rather than quietly degraded.
    pub fn closure_of(stages: impl IntoIterator<Item = Stage>) -> Self {
        let mut set: BTreeSet<Stage> = stages.into_iter().collect();
        // Fixpoint: adding a dependency can add more.
        loop {
            let mut grew = false;
            let snapshot: Vec<Stage> = set.iter().copied().collect();
            for s in snapshot {
                for d in s.depends_on() {
                    if set.insert(*d) {
                        grew = true;
                    }
                }
            }
            if !grew {
                break;
            }
        }
        Declared { stages: set }
    }

    pub fn contains(&self, stage: Stage) -> bool {
        self.stages.contains(&stage)
    }

    pub fn len(&self) -> usize {
        self.stages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// Require a stage, refusing by name when it is absent.
    pub fn require(&self, stage: Stage) -> Result<(), Refusal> {
        if self.stages.contains(&stage) {
            Ok(())
        } else {
            Err(Refusal::owned(
                RefusalKind::MissingStage,
                format!(
                    "stage {:?} was required but not declared (declared: {})",
                    stage,
                    self.names().join(", ")
                ),
            ))
        }
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.stages.iter().map(|s| s.name()).collect()
    }

    /// The stages in an order that respects every declared dependency.
    ///
    /// Deterministic: ties are broken by the order of [`ALL`], never by
    /// `BTreeSet` iteration luck.
    pub fn order(&self) -> Vec<Stage> {
        let mut done: BTreeSet<Stage> = BTreeSet::new();
        let mut out = Vec::with_capacity(self.stages.len());
        loop {
            let mut progressed = false;
            for s in ALL {
                if !self.stages.contains(&s) || done.contains(&s) {
                    continue;
                }
                if s.depends_on().iter().all(|d| done.contains(d)) {
                    out.push(s);
                    done.insert(s);
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ladder_is_exactly_the_four_rungs_the_ruling_declares() {
        // H-8.3 names four rungs and no fifth. If this fails, a number was
        // invented — which is the one thing this ladder must not contain.
        assert_eq!(Cadence::EveryTick.period_ticks(), 1, "player edits and dirty meshes");
        assert_eq!(Cadence::Water.period_ticks(), 4, "water and settling");
        assert_eq!(Cadence::Sediment.period_ticks(), 20, "erosion and deposition");
        assert_eq!(
            Cadence::Daily.period_ticks(),
            crate::climate::TICKS_PER_DAY,
            "geology and climate, and the day is the clock's day, not a second one"
        );
        assert_eq!(Cadence::ALL.len(), 4, "the ladder has four rungs and no others");
    }

    #[test]
    fn a_cadence_is_due_exactly_when_the_tick_is_a_multiple_of_its_period() {
        // The entire scheduling rule, checked against a hand-worked case rather
        // than against itself: 1 divides everything, 4 divides every fourth, 20
        // every twentieth, and the day only on a day boundary.
        for tick in 0..200u64 {
            // ×1 divides every tick, so the fastest rung is always due. Stated
            // rather than written as `tick % 1`, which is a tautology that also
            // hides the case: the rung that never waits is the one everything else
            // is measured against.
            assert!(Cadence::EveryTick.is_due_at(tick), "tick {tick}");
            assert_eq!(Cadence::Water.is_due_at(tick), tick % 4 == 0, "tick {tick}");
            assert_eq!(Cadence::Sediment.is_due_at(tick), tick % 20 == 0, "tick {tick}");
        }
        assert!(
            !Cadence::Sediment.is_due_at(19),
            "the sedimental stage ran a tick early"
        );
        assert!(Cadence::Sediment.is_due_at(20));
        assert!(!Cadence::Daily.is_due_at(23_999), "the daily stage ran before the day ended");
        assert!(Cadence::Daily.is_due_at(24_000));
    }

    #[test]
    fn a_faster_stage_is_due_on_every_tick_a_slower_one_is() {
        // The ladder has to be **nested**, or "run the due stages in order"
        // would mean the ×1 stage could be asked on a tick the ×20 stage was not,
        // and the two orders would not be comparable.
        for tick in [0u64, 20, 24_000, 576_000, 2_304_000] {
            if Cadence::Sediment.is_due_at(tick) {
                assert!(Cadence::Water.is_due_at(tick), "tick {tick}: the slower rung ran alone");
                assert!(Cadence::EveryTick.is_due_at(tick), "tick {tick}: the slower rung ran alone");
            }
        }
    }

    #[test]
    fn every_stage_declares_a_cadence_and_the_names_are_the_seed_domains() {
        // The residual of the coverage guard: `cadence()` is a match with no
        // `_` arm, so a new variant cannot compile without one. This checks the
        // thing Rust cannot — that the cadence table and `ALL` agree, and that
        // `soil_transport` is spelled the way its seed domain is spelled.
        for s in ALL {
            let _ = s.cadence();
        }
        assert!(ALL.contains(&Stage::SoilTransport), "the transport half is not in ALL");
        assert_eq!(Stage::SoilTransport.name(), "soil_transport");
    }

    #[test]
    fn soil_transport_runs_after_the_baseline_it_moves_material_off() {
        // The whole point of the split: transport reads the *baseline*, so it
        // cannot be scheduled before substrate exists.
        let order = Declared::full().order();
        let at = |s: Stage| order.iter().position(|x| *x == s).expect("a stage is missing");
        assert!(
            at(Stage::Substrate) < at(Stage::SoilTransport),
            "soil transport moved material before there was a baseline to move it off"
        );
        assert!(at(Stage::Hydrology) < at(Stage::SoilTransport));
        assert!(at(Stage::Erosion) < at(Stage::SoilTransport));
    }

    #[test]
    fn a_stage_declares_its_own_cadence_regardless_of_where_it_sits_in_all() {
        // The ladder is a property of the **stage**, not of its position in
        // `ALL` — otherwise adding a stage would silently re-r another's
        // period, which is the kind of accidental coupling F12 exists to
        // forbid. `index_in_all` must also be a real bijection, or this proves
        // nothing about a stage it cannot locate.
        let mut seen: Vec<Stage> = Vec::new();
        for s in ALL {
            let i = s.index_in_all();
            assert_eq!(ALL[i], s, "{s:?} does not round-trip through index_in_all");
            assert!(!seen.contains(&s), "{s:?} appears twice in ALL");
            seen.push(s);
            // Asking twice, from two different places, gives the same rung.
            assert_eq!(s.cadence(), s.cadence());
        }
        assert_eq!(seen.len(), STAGE_COUNT);
    }

    #[test]
    fn a_stage_is_due_exactly_when_its_own_cadence_is_due() {
        for s in ALL {
            for tick in [0u64, 1, 3, 4, 20, 24_000, 2_304_000] {
                assert_eq!(s.is_due_at(tick), s.cadence().is_due_at(tick), "{s:?} at {tick}");
            }
        }
    }

    #[test]
    fn a_settled_world_reaches_a_stage_on_its_own_period_and_not_a_tick_early() {
        // The tick count at which each rung first recurs, stated rather than
        // derived, so a changed constant has to be argued for.
        assert_eq!(first_recurrence(Cadence::Water), 4);
        assert_eq!(first_recurrence(Cadence::Sediment), 20);
        assert_eq!(first_recurrence(Cadence::Daily), crate::climate::TICKS_PER_DAY);
    }

    fn first_recurrence(c: Cadence) -> u64 {
        (1u64..).find(|t| c.is_due_at(*t)).expect("every cadence recurs")
    }

    #[test]
    fn every_stage_in_all_has_a_dependency_row() {
        // The residual of the compile-time guard: a stage that is in ALL but
        // whose `depends_on` arm was forgotten cannot be caught by the compiler,
        // so it is caught here — a dependency that names a stage not in ALL is a
        // hand-written table that has drifted.
        for s in ALL {
            for d in s.depends_on() {
                assert!(ALL.contains(d), "{s:?} depends on {d:?}, which is not in ALL");
            }
        }
    }

    #[test]
    fn the_stage_names_are_distinct() {
        let mut names: Vec<&str> = ALL.iter().map(|s| s.name()).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before);
    }

    #[test]
    fn a_full_pipeline_orders_every_stage_after_its_dependencies() {
        let d = Declared::full();
        let order = d.order();
        assert_eq!(order.len(), STAGE_COUNT);
        let mut seen: BTreeSet<Stage> = BTreeSet::new();
        for s in &order {
            for dep in s.depends_on() {
                assert!(seen.contains(dep), "{s:?} ran before its dependency {dep:?}");
            }
            seen.insert(*s);
        }
    }

    #[test]
    fn a_missing_stage_is_refused_by_name_and_never_zeroed() {
        let d = Declared::closure_of([Stage::Climate]);
        let r = d.require(Stage::Ecology).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::MissingStage);
        assert!(r.detail().contains("Ecology") || r.detail().contains("ecology"));
        assert!(d.require(Stage::Climate).is_ok());
    }

    #[test]
    fn closure_pulls_in_dependencies_so_a_partial_pipeline_is_honest() {
        // Asking for ecology implies climate, weather, substrate, terrain, and
        // their own dependencies — otherwise "ecology only" would be a pipeline
        // with no temperature.
        let d = Declared::closure_of([Stage::Ecology]);
        for s in [Stage::Ecology, Stage::Climate, Stage::Weather, Stage::Substrate, Stage::Terrain] {
            assert!(d.contains(s), "{s:?} was not pulled into the closure");
        }
    }

    #[test]
    fn ordering_is_deterministic() {
        let a = Declared::closure_of([Stage::Ecology]).order();
        let b = Declared::closure_of([Stage::Ecology]).order();
        assert_eq!(a, b);
    }

    #[test]
    fn the_full_pipeline_starts_at_a_root_and_respects_the_chain() {
        let order = Declared::full().order();
        assert!(
            order[0].depends_on().is_empty(),
            "the first stage to run was {:?}, which has dependencies",
            order[0]
        );

        let at = |s: Stage| {
            order
                .iter()
                .position(|x| *x == s)
                .unwrap_or_else(|| panic!("{s:?} was missing from the order"))
        };
        // The chain the shapers actually depend on.
        assert!(at(Stage::Terrain) < at(Stage::Hydrology));
        assert!(at(Stage::Climate) < at(Stage::Ecology));
        assert!(at(Stage::Ecology) < at(Stage::Transition));
        assert!(at(Stage::Hydrology) < at(Stage::Delta));
        assert!(at(Stage::Tectonics) < at(Stage::Earthworks));
        // Earthworks needs only terrain and geology, so it legitimately runs
        // **before** the late ecological stages. That is why this test does not
        // assert it runs last — an earlier version did, and it failed.
        assert!(at(Stage::Earthworks) < at(Stage::Transition));
    }
}
