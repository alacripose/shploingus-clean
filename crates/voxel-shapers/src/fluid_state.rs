//! Where the world's water **is**, as fluid state — the ocean, the lakes, and the
//! flooded karst.
//!
//! # What this is, and what it is not
//!
//! It is a **query**, not a store: [`fluid_at`] is a pure function of the world
//! spec and the address, derived from the same laws that already place water —
//! [`chunk_fields`]'s material path for the ocean and the flooded voids, and
//! [`hydrology`]'s priority-flood depth for the lakes — and **nothing is kept
//! between calls** (C-2). It is:
//!
//! * the state the **edit path refuses through** (C-152.2's filed defect: *"for
//!   some reason I can break water blocks"* — an edit that targets a fluid refuses,
//!   and a *lake* has no material voxel, so the refusal has to ask for the fluid,
//!   not the material);
//! * the vocabulary the appearance layer draws from: [`FillLevel`] is the fill
//!   interface the user named (*"we can use the … base subvoxels for interpolating
//!   fluids within blocks"*, C-153.2), resolved at the drawn scale — `FILL_LEVELS` a
//!   block, one per 31.25 mm, 32 per block axis — by their ruling of 2026-10-05
//!   (*"I like the 32x per side of block, so if there's stuff happening below that,
//!   then I don't want it"*, C-164).
//!
//! It is **not** the fluidics lattice: [`crate::fluidics`] moves fluid between
//! porous cells, and this says which fluid occupies a world voxel. The two meet at
//! the fill level and at [`voxel_model::matter`]'s species/phase roster; a moving
//! water table is the lattice's state, and this query is what it will be asked for.
//!
//! # Salinity is not here
//!
//! [`crate::water`]'s ledger splits water into fresh and saline **reservoirs**, and
//! this query does not repeat that split: a voxel of the sea is saline water, a
//! lake is fresh, and a *component* split (salt as dissolved content, H-23.4) is
//! the roster's job. The kind is stated ([`FluidKind`]) and the ledger keeps the
//! books; two statements of salinity is how the sea comes to be fresh.
//!
//! # The cost is declared, not hidden
//!
//! A voxel that holds water **material** answers without building anything. A voxel
//! above the terrain asks [`hydrology::sample`], which builds a region and
//! priority-floods it — the honest price of the honest model (flow is not local,
//! `hydrology`'s own words). This is an **edit-time** query, not a per-frame one:
//! `Pool::edit` calls it once per edit, and the tests below call it where a lake
//! must be found.

use crate::chunk_fields;
use crate::fluidics::{FillLevel, FILL_LEVELS};
use crate::hydrology;
use crate::materials;
use crate::spec::WorldSpec;
use crate::terrain;
use voxel_model::matter::{FluidSpecies, Phase};
use voxel_model::{Refusal, RefusalKind, Refused};

/// Which water this is, by where the world put it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum FluidKind {
    /// The sea: above the terrain, at or below sea level.
    Ocean,
    /// Standing fresh water in a filled depression — `hydrology`'s priority-flood
    /// depth, the same figure the water ledger books as `surface`.
    Lake,
    /// A flooded void inside the rock, below the terrain.
    Karst,
}

impl FluidKind {
    /// Every kind, hand-written for the same reason [`crate::stages::ALL`] is: a
    /// kind added without a name should not compile silently.
    pub const ALL: [FluidKind; 3] = [FluidKind::Ocean, FluidKind::Lake, FluidKind::Karst];

    pub const fn name(self) -> &'static str {
        match self {
            FluidKind::Ocean => "ocean",
            FluidKind::Lake => "lake",
            FluidKind::Karst => "karst",
        }
    }
}

/// The fluid at one voxel: which water, in which species and phase, and how full.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FluidState {
    pub kind: FluidKind,
    /// From the world's roster ([`voxel_model::matter::FluidSpecies`]). Water is the
    /// only fluid this world places today; the field is the roster's, not a constant
    /// pretending the roster has one entry.
    pub species: FluidSpecies,
    /// Liquid today. Phases wait for the thermal system (H-22.2), so this is
    /// **stated** and not derived — the same honesty `water::Phase` uses.
    pub phase: Phase,
    /// How full the voxel is, in 1/32nds ([`FillLevel`], `coord::FILL_SIDE`) —
    /// floored, never rounded up, so a drawn surface is never above the books.
    pub fill: FillLevel,
}

