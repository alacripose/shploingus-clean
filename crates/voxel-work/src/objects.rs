//! **The object lane: bodies the streamer loads, rather than a world it is resident in.**
//!
//! Everything else this crate streams is *terrain*, and terrain has a shape the residency rules
//! assume: it sits in a band around the eye, it is a function of `(seed, tick)` and not of the
//! clock, and the eye walks through it. The Sun and the Moon are none of those things. They are
//! bodies at their own **8×-scaled sizes and distances** ([`voxel_shapers::scale`]'s one ruling,
//! both halves of it: scaling a size and a distance by the same number preserves the angle, so
//! each body keeps the angular diameter it actually has), they are *placed* by an orbit rather
//! than found at a position, they turn about their own axes, and an eye at the surface never gets
//! near one.
//!
//! # Why they are a lane and not a horizon of the terrain window
//!
//! The four horizons ([`voxel_stream::Horizons`]) are chunk radii around the eye, and that is the
//! right shape for a world you stand in. A body at the Moon's own scaled distance is **1.5 million
//! chunks** away, so "resident" cannot mean what it means for a chunk of ground: the window would
//! have to be a cube of chunks reaching 48 050 km up, at 89 kB a tree and 90 kB a mesh, to hold a
//! body that is 32 cells across. So each body carries **its own horizon** — a reach of its own,
//! out of which it is not loaded at all — exactly as the cloud field carries its own, and the
//! lane loads what the sky has put up rather than what the window happens to cover.
//!
//! # The body is a cuboid, and that is not a compromise
//!
//! The Sun and the Moon are the **same kind of object** here, and the repository already ruled on
//! the shape: `sky.wgsl` and `sun.rs` both draw a cuboid, with a test
//! (`the_drawn_face_is_a_square_and_not_a_disc`) holding it there, and `sun.rs` records that the
//! ray–sphere solve *used to live in the shader*. A streamed body is the same body, so it is the
//! same shape — there is no sphere here to be faithful to.
//!
//! # One cell is a cuboid test, and the rotation is what moves it
//!
//! A body's voxels are **its own lattice**: `cells` across, with one cell
//! [`BodyDeclaration::cell_m`] metres on a side. For the Moon that cell is kilometres — a body
//! whose *cell* is the world's metre would be 217 175 voxels across — and it is the honest unit
//! for it, because 32 cells already put more pixels on the screen than the body subtends. A cell
//! is solid when its own centre, carried into the body's frame, lies inside the cuboid; spinning
//! the cuboid therefore re-cuts the surface into a staircase, which is what a **rotating voxel
//! body** is.
//!
//! **And that rotation is a declared cost, not a free animation.** Re-cutting the lattice is a
//! raster fill and a mesh, so a body is rebuilt only when its own spin has crossed
//! [`BodyDeclaration::spin_step_turns`] — one cell's worth of turn — since the last build. The
//! count of rebuilds a caller gets back from [`ObjectLane::refresh`] is that cost, reported rather
//! than hidden.
//!
//! # What is deliberately not here
//!
//! No GPU type, no thread and no clock: [`ObjectLane::refresh`] is handed a tick and an eye and is
//! a pure function of them (two calls at one tick build the same bytes), which is the same
//! determinism contract the rest of this crate keeps. **And no materials of its own** — a body's
//! voxel carries a material from the world's own registry, and an emissive row for the Sun is a
//! gap this lane does not paper over: `materials::IGNEOUS` is a *reflective* row, so a streamed Sun
//! is lit like rock until the emitter work lands.

use voxel_mesh::{Chunk, Mesh, MeshConfig, Mesher, SparseVoxelOctree};
use voxel_model::{ChunkPos, LocalPos, MaterialId, Refused, VoxelShape};
use voxel_shapers::sky::{self, MoonPosition, SolarPosition};
use voxel_shapers::{materials, scale, WorldSpec};

