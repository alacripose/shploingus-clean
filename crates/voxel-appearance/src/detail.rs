//! The detail lattice — the base the record calls **256**: **addressed, never
//! stored** (C-2).
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
//! | one **subvoxel** (16³) | **4,096** |
//! | one **voxel** (64 subvoxels) | **262,144** |
//! | one **chunk** (32,768 voxels) | **8,589,934,592** |
//!
//! That is 2³³ samples for a single chunk — about **8.6 GB** at one byte a sample,
//! for one chunk of a world addressed at 15.6 mm detail cells. `DETAIL_SIDE` is
//! *per subvoxel axis* and is **16**, which is why the base is called 256: a
//! subvoxel **face** carries 16×16 = **256** samples, exactly as the composition
//! base is 4×4×4 within the voxel. The user set that reading on 2026-10-05
//! (*"256 was supposed to be 16x16x16 like the 64 base was 4x4x4"*), correcting
//! this header's earlier 256-per-axis form; the superseded figure is pinned by a
//! test that asserts it does **not** reproduce — the same shape as C-10 and C-28.
//!
//! # Why the subvoxel is part of the address
//!
//! `APPEARANCE.md` §2 writes the function as `detail(material, subvoxel, i, j, k)`,
//! and the subvoxel is not decoration: a lattice that ignored it would be one
//! 16³ pattern **tiled 64 times** across the voxel, which is visible at every
//! subvoxel boundary. Folding the subvoxel into the lattice's seed makes each
//! subvoxel its own lattice, still a pure function of `(seed, material,
//! subvoxel, address)`.
//!
//! # Which subvoxel, and the correction that answer needed (C-162)
//!
//! The subvoxel that folds in is the **world's** own cell, `floor(p / 0.25 m)`
//! per axis — not the subvoxel's index inside its block. The first form of this
//! code used the block-local index, and it is the same number in every block of
//! the world: every block therefore drew the *same* pattern, a one-metre stamp.
//! The eye reads a repeated motif as a grid, and the user's eye-check said so
//! (*"the tiling is very noticeable"*), which is the same argument the paragraph
//! above makes at the subvoxel boundary — a lattice that ignores where its cell
//! is repeats at that boundary, and the block boundary is one of them. C-91
//! already names the instance the field lives on; the world cell is that
//! instance.

use crate::provenance::LATTICE_DOMAIN_TAG;
use voxel_model::{DetailIndex, MaterialId, Refusal, RefusalKind, Refused, DETAIL_SIDE, SUBVOXEL_COUNT, CHUNK_VOLUME};
use voxel_noise::{mix, rng::signed3};

/// The two channels the lattice varies. Domain tags, so the roughness value at
/// an address can never be the reflectance value at the same address.
const CHANNEL_ROUGHNESS: i64 = 1;
const CHANNEL_REFLECTANCE: i64 = 2;

