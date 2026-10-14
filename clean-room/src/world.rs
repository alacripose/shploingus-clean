//! **World identity and residency — one owner each, kept apart.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §1.3: *"the world is a pure function of `(seed, tick, declared
//! period)` … rebuild reproduces truth with digests; worker count is a pure performance knob that
//! never changes output"*, and §2 F1 (replay inputs are recorded, not derived). The `LEDGER` rule
//! this module is built around is that a claim about the world is checkable **by digest**, so the
//! determinism is a number rather than a sentence.
//!
//! # One owner per concern, and why the two are different objects
//!
//! The **world truth** is `f(seed, declared period, tick)` — nothing else. The **residency** is which
//! chunks a renderer happens to hold right now; it is a cache, and a cache must never be able to
//! rewrite the world. Keeping them in two distinct quantities makes that checkable: the truth digest
//! is blind to the anchor and to the resident set, and the residency digest is blind to the tick, so
//! neither can drift into the other's job. The anchor is a *view* choice too, and is likewise
//! outside truth.
//!
//! Deterministic sub-streams are drawn from a [`Domain`] — a **closed** set, so a shaper cannot
//! invent a stream nobody can name — and mixed by a declared integer mixer, so the same
//! `(seed, period, tick, domain, position)` is the same number on every machine and every worker.

use std::collections::BTreeSet;

use crate::coordinates::{ChunkKey, VoxelPos};
use crate::digest::Digest;
use crate::refusal::RefusalKind;

/// The declared period the world is a function of. Refuses zero: no world is a function of nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclaredPeriod {
    seconds: u64,
}

impl DeclaredPeriod {
    pub fn seconds(seconds: u64) -> Result<Self, RefusalKind> {
        if seconds == 0 {
            return Err(RefusalKind::ZeroPeriod);
        }
        Ok(DeclaredPeriod { seconds })
    }

    pub const fn get(self) -> u64 {
        self.seconds
    }
}

/// Immutable: a world's identity does not change. Tick and view change; identity does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldIdentity {
    pub seed: u64,
    pub period: DeclaredPeriod,
}

impl WorldIdentity {
    pub const fn new(seed: u64, period: DeclaredPeriod) -> Self {
        WorldIdentity { seed, period }
    }
}

/// The world's clock. Ordinal; its value is meaningless except that it is ordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(pub u64);

/// The **closed** set of deterministic sub-streams. A stream that is not declared here cannot be
/// sampled, which is what stops two systems quietly sharing one stream and desynchronising.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Domain {
    Terrain,
    Geology,
    Climate,
    Hydrology,
    Detail,
}

impl Domain {
    pub const ALL: [Domain; 5] = [
        Domain::Terrain,
        Domain::Geology,
        Domain::Climate,
        Domain::Hydrology,
        Domain::Detail,
    ];

    pub const COUNT: usize = 5;

    pub const fn name(self) -> &'static str {
        match self {
            Domain::Terrain => "terrain",
            Domain::Geology => "geology",
            Domain::Climate => "climate",
            Domain::Hydrology => "hydrology",
            Domain::Detail => "detail",
        }
    }

    const fn tag(self) -> u64 {
        match self {
            Domain::Terrain => 1,
            Domain::Geology => 2,
            Domain::Climate => 3,
            Domain::Hydrology => 4,
            Domain::Detail => 5,
        }
    }
}

/// A declared integer mixer (the SplitMix64 finalizer). Deterministic, `u64`-only, and identical on
/// every platform — which is exactly what a pure world function needs and what a per-thread RNG is
/// not.
///
/// `pub(crate)`: the noise primitives (`src/noise.rs`, U3) hash lattice corners with **this** mixer
/// rather than a second copy of it. One mixer, one name — a second one would be a second decision
/// about what "the same seed" means (P5).
pub(crate) fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn chunk_bytes(key: ChunkKey) -> Vec<u8> {
    let mut out = Vec::with_capacity(12);
    out.extend_from_slice(&key.x.to_le_bytes());
    out.extend_from_slice(&key.y.to_le_bytes());
    out.extend_from_slice(&key.z.to_le_bytes());
    out
}

/// **The world.** One identity, one tick, one anchor and one residency cache — and truth defined so
/// that only the first two can touch it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    identity: WorldIdentity,
    tick: Tick,
    anchor: ChunkKey,
    resident: BTreeSet<ChunkKey>,
}