/// Which body.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Body {
    Sun,
    Moon,
}

impl Body {
    /// Both bodies, in the order the lane holds them.
    pub const ALL: [Body; 2] = [Body::Sun, Body::Moon];

    /// A stable, machine-readable name. Never prose.
    pub const fn name(self) -> &'static str {
        match self {
            Body::Sun => "sun",
            Body::Moon => "moon",
        }
    }
}

/// **The Sun's radius**, metres — IAU 2015 nominal solar radius.
pub const SUN_RADIUS_M: f64 = 6.957e8;

/// **The Sun's mean distance**, metres — one astronomical unit, IAU 2012 definition.
pub const SUN_DISTANCE_M: f64 = 1.495_978_707e11;

/// **The Moon's mean radius**, metres — IAU WGCCRE (`voxel_light::MOON_RADIUS_KM`'s own figure).
pub const MOON_RADIUS_M: f64 = 1.7374e6;

/// **The Moon's mean distance**, metres — the same published source as the radius.
pub const MOON_DISTANCE_M: f64 = 3.844e8;

/// **The Sun's rotation period**, in days — Carrington's sidereal 25.38.
const SUN_ROTATION_DAYS: f64 = 25.38;

/// **One body, declared**: the published figures, and the four choices this world makes with them.
///
/// A declaration is `Copy` and holds no state: what a body *is* is a fact about the body, and where
/// it is this tick is [`ObjectLane`]'s business.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BodyDeclaration {
    /// Which body.
    pub body: Body,
    /// The cited real radius, metres.
    pub real_radius_m: f64,
    /// The cited real distance, metres.
    pub real_distance_m: f64,
    /// **Cells across the body's own lattice.** A power of two, because the octree's side is one
    /// and the body must fill it without a half-cell of slack.
    pub cells: u32,
    /// **How many times the body turns about its own axis in one in-game day.**
    ///
    /// Both bodies are tidally or physically slow, and this is the *measured* figure rather than a
    /// spin chosen for the picture: the Moon turns once per **sidereal** month, which is what being
    /// tidally locked means, and the Sun once per [`SUN_ROTATION_DAYS`]. A visible spin is owed to
    /// the orbit and not to this number, and inventing a faster one would be inventing an orbit.
    pub turns_per_day: f64,
    /// **The turn at which the body is re-cut**, as a fraction of one turn.
    ///
    /// A quarter of one cell's share of the body: `1 / (4 · cells)` turns moves the silhouette by
    /// about a cell at the body's edge, so a step smaller than this refines nothing a pixel can
    /// show and a step larger would let the staircase jump.
    pub spin_step_turns: f64,
    /// The material a solid cell carries.
    pub material: MaterialId,
}

impl BodyDeclaration {
    /// The body's radius in **engine metres** — the real figure through [`scale::voxels`].
    pub fn engine_radius_m(&self) -> f64 {
        scale::voxels(self.real_radius_m)
    }

    /// The body's distance in **engine metres**, the same conversion, which is what keeps the
    /// angular diameter the body actually has.
    pub fn engine_distance_m(&self) -> f64 {
        scale::voxels(self.real_distance_m)
    }

    /// **One cell of the body's own lattice**, metres: the cuboid's extent over `cells`.
    pub fn cell_m(&self) -> f64 {
        2.0 * self.engine_radius_m() / self.cells as f64
    }

    /// **The body's own horizon**: the reach out of which it is not loaded at all, metres.
    ///
    /// Its distance plus its own radius — the far edge of the body — so a body is inside its
    /// horizon exactly while any part of it could be in front of the eye. This is the number that
    /// takes the place of a chunk radius for a thing that is not terrain.
    pub fn reach_m(&self) -> f64 {
        self.engine_distance_m() + self.engine_radius_m()
    }

    /// **The angular diameter the body subtends**, degrees — what the 8× ruling preserves.
    pub fn angular_diameter_deg(&self) -> f64 {
        2.0 * (self.engine_radius_m() / self.engine_distance_m()).atan().to_degrees()
    }

