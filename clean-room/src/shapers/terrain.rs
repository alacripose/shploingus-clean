//! **The terrain group — `tectonics`, `fields`, `geology`, `terrain`.**
//!
//! Derived from `docs/SHAPERS.md` §1 (the shaper table: each stage and the **one** paper it is built
//! from) and §2 (the determinism contract). The stage contract is
//! `specs/003-u3-shapers/contracts/u3-seams.md` C1; the roster that wires these bodies is
//! [`crate::process::roster`].
//!
//! | Stage | Paper (or ruling) | Produces |
//! |---|---|---|
//! | `tectonics` | Kreischer, Bommes et al., *Procedural Tectonic Planets*, CGF 2019; Wilson-cycle plate-boundary taxonomy | `uplift`, `plate_id` |
//! | `fields` | Shepard, *A Two-Dimensional Interpolation Function for Irregularly-Spaced Data*, ACM 1968; the F4 20-level ladder ruling | `lithology` |
//! | `geology` | Allen & Allen, *Basin Analysis*, Wiley 2013 | `stratigraphy` |
//! | `terrain` | Mandelbrot 1982 (fractal landscapes); Musgrave, Kolb & Mace 1989 (eroded fractal terrains) | `elevation` |

use crate::coordinates::LADDER_LEVELS;
use crate::refusal::{Refusal, RefusalKind};
use crate::stage::{FieldId, Produced, StageContext};

/// The sub-stream tag each stage's noise is drawn from.
const SALT_TECTONICS: u64 = 0x01;
const SALT_FIELDS: u64 = 0x02;
const SALT_GEOLOGY: u64 = 0x03;
const SALT_TERRAIN: u64 = 0x04;

fn out_of_range(what: &str, value: f64) -> Refusal {
    Refusal::new(RefusalKind::ParameterOutOfRange, format!("{what} = {value}"))
}

/// **Shepard's inverse-distance weights** (Shepard 1968): `w_i = |d_i|^-p`, normalised so the
/// weights sum to one. An exact hit on a control point takes all the weight — not "nearly all",
/// which is what a finite-distance fallback would give.
///
/// This is the one interpolation in the crate: `fields` uses it for the regional lithology ladder
/// and `transition` uses it to blend two domains, so the partition-of-unity property is stated here
/// once rather than re-derived by each caller.
pub fn shepard_weights(points: &[(f64, f64)], at: (f64, f64), power: f64) -> Vec<f64> {
    // A non-positive or non-finite power has no weights to give, and a `NaN` power is not a
    // request for a distance law: both are an empty answer rather than a panic.
    if points.is_empty() || power <= 0.0 || !power.is_finite() {
        return Vec::new();
    }
    let mut weights = vec![0.0; points.len()];
    let mut total = 0.0;
    for (index, (px, py)) in points.iter().enumerate() {
        let squared = (px - at.0) * (px - at.0) + (py - at.1) * (py - at.1);
        if squared == 0.0 {
            let mut exact = vec![0.0; points.len()];
            exact[index] = 1.0;
            return exact;
        }
        let weight = squared.powf(-power / 2.0);
        weights[index] = weight;
        total += weight;
    }
    if total > 0.0 {
        for weight in &mut weights {
            *weight /= total;
        }
    }
    weights
}

/// **`tectonics`** (Kreischer, Bommes et al. 2019): a Voronoi plate partition plus a per-plate
/// signed bias and a boundary term. The cellular field's cells *are* the plates, so a plate is a
/// named region rather than a threshold on a noise field.
pub fn tectonics(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    let scale = ctx.dial("plate_scale_cells")?;
    if scale <= 0.0 {
        return Err(out_of_range("plate_scale_cells", scale));
    }
    let boundary_uplift = ctx.dial("boundary_uplift_m")?;
    let plate_bias = ctx.dial("plate_bias_m")?;
    let field = ctx.noise(SALT_TECTONICS);

    let mut uplift = Vec::with_capacity(ctx.len());
    let mut plates = Vec::with_capacity(ctx.len());
    for (_index, at) in ctx.columns() {
        let cellular = field.cellular(at.x as f64 / scale, at.y as f64 / scale);
        let hash = field.hash_at(cellular.cell.0, cellular.cell.1);
        // The plate's own bias: a signed value in [-1, 1) from the plate's identity.
        let bias = (hash >> 11) as f64 * (2.0 / 9_007_199_254_740_992.0) - 1.0;
        // Closeness to a boundary: two feature points at one distance *is* a boundary, so the gap
        // between the nearest and second-nearest shrinks toward it.
        let closeness = (1.0 - (cellular.f2 - cellular.f1)).clamp(0.0, 1.0);
        uplift.push(plate_bias * bias + boundary_uplift * closeness);
        // A plate's stable number, from its lattice cell — not from the sample's position.
        let number = (i128::from(cellular.cell.0) * 73_856_093 + i128::from(cellular.cell.1) * 19_349_663)
            .rem_euclid(1_000_003);
        plates.push(number as f64);
    }

    let mut produced = Produced::one(FieldId::Uplift, uplift);
    produced.push(FieldId::PlateId, plates);
    Ok(produced)
}

