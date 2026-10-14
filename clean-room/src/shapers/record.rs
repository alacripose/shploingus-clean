//! **The record group — `delta`, `transition`, `earthworks`.**
//!
//! Derived from `docs/SHAPERS.md` §1's `delta`/`transition`/`earthworks` rows and §2's rule 6, and
//! `docs/MISSING-SYSTEMS.md` §4 (delta's law-plus-state; transition's partition of unity) and §5
//! (*“Earthworks is deliberately bodyless: it is the record, not a generator (F10)”*).
//!
//! | Stage | Paper | Produces |
//! |---|---|---|
//! | `delta` | Gilbert, *The topographic features of lake shores*, USGS 1885; Hjulström, *Studies of the morphological activity of rivers*, 1935 / Shields, 1936; Slingerland & Smith, *River avulsion and its role in alluvial architecture*, JSR 2004 | `deposition` |
//! | `transition` | Shepard, *A Two-Dimensional Interpolation Function for Irregularly-Spaced Data*, ACM 1968; normalized sparse convolution over a jittered grid (NoisePosti.ng 2021) | `blend` |
//! | `earthworks` | the F10 ruling — not a paper: a **conservation ledger** | *refuses* |
//!
//! Two of the three have bodies now, and the third is bodyless **by design** rather than by debt. The
//! distinction is what this module is for: `delta` and `transition` are laws with declared dials and
//! their own properties, while `earthworks` is a *decision* — the record is U2's chain, not a
//! generator — and it stays refused for that reason, in those words.
//!
//! What `delta` still does **not** carry is the state half: `DeltaState` (accumulation and avulsion)
//! has no home, and `MISSING-SYSTEMS` §4.1 rules that a per-tick accumulator outside the record would
//! be a second simulation. So the body lands the *law* — deposition at a river mouth, from the
//! delivered load and the accommodation the standing water leaves — and avulsion stays owed, named
//! where it is owed.

use crate::refusal::{Refusal, RefusalKind};
use crate::shapers::water::{accumulate_of, d8_receivers};
use crate::stage::{FieldId, Patch, Produced, StageContext};

fn out_of_range(what: &str, value: f64) -> Refusal {
    Refusal::new(RefusalKind::ParameterOutOfRange, format!("{what} = {value}"))
}

/// **`delta`** (Gilbert 1885; Hjulström 1935 / Shields 1936): deposition where a channel's load
/// exceeds what the flow can still carry and there is water to fill.
///
/// Four readings, in order:
///
/// 1. **Supply** — the eroded material delivered to a column, accumulated over `hydrology`'s D8
///    graph: the sum of `erosion_rate` over every column draining through this one
///    ([`accumulate_of`], the same routing the discharge is counted on).
/// 2. **Capacity** — what the flow carries on: `deposition_capacity_gain · discharge · slope`, so the
///    fraction of the load a channel keeps falls as its gradient does and **saturates at zero** at
///    the declared `deposition_slope` (Shields' threshold read as a slope the flow must hold).
/// 3. **The load it drops** — `(supply − capacity) · deposit_gain`, floored at zero: a channel below
///    its capacity deposits nothing rather than scouring a negative thickness.
/// 4. **Accommodation** — a delta cannot build above the water. The deposit is capped by the depth
///    `water_level − elevation` that lake fill found, so **a delta exists only at a shoreline**.
///
/// Reads `elevation`, `discharge`, `erosion_rate` and `water_level`, all declared above it.
///
/// **Owed:** avulsion (Slingerland & Smith 2004) is a decision about the channel *over time*, so it
/// needs the long-term state `MISSING-SYSTEMS` §4.1 leaves homeless. This reading is the deposition a
/// later state layer would avulse against; it is not that layer.
pub fn delta(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    ctx.require_region()?;
    let gain = ctx.dial("deposit_gain")?;
    if gain < 0.0 {
        return Err(out_of_range("deposit_gain", gain));
    }
    let threshold = ctx.dial("deposition_slope")?;
    if threshold <= 0.0 {
        return Err(out_of_range("deposition_slope", threshold));
    }
    let capacity_gain = ctx.dial("deposition_capacity_gain")?;
    if capacity_gain < 0.0 {
        return Err(out_of_range("deposition_capacity_gain", capacity_gain));
    }
    let elevation = ctx.read(FieldId::Elevation)?;
    let discharge = ctx.read(FieldId::Discharge)?;
    let eroded = ctx.read(FieldId::ErosionRate)?;
    let water_level = ctx.read(FieldId::WaterLevel)?;

    let supply = accumulate_of(
        ctx.patch,
        elevation,
        &d8_receivers(ctx.patch, elevation),
        eroded,
    );
    let mut deposition = Vec::with_capacity(ctx.len());
    for index in 0..ctx.len() {
        let slope = channel_slope(ctx.patch, elevation, index);
        let capacity = capacity_gain * discharge[index] * slope;
        let dropped = (supply[index] - capacity).max(0.0) * gain;
        // Gilbert: the deposit fills the standing water and never rises above it.
        let accommodation = (water_level[index] - elevation[index]).max(0.0) * gain;
        deposition.push(dropped.min(accommodation));
    }
    Ok(Produced::one(FieldId::Deposition, deposition))
}

