//! The 256× detail lattice: **addressed, never stored** (C-2).
//!
//! A subvoxel's material detail is a *deterministic function of its address*,
//! evaluated when the renderer asks for it and forgotten immediately. There is
//! no grid here, no allocation, and no cache: [`DetailLattice`] is a `u64` seed
//! and nothing else, which a test in this module asserts with
//! `size_of::<DetailLattice>()`.
//!
//! # The arithmetic that justifies it, measured
//!
//! The alternative is storage, and the figure is worth writing down because it is
//! the whole argument:
//!
//! | Level | Dense samples |
//! |---|---|
//! | one **subvoxel** (256³) | **16,777,216** |
//! | one **voxel** (64 subvoxels) | **1,073,741,824** |
//! | one **chunk** (32,768 voxels) | **35,184,372,088,832** |
//!
//! That is 2⁴⁵ samples for a single chunk — about **35 TB** at one byte a sample.
//! `DETAIL_SIDE` is *per subvoxel axis*, so the per-voxel figure is 64× the
//! per-subvoxel one. `APPEARANCE.md` §2's prose said "16,777,216 samples per
//! voxel … 5.5 × 10¹¹ per chunk", which is the same number read as per-voxel: it
//! is low by exactly **64×**, the subvoxel count. The correction is recorded
//! (`LEDGER.md` C-27) and the wrong figure is pinned by a test that asserts it
//! does **not** reproduce — the same shape as C-10.
//!
//! # Why the subvoxel is part of the address
//!
//! `APPEARANCE.md` §2 writes the function as `detail(material, subvoxel, i, j, k)`,
//! and the subvoxel is not decoration: a lattice that ignored it would be one
//! 256³ pattern **tiled 64 times** across the voxel, which is visible at every
//! subvoxel boundary. Folding the subvoxel into the lattice's seed makes each
//! subvoxel its own lattice, still a pure function of `(seed, material,
//! subvoxel, address)`.

use crate::provenance::LATTICE_DOMAIN_TAG;
use voxel_model::{DetailIndex, MaterialId, Refusal, RefusalKind, Refused, SubvoxelIndex, DETAIL_SIDE, SUBVOXEL_COUNT, CHUNK_VOLUME};
use voxel_noise::{mix, rng::signed3};

/// The two channels the lattice varies. Domain tags, so the roughness value at
/// an address can never be the reflectance value at the same address.
const CHANNEL_ROUGHNESS: i64 = 1;
const CHANNEL_REFLECTANCE: i64 = 2;

/// Dense samples in one subvoxel's lattice if it were stored: `256³`.
pub const DENSE_SAMPLES_PER_SUBVOXEL: u64 =
    DETAIL_SIDE as u64 * DETAIL_SIDE as u64 * DETAIL_SIDE as u64;
/// Dense samples in one voxel: 64 subvoxels' worth.
pub const DENSE_SAMPLES_PER_VOXEL: u64 = DENSE_SAMPLES_PER_SUBVOXEL * SUBVOXEL_COUNT as u64;
/// Dense samples in one chunk. **2⁴⁵.**
pub const DENSE_SAMPLES_PER_CHUNK: u64 = DENSE_SAMPLES_PER_VOXEL * CHUNK_VOLUME as u64;

/// What the material detail looks like at one address: a deviation from the
/// material's own base inputs, never a substitute for them.
///
/// Both channels are in `[-1, 1]` and the lattice's mean is **zero by symmetry**,
/// so a subvoxel's detail carries no net bias — see [`DetailSample::NONE`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DetailSample {
    /// How much rougher (`+`) or smoother (`−`) than the material's base.
    pub roughness_delta: f64,
    /// How much more (`+`) or less (`−`) reflective than the material's base.
    pub reflectance_delta: f64,
}

impl DetailSample {
    /// No variation at all — the lattice's own mean.
    ///
    /// This is a *derived* zero and not a fudge: the hash maps a uniform field
    /// onto `[−1, 1)`, so the expected deviation is exactly zero for any material.
    /// A test measures that over a sample rather than assuming it.
    pub const NONE: DetailSample = DetailSample { roughness_delta: 0.0, reflectance_delta: 0.0 };

    pub fn is_finite(&self) -> bool {
        self.roughness_delta.is_finite() && self.reflectance_delta.is_finite()
    }
}

impl Default for DetailSample {
    fn default() -> Self {
        Self::NONE
    }
}

/// The addressed detail lattice. One `u64`; nothing is stored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DetailLattice {
    seed: u64,
}