    /// The body's semi-angle, degrees — half its angular diameter.
    pub fn angular_radius_deg(&self) -> f64 {
        self.angular_diameter_deg() * 0.5
    }

    /// The octree level count whose side is [`BodyDeclaration::cells`].
    pub fn levels(&self) -> u32 {
        self.cells.trailing_zeros()
    }

    /// **The body's own voxels**, cut from a cuboid spun about its own up axis.
    ///
    /// `spin_turns` is the body's turn this tick, in `0..1`. A cell is solid when its centre,
    /// carried back into the body's frame, lies inside the cuboid of half-extent
    /// [`BodyDeclaration::engine_radius_m`]; the inset that leaves is half a cell at each face,
    /// which is 1.6 % of the body and named rather than corrected by a second rule.
    ///
    /// The tree is built through [`SparseVoxelOctree::set_voxel`], so it is **canonical**: the
    /// same voxels inserted in any order digest the same, and a fully solid cell run collapses to
    /// one leaf. That is what lets a body's identity be compared rather than merely counted.
    pub fn rasterise(&self, spin_turns: f64) -> SparseVoxelOctree {
        let side = self.cells;
        let half = side as f64 * 0.5;
        let cell = self.cell_m();
        let radius = self.engine_radius_m();
        let angle = spin_turns * std::f64::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        let mut tree = SparseVoxelOctree::with_levels(self.levels());
        for k in 0..side {
            for j in 0..side {
                for i in 0..side {
                    // The cell's centre, in the body's own frame, metres.
                    let x = (i as f64 + 0.5 - half) * cell;
                    let y = (j as f64 + 0.5 - half) * cell;
                    let z = (k as f64 + 0.5 - half) * cell;
                    // **The cuboid is fixed and the sample turns**, which is the same cut as a
                    // turning cuboid sampled on a fixed lattice and needs no second rotation.
                    let turned_x = cos * x + sin * z;
                    let turned_z = -sin * x + cos * z;
                    if turned_x.abs() <= radius && y.abs() <= radius && turned_z.abs() <= radius {
                        tree.set_voxel(
                            LocalPos { x: i, y: j, z: k },
                            VoxelShape::solid(self.material),
                        );
                    }
                }
            }
        }
        tree
    }
}

/// **The two bodies this world has**, at their published figures and at 8× scale.
///
/// `cells` is 32 for both, which is the octree's own level-5 side: the number is not a quality
/// setting — 32 cells subtend many times the pixels either body occupies, and a finer lattice would
/// put detail on the screen that no eye can resolve while multiplying the raster.
pub const DECLARED: [BodyDeclaration; 2] = [
    BodyDeclaration {
        body: Body::Sun,
        real_radius_m: SUN_RADIUS_M,
        real_distance_m: SUN_DISTANCE_M,
        cells: 32,
        turns_per_day: 1.0 / SUN_ROTATION_DAYS,
        spin_step_turns: 1.0 / (4.0 * 32.0),
        material: materials::IGNEOUS,
    },
    BodyDeclaration {
        body: Body::Moon,
        real_radius_m: MOON_RADIUS_M,
        real_distance_m: MOON_DISTANCE_M,
        cells: 32,
        // **Tidally locked**: one turn per *sidereal month*, which is why the near side never
        // moves. `sky::SIDEREAL_MONTH_DAYS` is the orbit's own period, so the spin and the orbit
        // cannot come apart.
        turns_per_day: 1.0 / sky::SIDEREAL_MONTH_DAYS,
        spin_step_turns: 1.0 / (4.0 * 32.0),
        material: materials::REGOLITH,
    },
];

