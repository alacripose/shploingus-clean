//! **The map's ground, and the one owner of it: the surface, the survey and the cache.**
//!
//! # What the map reads, and why it is not the law
//!
//! The map window used to be a re-derivation of `terrain::elevation_m` — a smooth field evaluated
//! on a grid. The world is not that field: it is **the voxels the streamer holds**, placed by the
//! field plane, cut by caves, dressed by flora, and changed by every dig
//! ([`voxel_shapers::earthworks`]). A map drawn from the law therefore disagrees with the ground a
//! player is standing on — by the quantisation of the voxel, and by everything that has happened
//! since the chunk was generated.
//!
//! So the map asks the world instead: [`MapGround`] is the residency itself, and the survey reads
//! **the top of the topmost solid voxel of every column** out of the trees the streamer actually
//! has. A `Mixed` voxel is read at its topmost solid **subvoxel**, so a shaper's ¾-metre face is
//! drawn at ¾ of a metre rather than rounded to its cell; and because the answer is the world's own
//! voxels, whatever the mesher draws — displacement, a placed block, a dug pit — the map cannot
//! disagree with it.
//!
//! **Where nothing is resident the world is asked anyway, one column at a time.** The
//! streamer's window is a few hundred metres; a map of a valley is kilometres. A chunk outside
//! it has never been built and building one to read one column would cost 136 ms, so the
//! survey asks the *generator* for the columns it needs at its own sample LOD:
//! [`MapGround::column_surface`] is the world's own material law read at a handful of voxels of
//! one column, canopy included, and it is the same law the resident chunks were built from.
//! **The law is the fallback only where the world answers nothing** — an ocean column whose
//! only matter is water, a column past the survey the world can be probed in — and
//! [`Observation::from_chunks`] and [`Observation::from_world`] are the counts of samples that
//! came from the residency and from the generator, numbers a baseline records rather than a
//! claim in prose.
//!
//! **The far ground arrives in bands.** A column probe measured **22 µs**
//! (`measure_the_far_column_probe`), so the widest window's 36 864 samples are 0.9 s of work —
//! more than a frame owes. The survey reads [`FAR_BAND_PROBES`] of them per ground write, in
//! rectangles growing out from the middle of the picture, and **remembers every column it has
//! asked** until the world's record moves, so panning back over ground the map has already
//! surveyed costs nothing and the count of the world's own samples climbs instead of restarting.
//!
//! # One owner
//!
//! The plan the survey was taken for, the law's field, the composed ground, the key all three are
//! cached under, and the raster built from them are **one value**: [`MapSurvey`]. A caller observes
//! the world with one call and draws with another; it never derives a key, never holds a terrain
//! between frames, and cannot draw a survey of one world beside a marker of another.
//!
//! `main.rs` used to re-derive the survey key every frame while `MapTerrain`, the raster cache and
//! the camera lived in three places; this module is that answer, and
//! `docs/MAP-CONTROLS.md` states the structure.

use voxel_mesh::octree::Node;
use voxel_mesh::SparseVoxelOctree;
use voxel_model::{ChunkPos, CHUNK_SIDE, SUBVOXEL_SIDE};
use voxel_shapers::WorldSpec;
use voxel_ui::{Element, Rect};
use voxel_work::Streamer;

use crate::map_view::{filter_key, MapCamera, MapTerrain, SurveyPlan, ViewFilter};

/// **How many ticks one climate reading stands for.** Biomes move over thousands of ticks, so the
/// survey is keyed on the bucketed clock rather than the raw one: two frames of the same moment
/// are the same survey.
pub const TICK_BUCKET: u64 = 256;

/// **The world as the map may read it** — the residency, in the residency's own vocabulary.
///
/// The implementor is whatever holds the world's chunks ([`voxel_work::Streamer`] here, a stub in
/// a test); the map's question is one call per chunk, because a chunk is the unit the world is
/// held in and the unit a survey's samples are read out of.
pub trait MapGround {
    /// **A chunk column's own surface**: the top of the topmost **solid** voxel of each of its
    /// `32 × 32` columns, in metres above the datum, in `x + CHUNK_SIDE·z` order, or `None` where
    /// no chunk over that column is resident.
    ///
    /// **A fluid is not the ground.** Water is a voxel material here, so a sea column's topmost
    /// *voxel* is its skin at sea level; reading that as the surface would paint every resident
    /// ocean chunk as an island, because the ink below sea level is what draws water. The surface
    /// is the topmost voxel that is not [`voxel_shapers::materials::is_fluid`] — the one rule the
    /// mesh, the movement check and this map all ask.
    ///
    /// **A column may hold matter in more than one layer** — a canopy is a chunk above the ground,
    /// a ledge one below — and which layers there are is the source's business, so the answer is
    /// the highest solid voxel of all of them rather than the one named by any index here.
    fn chunk_surface(&self, chunk: [i32; 2]) -> Option<ChunkSurface>;

    /// **The world's own surface at one column, asked of the generator**: the top of the
    /// topmost solid voxel the world's material law makes there — canopy included, fluids
    /// skipped — or `None` when nothing solid stands in the probed span.
    ///
    /// This is what the map reads where no chunk is resident. It is the *same world* the
    /// resident chunks were built from (the generator with the earthworks record composed over
    /// it), reached at the column's own LOD rather than by building a chunk for it; the map asks
    /// it at the survey's sample spacing, which is why a far view costs a probe per *sample*
    /// and not per voxel of the country it covers.
    ///
    /// **The default answers nothing**, which is the honest default: a ground that holds chunks
    /// and has no generator behind them — a heightfield stub, a test's sheet — can only speak
    /// for what it holds, and the law's field is the map's fallback there.
    fn column_surface(&self, _x: i32, _z: i32) -> Option<f32> {
        None
    }

    /// **A number that moves when the resident world moves**: a chunk arriving, leaving or being
    /// rebuilt, and an edit landing. The survey writes the world's own ground over the law's when
    /// this changes, so a dig reaches the map without a plan moving.
    fn ground_revision(&self) -> u64;

    /// **A number that moves only when the world's *record* moves** — an edit lands, and not a
    /// chunk merely arriving or leaving.
    ///
    /// [`MapGround::column_surface`]'s answers are kept between surveys, and this is the key they
    /// are kept under: the generator is a pure function of the spec and the column, so the only
    /// thing that can change an answer is the earthworks record. Keying the cache on
    /// [`MapGround::ground_revision`] instead would throw it away every time a chunk streamed in.
    /// The default never moves, for a ground with no record to move.
    fn world_generation(&self) -> u64 {
        0
    }
}

/// **The world's own surface of one chunk**: one height per column, of the **solid** surface.
///
/// A column with no solid matter at all is [`f32::NEG_INFINITY`], so "the world says nothing here"
/// and "the world says the ground is at −∞" cannot be confused; [`ChunkSurface::top`] is the
/// reading. A column of water over no solid voxel is unanswered — the law's own elevation is the
/// honest answer there, and it is the one the ink below sea level already draws.
#[derive(Clone, PartialEq, Debug)]
pub struct ChunkSurface {
    tops: Vec<f32>,
}

impl ChunkSurface {
    /// A chunk of nothing: every column unanswered.
    pub fn empty() -> ChunkSurface {
        ChunkSurface {
            tops: vec![f32::NEG_INFINITY; (CHUNK_SIDE * CHUNK_SIDE) as usize],
        }
    }

    /// **A chunk of one flat surface**: every column answers `top_m` metres above the datum.
    ///
    /// For a ground that knows its own heights without holding voxels — a heightfield source, a
    /// test's stub — where the tree walk is ceremony rather than the answer.
    pub fn sheet(top_m: f32) -> ChunkSurface {
        ChunkSurface {
            tops: vec![top_m; (CHUNK_SIDE * CHUNK_SIDE) as usize],
        }
    }

    /// The top of a column, in metres, or `None` where the chunk holds no matter there.
    ///
    /// **A column outside the chunk is not this chunk's column.** The grid is `32 × 32` and one
    /// past its right edge is not "the first column of the next row": it is nothing, and answering
    /// the *wrapped* neighbour there put a second chunk's column on a sample that already had an
    /// owner — the sample a survey grid lands exactly on a chunk boundary, which is the one place a
    /// map and a chunk grid can disagree about who owns a metre.
    pub fn top(&self, local: [i32; 2]) -> Option<f32> {
        if local[0] < 0 || local[1] < 0 {
            return None;
        }
        if local[0] >= CHUNK_SIDE || local[1] >= CHUNK_SIDE {
            return None;
        }
        let at = (local[0] + CHUNK_SIDE * local[1]) as usize;
        self.tops.get(at).copied().filter(|top| top.is_finite())
    }

    /// Whether any column answered at all — the difference between "a chunk of air" and "no chunk".
    pub fn is_empty(&self) -> bool {
        !self.tops.iter().any(|top| top.is_finite())
    }

