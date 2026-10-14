//! **The life group — `ecology`, and `caves` refusing by name.**
//!
//! Derived from `docs/SHAPERS.md` §1's `caves` and `ecology` rows, `docs/MISSING-SYSTEMS.md` §3 (the
//! karst field's absence) and §5 (the ecology capacities), and `docs/DESIGN-SPEC.md` §2 F4–F10.
//!
//! | Stage | Paper | Produces |
//! |---|---|---|
//! | `ecology` | Holdridge, *Determination of World Plant Formations from Simple Climatic Data*, Science 1947 (the humidity-province ladder); Lieth, *Modeling the primary productivity of the world*, in *Primary Productivity of the Biosphere*, 1975 (the Miami model — two single-factor curves, the minimum is the limiting one); Reynolds, *Flocks, herds and schools: a distributed behavioral model*, SIGGRAPH 1987 (boids) | `biome`, `carrying_capacity`, `flock` |
//! | `caves` | Ford & Williams, *Karst Hydrogeology and Geomorphology*, Wiley 2007; Nie et al., *Algorithms* 17(10):466, 2024 — **owed**: the occupancy field is not one of the declared fields | *refuses* |
//!
//! The three ecology outputs are one reading of one column set: the **province** the climate can
//! classify (Holdridge), the **production** that province can carry (Lieth, limited by the soil the
//! substrate produced), and the **flock** the province's agents form (Reynolds). Each is a declared
//! field with its own law, so a reader can tell which of the three a change moved.

use crate::coordinates::VoxelPos;
use crate::noise::Noise;
use crate::refusal::{Refusal, RefusalKind};
use crate::stage::{FieldId, Produced, StageContext};

fn out_of_range(what: &str, value: f64) -> Refusal {
    Refusal::new(RefusalKind::ParameterOutOfRange, format!("{what} = {value}"))
}

