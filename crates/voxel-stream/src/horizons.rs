//! The four horizons — **computed, not copied** (SPEC-5).
//!
//! A voxel world has four different notions of "how far out", and the classic
//! bug is to believe there is one. They are:
//!
//! | Horizon | The question it answers | What bounds it |
//! |---|---|---|
//! | [`Horizon::Render`] | what is on screen | the frame budget |
//! | [`Horizon::Simulation`] | what the tick updates | the tick budget |
//! | [`Horizon::Residency`] | what is kept in memory at all | memory |
//! | [`Horizon::Field`] | what a regional decision reads | the decision's own reach |
//!
//! They are **strictly nested** — render ⊆ simulation ⊆ residency ⊆ field — and
//! that nesting is the property [`Horizons::validate`] exists to check, because
//! a horizon set that is not nested is not four horizons: it is four unrelated
//! numbers, and the day they are consulted together is the day the world
//! behaves differently depending on which one a caller happened to ask.
//!
//! # These numbers are this repository's first declared values
//!
//! They are **not inherited** from the old tree, and they are not derived from
//! anything. They are choices, each with its reason, and each is expected to
//! move once there is a frame to measure. What is *not* a choice is the nesting
//! and the agreement with the field ladder, and both are enforced.
//!
//! # The ladder bound is a real constraint, not decoration
//!
//! [`HorizonRule::level`] is an index into [`voxel_shapers::fields`]' 20-level
//! ladder, which is aligned to the octree (`blocks_at_level(level) = 32 · 2^level`).
//! A horizon must not read a lattice cell **larger than the horizon itself** — it
//! would then be reporting a value centred somewhere it is not looking. That is
//! checked, and it is the check that keeps a coarse field level from quietly
//! describing a place outside the area the horizon covers.

use voxel_model::{ChunkPos, Refusal, RefusalKind, Refused, CHUNK_SIDE};
use voxel_shapers::fields::{blocks_at_level, STANDARD_LEVELS};

/// One of the four horizons.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Horizon {
    /// What is drawn.
    Render,
    /// What the tick updates.
    Simulation,
    /// What is resident in memory.
    Residency,
    /// What a regional decision reads.
    Field,
}

impl Horizon {
    /// Widest last, so a slice in this order is ordered by extent.
    pub const ALL: [Horizon; 4] = [
        Horizon::Render,
        Horizon::Simulation,
        Horizon::Residency,
        Horizon::Field,
    ];

    /// A stable, machine-readable name. Never prose.
    pub const fn name(self) -> &'static str {
        match self {
            Horizon::Render => "render",
            Horizon::Simulation => "simulation",
            Horizon::Residency => "residency",
            Horizon::Field => "field",
        }
    }

    /// The horizon immediately outside this one, if there is one.
    pub const fn outward(self) -> Option<Horizon> {
        match self {
            Horizon::Render => Some(Horizon::Simulation),
            Horizon::Simulation => Some(Horizon::Residency),
            Horizon::Residency => Some(Horizon::Field),
            Horizon::Field => None,
        }
    }
}

/// One horizon's declared extent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HorizonRule {
    pub horizon: Horizon,
    /// **Inclusive** radius in chunks around the viewer's chunk.
    pub radius_chunks: i32,
    /// The field-ladder level this horizon reads at. Finer = smaller index.
    pub level: u32,
}

impl HorizonRule {
    /// The horizon's edge length, in metres: a cube of `2r + 1` chunks.
    pub fn extent_m(&self) -> i64 {
        (2 * self.radius_chunks as i64 + 1) * CHUNK_SIDE as i64
    }

    /// The world extent of one lattice cell at this horizon's field level.
    pub fn lattice_cell_m(&self) -> u64 {
        blocks_at_level(self.level)
    }