    /// The raw column grid, for the survey's own sampling.
    fn column(&self, local: [i32; 2]) -> Option<f32> {
        self.top(local)
    }
}

/// **Write one tree's own surface into a chunk's column tops.**
///
/// `floor_m` is the world height of the tree's own floor: a leaf's `local[1]` is in the tree's
/// vault, and the map asks about the world. The highest solid voxel of a column wins, so a tree
/// asked after another layer is merged rather than replaced.
pub fn absorb_surface(tops: &mut [f32], floor_m: f32, tree: &SparseVoxelOctree) {
    tree.visit_leaves(&mut |local: [i32; 3], level: u32, node: &Node| {
        let span = 1i32 << level;
        let top_m = match node {
            // A uniform leaf is one material over its whole region, so its top face is the
            // region's — when it is solid. A leaf of water is a skin, not a surface.
            Node::Uniform(material)
                if !material.is_empty() && !voxel_shapers::materials::is_fluid(*material) =>
            {
                floor_m + (local[1] + span) as f32
            }
            // **A mixed voxel is read at its own shape's top**: only ever one voxel wide
            // (`Node`'s own rule), so the topmost solid subvoxel is the surface a viewer sees —
            // 1/4, 1/2 or 3/4 of a metre above the voxel's floor, not its whole cell. A subvoxel of
            // water is skipped for the same reason a leaf of it is.
            Node::Mixed(shape) => {
                let Some(subvoxel) = topmost_subvoxel(shape) else {
                    return;
                };
                floor_m + local[1] as f32 + (subvoxel + 1) as f32 / SUBVOXEL_SIDE as f32
            }
            _ => return,
        };
        for z in local[2]..(local[2] + span) {
            for x in local[0]..(local[0] + span) {
                let at = (x + CHUNK_SIDE * z) as usize;
                if let Some(slot) = tops.get_mut(at) {
                    if top_m > *slot {
                        *slot = top_m;
                    }
                }
            }
        }
    });
}

/// The topmost solid subvoxel of a 4 × 4 × 4 shape: its `y` level, `0..SUBVOXEL_SIDE`.
fn topmost_subvoxel(shape: &voxel_model::VoxelShape) -> Option<i32> {
    // The 64 cells are `x + 4y + 16z` (`SubvoxelIndex::flat`), so the topmost solid subvoxel is
    // the one with the largest `(ordinal / 4) % 4` — the same expression the far census reads.
    shape
        .cells()
        .iter()
        .enumerate()
        .filter(|(_, material)| {
            // **A subvoxel of water is not the ground**, for the same reason a leaf of it is not: the
            // sea is drawn from the ink below sea level, not from its skin.
            !material.is_empty() && !voxel_shapers::materials::is_fluid(**material)
        })
        .map(|(ordinal, _)| (ordinal as i32 / 4) % SUBVOXEL_SIDE as i32)
        .max()
}

/// **The streamer is the map's ground.** Its trees are the world's own voxels, with the earthworks
/// record already composed into them, so this is the one implementation that reads a live world.
impl MapGround for Streamer {
    /// **The generator's own column**, through the streamer that holds the world it reads: the
    /// same `WorldSource` the resident chunks were built from, with the earthworks record
    /// composed over it, so a placed block outside the window is in the map like one inside it.
    fn column_surface(&self, x: i32, z: i32) -> Option<f32> {
        // A refusal is the generator saying it cannot answer about this column (a value out of
        // the declared range, say); the law is the map's fallback there, exactly as it is for a
        // column the world answers nothing for.
        Streamer::column_surface(self, x, z).ok().flatten()
    }

    fn chunk_surface(&self, chunk: [i32; 2]) -> Option<ChunkSurface> {
        let window = self.window();
        let centre = self.centre();
        let mut surface = ChunkSurface::empty();
        let mut held = false;
        // **Every layer the window holds over this column is asked**, from the top down: a column
        // whose matter stands in a chunk above the ground (a canopy) reads the top of that one, and
        // a chunk being rebuilt or refused answers nothing and leaves the layers below it.
        for y in (centre.y - window.vertical_chunks..=centre.y + window.vertical_chunks).rev() {
            let pos = ChunkPos::new(chunk[0], y, chunk[1]);
            let Some(tree) = self.tree(pos) else {
                continue;
            };
            held = true;
            absorb_surface(&mut surface.tops, (y * CHUNK_SIDE) as f32, tree);
        }
        held.then_some(surface)
    }

    fn ground_revision(&self) -> u64 {
        // Both clocks: the record's says an edit landed, the residency's says a chunk arrived,
        // left or was rebuilt. Either one moves the map's ground.
        self.residency_generation()
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ Streamer::world_generation(self)
    }

    fn world_generation(&self) -> u64 {
        // **The record's own clock, and deliberately not the residency's.** A far column's
        // answer is the generator's, which no chunk arriving can change; a dig changes it, and
        // that is this number.
        Streamer::world_generation(self)
    }
}

/// **The world one observation reads**: the ground, the laws the far field comes from, and the
/// clock the climate stands at.
///
/// The ground is `Option` because a host can have a map and no streamer — the window is a package's
/// and the world is the program's — and a map of the laws alone is the honest picture then, not a
/// failure.
#[derive(Clone, Copy)]
pub struct MapWorld<'a> {
    /// The resident world, when there is one.
    pub ground: Option<&'a dyn MapGround>,
    /// The world's own declaration: what the far field is sampled from.
    pub spec: &'a WorldSpec,
    /// The climate clock, which the survey buckets rather than following every frame.
    pub tick: u64,
}

/// What one [`MapSurvey::observe`] did, so a caller can measure the map instead of trusting it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Observation {
    /// The law's own field was resampled: a new plan, a new view, a new tick bucket, or the first
    /// observation of all.
    pub law: bool,
    /// The world's own surface was written over it — a chunk arrived, left, was rebuilt, or an
    /// edit landed.
    pub ground: bool,
    /// **How many of the survey's samples are the residency's own voxels** — a chunk the
    /// streamer holds, read exactly.
    pub from_chunks: usize,
    /// **How many are the generator's own surface**, asked column by column at the survey's
    /// sample LOD: ground the window looks over and no chunk has been built for.
    pub from_world: usize,
    /// The survey's samples in all.
    pub samples: usize,
}

impl Observation {
    /// **How many of the survey's samples are the world's own**, residency and generator
    /// together, rather than the law's fallback field. This is the number that climbs as the far
    /// fill reads the window's own columns.
    pub fn world_samples(&self) -> usize {
        self.from_chunks + self.from_world
    }

    /// Whether every sample of the survey is the world's own: nothing on the map is the law's
    /// field. This is what the far fill is climbing towards.
    pub fn whole(&self) -> bool {
        self.samples > 0 && self.world_samples() == self.samples
    }
}

/// **The map's survey, its cache and its keys, under one owner.**
///
/// The caller observes (the world moves, the camera moves) and draws; it holds no key, no terrain
/// and no raster of its own.
pub struct MapSurvey {
    /// **The law's own field for the current plan** — the far ground, and the base the world's own
    /// columns are written over. Kept apart from the composed ground so a chunk arriving costs a
    /// clone rather than a resample of the laws.
    law: Option<MapTerrain>,
    /// The composed ground: the law with every resident column's own voxel top over it. This is
    /// what the picture, the picking and the marker read.
    terrain: Option<MapTerrain>,
    /// The plan the law was sampled for, and the moment it was sampled at.
    key: Option<SurveyKey>,
    /// The ground revision the world's own columns were written from.
    ground: Option<u64>,
    /// Seconds since the world's own ground was last written (see [`MapSurvey::GROUND_INTERVAL_S`]).
    since_ground_s: f64,
    /// **The cached ground raster**, with the key it was built for: camera, ground revision, view
    /// and body rect. The marks are drawn fresh every frame; the ground only when one of those
    /// moves.
    raster: Option<(u64, Vec<Element>)>,
    /// How many samples of the current survey are the residency's own.
    from_chunks: usize,
    /// How many are the generator's own.
    from_world: usize,
    /// **Whether the survey still holds columns the generator has not been asked about**: the far
    /// fill climbs by one band per ground write until this is false.
    far_left: bool,
    /// **The world's own ground beyond the residency**, and the answers it has already given.
    far: Far,
}

/// The plan and clock a law sample stands for. Floats as bits, because this is an equality key and
/// not a distance: a plan a millimetre away is a different survey.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct SurveyKey {
    centre: [u64; 2],
    sample_m: u64,
    declared_m: u64,
    extent: [u32; 2],
    tick: u64,
}

impl Default for MapSurvey {
    fn default() -> Self {
        MapSurvey::new()
    }
}