/// **One body as this tick has it**: where it is, how far it has turned, and its own voxels.
///
/// `tree` and `mesh` are both in the body's **own lattice** — a cell is
/// [`BodyDeclaration::cell_m`] metres and not the world's metre — so something that places this
/// body has to carry that unit with it. `Mesh::corner_positions_m` is deliberately not called
/// anywhere here: it states metres, and for a body those metres would be cell counts.
#[derive(Clone, PartialEq, Debug)]
pub struct LoadedBody {
    /// What is loaded.
    pub declaration: BodyDeclaration,
    /// Where the orbit puts it, degrees above the horizon.
    pub altitude_deg: f64,
    /// Its azimuth, degrees from due north and increasing eastwards.
    pub azimuth_deg: f64,
    /// **Where the body stands, world metres** — the eye plus its own direction, out at its own
    /// scaled distance. A body is placed *from* the eye and not found at an address, which is what
    /// makes it the same body for every eye in the world.
    pub centre_m: [f64; 3],
    /// How far it has turned about its own axis, in turns.
    pub spin_turns: f64,
    /// The turn it was last cut at — what [`ObjectLane::refresh`] compares against.
    pub cut_at_turns: f64,
    /// The body's own octree, in its own lattice.
    pub tree: SparseVoxelOctree,
    /// The body's own visible surface, from this crate's mesher.
    pub mesh: Mesh,
}

/// **The lane**: the bodies the sky currently has up, and the machinery that cuts them.
///
/// It is deliberately not a [`Streamer`](crate::Streamer) and deliberately not a second one: the
/// streamer's whole shape is a residency over a window of chunks, and a body is at 1.5 million
/// chunks. What it *shares* with the streamer is everything that matters — the same
/// [`SparseVoxelOctree`], the same [`Mesher`], the same material registry and the same refusal
/// vocabulary — so a body's surface is the world's own surface and not a second rendering path.
pub struct ObjectLane {
    declarations: Vec<BodyDeclaration>,
    config: MeshConfig,
    mesher: Mesher,
    loaded: Vec<LoadedBody>,
    cut: u64,
}

impl ObjectLane {
    /// The lane over this world's bodies, meshed with the world's own material config.
    ///
    /// `Refused` rather than defaulted, for the reason [`voxel_mesh::MeshConfig`] is: whether a
    /// face is drawn is a property of the material, so a lane with an invented config would draw a
    /// body whose surface the world would not.
    pub fn new() -> Refused<Self> {
        let registry = materials::registry()?;
        let config = MeshConfig::from_registry(&registry)?;
        Ok(ObjectLane {
            declarations: DECLARED.to_vec(),
            config,
            mesher: Mesher::new(),
            loaded: Vec::new(),
            cut: 0,
        })
    }

    /// The declarations this lane loads, in order.
    pub fn declarations(&self) -> &[BodyDeclaration] {
        &self.declarations
    }

    /// What is loaded right now, in declaration order.
    pub fn loaded(&self) -> &[LoadedBody] {
        &self.loaded
    }

    /// Whether a body is currently loaded.
    pub fn is_loaded(&self, body: Body) -> bool {
        self.loaded.iter().any(|b| b.declaration.body == body)
    }

    /// A loaded body, by name.
    pub fn get(&self, body: Body) -> Option<&LoadedBody> {
        self.loaded.iter().find(|b| b.declaration.body == body)
    }

    /// **How many times a body has been cut since this lane existed** — the lane's own cost.
    pub fn cut_count(&self) -> u64 {
        self.cut
    }