    /// Whether a chunk is inside this horizon of `centre`.
    pub fn contains(&self, centre: ChunkPos, at: ChunkPos) -> bool {
        let d = |a: i32, b: i32| (a as i64 - b as i64).abs();
        // A cube, not a sphere: a chunk is resident or it is not, and a sphere
        // would make the boundary depend on the rounding of a square root.
        d(centre.x, at.x) <= self.radius_chunks as i64
            && d(centre.y, at.y) <= self.radius_chunks as i64
            && d(centre.z, at.z) <= self.radius_chunks as i64
    }
}

/// The four horizons, together.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Horizons {
    rules: [HorizonRule; 4],
}

impl Horizons {
    /// This repository's first declared values.
    ///
    /// | Horizon | radius | why |
    /// |---|---|---|
    /// | render | 8 chunks (272 m) | what a 70° field of view can plausibly hold at a distance where a subvoxel is still worth drawing |
    /// | simulation | 12 chunks (400 m) | wider than render because a thing just off screen must still be where it was when it comes back |
    /// | residency | 24 chunks (784 m) | twice simulation, so the boundary of simulation is never moving through chunks that are being generated for the first time |
    /// | field | 96 chunks (3 km) | the reach of a regional decision |
    ///
    /// The ladder levels rise outward for the same reason: the coarser a horizon,
    /// the less its interior detail can matter to what is drawn from the centre.
    pub const fn declared() -> Self {
        Horizons {
            rules: [
                HorizonRule { horizon: Horizon::Render, radius_chunks: 8, level: 0 },
                HorizonRule { horizon: Horizon::Simulation, radius_chunks: 12, level: 1 },
                HorizonRule { horizon: Horizon::Residency, radius_chunks: 24, level: 3 },
                HorizonRule { horizon: Horizon::Field, radius_chunks: 96, level: 5 },
            ],
        }
    }/// Replace one horizon's rule, **validating the whole set again**.
    ///
    /// The fields of [`HorizonRule`] are public so a rule can be read, but a set
    /// is only meaningful nested, and hand-building one out of four public structs
    /// is how a caller ends up with horizons that are individually plausible and
    /// jointly wrong. This is the only way to change one, and it cannot produce a
    /// set that [`Self::validate`] would refuse.
    pub fn with_rule(self, horizon: Horizon, radius_chunks: i32, level: u32) -> Refused<Self> {
        let mut next = self;
        for r in next.rules.iter_mut() {
            if r.horizon == horizon {
                r.radius_chunks = radius_chunks;
                r.level = level;
            }
        }
        next.validate()?;
        Ok(next)
    }

    pub fn rule(&self, horizon: Horizon) -> &HorizonRule {
        for r in &self.rules {
            if r.horizon == horizon {
                return r;
            }
        }
        // Unreachable: `rules` holds every `Horizon` exactly once, and
        // `validate` proves the set is exactly that.
        &self.rules[0]
    }

    pub fn rules(&self) -> &[HorizonRule; 4] {
        &self.rules
    }

    /// Whether a chunk lies inside a horizon of `centre`.
    pub fn contains(&self, horizon: Horizon, centre: ChunkPos, at: ChunkPos) -> bool {
        self.rule(horizon).contains(centre, at)
    }

    /// How many of the four horizons hold this chunk: **4 for the ones nearest
    /// the viewer, fewer the further out**.
    ///
    /// Because the horizons are strictly nested, the count *is* the outermost
    /// reach — a chunk in all four is in none of the others' exclusive bands, and
    /// a chunk in one is past every inner horizon. So this is not a fourth
    /// ordering that has to be kept in agreement with `contains`; it is derived
    /// from it, and cannot drift.
    ///
    /// It exists because distance alone is the wrong answer when the machine is
    /// behind. A chunk at the far side of the render cube and one just past it can
    /// be the same distance away, and only one of them is inside the horizon the
    /// viewer can actually see into. Counting horizons is how that difference gets
    /// a name without inventing a fifth concept.
    pub fn tier(&self, centre: ChunkPos, at: ChunkPos) -> u8 {
        Horizon::ALL.iter().filter(|h| self.contains(**h, centre, at)).count() as u8
    }