/// **`ecology`** (Holdridge 1947): the biome class of a column.
///
/// The class is the column's **humidity province** — the band its humidity ratio `r = P / PET` falls
/// in, with `PET = 58.93 · T_bio` the paper's own coefficient. Holdridge's province ladder is
/// **geometric in the ratio** (each province spans twice the one below it), so the band is
/// `log₂(r / driest_ratio)` and `driest_ratio` is the declared edge of the driest province the set
/// names — which is what makes the ladder the paper's rather than an offset somebody fitted.
///
/// Class `0` is reserved for **bare ground**: Heimsath et al.'s soil production function is why a
/// column whose soil cannot form does not carry a formation, and the threshold is a declared dial
/// rather than a slope quietly subtracted from the ratio. Soil is **not** folded into the ratio:
/// that would let a steep hillside refuse a *climate* it can classify perfectly well.
///
/// Lieth's Miami capacity (Lieth 1975) is named by the spec's roster cell and is **owed**: the
/// declared `biome` field is one class per column, and a capacity in g/m²/yr is a second quantity.
pub fn ecology(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    let floor_c = ctx.dial("biotemperature_floor_c")?;
    let bands = ctx.dial("biome_bands")?;
    if bands < 2.0 {
        return Err(out_of_range("biome_bands", bands));
    }
    let driest = ctx.dial("driest_ratio")?;
    if driest <= 0.0 {
        return Err(out_of_range("driest_ratio", driest));
    }
    let min_soil = ctx.dial("min_soil_depth_m")?;
    if min_soil < 0.0 {
        return Err(out_of_range("min_soil_depth_m", min_soil));
    }
    let temperature = ctx.read(FieldId::Temperature)?;
    let precipitation = ctx.read(FieldId::Precipitation)?;
    let soil = ctx.read(FieldId::SoilDepth)?;

    let mut biome = Vec::with_capacity(ctx.len());
    for index in 0..ctx.len() {
        // Biotemperature: below the declared floor there is no plant productivity to speak of, and
        // therefore no potential evapotranspiration to divide by. That is a refusal, not a zero.
        let biotemperature = (temperature[index] - floor_c).max(0.0);
        let potential = 58.93 * biotemperature;
        if potential <= 0.0 {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "no potential evapotranspiration at {:.3} °C (floor {floor_c} °C): a humidity ratio cannot be formed",
                    temperature[index]
                ),
            ));
        }
        // Bare ground: no soil, no formation — the barren class, not a wetter province and not a
        // clamp of the ratio.
        if soil[index] < min_soil {
            biome.push(BARE_CLASS);
            continue;
        }
        let humidity_ratio = precipitation[index] / potential;
        if humidity_ratio < driest {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "humidity ratio {humidity_ratio:.6} is drier than the declared driest province {driest}: the band is outside the declared set"
                ),
            ));
        }
        let class = (humidity_ratio / driest).log2().floor();
        if class >= bands - 1.0 {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "humidity ratio {humidity_ratio:.6} is wetter than the {bands} declared classes can name"
                ),
            ));
        }
        biome.push(1.0 + class);
    }

    // The other two readings follow from the same pass: the production the province can carry and
    // the flock its agents form. Both are declared outputs, so neither is a side effect.
    let half_saturation = ctx.dial("soil_half_saturation_m")?;
    let mut capacity = Vec::with_capacity(ctx.len());
    for index in 0..ctx.len() {
        let biotemperature = (temperature[index] - floor_c).max(0.0);
        capacity.push(carrying_capacity(
            biotemperature,
            precipitation[index],
            soil[index],
            half_saturation,
        )?);
    }

    let radius = ctx.dial("flock_radius_cells")?;
    let heading_scale = ctx.dial("heading_scale")?;
    let weights = (
        ctx.dial("separation_weight")?,
        ctx.dial("alignment_weight")?,
        ctx.dial("cohesion_weight")?,
    );
    let positions: Vec<(usize, VoxelPos)> = ctx.columns().collect();
    let flock = flock(
        &biome,
        &positions,
        ctx.noise(SALT_FLOCK),
        heading_scale,
        radius,
        weights,
    )?;

    let mut produced = Produced::one(FieldId::Biome, biome);
    produced.push(FieldId::CarryingCapacity, capacity);
    produced.push(FieldId::Flock, flock);
    Ok(produced)
}

/// The noise stream `ecology` draws its agent headings from. Its own stream, so a heading can never
/// be a second reading of the terrain's noise.
const SALT_FLOCK: u64 = 0x5A17_F10C;

/// **Lieth's Miami model** (Lieth 1975): the carrying capacity of a column as the **minimum** of two
/// single-factor curves, because the scarcer factor is the one that limits:
///
/// ```text
/// NPP_T = 3000 / (1 + exp(1.315 - 0.119·T))     g DM m⁻² yr⁻¹,  T = biotemperature °C
/// NPP_P = 3000 · (1 - exp(-0.000664·P))          P = annual precipitation, mm
/// NPP   = min(NPP_T, NPP_P)
/// ```
///
/// then **limited by the soil depth** the substrate produced (Heimsath et al. 1997): a thinner cover
/// carries less, saturating rather than clipping, so a deep soil stops being the limit instead of
/// making a desert wet. The `3000` asymptote, `0.119`, `1.315` and `0.000664` are the paper's
/// coefficients, and `soil_half_saturation_m` is the declared dial.
pub fn carrying_capacity(
    biotemperature_c: f64,
    precipitation_mm: f64,
    soil_m: f64,
    soil_half_saturation_m: f64,
) -> Result<f64, Refusal> {
    for (what, value) in [
        ("biotemperature_c", biotemperature_c),
        ("precipitation_mm", precipitation_mm),
        ("soil_m", soil_m),
    ] {
        if !value.is_finite() {
            return Err(out_of_range(what, value));
        }
    }
    if soil_m < 0.0 {
        return Err(out_of_range("soil_m", soil_m));
    }
    if soil_half_saturation_m <= 0.0 {
        return Err(out_of_range("soil_half_saturation_m", soil_half_saturation_m));
    }
    let by_temperature = MIAMI_ASYMPTOTE_G_M2_YR / (1.0 + (1.315 - 0.119 * biotemperature_c).exp());
    let by_precipitation =
        MIAMI_ASYMPTOTE_G_M2_YR * (1.0 - (-0.000_664 * precipitation_mm).exp());
    let climate_limited = by_temperature.min(by_precipitation);
    let soil_factor = soil_m / (soil_m + soil_half_saturation_m);
    Ok(climate_limited * soil_factor)
}

