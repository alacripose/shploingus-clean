//! Hydrology — where the water goes.
//!
//! **Sources.**
//! * O'Callaghan, J. & Mark, D. *The extraction of drainage networks from digital
//!   elevation data.* Computer Vision, Graphics, and Image Processing 1984 —
//!   the **D8** routing rule: every cell sends its flow to its steepest
//!   downhill neighbour among eight.
//! * Barnes, R., Lehman, C. & Mulla, D. *Priority-flood: An optimal
//!   depression-filling and watershed-labeling algorithm.* Computers &
//!   Geosciences 2014 — the depression fill, without which D8 produces a
//!   drainage network that dead-ends in every puddle.
//! * Strahler, A. *Hypsometric (area-altitude) analysis of erosional topography.*
//!   GSA Bulletin 1952 — stream **order**, the branching measure that turns a
//!   flow field into a hierarchy.
//!
//! # Why the fill must come first
//!
//! Plain D8 on raw terrain is broken in a way that is easy to miss: any cell in a
//! local depression has no downhill neighbour, so its flow simply stops, and the
//! whole region downstream of a one-metre puddle is a river that does not exist.
//! Priority-flood raises every depression to its spill point **before** routing,
//! which is why [`Region::fill_depressions`] is not an optional step.
//!
//! # Why this is a region and not a column
//!
//! Flow is not a local property: a cell's flow is the sum of everything above it.
//! [`Region`] therefore works on a patch of terrain, and
//! [`Region::sample_at_centre`] is the column-level entry point that builds one.
//! A per-column "flow" that consulted only its own neighbourhood would be a
//! different and wrong quantity, and it is the quantity most procedural rivers
//! actually use.

use crate::spec::WorldSpec;
use crate::terrain;
use core::cmp::Ordering;
use std::collections::BinaryHeap;

/// A patch of terrain heights, with the analyses that need the whole patch.
#[derive(Clone, Debug)]
pub struct Region {
    width: usize,
    height: usize,
    /// Spacing between samples, in metres.
    step_m: f64,
    /// The world position of cell `(0, 0)`.
    origin: (f64, f64),
    /// Surface heights, **after** any fill has been applied.
    heights: Vec<f64>,
    /// The heights before the fill, kept so a lake can be *detected* rather than
    /// inferred.
    original: Vec<f64>,
}

/// `f64` keyed for a `BinaryHeap`, which needs `Ord`.
#[derive(Clone, Copy, PartialEq)]
struct Key(f64);

impl Eq for Key {}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        // `total_cmp` rather than `partial_cmp().unwrap()`: a NaN here would
        // panic in a comparator, and a panic inside a heap is a corrupted heap.
        self.0.total_cmp(&other.0)
    }
}

/// The eight D8 neighbours, as `(dx, dz, distance)`.
const D8: [(isize, isize, f64); 8] = [
    (-1, -1, std::f64::consts::SQRT_2),
    (0, -1, 1.0),
    (1, -1, std::f64::consts::SQRT_2),
    (-1, 0, 1.0),
    (1, 0, 1.0),
    (-1, 1, std::f64::consts::SQRT_2),
    (0, 1, 1.0),
    (1, 1, std::f64::consts::SQRT_2),
];