impl FluidState {
    /// The water the material path placed: the sea, or a flooded void.
    ///
    /// Both are whole voxels of fluid by construction, so the fill is full.
    fn of_material(kind: FluidKind) -> FluidState {
        FluidState {
            kind,
            species: FluidSpecies::Water,
            phase: Phase::Liquid,
            fill: FillLevel::FULL,
        }
    }
}

/// The fluid the world holds at a voxel, or nothing.
///
/// `hydro_radius` is the same declared neighbourhood [`crate::hydrology::sample`]
/// takes — shrinking it shrinks the watershed rather than approximating the same
/// one, which is why it is an argument and not a default buried here.
pub fn fluid_at(
    spec: &WorldSpec,
    x: i32,
    y: i32,
    z: i32,
    hydro_radius: usize,
) -> Refused<Option<FluidState>> {
    let fx = f64::from(x) + 0.5;
    let fy = f64::from(y) + 0.5;
    let fz = f64::from(z) + 0.5;
    let surface_m = terrain::elevation_m(spec, fx, fz);

    // The material path's water: the sea above the terrain, a flooded void inside
    // it. One read of the world's own material law, so this cannot disagree with
    // the chunks the player sees.
    if materials::is_fluid(chunk_fields::material_at(spec, x, y, z)?) {
        let kind = if fy > surface_m { FluidKind::Ocean } else { FluidKind::Karst };
        return Ok(Some(FluidState::of_material(kind)));
    }

    // No water material, so the only fluid left is a **lake** — state above the
    // terrain, from the priority flood. Below the surface there is nothing to ask.
    if fy <= surface_m {
        return Ok(None);
    }
    let water_depth_m = hydrology::sample(spec, fx, fz, hydro_radius).water_depth_m;
    let fluid_surface_m = surface_m + water_depth_m;
    if fy > fluid_surface_m {
        return Ok(None);
    }
    Ok(Some(FluidState {
        kind: FluidKind::Lake,
        species: FluidSpecies::Water,
        phase: Phase::Liquid,
        fill: fill_level_at(y, fluid_surface_m)?,
    }))
}

/// The refusal an edit gets when its target holds fluid.
///
/// One sentence for every caller — [`crate::earthworks::hand_change`]'s material
/// half (the sea, a flooded void) and `Pool::edit`'s derived half (a lake) — so the
/// rule the user filed cannot drift into two rules.
pub fn refuse_edit(x: i32, y: i32, z: i32) -> Refusal {
    Refusal::owned(
        RefusalKind::FluidIsNotABlock,
        format!(
            "the edit at ({x}, {y}, {z}) targets water — a fluid is world state, not a \
             block: an edit that targets a fluid refuses rather than deleting it, and \
             displacement is not modelled yet"
        ),
    )
}