impl MapSurvey {
    /// **How often the world's own ground is written over the law's**, seconds.
    ///
    /// A survey is a measurement, not a per-frame cost. Chunks arrive while the map is open and an
    /// edit lands between two frames; writing every resident column again sixty times a second
    /// would spend the frame on a picture the camera is about to move anyway. Four times a second
    /// keeps the map's ground within a quarter second of the world, and a plan change resamples
    /// immediately whatever the gate says.
    pub const GROUND_INTERVAL_S: f64 = 0.25;

    /// A survey with nothing sampled yet.
    pub fn new() -> MapSurvey {
        MapSurvey {
            law: None,
            terrain: None,
            key: None,
            ground: None,
            since_ground_s: Self::GROUND_INTERVAL_S,
            raster: None,
            from_chunks: 0,
            from_world: 0,
            far_left: false,
            far: Far::new(),
        }
    }

    /// **Observe the world for one plan.** Samples the laws when the plan, view or climate moved,
    /// writes the world's own surface over them when the residency moved, and leaves the last
    /// survey in hand when neither did.
    pub fn observe(&mut self, world: MapWorld<'_>, plan: SurveyPlan, dt_s: f64) -> Observation {
        let mut observation = Observation {
            samples: plan.columns * plan.rows,
            ..Observation::default()
        };
        let key = SurveyKey {
            centre: [plan.centre_xz[0].to_bits(), plan.centre_xz[1].to_bits()],
            sample_m: plan.sample_m.to_bits(),
            declared_m: plan.declared_m.to_bits(),
            extent: [plan.columns as u32, plan.rows as u32],
            tick: world.tick / TICK_BUCKET,
        };
        if self.key != Some(key) {
            self.law = Some(MapTerrain::sample(world.spec, world.tick, plan));
            self.key = Some(key);
            self.ground = None;
            self.since_ground_s = Self::GROUND_INTERVAL_S;
            observation.law = true;
            // **A new plan is a new fill.** The world's own columns already read are kept —
            // they are the world's, wherever the window looks — and the survey is not finished
            // until the band walk has been over it, so the fill is open again.
            self.far_left = true;
        }
        let revision = world.ground.map(MapGround::ground_revision);
        self.since_ground_s += dt_s.max(0.0);
        // **The gate, and why it exists.** A world streaming in moves the residency generation
        // every frame a chunk lands; re-reading every resident column at that rate is work the
        // frame cannot spare, and no viewer can see. A plan change opens the gate at once, because
        // a survey of the wrong place is not a survey of the world at all.
        let open = observation.law || self.since_ground_s >= Self::GROUND_INTERVAL_S;
        let world_moved = revision.is_some() && self.ground != revision && open;
        // **The world is read where the map can see it, and the read is not finished until the
        // far fill is.** A plan change or an edit means there is new ground to read; a fill still
        // climbing means there is a band of it left. Either one opens this branch — and the band
        // is spent on the gate's own tick, because a write republishes the ground and a
        // republished ground rebuilds the picture's raster: a band per frame would draw the whole
        // window sixty times a second to show one band of it.
        let filling = self.far_left && world.ground.is_some() && open;
        if observation.law || world_moved || filling {
            let Some(law) = self.law.as_ref() else {
                return observation;
            };
            let mut terrain = law.clone();
            let (resident, from_chunks) = match world.ground {
                Some(ground) => write_resident(&mut terrain, ground),
                None => (Vec::new(), 0),
            };
            let far = match world.ground {
                Some(ground) => self.far.write(&mut terrain, ground, &resident),
                None => FarWrite {
                    written: 0,
                    left: false,
                },
            };
            self.from_chunks = from_chunks;
            self.from_world = far.written;
            self.far_left = far.left;
            // The span is re-measured and the revision moves: the relief's ink and the water's
            // depth are read from the first, and a cached raster keys on the second — which is
            // also what makes a band *visible* rather than a private write.
            terrain.republish();
            self.terrain = Some(terrain);
            self.ground = revision;
            self.since_ground_s = 0.0;
            observation.ground = world.ground.is_some();
        }
        // **The survey's own coverage, reported whatever this observation wrote.** A write that
        // recomposed nothing is a survey whose counts stand; a reader that watched the numbers
        // climb wants the climb, not the last band's own size.
        observation.from_chunks = self.from_chunks;
        observation.from_world = self.from_world;
        observation
    }

    /// **Move the marker to the player**, once per frame. The height is the survey's own ground —
    /// what the picture shows under the marker — so a step costs one pose write and never a sample.
    pub fn follow(&mut self, spec: &WorldSpec, xz: [f64; 2], bearing_deg: f64) {
        let Some(terrain) = self.terrain.as_mut() else {
            return;
        };
        let ground = terrain
            .ground_at(xz)
            .map(|(height, _)| height)
            .unwrap_or_else(|| voxel_shapers::terrain::elevation_m(spec, xz[0], xz[1]));
        terrain.follow(xz, ground, bearing_deg);
    }

    /// **The body's elements**: the cached ground raster plus the marks, projected through the one
    /// camera. The raster is rebuilt only when the camera, the ground, the view or the body moves.
    pub fn body_elements(
        &mut self,
        camera: &MapCamera,
        rect: Rect,
        filter: &ViewFilter,
        selection: Option<[f64; 2]>,
        cursor: Option<(i32, i32)>,
    ) -> Vec<Element> {
        let Some(terrain) = self.terrain.as_ref() else {
            return Vec::new();
        };
        let key = camera.key()
            ^ terrain.revision.wrapping_mul(31)
            ^ (filter_key(filter) << 3)
            ^ rect.x.to_bits()
            ^ rect.y.to_bits().rotate_left(7)
            ^ rect.width_dp.to_bits().rotate_left(13)
            ^ rect.height_dp.to_bits().rotate_left(19);
        if self.raster.as_ref().map(|(held, _)| *held) != Some(key) {
            self.raster = Some((key, terrain.ground_elements(rect, camera, filter)));
        }
        let mut elements = self
            .raster
            .as_ref()
            .map(|(_, held)| held.clone())
            .unwrap_or_default();
        elements.extend(terrain.mark_elements(rect, camera, selection, cursor));
        elements
    }

    /// Whether a survey has been sampled — whether the map has live content to draw at all.
    pub fn observed(&self) -> bool {
        self.terrain.is_some()
    }

    /// The composed ground, for a caller that reads the survey rather than drawing it.
    pub fn terrain(&self) -> Option<&MapTerrain> {
        self.terrain.as_ref()
    }

    /// **How many samples of the survey are the world's own voxels** rather than the law's field:
    /// the residency's exact chunks, and the generator's columns beyond them.
    pub fn world_samples(&self) -> usize {
        self.from_chunks + self.from_world
    }

    /// The two counts apart, for a report that has to say which part of the world answered.
    pub fn world_split(&self) -> (usize, usize) {
        (self.from_chunks, self.from_world)
    }

    /// **Whether the far fill has read every sample of the survey** — nothing on the map is the
    /// law's field.
    pub fn whole(&self) -> bool {
        self.observed()
            && self
                .terrain
                .as_ref()
                .is_some_and(|terrain| terrain.cells() > 0)
            && self.world_samples() >= self.terrain.as_ref().map_or(0, MapTerrain::cells)
    }
}