impl World {
    /// A fresh world at tick 0, anchored at the origin, holding nothing.
    pub fn new(identity: WorldIdentity) -> Self {
        World {
            identity,
            tick: Tick(0),
            anchor: ChunkKey::new(0, 0, 0),
            resident: BTreeSet::new(),
        }
    }

    pub const fn identity(&self) -> WorldIdentity {
        self.identity
    }

    pub const fn tick(&self) -> Tick {
        self.tick
    }

    /// The same world at another tick — truth follows, the cache and the anchor are carried. Pure:
    /// the receiver is unchanged.
    pub fn with_tick(&self, tick: Tick) -> World {
        World { tick, ..self.clone() }
    }

    /// Advance the world's clock by one.
    pub fn advance(&mut self) {
        self.tick.0 = self.tick.0.wrapping_add(1);
    }

    pub const fn anchor(&self) -> ChunkKey {
        self.anchor
    }

    /// Move the view anchor. **Not** a truth change.
    pub fn set_anchor(&mut self, anchor: ChunkKey) {
        self.anchor = anchor;
    }

    /// **The deterministic sample at a position in a declared domain.** Pure — a function of the
    /// identity, the tick, the domain and the position, and of nothing else.
    pub fn stage_seed(&self, domain: Domain, pos: VoxelPos) -> u64 {
        let mut h = mix(self.identity.seed ^ 0x2545_F491_4F6C_DD1D);
        h = mix(h ^ self.identity.period.get());
        h = mix(h ^ self.tick.0);
        h = mix(h ^ domain.tag());
        h = mix(h ^ (pos.x as i64 as u64));
        h = mix(h ^ (pos.y as i64 as u64));
        h = mix(h ^ (pos.z as i64 as u64));
        h
    }

    /// **The declared seed of one sub-stream.** A world's seed systems are **five** — one per declared
    /// [`Domain`] — not one dial with five names: this is a domain's own seed, independent of
    /// position, and it is the number a save carries so that two worlds' streams can be compared
    /// without sampling either (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` §16.1 U9;
    /// `specs/006-world-saves-and-seeds/contracts/u9-seams.md` C3).
    ///
    /// It is **not** the per-position sample ([`World::stage_seed`]) and it is deliberately not
    /// re-defined as one: re-rooting `stage_seed` on this value would move the digests U3 and U4 have
    /// already published (`460214b1…2fc6`; U4's own digest is the **face merge's**,
    /// `44a682e1…4ffb`, since plan §40 made the reading read the chunk's own ring — §39's `2db62b78…b79c`
    /// was that merge before the extent contract, and the merge's unit is still the cells' voxel face), for no
    /// acceptance value — two declared numbers over one identity (D2 in that spec's `research.md`).
    pub fn stream_seed(&self, domain: Domain) -> u64 {
        let mut h = mix(self.identity.seed ^ 0x5354_5245_414D_5345);
        h = mix(h ^ self.identity.period.get());
        h = mix(h ^ self.tick.0);
        mix(h ^ domain.tag())
    }

    /// **World truth**, digested. A pure function of `(seed, declared period, tick)` — the anchor
    /// and the resident set are deliberately absent from the canonical bytes, which is what makes
    /// "a cache cannot rewrite the world" a test rather than a promise.
    pub fn truth_digest(&self) -> Digest {
        let seed = self.identity.seed.to_le_bytes();
        let period = self.identity.period.get().to_le_bytes();
        let tick = self.tick.0.to_le_bytes();
        Digest::of_parts(&[b"shploingus.world-truth/1", &seed, &period, &tick])
    }