    /// Every horizon a chunk lies inside, widest first.
    pub fn horizons_containing(&self, centre: ChunkPos, at: ChunkPos) -> Vec<Horizon> {
        let mut out: Vec<Horizon> = Horizon::ALL
            .iter()
            .copied()
            .filter(|h| self.contains(*h, centre, at))
            .collect();
        out.reverse();
        out
    }

    /// Refuse a horizon set that is not four nested horizons.
    pub fn validate(&self) -> Refused<()> {
        // The set must be exactly one rule per horizon.
        for h in Horizon::ALL {
            let count = self.rules.iter().filter(|r| r.horizon == h).count();
            if count != 1 {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!("horizon {} appears {count} times, not once", h.name()),
                ));
            }
        }
        // Order must be the enumeration order, so `rules()` is ordered by extent.
        for (i, h) in Horizon::ALL.iter().enumerate() {
            if self.rules[i].horizon != *h {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!("rule {i} is {} but {} was expected", self.rules[i].horizon.name(), h.name()),
                ));
            }
        }
        for rule in &self.rules {
            if rule.radius_chunks < 0 {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!("horizon {} has a negative radius", rule.horizon.name()),
                ));
            }
            if rule.level >= STANDARD_LEVELS {
                return Err(Refusal::owned(
                    RefusalKind::IndexOutOfRange,
                    format!(
                        "horizon {} reads ladder level {} but the ladder is {STANDARD_LEVELS} levels",
                        rule.horizon.name(),
                        rule.level
                    ),
                ));
            }
            // A lattice cell wider than the horizon would centre a value
            // outside the area the horizon is looking at.
            let extent = rule.extent_m() as u64;
            if rule.lattice_cell_m() > extent {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "horizon {} is {extent} m across but reads a {} m lattice cell",
                        rule.horizon.name(),
                        rule.lattice_cell_m()
                    ),
                ));
            }
        }
        // Strict nesting outward.
        for pair in self.rules.windows(2) {
            let (inner, outer) = (pair[0], pair[1]);
            if inner.radius_chunks >= outer.radius_chunks {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "horizon {} ({} chunks) is not strictly inside {} ({} chunks)",
                        inner.horizon.name(),
                        inner.radius_chunks,
                        outer.horizon.name(),
                        outer.radius_chunks
                    ),
                ));
            }
            if inner.level > outer.level {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "horizon {} reads finer (level {}) than {} inside it (level {})",
                        outer.horizon.name(),
                        outer.level,
                        inner.horizon.name(),
                        inner.level
                    ),
                ));
            }
        }
        Ok(())
    }

    /// The ladder level to read a chunk at, by **ringed distance**.
    ///
    /// Each doubling of the render radius costs one ladder level. Declared, and
    /// clamped to the field horizon's level so a far chunk is never read at a
    /// resolution the horizon set does not claim to know.
    pub fn ring_level(&self, centre: ChunkPos, at: ChunkPos) -> u32 {
        let rule = self.rule(Horizon::Render);
        let reach = (rule.radius_chunks as i64).max(1);
        let d = [(centre.x, at.x), (centre.y, at.y), (centre.z, at.z)]
            .iter()
            .map(|(a, b)| (*a as i64 - *b as i64).abs())
            .max()
            .unwrap_or(0);
        let mut level = 0u32;
        let mut bound = reach;
        while d > bound {
            level += 1;
            // `saturating_mul` rather than `*`: a coordinate far enough away to
            // overflow would otherwise wrap into a *finer* level, which is the
            // wrong direction to be wrong in.
            bound = bound.saturating_mul(2);
            if level >= STANDARD_LEVELS {
                break;
            }
        }
        level.min(self.rule(Horizon::Field).level)
    }
}