/// **Write the world's own surface over the law's**, and answer how many samples it reached.
///
/// **Write the residency's own surface over the law's**, and answer the samples it reached.
///
/// One pass over the chunks the survey's own extent covers, so the ground is asked about the
/// ground it can draw and nothing else: a chunk is asked once and its 32 × 32 columns answer every
/// sample over it.
///
/// The answer is a **mask** as well as a count, because the far pass that follows must not ask the
/// generator about a column the streamer is holding: a chunk in hand is the live world, edits and
/// canopies and all, and it is the answer the map draws wherever there is one.
fn write_resident(terrain: &mut MapTerrain, ground: &dyn MapGround) -> (Vec<bool>, usize) {
    let columns = terrain.columns;
    let rows = terrain.rows;
    let mut held = vec![false; columns * rows];
    if columns == 0 || rows == 0 {
        return (held, 0);
    }
    let span_x = columns as f64 * terrain.cell_m;
    let span_z = rows as f64 * terrain.cell_row_m;
    let side = f64::from(CHUNK_SIDE);
    let low = |centre: f64, span: f64| ((centre - span / 2.0) / side).floor() as i32;
    let high = |centre: f64, span: f64| ((centre + span / 2.0) / side).floor() as i32;
    let (min_x, max_x) = (
        low(terrain.centre_xz[0], span_x),
        high(terrain.centre_xz[0], span_x),
    );
    let (min_z, max_z) = (
        low(terrain.centre_xz[1], span_z),
        high(terrain.centre_xz[1], span_z),
    );
    let mut from_chunks = 0usize;
    for chunk_x in min_x..=max_x {
        for chunk_z in min_z..=max_z {
            let Some(surface) = ground.chunk_surface([chunk_x, chunk_z]) else {
                continue;
            };
            if surface.is_empty() {
                continue;
            }
            // **The samples over this chunk, by index rather than by search**: the grid's own
            // arithmetic inverted, so the pass touches each of the survey's samples at most once.
            let (x_lo, x_hi) = (f64::from(chunk_x) * side, f64::from(chunk_x + 1) * side);
            let (z_lo, z_hi) = (f64::from(chunk_z) * side, f64::from(chunk_z + 1) * side);
            let first = |lo: f64, centre: f64, cell: f64, count: usize| {
                (((lo - centre) / cell) + count as f64 / 2.0 - 0.5)
                    .ceil()
                    .max(0.0) as usize
            };
            let last = |hi: f64, centre: f64, cell: f64, count: usize| {
                (((hi - centre) / cell) + count as f64 / 2.0 - 0.5)
                    .floor()
                    .clamp(0.0, count as f64 - 1.0) as usize
            };
            for column in first(x_lo, terrain.centre_xz[0], terrain.cell_m, columns)
                ..=last(x_hi, terrain.centre_xz[0], terrain.cell_m, columns)
            {
                for row in first(z_lo, terrain.centre_xz[1], terrain.cell_row_m, rows)
                    ..=last(z_hi, terrain.centre_xz[1], terrain.cell_row_m, rows)
                {
                    let x = terrain.centre_xz[0]
                        + (column as f64 + 0.5 - columns as f64 / 2.0) * terrain.cell_m;
                    let z = terrain.centre_xz[1]
                        + (row as f64 + 0.5 - rows as f64 / 2.0) * terrain.cell_row_m;
                    let local = [
                        x.floor() as i32 - chunk_x * CHUNK_SIDE,
                        z.floor() as i32 - chunk_z * CHUNK_SIDE,
                    ];
                    let Some(top) = surface.column(local) else {
                        continue;
                    };
                    let at = row * columns + column;
                    if let Some(slot) = terrain.heights.get_mut(at) {
                        *slot = top;
                    }
                    // **A sample is the residency's or it is not** — counted once however many
                    // chunks reach it: the mask is what the far pass skips by, so the count is the
                    // mask's own tally rather than a tally of writes, and the two cannot disagree
                    // about how many samples the world answered.
                    if let Some(mark) = held.get_mut(at) {
                        if !*mark {
                            *mark = true;
                            from_chunks += 1;
                        }
                    }
                }
            }
        }
    }
    (held, from_chunks)
}

/// **How many columns one ground write may ask the generator about.**
///
/// A probe measured **22 µs** (`measure_the_far_column_probe`), and a survey of the widest window
/// holds 192 × 192 = 36 864 samples: read in one write that is 0.9 s, which is not a frame. A band
/// is what a write can pay for — 1 024 columns, 22 ms at that measurement — and the count of
/// samples that came from the world climbs a band at a time until the window is the world's own.
///
/// **Cached columns are not probes**: they are written for the cost of a lookup, so a plan that
/// moves over ground the map has already read pays the band only for what is new to it.
pub const FAR_BAND_PROBES: usize = 1024;

/// **The most columns the far ground remembers**, after which it forgets them all.
///
/// The cache is a cache and this is the one rule that keeps it from growing for the life of a
/// session: a walk of a hundred kilometres at 8 m samples would otherwise hold millions of
/// columns. Forgetting costs probes and never truth — every answer is the generator's own, so a
/// forgotten column answers the same the next time it is asked.
const FAR_CACHE_MAX: usize = 65_536;

/// **The world's own ground beyond the residency**, and the answers it has already given.
///
/// Two things live here and nowhere else: the answers the generator gave, and the world *record*
/// generation they were given at. The survey asks it once per ground write; everything else in the
/// map reads the composed terrain.
struct Far {
    /// One answer per world column asked: the generator's own surface, or `None` for a column it
    /// answered nothing for — kept, so a sea column is asked once and not once per write.
    columns: std::collections::HashMap<(i32, i32), Option<f32>>,
    /// The record generation these answers were read at. An edit empties the map, because an edit
    /// is the one thing that can change what the generator says about a column.
    at: Option<u64>,
}

/// What one [`Far::write`] did: how many samples it wrote from the world's own ground, and
/// whether the survey still holds columns the generator has not been asked about.
struct FarWrite {
    written: usize,
    left: bool,
}

impl Far {
    fn new() -> Far {
        Far {
            columns: std::collections::HashMap::new(),
            at: None,
        }
    }

    /// **Write the world's own ground over the law's, and read a band of the columns that are new
    /// to it.**
    ///
    /// The walk is the whole survey in its ring order, and that is what makes the pass
    /// self-healing: a composed terrain starts from the law every time the ground is written, so a
    /// write re-establishes every column the map has already read — a lookup each, no probe — and
    /// spends its band ([`FAR_BAND_PROBES`] generator asks) on the columns that are new to it. The
    /// samples the residency wrote are skipped: a chunk in hand is the live world, edits and
    /// canopies and all.
    ///
    /// The band is what makes the fill **climb**: the columns inside it are the map's own, the
    /// ones past its end fall back to the law for now, and the next write takes the next band.
    fn write(
        &mut self,
        terrain: &mut MapTerrain,
        ground: &dyn MapGround,
        resident: &[bool],
    ) -> FarWrite {
        let columns = terrain.columns;
        let rows = terrain.rows;
        let total = terrain.cells();
        if columns == 0 || rows == 0 || total == 0 {
            return FarWrite {
                written: 0,
                left: false,
            };
        }
        // **The record's own key.** An edit is the one thing that can change a column's answer; a
        // chunk arriving at the streamer cannot.
        let generation = ground.world_generation();
        if self.at != Some(generation) {
            self.columns.clear();
            self.at = Some(generation);
        }
        // **Rectangles growing out from the middle of the picture**: the ground nearest the centre
        // of the view is the ground a viewer reads first, and the fill is visible as the picture
        // sharpening outward rather than as a stripe moving down it.
        let down = ring(rows);
        let across = ring(columns);
        let mut written = 0usize;
        let mut probes = 0usize;
        let mut left = false;
        for step in 0..total {
            let at = down[step / columns] * columns + across[step % columns];
            if resident.get(at).copied().unwrap_or(false) {
                continue;
            }
            let [x, z] =
                terrain.sample_xz((across[step % columns] as i32, down[step / columns] as i32));
            let key = (x.floor() as i32, z.floor() as i32);
            let answer = match self.columns.get(&key) {
                Some(held) => *held,
                None => {
                    if probes >= FAR_BAND_PROBES {
                        left = true;
                        continue;
                    }
                    probes += 1;
                    if self.columns.len() >= FAR_CACHE_MAX {
                        self.columns.clear();
                    }
                    let answer = ground.column_surface(key.0, key.1);
                    self.columns.insert(key, answer);
                    answer
                }
            };
            if let Some(top) = answer {
                if let Some(slot) = terrain.heights.get_mut(at) {
                    *slot = top;
                    written += 1;
                }
            }
        }
        FarWrite { written, left }
    }
}