/// The steepest descent from a column to a neighbour, in metres per metre: the channel gradient the
/// transport threshold is compared against. A column whose neighbours are all higher has no descent,
/// so its gradient is zero — which is exactly a mouth, where the flow stops carrying anything on.
fn channel_slope(patch: Patch, elevation: &[f64], index: usize) -> f64 {
    patch
        .neighbour_steps(index)
        .into_iter()
        .map(|(neighbour, dx, dy)| {
            let step = f64::from(dx * dx + dy * dy).sqrt();
            ((elevation[index] - elevation[neighbour]) / step).max(0.0)
        })
        .fold(0.0, f64::max)
}

/// The noise stream `transition` jitters its control points with — its own, so the grid cannot be a
/// second reading of another stage's field.
const SALT_TRANSITION: u64 = 0x7A11_510B;

/// **`transition`** (Shepard 1968; normalized sparse convolution over a jittered grid, NoisePosti.ng
/// 2021): the partition of unity that lets two domains meet without a seam.
///
/// The region's **control points** are a grid at `control_point_spacing_cells`, jittered by the
/// step's own noise (`control_point_jitter_cells` is the half-width — the jitter is what stops a
/// lattice of points producing grid-aligned creases). Each point carries the **domain** of the column
/// it landed on: the pair (biome province, lithology class), so two neighbouring points that share a
/// province but sit on different rock are still different domains.
///
/// A column's `blend` is the normalised inverse-distance weight (`shepard_power`) of the region's
/// **canonical domain** — the lowest pair by class, position breaking ties, so "the first domain" is
/// a declared rule rather than whichever point the loop reached first. The two domains therefore
/// always weigh one in total, and the field crosses a boundary smoothly rather than stepping in one
/// column. **A tie refuses** (`domain_tie`): a column whose two domains weigh the same within
/// `tie_tolerance` has no dominant domain, and picking one would invent a boundary the world does not
/// have. Reads `lithology` and `biome`.
pub fn transition(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    ctx.require_region()?;
    let spacing = ctx.dial("control_point_spacing_cells")?;
    if spacing < 1.0 {
        return Err(out_of_range("control_point_spacing_cells", spacing));
    }
    let jitter = ctx.dial("control_point_jitter_cells")?;
    if jitter < 0.0 {
        return Err(out_of_range("control_point_jitter_cells", jitter));
    }
    let power = ctx.dial("shepard_power")?;
    if power <= 0.0 {
        return Err(out_of_range("shepard_power", power));
    }
    let tolerance = ctx.dial("tie_tolerance")?;
    if tolerance < 0.0 {
        return Err(out_of_range("tie_tolerance", tolerance));
    }
    let lithology = ctx.read(FieldId::Lithology)?;
    let biome = ctx.read(FieldId::Biome)?;

    // The control grid: from the region's first column to its last, plus the last column itself even
    // when the spacing does not divide the side (an uncovered far edge would leave those columns with
    // no point of their own). A spacing below one column is refused rather than clamped: it would put
    // more than one point in a column, and then "the domain of the point" would no longer be the
    // domain of any column.
    let side = ctx.patch.side() as i32;
    let last = f64::from(side - 1);
    let mut grid: Vec<f64> = Vec::new();
    let mut at = 0.0_f64;
    while at < last {
        grid.push(at);
        at += spacing;
    }
    if grid.last().copied() != Some(last) {
        grid.push(last);
    }

    // The jittered control points, each carrying the domain of the column it landed on.
    let field = ctx.noise(SALT_TRANSITION);
    let sample = |x: f64, y: f64| -> usize {
        let (cx, cy) = (
            (x.floor() as i32).clamp(0, side - 1),
            (y.floor() as i32).clamp(0, side - 1),
        );
        (cy * side + cx) as usize
    };
    let mut points: Vec<(f64, f64)> = Vec::new();
    let mut domains: Vec<(i64, i64)> = Vec::new();
    for (row, by) in grid.iter().enumerate() {
        for (column, bx) in grid.iter().enumerate() {
            let x = bx + (field.unit_at(column as i64, row as i64) - 0.5) * 2.0 * jitter;
            let y =
                by + (field.unit_at(column as i64 + 1_013, row as i64 + 977) - 0.5) * 2.0 * jitter;
            let landed = sample(x, y);
            points.push((x, y));
            domains.push((biome[landed].round() as i64, lithology[landed].round() as i64));
        }
    }

    // The canonical domain: the lowest pair by class, position breaking ties.
    let canonical = points
        .iter()
        .enumerate()
        .min_by(|(a, pa), (b, pb)| {
            domains[*a]
                .cmp(&domains[*b])
                .then(pa.0.total_cmp(&pb.0))
                .then(pa.1.total_cmp(&pb.1))
        })
        .map(|(index, _)| domains[index])
        .expect("a region has at least one column, and so at least one control point");

    let mut blend = Vec::with_capacity(ctx.len());
    for (_index, at) in ctx.columns() {
        let (x, y) = (f64::from(at.x), f64::from(at.y));
        let mut canonical_weight = 0.0;
        let mut total = 0.0;
        for (point, domain) in points.iter().zip(&domains) {
            let squared = (point.0 - x) * (point.0 - x) + (point.1 - y) * (point.1 - y);
            if squared == 0.0 {
                // An exact hit takes all the weight: the column *is* that point's domain.
                canonical_weight = if *domain == canonical { 1.0 } else { 0.0 };
                total = 1.0;
                break;
            }
            let weight = squared.powf(-power / 2.0);
            total += weight;
            if *domain == canonical {
                canonical_weight += weight;
            }
        }
        let share = canonical_weight / total;
        if (share - 0.5).abs() <= tolerance {
            return Err(Refusal::new(
                RefusalKind::DomainTie,
                format!(
                    "the two domains weigh the same at ({}, {}) within the declared tolerance \
                     {tolerance}: no dominant domain to pick",
                    at.x, at.y
                ),
            ));
        }
        blend.push(share);
    }
    Ok(Produced::one(FieldId::Blend, blend))
}