/// **`fields`** (Shepard 1968 over the F4 ladder): the regional lithology ladder. Each ladder cell
/// corner carries a class derived from the world's identity, and a column's lithology is the
/// Shepard interpolation of its cell's four corners — so the field is **identity-bound by effect**
/// and multi-resolution by construction: a coarser level is a coarser interpolation of the same
/// declaration, not a second field.
pub fn fields(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    let level = ctx.dial("ladder_level")?;
    if !(0.0..f64::from(LADDER_LEVELS)).contains(&level) {
        return Err(Refusal::new(
            RefusalKind::LadderOutOfRange,
            format!("ladder_level {level} is outside 0..{LADDER_LEVELS}"),
        ));
    }
    let classes = ctx.dial("lithology_classes")?;
    if classes < 1.0 {
        return Err(out_of_range("lithology_classes", classes));
    }
    let power = ctx.dial("shepard_power")?;
    if power <= 0.0 {
        return Err(out_of_range("shepard_power", power));
    }
    // The declared ladder: level `L` spans `32 · 2^L` blocks (`docs/DESIGN-SPEC.md` §1.2).
    let cell = 32.0 * 2f64.powf(level);
    let field = ctx.noise(SALT_FIELDS);
    let class_count = classes.round() as u64;

    let mut lithology = Vec::with_capacity(ctx.len());
    for (_index, at) in ctx.columns() {
        let (gx, gy) = (at.x as f64 / cell, at.y as f64 / cell);
        let (cx, cy) = (gx.floor(), gy.floor());
        let corners = [(cx, cy), (cx + 1.0, cy), (cx, cy + 1.0), (cx + 1.0, cy + 1.0)];
        let values: Vec<f64> = corners
            .iter()
            .map(|(px, py)| (field.hash_at(*px as i64, *py as i64) % class_count) as f64)
            .collect();
        let weights = shepard_weights(&corners, (gx, gy), power);
        let blended: f64 = weights.iter().zip(&values).map(|(w, v)| w * v).sum();
        lithology.push(blended);
    }
    Ok(Produced::one(FieldId::Lithology, lithology))
}

/// **`geology`** (Allen & Allen 2013): the exposed layer of a per-column stratigraphic stack, warped
/// by `tectonics`' uplift and by its own fBm relief.
///
/// A column that exposes a layer **outside the declared stack** is refused by name: the layer's
/// content is owed, and a column that silently wrapped around to the stack's bottom would be a
/// plausible substitute for a missing record — the one thing P2 forbids.
pub fn geology(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    let thickness = ctx.dial("layer_thickness_m")?;
    if thickness <= 0.0 {
        return Err(out_of_range("layer_thickness_m", thickness));
    }
    let layers = ctx.dial("layer_count")?;
    if layers < 1.0 {
        return Err(out_of_range("layer_count", layers));
    }
    let datum = ctx.dial("datum_m")?;
    let warp_gain = ctx.dial("deposit_warp_m")?;
    let uplift = ctx.read(FieldId::Uplift)?;
    let field = ctx.noise(SALT_GEOLOGY);

    let mut stack = Vec::with_capacity(ctx.len());
    for (index, at) in ctx.columns() {
        let warp = field.fbm(at.x as f64 * 0.01, at.y as f64 * 0.01, 4, 2.0, 0.5) * warp_gain;
        let depth = datum + uplift[index] + warp;
        let layer = (depth / thickness).floor();
        if layer < 0.0 || layer >= layers {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "the column at ({}, {}) exposes layer {layer}, outside the declared stack of {layers} layers",
                    at.x, at.y
                ),
            ));
        }
        stack.push(layer);
    }
    Ok(Produced::one(FieldId::Stratigraphy, stack))
}