/// Lieth's declared upper asymptote: each single-factor curve approaches it, and neither passes it.
const MIAMI_ASYMPTOTE_G_M2_YR: f64 = 3000.0;

/// **Reynolds' boids** (Reynolds 1987) as a per-column reading.
///
/// Every **vegetated** column is an agent, and its heading is drawn from the step's own noise — never
/// a stored state, because a remembered heading would be a second simulation (`MISSING-SYSTEMS` §4.1).
/// For each agent the three rules are computed over the agents within `radius`: **separation** away
/// from crowding, **alignment** with the neighbours' headings, and **cohesion** toward their centre.
/// The field is the magnitude of their resultant, normalised by the declared weights, so it reads
/// `0` where the flock is at rest or absent and rises where the three pull together.
///
/// A column that carries no formation carries no agent: `bare` ground is `0`, which is how this
/// reading stays **coupled** to `ecology`'s own province rather than becoming a second map.
///
/// The `0` of a lone neighbour is deliberate: an agent with nothing to align with has no resultant,
/// and reporting a heading for it would invent a flock of one.
pub fn flock(
    biome: &[f64],
    positions: &[(usize, VoxelPos)],
    headings_from: Noise,
    heading_scale: f64,
    radius: f64,
    weights: (f64, f64, f64),
) -> Result<Vec<f64>, Refusal> {
    if biome.len() != positions.len() {
        return Err(out_of_range("flock agents", biome.len() as f64));
    }
    if !heading_scale.is_finite() || heading_scale <= 0.0 {
        return Err(out_of_range("heading_scale", heading_scale));
    }
    if !radius.is_finite() || radius <= 0.0 {
        return Err(out_of_range("flock radius", radius));
    }
    let (separation, alignment, cohesion) = weights;
    for weight in [separation, alignment, cohesion] {
        if !weight.is_finite() || weight < 0.0 {
            return Err(out_of_range("flock weight", weight));
        }
    }
    let total_weight = separation + alignment + cohesion;
    if total_weight <= 0.0 {
        return Err(out_of_range("flock weights sum", total_weight));
    }

    // The agents, and the heading each one flies on: two independent noise samples, so no
    // trigonometry is needed to turn a number into a direction (`noise`'s own rule).
    let agents: Vec<usize> = (0..biome.len()).filter(|i| biome[*i] >= 1.0).collect();
    let headings: Vec<(f64, f64)> = positions
        .iter()
        .map(|(_, at)| {
            let scale = heading_scale;
            let x = f64::from(at.x) * scale;
            let y = f64::from(at.y) * scale;
            let (hx, hy) = (
                headings_from.gradient(x, y),
                headings_from.gradient(x + 64.0, y + 64.0),
            );
            let length = (hx * hx + hy * hy).sqrt();
            if length == 0.0 {
                // A sample of exactly zero has no direction; flying due +x is the declared
                // convention for that one degenerate case, and it is still a pure function.
                (1.0, 0.0)
            } else {
                (hx / length, hy / length)
            }
        })
        .collect();

    let mut flock = vec![0.0; biome.len()];
    for &index in &agents {
        let at = positions[index].1;
        let mut neighbours = 0_usize;
        let (mut separation_x, mut separation_y) = (0.0, 0.0);
        let (mut alignment_x, mut alignment_y) = (0.0, 0.0);
        let (mut centre_x, mut centre_y) = (0.0, 0.0);
        for &other in &agents {
            if other == index {
                continue;
            }
            let there = positions[other].1;
            let (dx, dy) = (
                f64::from(at.x - there.x),
                f64::from(at.y - there.y),
            );
            let squared = dx * dx + dy * dy;
            if squared > radius * radius || squared == 0.0 {
                continue;
            }
            neighbours += 1;
            separation_x += dx / squared;
            separation_y += dy / squared;
            alignment_x += headings[other].0;
            alignment_y += headings[other].1;
            centre_x += f64::from(there.x);
            centre_y += f64::from(there.y);
        }
        if neighbours == 0 {
            continue; // nothing to flock with: the field stays at its declared zero
        }
        let count = neighbours as f64;
        // A rule that has no direction contributes nothing: there is no heading to report for a
        // flock that is not there.
        let scaled = |direction: (f64, f64), weight: f64| {
            if direction == (0.0, 0.0) {
                (0.0, 0.0)
            } else {
                (
                    direction.0 * weight / total_weight,
                    direction.1 * weight / total_weight,
                )
            }
        };
        let resultant = add(
            add(
                scaled(unit(separation_x, separation_y), weights.0),
                scaled(unit(alignment_x, alignment_y), weights.1),
            ),
            scaled(
                unit(centre_x / count - f64::from(at.x), centre_y / count - f64::from(at.y)),
                weights.2,
            ),
        );
        // The triangle inequality already bounds this by 1; the `min` is a floating-point guard, not
        // a clamp of an unrepresentable value (unit directions and normalised weights are in range).
        flock[index] = (resultant.0 * resultant.0 + resultant.1 * resultant.1)
            .sqrt()
            .min(1.0);
    }
    Ok(flock)
}