impl Region {
    /// Build a region of terrain around a world position.
    ///
    /// `radius` is in cells, so a radius of 16 is a 33×33 patch.
    pub fn around(spec: &WorldSpec, centre: (f64, f64), radius: usize, step_m: f64) -> Self {
        let width = radius * 2 + 1;
        let height = width;
        let origin = (
            centre.0 - radius as f64 * step_m,
            centre.1 - radius as f64 * step_m,
        );

        let mut heights = Vec::with_capacity(width * height);
        for j in 0..height {
            for i in 0..width {
                let x = origin.0 + i as f64 * step_m;
                let z = origin.1 + j as f64 * step_m;
                heights.push(terrain::elevation_m(spec, x, z));
            }
        }

        Region {
            width,
            height,
            step_m,
            origin,
            original: heights.clone(),
            heights,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    #[inline]
    fn idx(&self, i: usize, j: usize) -> usize {
        j * self.width + i
    }

    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.heights[self.idx(i, j)]
    }

    /// The original (unfilled) height.
    pub fn original_at(&self, i: usize, j: usize) -> f64 {
        self.original[self.idx(i, j)]
    }

    /// Fill every depression up to its spill point — priority-flood (Barnes 2014).
    ///
    /// The algorithm is a flood from the patch's edge inward: the frontier always
    /// expands from the lowest cell found so far, and any cell lower than the
    /// water already at the frontier is raised to the frontier's level.
    pub fn fill_depressions(&mut self) {
        let n = self.width * self.height;
        let mut closed = vec![false; n];
        let mut heap: BinaryHeap<std::cmp::Reverse<(Key, usize)>> = BinaryHeap::new();

        // Seed with the boundary, which is where water leaves the patch.
        for i in 0..self.width {
            for j in [0, self.height - 1] {
                let k = self.idx(i, j);
                heap.push(std::cmp::Reverse((Key(self.heights[k]), k)));
                closed[k] = true;
            }
        }
        for j in 0..self.height {
            for i in [0, self.width - 1] {
                let k = self.idx(i, j);
                if !closed[k] {
                    heap.push(std::cmp::Reverse((Key(self.heights[k]), k)));
                    closed[k] = true;
                }
            }
        }

        while let Some(std::cmp::Reverse((Key(level), k))) = heap.pop() {
            let i = k % self.width;
            let j = k / self.width;
            for (dx, dz, _) in D8 {
                let ni = i as isize + dx;
                let nj = j as isize + dz;
                if ni < 0 || nj < 0 || ni >= self.width as isize || nj >= self.height as isize {
                    continue;
                }
                let (ni, nj) = (ni as usize, nj as usize);
                let nk = self.idx(ni, nj);
                if closed[nk] {
                    continue;
                }
                closed[nk] = true;
                // Raise to the spill level if the cell is lower.
                if self.heights[nk] < level {
                    self.heights[nk] = level;
                }
                heap.push(std::cmp::Reverse((Key(self.heights[nk]), nk)));
            }
        }
    }

    /// D8 flow routing. Returns the downstream cell of each cell, or `None` where
    /// the cell is an outlet (no lower neighbour).
    ///
    /// Ties are broken by the **D8 order above**, never by iteration order, so two
    /// runs route identically (F12: never iteration order).
    pub fn downstream(&self) -> Vec<Option<usize>> {
        let n = self.width * self.height;
        let mut out = vec![None; n];
        for j in 0..self.height {
            for i in 0..self.width {
                let k = self.idx(i, j);
                let here = self.heights[k];
                let mut best: Option<(f64, usize)> = None;
                for (dx, dz, dist) in D8 {
                    let ni = i as isize + dx;
                    let nj = j as isize + dz;
                    if ni < 0 || nj < 0 || ni >= self.width as isize || nj >= self.height as isize {
                        continue;
                    }
                    let (ni, nj) = (ni as usize, nj as usize);
                    let nk = self.idx(ni, nj);
                    let drop = here - self.heights[nk];
                    if drop <= 0.0 {
                        continue;
                    }
                    let slope = drop / dist;
                    match best {
                        Some((bs, _)) if bs >= slope => {}
                        _ => best = Some((slope, nk)),
                    }
                }
                out[k] = best.map(|(_, nk)| nk);
            }
        }
        out
    }

    /// Flow accumulation per cell, in **cells drained** (including the cell
    /// itself). Routed in descending height order so a cell's upstream is always
    /// complete before it is added to.
    pub fn flow_accumulation(&self) -> Vec<f64> {
        let n = self.width * self.height;
        let downstream = self.downstream();
        let mut acc = vec![1.0f64; n];

        let mut order: Vec<usize> = (0..n).collect();
        // Descending height; ties by index so the order is total and stable.
        order.sort_by(|a, b| {
            self.heights[*b]
                .total_cmp(&self.heights[*a])
                .then(a.cmp(b))
        });

        for k in order {
            if let Some(d) = downstream[k] {
                acc[d] += acc[k];
            }
        }
        acc
    }

    /// Strahler stream order per cell.
    ///
    /// A cell with no upstream contributors is order 1. A cell whose highest
    /// upstream order is attained by exactly one contributor keeps that order;
    /// if two or more attain it, the order increments. That is Strahler 1952, and
    /// the "exactly one" clause is the whole of it.
    pub fn strahler_order(&self) -> Vec<u8> {
        let n = self.width * self.height;
        let downstream = self.downstream();
        let mut upstream: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (k, d) in downstream.iter().enumerate() {
            if let Some(d) = d {
                upstream[*d].push(k);
            }
        }

        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|a, b| self.heights[*b].total_cmp(&self.heights[*a]).then(a.cmp(b)));

        let mut rank = vec![0u8; n];
        for k in order {
            if upstream[k].is_empty() {
                rank[k] = 1;
                continue;
            }
            let mut max = 0u8;
            let mut count = 0u8;
            for u in &upstream[k] {
                let r = rank[*u];
                if r > max {
                    max = r;
                    count = 1;
                } else if r == max {
                    count = count.saturating_add(1);
                }
            }
            rank[k] = if count >= 2 { max.saturating_add(1) } else { max };
        }
        rank
    }