/// Dense samples in one subvoxel's lattice if it were stored: `16³`.
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

    /// Samples per axis — **16**. The base the record calls `256`, because a
    /// subvoxel face carries 16×16 samples.
    pub const fn side(&self) -> u32 {
        DETAIL_SIDE
    }

    /// The seed of one `(material, world subvoxel)` pair's own lattice.
    ///
    /// Folded through [`mix`] rather than added, so neighbouring materials and
    /// neighbouring cells do not produce correlated lattices — an additive fold
    /// would leave `(2, 3)` and `(3, 2)` on the same lattice.
    ///
    /// The three axes fold in the order `shading.wgsl` transcribes:
    /// `mix(x ^ mix(y ^ mix(z)))`. Each cell coordinate enters as 32 bits
    /// (`cell[i] as u32`) because the shader's cells arrive as `i32` and WGSL
    /// cannot express a 64-bit sign extension; a `u64` fold of the sign-extended
    /// value would not be the function the GPU draws.
    fn domain(&self, material: MaterialId, cell: [i32; 3]) -> u64 {
        let x = cell[0] as u32 as u64;
        let y = cell[1] as u32 as u64;
        let z = cell[2] as u32 as u64;
        let folded = mix(x ^ mix(y ^ mix(z)));
        let inner = mix(material.0 as u64 ^ folded);
        let mid = mix(LATTICE_DOMAIN_TAG ^ inner);
        mix(self.seed ^ mid)
    }

    /// The address's flat index, packed at the side's own bit width: `x` is the
    /// fast axis. Packing them **tight** is what let the refusal below catch the
    /// address that would otherwise alias onto another axis's bits, and
    /// `shading.wgsl` derives the same shift from the side it is sent.
    const fn flat(at: DetailIndex) -> i64 {
        let shift = DETAIL_SIDE.trailing_zeros();
        at.x as i64 | ((at.y as i64) << shift) | ((at.z as i64) << (2 * shift))
    }

    /// The detail at one address of one **world subvoxel**.
    ///
    /// `cell` is the subvoxel's own place in the world, in subvoxel units
    /// (`floor(p / 0.25 m)` per axis), so two cells of one material never share a
    /// pattern (C-162).
    ///
    /// Refuses an address outside the lattice rather than hashing it: the fields
    /// of [`DetailIndex`] are public, so a caller can build one by hand, and a
    /// hand-built address of `(16, 0, 0)` would otherwise alias onto `(0, 1, 0)`
    /// through the flat index and quietly give the wrong sample.
    pub fn sample(
        &self,
        material: MaterialId,
        cell: [i32; 3],
        at: DetailIndex,
    ) -> Refused<DetailSample> {
        if DetailIndex::new(at.x, at.y, at.z).is_err() {
            return Err(Refusal::owned(
                RefusalKind::IndexOutOfRange,
                format!("detail address ({}, {}, {}) outside 0..{DETAIL_SIDE}", at.x, at.y, at.z),
            ));
        }
        let domain = self.domain(material, cell);
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
    use voxel_model::SUBVOXEL_SIDE;

    fn lattice() -> DetailLattice {
        DetailLattice::new(0x5A17_0000_0000_0001)
    }

    fn cell(x: i32, y: i32, z: i32) -> [i32; 3] {
        [x, y, z]
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
        assert_eq!(l.sample(MaterialId(6), cell(1, 2, 3), at(7, 8, 9)).unwrap(), l.sample(MaterialId(6), cell(1, 2, 3), at(7, 8, 9)).unwrap());
        assert_eq!(DetailLattice::new(4).sample(MaterialId(1), cell(0, 0, 0), at(1, 1, 1)).unwrap(), DetailLattice::new(4).sample(MaterialId(1), cell(0, 0, 0), at(1, 1, 1)).unwrap());
    }

    #[test]
    fn the_material_the_cell_and_the_seed_all_change_the_lattice() {
        let base = lattice().sample(MaterialId(6), cell(1, 1, 1), at(3, 3, 3)).unwrap();
        assert_ne!(base, lattice().sample(MaterialId(7), cell(1, 1, 1), at(3, 3, 3)).unwrap());
        // The cell matters: otherwise the one 16³ pattern is tiled across the world.
        assert_ne!(base, lattice().sample(MaterialId(6), cell(1, 1, 2), at(3, 3, 3)).unwrap());
        assert_ne!(base, DetailLattice::new(99).sample(MaterialId(6), cell(1, 1, 1), at(3, 3, 3)).unwrap());
    }

    #[test]
    fn the_pattern_does_not_repeat_every_block() {
        // **The correction C-162 exists for.** The first addressing folded the
        // subvoxel's index *inside its block*, which is the same number in every
        // block: every block drew the same pattern, a one-metre stamp the eye
        // reads as tiling. Four subvoxels make one voxel, so `cell` and
        // `cell + 4` are one block apart with the same local index — they must
        // not share a value. Measured over many cells and many addresses rather
        // than one pair, because one `assert_ne!` can pass by luck.
        let l = lattice();
        let material = MaterialId(6);
        let mut same = 0;
        let mut compared = 0;
        for i in 0..64i32 {
            let here = cell(i, 0, 0);
            let one_block_over = cell(i + SUBVOXEL_SIDE as i32, 0, 0);
            for a in 0..16u16 {
                let here_sample = l.sample(material, here, at(a, 15 - a, (a * 7) % 16)).unwrap();
                let there_sample =
                    l.sample(material, one_block_over, at(a, 15 - a, (a * 7) % 16)).unwrap();
                if here_sample == there_sample {
                    same += 1;
                }
                compared += 1;
            }
        }
        assert_eq!(compared, 64 * 16);
        // One sample in two million colliding is the hash's birthday, not a
        // pattern; a stamp of the old form would make this *all* of them.
        assert!(same <= 2, "{same} of {compared} cells one block apart shared a pattern");
    }

    #[test]
    fn every_channel_of_the_lattice_is_in_range() {
        // Every address of the lattice: 16³ = 4,096 of them.
        let l = lattice();
        for i in 0..4096u32 {
            let (x, y, z) = ((i % 16) as u16, ((i / 16) % 16) as u16, (i / 256) as u16);
            let s = l.sample(MaterialId(3), cell(0, 0, 0), at(x, y, z)).unwrap();
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
        for i in 0..16u16 {
            let s = l.sample(MaterialId(4), cell(2, 2, 2), at(i, i, i)).unwrap();
            if s.roughness_delta == s.reflectance_delta {
                same += 1;
            }
        }
        assert_eq!(same, 0, "the channels coincided at {same} of 16 addresses");
    }

    #[test]
    fn adjacent_addresses_differ_so_the_lattice_really_resolves_its_side() {
        // A lattice that quantised its address — say to 8 levels — would still
        // pass a "two addresses differ" test. Stepping *every* address along each
        // axis and requiring a change proves the resolution is real at one sample,
        // all the way to the side the base declares (16, so 15 steps an axis).
        let l = lattice();
        let material = MaterialId(6);
        let s = cell(0, 0, 0);
        for axis in 0..3u16 {
            let mut changed = 0;
            for i in 0..15u16 {
                let (a, b) = match axis {
                    0 => (at(i, 8, 8), at(i + 1, 8, 8)),
                    1 => (at(8, i, 8), at(8, i + 1, 8)),
                    _ => (at(8, 8, i), at(8, 8, i + 1)),
                };
                if l.sample(material, s, a).unwrap() != l.sample(material, s, b).unwrap() {
                    changed += 1;
                }
            }
            assert!(
                changed > 12,
                "axis {axis} only changed at {changed} of 15 adjacent pairs, so the lattice is aliasing"
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
        // The whole lattice, 16³ = 4,096 addresses.
        let n = 4096u32;
        for i in 0..n {
            let x = (i % 16) as u16;
            let y = ((i / 16) % 16) as u16;
            let z = (i / 256) as u16;
            let s = l.sample(MaterialId(6), cell(0, 0, 0), at(x, y, z)).unwrap();
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
        // A hand-built index the constructor would have refused. With four bits an
        // axis, `(16, 0, 0)` flattens onto the same integer as `(0, 1, 0)`, which
        // is exactly the aliasing this refusal exists to prevent.
        let bad = DetailIndex { x: 16, y: 0, z: 0 };
        assert_eq!(DetailLattice::flat(bad), DetailLattice::flat(DetailIndex { x: 0, y: 1, z: 0 }));
        let r = lattice().sample(MaterialId(6), cell(0, 0, 0), bad).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::IndexOutOfRange);
    }

    #[test]
    fn the_dense_figures_are_two_to_the_thirty_third_and_not_the_superseded_one() {
        assert_eq!(DENSE_SAMPLES_PER_SUBVOXEL, 4_096);
        assert_eq!(DENSE_SAMPLES_PER_VOXEL, 262_144);
        assert_eq!(DENSE_SAMPLES_PER_CHUNK, 8_589_934_592);
        assert_eq!(DENSE_SAMPLES_PER_CHUNK, 1u64 << 33);

        // The superseded reading: 256 samples *per axis*, which made a subvoxel
        // 16,777,216 and a chunk 2⁴⁵ (35 TB). The base's name promised 16×16×16 —
        // a 256-per-face lattice — and the old figure must not reproduce. C-10's
        // shape, one correction later.
        let superseded = 16_777_216u64 * SUBVOXEL_COUNT as u64 * CHUNK_VOLUME as u64;
        assert_eq!(superseded, 1u64 << 45);
        assert_ne!(DENSE_SAMPLES_PER_CHUNK, superseded);
        assert_eq!(superseded / DENSE_SAMPLES_PER_CHUNK, 4_096);
    }
}
