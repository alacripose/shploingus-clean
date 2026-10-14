//! Regional fields — F4's 20-level ladder, and the identity rule that goes with it.
//!
//! **Source.** The level count and the alignment are the F4 ruling, not a paper:
//! a 16-level ladder was measured to top out at `32 · 2¹⁵ = 1,048,576` blocks —
//! a 1.05 km world, which is a *hill*, not a world — and was superseded at **20
//! levels**, giving `32 · 2¹⁹ = 16,777,216` blocks ≈ 16.8 km. The interpolation
//! machinery is Shepard, D. *A Two-Dimensional Interpolation Function for
//! Irregularly-Spaced Data.* ACM 1968, in its regular-grid special case.
//!
//! # The ladder carries geology only
//!
//! After climate became a **process** rather than a field (F4), the ladder carries
//! **lithology, elevation and soil depth**. Climate is read from process state.
//! Carrying a climate *field* as well would be two records of one truth, which is
//! the thing D34 forbids one layer up.
//!
//! # Identity binds **by effect, not by kind**
//!
//! Anything that can change a derived authoritative value is part of the field's
//! identity: the level count, the interpolation method, the provenance and every
//! declared tolerance. Only **provably value-independent** metadata is split out.
//! That is what makes a value-affecting generator change *refuse* instead of
//! silently re-deriving a world that no longer matches its recorded evidence.
use voxel_model::endian::LeBytes;

use voxel_model::{Identity, Refusal, RefusalKind, Refused};
use voxel_model::coord::CHUNK_SIDE;

/// The ladder's level count, as ruled (F4).
pub const STANDARD_LEVELS: u32 = 20;

/// The world extent at a level, in blocks: `32 · 2^level`.
///
/// The `32` is not arbitrary — it is [`CHUNK_SIDE`]. The ladder is **aligned to
/// the octree**, so a field level and the octree node a chunk samples at are the
/// same structure rather than two that must be kept in step.
pub fn blocks_at_level(level: u32) -> u64 {
    (CHUNK_SIDE as u64) << level
}

/// A level in the ladder. Refuses an index the ladder does not have.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Level(u32);

impl Level {
    pub const fn index(self) -> u32 {
        self.0
    }

    pub fn new(index: u32, ladder_levels: u32) -> Refused<Self> {
        if index >= ladder_levels {
            return Err(Refusal::owned(
                RefusalKind::IndexOutOfRange,
                format!("field level {index} is outside a {ladder_levels}-level ladder"),
            ));
        }
        Ok(Level(index))
    }

    /// This level's world extent, in blocks.
    pub fn blocks(self) -> u64 {
        blocks_at_level(self.0)
    }

    /// The lattice coordinate at this level for a world position.
    ///
    /// Uses **Euclidean** division, for the same reason [`voxel_model::coord`]
    /// does: truncating division mirrors the world about the origin, and a
    /// mirrored field is a field that disagrees with itself across x = 0.
    pub fn lattice_of(self, x: i32, z: i32) -> (i64, i64) {
        let b = self.blocks() as i64;
        ((x as i64).div_euclid(b), (z as i64).div_euclid(b))
    }

    /// The world position at the low corner of a lattice cell.
    pub fn world_of(self, i: i64, j: i64) -> (i64, i64) {
        let b = self.blocks() as i64;
        (i * b, j * b)
    }
}

/// The three things the ladder carries.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum FieldKind {
    Lithology,
    Elevation,
    SoilDepth,
}

impl FieldKind {
    /// Every kind the ladder carries. A kind added here without an entry in
    /// [`FieldKind::name`] does not compile; a kind added without being in this
    /// list is caught by [`tests::every_kind_is_carried`].
    pub const ALL: [FieldKind; 3] = [FieldKind::Lithology, FieldKind::Elevation, FieldKind::SoilDepth];

    pub const fn name(self) -> &'static str {
        match self {
            FieldKind::Lithology => "lithology",
            FieldKind::Elevation => "elevation",
            FieldKind::SoilDepth => "soil_depth",
        }
    }

    /// The authoritative unit of this field, declared rather than implied.
    ///
    /// A field with no declared unit is a number with no meaning, and two fields
    /// of different units compared as though they matched is the axis-interchange
    /// error this project has a gate for.
    pub const fn unit(self) -> &'static str {
        match self {
            FieldKind::Lithology => "class",
            FieldKind::Elevation => "m",
            FieldKind::SoilDepth => "m",
        }
    }
}