impl DetailLattice {
    pub const fn new(seed: u64) -> Self {
        DetailLattice { seed }
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Samples per axis — the `256` of "256×".
    pub const fn side(&self) -> u32 {
        DETAIL_SIDE
    }

    /// The seed of one `(material, subvoxel)` pair's own lattice.
    ///
    /// Folded through [`mix`] rather than added, so neighbouring materials and
    /// neighbouring subvoxels do not produce correlated lattices — an additive
    /// fold would leave `(2, 3)` and `(3, 2)` on the same lattice.
    fn domain(&self, material: MaterialId, subvoxel: SubvoxelIndex) -> u64 {
        let cell = (subvoxel.flat() as u64).wrapping_add(1);
        mix(self.seed ^ mix(LATTICE_DOMAIN_TAG ^ mix(material.0 as u64 ^ cell)))
    }

    /// The address's flat index in `0..256³`. `x` is the fast axis, which is what
    /// makes `x` the one that must fit in eight bits.
    const fn flat(at: DetailIndex) -> i64 {
        at.x as i64 | ((at.y as i64) << 8) | ((at.z as i64) << 16)
    }

    /// The detail at one address.
    ///
    /// Refuses an address outside the lattice rather than hashing it: the fields
    /// of [`DetailIndex`] are public, so a caller can build one by hand, and a
    /// hand-built address of `(256, 0, 0)` would otherwise alias onto `(0, 0, 1)`
    /// through the flat index and quietly give the wrong sample.
    pub fn sample(
        &self,
        material: MaterialId,
        subvoxel: SubvoxelIndex,
        at: DetailIndex,
    ) -> Refused<DetailSample> {
        if DetailIndex::new(at.x, at.y, at.z).is_err() {
            return Err(Refusal::owned(
                RefusalKind::IndexOutOfRange,
                format!("detail address ({}, {}, {}) outside 0..{DETAIL_SIDE}", at.x, at.y, at.z),
            ));
        }
        let domain = self.domain(material, subvoxel);
        let flat = Self::flat(at);
        Ok(DetailSample {
            roughness_delta: signed3(domain, CHANNEL_ROUGHNESS, flat, LATTICE_DOMAIN_TAG as i64),
            reflectance_delta: signed3(domain, CHANNEL_REFLECTANCE, flat, LATTICE_DOMAIN_TAG as i64),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lattice() -> DetailLattice {
        DetailLattice::new(0x5A17_0000_0000_0001)
    }

    fn sub(x: u8, y: u8, z: u8) -> SubvoxelIndex {
        SubvoxelIndex::new(x, y, z).unwrap()
    }

    fn at(x: u16, y: u16, z: u16) -> DetailIndex {
        DetailIndex::new(x, y, z).unwrap()
    }

    #[test]
    fn nothing_is_stored() {
        // The strongest statement this crate can make about C-2: the lattice is
        // a seed. If it ever grows a grid, this test fails.
        assert_eq!(std::mem::size_of::<DetailLattice>(), std::mem::size_of::<u64>());
    }

    #[test]
    fn detail_is_a_pure_function_of_its_address() {
        let l = lattice();
        assert_eq!(l.sample(MaterialId(6), sub(1, 2, 3), at(7, 8, 9)).unwrap(), l.sample(MaterialId(6), sub(1, 2, 3), at(7, 8, 9)).unwrap());
        assert_eq!(DetailLattice::new(4).sample(MaterialId(1), sub(0, 0, 0), at(1, 1, 1)).unwrap(), DetailLattice::new(4).sample(MaterialId(1), sub(0, 0, 0), at(1, 1, 1)).unwrap());
    }

    #[test]
    fn the_material_the_subvoxel_and_the_seed_all_change_the_lattice() {
        let base = lattice().sample(MaterialId(6), sub(1, 1, 1), at(3, 3, 3)).unwrap();
        assert_ne!(base, lattice().sample(MaterialId(7), sub(1, 1, 1), at(3, 3, 3)).unwrap());
        // The subvoxel matters: otherwise the one 256³ pattern is tiled 64 times.
        assert_ne!(base, lattice().sample(MaterialId(6), sub(1, 1, 2), at(3, 3, 3)).unwrap());
        assert_ne!(base, DetailLattice::new(99).sample(MaterialId(6), sub(1, 1, 1), at(3, 3, 3)).unwrap());
    }

    #[test]
    fn every_channel_of_the_lattice_is_in_range() {
        let l = lattice();
        for i in 0..64u16 {
            let s = l.sample(MaterialId(3), sub(0, 0, 0), at(i, 255 - i, (i * 7) % 256)).unwrap();
            for v in [s.roughness_delta, s.reflectance_delta] {
                assert!((-1.0..1.0).contains(&v), "detail channel out of range: {v}");
            }
        }
    }

    #[test]
    fn the_two_channels_are_domain_separated() {
        // Same address, same material: if the channels were one hash, roughness
        // and reflectance would move together and every material would show one
        // pattern instead of two.
        let l = lattice();
        let mut same = 0;
        for i in 0..256u16 {
            let s = l.sample(MaterialId(4), sub(2, 2, 2), at(i, i, i)).unwrap();
            if s.roughness_delta == s.reflectance_delta {
                same += 1;
            }
        }
        assert_eq!(same, 0, "the channels coincided at {same} of 256 addresses");
    }

    #[test]
    fn adjacent_addresses_differ_so_the_lattice_really_resolves_256() {
        // A lattice that quantised its address — say to 32 levels — would still
        // pass a "two addresses differ" test. Stepping *every* address along each
        // axis and requiring a change proves the resolution is real at one sample.
        let l = lattice();
        let material = MaterialId(6);
        let s = sub(0, 0, 0);
        for axis in 0..3u16 {
            let mut changed = 0;
            for i in 0..255u16 {
                let (a, b) = match axis {
                    0 => (at(i, 128, 128), at(i + 1, 128, 128)),
                    1 => (at(128, i, 128), at(128, i + 1, 128)),
                    _ => (at(128, 128, i), at(128, 128, i + 1)),
                };
                if l.sample(material, s, a).unwrap() != l.sample(material, s, b).unwrap() {
                    changed += 1;
                }
            }
            assert!(
                changed > 240,
                "axis {axis} only changed at {changed} of 255 adjacent pairs, so the lattice is aliasing"
            );
        }
    }

    #[test]
    fn the_mean_deviation_is_zero_as_the_hash_s_symmetry_says() {
        // `DetailSample::NONE` claims the lattice carries no net bias. That is a
        // claim about the hash's distribution, so it is measured rather than
        // assumed. The tolerance is a measurement tolerance for this sample
        // count — a statistical bound, not a constant about the world.
        let l = lattice();
        let mut sum_r = 0.0;
        let mut sum_f = 0.0;
        let n = 4096u32;
        for i in 0..n {
            let x = (i % 256) as u16;
            let y = ((i / 16) % 256) as u16;
            let z = ((i * 13) % 256) as u16;
            let s = l.sample(MaterialId(6), sub(0, 0, 0), at(x, y, z)).unwrap();
            sum_r += s.roughness_delta;
            sum_f += s.reflectance_delta;
        }
        let mean_r = sum_r / n as f64;
        let mean_f = sum_f / n as f64;
        assert!(mean_r.abs() < 0.03, "mean roughness deviation was {mean_r}");
        assert!(mean_f.abs() < 0.03, "mean reflectance deviation was {mean_f}");
    }

    #[test]
    fn an_address_outside_the_lattice_is_refused_rather_than_wrapped() {
        // A hand-built index the constructor would have refused. `(256, 0, 0)`
        // flattens onto the same integer as `(0, 0, 1)`, which is exactly the
        // aliasing this refusal exists to prevent.
        let bad = DetailIndex { x: 256, y: 0, z: 0 };
        let r = lattice().sample(MaterialId(6), sub(0, 0, 0), bad).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::IndexOutOfRange);
    }

    #[test]
    fn the_dense_figures_are_two_to_the_forty_fifth_and_not_appearance_md_s() {
        assert_eq!(DENSE_SAMPLES_PER_SUBVOXEL, 16_777_216);
        assert_eq!(DENSE_SAMPLES_PER_VOXEL, 1_073_741_824);
        assert_eq!(DENSE_SAMPLES_PER_CHUNK, 35_184_372_088_832);
        assert_eq!(DENSE_SAMPLES_PER_CHUNK, 1u64 << 45);

        // The figure `APPEARANCE.md` §2 asserted: 16,777,216 read as *per voxel*,
        // which makes a chunk 5.5e11. It is 64x low, and the gap is exactly the
        // subvoxel count — and it must not reproduce here. C-10's shape.
        let wrong = 16_777_216u64 * 32_768;
        assert_eq!(wrong, 549_755_813_888);
        assert_ne!(DENSE_SAMPLES_PER_CHUNK, wrong);
        assert_eq!(DENSE_SAMPLES_PER_CHUNK / wrong, SUBVOXEL_COUNT as u64);
    }
}