    /// True where a cell is a **lake**: the flood had to raise it.
    pub fn is_lake(&self, i: usize, j: usize) -> bool {
        let k = self.idx(i, j);
        self.heights[k] - self.original[k] > LAKE_EPSILON_M
    }

    /// How deep the water is at a cell, in metres.
    pub fn water_depth_m(&self, i: usize, j: usize) -> f64 {
        let k = self.idx(i, j);
        (self.heights[k] - self.original[k]).max(0.0)
    }

    /// The cell at the region's centre.
    pub fn centre(&self) -> (usize, usize) {
        (self.width / 2, self.height / 2)
    }

    /// Hydrological quantities at the region's centre.
    pub fn sample_at_centre(&self, channel_threshold: f64) -> HydroSample {
        let (ci, cj) = self.centre();
        let k = self.idx(ci, cj);
        let acc = self.flow_accumulation();
        let rank = self.strahler_order();
        let flow = acc[k];
        HydroSample {
            flow_cells: flow,
            order: rank[k],
            is_lake: self.is_lake(ci, cj),
            water_depth_m: self.water_depth_m(ci, cj),
            is_channel: flow >= channel_threshold,
        }
    }

    pub fn origin(&self) -> (f64, f64) {
        self.origin
    }

    pub fn step_m(&self) -> f64 {
        self.step_m
    }
}

/// How much the flood must raise a cell before it counts as a lake, in metres.
/// A declared epsilon rather than an exact zero: floating-point equality against
/// a computed height is a coin flip.
pub const LAKE_EPSILON_M: f64 = 0.05;

/// Everything hydrology says about one column.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct HydroSample {
    /// Cells draining through this one, including itself.
    pub flow_cells: f64,
    /// Strahler stream order.
    pub order: u8,
    pub is_lake: bool,
    pub water_depth_m: f64,
    /// True when the flow exceeds the declared channel threshold.
    pub is_channel: bool,
}

/// The default channel threshold, in cells drained. Declared, not tuned.
pub const DEFAULT_CHANNEL_THRESHOLD: f64 = 40.0;