/// The fill level of the voxel spanning `y..y+1`, given the fluid's surface height.
///
/// **Floored**: the level counts the complete levels the fluid covers, so a surface
/// one level below the voxel's top is not full. The surface height is a
/// measured `f64` (a flood depth), so this is the one place a measurement becomes an
/// integer display level — and it floors, never rounds up, because the drawn surface
/// must never be above where the books say the fluid is.
pub(crate) fn fill_level_at(y: i32, fluid_surface_m: f64) -> Refused<FillLevel> {
    let covered = (fluid_surface_m - f64::from(y)).clamp(0.0, 1.0);
    FillLevel::new((covered * f64::from(FILL_LEVELS)).floor() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    const RADIUS: usize = crate::materials::DEFAULT_HYDRO_RADIUS;

    fn surface_m(spec: &WorldSpec, x: i32, z: i32) -> f64 {
        terrain::elevation_m(spec, f64::from(x) + 0.5, f64::from(z) + 0.5)
    }

    /// The first voxel whose centre is above `height`: `fy > height` is
    /// `y > height - 0.5`, so the floor of `height - 0.5` plus one.
    fn first_above(height: f64) -> i32 {
        (height - 0.5).floor() as i32 + 1
    }

    /// A column meaningfully below sea level, by **grid** search — a single line
    /// would be a fact about the line (the lesson the column tests already record).
    fn ocean_column(spec: &WorldSpec) -> (i32, i32) {
        for x in (-6000..6000).step_by(53) {
            for z in (-6000..6000).step_by(97) {
                if surface_m(spec, x, z) < spec.config.sea_level_m - 4.0 {
                    return (x, z);
                }
            }
        }
        panic!("this world has no column below sea level - 4 m");
    }

    /// A **lake** column: above sea level, with a priority-flood depth a body could
    /// swim in — so the found voxels are lakes and not the sea. The step and the
    /// depth threshold were **measured**, not guessed: over this world's above-sea
    /// grid (step 101) the deepest fill is ~2.04 m and thirteen columns exceed 1 m,
    /// so a metre is a real lake and not a rounding.
    fn lake_column(spec: &WorldSpec) -> (i32, i32, f64, f64) {
        for x in (-6000..6000).step_by(101) {
            for z in (-6000..6000).step_by(101) {
                let height = surface_m(spec, x, z);
                if height < spec.config.sea_level_m + 1.0 {
                    continue;
                }
                let depth = hydrology::sample(
                    spec,
                    f64::from(x) + 0.5,
                    f64::from(z) + 0.5,
                    RADIUS,
                )
                .water_depth_m;
                if depth > 1.0 {
                    return (x, z, height, depth);
                }
            }
        }
        panic!("no lake deeper than a metre in the searched window");
    }

    /// A flooded void below the sea floor, found by asking the material path itself:
    /// a water voxel *inside* the terrain is karst, not sea. Measured: 179 of the
    /// first 1 760 ocean columns hold one within 80 m of the surface, so the search
    /// walks columns and does not gamble on the first.
    fn flooded_karst(spec: &WorldSpec) -> (i32, i32, i32) {
        for x in (-6000..6000).step_by(53) {
            for z in (-6000..6000).step_by(97) {
                let height = surface_m(spec, x, z);
                if height >= spec.config.sea_level_m - 4.0 {
                    continue;
                }
                let start = (height - 1.0).floor() as i32;
                for y in (start - 80..start).rev() {
                    if chunk_fields::material_at(spec, x, y, z).unwrap() == materials::WATER {
                        return (x, y, z);
                    }
                }
            }
        }
        panic!("no flooded void below the sea in the searched window");
    }

    #[test]
    fn the_sea_is_fluid_state_at_full_fill() {
        let spec = spec();
        let (x, z) = ocean_column(&spec);
        let y = (spec.config.sea_level_m - 1.0).floor() as i32;
        let state = fluid_at(&spec, x, y, z, RADIUS).unwrap().expect("the sea is not fluid");
        assert_eq!(state.kind, FluidKind::Ocean);
        assert_eq!(state.species, FluidSpecies::Water);
        assert_eq!(state.phase, Phase::Liquid);
        assert_eq!(state.fill, FillLevel::FULL);
        // And that voxel is the material path's water, so the query and the world
        // the player sees agree about where the sea is.
        assert_eq!(
            chunk_fields::material_at(&spec, x, y, z).unwrap(),
            materials::WATER
        );
    }

    #[test]
    fn a_flooded_void_inside_the_rock_is_karst() {
        let spec = spec();
        let (x, y, z) = flooded_karst(&spec);
        let state = fluid_at(&spec, x, y, z, RADIUS).unwrap().expect("karst is not fluid");
        assert_eq!(state.kind, FluidKind::Karst, "water below the surface was called the sea");
        assert_eq!(state.fill, FillLevel::FULL);
    }

    #[test]
    fn a_lake_is_fluid_state_above_the_terrain_with_a_floored_fill() {
        let spec = spec();
        let (x, z, height, depth) = lake_column(&spec);
        let fluid_surface = height + depth;

        // The first voxel above the ground is inside the lake, and it is **not**
        // material water: this is the state the material path does not carry.
        let first = first_above(height);
        assert_eq!(
            chunk_fields::material_at(&spec, x, first, z).unwrap(),
            materials::AIR,
            "the lake voxel is a water material, so this test measures nothing"
        );
        let state = fluid_at(&spec, x, first, z, RADIUS).unwrap().expect("the lake is not fluid");
        assert_eq!(state.kind, FluidKind::Lake);
        assert!(
            state.fill.raw() >= 1,
            "the first voxel above the ground is at level {} — the depth is not reaching it",
            state.fill.raw()
        );

        // The topmost covered voxel is partially filled — the fill interface.
        let top = (fluid_surface - 0.5).floor() as i32;
        let top_state = fluid_at(&spec, x, top, z, RADIUS).unwrap().expect("the surface voxel");
        assert_eq!(top_state.kind, FluidKind::Lake);
        assert!(
            top_state.fill.raw() >= FILL_LEVELS / 2 && top_state.fill.raw() <= FILL_LEVELS,
            "the surface voxel's fill is {}, outside the half-full-to-full band",
            top_state.fill.raw()
        );

        // And just above the fluid surface there is no fluid.
        let above = first_above(fluid_surface);
        assert_eq!(fluid_at(&spec, x, above, z, RADIUS).unwrap(), None);
    }

    #[test]
    fn a_dry_high_column_has_no_fluid_state() {
        let spec = spec();
        let mut found = None;
        'search: for x in (-6000..6000).step_by(53) {
            for z in (-6000..6000).step_by(97) {
                let height = surface_m(&spec, x, z);
                if height < spec.config.sea_level_m + 20.0 {
                    continue;
                }
                let depth = hydrology::sample(
                    &spec,
                    f64::from(x) + 0.5,
                    f64::from(z) + 0.5,
                    RADIUS,
                )
                .water_depth_m;
                if depth == 0.0 {
                    found = Some((x, z, height));
                    break 'search;
                }
            }
        }
        let (x, z, height) = found.expect("no dry high column in the searched window");
        let y = first_above(height) + 5;
        assert_eq!(fluid_at(&spec, x, y, z, RADIUS).unwrap(), None);
    }

    #[test]
    fn the_query_is_deterministic_in_the_address() {
        // Nothing is stored between calls: the same address is the same answer,
        // twice, in either order (the purity the tick ladder claims).
        let spec = spec();
        let (x, z) = ocean_column(&spec);
        let y = (spec.config.sea_level_m - 1.0).floor() as i32;
        let a = fluid_at(&spec, x, y, z, RADIUS).unwrap();
        let b = fluid_at(&spec, x, y, z, RADIUS).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn a_fill_level_is_the_floor_of_the_covered_fraction_of_the_voxel() {
        // The display derivation, exact at its boundaries: the level counts complete
        // levels, so half a metre is half of them and the top of the voxel is full.
        assert_eq!(fill_level_at(0, 0.5).unwrap(), FillLevel::new(FILL_LEVELS / 2).unwrap());
        assert_eq!(fill_level_at(0, 1.0).unwrap(), FillLevel::FULL);
        assert_eq!(fill_level_at(0, 1.5).unwrap(), FillLevel::FULL, "a voxel cannot overflow");
        assert_eq!(fill_level_at(3, 3.0).unwrap(), FillLevel::EMPTY);
        // **Floored, never rounded up**: 0.999 of a metre is one level short, not full.
        assert_eq!(fill_level_at(0, 0.999).unwrap(), FillLevel::new(FILL_LEVELS - 1).unwrap());
    }

    #[test]
    fn a_fluid_edit_refusal_names_the_rule_and_the_kind_is_stable() {
        let r = refuse_edit(1, -2, 3);
        assert_eq!(r.kind(), RefusalKind::FluidIsNotABlock);
        assert!(r.detail().contains("water"), "got {r}");
        assert!(r.detail().contains("a fluid is world state"), "got {r}");
        // Every kind is named, and no two share a name.
        let mut names: Vec<&str> = FluidKind::ALL.iter().map(|k| k.name()).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before);
        assert_eq!(FluidKind::ALL.len(), 3);
    }
}