/// **`terrain`** (Mandelbrot 1982; Musgrave, Kolb & Mace 1989): elevation = base + fBm relief +
/// tectonic uplift − smoothing.
///
/// The uplift term is added, not mixed: a tectonic claim and a relief claim are different claims, and
/// a test can therefore hold one still and move the other.
pub fn terrain(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    let base = ctx.dial("base_elevation_m")?;
    let amplitude = ctx.dial("relief_amplitude_m")?;
    let smoothing = ctx.dial("glacial_smoothing_m")?;
    let octaves = ctx.dial("octaves")?;
    if octaves < 1.0 || octaves > 16.0 {
        return Err(out_of_range("octaves", octaves));
    }
    let lacunarity = ctx.dial("lacunarity")?;
    if lacunarity <= 1.0 {
        return Err(out_of_range("lacunarity", lacunarity));
    }
    let gain = ctx.dial("gain")?;
    if !(0.0..1.0).contains(&gain) {
        return Err(out_of_range("gain", gain));
    }
    let uplift = ctx.read(FieldId::Uplift)?;
    let field = ctx.noise(SALT_TERRAIN);
    let octaves = octaves.round() as u32;

    let mut elevation = Vec::with_capacity(ctx.len());
    for (index, at) in ctx.columns() {
        // Domain warping (Perlin 1985 §; Quilez 2002): the relief is sampled at a position the field
        // has itself moved, so ridges bend instead of aligning with the lattice.
        let (wx, wy) = field.warp(at.x as f64 * 0.02, at.y as f64 * 0.02, 1.5);
        let relief = field.fbm(wx, wy, octaves, lacunarity, gain);
        let ridge = field.ridged(wx, wy, octaves, lacunarity, gain);
        elevation.push(base + uplift[index] + relief * amplitude - ridge * smoothing);
    }
    Ok(Produced::one(FieldId::Elevation, elevation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::{Dials, FieldValues, Patch, StageContext};
    use crate::world::{DeclaredPeriod, Tick, WorldIdentity};

    use crate::coordinates::VoxelPos;

    fn identity() -> WorldIdentity {
        WorldIdentity::new(0x5EED_2468, DeclaredPeriod::seconds(1).expect("a one-second period"))
    }

    fn context<'a>(
        patch: Patch,
        ordinal: u32,
        stage: &'static str,
        dials: &'a Dials,
        values: &'a FieldValues,
    ) -> StageContext<'a> {
        StageContext {
            patch,
            identity: identity(),
            tick: Tick(0),
            ordinal,
            stage,
            dials,
            values,
        }
    }

    fn patch(side: u32) -> Patch {
        Patch::new(VoxelPos::new(0, 0, 0), side).expect("a patch")
    }

    #[test]
    fn the_shepard_weights_are_a_partition_of_unity() {
        let points = [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0)];
        for at in [(2.0, 3.0), (5.0, 5.0), (-4.0, 7.5), (10.0, 10.0)] {
            let weights = shepard_weights(&points, at, 2.0);
            let total: f64 = weights.iter().sum();
            assert!((total - 1.0).abs() < 1e-12, "weights must sum to one at {at:?}: {weights:?}");
            // A nearest control point carries the most weight. At a point equidistant from two
            // control points the weights tie, so the claim is about the *set* of nearest points and
            // not about which of two equal entries a sort happens to put first.
            let distance = |point: &(f64, f64)| (point.0 - at.0).hypot(point.1 - at.1);
            let nearest = points.iter().map(distance).fold(f64::INFINITY, f64::min);
            let tied: Vec<usize> = points
                .iter()
                .enumerate()
                .filter(|(_, point)| (distance(point) - nearest).abs() < 1e-12)
                .map(|(index, _)| index)
                .collect();
            let heaviest = weights
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(index, _)| index)
                .expect("four points");
            assert!(
                tied.contains(&heaviest),
                "the heaviest weight at {at:?} is {heaviest}, not one of the nearest {tied:?}"
            );
            for (index, weight) in weights.iter().enumerate() {
                if !tied.contains(&index) {
                    assert!(*weight <= weights[heaviest], "a farther point outweighs a nearer one");
                }
            }
        }
        // An exact hit takes all the weight.
        let exact = shepard_weights(&points, (0.0, 0.0), 2.0);
        assert_eq!(exact, vec![1.0, 0.0, 0.0, 0.0]);
        // A degenerate request is empty rather than a panic.
        assert!(shepard_weights(&[], (0.0, 0.0), 2.0).is_empty());
        assert!(shepard_weights(&points, (0.0, 0.0), 0.0).is_empty());
    }

    #[test]
    fn tectonics_is_a_pure_function_of_the_world_and_names_its_plates() {
        let dials = Dials::new(&[
            ("plate_scale_cells", 64.0),
            ("boundary_uplift_m", 120.0),
            ("plate_bias_m", 40.0),
        ]);
        let values = FieldValues::new(patch(6));
        let run = || {
            let ctx = context(patch(6), 1, "tectonics", &dials, &values);
            tectonics(&ctx).expect("the reference dials are valid")
        };
        let first = run();
        let second = run();
        assert_eq!(first, second, "the stage is a function of (seed, tick, ordinal)");

        let uplift = first.field(FieldId::Uplift).expect("uplift");
        let plates = first.field(FieldId::PlateId).expect("plate_id");
        assert_eq!(uplift.len(), 36);
        assert_eq!(plates.len(), 36);
        // Every uplift is inside the declared bounds, and the plates are integers in range.
        for value in uplift {
            assert!(
                value.abs() <= 160.0 + 1e-9,
                "uplift must stay within |bias| + boundary: {value}"
            );
        }
        for plate in plates {
            assert_eq!(plate.fract(), 0.0, "a plate number is an identity, not a blend");
            assert!((0.0..1_000_003.0).contains(plate));
        }
        // A boundary term is really present: two columns of one patch do not all agree.
        assert!(uplift.iter().any(|a| uplift.iter().any(|b| (a - b).abs() > 1e-6)));
    }

    #[test]
    fn the_field_ladder_is_multi_resolution_and_carries_its_class_range() {
        let dials = Dials::new(&[
            ("ladder_level", 3.0),
            ("lithology_classes", 6.0),
            ("shepard_power", 2.0),
        ]);
        let values = FieldValues::new(patch(4));
        let ctx = context(patch(4), 2, "fields", &dials, &values);
        let lithology = fields(&ctx).expect("the reference dials are valid");
        let classes = lithology.field(FieldId::Lithology).expect("lithology");
        assert_eq!(classes.len(), 16);
        // An interpolation of class indices is bounded by the declared classes.
        for class in classes {
            assert!((0.0..6.0).contains(class), "class out of range: {class}");
        }
        assert!(
            classes.iter().any(|c| c.fract() != 0.0),
            "a Shepard blend of integer classes is an interpolation, not a lookup"
        );

        // A level past the ladder is refused by name. `20.0` is the ladder's own count: the dial is
        // a declared number, and the ladder is closed at 20 levels (`LADDER_LEVELS`).
        const PAST_THE_LADDER: Dials = Dials::new(&[
            ("ladder_level", LADDER_LEVELS as f64),
            ("lithology_classes", 6.0),
            ("shepard_power", 2.0),
        ]);
        let past = PAST_THE_LADDER;
        let ctx = context(patch(2), 2, "fields", &past, &values);
        let refused = fields(&ctx)
            .expect_err("a level past the ladder must be refused");
        assert_eq!(refused.kind(), RefusalKind::LadderOutOfRange);
    }

    #[test]
    fn geology_keeps_what_tectonics_decided_and_refuses_a_layer_it_cannot_name() {
        let dials = Dials::new(&[
            ("layer_thickness_m", 20.0),
            ("layer_count", 24.0),
            ("datum_m", 200.0),
            ("deposit_warp_m", 12.0),
        ]);
        let patch = patch(4);
        let mut values = FieldValues::new(patch);
        values.insert(FieldId::Uplift, vec![0.0; 16]).expect("uplift");

        let flat = geology(&context(patch, 3, "geology", &dials, &values)).expect("inside the stack");
        let mut raised = FieldValues::new(patch);
        raised.insert(FieldId::Uplift, vec![40.0; 16]).expect("uplift");
        let lifted = geology(&context(patch, 3, "geology", &dials, &raised)).expect("inside the stack");

        // Tectonics' decision is **preserved**: more uplift cannot expose a lower layer.
        let flat_layers = flat.field(FieldId::Stratigraphy).expect("stratigraphy");
        let lifted_layers = lifted.field(FieldId::Stratigraphy).expect("stratigraphy");
        for (low, high) in flat_layers.iter().zip(lifted_layers) {
            assert!(high >= low, "40 m more uplift exposed layer {high} under layer {low}");
            assert!((high - low - 2.0).abs() < 1e-9, "40 m at 20 m/layer is exactly two layers");
        }

        // A declared stack too thin to hold the column is refused, naming the layer.
        let thin = Dials::new(&[
            ("layer_thickness_m", 20.0),
            ("layer_count", 1.0),
            ("datum_m", 200.0),
            ("deposit_warp_m", 12.0),
        ]);
        let refused = geology(&context(patch, 3, "geology", &thin, &values))
            .expect_err("a layer outside the stack must be refused");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(refused.detail().contains("outside the declared stack"));
    }

    #[test]
    fn terrain_adds_the_uptake_it_is_given_and_keeps_its_own_relief_distinct() {
        let dials = Dials::new(&[
            ("base_elevation_m", 64.0),
            ("relief_amplitude_m", 120.0),
            ("glacial_smoothing_m", 30.0),
            ("octaves", 6.0),
            ("lacunarity", 2.0),
            ("gain", 0.5),
        ]);
        let patch = patch(5);
        let mut low = FieldValues::new(patch);
        low.insert(FieldId::Uplift, vec![10.0; 25]).expect("uplift");
        let mut high = FieldValues::new(patch);
        high.insert(FieldId::Uplift, vec![60.0; 25]).expect("uplift");

        let low = terrain(&context(patch, 4, "terrain", &dials, &low)).expect("relief");
        let high = terrain(&context(patch, 4, "terrain", &dials, &high)).expect("relief");
        let low_elevation = low.field(FieldId::Elevation).expect("elevation");
        let high_elevation = high.field(FieldId::Elevation).expect("elevation");

        for (a, b) in low_elevation.iter().zip(high_elevation) {
            assert!((b - a - 50.0).abs() < 1e-12, "50 m more uplift must be 50 m more elevation");
        }
        // The relief is really applied: a patch is not a plane, and it is not constant.
        let spread = low_elevation.iter().cloned().fold(f64::MIN, f64::max)
            - low_elevation.iter().cloned().fold(f64::MAX, f64::min);
        assert!(spread > 1.0, "a relief of 120 m must move the surface: spread {spread}");
        assert!(spread < 300.0, "and must stay inside the declared amplitude: spread {spread}");

        // Octaves are a declared positive count with a declared ceiling.
        let bad = Dials::new(&[
            ("base_elevation_m", 64.0),
            ("relief_amplitude_m", 120.0),
            ("glacial_smoothing_m", 30.0),
            ("octaves", 0.0),
            ("lacunarity", 2.0),
            ("gain", 0.5),
        ]);
        let values = FieldValues::new(patch);
        let refused = terrain(&context(patch, 4, "terrain", &bad, &values))
            .expect_err("zero octaves is not a fractal");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
    }

    #[test]
    fn a_stage_missing_an_input_it_reads_refuses_by_name() {
        let dials = Dials::new(&[
            ("layer_thickness_m", 20.0),
            ("layer_count", 24.0),
            ("datum_m", 200.0),
            ("deposit_warp_m", 12.0),
        ]);
        let empty = FieldValues::new(patch(2));
        let refused = geology(&context(patch(2), 3, "geology", &dials, &empty))
            .expect_err("geology without uplift must refuse");
        assert_eq!(refused.kind(), RefusalKind::StageInputAbsent);
        assert!(refused.detail().contains("uplift"), "{refused}");
    }
}