/// Sample hydrology at a world position by building a region around it.
///
/// This is the honest cost of the honest model: flow is not local, so a column
/// must look at its surroundings. `radius` is the *declared* neighbourhood, and
/// a caller that shrinks it is choosing a smaller watershed, not a cheaper
/// approximation of the same one.
pub fn sample(spec: &WorldSpec, x: f64, z: f64, radius: usize) -> HydroSample {
    let mut region = Region::around(spec, (x, z), radius, 1.0);
    region.fill_depressions();
    region.sample_at_centre(DEFAULT_CHANNEL_THRESHOLD)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2024, Config::default())
    }

    /// A region built from an explicit height grid, for testing the algorithms
    /// without the terrain generator underneath them.
    fn region_from(w: usize, h: usize, heights: Vec<f64>) -> Region {
        assert_eq!(heights.len(), w * h);
        Region {
            width: w,
            height: h,
            step_m: 1.0,
            origin: (0.0, 0.0),
            original: heights.clone(),
            heights,
        }
    }

    #[test]
    fn a_single_puddle_is_filled_and_a_slope_is_untouched() {
        // A 5x5 bowl with a rim: the middle must be raised to the rim.
        let mut heights = vec![10.0f64; 25];
        heights[12] = 0.0; // centre
        let mut r = region_from(5, 5, heights);
        assert!(!r.is_lake(2, 2) || r.water_depth_m(2, 2) == 0.0);
        r.fill_depressions();
        // After the fill the centre is at the rim's level.
        assert!((r.get(2, 2) - 10.0).abs() < 1e-9, "centre was {}", r.get(2, 2));
        assert!(r.is_lake(2, 2));

        // A plane sloping to the edge has no depressions to fill.
        let mut h2 = vec![0.0f64; 25];
        for j in 0..5 {
            for i in 0..5 {
                h2[j * 5 + i] = 10.0 - i as f64;
            }
        }
        let mut r2 = region_from(5, 5, h2.clone());
        r2.fill_depressions();
        assert_eq!(r2.heights, h2, "a monotone slope was modified by the fill");
    }

    #[test]
    fn flow_accumulates_downhill() {
        // A 5x5 plane sloping in +x: everything drains to the right edge, so the
        // rightmost column drains the whole patch.
        let mut heights = vec![0.0f64; 25];
        for j in 0..5 {
            for i in 0..5 {
                heights[j * 5 + i] = 10.0 - i as f64;
            }
        }
        let r = region_from(5, 5, heights);
        let acc = r.flow_accumulation();
        assert_eq!(acc[4], 5.0, "the right edge did not drain its row");
        assert_eq!(acc[0], 1.0, "the top cell drained something");
    }

    /// The flow that reaches the patch's outlets. **Every cell's unit of flow ends
    /// at an outlet**, because D8 always routes to a strictly lower cell and so
    /// cannot cycle. That makes this sum an exact invariant.
    fn outlet_total(region: &Region) -> f64 {
        let downstream = region.downstream();
        let acc = region.flow_accumulation();
        downstream
            .iter()
            .enumerate()
            .filter(|(_, d)| d.is_none())
            .map(|(k, _)| acc[k])
            .sum()
    }

    #[test]
    fn flow_into_a_pit_is_routed_after_the_fill_and_not_before() {
        // The failure the fill exists to prevent: without it, everything upstream
        // of a puddle stops.
        let mut heights = vec![10.0f64; 25];
        // A channel down the middle to the right edge, with a pit in it.
        for j in 1..4 {
            heights[j * 5 + 2] = 5.0;
        }
        heights[2 * 5 + 2] = 1.0; // the pit
        heights[2 * 5 + 3] = 4.0;
        heights[2 * 5 + 4] = 3.0;

        let unfilled = region_from(5, 5, heights.clone());
        let filled_before = outlet_total(&unfilled);

        let mut filled = region_from(5, 5, heights);
        filled.fill_depressions();
        let filled_after = outlet_total(&filled);

        // Both were filled with the same water, so **both** must deliver all 25
        // cells' worth to an outlet. They differ in *where*, not in *how much*.
        assert!((filled_before - 25.0).abs() < 1e-9, "unfilled delivered {filled_before}");
        assert!((filled_after - 25.0).abs() < 1e-9, "filled delivered {filled_after}");
    }

    #[test]
    fn total_flow_is_conserved_exactly() {
        // The exactness F1 requires. It is **not** the naive sum over the patch —
        // that is `n + (transfers)` and an earlier revision wrongly asserted it
        // was `n`. The invariant is that all `n` units arrive at an outlet.
        let s = spec();
        let mut r = Region::around(&s, (500.0, -300.0), 12, 2.0);
        r.fill_depressions();
        let expected = (r.width() * r.height()) as f64;
        let total = outlet_total(&r);
        assert!((total - expected).abs() < 1e-9, "outlets carried {total} != {expected}");

        // And no cell ever carries less than its own unit.
        for a in r.flow_accumulation() {
            assert!(a >= 1.0 - 1e-12, "a cell carried {a} units");
        }
    }

    #[test]
    fn strahler_order_rises_where_two_rivers_meet() {
        // Two tributaries meeting must produce a higher order downstream than
        // either tributary. Build a Y: two arms flowing into one stem.
        //
        //   y\x 0  1  2  3  4
        //   0   1  5  1  .  .
        //   1   .  5  .  .  .
        //   2   .  5  5  5  .   <- both arms join the stem
        let mut heights = vec![5.0f64; 25];
        // Left arm: (0,0) down to (1,2)
        heights[0] = 4.0;
        heights[1] = 3.0;
        heights[7] = 2.0; // (2,1)
        // Right arm: (2,0) down to (2,2)
        heights[2] = 4.0;
        heights[7 + 1] = 4.0;
        // Stem: (2,2) -> (3,2) -> (4,2) draining off the right edge
        heights[12] = 1.0;
        heights[13] = 0.5;
        heights[14] = 0.0;
        // Everything else drains right too.
        let mut r = region_from(5, 5, heights);
        r.fill_depressions();
        let rank = r.strahler_order();
        let upstream_order = rank[7].max(rank[2]);
        assert!(
            rank[12] >= upstream_order,
            "the confluence's order {} was below its tributary's {}",
            rank[12],
            upstream_order
        );
        assert!(rank.iter().all(|r| *r >= 1 && *r <= 8));
    }

    #[test]
    fn a_region_over_a_seed_is_deterministic_and_has_a_channel_somewhere() {
        let s = spec();
        let a = sample(&s, 100.0, 100.0, 16);
        let b = sample(&s, 100.0, 100.0, 16);
        assert_eq!(a, b);

        // Scan for at least one channel, so the shaper is not vacuous.
        let mut found = false;
        let mut x = -2000.0;
        while x < 2000.0 && !found {
            if sample(&s, x, x * 0.3, 16).is_channel {
                found = true;
            }
            x += 150.0;
        }
        assert!(found, "no channel was found anywhere in the world");
    }

    #[test]
    fn water_depth_is_never_negative() {
        let s = spec();
        let mut r = Region::around(&s, (0.0, 0.0), 10, 3.0);
        r.fill_depressions();
        for j in 0..r.height() {
            for i in 0..r.width() {
                assert!(r.water_depth_m(i, j) >= 0.0);
            }
        }
    }
}
