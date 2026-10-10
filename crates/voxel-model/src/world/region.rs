//! `region` — the **declared, validated region** a governed change may land in.
//!
//! # What is deliberately *not* here, and why that is the whole point of the file
//!
//! `ala-cities/src/world/region.rs` (141 lines) defines `Coord`, `ChunkCoord`, `CHUNK_SIZE` and
//! the first-band constants. **This tree already owns all four**, and the first draft of this
//! file copied them — which is `event-0131` (the `season`) repeating one layer down. The
//! measurement:
//!
//! | ala-cities | this tree, already | verified |
//! |---|---|---|
//! | `Coord { x, y, z: i32 }` | `coord::VoxelPos { x, y, z: i32 }` | `coord.rs:54` |
//! | `ChunkCoord` | `coord::ChunkPos` | `coord.rs:61` |
//! | `CHUNK_SIZE = 32` | `coord::CHUNK_SIDE = 32` | `coord.rs:17` |
//! | `FIRST_BAND_MIN_Z = -16` | `coord::BAND_MIN_Z = -16` | `coord.rs:29` |
//! | `FIRST_BAND_MAX_Z = 47` | `coord::BAND_MAX_Z - 1` — the tree's is **exclusive** (`48`) | `coord.rs:31` |
//!
//! So the machine's `crate::world::Coord` is a **re-export** of the tree's own type under the
//! machine's own name, not a second type. That is what "copied whole, seams only" means when the
//! tree already owns the thing: the seam is a `pub use`, and the machine stays byte-identical.
//!
//! **The inclusive/exclusive difference is real and is named rather than papered over.** The
//! source's band is `-16..=47`; this tree's `BAND_MIN_Z..BAND_MAX_Z` is `-16..48`, one past the
//! last. An earlier draft asserted `FIRST_BAND_MAX_Z == coord::BAND_MAX_Z` — an assertion that
//! would have been **false**, and whose only purpose was to make a duplicated constant feel safe.
//!
//! # Why `SignedRegion` *is* here
//!
//! It is the one thing in `region.rs` this tree does not have, and it is load-bearing beyond
//! convenience: `SignedRegion::new` **refuses** a region that does not span the first playable
//! band rather than clamping it, and `forbidden.box-selection` in `architecture/workspace.dsl`
//! requires exactly that refusal shape of the world selection — *"a declared, validated region
//! that refuses rather than clamps"*. A refusal that carries the offending region back is the
//! difference between a caller that can fix its bounds and one that only learns it failed.

use super::WorldError;
use crate::coord::{BAND_MAX_Z, BAND_MIN_Z};
use core::fmt;

/// The lowest voxel z in the first playable band, **inclusive**.
///
/// The source's `FIRST_BAND_MIN_Z`; the tree's `BAND_MIN_Z`, which is also inclusive.
pub const FIRST_BAND_MIN_Z: i32 = BAND_MIN_Z;

/// The highest voxel z in the first playable band, **inclusive**.
///
/// The source's `FIRST_BAND_MAX_Z = 47`. This tree's `BAND_MAX_Z` is `48` and is **one past the
/// last**, documented as the half-open range's end. Derived, never restated as a literal.
pub const FIRST_BAND_MAX_Z: i32 = BAND_MAX_Z - 1;

/// The machine's `Coord`, which is this tree's [`VoxelPos`]. One type, two names, and the name
/// lives here because the machine reads `crate::world::Coord`.
pub use crate::coord::VoxelPos as Coord;

/// The machine's `ChunkCoord`, which is this tree's [`crate::coord::ChunkPos`].
pub use crate::coord::ChunkPos as ChunkCoord;

/// An axis-aligned, band-checked region.
///
/// Copied whole from `ala-cities/src/world/region.rs` except for the two substitutions above.
/// The invariant `first_band_z == (FIRST_BAND_MIN_Z, FIRST_BAND_MAX_Z)` is part of the type's
/// shape and is re-checked by [`SignedRegion::validate`], because a region is a value that
/// crosses a boundary and a value that crosses a boundary can be built by hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedRegion {
    pub min: Coord,
    pub max: Coord,
    pub first_band_z: (i32, i32),
}

