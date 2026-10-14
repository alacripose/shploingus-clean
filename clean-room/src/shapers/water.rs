//! **The water group — `hydrology`, `substrate`, `erosion`.**
//!
//! Derived from `docs/SHAPERS.md` §1 and `docs/MISSING-SYSTEMS.md` §2 (the two landscape-scale laws
//! the record names as genuinely absent) and §2.2 (nonlinear hillslope diffusion).
//!
//! | Stage | Paper | Produces |
//! |---|---|---|
//! | `hydrology` | O'Callaghan & Mark, *The extraction of drainage networks from digital elevation data* (D8), CVGIP 1984; Strahler, *Hypsometric (area-altitude) analysis of erosional topography*, GSA Bulletin 1952; Barnes, Lehman & Mulla, *Priority-Flood*, Computers & Geosciences 2014 | `discharge`, `stream_order`, `water_level` |
//! | `substrate` | Heimsath, Dietrich, Nishiizumi & Finkel, *The soil production function and landscape equilibrium*, Nature 1997 | `soil_depth` |
//! | `erosion` | Whipple & Tucker, *Dynamics of the stream-power river incision model*, JGR 1999; Lague, *ESPL* 2014; Roering, Kirchner & Dietrich, *Evidence for nonlinear, diffusive sediment transport on hillslopes*, WRR 1999 | `erosion_rate`, `hillslope_flux` |
//!
//! Every law here is a relation **between columns**, so every one of them refuses a lone site by
//! name (`site_alone`) instead of inventing a neighbourhood: the runtime consumes the region its
//! caller borrowed, and a site alone cannot evaluate a slope.
//!
//! The three papers in the `hydrology` row are three halves of one routing graph, and each is landed
//! against the same receivers: the D8 descent decides *where* water goes, the accumulation decides
//! *how much*, Strahler's rule decides *what order* the channel is, and Priority-Flood decides
//! *where it stands*. `erosion` then reads the graph twice: the channel law over the discharge
//! (stream power) and the hillslope law over the elevation (nonlinear diffusion).

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use crate::refusal::{Refusal, RefusalKind};
use crate::stage::{slopes, FieldId, Patch, Produced, StageContext};

fn out_of_range(what: &str, value: f64) -> Refusal {
    Refusal::new(RefusalKind::ParameterOutOfRange, format!("{what} = {value}"))
}

/// **`hydrology`, routing half** (O'Callaghan & Mark 1984): the D8 receiver of every column — the
/// neighbour with the steepest descent. A column no neighbour is lower than is a **pit**, and has no
/// receiver (`None`): water stops there rather than being sent uphill.
pub fn d8_receivers(patch: Patch, elevation: &[f64]) -> Vec<Option<usize>> {
    let mut receivers = vec![None; patch.len()];
    for (index, receiver) in receivers.iter_mut().enumerate() {
        let mut best: Option<(usize, f64)> = None;
        for (neighbour, _dx, _dy) in patch.neighbour_steps(index) {
            let drop = elevation[index] - elevation[neighbour];
            if drop > 0.0 && best.map(|(_, steepest)| drop > steepest).unwrap_or(true) {
                best = Some((neighbour, drop));
            }
        }
        *receiver = best.map(|(neighbour, _)| neighbour);
    }
    receivers
}

/// The columns in the order a routing law must visit them: **highest first**, index breaking ties, so
/// a column is always finished before the column it drains into. One order, computed from the
/// elevation itself, so no law can disagree with another about it.
fn upstream_first(patch: Patch, elevation: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..patch.len()).collect();
    order.sort_by(|a, b| {
        elevation[*b]
            .partial_cmp(&elevation[*a])
            .unwrap_or(Ordering::Equal)
            .then(a.cmp(b))
    });
    order
}

/// **`hydrology`, accumulation half**: how many columns drain through each column, counting itself.
pub fn accumulate(patch: Patch, elevation: &[f64], receivers: &[Option<usize>]) -> Vec<f64> {
    accumulate_of(patch, elevation, receivers, &vec![1.0; patch.len()])
}