impl Default for Horizons {
    fn default() -> Self {
        Self::declared()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::fields::STANDARD_LEVELS;

    #[test]
    fn the_declared_horizons_are_valid() {
        Horizons::declared().validate().unwrap();
    }

    #[test]
    fn a_horizon_set_that_is_not_nested_is_refused_and_says_so() {
        let mut h = Horizons::declared();
        h.rules[1].radius_chunks = 4; // simulation inside render
        let r = h.validate().unwrap_err();
        assert!(r.detail().contains("not strictly inside"), "got {r}");

        let mut h = Horizons::declared();
        h.rules[3].level = 99;
        assert_eq!(h.validate().unwrap_err().kind(), RefusalKind::IndexOutOfRange);

        let mut h = Horizons::declared();
        h.rules[0].horizon = Horizon::Field; // Field twice, Render missing
        assert!(h.validate().unwrap_err().detail().contains("times, not once"));

        let mut h = Horizons::declared();
        h.rules[0].radius_chunks = -1;
        assert!(h.validate().unwrap_err().detail().contains("negative radius"));
    }

    #[test]
    fn a_field_level_coarser_than_the_horizon_itself_is_refused() {
        // The constraint that keeps a coarse lattice from describing a place the
        // horizon is not looking at.
        let mut h = Horizons::declared();
        h.rules[0].level = 6; // 32 << 6 = 2048 m, inside a 272 m render horizon
        let r = h.validate().unwrap_err();
        assert!(r.detail().contains("lattice cell"), "got {r}");
    }

    #[test]
    fn an_outer_horizon_may_not_read_finer_than_an_inner_one() {
        let mut h = Horizons::declared();
        h.rules[3].level = 0;
        let r = h.validate().unwrap_err();
        assert!(r.detail().contains("finer"), "got {r}");
    }

    #[test]
    fn horizons_are_cubes_and_the_boundary_is_where_it_says() {
        let h = Horizons::declared();
        let c = ChunkPos::new(0, 0, 0);
        let r = h.rule(Horizon::Render).radius_chunks;
        assert!(h.contains(Horizon::Render, c, ChunkPos::new(r, 0, 0)));
        assert!(!h.contains(Horizon::Render, c, ChunkPos::new(r + 1, 0, 0)));
        // A corner at exactly the radius is inside, because a cube is a square
        // in every axis and not a rounded one.
        assert!(h.contains(Horizon::Render, c, ChunkPos::new(r, r, r)));
    }

    #[test]
    fn every_horizon_a_chunk_is_in_contains_every_horizon_inside_it() {
        // The nesting, exercised rather than asserted: `contains` is the thing
        // callers actually use, and a `validate` that agreed with itself while
        // `contains` disagreed would be the worst outcome — a checked invariant
        // that does not hold.
        let h = Horizons::declared();
        let c = ChunkPos::new(0, 0, 0);
        for x in [-200i32, -97, -25, -13, -9, -1, 0, 1, 9, 13, 25, 97, 200] {
            let at = ChunkPos::new(x, 0, 0);
            for (i, horizon) in Horizon::ALL.iter().enumerate() {
                if !h.contains(*horizon, c, at) {
                    continue;
                }
                // Everything **outside** it holds too. The other direction is
                // not nesting, it is the opposite of it: a chunk just past the
                // render horizon is inside the field horizon and nowhere else.
                for outer in &Horizon::ALL[i + 1..] {
                    assert!(
                        h.contains(*outer, c, at),
                        "{horizon:?} holds x = {x} but {outer:?} does not"
                    );
                }
            }
        }
    }

    #[test]
    fn horizons_containing_reports_the_widest_horizons_and_only_those() {
        let h = Horizons::declared();
        let c = ChunkPos::new(0, 0, 0);
        // Near the centre: all four, widest first — which is the reverse of
        // `Horizon::ALL`, and the difference is worth stating because the two
        // orders serve different callers: `ALL` is ordered by extent for
        // iteration, this is ordered coarsest-first for "what can I read here".
        let mut widest = Horizon::ALL.to_vec();
        widest.reverse();
        assert_eq!(h.horizons_containing(c, ChunkPos::new(0, 0, 0)), widest);
        // Just past render but inside simulation.
        assert_eq!(
            h.horizons_containing(c, ChunkPos::new(9, 0, 0)),
            vec![Horizon::Field, Horizon::Residency, Horizon::Simulation]
        );
        // Past residency, inside field only. This is the case the earlier
        // version of this test got wrong by assuming the answer was always a
        // *prefix* of the horizon list.
        assert_eq!(
            h.horizons_containing(c, ChunkPos::new(25, 0, 0)),
            vec![Horizon::Field]
        );
        assert!(h.horizons_containing(c, ChunkPos::new(200, 0, 0)).is_empty());
    }

    #[test]
    fn the_tier_counts_horizons_and_falls_only_outward() {
        // The count is the outermost reach, because the set is nested. If these
        // two ever disagree, one of them is wrong.
        let h = Horizons::declared();
        let c = ChunkPos::new(0, 0, 0);
        assert_eq!(h.tier(c, c), 4);
        // Inside render, outside nothing.
        assert_eq!(h.tier(c, ChunkPos::new(8, 0, 0)), 4);
        // Just outside render (8), inside simulation (12): one horizon fewer.
        assert_eq!(h.tier(c, ChunkPos::new(9, 0, 0)), 3);
        // Just outside simulation, inside residency (24).
        assert_eq!(h.tier(c, ChunkPos::new(13, 0, 0)), 2);
        // Just outside residency, inside field (96).
        assert_eq!(h.tier(c, ChunkPos::new(25, 0, 0)), 1);
        assert_eq!(h.tier(c, ChunkPos::new(97, 0, 0)), 0);

        // Monotone along one ray, and always equal to the count it claims.
        let mut last = u8::MAX;
        for d in [0i32, 4, 8, 9, 12, 13, 24, 25, 96, 97, 500] {
            let at = ChunkPos::new(d, 0, 0);
            let n = h.tier(c, at);
            assert!(n <= last, "tier rose at d = {d}");
            last = n;
            let counted = Horizon::ALL.iter().filter(|x| h.contains(**x, c, at)).count() as u8;
            assert_eq!(n, counted, "tier and contains disagree at d = {d}");
        }
    }

    #[test]
    fn the_ring_level_rises_outward_and_is_clamped() {
        let h = Horizons::declared();
        let c = ChunkPos::new(0, 0, 0);
        assert_eq!(h.ring_level(c, ChunkPos::new(0, 0, 0)), 0);
        assert_eq!(h.ring_level(c, ChunkPos::new(8, 0, 0)), 0);
        assert_eq!(h.ring_level(c, ChunkPos::new(9, 0, 0)), 1);
        assert_eq!(h.ring_level(c, ChunkPos::new(17, 0, 0)), 2);
        // Clamped to the field horizon's level, however far away.
        assert_eq!(h.ring_level(c, ChunkPos::new(1_000_000, 0, 0)), 5);
    }

    #[test]
    fn the_ladder_extents_the_horizons_read_are_the_rungs_they_say() {
        // Ties the horizon levels to the shaper's own function rather than to a
        // second copy of the arithmetic.
        let h = Horizons::declared();
        assert_eq!(h.rule(Horizon::Render).lattice_cell_m(), blocks_at_level(0));
        assert_eq!(h.rule(Horizon::Field).lattice_cell_m(), blocks_at_level(5));
        assert!(STANDARD_LEVELS > h.rule(Horizon::Field).level);
    }

    #[test]
    fn the_extent_of_a_horizon_is_an_odd_number_of_chunks() {
        // A horizon is centred on the viewer's chunk, so it is 2r+1 chunks wide,
        // which is always **odd**. An even width would put the viewer between two
        // chunks, and "the chunk I am in" would stop being well defined.
        let h = Horizons::declared();
        for rule in h.rules() {
            let chunks_wide = rule.extent_m() / CHUNK_SIDE as i64;
            assert_eq!(chunks_wide, 2 * rule.radius_chunks as i64 + 1);
            assert_eq!(chunks_wide % 2, 1, "{} is an even number of chunks wide", rule.horizon.name());
        }
    }
}