    /// The turn a body is at this tick, in `0..1`.
    ///
    /// A pure function of the **absolute** tick and the body's own rate: a rotation accumulates,
    /// so the turn is `turns_per_day · tick` and not a function of the time of day. Phase-locking it
    /// to `tick % TICKS_PER_DAY` was the first version of this line and it was wrong in a way only a
    /// whole day of walking finds: the body turned through a day and then **snapped back to turn
    /// zero** at the boundary, so a body that had slowly come round was rewound once a day. The
    /// clock's day is the rate's *unit* — `turns_per_day` is read off
    /// [`voxel_shapers::climate::TICKS_PER_DAY`] at its own declaration — and not its phase.
    ///
    /// **The rate's unit is a day and the argument is a tick**, so the day is divided out here and
    /// nowhere else. Leaving it out was the second version of this line, and the slip is easy to
    /// miss because both versions look like a rotation: the turn then advanced
    /// `turns_per_day` **per tick** — 24 000 times the body's actual rate — and every body was
    /// re-cut on every frame. A rate and the clock it is a rate *over* are two things.
    ///
    /// `f64` holds the quotient exactly enough for this: at a tick of `1e9` the fractional part
    /// carries about `9e-9` of a turn, three orders below the finest [`BodyDeclaration::spin_step_turns`]
    /// any body declares.
    pub fn spin_turns(declaration: &BodyDeclaration, tick: u64) -> f64 {
        let day = voxel_shapers::climate::TICKS_PER_DAY as f64;
        (declaration.turns_per_day * tick as f64 / day).rem_euclid(1.0)
    }

    /// **Where the sky puts a body this tick**, and how lit it is.
    ///
    /// One reading per body, from the shaper that owns the orbit: the Moon rides
    /// [`sky::moon_position`] (its own sidereal month, its own hour angle) and the Sun
    /// [`sky::solar_position`]. Neither is re-derived here — the orbit has one home.
    pub fn sky_of(spec: &WorldSpec, tick: u64) -> (SolarPosition, MoonPosition) {
        (
            sky::solar_position(spec, tick),
            sky::moon_position(spec, tick, 0),
        )
    }

    /// Whether a body is in the sky at this altitude: above the horizon, or still partly up.
    ///
    /// `-angular_radius` and not `0`, because a body half over the horizon is *up*: it is a body
    /// standing on the skyline, which is the most visible thing a body does. A body that has fully
    /// set is not loaded at all, which is the whole of "dynamic" here.
    pub fn in_sky(declaration: &BodyDeclaration, altitude_deg: f64) -> bool {
        altitude_deg > -declaration.angular_radius_deg()
    }

