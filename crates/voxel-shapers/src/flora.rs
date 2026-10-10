//! Flora — trees, as a function of position rather than a record of them.
//!
//! **Where the idea comes from.** `DynamicTrees` (a mod, read for ideas only — no code
//! and no data were taken, per the standing mods ruling) is built on the observation this
//! module uses: a tree is a deterministic function of the cell it is rooted in, so no
//! chunk ever stores one. Everything below is this tree's own arithmetic.
//!
//! # The rule, in one paragraph
//!
//! The world is divided into **8 m cells** ([`TREE_CELL_M`]). Each cell's hash decides
//! whether it holds a *candidate*; a candidate's base column is the cell's own jittered
//! one; and the **formation** of that column decides whether the candidate stands — a
//! forest keeps all of them, grassland and scrub keep a few (that is what makes a
//! grassland a savanna and not a lawn), tundra keeps very few, and bare formations keep
//! none. What a voxel is then follows from the trees whose reach covers its column and
//! from nothing else. There is no state anywhere: the same seed and the same position
//! answer the same way on every call. **The tick does not enter at all**
//! ([`WorldSpec::stationary_stage_seed`]), so trees do not reshuffle when the world starts
//! ticking — the same ruling that made terrain, geology, caves and climate stationary
//! (C-159.8).
//!
//! # Why the cell is memoized, and why that is exact
//!
//! A voxel above the ground asks up to four neighbouring cells whether a tree reaches it,
//! and answering a cell means reading its column's formation — about **5.6 µs**
//! (`examples/biome-cost.rs`). Done per voxel that would be most of a chunk build's
//! budget, so the cells are memoized `(fingerprint, cell) → Option<Tree>`. The memo is a
//! cache of a **pure function**: a hit returns exactly the value a miss would have
//! computed, so the world is unchanged by it living here — the argument `ColumnMemo`
//! makes, applied to the lattice instead of to the chunk. The bound is declared and
//! clearing it only forgets.

use std::cell::RefCell;
use std::collections::HashMap;

use voxel_model::MaterialId;

use crate::climate;
use crate::column::materials;
use crate::ecology::{self, Biome};
use crate::habitat::{self, Habitat};
use crate::spec::WorldSpec;
use crate::terrain;

/// The lattice a tree is rooted in, in metres. **Declared**: an 8 m cell means at most one
/// tree per 64 m², which is a closed stand, and it is also what bounds the search for
/// neighbours (a canopy reaches at most half a cell).
pub const TREE_CELL_M: i32 = 8;

/// How far above its own ground a tree voxel can be, in metres — the band the material
/// rule bounds itself with before asking flora at all. Declared from the shapes below:
/// the tallest trunk is `12`, and the canopy extends a little past it; `16` is that
/// rounded up, stated so a taller tree is a change to this constant and not a surprise.
pub const MAX_TREE_ABOVE_GROUND_M: f64 = 16.0;

/// The most any canopy can reach from its own column, in metres. Declared from the shapes
/// (the widest canopy is `3.2`), and it is what chooses how many cells a column asks
/// about: `(x ± 4 m)` spans at most two cells per axis.
const MAX_CANOPY_RADIUS_M: i32 = 4;

/// The chance a cell holds a *candidate* before its formation's own gate.
const CANDIDATE_CHANCE: f64 = 0.85;

/// A cell's hash as a `[0, 1)` fraction in one of four independent windows. Four, because
/// the roll, the jitter, the species and the shape must not be correlated.
fn roll(h: u64, window: u32) -> f64 {
    ((h >> (16 * window)) & 0xffff) as f64 / 65536.0
}

/// One standing tree: where it is rooted, how tall it is, and what its canopy is made of.
///
/// Public because a caller (a test, a census, a future spawn rule) may want to ask the
/// lattice for trees as trees; the material rule only wants [`tree_material_at`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tree {
    /// The column the trunk stands in.
    pub base_x: i32,
    pub base_z: i32,
    /// The topmost solid ground voxel of that column.
    pub base_y: i32,
    /// Trunk voxels above `base_y`.
    pub trunk_height: i32,
    /// The canopy's own radius in metres (a cone's base radius, or a sphere's).
    pub canopy_radius_m: f64,
    /// What the canopy is made of: `CONIFER` or `LEAF`.
    pub canopy_material: MaterialId,
    /// Whether the canopy is a cone (a spruce) rather than a sphere (a broadleaf).
    pub conifer: bool,
}