/// How a lattice is interpolated between its samples.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Interpolation {
    /// Nearest sample. Blocky, and correct for a *class* field.
    Nearest,
    /// Trilinear. The default for continuous fields.
    Linear,
    /// Cubic. Smoother, and more expensive; used where a ridge must not facet.
    Cubic,
}

impl Interpolation {
    pub const fn name(self) -> &'static str {
        match self {
            Interpolation::Nearest => "nearest",
            Interpolation::Linear => "linear",
            Interpolation::Cubic => "cubic",
        }
    }
}

/// A declared field: what it is, how it is sampled, and everything that can
/// change its values.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldSpec {
    pub kind: FieldKind,
    pub levels: u32,
    pub interpolation: Interpolation,
    /// The provenance: where this field's values come from.
    pub provenance: String,
    /// The declared tolerance, as a numerator/denominator pair kept exact.
    ///
    /// `None` means the field makes **no tolerance claim**, which is different
    /// from a tolerance of zero and must not be rendered as one.
    pub tolerance: Option<(u64, u64)>,
    /// Value-independent metadata — a display label. Split out deliberately: it
    /// is the one thing that may change without invalidating a rebuild.
    pub label: String,
}

impl FieldSpec {
    /// A sensible default spec for a kind.
    pub fn standard(kind: FieldKind, provenance: impl Into<String>) -> Self {
        FieldSpec {
            kind,
            levels: STANDARD_LEVELS,
            interpolation: match kind {
                // A class field interpolated linearly produces a rock that is
                // 40% granite and 60% shale, which is not a rock.
                FieldKind::Lithology => Interpolation::Nearest,
                _ => Interpolation::Linear,
            },
            provenance: provenance.into(),
            tolerance: None,
            label: kind.name().to_string(),
        }
    }

    /// The field's identity: **everything that can change a value**.
    ///
    /// `label` is deliberately excluded, and so is nothing else. The exclusion is
    /// safe only because a label is provably value-independent — it is never
    /// read by an interpolation. If that ever stops being true, this function is
    /// the one place that has to change.
    pub fn identity(&self) -> Identity {
        // One byte of tag plus the two components, so that `None`, `0/1` and a
        // real tolerance are three different byte strings rather than a value
        // whose `None` happens to sit at the same place as some `Some`.
        let mut tolerance = [0u8; 17];
        match self.tolerance {
            // `None` and `0/1` are different declarations and must not hash the
            // same: one makes no claim, the other claims exactness about an
            // inexact value.
            None => {}
            Some((0, _)) => tolerance[0] = 1,
            Some((n, d)) => {
                tolerance[0] = 2;
                tolerance[1..9].copy_from_slice(&n.into_le_bytes());
                tolerance[9..17].copy_from_slice(&d.into_le_bytes());
            }
        }
        Identity::of_fields(
            "regional-field/v1",
            &[
                self.kind.name().as_bytes(),
                &self.levels.into_le_bytes(),
                self.interpolation.name().as_bytes(),
                self.provenance.as_bytes(),
                &tolerance,
            ],
        )
    }

    /// True when a change from `self` to `other` **invalidates** anything derived
    /// from `self` and therefore refuses rather than re-deriving.
    pub fn rebind_required(&self, other: &FieldSpec) -> bool {
        self.identity() != other.identity()
    }