    /// **Adopt this tick's sky**: load the bodies it has put up, cut the ones that have turned.
    ///
    /// Returns **how many bodies were cut**, which is the lane's work this call — zero for a tick
    /// that changed nothing, which is every frame between spin steps and every frame at all for a
    /// body that is down. A caller that reports this number is reporting the lane's cost rather
    /// than asserting it is small.
    ///
    /// A body that has set is dropped, tree and mesh and all: nothing downstream can be handed a
    /// body the sky does not have, because there is nothing to hand.
    pub fn refresh(
        &mut self,
        spec: &WorldSpec,
        tick: u64,
        eye_m: [f64; 3],
    ) -> Refused<usize> {
        let (sun, moon) = Self::sky_of(spec, tick);
        let mut cuts = 0;
        let mut next: Vec<LoadedBody> = Vec::with_capacity(self.loaded.len().max(2));
        for declaration in &self.declarations {
            let (altitude_deg, azimuth_deg) = match declaration.body {
                Body::Sun => (sun.altitude_deg, sun.azimuth_deg),
                Body::Moon => (moon.altitude_deg, moon.azimuth_deg),
            };
            if !Self::in_sky(declaration, altitude_deg) {
                continue;
            }
            let spin_turns = Self::spin_turns(declaration, tick);
            let direction = sky::direction_of(altitude_deg, azimuth_deg);
            let distance = declaration.engine_distance_m();
            let centre_m = [
                eye_m[0] + direction[0] * distance,
                eye_m[1] + direction[1] * distance,
                eye_m[2] + direction[2] * distance,
            ];
            // **The cut is kept when the spin has not crossed its step.** A body turns
            // continuously and its lattice cannot: re-cutting for a turn smaller than the lattice
            // can show would rebuild a mesh nobody can tell from the one already in hand, once per
            // frame, for the slowest-moving thing in the frame.
            let already = self
                .get(declaration.body)
                .filter(|old| (spin_turns - old.cut_at_turns).abs() < declaration.spin_step_turns);
            if let Some(old) = already {
                let mut kept = old.clone();
                kept.altitude_deg = altitude_deg;
                kept.azimuth_deg = azimuth_deg;
                kept.centre_m = centre_m;
                next.push(kept);
                continue;
            }
            let tree = declaration.rasterise(spin_turns);
            let chunk = Chunk::from_tree(ChunkPos::new(0, 0, 0), tree.clone());
            // **No neighbourhood**: a body stands in the sky, and `None` is the mesher's own
            // statement that everything past its edge is clear.
            let mesh = self.mesher.build(&chunk, &self.config, None)?;
            self.cut += 1;
            cuts += 1;
            next.push(LoadedBody {
                declaration: *declaration,
                altitude_deg,
                azimuth_deg,
                centre_m,
                spin_turns,
                cut_at_turns: spin_turns,
                tree,
                mesh,
            });
        }
        self.loaded = next;
        Ok(cuts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    fn lane() -> ObjectLane {
        ObjectLane::new().expect("the world's own material config")
    }

    #[test]
    fn the_eight_times_ruling_is_applied_to_both_the_size_and_the_distance() {
        // The whole reason the angles survive: `scale::voxels` is one ruling and the bodies take it
        // twice. A body that had only its *size* scaled would subtend eight times the angle it has,
        // so this is the assertion that keeps a "sensible" size from being chosen later.
        let sun = &DECLARED[0];
        let moon = &DECLARED[1];
        assert!((sun.engine_radius_m() - SUN_RADIUS_M / 8.0).abs() < 1.0);
        assert!((sun.engine_distance_m() - SUN_DISTANCE_M / 8.0).abs() < 1.0);
        assert!((moon.engine_radius_m() - MOON_RADIUS_M / 8.0).abs() < 1.0);
        assert!((moon.engine_distance_m() - MOON_DISTANCE_M / 8.0).abs() < 1.0);
        // And what the scale was *for*: both bodies keep the half a degree they really have.
        for body in [sun, moon] {
            let deg = body.angular_diameter_deg();
            assert!(
                (0.45..0.58).contains(&deg),
                "{} subtends {deg}°, which is not the half a degree it has",
                body.body.name()
            );
        }
    }

    #[test]
    fn a_body_has_its_own_horizon_and_it_holds_its_own_distance() {
        // A body is not inside the terrain window and could not be: the Moon's scaled distance is
        // about 1.5 million chunks. What it has instead is a reach of its own, and the reach of a
        // body has to cover the body.
        let moon = &DECLARED[1];
        assert!(
            moon.reach_m() > moon.engine_distance_m(),
            "a horizon inside its own body's distance is not a horizon"
        );
        assert!((moon.reach_m() - (moon.engine_distance_m() + moon.engine_radius_m())).abs() < 1.0);
        // The Sun's is the wider of the two, as the Sun's distance is — the two horizons are not
        // one number, which is why the lane asks each body for its own.
        assert!(DECLARED[0].reach_m() > DECLARED[1].reach_m());
        // And neither is inside the terrain's own declared field horizon (96 chunks, 3 km).
        let field_horizon_m = 96.0 * voxel_model::CHUNK_SIDE as f64;
        assert!(DECLARED[1].reach_m() > field_horizon_m);
    }

    #[test]
    fn a_body_is_cut_into_the_game_s_own_octree_and_mesher() {
        // The surface a body is drawn from is this crate's mesher over a canonical octree — not a
        // second geometry path. A cuboid cut on a 32³ lattice and meshed from outside must come
        // back with the six faces of a cube, merged: the greedy merge cannot tell a flat face of
        // one material from another.
        let moon = &DECLARED[1];
        let tree = moon.rasterise(0.0);
        assert_eq!(tree.solid_count(), (moon.cells * moon.cells * moon.cells) as u64);
        let mut mesher = Mesher::new();
        let registry = materials::registry().expect("the registry");
        let config = MeshConfig::from_registry(&registry).expect("a mesh config");
        let chunk = Chunk::from_tree(ChunkPos::new(0, 0, 0), tree);
        let mesh = mesher.build(&chunk, &config, None).expect("a meshed cuboid");
        assert_eq!(
            mesh.quads.len(),
            6,
            "a cube of one material is six merged rectangles, not {}",
            mesh.quads.len()
        );
        // Every rectangle is the whole face: the body's own extent, in subvoxels.
        let side_sub = moon.cells * voxel_model::SUBVOXEL_SIDE;
        for quad in &mesh.quads {
            assert_eq!(
                quad.extent,
                [side_sub as u16, side_sub as u16],
                "a face of the cuboid did not cover it"
            );
        }
    }

    #[test]
    fn the_spin_re_cuts_the_body_and_only_when_it_has_crossed_its_step() {
        // The rotation is geometry and it is a declared cost. One turn cuts the cube into a
        // staircase; a turn smaller than the lattice's own step must not rebuild anything, and a
        // turn past it must.
        let moon = &DECLARED[1];
        let square = moon.rasterise(0.0);
        // **An eighth of a turn, not a quarter**: a cube is its own symmetry four times over, so a
        // quarter turn is the same body cut the same way — which is a fact about cuboids and not
        // about the rasteriser, and a test that used it would be measuring nothing.
        let turned = moon.rasterise(0.125);
        assert!(
            turned.solid_count() < square.solid_count(),
            "an eighth turn cut {} cells against the square's {}",
            turned.solid_count(),
            square.solid_count()
        );
        assert_ne!(
            square.digest(),
            turned.digest(),
            "an eighth turn produced the same body"
        );
        // And the same turn twice is the same body, which is what makes a body's digest meaningful.
        assert_eq!(square.digest(), moon.rasterise(0.0).digest());
    }

    #[test]
    fn the_lane_loads_what_the_sky_has_up_and_drops_what_has_set() {
        let spec = spec();
        let mut lane = lane();
        let eye = [0.0, 17.0, 0.0];

        // **A tick whose sun is up.** Scanned rather than asserted from a constant, for the reason
        // `sun.rs`'s own moon test gives: the orbit has a rise time, and a hard-coded tick is a
        // claim about the orbit that nothing keeps true. **The day's own maximum** rather than the
        // first tick over a threshold, so the margin below the horizon is the widest the day has and
        // the spin test below cannot run out of daylight.
        let day = voxel_shapers::climate::TICKS_PER_DAY;
        let noon = (0..day)
            .max_by(|a, b| {
                sky::solar_position(&spec, *a)
                    .altitude_deg
                    .total_cmp(&sky::solar_position(&spec, *b).altitude_deg)
            })
            .expect("a day has ticks");
        let cuts = lane.refresh(&spec, noon, eye).expect("a refresh");
        // **Exactly the bodies the sky has up**, which is not necessarily one: the Moon is up for
        // half of every day and its own orbit decides which half. The claim is the set, derived from
        // the same two orbit readings the lane uses.
        let (sun_sky, moon_sky) = ObjectLane::sky_of(&spec, noon);
        let want_sun = ObjectLane::in_sky(&DECLARED[0], sun_sky.altitude_deg);
        let want_moon = ObjectLane::in_sky(&DECLARED[1], moon_sky.altitude_deg);
        assert!(want_sun, "the day's own noon does not have the Sun up");
        assert_eq!(lane.is_loaded(Body::Sun), want_sun);
        assert_eq!(lane.is_loaded(Body::Moon), want_moon);
        assert_eq!(cuts, want_sun as usize + want_moon as usize, "a loaded body is cut once");
        let sun = lane.get(Body::Sun).expect("the Sun").clone();
        assert!(sun.mesh.quads.len() >= 6, "the Sun has no surface");
        // It stands at its own scaled distance along its own direction.
        let range = ((sun.centre_m[0] - eye[0]).powi(2)
            + (sun.centre_m[1] - eye[1]).powi(2)
            + (sun.centre_m[2] - eye[2]).powi(2))
        .sqrt();
        assert!(
            (range - DECLARED[0].engine_distance_m()).abs() < 1.0,
            "the Sun stands {range} m away, not at its own distance"
        );

        // Nothing moved: the same tick builds nothing, and the body is the one already in hand.
        assert_eq!(lane.refresh(&spec, noon, eye).expect("a second refresh"), 0);
        assert_eq!(lane.get(Body::Sun).expect("the Sun").cut_at_turns, sun.cut_at_turns);

        // **The keep rule, through the lane.** While the Sun is up and its own turn is below the
        // step, the body in hand is the right body: same digest, no raster. The scan stops where the
        // sun sets — this world's own day is shorter than a full step of the Sun's spin, which is
        // the reason the *crossing* is asserted a whole day out below.
        let step = DECLARED[0].spin_step_turns;
        let mut kept = 0;
        for dt in (100..=day).step_by(100) {
            if DECLARED[0].turns_per_day * dt as f64 / day as f64 >= step {
                break;
            }
            let Some(before) = lane.get(Body::Sun).map(|b| b.tree.digest()) else { break };
            lane.refresh(&spec, noon + dt, eye).expect("a refresh");
            let Some(after) = lane.get(Body::Sun).map(|b| b.tree.digest()) else { break };
            assert_eq!(
                after,
                before,
                "dt {dt}: the Sun was re-cut by a turn below the {step} it declares"
            );
            kept += 1;
        }
        assert!(kept > 10, "the daylight was too short to see the keep rule ({kept} samples)");

        // **The step, spent.** A whole day later the Sun stands where it stood at noon — the hour
        // angle is the same and the declination has moved a degree, so it is certainly still up —
        // while its own spin has advanced a full `turns_per_day`, several of its steps. The body in
        // hand is then the wrong body, and the lane has to cut it again.
        //
        // (The loop above ends with the Sun **set** — it broke on the iteration where the Sun was
        // gone — so the reference is the noon body captured at the top of the test and not whatever
        // is in the lane now. Comparing against the lane's current body would compare against
        // nothing, which is the shape of assertion that passes for the wrong reason.)
        assert!(DECLARED[0].turns_per_day >= step, "a day's spin is below one step");
        assert!(
            lane.refresh(&spec, noon + day, eye).expect("a refresh") >= 1,
            "a whole day of spin cut nothing"
        );
        let after = lane
            .get(Body::Sun)
            .expect("the Sun is up at the same hour a day later");
        assert_ne!(
            after.tree.digest(),
            sun.tree.digest(),
            "a whole day of spin and the Sun came back the same body"
        );

        // **A set Sun is dropped.** It is not a body drawn dimmer — it is not loaded at all, so a
        // caller cannot be handed one. The night is found by the same scan as the noon, for the same
        // reason.
        let midnight = (0..day)
            .min_by(|a, b| {
                sky::solar_position(&spec, *a)
                    .altitude_deg
                    .total_cmp(&sky::solar_position(&spec, *b).altitude_deg)
            })
            .expect("a day has ticks");
        let (night_sun, _) = ObjectLane::sky_of(&spec, midnight);
        assert!(
            !ObjectLane::in_sky(&DECLARED[0], night_sun.altitude_deg),
            "the day's own midnight has the Sun up"
        );
        lane.refresh(&spec, midnight, eye).expect("a night refresh");
        assert!(!lane.is_loaded(Body::Sun), "a set Sun is still loaded");
        assert!(lane.loaded().iter().all(|b| b.declaration.body != Body::Sun));
    }
}