/// The same accumulation over an **arbitrary per-column contribution**: what a column delivers, plus
/// everything delivered to it from upstream. One implementation, so a law that accumulates a load
/// and a law that counts cells cannot disagree about the routing graph.
pub fn accumulate_of(
    patch: Patch,
    elevation: &[f64],
    receivers: &[Option<usize>],
    contribution: &[f64],
) -> Vec<f64> {
    let mut total = contribution.to_vec();
    for index in upstream_first(patch, elevation) {
        if let Some(receiver) = receivers[index] {
            total[receiver] += total[index];
        }
    }
    total
}

/// **Strahler stream order** (Strahler 1952) over the same receiver graph: a headwater is order `1`,
/// and where **two tributaries of equal order** meet the trunk takes the next order up. Unequal
/// tributaries leave it at the higher one — which is why a long unbranched channel never climbs, and
/// why the order is a reading of the network's *branching* rather than of its length.
pub fn strahler_order(patch: Patch, elevation: &[f64], receivers: &[Option<usize>]) -> Vec<f64> {
    let mut donors: Vec<Vec<usize>> = vec![Vec::new(); patch.len()];
    for (index, receiver) in receivers.iter().enumerate() {
        if let Some(receiver) = receiver {
            donors[*receiver].push(index);
        }
    }
    let mut order = vec![1.0; patch.len()];
    for index in upstream_first(patch, elevation) {
        let mut highest = 0.0_f64;
        let mut equal = 0_usize;
        for donor in &donors[index] {
            let donor_order = order[*donor];
            if donor_order > highest {
                highest = donor_order;
                equal = 1;
            } else if donor_order == highest {
                equal += 1;
            }
        }
        order[index] = if equal >= 2 { highest + 1.0 } else { highest.max(1.0) };
    }
    order
}