impl SignedRegion {
    /// The band-checked region between two corners.
    ///
    /// Refuses — naming the region, not merely failing — when the span does not cover the first
    /// playable band. It does **not** clamp: a clamped region is a region the caller believes it
    /// has and does not.
    pub fn new(a: Coord, b: Coord) -> Result<Self, WorldError> {
        let min = Coord::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let max = Coord::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
        if min.z > FIRST_BAND_MIN_Z || max.z < FIRST_BAND_MAX_Z {
            return Err(WorldError::OutOfRegionAccess {
                region: SignedRegion {
                    min,
                    max,
                    first_band_z: (FIRST_BAND_MIN_Z, FIRST_BAND_MAX_Z),
                },
                coord: Coord::new(0, 0, FIRST_BAND_MIN_Z),
            });
        }

        Ok(Self {
            min,
            max,
            first_band_z: (FIRST_BAND_MIN_Z, FIRST_BAND_MAX_Z),
        })
    }

    /// Re-check an invariant the constructor already established.
    ///
    /// The constructor is not the only way a region arrives — a region that crossed a boundary
    /// was built from bytes on the other side — so the check exists independently of it. Its
    /// first reader is the governed apply, which is the next landing.
    #[allow(dead_code)]
    pub(crate) fn validate(&self) -> Result<(), WorldError> {
        if self.min.x > self.max.x
            || self.min.y > self.max.y
            || self.min.z > self.max.z
            || self.first_band_z != (FIRST_BAND_MIN_Z, FIRST_BAND_MAX_Z)
            || self.min.z > FIRST_BAND_MIN_Z
            || self.max.z < FIRST_BAND_MAX_Z
        {
            return Err(WorldError::OutOfRegionAccess {
                region: *self,
                coord: Coord::new(0, 0, FIRST_BAND_MIN_Z),
            });
        }
        Ok(())
    }

    pub fn contains(&self, coord: Coord) -> bool {
        (self.min.x..=self.max.x).contains(&coord.x)
            && (self.min.y..=self.max.y).contains(&coord.y)
            && (self.min.z..=self.max.z).contains(&coord.z)
    }

    pub fn contains_chunk(&self, chunk: ChunkCoord) -> bool {
        let min = self.min.chunk();
        let max = self.max.chunk();
        (min.x..=max.x).contains(&chunk.x)
            && (min.y..=max.y).contains(&chunk.y)
            && (min.z..=max.z).contains(&chunk.z)
    }
}

impl fmt::Display for SignedRegion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}..={}",
            super::coord_text(self.min),
            super::coord_text(self.max)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The band is derived from this tree's, and the difference in convention is the point.**
    /// The source's top is inclusive; the tree's is exclusive. A test that asserted equality
    /// would be asserting a bug.
    #[test]
    fn the_band_is_this_trees_band_and_the_conventions_differ_by_one() {
        assert_eq!(FIRST_BAND_MIN_Z, crate::coord::BAND_MIN_Z);
        assert_eq!(FIRST_BAND_MAX_Z, crate::coord::BAND_MAX_Z - 1);
        assert_eq!(FIRST_BAND_MAX_Z + 1, crate::coord::BAND_MAX_Z);
        assert_ne!(FIRST_BAND_MAX_Z, crate::coord::BAND_MAX_Z);
    }

    #[test]
    fn a_region_that_misses_the_band_is_refused_not_clamped() {
        let low = SignedRegion::new(Coord::new(0, 0, 0), Coord::new(1, 1, 1));
        let error = low.expect_err("a region above the band is refused");
        assert_eq!(error.kind_name(), "out_of_region_access");

        let ok = SignedRegion::new(Coord::new(-32, -32, -16), Coord::new(31, 31, 47)).unwrap();
        assert!(ok.contains(Coord::new(0, 0, 0)));
        assert!(ok.contains(Coord::new(31, 31, 47)));
        assert!(!ok.contains(Coord::new(32, 0, 0)));
    }

    #[test]
    fn chunking_is_euclidean_because_the_trees_own_type_says_so() {
        assert_eq!(Coord::new(-1, -1, -1).chunk(), ChunkCoord::new(-1, -1, -1));
    }
}