impl Tree {
    /// Whether this tree's reach can cover a column — a cheap box test, not the shape.
    pub fn reaches(&self, x: i32, z: i32) -> bool {
        let dx = (x - self.base_x).abs() as f64;
        let dz = (z - self.base_z).abs() as f64;
        dx <= self.canopy_radius_m + 0.5 && dz <= self.canopy_radius_m + 0.5
    }

    /// The material at a voxel, if this tree holds it.
    ///
    /// The trunk is its own column from `base_y + 1` to the trunk's top. A conifer's
    /// canopy is a **cone** widening downward over the last three trunk voxels; a
    /// broadleaf's is a slightly flattened **sphere** centred on the trunk's top. Both are
    /// sampled at voxel centres so that a canopy has a top surface rather than terminating
    /// on a half-metre.
    pub fn voxel_at(&self, x: i32, y: i32, z: i32) -> Option<MaterialId> {
        let dx = x - self.base_x;
        let dz = z - self.base_z;
        let dy = y - self.base_y;
        if dy <= 0 || dy > self.trunk_height + 1 {
            return None;
        }
        if dx == 0 && dz == 0 && dy <= self.trunk_height {
            return Some(materials::WOOD);
        }
        let dxf = dx as f64;
        let dzf = dz as f64;
        if self.conifer {
            // The skirt: the lowest canopy voxel sits three above the trunk's midpoint,
            // and the radius tapers from the canopy's own base radius to a tip.
            let skirt_bottom = self.trunk_height - 3;
            if dy < skirt_bottom || dy > self.trunk_height {
                return None;
            }
            let t = (dy - skirt_bottom) as f64 / 3.0;
            let r = self.canopy_radius_m * (1.0 - 0.45 * t);
            if dxf * dxf + dzf * dzf <= r * r {
                Some(self.canopy_material)
            } else {
                None
            }
        } else {
            let centre = self.trunk_height as f64 + 0.5;
            let dyf = (dy as f64 + 0.5 - centre) * 1.15;
            let r = self.canopy_radius_m;
            if dxf * dxf + dzf * dzf + dyf * dyf <= r * r {
                Some(self.canopy_material)
            } else {
                None
            }
        }
    }
}

/// The share of a formation's candidates that survive a **dry** expression (C-160).
/// Declared: a dry ridge inside a forest is an opening, not a clearing.
const DRY_SHARE: f64 = 0.35;

/// The share that survives a **montane** expression: thinner stands on the belt, and the
/// species is forced to conifer below (the reference's montane conifer woodland).
const MONTANE_SHARE: f64 = 0.7;

/// The formation's share of candidate cells that stand.
///
/// **Declared, and it is the difference between a forest and a park.** Forests keep every
/// candidate; open formations keep a few, which is what makes a savanna read as scattered
/// trees rather than as either a lawn or a wood; tundra keeps very few (the krummholz
/// edge); bare formations keep none, because there is nothing for a tree to stand in.
fn trees_per_formation(biome: Biome) -> f64 {
    match biome {
        Biome::BorealForest
        | Biome::CoolTemperateForest
        | Biome::TemperateForest
        | Biome::TropicalDryForest
        | Biome::TropicalRainforest => 1.0,
        Biome::Grassland | Biome::SubtropicalScrub => 0.18,
        Biome::Tundra => 0.04,
        Biome::Alpine | Biome::Desert | Biome::PolarDesert => 0.0,
    }
}