/// **Priority-Flood** (Barnes, Lehman & Mulla 2014): the **filled** water surface of the region, in
/// metres — the fill half of lake fill.
///
/// The region is borrowed from the caller, so it is open at the **rim**: water leaves at the edge.
/// Every interior depression therefore fills to its **spill level**, the lowest level at which it can
/// escape — the standard priority-flood sweep, seeded with the rim and always expanding from the
/// lowest surface found so far. A column with no standing water reports its own elevation, so
/// `level − elevation` is the depth and nothing is invented where there is nothing to fill.
///
/// Ties are broken by index and nothing else, so two runs agree; the heap is a `BinaryHeap` over a
/// total order (`f64::total_cmp`), not a comparison that can contradict itself on equal levels.
pub fn lake_levels(patch: Patch, elevation: &[f64]) -> Vec<f64> {
    /// One candidate surface on the flood front. Ordered by level, then by index, so the order is
    /// total and the sweep is reproducible.
    #[derive(PartialEq)]
    struct Spill {
        level: f64,
        index: usize,
    }

    impl Eq for Spill {}

    impl Ord for Spill {
        fn cmp(&self, other: &Self) -> Ordering {
            self.level
                .total_cmp(&other.level)
                .then(self.index.cmp(&other.index))
        }
    }

    impl PartialOrd for Spill {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut filled = elevation.to_vec();
    let mut visited = vec![false; patch.len()];
    let mut front: BinaryHeap<Reverse<Spill>> = BinaryHeap::new();
    for index in 0..patch.len() {
        if patch.is_edge(index) {
            visited[index] = true;
            front.push(Reverse(Spill { level: elevation[index], index }));
        }
    }
    while let Some(Reverse(Spill { level, index })) = front.pop() {
        for (neighbour, _dx, _dy) in patch.neighbour_steps(index) {
            if visited[neighbour] {
                continue;
            }
            visited[neighbour] = true;
            // Water stands at the lowest level it can escape at, so the surface never drops below
            // the ground it crossed to get here.
            let surface = elevation[neighbour].max(level);
            filled[neighbour] = surface;
            front.push(Reverse(Spill { level: surface, index: neighbour }));
        }
    }
    filled
}

/// **`hydrology`**: the routing graph, read three ways. Discharge = the columns draining through a
/// column × the declared rainfall per column; stream order = Strahler's rule over the same receivers;
/// water level = the filled surface Priority-Flood finds. Reads `elevation`.
pub fn hydrology(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    ctx.require_region()?;
    let rainfall = ctx.dial("rainfall_per_cell")?;
    if rainfall < 0.0 {
        return Err(out_of_range("rainfall_per_cell", rainfall));
    }
    let elevation = ctx.read(FieldId::Elevation)?;
    let receivers = d8_receivers(ctx.patch, elevation);
    let cells = accumulate(ctx.patch, elevation, &receivers);
    let mut produced = Produced::one(
        FieldId::Discharge,
        cells.into_iter().map(|count| count * rainfall).collect(),
    );
    produced.push(
        FieldId::StreamOrder,
        strahler_order(ctx.patch, elevation, &receivers),
    );
    produced.push(FieldId::WaterLevel, lake_levels(ctx.patch, elevation));
    Ok(produced)
}

/// **`substrate`** (Heimsath et al. 1997): soil depth from the soil production function, with the
/// slope standing in for the production the cover cannot keep up with. Reads `elevation`.
pub fn substrate(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    ctx.require_region()?;
    let maximum = ctx.dial("max_soil_depth_m")?;
    if maximum <= 0.0 {
        return Err(out_of_range("max_soil_depth_m", maximum));
    }
    let decay = ctx.dial("decay_per_slope")?;
    if decay < 0.0 {
        return Err(out_of_range("decay_per_slope", decay));
    }
    let elevation = ctx.read(FieldId::Elevation)?;
    let slope = slopes(ctx.patch, elevation);
    // The production function's own shape: production falls exponentially with the cover's depth,
    // so the steady depth falls exponentially with slope.
    Ok(Produced::one(
        FieldId::SoilDepth,
        slope.into_iter().map(|s| maximum * (-decay * s).exp()).collect(),
    ))
}

/// **The stream-power law** `E = K · A^m · S^n` (Whipple & Tucker 1999; Lague 2014). `K`, `m` and `n`
/// are **declared dials**; the observed downslope decrease of channel slope reproduces for
/// `m/n ≈ 0.5` (Lague 2014), and the dials are where that reading is declared.
pub fn stream_power(k: f64, area: f64, slope: f64, m: f64, n: f64) -> Result<f64, Refusal> {
    if k < 0.0 {
        return Err(out_of_range("stream_power k", k));
    }
    if area < 0.0 {
        return Err(out_of_range("drainage area", area));
    }
    if slope < 0.0 {
        return Err(out_of_range("channel slope", slope));
    }
    if m < 0.0 || n < 0.0 {
        return Err(out_of_range("stream_power exponents", m))
    }
    Ok(k * area.powf(m) * slope.powf(n))
}

/// **Nonlinear hillslope transport** (Roering, Kirchner & Dietrich 1999):
/// `q = K · S / (1 − (S/S_c)²)`. At or past the critical slope the law has no finite value, and that
/// is a refusal with both numbers in it — not a clamp to the largest value the formula can express.
pub fn hillslope_flux(slope: f64, transport_rate: f64, critical_slope: f64) -> Result<f64, Refusal> {
    if transport_rate < 0.0 {
        return Err(out_of_range("hillslope transport_rate", transport_rate));
    }
    if critical_slope <= 0.0 {
        return Err(out_of_range("critical slope", critical_slope));
    }
    let ratio = slope / critical_slope;
    if ratio.abs() >= 1.0 {
        return Err(Refusal::new(
            RefusalKind::ParameterOutOfRange,
            format!(
                "slope {slope} is at or past the critical slope {critical_slope}: the transport law has no finite value there"
            ),
        ));
    }
    Ok(transport_rate * slope / (1.0 - ratio * ratio))
}

/// **The divergence of nonlinear hillslope transport** (Roering, Kirchner & Dietrich 1999), as the
/// field `erosion` declares.
///
/// Every unordered pair of neighbours inside the region exchanges `q = K·S/(1−(S/S_c)²)` from the
/// higher column to the lower — the same amount leaves one and arrives at the other, so over a
/// **closed** region the divergence sums to zero (SC-005). The region is closed because the runtime
/// only ever hands a law neighbours *inside* the patch: there is no boundary flux to account for
/// because there is no boundary in the graph.
///
/// A pair at or past the critical slope makes the law refuse (its value is not finite there), and the
/// refusal is the *stage's* — a column of exposed rock refuses to be described by a soil-mantled law
/// rather than being clamped into one.
pub fn hillslope_divergence(
    patch: Patch,
    elevation: &[f64],
    transport_rate: f64,
    critical_slope: f64,
) -> Result<Vec<f64>, Refusal> {
    let mut divergence = vec![0.0; patch.len()];
    for index in 0..patch.len() {
        for (neighbour, dx, dy) in patch.neighbour_steps(index) {
            if neighbour < index {
                continue; // each pair once: the exchange is antisymmetric by construction
            }
            let step = f64::from(dx * dx + dy * dy).sqrt();
            let gradient = (elevation[index] - elevation[neighbour]) / step;
            let flux = hillslope_flux(gradient.abs(), transport_rate, critical_slope)?;
            let signed = if gradient > 0.0 {
                flux
            } else if gradient < 0.0 {
                -flux
            } else {
                0.0
            };
            divergence[index] += signed;
            divergence[neighbour] -= signed;
        }
    }
    Ok(divergence)
}

/// **`erosion`**: the two laws the record names as genuinely absent, now on one field pair.
///
/// `erosion_rate` is the stream-power incision `E = K·A^m·S^n` (Whipple & Tucker 1999; Lague 2014)
/// over the discharge the graph produced; `hillslope_flux` is the divergence of the nonlinear
/// hillslope transport above. Reads `discharge` and `elevation`.
pub fn erosion(ctx: &StageContext<'_>) -> Result<Produced, Refusal> {
    ctx.require_region()?;
    let k = ctx.dial("stream_power_k")?;
    let m = ctx.dial("stream_power_m")?;
    let n = ctx.dial("stream_power_n")?;
    let reference = ctx.dial("reference_discharge")?;
    if reference <= 0.0 {
        return Err(out_of_range("reference_discharge", reference));
    }
    let transport_rate = ctx.dial("hillslope_transport_rate")?;
    let critical_slope = ctx.dial("critical_slope")?;
    let discharge = ctx.read(FieldId::Discharge)?;
    let elevation = ctx.read(FieldId::Elevation)?;
    let slope = slopes(ctx.patch, elevation);
    let mut rates = Vec::with_capacity(ctx.len());
    for (area, steepness) in discharge.iter().zip(&slope) {
        rates.push(stream_power(k, area / reference, *steepness, m, n)?);
    }
    let flux = hillslope_divergence(ctx.patch, elevation, transport_rate, critical_slope)?;
    let mut produced = Produced::one(FieldId::ErosionRate, rates);
    produced.push(FieldId::HillslopeFlux, flux);
    Ok(produced)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::VoxelPos;
    use crate::stage::{Dials, FieldValues, StageContext};
    use crate::world::{DeclaredPeriod, Tick, WorldIdentity};

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
            identity: WorldIdentity::new(7, DeclaredPeriod::seconds(1).expect("a period")),
            tick: Tick(0),
            ordinal: 5,
            stage,
            dials,
            values,
        }
    }

    #[test]
    fn d8_routes_downhill_and_leaves_a_pit_alone() {
        let patch = patch(3);
        // A ramp falling to +x, 2 m per column, with the last column a pit floor.
        let mut elevation: Vec<f64> = (0..9).map(|i| 9.0 - (i % 3) as f64 * 2.0).collect();
        elevation[2] = -50.0;
        let receivers = d8_receivers(patch, &elevation);
        assert_eq!(receivers.len(), 9);
        assert_eq!(receivers[0], Some(1), "the steepest descent from column 0 is to its east");
        assert_eq!(receivers[1], Some(2));
        assert_eq!(receivers[2], None, "a pit sends water nowhere: it does not route uphill");
        for (index, receiver) in receivers.iter().enumerate() {
            if let Some(receiver) = receiver {
                assert!(
                    elevation[index] > elevation[*receiver],
                    "a receiver must be lower: {index} -> {receiver}"
                );
            }
        }
    }

    #[test]
    fn discharge_accumulates_downstream_and_the_reference_dials_refuse_nothing() {
        let patch = patch(3);
        let elevation: Vec<f64> = (0..9).map(|i| 9.0 - (i % 3) as f64 * 2.0).collect();
        let receivers = d8_receivers(patch, &elevation);
        let cells = accumulate(patch, &elevation, &receivers);
        // The ramp falls along +x only, so the three rows are independent drains and each row's
        // third column is that row's outlet.
        assert_eq!(cells[2], 3.0, "a row drains through its own outlet: {cells:?}");
        assert_eq!(cells[0], 1.0, "a source counts only itself");
        // Conservation, stated over the outlets: every cell drains into exactly one outlet, so the
        // outlets' accumulations account for every column exactly once. (Summing *all* the
        // accumulations would count a column once per cell upstream of it, which is a different
        // number: 1+2+3 per row here.)
        let outlets: Vec<usize> = (0..9).filter(|index| receivers[*index].is_none()).collect();
        assert_eq!(outlets, vec![2, 5, 8], "one outlet per row");
        let drained: f64 = outlets.iter().map(|outlet| cells[*outlet]).sum();
        assert_eq!(drained, 9.0, "every column is accounted for exactly once");

        let dials = Dials::new(&[("rainfall_per_cell", 0.5)]);
        let mut values = FieldValues::new(patch);
        values.insert(FieldId::Elevation, elevation).expect("elevation");
        let discharge = hydrology(&context(patch, "hydrology", &dials, &values)).expect("discharge");
        let discharge = discharge.field(FieldId::Discharge).expect("discharge");
        assert!((discharge[2] - 1.5).abs() < 1e-12, "three cells at 0.5 each");

        // A basin that falls on both axes funnels every column to one outlet, so the accumulation
        // is the whole patch and no drop is lost on the way.
        let fall: Vec<f64> = (0..9)
            .map(|i| 10.0 - 2.0 * (i % 3) as f64 - 3.0 * (i / 3) as f64)
            .collect();
        let receivers = d8_receivers(patch, &fall);
        let cells = accumulate(patch, &fall, &receivers);
        assert_eq!(
            cells[8], 9.0,
            "the lowest corner drains the whole patch: {cells:?}"
        );
        assert_eq!(
            (0..9).filter(|index| receivers[*index].is_none()).collect::<Vec<_>>(),
            vec![8],
            "and it is the only outlet, so nothing is left draining elsewhere"
        );
    }

    #[test]
    fn a_lone_site_cannot_evaluate_a_law_about_a_relation() {
        let dials = Dials::new(&[("rainfall_per_cell", 1.0)]);
        let mut values = FieldValues::new(patch(1));
        values.insert(FieldId::Elevation, vec![3.0]).expect("elevation");
        let refused = hydrology(&context(patch(1), "hydrology", &dials, &values))
            .expect_err("a lone site cannot route water");
        assert_eq!(refused.kind(), RefusalKind::SiteAlone);
        assert!(refused.detail().contains("lone site"), "{refused}");
    }

    #[test]
    fn stream_order_counts_branches_and_not_length() {
        let patch = patch(3);
        // A Y: two headwaters (0,0)/(0,2) join at (0,1), and the arms of a second pair join there
        // too, around one pit. Two tributaries of equal order meeting take the next order up.
        let junction: Vec<f64> = vec![
            3.0, 3.0, 3.0, //
            2.0, 0.0, 2.0, //
            3.0, 3.0, 3.0,
        ];
        let receivers = d8_receivers(patch, &junction);
        let order = strahler_order(patch, &junction, &receivers);
        assert_eq!(order[1], 1.0, "a headwater is order one");
        assert_eq!(order[3], 2.0, "two twigs make a second-order stream");
        assert_eq!(order[4], 3.0, "two second-order streams make a third: {order:?}");
        assert!(order.iter().all(|o| *o >= 1.0 && o.fract() == 0.0), "{order:?}");

        // A long unbranched channel is order one all the way down: order reads branching, not length.
        let ramp: Vec<f64> = (0..9).map(|i| 9.0 - (i % 3) as f64 * 2.0).collect();
        let receivers = d8_receivers(patch, &ramp);
        let order = strahler_order(patch, &ramp, &receivers);
        assert!(order.iter().all(|o| *o == 1.0), "no junction, no climb: {order:?}");
    }

    #[test]
    fn lake_fill_reaches_the_spill_level_and_never_lowers_the_ground() {
        let patch = patch(3);
        // Flat ground has nothing to fill.
        let flat = lake_levels(patch, &[5.0; 9]);
        assert!(flat.iter().all(|level| *level == 5.0), "{flat:?}");

        // A bowl fills to the spill level: its rim.
        let mut bowl = [2.0; 9];
        bowl[4] = 0.0;
        let filled = lake_levels(patch, &bowl);
        assert_eq!(filled[4], 2.0, "the pit fills to the level it can escape at");
        assert!(
            filled.iter().zip(&bowl).all(|(level, ground)| *level >= *ground),
            "filling never lowers the ground: {filled:?}"
        );

        // Two pits behind one sill share one level: a water surface is level, not per-pit.
        let mut two = [4.0; 25];
        two[6] = 1.0; // (1,1)
        two[8] = 1.0; // (3,1)
        let wide = Patch::new(VoxelPos::new(0, 0, 0), 5).expect("a 5 × 5 patch");
        let filled = lake_levels(wide, &two);
        assert_eq!(filled[6], 4.0, "the first pit fills to the sill");
        assert_eq!(filled[8], filled[6], "and the second shares the same surface");
        assert!(filled.iter().zip(&two).all(|(level, ground)| *level >= *ground));
    }

    #[test]
    fn hillslope_transport_conserves_over_a_closed_region_and_refuses_a_cliff() {
        let patch = patch(3);
        // A gentle ramp: every pair is below the critical slope, so the law has a value everywhere.
        let ramp: Vec<f64> = (0..9).map(|i| 3.0 - (i % 3) as f64 * 0.5).collect();
        let divergence = hillslope_divergence(patch, &ramp, 0.01, 1.0).expect("below the cliff");
        let total: f64 = divergence.iter().sum();
        let magnitude: f64 = divergence.iter().map(|q| q.abs()).sum();
        assert!(
            total.abs() <= 1e-12 * magnitude.max(1.0),
            "a closed region's exchanges cancel: {total} against {magnitude}"
        );
        assert!(divergence[0] > 0.0, "the top of the ramp sheds material");
        assert!(divergence[2] < 0.0, "and the bottom receives it");

        // A cliff has no finite transport value, so the law refuses instead of clamping.
        let refused = hillslope_divergence(patch, &ramp, 0.01, 0.25)
            .expect_err("a 0.5 m step is past a 0.25 critical slope");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(refused.detail().contains("critical slope"), "{refused}");
    }

    #[test]
    fn erosion_declares_both_laws_and_the_hillslope_half_conserves() {
        let dials = Dials::new(&[
            ("stream_power_k", 0.001),
            ("stream_power_m", 0.5),
            ("stream_power_n", 1.0),
            ("reference_discharge", 8.0),
            ("hillslope_transport_rate", 0.01),
            ("critical_slope", 1.0),
        ]);
        let patch = patch(3);
        let ramp: Vec<f64> = (0..9).map(|i| 3.0 - (i % 3) as f64 * 0.5).collect();
        let mut values = FieldValues::new(patch);
        values.insert(FieldId::Elevation, ramp).expect("elevation");
        values.insert(FieldId::Discharge, (0..9).map(|i| 0.5 + i as f64).collect()).expect("discharge");

        let produced = erosion(&context(patch, "erosion", &dials, &values)).expect("both laws");
        let rate = produced.field(FieldId::ErosionRate).expect("erosion_rate");
        let flux = produced.field(FieldId::HillslopeFlux).expect("hillslope_flux");
        assert!(rate.iter().all(|e| *e > 0.0), "stream power is positive on a channel");
        let total: f64 = flux.iter().sum();
        let magnitude: f64 = flux.iter().map(|q| q.abs()).sum();
        assert!(
            total.abs() <= 1e-12 * magnitude.max(1.0),
            "the hillslope half of the field conserves: {total}"
        );

        // The stage refuses for a cliff rather than reporting a clamped flux there.
        let steep = Dials::new(&[
            ("stream_power_k", 0.001),
            ("stream_power_m", 0.5),
            ("stream_power_n", 1.0),
            ("reference_discharge", 8.0),
            ("hillslope_transport_rate", 0.01),
            ("critical_slope", 0.25),
        ]);
        let refused = erosion(&context(patch, "erosion", &steep, &values))
            .expect_err("a 0.5 m step is past a 0.25 critical slope");
        assert!(refused.detail().contains("critical slope"), "{refused}");
    }

    #[test]
    fn soil_is_thicker_where_the_ground_is_flat() {
        let dials = Dials::new(&[("max_soil_depth_m", 1.2), ("decay_per_slope", 3.0)]);
        let patch = patch(3);
        let mut flat = FieldValues::new(patch);
        flat.insert(FieldId::Elevation, vec![100.0; 9]).expect("elevation");
        let mut steep = FieldValues::new(patch);
        steep.insert(FieldId::Elevation, (0..9).map(|i| 100.0 - (i % 3) as f64 * 4.0).collect()).expect("elevation");

        let flat_soil = substrate(&context(patch, "substrate", &dials, &flat)).expect("soil");
        let steep_soil = substrate(&context(patch, "substrate", &dials, &steep)).expect("soil");
        let flat_soil = flat_soil.field(FieldId::SoilDepth).expect("soil");
        let steep_soil = steep_soil.field(FieldId::SoilDepth).expect("soil");
        assert!((flat_soil[0] - 1.2).abs() < 1e-12, "flat ground keeps its full depth");
        assert!(steep_soil[0] < flat_soil[0], "a 4 m step must thin the cover");
        assert!(steep_soil.iter().all(|s| *s > 0.0), "soil is thin, not absent");
    }

    #[test]
    fn the_stream_power_law_rises_with_area_and_with_slope() {
        let base = stream_power(0.001, 1.0, 0.1, 0.5, 1.0).expect("a declared law");
        let more_area = stream_power(0.001, 4.0, 0.1, 0.5, 1.0).expect("a declared law");
        let steeper = stream_power(0.001, 1.0, 0.2, 0.5, 1.0).expect("a declared law");
        assert!(more_area > base, "E rises with drainage area");
        assert!(steeper > base, "E rises with slope");
        assert!((more_area / base - 2.0).abs() < 1e-12, "A^0.5 doubles when A quadruples");
        // A negative area or slope is refused, never clamped.
        assert_eq!(stream_power(0.001, -1.0, 0.1, 0.5, 1.0).err().unwrap().kind(), RefusalKind::ParameterOutOfRange);
        assert_eq!(stream_power(0.001, 1.0, -0.1, 0.5, 1.0).err().unwrap().kind(), RefusalKind::ParameterOutOfRange);
    }

    #[test]
    fn the_hillslope_law_refuses_at_the_critical_slope() {
        let finite = hillslope_flux(0.5, 0.01, 1.0).expect("below the critical slope");
        assert!(finite > 0.0);
        let steeper = hillslope_flux(0.9, 0.01, 1.0).expect("still below");
        assert!(steeper > finite, "nonlinear transport grows faster than linear near the critical slope");
        let refused = hillslope_flux(1.0, 0.01, 1.0).expect_err("the critical slope is not finite");
        assert_eq!(refused.kind(), RefusalKind::ParameterOutOfRange);
        assert!(refused.detail().contains("no finite value"), "{refused}");
    }

}
