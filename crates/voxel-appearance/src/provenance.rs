//! Where every bare number came from.
//!
//! `APPEARANCE.md` §3 states the **four-bucket rule**: every geometry constant is
//! labelled *MD1 rule / MD1 example / platform translation / project extension*,
//! and **"never turn a missing value into an invented constant"**. A shade
//! constant deserves exactly the same treatment as a geometry constant, and the
//! rule is easier to keep as a **type** than as a promise — a comment does not
//! fail a build.
//!
//! So the definitions live here, and [`CONSTANTS`] is their inventory. The point
//! of the inventory is the last test in this module: the number of
//! [`Provenance::ProjectExtension`] entries is **asserted**. A constant that
//! nobody can source has to be added to the count *in a test*, which is the
//! smallest possible thing that makes the debt visible instead of silent.

/// Why a numeric constant has the value it has.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Provenance {
    /// A range or a semantic the OpenPBR specification itself fixes, rather than
    /// a value this crate chose. Clamping into `[0, 1]` is this bucket: the bound
    /// is the parameter's declared domain, not an opinion.
    OpenPbrSpec,
    /// A constant as a cited paper publishes it.
    Paper(&'static str),
    /// **A project extension.** No source in the record. Every one of these is a
    /// named debt, and the count is pinned by a test in this module.
    ProjectExtension,
}

/// Schlick's approximation exponent (Schlick 1994). Used for the fresnel-shaped
/// rim term and for the fluid surface's reflectance, which is the same
/// approximation at the same exponent applied to a different boundary.
pub const RIM_SCHLICK_EXPONENT: i32 = 5;

/// The refractive index of liquid water in the visible band, `n = 1.333`.
///
/// The standard figure at 20 °C and the sodium D line (IAPWS's formulation of
/// the refractive index of water; the value is quoted to three places in every
/// optics table). It is the input [`WATER_F0`] is derived from, so the two can
/// never disagree.
pub const WATER_IOR: f64 = 1.333;

/// The refractive index of a **clear coat** layer, `n = 1.5`.
///
/// OpenPBR's coat is specified as a smooth dielectric layer with this index, so the coat's
/// normal-incidence reflectance is [`COAT_F0`] by the same relation as any other boundary.
/// It is published, not a project extension, and the coat reads *no* other number.
pub const COAT_IOR: f64 = 1.5;

/// **The normal-incidence reflectance of a material's own specular boundary**, from its
/// `specular_ior`.
///
/// This is what gives `specular_ior` a home: it is the base layer's reflectance, where
/// `COAT_F0` is the coat's. The warp's *step* is untouched — it still fires at full
/// `specular_weight` — so the stylisation survives, and what changes is how much the
/// material reflects at all, which is the physically meaningful difference between a
/// mineral at `n = 1.6` and one at `n = 1.9`.
pub const fn specular_f0(ior: f64) -> f64 {
    normal_incidence_reflectance(ior)
}

/// The normal-incidence reflectance of a boundary, `((n − 1)/(n + 1))²`.
///
/// **The relation, named once**, so a material's `specular_ior` becomes a reflectance
/// by arithmetic instead of by a second quoted number.
pub const fn normal_incidence_reflectance(n: f64) -> f64 {
    let ratio = (n - 1.0) / (n + 1.0);
    ratio * ratio
}

/// The normal-incidence reflectance of a water surface, `((n − 1)/(n + 1))²`.
///
/// **Derived from [`WATER_IOR`] rather than quoted**: a water/air boundary
/// reflects `2.0 %` of the light hitting it head-on, and the Fresnel formula is
/// what turns the refractive index into that figure. Quoting both would let a
/// later edit change one and not the other.
pub const WATER_F0: f64 = normal_incidence_reflectance(WATER_IOR);

/// The normal-incidence reflectance of a clear coat, from [`COAT_IOR`]: about `4 %`.
pub const COAT_F0: f64 = normal_incidence_reflectance(COAT_IOR);