    /// **The residency cache**, digested in sorted key order. Building the same set in a different
    /// order is the same cache, so this is one digest — the property that says the cache is a set
    /// and not a history.
    pub fn residency_digest(&self) -> Digest {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"shploingus.world-residency/1");
        for key in &self.resident {
            bytes.extend_from_slice(&chunk_bytes(*key));
        }
        Digest::of(&bytes)
    }

    /// A **canonical** plan digest: order-free because it sorts the keys, and duplicate-exact
    /// because it keeps every one. A plan that names a chunk twice is not the plan that names it
    /// once, and no plan folds to the empty digest — the old XOR fold cancelled duplicates and made
    /// a two-worker plan equal an empty one (`docs/CLEAN-ROOM-PLAN-2026-10-11.md` audit, 2026-10-11).
    pub fn plan_digest<I: IntoIterator<Item = ChunkKey>>(&self, keys: I) -> Digest {
        let mut keys: Vec<ChunkKey> = keys.into_iter().collect();
        keys.sort_unstable();
        let mut bytes = Vec::with_capacity(28 + keys.len() * 12);
        bytes.extend_from_slice(b"shploingus.plan/1");
        for key in keys {
            bytes.extend_from_slice(&chunk_bytes(key));
        }
        Digest::of(&bytes)
    }

    /// Add a chunk to the cache. Returns whether it was newly added.
    pub fn reside(&mut self, key: ChunkKey) -> bool {
        self.resident.insert(key)
    }

    /// Remove a chunk from the cache. Returns whether it was present.
    pub fn evict(&mut self, key: ChunkKey) -> bool {
        self.resident.remove(&key)
    }

    pub fn is_resident(&self, key: ChunkKey) -> bool {
        self.resident.contains(&key)
    }

    pub fn resident_count(&self) -> usize {
        self.resident.len()
    }

    /// The resident keys in ascending order.
    pub fn resident_keys(&self) -> Vec<ChunkKey> {
        self.resident.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> WorldIdentity {
        WorldIdentity::new(2_468, DeclaredPeriod::seconds(1).unwrap())
    }

    #[test]
    fn a_declared_period_of_zero_is_refused() {
        assert_eq!(
            DeclaredPeriod::seconds(0).unwrap_err(),
            RefusalKind::ZeroPeriod
        );
        assert_eq!(DeclaredPeriod::seconds(20).unwrap().get(), 20);
    }

    #[test]
    fn truth_is_a_function_of_seed_tick_and_period() {
        let world = World::new(identity());

        // Same identity and tick, twice: the same truth.
        let same = World::new(identity());
        assert_eq!(world.truth_digest(), same.truth_digest());

        // The tick changes truth.
        let ticked = world.with_tick(Tick(1));
        assert_ne!(world.truth_digest(), ticked.truth_digest());
        assert_eq!(world.tick(), Tick(0), "with_tick is pure");

        // The seed changes truth; the period changes truth.
        let other_seed = World::new(WorldIdentity::new(2_469, identity().period));
        assert_ne!(world.truth_digest(), other_seed.truth_digest());
        let other_period = World::new(WorldIdentity::new(
            2_468,
            DeclaredPeriod::seconds(2).unwrap(),
        ));
        assert_ne!(world.truth_digest(), other_period.truth_digest());

        // **A moved anchor and a populated cache do NOT change truth.**
        let mut viewed = World::new(identity());
        viewed.set_anchor(ChunkKey::new(-7, 3, 12));
        for x in -3..3 {
            viewed.reside(ChunkKey::new(x, 0, 0));
        }
        assert_eq!(viewed.anchor(), ChunkKey::new(-7, 3, 12));
        assert!(viewed.resident_count() > 0);
        assert_eq!(
            world.truth_digest(),
            viewed.truth_digest(),
            "the cache and the anchor must be invisible to truth"
        );
    }

    #[test]
    fn the_cache_digest_does_not_depend_on_insertion_order() {
        let keys = [
            ChunkKey::new(1, 0, 0),
            ChunkKey::new(-1, 2, 0),
            ChunkKey::new(0, 0, 5),
            ChunkKey::new(7, -3, 2),
        ];

        let digest_for = |order: &[usize]| {
            let mut w = World::new(identity());
            for &i in order {
                w.reside(keys[i]);
            }
            w.residency_digest()
        };

        let a = digest_for(&[0, 1, 2, 3]);
        let b = digest_for(&[3, 2, 1, 0]);
        let c = digest_for(&[2, 0, 3, 1]);
        assert_eq!(a, b);
        assert_eq!(a, c);

        // And it is blind to the tick — the cache is not the world.
        let mut ticked = World::new(identity()).with_tick(Tick(99));
        for key in keys {
            ticked.reside(key);
        }
        assert_eq!(a, ticked.residency_digest());

        // Eviction returns the cache to the empty digest.
        let mut w = World::new(identity());
        for key in keys {
            w.reside(key);
        }
        assert_eq!(w.resident_count(), 4);
        for key in keys {
            assert!(w.evict(key));
        }
        assert_eq!(w.resident_count(), 0);
        assert_eq!(w.residency_digest(), World::new(identity()).residency_digest());
    }

    #[test]
    fn a_plan_digest_is_order_independent() {
        let world = World::new(identity());
        let keys: Vec<ChunkKey> = (0..40).map(|i| ChunkKey::new(i, -i % 5, i / 3)).collect();

        let forward = world.plan_digest(keys.clone());
        let mut reversed = keys.clone();
        reversed.reverse();
        let backward = world.plan_digest(reversed);
        let shuffled: Vec<ChunkKey> = keys.iter().rev().step_by(2).chain(keys.iter().step_by(2)).copied().collect();
        let interleaved = world.plan_digest(shuffled);

        assert_eq!(forward, backward);
        assert_eq!(forward, interleaved);
        // A different set is a different plan.
        assert_ne!(forward, world.plan_digest(keys[..39].iter().copied()));
    }

    #[test]
    fn stage_seeds_differ_by_domain_and_by_position() {
        let world = World::new(identity());
        let pos = VoxelPos::new(3, 4, 5);

        // Every declared domain gives a distinct value at one position.
        let mut seen = BTreeSet::new();
        for domain in Domain::ALL {
            assert!(seen.insert(world.stage_seed(domain, pos)), "domain {domain:?} collided");
        }
        assert_eq!(seen.len(), Domain::COUNT);

        // A different position in the same domain gives a different value.
        assert_ne!(
            world.stage_seed(Domain::Terrain, pos),
            world.stage_seed(Domain::Terrain, VoxelPos::new(3, 4, 6))
        );

        // Pure: the same inputs give the same value, twice.
        assert_eq!(
            world.stage_seed(Domain::Geology, pos),
            world.stage_seed(Domain::Geology, pos)
        );

        // The tick participates.
        let ticked = world.with_tick(Tick(1));
        assert_ne!(
            world.stage_seed(Domain::Terrain, pos),
            ticked.stage_seed(Domain::Terrain, pos)
        );
    }

    #[test]
    fn stream_seeds_differ_by_domain_and_are_pure() {
        let world = World::new(identity());

        // **Five streams, five seeds.** A save that carried one seed and called the five streams
        // derived would be one dial with five names.
        let mut seen = BTreeSet::new();
        for domain in Domain::ALL {
            assert!(
                seen.insert(world.stream_seed(domain)),
                "domain {domain:?} collided with another domain's seed"
            );
        }
        assert_eq!(seen.len(), Domain::COUNT);

        // Pure: the same identity, tick and domain give the same number, twice.
        assert_eq!(
            world.stream_seed(Domain::Terrain),
            world.stream_seed(Domain::Terrain)
        );

        // The seed, the period and the tick each participate — a stream is the world's, not a constant.
        assert_ne!(
            world.stream_seed(Domain::Terrain),
            World::new(WorldIdentity::new(2_469, identity().period)).stream_seed(Domain::Terrain)
        );
        assert_ne!(
            world.stream_seed(Domain::Terrain),
            World::new(WorldIdentity::new(2_468, DeclaredPeriod::seconds(2).unwrap()))
                .stream_seed(Domain::Terrain)
        );
        assert_ne!(
            world.stream_seed(Domain::Terrain),
            world.with_tick(Tick(1)).stream_seed(Domain::Terrain)
        );
    }

    #[test]
    fn the_truth_digest_is_the_declared_bytes_and_not_something_else() {
        // Pins the canonical encoding: a change to it is a visible change here, not a silent one.
        let world = World::new(WorldIdentity::new(1, DeclaredPeriod::seconds(1).unwrap()));
        let expected = Digest::of_parts(&[
            b"shploingus.world-truth/1",
            &1u64.to_le_bytes(),
            &1u64.to_le_bytes(),
            &0u64.to_le_bytes(),
        ]);
        assert_eq!(world.truth_digest(), expected);
        assert_eq!(world.truth_digest().to_hex().len(), 64);
    }

    #[test]
    fn a_plan_digest_does_not_cancel_a_duplicate() {
        let world = World::new(identity());
        let a = ChunkKey::new(1, 2, 3);
        let empty = world.plan_digest(Vec::<ChunkKey>::new());
        let once = world.plan_digest([a]);
        let twice = world.plan_digest([a, a]);
        assert_ne!(twice, empty, "a duplicated chunk must not read as an empty plan");
        assert_ne!(
            twice, once,
            "a plan that names a chunk twice is not the plan that names it once"
        );
    }
}