/// **An axis walked from its middle outward**: 4 → `[2, 1, 3, 0]`, 5 → `[2, 1, 3, 0, 4]`.
fn ring(count: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(count);
    if count == 0 {
        return out;
    }
    let middle = count / 2;
    out.push(middle);
    for step in 1..count {
        let ahead = middle + step;
        let behind = middle as isize - step as isize;
        if ahead < count {
            out.push(ahead);
        }
        if behind >= 0 {
            out.push(behind as usize);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_view::{MapCamera, ViewFilter};
    use voxel_mesh::MeshConfig;
    use voxel_model::VoxelShape;
    use voxel_stream::WorldSource;
    use voxel_ui::Rect;

    /// **A ground of one declared sheet**: every chunk answers, and every column is `top_m`.
    ///
    /// The smallest thing that can tell "the map read the world" from "the map read the law": the
    /// sheet is far from any elevation the laws would give, so a survey that answers it is reading
    /// the ground and one that answers the law is not.
    struct Sheet {
        top_m: f32,
        revision: u64,
        chunks: Vec<[i32; 2]>,
    }

    impl Sheet {
        fn new(top_m: f32, chunks: Vec<[i32; 2]>) -> Sheet {
            Sheet {
                top_m,
                revision: 1,
                chunks,
            }
        }

        fn call(&mut self) {
            self.revision += 1;
        }
    }

    impl MapGround for Sheet {
        fn chunk_surface(&self, chunk: [i32; 2]) -> Option<ChunkSurface> {
            if !self.chunks.contains(&chunk) {
                return None;
            }
            let mut surface = ChunkSurface::empty();
            for top in surface.tops.iter_mut() {
                *top = self.top_m;
            }
            Some(surface)
        }

        fn ground_revision(&self) -> u64 {
            self.revision
        }
    }

    /// **A ground with a generator behind it**: the chunks it holds answer one sheet, and every
    /// column it does not hold answers the generator's own — another sheet, so a sample's height
    /// says which part of the world answered it, and how many times a column was asked is counted
    /// rather than inferred.
    struct TwoSheets {
        resident: f32,
        far: f32,
        revision: u64,
        generation: u64,
        chunks: Vec<[i32; 2]>,
        probes: std::cell::Cell<usize>,
    }

    impl TwoSheets {
        fn new(resident: f32, far: f32, chunks: Vec<[i32; 2]>) -> TwoSheets {
            TwoSheets {
                resident,
                far,
                revision: 1,
                generation: 1,
                chunks,
                probes: std::cell::Cell::new(0),
            }
        }
    }

    impl MapGround for TwoSheets {
        fn chunk_surface(&self, chunk: [i32; 2]) -> Option<ChunkSurface> {
            if !self.chunks.contains(&chunk) {
                return None;
            }
            Some(ChunkSurface::sheet(self.resident))
        }

        fn column_surface(&self, _x: i32, _z: i32) -> Option<f32> {
            self.probes.set(self.probes.get() + 1);
            Some(self.far)
        }

        fn ground_revision(&self) -> u64 {
            self.revision
        }

        fn world_generation(&self) -> u64 {
            self.generation
        }
    }

    fn body() -> Rect {
        Rect::new(0.0, 0.0, 320.0, 240.0)
    }

    fn camera(centre_m: [f64; 2], pixels_per_m: f64) -> MapCamera {
        MapCamera {
            centre_m,
            pixels_per_m,
            ..MapCamera::default()
        }
    }

    fn spec() -> WorldSpec {
        WorldSpec::new(crate::game::GAME_SEED, voxel_shapers::Config::default())
    }

    #[test]
    fn a_column_top_is_the_topmost_solid_voxel_and_a_subvoxel_shape_is_read_at_its_own_top() {
        // A chunk of rock to y = 8, and one voxel above it whose topmost solid subvoxel is the
        // third of four — so the surface is 9 + 3/4 and not 10.
        const ROCK: voxel_model::MaterialId = voxel_shapers::materials::IGNEOUS;
        let mut tree = SparseVoxelOctree::new();
        tree.set_material_region([0, 0, 0], 3, ROCK);
        let mut shape = VoxelShape::empty();
        shape.set(
            voxel_model::SubvoxelIndex::new(0, 2, 0).expect("a subvoxel of the grid"),
            ROCK,
        );
        tree.set_voxel(voxel_model::LocalPos { x: 5, y: 8, z: 7 }, shape);
        let mut tops = vec![f32::NEG_INFINITY; (CHUNK_SIDE * CHUNK_SIDE) as usize];
        absorb_surface(&mut tops, 0.0, &tree);
        let surface = ChunkSurface { tops };
        assert_eq!(surface.top([5, 7]), Some(8.0 + 0.75), "a shape's own top");
        assert_eq!(surface.top([1, 1]), Some(8.0), "the rock's own top face");
        assert_eq!(
            surface.top([20, 20]),
            None,
            "a column with no matter answers nothing"
        );
        // And the highest voxel wins, not the last one visited.
        let mut merged = vec![f32::NEG_INFINITY; 4];
        let mut low = SparseVoxelOctree::new();
        low.set_material_region([0, 0, 0], 1, ROCK);
        absorb_surface(&mut merged, 0.0, &low);
        absorb_surface(&mut merged, 32.0, &low);
        assert_eq!(merged[0], 34.0, "a higher layer wins the merge");
    }

    #[test]
    fn the_survey_reads_the_worlds_own_voxels_and_the_law_is_only_the_fallback() {
        let spec = spec();
        let plan = camera([0.0, 0.0], 1.0).survey(body(), 32.0);
        // **A sheet one kilometre above the world's own relief**, so every sample inside it is
        // unmistakably the ground's and every sample outside it is the law's.
        let mut ground = Sheet::new(1000.0, vec![[0, 0]]);
        let mut survey = MapSurvey::new();
        let observation = survey.observe(
            MapWorld {
                ground: Some(&ground),
                spec: &spec,
                tick: 0,
            },
            plan,
            1.0,
        );
        assert!(observation.law, "the first observation samples the laws");
        assert!(observation.ground, "and the world's own surface over them");
        assert!(
            observation.from_chunks > 0,
            "the sheet answered: {observation:?}"
        );
        let (columns, rows) = {
            let terrain = survey.terrain().expect("a survey");
            (terrain.columns as i32, terrain.rows as i32)
        };
        let middle = (columns / 2, rows / 2);
        assert_eq!(
            survey.terrain().expect("a survey").height(middle),
            Some(1000.0),
            "the world's own voxels, not the law"
        );
        assert_eq!(
            survey.world_samples(),
            observation.from_chunks,
            "the count is the survey's own"
        );
        // Far outside the one chunk, the survey is the law's field: the same camera, a chunk the
        // ground does not hold.
        let edge = survey.terrain().expect("a survey").height((0, 0));
        let law_at_edge = {
            let [x, z] = survey.terrain().expect("a survey").sample_xz((0, 0));
            voxel_shapers::terrain::elevation_m(&spec, x, z)
        };
        assert!(
            edge.is_some_and(|height| (height - law_at_edge).abs() < 1e-4),
            "beyond the residency the survey is the law's: {edge:?} against {law_at_edge}"
        );

        // **A dig reaches the map**: the ground moves, and the gate lets it through.
        ground.top_m = 998.0;
        ground.call();
        let observation = survey.observe(
            MapWorld {
                ground: Some(&ground),
                spec: &spec,
                tick: 0,
            },
            plan,
            1.0,
        );
        assert!(!observation.law, "the plan did not move");
        assert!(observation.ground, "the world did");
        assert_eq!(
            survey.terrain().expect("a survey").height(middle),
            Some(998.0),
            "the map is the world's own ground"
        );
        // **And the gate holds it back when the world moves faster than the survey is read.**
        ground.top_m = 996.0;
        ground.call();
        let observation = survey.observe(
            MapWorld {
                ground: Some(&ground),
                spec: &spec,
                tick: 0,
            },
            plan,
            0.0,
        );
        assert!(!observation.ground, "a quarter of a second has not passed");
        assert_eq!(
            survey.terrain().expect("a survey").height(middle),
            Some(998.0),
            "the last survey stands until the gate opens"
        );
        let observation = survey.observe(
            MapWorld {
                ground: Some(&ground),
                spec: &spec,
                tick: 0,
            },
            plan,
            MapSurvey::GROUND_INTERVAL_S,
        );
        assert!(observation.ground, "and then it opens");
        assert_eq!(
            survey.terrain().expect("a survey").height(middle),
            Some(996.0)
        );
    }

    /// **The world is read whole, not only where a chunk is held.** A survey far wider than the
    /// residency: the samples over the held chunk are the residency's own sheet, and every other
    /// sample is the generator's own — so the map draws the world to its edges and the law is
    /// nowhere in the picture.
    #[test]
    fn the_far_ground_is_the_generators_own_and_the_residency_wins_where_it_holds_one() {
        let spec = spec();
        // 8 m samples over a body that shows 512 m: the one held chunk is 32 m of it.
        let plan = camera([0.0, 0.0], 1.0).survey(Rect::new(0.0, 0.0, 512.0, 400.0), 32.0);
        let mut ground = TwoSheets::new(2000.0, 3000.0, vec![[0, 0]]);
        let mut survey = MapSurvey::new();
        let mut observation = Observation::default();
        // Fill the whole survey: one band a gate, and the gate is the survey's own interval.
        for _ in 0..64 {
            observation = survey.observe(
                MapWorld {
                    ground: Some(&ground),
                    spec: &spec,
                    tick: 0,
                },
                plan,
                MapSurvey::GROUND_INTERVAL_S,
            );
            if observation.whole() {
                break;
            }
        }
        assert!(
            observation.whole(),
            "the fill climbed to the whole survey: {observation:?}"
        );
        let (resident, far) = survey.world_split();
        assert!(resident > 0, "the held chunk answered: {observation:?}");
        assert_eq!(
            resident + far,
            observation.samples,
            "every sample is the world's own: {observation:?}"
        );
        // **Both parts of the world are in the picture, and each in its own place**: the sheet
        // the chunk holds over the chunk, the generator's sheet everywhere else. Neither is the
        // law's elevation, which is what an unwritten sample would have been.
        let terrain = survey.terrain().expect("a survey");
        let heights: Vec<f64> = terrain
            .heights
            .iter()
            .map(|height| f64::from(*height))
            .collect();
        assert!(
            heights.iter().any(|height| (*height - 2000.0).abs() < 1e-3),
            "the residency is drawn"
        );
        assert!(
            heights.iter().any(|height| (*height - 3000.0).abs() < 1e-3),
            "the generator is drawn"
        );
        assert!(
            !heights.iter().any(|height| {
                let law = voxel_shapers::terrain::elevation_m(&spec, 0.0, 0.0);
                (*height - law).abs() < 1e-3
            }),
            "no sample is the law's field"
        );
        // **And the same plan is not read again**: the fill is done, so the next observations
        // ask the generator nothing.
        let before = ground.probes.get();
        for _ in 0..8 {
            survey.observe(
                MapWorld {
                    ground: Some(&ground),
                    spec: &spec,
                    tick: 0,
                },
                plan,
                MapSurvey::GROUND_INTERVAL_S,
            );
        }
        assert_eq!(
            ground.probes.get(),
            before,
            "a settled survey re-reads no column"
        );
        // **An edit does re-read them**: the world's record moved, so every cached answer is
        // stale — and the map must not draw a block the player has since dug away.
        ground.generation += 1;
        ground.revision += 1;
        survey.observe(
            MapWorld {
                ground: Some(&ground),
                spec: &spec,
                tick: 0,
            },
            plan,
            MapSurvey::GROUND_INTERVAL_S,
        );
        assert!(ground.probes.get() > before, "an edit re-reads the world");
    }

    /// **The far ground arrives in bands**: a survey wider than one band is not the world's own
    /// in its first observation, the count climbs a band at a time, and it reaches the whole
    /// survey without the plan ever moving.
    #[test]
    fn the_far_fill_climbs_a_band_at_a_time_to_whole() {
        let spec = spec();
        // 64 x 64 samples at 8 m = 4096, four bands of the survey's own budget.
        let plan = camera([0.0, 0.0], 1.0).survey(Rect::new(0.0, 0.0, 512.0, 512.0), 32.0);
        assert!(
            plan.columns * plan.rows > FAR_BAND_PROBES,
            "the plan spans more than one band"
        );
        fn world<'a>(spec: &'a WorldSpec, ground: &'a TwoSheets) -> MapWorld<'a> {
            MapWorld {
                ground: Some(ground),
                spec,
                tick: 0,
            }
        }
        let ground = TwoSheets::new(0.0, 3000.0, Vec::new());
        let mut survey = MapSurvey::new();
        let first = survey.observe(world(&spec, &ground), plan, MapSurvey::GROUND_INTERVAL_S);
        assert!(first.law && first.ground);
        assert!(first.from_world > 0, "the first band is read");
        assert!(
            !first.whole(),
            "one band is not the whole survey: {first:?}"
        );
        let second = survey.observe(world(&spec, &ground), plan, MapSurvey::GROUND_INTERVAL_S);
        assert!(
            second.from_world > first.from_world,
            "the count climbs: {second:?}"
        );
        assert!(!second.whole());
        let mut last = second;
        for _ in 0..64 {
            last = survey.observe(world(&spec, &ground), plan, MapSurvey::GROUND_INTERVAL_S);
            if last.whole() {
                break;
            }
        }
        assert!(last.whole(), "the fill reaches the whole survey: {last:?}");
        assert_eq!(
            survey.world_samples(),
            last.samples,
            "and the survey says so"
        );
    }

    /// **A sample on a chunk boundary belongs to one chunk, and it is counted once.** A survey grid
    /// can land a sample's centre exactly on a chunk edge — 9 samples of 8 m across, centred on the
    /// origin, put one at x = 32 — and the chunk below the edge used to answer it from the *wrapped*
    /// end of its own column grid: a real height, from the wrong column, for a sample the chunk above
    /// the edge was answering too. Two writes, one sample, and the survey's own tally of the world's
    /// ground came out **larger than the survey** — which is what a run of the game found, as an
    /// arithmetic overflow in the line that reports it, at the moment the fill first reached whole.
    #[test]
    fn a_sample_on_a_chunk_boundary_is_one_chunks_own_and_is_counted_once() {
        let spec = spec();
        // **The plan is built rather than derived**, because this is a claim about where a sample
        // lands: 8 m samples, 9 columns, centred on the origin — so column 8 is centred at x = 32,
        // the edge between chunk 0 and chunk 1, and row 4 is centred at z = 4, inside chunk row 0.
        let plan = SurveyPlan {
            centre_xz: [0.0, 0.0],
            sample_m: 8.0,
            declared_m: 32.0,
            columns: 9,
            rows: 8,
        };
        assert_eq!(
            MapTerrain::sample(&spec, 0, plan).sample_xz((8, 4)),
            [32.0, 4.0],
            "the sample whose centre is on the boundary"
        );
        let ground = TwoSheets::new(2000.0, 3000.0, vec![[0, 0], [1, 0]]);
        let mut survey = MapSurvey::new();
        let mut observation = Observation::default();
        for _ in 0..8 {
            observation = survey.observe(
                MapWorld {
                    ground: Some(&ground),
                    spec: &spec,
                    tick: 0,
                },
                plan,
                MapSurvey::GROUND_INTERVAL_S,
            );
            assert!(
                observation.world_samples() <= observation.samples,
                "the world cannot have answered more samples than the survey holds: {observation:?}"
            );
        }
        assert!(
            observation.whole(),
            "the fill reaches the whole survey, boundary samples and all: {observation:?}"
        );
        // The sample itself is the chunk *above* the edge's own column — its local x is 0 — so the
        // map's height there is the resident sheet's, and not the wrapped column the lower chunk
        // used to answer with.
        assert_eq!(
            survey.terrain().expect("a survey").height((8, 4)),
            Some(2000.0),
            "the world's own surface, from the chunk that owns the column"
        );
    }

    /// **A chunk answers nothing outside its own 32 × 32 columns.** The neighbour owns that metre;
    /// answering the wrapped entry made the surface a fact about the wrong column.
    #[test]
    fn a_chunks_surface_refuses_a_column_outside_its_own_grid() {
        let mut surface = ChunkSurface::empty();
        for top in surface.tops.iter_mut() {
            *top = 12.0;
        }
        assert_eq!(surface.top([CHUNK_SIDE - 1, CHUNK_SIDE - 1]), Some(12.0));
        assert_eq!(
            surface.top([CHUNK_SIDE, 0]),
            None,
            "one past the right edge"
        );
        assert_eq!(
            surface.top([0, CHUNK_SIDE]),
            None,
            "one past the bottom edge"
        );
        assert_eq!(surface.top([-1, 0]), None);
    }

    /// **The far ground is the streamer's own generator, and it agrees with the chunks the
    /// streamer holds where the two overlap.** The seam the old law-fallback left — the residency
    /// drawn from voxels and the ground beyond it from a smooth field — is what this pins shut.
    #[test]
    fn the_probe_agrees_with_the_resident_chunks_it_overlaps() {
        let spec = spec();
        let registry = voxel_shapers::materials::registry().expect("the material registry");
        let config = MeshConfig::from_registry(&registry).expect("the mesh config");
        let mut streamer =
            Streamer::new(WorldSource::new(spec.clone()), config, 2, 128 * 1024 * 1024)
                .expect("a streamer")
                .with_window(voxel_work::ChunkWindow {
                    radius_chunks: 1,
                    vertical_chunks: 1,
                })
                .expect("a three-by-three window");
        streamer.set_centre(ChunkPos::new(0, 0, 0));
        streamer.settle().expect("the world settles");

        // The residency's own answer for the chunk the columns below stand in.
        let held = streamer
            .chunk_surface([0, 0])
            .expect("the middle chunk is resident");
        let mut agreed = 0usize;
        let mut differed_from_law = 0usize;
        for x in (0..32).step_by(5) {
            for z in (0..32).step_by(5) {
                let probe = streamer
                    .column_surface(x, z)
                    .expect("the generator answers");
                let resident = held.top([x, z]);
                assert_eq!(
                    probe, resident,
                    "the probe and the chunk disagree at ({x}, {z}): {probe:?} against {resident:?}"
                );
                if let Some(top) = probe {
                    agreed += 1;
                    let law =
                        voxel_shapers::terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5);
                    if (f64::from(top) - law).abs() > 1e-4 {
                        differed_from_law += 1;
                    }
                }
            }
        }
        assert!(agreed > 0, "no column answered");
        assert!(
            differed_from_law > 0,
            "every probed column agreed with the elevation law, which means the law was read"
        );
        // **And beyond the residency the same call answers**, where no chunk was ever built:
        // a column a kilometre away is still the world's own surface, not the law's field.
        let far = streamer
            .column_surface(1000, -1000)
            .expect("the generator answers far outside the window");
        assert!(far.is_some(), "a column a kilometre away still has ground");
        let law = voxel_shapers::terrain::elevation_m(&spec, 1000.5, -999.5);
        assert!(
            (f64::from(far.expect("answered")) - law).abs() > 1e-4,
            "the far column is the world's own surface and not the law: {far:?} against {law}"
        );
    }

    /// **What the far ground costs**, measured rather than guessed: one column probe, and a
    /// whole survey's worth of them, against the same settled world the live tests use.
    ///
    /// The band the survey fills per observation is chosen from this number, so it is kept
    /// beside the code that spends it.
    #[test]
    #[ignore = "measurement, not an assertion: run with `cargo test <filter> -- --ignored --nocapture`"]
    fn measure_the_far_column_probe() {
        use std::time::Instant;
        let spec = spec();
        let registry = voxel_shapers::materials::registry().expect("the material registry");
        let config = MeshConfig::from_registry(&registry).expect("the mesh config");
        let mut streamer = Streamer::new(WorldSource::new(spec), config, 4, 256 * 1024 * 1024)
            .expect("a streamer")
            .with_window(voxel_work::ChunkWindow {
                radius_chunks: 0,
                vertical_chunks: 1,
            })
            .expect("a one-chunk window");
        streamer.set_centre(ChunkPos::new(0, 0, 0));
        streamer.settle().expect("the world settles");

        // One column, hot: the probe's own cost with nothing else in the measurement.
        let columns: Vec<(i32, i32)> = (0..64)
            .flat_map(|dx| (0..64).map(move |dz| (dx * 7, dz * 7)))
            .collect();
        let t = Instant::now();
        let mut answered = 0usize;
        let mut sum = 0.0f64;
        for (x, z) in &columns {
            if let Some(top) = streamer
                .column_surface(*x, *z)
                .expect("the generator answers")
            {
                answered += 1;
                sum += f64::from(top);
            }
        }
        let per_column = t.elapsed().as_secs_f64() * 1e6 / columns.len() as f64;
        println!(
            "column probe: {:.1} µs a column, {answered} of {} answered, mean top {:.1} m",
            per_column,
            columns.len(),
            sum / answered.max(1) as f64
        );

        // **A whole survey's worth**, at the two sizes the live tests take their pictures at:
        // 8 m samples over the window the game's own map draws.
        for (label, columns, rows) in [
            ("480 × 440 dp", 64usize, 58usize),
            ("1920 × 1080 dp", 192, 192),
        ] {
            let plan = SurveyPlan {
                centre_xz: [0.0, 0.0],
                sample_m: 8.0,
                declared_m: 32.0,
                columns,
                rows,
            };
            let t = Instant::now();
            let mut answered = 0usize;
            for row in 0..rows {
                for column in 0..columns {
                    let x = (column as f64 - columns as f64 / 2.0) * 8.0 + 4.0;
                    let z = (row as f64 - rows as f64 / 2.0) * 8.0 + 4.0;
                    if streamer
                        .column_surface(x.floor() as i32, z.floor() as i32)
                        .expect("the generator answers")
                        .is_some()
                    {
                        answered += 1;
                    }
                }
            }
            let ms = t.elapsed().as_secs_f64() * 1e3;
            println!(
                "{label}: {columns} × {rows} = {} column(s) in {ms:.1} ms ({:.2} ms per 1,000), {answered} answered",
                columns * rows,
                ms * 1000.0 / (columns * rows) as f64
            );
        }
    }

    #[test]
    fn a_world_with_no_ground_is_a_survey_of_the_laws() {
        // The host with no streamer: the picture is the law's field and the survey says so.
        let spec = spec();
        let plan = camera([0.0, 0.0], 1.0).survey(body(), 32.0);
        let mut survey = MapSurvey::new();
        let observation = survey.observe(
            MapWorld {
                ground: None,
                spec: &spec,
                tick: 0,
            },
            plan,
            1.0,
        );
        assert!(observation.law);
        assert!(!observation.ground);
        assert_eq!(observation.from_chunks, 0);
        let terrain = survey.terrain().expect("a survey");
        let [x, z] = terrain.sample_xz((1, 1));
        let law = voxel_shapers::terrain::elevation_m(&spec, x, z);
        let height = terrain.height((1, 1)).expect("the sample grounds");
        // The survey holds its heights as `f32` (a map sample is not a surveyor's rod), so the
        // comparison is to a sample's precision rather than to the last bit.
        assert!(
            (height - law).abs() < 1e-4,
            "the law, and nothing else: {height} against {law}"
        );
    }

    /// **The real streamer is the map's ground.** A live world, settled, and the map's answer
    /// compared with the world's own trees column by column: the surface the map draws is the
    /// topmost solid voxel the streamer holds, not a re-derivation of the law that made it.
    #[test]
    fn a_live_streamers_own_voxels_are_what_the_map_draws() {
        let spec = spec();
        let registry = voxel_shapers::materials::registry().expect("the material registry");
        let config = MeshConfig::from_registry(&registry).expect("the mesh config");
        let mut streamer = Streamer::new(WorldSource::new(spec), config, 4, 256 * 1024 * 1024)
            .expect("a streamer")
            .with_window(voxel_work::ChunkWindow {
                radius_chunks: 0,
                vertical_chunks: 1,
            })
            .expect("a one-chunk window");
        streamer.set_centre(ChunkPos::new(0, 0, 0));
        let frame = streamer.settle().expect("the world settles");
        assert!(frame.resident > 0, "a resident chunk to read: {frame:?}");

        // The camera looks straight at the chunk's middle, close enough that the survey is the
        // chunk rather than the kilometres around it.
        let centre = [16.0, 16.0];
        let plan = camera(centre, 6.0).survey(Rect::new(0.0, 0.0, 240.0, 200.0), 32.0);
        let mut survey = MapSurvey::new();
        let observation = survey.observe(
            MapWorld {
                ground: Some(&streamer),
                spec: &spec,
                tick: 0,
            },
            plan,
            1.0,
        );
        assert!(observation.from_chunks > 0, "the chunk is the survey now");
        let terrain = survey.terrain().expect("a survey");

        // **An independent walk of the streamer's own trees**, one voxel at a time, is the
        // witness: the map's height for a column is the topmost solid voxel's top face.
        let mut checked = 0usize;
        for column in 0..terrain.columns {
            for row in 0..terrain.rows {
                let cell = (column as i32, row as i32);
                let Some(height) = terrain.height(cell) else {
                    continue;
                };
                // The sample's own column, not the declared cell it is named as: the walk has to
                // ask the world about the place the survey read.
                let [x, z] = terrain.sample_xz(cell);
                let (vx, vz) = (x.floor() as i32, z.floor() as i32);
                // **The same layers the ground merges** — the streamer's window holds one chunk
                // above and below — read top down, and **the same rule**: the topmost subvoxel
                // that is neither empty nor a fluid, at its own quarter-metre height.
                let mut expected = f64::NEG_INFINITY;
                for layer in [1, 0, -1] {
                    let pos =
                        ChunkPos::new(vx.div_euclid(CHUNK_SIDE), layer, vz.div_euclid(CHUNK_SIDE));
                    let Some(tree) = streamer.tree(pos) else {
                        continue;
                    };
                    for y in (0..CHUNK_SIDE).rev() {
                        let local = voxel_model::LocalPos {
                            x: vx.rem_euclid(CHUNK_SIDE) as u32,
                            y: y as u32,
                            z: vz.rem_euclid(CHUNK_SIDE) as u32,
                        };
                        let shape = tree.voxel(local);
                        let top = shape
                            .cells()
                            .iter()
                            .enumerate()
                            .filter(|(_, material)| {
                                !material.is_empty()
                                    && !voxel_shapers::materials::is_fluid(**material)
                            })
                            .map(|(ordinal, _)| (ordinal / 4) % 4)
                            .max();
                        let Some(subvoxel) = top else { continue };
                        expected = f64::from(pos.y * CHUNK_SIDE + y)
                            + (subvoxel + 1) as f64 / f64::from(SUBVOXEL_SIDE);
                        break;
                    }
                    if expected.is_finite() {
                        break;
                    }
                }
                if !expected.is_finite() {
                    continue;
                }
                assert_eq!(
                    height, expected,
                    "the map's height at {vx},{vz} is not the world's own topmost solid voxel"
                );
                checked += 1;
                if checked > 64 {
                    break;
                }
            }
            if checked > 64 {
                break;
            }
        }
        assert!(checked > 0, "no column of the survey was checked");
        // **And the law is not what it read**: the world's surface is the voxel's top face, which
        // a smooth field does not agree with everywhere.
        let differs = (0..terrain.columns).any(|column| {
            (0..terrain.rows).any(|row| {
                let cell = (column as i32, row as i32);
                let Some(height) = terrain.height(cell) else {
                    return false;
                };
                let [x, z] = terrain.sample_xz(cell);
                (height - voxel_shapers::terrain::elevation_m(&spec, x, z)).abs() > 1e-4
            })
        });
        assert!(
            differs,
            "every sample agreed with the law to the last bit, which means the laws were read"
        );
    }

    /// **A hammer stroke is what the map's ground reads back.** The stroke is recorded through the
    /// streamer — the same door the game uses — and the next survey, after the chunk it reached is
    /// rebuilt, draws the surface the stroke left. This is the claim the mission's map edit rests on:
    /// the picture is the world, so the world changing is the picture changing.
    #[test]
    fn a_hammer_stroke_reads_back_as_the_maps_own_ground() {
        // **A world with dry land in its first chunk.** The game's own seed puts this chunk under
        // the sea, and the world refuses a hand that places into water — correctly — so the
        // column this test strikes has to be above the waterline.
        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        let registry = voxel_shapers::materials::registry().expect("the material registry");
        let config = MeshConfig::from_registry(&registry).expect("the mesh config");
        let mut streamer = Streamer::new(WorldSource::new(spec), config, 4, 256 * 1024 * 1024)
            .expect("a streamer")
            .with_window(voxel_work::ChunkWindow {
                radius_chunks: 0,
                vertical_chunks: 1,
            })
            .expect("a one-chunk window");
        streamer.set_centre(ChunkPos::new(0, 0, 0));
        streamer.settle().expect("the world settles");
        let plan = camera([16.0, 16.0], 6.0).survey(Rect::new(0.0, 0.0, 240.0, 200.0), 32.0);
        let mut survey = MapSurvey::new();
        survey.observe(
            MapWorld {
                ground: Some(&streamer),
                spec: &spec,
                tick: 0,
            },
            plan,
            1.0,
        );
        // **A column of the survey's own, and a dry one**: the highest sample the window holds,
        // because a stroke over ground below the sea is refused by the world's own rule (a hand
        // may not place into water) and this test is about the map reading a stroke back.
        let terrain = survey.terrain().expect("a survey");
        let mut highest = ((0, 0), f64::MIN);
        for column in 0..terrain.columns {
            for row in 0..terrain.rows {
                let cell = (column as i32, row as i32);
                if let Some(height) = terrain.height(cell) {
                    if height > highest.1 {
                        highest = (cell, height);
                    }
                }
            }
        }
        let (cell, before) = highest;
        assert!(
            before > f64::from(spec.config.sea_level_m),
            "the survey holds no dry column to strike: {before}"
        );
        let [x, z] = terrain.sample_xz(cell);
        let column = [x.floor() as i32, z.floor() as i32];

        let shaped = streamer
            .shape_columns(&[column], 3, "a map's hammer")
            .expect("the stroke records");
        assert_eq!(shaped.raised, 3, "{shaped:?}");
        // **The world rebuilds, then the map reads it** — the gate is the survey's own quarter
        // second, spent here rather than slept through.
        streamer.settle().expect("the edited chunk rebuilds");
        let observation = survey.observe(
            MapWorld {
                ground: Some(&streamer),
                spec: &spec,
                tick: 0,
            },
            plan,
            MapSurvey::GROUND_INTERVAL_S,
        );
        assert!(
            observation.ground,
            "the ground was not rewritten: {observation:?}"
        );
        let after = survey
            .terrain()
            .and_then(|terrain| terrain.height(cell))
            .expect("the middle sample still grounds");
        assert!(
            (after - (before + 3.0)).abs() < 1e-4,
            "the map draws {after} over a column the stroke raised three metres from {before}"
        );
    }

    /// **A stroke reads back at the live game's own column.** A streamer over the game's seed and
    /// window — the row of chunks the live map's camera looks at — with a 16 m footprint raised
    /// eight metres. This is the exact scene the frame loop drives, and it is here because the live
    /// run is slow to bisect: if this passes and the game's map does not move, the fault is in the
    /// wiring rather than in the world.
    #[test]
    fn the_live_maps_own_stroke_reads_back_at_the_column_it_struck() {
        let spec = spec();
        let registry = voxel_shapers::materials::registry().expect("the material registry");
        let config = MeshConfig::from_registry(&registry).expect("the mesh config");
        let mut streamer = Streamer::new(WorldSource::new(spec), config, 4, 512 * 1024 * 1024)
            .expect("a streamer")
            .with_window(voxel_work::ChunkWindow {
                radius_chunks: 2,
                vertical_chunks: 3,
            })
            .expect("a window");
        // The chunk the live run's brush stood in: x = 16, z = −48.
        streamer.set_centre(ChunkPos::new(0, 0, -2));
        streamer.settle().expect("the world settles");
        let plan = camera([16.0, -48.0], 7.2).survey(Rect::new(0.0, 0.0, 336.0, 350.0), 32.0);
        let mut survey = MapSurvey::new();
        survey.observe(
            MapWorld {
                ground: Some(&streamer),
                spec: &spec,
                tick: 0,
            },
            plan,
            1.0,
        );
        let terrain = survey.terrain().expect("a survey");
        // **The sample nearest the stroke's centre** — the survey's grid is centred on the plan's
        // own snapped column, which is not the brush's; a middle sample can be twenty metres off.
        let mut nearest = ((0, 0), f64::MAX);
        for column in 0..terrain.columns {
            for row in 0..terrain.rows {
                let cell = (column as i32, row as i32);
                let [x, z] = terrain.sample_xz(cell);
                let away = (x - 16.0).hypot(z + 48.0);
                if away < nearest.1 {
                    nearest = (cell, away);
                }
            }
        }
        let (cell, away) = nearest;
        assert!(
            away <= 16.0,
            "the nearest sample is {away} m from the stroke"
        );
        let before = terrain.height(cell).expect("the sample grounds");
        // The struck sample's own column, so the law can be asked at the same place the survey is.
        let [sx, sz] = terrain.sample_xz(cell);
        let columns = crate::map_view::columns_within([16.0, -48.0], 16.0);
        let shaped = streamer
            .shape_columns(&columns, 8, "a map's hammer")
            .expect("the stroke records");
        assert_eq!(shaped.refused, 0, "{shaped:?}");
        streamer.settle().expect("the edited chunks rebuild");
        let observation = survey.observe(
            MapWorld {
                ground: Some(&streamer),
                spec: &spec,
                tick: 0,
            },
            plan,
            MapSurvey::GROUND_INTERVAL_S,
        );
        assert!(observation.ground, "{observation:?}");
        let after = survey
            .terrain()
            .and_then(|terrain| terrain.height(cell))
            .expect("the sample still grounds");
        assert!(
            (after - (before + 8.0)).abs() < 1e-4,
            "the map draws {after} over a column the stroke raised eight metres from {before}"
        );
        // **And the edit survives a resample of the laws.** The climate is keyed on the bucketed
        // clock, so the next bucket re-reads every law's field under the plan — the moment the
        // live run prints `laws resampled`. The stroke is the *world's*, not the law's, so the
        // picture must not move back to the elevation the generator would have put there: three
        // passes (the laws, the residency, the far fill) all run again and the ground is still the
        // stroke's.
        let resampled = survey.observe(
            MapWorld {
                ground: Some(&streamer),
                spec: &spec,
                tick: TICK_BUCKET,
            },
            plan,
            MapSurvey::GROUND_INTERVAL_S,
        );
        assert!(
            resampled.law,
            "the next climate bucket is a resample: {resampled:?}"
        );
        let held = survey
            .terrain()
            .and_then(|terrain| terrain.height(cell))
            .expect("the sample still grounds after the resample");
        assert!(
            (held - (before + 8.0)).abs() < 1e-4,
            "the resample drew {held} over the stroke's {after}"
        );
        assert!(
            held > voxel_shapers::terrain::elevation_m(&spec, sx, sz),
            "the resample fell back to the law at the struck column"
        );
    }

    #[test]
    fn the_body_raster_is_cached_under_the_camera_the_ground_and_the_view() {
        let spec = spec();
        let plan = camera([0.0, 0.0], 1.0).survey(body(), 32.0);
        let mut survey = MapSurvey::new();
        let ground = Sheet::new(100.0, vec![[0, 0]]);
        survey.observe(
            MapWorld {
                ground: Some(&ground),
                spec: &spec,
                tick: 0,
            },
            plan,
            1.0,
        );
        survey.follow(&spec, [4.0, 4.0], 90.0);
        let camera = camera([0.0, 0.0], 1.0);
        let drawn = survey.body_elements(&camera, body(), &ViewFilter::Any, None, None);
        assert!(!drawn.is_empty(), "the body drew");
        // The same question is the same answer, and the marks ride the same camera.
        let again = survey.body_elements(&camera, body(), &ViewFilter::Any, None, None);
        assert_eq!(drawn, again);
        let marked =
            survey.body_elements(&camera, body(), &ViewFilter::Any, Some([8.0, 8.0]), None);
        assert_ne!(drawn, marked, "a selection is drawn");
        assert!(marked.iter().any(|element| element.text() == Some("@")));
        // **A selection key is part of the cache key**: two views that keep different ground
        // cannot share a picture.
        let alpine = survey.body_elements(
            &camera,
            body(),
            &ViewFilter::of_token(Some("mountain")),
            None,
            None,
        );
        assert_ne!(drawn, alpine, "the view changed the picture");
    }
}