/// **`earthworks`** — refused by name because it is **not a generator**.
///
/// `docs/SHAPERS.md` §1's last row and the F10 ruling: it is a **conservation ledger** — *a recipe
/// that is a mass pump must declare itself one* — and its home is U2's append-only [`crate::chain`],
/// not the terrain pipeline. A stage that generated terrain here would be the wrong shape of system;
/// a stage that quietly returned "no modification" would be a plausible substitute for a record
/// nobody has written yet.
pub fn earthworks(_ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    Err(Refusal::new(
        RefusalKind::NotAGenerator,
        "earthworks is the record, not a generator: recorded modification belongs to the append-only \
         chain, and its mass ledger is written by whoever changes the world",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::VoxelPos;
    use crate::stage::{Dials, FieldValues, StageContext};
    use crate::world::{DeclaredPeriod, Tick, WorldIdentity};

    /// A declared `const` so a bodyless stage needs no dials to refuse.
    const NO_DIALS: Dials = Dials::new(&[]);

    const DELTA_DIALS: Dials = Dials::new(&[
        ("deposit_gain", 1.0),
        ("deposition_slope", 1.0),
        ("deposition_capacity_gain", 0.05),
    ]);

    const TRANSITION_DIALS: Dials = Dials::new(&[
        ("control_point_spacing_cells", 2.0),
        ("control_point_jitter_cells", 0.0),
        ("shepard_power", 2.0),
        ("tie_tolerance", 0.0),
    ]);

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
            identity: WorldIdentity::new(9, DeclaredPeriod::seconds(1).expect("a period")),
            tick: Tick(0),
            ordinal: 13,
            stage,
            dials,
            values,
        }
    }

    /// A bowl with one channel draining into it: elevation falls from the rim to a pit, and the pit
    /// is flooded to its spill level, so its column is the region's one shoreline.
    fn bowl_with_a_channel() -> (Vec<f64>, Vec<f64>) {
        let elevation = vec![6.0, 6.0, 6.0, 4.0, 1.0, 6.0, 6.0, 6.0, 6.0];
        let water = vec![6.0, 6.0, 6.0, 4.0, 4.0, 6.0, 6.0, 6.0, 6.0];
        (elevation, water)
    }

    /// The fields a delta reads, with the same eroded load in every column.
    fn delta_inputs(
        patch: Patch,
        elevation: Vec<f64>,
        water: Vec<f64>,
        discharge: Vec<f64>,
    ) -> FieldValues {
        let mut values = FieldValues::new(patch);
        values.insert(FieldId::Elevation, elevation).expect("elevation");
        values.insert(FieldId::Discharge, discharge).expect("discharge");
        values
            .insert(FieldId::ErosionRate, vec![0.5; patch.len()])
            .expect("erosion_rate");
        values.insert(FieldId::WaterLevel, water).expect("water_level");
        values
    }

    /// One reading of `delta`, over the bowl's channel gradient at `step` metres per column and with
    /// water standing everywhere: the sum of the deposit, which is what the gradient moves.
    fn deposition_over(step: f64) -> f64 {
        let patch = patch(3);
        let elevation: Vec<f64> = (0..9).map(|i| 6.0 - (i % 3) as f64 * step).collect();
        let discharge: Vec<f64> = (0..9).map(|i| 1.0 + i as f64).collect();
        let values = delta_inputs(patch, elevation, vec![9.0; 9], discharge);
        let produced = delta(&context(patch, "delta", &DELTA_DIALS, &values)).expect("a reading");
        produced.field(FieldId::Deposition).expect("deposition").iter().sum()
    }

    #[test]
    fn delta_deposits_at_the_mouth_and_only_where_water_stands() {
        let patch = patch(3);
        let (elevation, water) = bowl_with_a_channel();
        let discharge: Vec<f64> = (0..9).map(|i| 1.0 + i as f64).collect();
        let values = delta_inputs(patch, elevation, water, discharge);

        let produced =
            delta(&context(patch, "delta", &DELTA_DIALS, &values)).expect("deposition at a mouth");
        let deposited = produced.field(FieldId::Deposition).expect("deposition");

        // Only the flooded pit receives: a delta exists at a shoreline, nowhere else.
        assert!(deposited[4] > 0.0, "the mouth receives the load: {deposited:?}");
        for index in [0, 1, 2, 3, 5, 6, 7, 8] {
            assert_eq!(
                deposited[index], 0.0,
                "column {index} is dry or still carrying: {deposited:?}"
            );
        }
        // It fills the accommodation the water leaves there — the whole 3 m — and no more.
        assert!((deposited[4] - 3.0).abs() < 1e-12, "and it fills them: {}", deposited[4]);
        assert!(deposited[4] <= 7.0 * 0.5, "never more than was delivered");
    }

    #[test]
    fn a_dry_region_deposits_nothing_and_a_gentler_channel_drops_more() {
        let patch = patch(3);
        let discharge: Vec<f64> = (0..9).map(|i| 1.0 + i as f64).collect();
        // The same ground with no water standing on it: nothing can be deposited at all.
        let (elevation, water) = bowl_with_a_channel();
        let dry: Vec<f64> = water.iter().map(|level| level - 3.0).collect();
        let values = delta_inputs(patch, elevation, dry, discharge);
        let produced = delta(&context(patch, "delta", &DELTA_DIALS, &values)).expect("a reading");
        assert!(
            produced
                .field(FieldId::Deposition)
                .expect("deposition")
                .iter()
                .all(|d| *d == 0.0),
            "no standing water, no delta"
        );

        // A gentler channel drops more of the same load, because its capacity is lower.
        assert!(
            deposition_over(0.75) < deposition_over(0.25) + 1e-12,
            "a steeper channel must not drop more"
        );
        assert!(
            deposition_over(0.75) < deposition_over(0.25),
            "and the difference is real: {} against {}",
            deposition_over(0.75),
            deposition_over(0.25)
        );
    }

    #[test]
    fn a_blend_is_a_partition_of_unity_that_crosses_the_boundary_without_a_step() {
        let dials = Dials::new(&[
            ("control_point_spacing_cells", 1.0),
            ("control_point_jitter_cells", 0.5),
            ("shepard_power", 2.0),
            ("tie_tolerance", 1e-9),
        ]);
        let patch = patch(5);
        // Two lithology domains, two columns wide each: the blend must cross between them.
        let mut values = FieldValues::new(patch);
        let lithology: Vec<f64> = (0..25).map(|i| if i % 5 < 2 { 0.0 } else { 3.0 }).collect();
        values.insert(FieldId::Lithology, lithology).expect("lithology");
        values.insert(FieldId::Biome, vec![2.0; 25]).expect("biome");

        let produced = transition(&context(patch, "transition", &dials, &values)).expect("a blend");
        let blend = produced.field(FieldId::Blend).expect("blend");
        assert_eq!(blend.len(), 25);
        assert!(
            blend.iter().all(|w| (0.0..=1.0).contains(w) && w.is_finite()),
            "a weight is a share of one: {blend:?}"
        );
        // The field crosses from one domain to the other along x, and nothing steps.
        let row: Vec<f64> = blend[..5].to_vec();
        for pair in row.windows(2) {
            assert!(pair[0] >= pair[1] - 0.05, "the crossing is not monotone: {row:?}");
            assert!((pair[1] - pair[0]).abs() < 0.6, "a seam, not a blend: {row:?}");
        }
        assert!(
            row[0] - row[4] > 0.5,
            "the domains must actually differ across the region: {row:?}"
        );
    }

    #[test]
    fn a_blend_tie_refuses_by_name_rather_than_picking_a_domain() {
        // A regular grid with no jitter, and one rock on column 0 against another on column 2: the
        // columns between the two points weigh both domains exactly equally, so no dominant domain.
        let patch = patch(3);
        let mut values = FieldValues::new(patch);
        let lithology: Vec<f64> = (0..9).map(|i| if i % 3 == 0 { 0.0 } else { 5.0 }).collect();
        values.insert(FieldId::Lithology, lithology).expect("lithology");
        values.insert(FieldId::Biome, vec![2.0; 9]).expect("biome");

        let refused = transition(&context(patch, "transition", &TRANSITION_DIALS, &values))
            .expect_err("a tie has no dominant domain");
        assert_eq!(refused.kind(), RefusalKind::DomainTie);
        assert_eq!(refused.kind().name(), "domain_tie");
        assert!(refused.detail().contains("dominant domain"), "{refused}");
    }

    #[test]
    fn the_record_refuses_by_name_rather_than_returning_nothing() {
        let patch = patch(2);
        let values = FieldValues::new(patch);
        let record = earthworks(&context(patch, "earthworks", &NO_DIALS, &values))
            .expect_err("earthworks is the record");
        assert_eq!(record.kind().name(), "not_a_generator");
        assert!(record.detail().contains("chain"), "{record}");
    }

    #[test]
    fn a_real_body_names_an_input_it_was_not_given() {
        // The wiring claim: these stages read real fields, so a request that omits them is refused
        // for the missing input rather than for anything else.
        let patch = patch(2);
        let empty = FieldValues::new(patch);
        let refused = delta(&context(patch, "delta", &DELTA_DIALS, &empty))
            .expect_err("no elevation, no channel");
        assert_eq!(refused.kind(), RefusalKind::StageInputAbsent);
        assert!(refused.detail().contains("elevation"), "{refused}");
        let refused = transition(&context(patch, "transition", &TRANSITION_DIALS, &empty))
            .expect_err("no domains, no blend");
        assert_eq!(refused.kind(), RefusalKind::StageInputAbsent);
        assert!(refused.detail().contains("lithology"), "{refused}");
    }
}