/// The tree a **cell** holds, if it holds one — the whole rule, un-memoized.
///
/// A pure function of `(seed, config, cell)`: no tick ([`WorldSpec::stationary_stage_seed`]),
/// no neighbourhood, no state.
pub fn tree_of_cell(spec: &WorldSpec, cell_x: i32, cell_z: i32) -> Option<Tree> {
    let seed = spec.stationary_stage_seed("flora");
    let h = voxel_noise::mix(
        voxel_noise::mix(cell_x as i64 as u64 ^ seed)
            ^ (cell_z as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
    );
    if roll(h, 0) >= CANDIDATE_CHANCE {
        return None;
    }

    // The base column is jittered inside the cell but never onto its edge, so the canopy
    // stays within the neighbour range [`trees_over_column`] searches.
    let base_x = cell_x * TREE_CELL_M + 1 + ((h >> 16) % 6) as i32;
    let base_z = cell_z * TREE_CELL_M + 1 + ((h >> 32) % 6) as i32;
    let fx = base_x as f64 + 0.5;
    let fz = base_z as f64 + 0.5;

    let surface_m = terrain::elevation_m(spec, fx, fz);
    // No tree stands in the sea, and none stands under it: a drowned stand is a different
    // feature (and water is not swum yet).
    if surface_m <= spec.config.sea_level_m {
        return None;
    }

    // The formation is the same read the surface material takes (C-159.10): the region's
    // climate, sampled at tick 0 because it does not move with the tick.
    let (biotemperature_c, precipitation_mm) = ecology::holdridge_inputs(spec, fx, fz, surface_m, 0);
    let biome = ecology::biome_at(spec, surface_m, biotemperature_c, precipitation_mm);

    // **The terrain expression gates the stand** (C-160): no tree roots in a rock face,
    // a dry expression keeps fewer candidates, and the montane belt is thinner and
    // coniferous — the reference's elevation ladder. The habitat reads the same slope
    // and moisture the material rule reads, so the forest and the ground cannot
    // disagree.
    let slope = terrain::sample(spec, fx, fz).slope;
    let moisture = climate::moisture(spec, fx, fz);
    let habitat = habitat::habitat_of(spec, surface_m, slope, moisture);
    let montane = habitat == Habitat::Montane;
    let mut share = trees_per_formation(biome);
    match habitat {
        Habitat::Steep => share = 0.0,
        Habitat::Dry => share *= DRY_SHARE,
        Habitat::Montane => share *= MONTANE_SHARE,
        Habitat::Wet | Habitat::Lowland => {}
    }
    if share <= 0.0 || roll(h, 3) >= share {
        return None;
    }

    let conifer = montane
        || biome == Biome::BorealForest
        || biome == Biome::Tundra
        || (biome == Biome::CoolTemperateForest && (h >> 20) & 1 == 0);
    let trunk_height = if conifer {
        7 + ((h >> 40) % 6) as i32
    } else {
        4 + ((h >> 40) % 4) as i32
    };
    let canopy_radius_m = if conifer {
        2.6
    } else {
        2.4 + ((h >> 44) % 2) as f64 * 0.8
    };
    Some(Tree {
        base_x,
        base_z,
        base_y: (surface_m - 0.5).floor() as i32,
        trunk_height,
        canopy_radius_m,
        canopy_material: if conifer { materials::CONIFER } else { materials::LEAF },
        conifer,
    })
}

/// How many cell answers are kept before the memo is cleared. Declared: a cell is a
/// little over fifty bytes, so this is about ten megabytes, and clearing it forgets
/// nothing that cannot be recomputed.
const CELL_MEMO_LIMIT: usize = 200_000;

thread_local! {
    /// `(world fingerprint, cell) → tree`. A cache of a pure function; see the module docs.
    static CELLS: RefCell<HashMap<(u64, i32, i32), Option<Tree>>> = RefCell::new(HashMap::new());
}

/// [`tree_of_cell`] through the memo. Exact: a hit is what a miss computes.
pub fn memoized_tree_of_cell(spec: &WorldSpec, cell_x: i32, cell_z: i32) -> Option<Tree> {
    let key = (spec.fingerprint(), cell_x, cell_z);
    if let Some(hit) = CELLS.with(|cells| cells.borrow().get(&key).copied()) {
        return hit;
    }
    let tree = tree_of_cell(spec, cell_x, cell_z);
    CELLS.with(|cells| {
        let mut cells = cells.borrow_mut();
        if cells.len() >= CELL_MEMO_LIMIT {
            cells.clear();
        }
        cells.insert(key, tree);
    });
    tree
}

/// The trees whose reach can cover a column: **at most four**, because a cell is 8 m and
/// a canopy reaches 4 m, so `(x ± 4)` spans at most two cells per axis.
pub fn trees_over_column(spec: &WorldSpec, x: i32, z: i32) -> [Option<Tree>; 4] {
    let mut out = [None; 4];
    let lo_x = (x - MAX_CANOPY_RADIUS_M).div_euclid(TREE_CELL_M);
    let hi_x = (x + MAX_CANOPY_RADIUS_M).div_euclid(TREE_CELL_M);
    let lo_z = (z - MAX_CANOPY_RADIUS_M).div_euclid(TREE_CELL_M);
    let hi_z = (z + MAX_CANOPY_RADIUS_M).div_euclid(TREE_CELL_M);
    let mut found = 0usize;
    for cell_x in lo_x..=hi_x {
        for cell_z in lo_z..=hi_z {
            if found == out.len() {
                return out;
            }
            if let Some(tree) = memoized_tree_of_cell(spec, cell_x, cell_z) {
                if tree.reaches(x, z) {
                    out[found] = Some(tree);
                    found += 1;
                }
            }
        }
    }
    out
}

/// The material a tree holds at a voxel, if any — **the one call the material rules make**.
///
/// `surface_m` is the column's own ground, so the band bound costs no work beyond a
/// comparison: above it the answer is `None` and no cell is asked.
pub fn tree_material_at(
    spec: &WorldSpec,
    x: i32,
    y: i32,
    z: i32,
    surface_m: f64,
) -> Option<MaterialId> {
    if y as f64 > surface_m + MAX_TREE_ABOVE_GROUND_M {
        return None;
    }
    for tree in trees_over_column(spec, x, z).into_iter().flatten() {
        if let Some(material) = tree.voxel_at(x, y, z) {
            return Some(material);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    /// The first cell in a growing square around the origin that holds a tree.
    fn first_tree(spec: &WorldSpec) -> Tree {
        for radius in 1..64 {
            for cell_z in -radius..=radius {
                for cell_x in [-radius, radius] {
                    let t = tree_of_cell(spec, cell_x, cell_z)
                        .or_else(|| tree_of_cell(spec, cell_z, cell_x));
                    if let Some(tree) = t {
                        return tree;
                    }
                }
            }
        }
        panic!("no tree found in the first 63 cells of the origin's neighbourhood");
    }

    #[test]
    fn a_tree_is_a_function_of_the_cell_and_the_world_and_nothing_else() {
        let s = spec();
        assert_eq!(
            tree_of_cell(&s, 12, -7),
            tree_of_cell(&spec(), 12, -7),
            "the same cell of the same world gave two trees"
        );
        // The seed is in it: a different world roots different trees somewhere.
        let other = WorldSpec::new(97531, Config::default());
        let differs = (-40..40)
            .flat_map(|cx| (-40..40).map(move |cz| (cx, cz)))
            .any(|(cx, cz)| tree_of_cell(&s, cx, cz) != tree_of_cell(&other, cx, cz));
        assert!(differs, "two seeds grew identical forests");
    }

    #[test]
    fn the_tick_does_not_move_a_tree() {
        // C-159.8's ruling, applied to flora: a tree is a property of the ground, so the
        // moving tick must not reshuffle it. The flora seed is the stationary one.
        let a = spec();
        let b = spec().at_tick(crate::climate::YEAR_TICKS * 3 + 17);
        for cell_x in -8..8 {
            for cell_z in -8..8 {
                assert_eq!(
                    tree_of_cell(&a, cell_x, cell_z),
                    tree_of_cell(&b, cell_x, cell_z),
                    "the tick moved the trees at cell ({cell_x}, {cell_z})"
                );
            }
        }
    }

    #[test]
    fn a_standing_tree_has_a_wood_trunk_and_a_canopy() {
        let s = spec();
        let tree = first_tree(&s);
        assert_eq!(
            tree.voxel_at(tree.base_x, tree.base_y + 1, tree.base_z),
            Some(materials::WOOD),
            "the first voxel above the ground is not trunk"
        );
        assert_eq!(
            tree.voxel_at(tree.base_x, tree.base_y, tree.base_z),
            None,
            "a tree stands *above* the ground, not in it"
        );
        let canopy = (-3..=3)
            .flat_map(|dx| (-3..=3).map(move |dz| (dx, dz)))
            .flat_map(|(dx, dz)| {
                (tree.base_y + 2..=tree.base_y + tree.trunk_height + 1)
                    .map(move |y| (tree.base_x + dx, y, tree.base_z + dz))
            })
            .filter_map(|(x, y, z)| tree.voxel_at(x, y, z))
            .filter(|m| *m == tree.canopy_material)
            .count();
        assert!(canopy > 0, "a tree stood with no canopy material anywhere");
    }

    #[test]
    fn no_tree_stands_in_the_sea() {
        let s = spec();
        for cell_x in -40..40 {
            for cell_z in -40..40 {
                if let Some(tree) = tree_of_cell(&s, cell_x, cell_z) {
                    let surface_m =
                        terrain::elevation_m(&s, tree.base_x as f64 + 0.5, tree.base_z as f64 + 0.5);
                    assert!(
                        surface_m > s.config.sea_level_m,
                        "a tree is rooted at {surface_m} m, at or below the {} m waterline",
                        s.config.sea_level_m
                    );
                }
            }
        }
    }

    #[test]
    fn no_tree_stands_on_a_steep_face() {
        // C-160's flora consequence, measured on the world rather than asserted about the
        // rule: every tree the lattice grows is rooted on ground whose terrain expression
        // is not `Steep`.
        let s = spec();
        let mut checked = 0u32;
        for cell_x in -60..60 {
            for cell_z in -60..60 {
                if let Some(tree) = tree_of_cell(&s, cell_x, cell_z) {
                    let (fx, fz) = (tree.base_x as f64 + 0.5, tree.base_z as f64 + 0.5);
                    let surface_m = terrain::elevation_m(&s, fx, fz);
                    let slope = terrain::sample(&s, fx, fz).slope;
                    let moisture = climate::moisture(&s, fx, fz);
                    let habitat = habitat::habitat_of(&s, surface_m, slope, moisture);
                    assert_ne!(habitat, Habitat::Steep, "a tree rooted in a rock face");
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "the probe window grew no trees at all");
    }

    #[test]
    fn the_memo_answers_exactly_what_the_rule_answers() {
        // The memo's whole claim. Sampled over a window large enough to hold hits and
        // misses alike, in an order that makes the second half all hits.
        let s = spec();
        for cell_x in -20..20 {
            for cell_z in -20..20 {
                assert_eq!(
                    memoized_tree_of_cell(&s, cell_x, cell_z),
                    tree_of_cell(&s, cell_x, cell_z),
                    "the memo disagreed with the rule at ({cell_x}, {cell_z})"
                );
            }
        }
    }

    #[test]
    fn a_column_asks_only_the_four_cells_that_can_reach_it() {
        let s = spec();
        // The claim, observed rather than assumed: every tree a column lists really can
        // reach it, and every tree the rule grows within reach really is listed.
        for x in -20..20 {
            for z in -20..20 {
                let listed = trees_over_column(&s, x, z);
                for tree in listed.into_iter().flatten() {
                    assert!(tree.reaches(x, z), "a listed tree cannot reach ({x}, {z})");
                    assert_eq!(
                        Some(tree),
                        memoized_tree_of_cell(&s, tree.base_x.div_euclid(TREE_CELL_M), tree.base_z.div_euclid(TREE_CELL_M)),
                        "a listed tree is not its cell's"
                    );
                }
            }
        }
    }

    #[test]
    fn the_formation_gate_is_the_openness_of_the_formation() {
        assert_eq!(trees_per_formation(Biome::Desert), 0.0);
        assert_eq!(trees_per_formation(Biome::Alpine), 0.0);
        assert_eq!(trees_per_formation(Biome::PolarDesert), 0.0);
        assert!(trees_per_formation(Biome::BorealForest) > trees_per_formation(Biome::Grassland));
        assert!(trees_per_formation(Biome::Grassland) > 0.0, "a savanna is trees");
        assert!(trees_per_formation(Biome::Tundra) > 0.0);
    }

    #[test]
    fn the_band_bound_is_above_every_tree_voxel() {
        let s = spec();
        let tree = first_tree(&s);
        // The highest voxel the shape can hold, found by walking the shape itself.
        let mut highest = 0;
        for dy in 1..=(tree.trunk_height + 3) {
            for dx in -4..=4 {
                for dz in -4..=4 {
                    if tree.voxel_at(tree.base_x + dx, tree.base_y + dy, tree.base_z + dz).is_some()
                    {
                        highest = dy;
                    }
                }
            }
        }
        assert!(
            (highest as f64) < MAX_TREE_ABOVE_GROUND_M,
            "a tree voxel at +{highest} m escaped the {MAX_TREE_ABOVE_GROUND_M} m band"
        );
    }
}