/// Pair addition, so a three-rule resultant reads as the sum it is.
fn add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

/// A vector's direction, or `(0, 0)` when it has none (an exactly-zero sum).
fn unit(x: f64, y: f64) -> (f64, f64) {
    let length = (x * x + y * y).sqrt();
    if length == 0.0 {
        (0.0, 0.0)
    } else {
        (x / length, y / length)
    }
}

/// The reserved **bare ground** class: no formation, because no soil to hold one. It sorts below
/// every vegetated class, so `biome` stays monotone in how much life a column can carry.
const BARE_CLASS: f64 = 0.0;

/// **`caves`** — refused by name, and the refusal names the missing field.
///
/// `docs/MISSING-SYSTEMS.md` §3 is exact about what is missing: the cave answer (open / flooded /
/// rock) is a closed vocabulary whose **occupancy field is not one of the declared fields**, and the
/// field — not the threshold — is the missing system. So the stage reads the flow `hydrology`
/// produces (proving the wiring is real) and then refuses for the input it does **not** have. It is
/// never quietly implemented as a threshold on the flow: that would be a plausible substitute for a
/// missing record, which is the one thing P2 forbids.
pub fn caves(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    let _flow = ctx.read(FieldId::Discharge)?;
    Err(Refusal::new(
        RefusalKind::BodyOwed,
        "cave occupancy needs a dissolution field (flow × CO₂ aggressiveness × residence time), \
         which is not one of the declared fields: the field is owed, not the threshold",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::VoxelPos;
    use crate::noise::Noise;
    use crate::stage::{Dials, FieldValues, Patch, StageContext};
    use crate::world::{DeclaredPeriod, Tick, WorldIdentity};

    /// A declared `const` so a context can borrow it: `caves` needs no dials to refuse.
    const NO_DIALS: Dials = Dials::new(&[]);

    fn patch(side: u32) -> Patch {
        Patch::new(VoxelPos::new(0, 0, 0), side).expect("a patch")
    }

    fn context<'a>(
        patch: Patch,
        stage: &'static str,
        dials: &'a Dials,
        values: &'a FieldValues,
    ) -> StageContext<'a> {
        StageContext {
            patch,
            identity: WorldIdentity::new(4, DeclaredPeriod::seconds(1).expect("a period")),
            tick: Tick(0),
            ordinal: 11,
            stage,
            dials,
            values,
        }
    }

    /// The declared dials the reference declaration uses, so the law is exercised at its own values.
    fn ecology_dials() -> Dials {
        Dials::new(&[
            ("biotemperature_floor_c", 0.0),
            ("biome_bands", 8.0),
            ("driest_ratio", 0.0625),
            ("min_soil_depth_m", 0.02),
            ("soil_half_saturation_m", 0.2),
            ("flock_radius_cells", 2.0),
            ("heading_scale", 0.25),
            ("separation_weight", 0.5),
            ("alignment_weight", 1.0),
            ("cohesion_weight", 0.5),
        ])
    }

    fn filled(temperature: f64, precipitation: f64, soil: f64) -> FieldValues {
        let patch = patch(2);
        let mut values = FieldValues::new(patch);
        values.insert(FieldId::Temperature, vec![temperature; 4]).expect("temperature");
        values.insert(FieldId::Precipitation, vec![precipitation; 4]).expect("precipitation");
        values.insert(FieldId::SoilDepth, vec![soil; 4]).expect("soil");
        values
    }

    fn band(rainfall_mm: f64) -> f64 {
        let dials = ecology_dials();
        let produced = ecology(&context(patch(2), "ecology", &dials, &filled(13.0, rainfall_mm, 1.2)))
            .expect("a class");
        produced.field(FieldId::Biome).expect("biome")[0]
    }

    #[test]
    fn a_wetter_climate_carries_a_wetter_class_and_the_class_never_leaves_its_declared_set() {
        let dry = band(60.0);
        let wetter = band(400.0);
        let drenched = band(2_000.0);
        assert!(wetter > dry, "more rain must move the class: {dry} -> {wetter}");
        assert!(drenched > wetter, "and keep moving it: {wetter} -> {drenched}");
        for class in [dry, wetter, drenched] {
            assert_eq!(class.fract(), 0.0, "a biome is a class, not a blend");
            assert!((1.0..8.0).contains(&class), "class {class} left the declared set");
        }
        // The ladder is geometric in the ratio, so one doubling of rain moves one class — the
        // paper's own shape, not an even split of the range.
        assert_eq!(band(120.0), dry + 1.0, "a doubling of rain is one province");
    }

    #[test]
    fn bare_ground_carries_no_formation() {
        let dials = ecology_dials();
        let patch = patch(2);
        let bare = ecology(&context(patch, "ecology", &dials, &filled(13.0, 400.0, 0.001)))
            .expect("a class");
        assert_eq!(
            bare.field(FieldId::Biome).expect("biome")[0],
            0.0,
            "soil below the declared threshold carries the bare class"
        );
        // And the same climate with soil carries a province: the difference is the soil, not the rain.
        assert!(band(400.0) > 0.0);
        assert!(bare.field(FieldId::Biome).expect("biome")[0] < band(400.0));
    }

    #[test]
    fn a_climate_the_declared_set_cannot_classify_is_refused_by_name() {
        let dials = ecology_dials();
        let patch = patch(2);
        // Below the floor there is no potential evapotranspiration to divide by.
        let frozen = ecology(&context(patch, "ecology", &dials, &filled(-4.0, 200.0, 1.0)))
            .expect_err("no evapotranspiration below the floor");
        assert_eq!(frozen.kind(), RefusalKind::ParameterOutOfRange);
        assert!(frozen.detail().contains("potential evapotranspiration"), "{frozen}");

        // And a true desert is drier than the driest province the set names: refused, not clamped
        // into the barren class, which would read as a province the column does not have.
        let drought = ecology(&context(patch, "ecology", &dials, &filled(13.0, 5.0, 1.2)))
            .expect_err("drier than the declared ladder");
        assert_eq!(drought.kind(), RefusalKind::ParameterOutOfRange);
        assert!(drought.detail().contains("drier than the declared driest province"), "{drought}");
    }

    #[test]
    fn ecology_refuses_a_climate_it_was_not_given() {
        let dials = ecology_dials();
        let patch = patch(2);
        let partial = filled(13.0, 200.0, 1.0);
        let mut values = FieldValues::new(patch);
        for field in [FieldId::Temperature, FieldId::Precipitation] {
            values
                .insert(field, partial.get(field).expect("copied").to_vec())
                .expect("a full column set");
        }
        let refused = ecology(&context(patch, "ecology", &dials, &values))
            .expect_err("ecology without soil is not an ecology");
        assert_eq!(refused.kind(), RefusalKind::StageInputAbsent);
        assert!(refused.detail().contains("soil_depth"), "{refused}");
    }

    #[test]
    fn miami_capacity_is_the_limited_curve_and_the_soil_limits_it() {
        let half = 0.2;
        // Cold and wet: the temperature curve is the scarcer one, so warming raises capacity.
        let cold = carrying_capacity(5.0, 2_000.0, 10.0, half).expect("a capacity");
        let warmer = carrying_capacity(10.0, 2_000.0, 10.0, half).expect("a capacity");
        assert!(warmer > cold, "temperature-limited: {cold} -> {warmer}");

        // Hot and dry: the precipitation curve is the scarcer one, so rain raises capacity.
        let dry = carrying_capacity(25.0, 50.0, 10.0, half).expect("a capacity");
        let wetter = carrying_capacity(25.0, 100.0, 10.0, half).expect("a capacity");
        assert!(wetter > dry, "precipitation-limited: {dry} -> {wetter}");
        assert!(dry < 100.0, "fifty millimetres cannot carry much: {dry}");

        // Neither curve passes the paper's asymptote: it is a limit approached from below, by a
        // climate hot and wet enough that neither factor is the scarce one.
        let mild = carrying_capacity(30.0, 5_000.0, 1_000.0, half).expect("a capacity");
        let tropical = carrying_capacity(50.0, 5_000.0, 1_000.0, half).expect("a capacity");
        let extreme = carrying_capacity(100.0, 20_000.0, 1_000.0, half).expect("a capacity");
        assert!(tropical > mild, "warming moves up the curve: {mild} -> {tropical}");
        assert!(extreme > tropical, "and so does more rain: {tropical} -> {extreme}");
        assert!(
            mild < 3_000.0 && tropical < 3_000.0 && extreme < 3_000.0,
            "the asymptote is not passed: {mild}, {tropical}, {extreme}"
        );
        assert!(extreme > 2_990.0, "and its tail is reachable: {extreme}");

        // And the soil limits it: no cover, no production, saturating rather than clipping.
        let bare = carrying_capacity(25.0, 2_000.0, 0.0, half).expect("a capacity");
        let thin = carrying_capacity(25.0, 2_000.0, half, half).expect("a capacity");
        assert_eq!(bare, 0.0, "no soil produces nothing");
        assert!(thin < mild, "a thin cover carries less: {thin} < {mild}");

        // A degenerate request is refused by name, never defaulted.
        for (t, p, soil, half) in [
            (f64::NAN, 100.0, 1.0, 0.2),
            (10.0, f64::INFINITY, 1.0, 0.2),
            (10.0, 100.0, -1.0, 0.2),
            (10.0, 100.0, 1.0, 0.0),
        ] {
            let refused = carrying_capacity(t, p, soil, half).expect_err("out of domain");
            assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        }
    }

    #[test]
    fn a_flock_needs_formation_to_carry_it_and_crowding_to_steer_it() {
        let patch = patch(3);
        let positions: Vec<(usize, VoxelPos)> = patch.columns().collect();
        let noise = Noise::new(7, 3);
        let weights = (0.5, 1.0, 0.5);

        // No formation anywhere: no agents, and the field says so rather than reading the noise.
        let bare = flock(&[0.0; 9], &positions, noise, 0.25, 2.0, weights).expect("a field");
        assert!(bare.iter().all(|f| *f == 0.0), "{bare:?}");

        // One agent among bare ground has nothing to flock with: still zero.
        let mut lone = [0.0; 9];
        lone[4] = 3.0;
        let single = flock(&lone, &positions, noise, 0.25, 2.0, weights).expect("a field");
        assert_eq!(single[4], 0.0, "a flock of one has no resultant");
        assert!(single.iter().all(|f| *f == 0.0), "{single:?}");

        // A full province is a flock, and it steers somewhere and stays bounded.
        let vegetated = flock(&[2.0; 9], &positions, noise, 0.25, 2.0, weights).expect("a field");
        assert!(vegetated.iter().any(|f| *f > 0.0), "a crowd has a resultant");
        assert!(
            vegetated.iter().all(|f| (0.0..=1.0).contains(f)),
            "the resultant is normalised by its weights: {vegetated:?}"
        );
        assert!(
            vegetated.iter().all(|f| f.is_finite()),
            "every column reports a number"
        );

        // The declared separation weight is load-bearing: dropping it moves the field.
        let no_separation =
            flock(&[2.0; 9], &positions, noise, 0.25, 2.0, (0.0, 1.0, 0.5)).expect("a field");
        assert!(
            vegetated.iter().zip(&no_separation).any(|(a, b)| a != b),
            "separation changes the resultant"
        );

        // It is a pure function of its inputs, and the radius is a real knob.
        let again = flock(&[2.0; 9], &positions, noise, 0.25, 2.0, weights).expect("a field");
        assert_eq!(vegetated, again, "same inputs, same flock");
        let nearby = flock(&[2.0; 9], &positions, noise, 0.25, 1.0, weights).expect("a field");
        assert_ne!(vegetated, nearby, "a smaller radius is a different flock");

        // Degenerate dials are refused by name.
        for (scale, radius, weights) in [
            (0.0, 2.0, (0.5, 1.0, 0.5)),
            (0.25, 0.0, (0.5, 1.0, 0.5)),
            (0.25, 2.0, (0.0, 0.0, 0.0)),
            (0.25, 2.0, (-1.0, 1.0, 0.5)),
        ] {
            let refused = flock(&[2.0; 9], &positions, noise, scale, radius, weights)
                .expect_err("out of domain");
            assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        }
    }

    #[test]
    fn ecology_produces_the_province_the_capacity_and_the_flock_in_one_pass() {
        let dials = ecology_dials();
        let patch = patch(2);
        let produced = ecology(&context(patch, "ecology", &dials, &filled(13.0, 400.0, 1.2)))
            .expect("three readings");
        let biome = produced.field(FieldId::Biome).expect("biome");
        let capacity = produced.field(FieldId::CarryingCapacity).expect("capacity");
        let flock = produced.field(FieldId::Flock).expect("flock");
        assert_eq!(biome.len(), 4);
        assert!(capacity.iter().all(|c| *c > 0.0), "a wet climate carries: {capacity:?}");
        // Four columns of the same formation are a flock of four, so it steers.
        assert!(flock.iter().any(|f| *f > 0.0), "{flock:?}");
    }

    #[test]
    fn caves_refuses_by_name_and_names_the_missing_field() {
        let patch = patch(2);
        let mut values = FieldValues::new(patch);
        values.insert(FieldId::Discharge, vec![4.0; 4]).expect("discharge");
        let refused = caves(&context(patch, "caves", &NO_DIALS, &values))
            .expect_err("cave occupancy is owed");
        assert_eq!(refused.kind(), RefusalKind::BodyOwed);
        assert_eq!(refused.kind().name(), "body_owed");
        assert!(refused.detail().contains("dissolution field"), "{refused}");

        // Without hydrology's flow it refuses for *that* reason first: the wiring is real.
        let empty = FieldValues::new(patch);
        let refused = caves(&context(patch, "caves", &NO_DIALS, &empty))
            .expect_err("no flow, no karst");
        assert_eq!(refused.kind(), RefusalKind::StageInputAbsent);
        assert!(refused.detail().contains("discharge"), "{refused}");
    }
}