/// The luminance weights of **Rec. 709** (ITU-R BT.709). Named, because
/// [`.2126 .7152 .0722`](LUMINANCE_RED) is a standard and not a preference —
/// a different triple would give a different band count for the same picture.
pub const LUMINANCE_RED: f64 = 0.2126;
/// See [`LUMINANCE_RED`].
pub const LUMINANCE_GREEN: f64 = 0.7152;
/// See [`LUMINANCE_RED`].
pub const LUMINANCE_BLUE: f64 = 0.0722;

/// How many uniform luminance bands a measurement resolves a surface into.
///
/// **This is a measurement resolution, not a look.** The FMA bar states
/// *"≥3 distinct luminance bands"* and states no band width, so the width has to
/// come from somewhere: it is derived from the observed range, and this constant
/// is only the number of divisions. Eight is a resolution (it can express the
/// bar's three with room to spare); it is a project extension because the bar
/// does not name it, and it is declared here rather than buried at the one call
/// site that needs it.
pub const LUMINANCE_BAND_RESOLUTION: usize = 8;

/// The domain tag the detail lattice mixes into its seed. A tag, not a value
/// about the world: its only job is that two different lattices cannot collide,
/// the same reason [`voxel_model::Identity::of_fields`] length-prefixes.
pub const LATTICE_DOMAIN_TAG: u64 = 0xA5A5_A5A5_0000_0001;

/// Every bare number this crate shades or measures with.
///
/// The definitions above are the single source; this is the inventory over them,
/// so the table and the constants cannot disagree about a value.
pub const CONSTANTS: &[(&str, f64, Provenance)] = &[
    ("rim_schlick_exponent", RIM_SCHLICK_EXPONENT as f64, Provenance::Paper("Schlick 1994")),
    ("luminance_red", LUMINANCE_RED, Provenance::Paper("ITU-R BT.709")),
    ("luminance_green", LUMINANCE_GREEN, Provenance::Paper("ITU-R BT.709")),
    ("luminance_blue", LUMINANCE_BLUE, Provenance::Paper("ITU-R BT.709")),
    (
        "luminance_band_resolution",
        LUMINANCE_BAND_RESOLUTION as f64,
        Provenance::ProjectExtension,
    ),
    (
        "water_ior",
        WATER_IOR,
        Provenance::Paper("IAPWS, Release on the Refractive Index of Ordinary Water Substance"),
    ),
    ("water_f0", WATER_F0, Provenance::Paper("Fresnel, 1823 — ((n−1)/(n+1))²")),
    (
        "coat_ior",
        COAT_IOR,
        Provenance::OpenPbrSpec,
    ),
    (
        "coat_f0",
        COAT_F0,
        Provenance::Paper("Fresnel, 1823 — ((n−1)/(n+1))²"),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_constant_is_named_once_and_sourced() {
        let mut names: Vec<&str> = CONSTANTS.iter().map(|(n, _, _)| *n).collect();
        names.sort_unstable();
        let unique = names.len();
        names.dedup();
        assert_eq!(names.len(), unique, "two constants share a name: {names:?}");
        for (name, _, provenance) in CONSTANTS {
            match provenance {
                // A paper citation with no paper in it is not a citation.
                Provenance::Paper(paper) => assert!(!paper.is_empty(), "{name} cites nothing"),
                Provenance::OpenPbrSpec | Provenance::ProjectExtension => {}
            }
        }
    }

    #[test]
    fn the_project_s_own_extensions_are_counted_here_and_nowhere_else() {
        // The four-bucket rule as an assertion. Adding an unsourced constant
        // fails *this test*, which is the point: the debt has to be counted.
        let unsourced: Vec<&str> = CONSTANTS
            .iter()
            .filter(|(_, _, p)| *p == Provenance::ProjectExtension)
            .map(|(n, _, _)| *n)
            .collect();
        assert_eq!(unsourced, vec!["luminance_band_resolution"]);
    }

    #[test]
    fn the_luminance_weights_are_the_rec_709_triple() {
        // They must sum to one: a triple that does not is not a luminance.
        let sum = LUMINANCE_RED + LUMINANCE_GREEN + LUMINANCE_BLUE;
        assert!((sum - 1.0).abs() < 1e-12, "weights sum to {sum}");
    }
}