    /// Validate the declared spec.
    pub fn validate(&self) -> Result<(), Refusal> {
        if self.levels == 0 {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "a field ladder with no levels has no samples",
            ));
        }
        if self.levels > 40 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("{} levels does not fit a 64-bit block count", self.levels),
            ));
        }
        if self.provenance.trim().is_empty() {
            return Err(Refusal::new(
                RefusalKind::MissingStage,
                "a field with no declared provenance cannot be rebuilt",
            ));
        }
        if let Some((_, 0)) = self.tolerance {
            return Err(Refusal::new(
                RefusalKind::RatiosDoNotSumToOne,
                "a declared tolerance with denominator 0 is not a tolerance",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_ladder_reaches_a_world_and_its_arithmetic_is_pinned() {
        assert_eq!(STANDARD_LEVELS, 20);
        // The measurement that superseded 16 levels.
        assert_eq!(blocks_at_level(15), 1_048_576);
        assert_eq!(blocks_at_level(19), 16_777_216);

        // **A correction, recorded rather than inherited.** The record that this
        // ladder comes from justifies 20 levels with *"32 · 524288 = 16,777,216
        // blocks, about 16.8 km"*, and calls level 15's 1,048,576 blocks *"a
        // 1.05 km top level"*. Both are **off by a factor of 1000**: at 1 m per
        // block, 1,048,576 m is 1,048 km and 16,777,216 m is 16,777 km. The
        // *ruling* (20 levels) stands — a world-scale view has to exist — and it
        // is the stated reason that was wrong. See `docs/LEDGER.md` C-10.
        let km = blocks_at_level(STANDARD_LEVELS - 1) as f64 / 1000.0;
        assert!((16_700.0..16_800.0).contains(&km), "top level was {km} km");
        assert!(
            (km - 16.8).abs() > 1000.0,
            "the record's 16.8 km figure reproduced itself; the arithmetic changed"
        );
    }

    #[test]
    fn every_kind_is_carried() {
        // F4: the ladder carries geology only. Climate is a process, and if it
        // ever appears here it must do so deliberately.
        assert_eq!(FieldKind::ALL.len(), 3);
        for k in FieldKind::ALL {
            assert!(!k.name().is_empty());
            assert!(!k.unit().is_empty(), "{k:?} has no declared unit");
        }
        // And every one of them is distinct: a list that repeats a kind would
        // pass the loop above while carrying only two fields.
        let mut names: Vec<&str> = FieldKind::ALL.iter().map(|k| k.name()).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before);
    }

    #[test]
    fn a_level_outside_the_ladder_is_refused() {
        assert_eq!(Level::new(19, 20).unwrap().index(), 19);
        let r = Level::new(20, 20).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::IndexOutOfRange);
    }

    #[test]
    fn lattice_coordinates_are_euclidean_across_the_origin() {
        let l = Level::new(0, 20).unwrap();
        assert_eq!(l.blocks(), 32);
        // -1 belongs to lattice cell -1, not cell 0.
        assert_eq!(l.lattice_of(-1, -1), (-1, -1));
        assert_eq!(l.lattice_of(0, 0), (0, 0));
        assert_eq!(l.lattice_of(31, 31), (0, 0));
        assert_eq!(l.lattice_of(32, 32), (1, 1));
        // Round trip.
        for (i, j) in [(-3i64, 5i64), (0, 0), (7, -2)] {
            let (x, z) = l.world_of(i, j);
            assert_eq!(l.lattice_of(x as i32, z as i32), (i, j));
        }
    }

    #[test]
    fn identity_binds_every_value_affecting_field_and_not_the_label() {
        let base = FieldSpec::standard(FieldKind::Elevation, "terrain/v1");

        let mut relabelled = base.clone();
        relabelled.label = "Height above datum".into();
        assert!(!base.rebind_required(&relabelled), "a label must not force a rebuild");

        let mut fewer_levels = base.clone();
        fewer_levels.levels = 16;
        assert!(base.rebind_required(&fewer_levels));

        let mut other_interp = base.clone();
        other_interp.interpolation = Interpolation::Cubic;
        assert!(base.rebind_required(&other_interp));

        let mut other_provenance = base.clone();
        other_provenance.provenance = "terrain/v2".into();
        assert!(base.rebind_required(&other_provenance));
    }

    #[test]
    fn no_tolerance_and_a_zero_tolerance_are_different_declarations() {
        let none = FieldSpec::standard(FieldKind::Elevation, "p");
        let mut zero = none.clone();
        zero.tolerance = Some((0, 1));
        assert!(none.rebind_required(&zero), "None and 0/1 hashed the same");

        let mut tiny = none.clone();
        tiny.tolerance = Some((1, 1000));
        assert!(none.rebind_required(&tiny));
    }

    #[test]
    fn a_field_must_declare_its_provenance_and_its_units() {
        assert!(FieldSpec::standard(FieldKind::SoilDepth, "p").validate().is_ok());

        let mut anonymous = FieldSpec::standard(FieldKind::Lithology, "   ");
        assert_eq!(anonymous.validate().unwrap_err().kind(), RefusalKind::MissingStage);

        anonymous.provenance = "p".into();
        anonymous.tolerance = Some((1, 0));
        assert!(anonymous.validate().is_err());
    }

    #[test]
    fn a_class_field_is_not_interpolated_into_a_blend() {
        // Nearest for lithology, and this is the reason: linear interpolation of
        // a class field invents a material that is 40% granite.
        let spec = FieldSpec::standard(FieldKind::Lithology, "geology/v1");
        assert_eq!(spec.interpolation, Interpolation::Nearest);
        let elev = FieldSpec::standard(FieldKind::Elevation, "terrain/v1");
        assert_eq!(elev.interpolation, Interpolation::Linear);
    }
}

