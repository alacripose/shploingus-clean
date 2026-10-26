//! **The Sun's light that reached a cell** — sampled through the world's own octree,
//! at the world's own subvoxel addresses.
//!
//! The shading model has had a per-cell light field since `Scene::build_with_light`
//! (`shading.wgsl` reads it as `light_present`), and nothing in the tree ever
//! **measured** one: the world's blocks were lit by a scene-wide rig, so a cell in a
//! cave mouth and a cell on an open ridge received the same Sun. This module is the
//! measurement.
//!
//! # What it walks, and why that is the octree
//!
//! A cell's light is decided by what stands between it and the Sun, so the question
//! is a **ray** question, and the answer has to be the world's real geometry or it is
//! a decoration. [`voxel_mesh::raycast::Crossings`] is Amanatides & Woo's traversal —
//! the same walk `SparseVoxelOctree::raycast_local` uses for picking — and it visits
//! *only* the voxels the ray passes through, in order, with no bound to choose. Above
//! it, the provider answers one **subvoxel address** at a time
//! ([`Matter::subvoxel`]), which is what makes a dug-out voxel shadow a beam where it
//! is solid and not where it has been removed: the address is the material system's
//! own (`SubvoxelIndex` into a `VoxelShape`), not a second grid invented here.
//!
//! # The three laws, and the one absence
//!
//! A **material** either **stops** the beam or **attenuates** it at a rate per metre
//! ([`Blocking`]). The world's own records carry no absorption coefficient
//! (`APPEARANCE.md` §5: absent, never defaulted, and the spec's Beer-Lambert wants a
//! `σ = −ln(T_ref)/d_ref` this tree has not been given), so the honest default is
//! [`Matter::blocking`]'s: **matter stops the beam.** The attenuating arm exists, is
//! tested, and is what a record with a measured extinction will use — it is not a
//! placeholder for a number nobody has.
//!
//! A **medium** is the third case, and it is not a material with a small rate.
//! `voxel_model::OpticalClass::Scattering` says what the difference is: a material has an
//! *interface*, so a beam is stopped at it or bent by it, while a medium has **no surface
//! at all** and the beam is attenuated *along the path inside it*. That is why the medium
//! is a second question on [`Matter`] ([`Matter::medium`]) rather than another [`Blocking`]:
//! a cloud cell's rate changes with where the cell is (density is highest in the middle of
//! the layer and zero at its base and top), and a rate carried by a *material id* could not
//! say so. The rate a medium reports is already the address's own, so the walk needs no
//! extra structure — and the cloud is the first provider (`voxel_client::cloud_field`).
//!
//! # Units
//!
//! Everything here is a **fraction in `[0, 1]`**: the transmittance along the beam,
//! and the sky's own visibility. Turning that into the bands `shading.wgsl` reads is
//! `voxel-client::skylight`'s job, because the sky's radiance and its normaliser live
//! there — a second copy of them here would be a second atmosphere.
//!
//! **Refuse, never default**, in this module's own form: a degenerate ray returns
//! `0.0` and not `1.0`. An unmeasurable beam is not an unobstructed one, and a
//! sampler that answered "fully lit" to a NaN would put a bright cell in a stone wall.

use std::f64::consts::FRAC_1_SQRT_2;

use voxel_mesh::octree::SparseVoxelOctree;
use voxel_mesh::raycast::{Crossings, Ray};
use voxel_model::{MaterialId, SubvoxelIndex, SUBVOXEL_SIDE};

/// What the world is, at one **subvoxel address**.
///
/// `None` is *nothing there* — air, and not a material with no colour. The whole
/// sampler rests on that distinction: a voxel the ray crosses with nothing in it
/// attenuates nothing, and one with matter in it does whatever [`Matter::blocking`]
/// says.
pub trait Matter {
    /// The material at `world`'s subvoxel `at`, or `None` where that address holds
    /// nothing.
    fn subvoxel(&self, world: [i32; 3], at: SubvoxelIndex) -> Option<MaterialId>;

    /// What the beam does when it meets `material`.
    ///
    /// The default is the honest one for a record with no absorption in it — **matter
    /// stops the beam** — and a provider with a measured extinction overrides it. The
    /// per-material question is deliberately *here* rather than in the walk: which
    /// material a ray met is an addressing question, and what the light does in it is
    /// an optics question, and the two change for different reasons.
    fn blocking(&self, material: MaterialId) -> Blocking {
        let _ = material;
        Blocking::Stops
    }

    /// The **participating medium** at `world`'s subvoxel `at`: its **extinction rate per
    /// metre**, or `None` where that address holds no medium.
    ///
    /// A medium is not a material and the trait keeps them apart for the reason
    /// [`Matter::blocking`] is per-material and this is per-address: a medium's rate is a
    /// function of *where* the beam is (a cloud's water), not of *what* it met. It takes
    /// precedence over [`Matter::subvoxel`], because a cell that holds a medium has no
    /// surface for a material to describe — so an address answers one question or the
    /// other, never both at once.
    ///
    /// A rate that is not a rate (`NaN`, negative) is a **refusal**, the same rule
    /// [`Blocking::Attenuates`] carries; `0.0` is legal and means clear. The default is
    /// `None`, which is every provider that existed before this question did.
    fn medium(&self, world: [i32; 3], at: SubvoxelIndex) -> Option<f64> {
        let _ = (world, at);
        None
    }
}

/// **A chunk's own octree, answering [`Matter`]** — the provider that travels beside a tree.
///
/// A resident chunk's world *is* its tree: the tree carries the authored subvoxels (a dug voxel
/// is absent from it), so it answers the addressing question without a second trip to the
/// generator, and [`Matter::blocking`]'s own default — **matter stops the beam** — is the truth
/// about records with no absorption in them (`APPEARANCE.md` §5).
///
/// `origin` is the chunk's own corner in **voxels**, because the addressing question is asked in
/// world coordinates while the tree is a shape: this is the one place that subtraction lives, so
/// no caller has to remember it twice.
///
/// `sigma` is the declared attenuating arm: `None` for the world's own rule, `Some(σ)` to charge a
/// measured extinction per metre instead of stopping the beam. It exists because `Blocking`'s two
/// arms are one law read twice and the walk must be held to both — a walk that only ever returned
/// `0.0` or `1.0` would never do the arithmetic.
pub struct ChunkMatter<'a> {
    tree: &'a SparseVoxelOctree,
    origin: [i32; 3],
    sigma: Option<f64>,
}

impl<'a> ChunkMatter<'a> {
    /// The world's own rule: matter stops the beam.
    pub fn new(tree: &'a SparseVoxelOctree, origin: [i32; 3]) -> Self {
        ChunkMatter { tree, origin, sigma: None }
    }

    /// The same provider, charging `sigma` per metre instead of stopping the beam.
    pub fn attenuating(tree: &'a SparseVoxelOctree, origin: [i32; 3], sigma: f64) -> Self {
        ChunkMatter { tree, origin, sigma: Some(sigma) }
    }
}

impl Matter for ChunkMatter<'_> {
    fn subvoxel(&self, world: [i32; 3], at: SubvoxelIndex) -> Option<MaterialId> {
        let local = voxel_mesh::octree::local_from([
            world[0] - self.origin[0],
            world[1] - self.origin[1],
            world[2] - self.origin[2],
        ])
        .ok()?;
        let material = self.tree.material_at(local, at);
        (!material.is_empty()).then_some(material)
    }

    fn blocking(&self, _material: MaterialId) -> Blocking {
        match self.sigma {
            Some(sigma) => Blocking::Attenuates(sigma),
            None => Blocking::Stops,
        }
    }
}

/// Any closure is a provider, so a test (or a caller with its own resident chunks) can
/// inline one.
impl<F> Matter for F
where
    F: Fn([i32; 3], SubvoxelIndex) -> Option<MaterialId>,
{
    fn subvoxel(&self, world: [i32; 3], at: SubvoxelIndex) -> Option<MaterialId> {
        self(world, at)
    }
}

/// What matter does to a beam.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Blocking {
    /// The beam does not get past it.
    Stops,
    /// The beam is attenuated at this **rate per metre** of the material it crosses:
    /// `exp(−σ·d)`, the spec's Beer-Lambert (`OpenPBR_Energy_Conservation.pdf` §10),
    /// applied to the path through the material rather than to a surface.
    Attenuates(f64),
}

/// The **sky's** own visibility: a fixed stencil of upward rays, in world space.
///
/// The dome is a hemisphere, so one vertical ray is not a dome. Five is still not a
/// hemisphere, and saying so is the point: this is a declared sample, not an integral,
/// and every cell in the world is measured with the *same* stencil so two cells' skies
/// are comparable. The four tilted rays carry `1/√2` in the horizontal and vertical
/// components, which is 45° above the horizon — and it is written as [`FRAC_1_SQRT_2`] rather
/// than as its decimal, so the value *is* the quantity the sentence above names instead of a
/// sixteen-digit string that happens to equal it.
pub const SKY_STENCIL: [[f64; 3]; 5] = [
    [0.0, 1.0, 0.0],
    [FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0],
    [0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2],
    [-FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0],
    [0.0, FRAC_1_SQRT_2, -FRAC_1_SQRT_2],
];

/// One cell's own light, as fractions of what the sky above it delivers.
///
/// `beam` is the direct Sun, `sky` the dome. Both are `[0, 1]`, and both are
/// **measurements of this cell**: a cell that opens to the sky is `1.0` and a cell
/// under a lid is `0.0`, which is the difference the shading model has been missing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Visibility {
    pub beam: f64,
    pub sky: f64,
}

/// The state every ray of a cell's measurement walks through, reusable across cells so
/// a chunk's worth of cells does not allocate per ray.
pub struct Sampler {
    /// The ray's direction, unit length: `Crossings`' `t` is in voxel units (metres),
    /// and the path length through a material is the difference of two `t`s.
    toward_sun: [f64; 3],
    reach_m: f64,
}

impl Sampler {
    /// A sampler aimed at the Sun, with the reach a shadow ray is allowed to walk.
    ///
    /// The direction is normalised here, once, because the arithmetic that depends on
    /// it (path length, and the angles of the sky's stencil) is wrong for an
    /// unnormalised one — and *silently* wrong, which is the kind of wrong worth
    /// spending a normalisation on. A degenerate direction is refused by
    /// [`Sampler::new`], because a sampler that cannot be aimed cannot be used
    /// honestly.
    pub fn new(toward_sun: [f64; 3], reach_m: f64) -> Option<Self> {
        let length = (toward_sun[0] * toward_sun[0]
            + toward_sun[1] * toward_sun[1]
            + toward_sun[2] * toward_sun[2])
            .sqrt();
        if !length.is_finite() || length <= 0.0 || !reach_m.is_finite() || reach_m <= 0.0 {
            return None;
        }
        Some(Sampler {
            toward_sun: [
                toward_sun[0] / length,
                toward_sun[1] / length,
                toward_sun[2] / length,
            ],
            reach_m,
        })
    }

    /// The Sun's own reach, as a fraction of the light delivered to the top of the sky:
    /// `1.0` clear, `0.0` stopped.
    pub fn beam(&self, matter: &impl Matter, from_m: [f64; 3]) -> f64 {
        transmittance(matter, from_m, self.toward_sun, self.reach_m)
    }

    /// **The same beam, with the tree's own hierarchy doing the walking**
    /// ([`transmittance_tree`]).
    ///
    /// The sampler is aimed at whatever light this world's beam is: by day the Sun's own
    /// direction, and at night the Moon's — one call, because the direction is the sampler's and
    /// not the walk's.
    ///
    /// `from_tree_m` is in the **tree's** own coordinates; see [`transmittance_tree`].
    pub fn beam_tree(
        &self,
        matter: &impl Matter,
        tree: &SparseVoxelOctree,
        from_tree_m: [f64; 3],
    ) -> f64 {
        transmittance_tree(matter, tree, from_tree_m, self.toward_sun, self.reach_m)
    }

    /// The **sky's** own visibility at a cell: the mean transmittance of
    /// [`SKY_STENCIL`]'s five upward rays.
    ///
    /// This is what makes a cave mouth darker than a ridge while both are out of the
    /// Sun's direct line: the dome is the world's other light, and a cell that can see
    /// less of it receives less of it.
    pub fn sky(&self, matter: &impl Matter, from_m: [f64; 3]) -> f64 {
        let mut total = 0.0;
        for direction in SKY_STENCIL {
            total += transmittance(matter, from_m, direction, self.reach_m);
        }
        total / SKY_STENCIL.len() as f64
    }

    /// Both halves of one cell's light.
    pub fn visibility(&self, matter: &impl Matter, from_m: [f64; 3]) -> Visibility {
        Visibility { beam: self.beam(matter, from_m), sky: self.sky(matter, from_m) }
    }
}

/// The transmittance of one ray: what fraction of the light starting at `from_m` and
///
/// travelling along `direction` survives the matter up to `reach_m` away.
///
/// **The cell's own voxel is not between it and the Sun.** The walk's first crossing is
/// the voxel the ray *starts* in, and counting it would make every cell shadow itself:
/// the start point is inside the surface whose light is being measured, so a wall
/// would be dark and a floor would be dark and the whole world would be black. That
/// one `continue` is the difference between a shadow and a black screen.
///
/// The direction need not be normalised; `reach_m` and the path lengths are in the
/// units of `direction`. [`Sampler`] normalises once for a whole cell's worth of rays.
pub fn transmittance(
    matter: &impl Matter,
    from_m: [f64; 3],
    direction: [f64; 3],
    reach_m: f64,
) -> f64 {
    let Some(walk) = Crossings::new(&Ray::new(from_m, direction), reach_m) else {
        // A degenerate ray is **not** a clear one: an unmeasurable beam is not an
        // unobstructed one.
        return 0.0;
    };
    let mut through = 1.0;
    // **The segment is charged to the voxel the ray was in, not the one it entered.**
    // A crossing reports the voxel *entered* at that distance, so its own length is the
    // distance to the *next* crossing: charging it at the moment it is reported would
    // attribute one voxel's matter to the voxel before it, which is half a metre of rock
    // counted as the air above it. The first version of this walk did exactly that, and
    // `an_attenuating_material_dims_the_beam_rather_than_stopping_it` measured it as
    // `exp(−σ/2)` where Beer-Lambert gives `exp(−σ)`.
    let mut held: Option<([i32; 3], SubvoxelIndex)> = None;
    let mut held_since = 0.0;
    let mut started = false;
    for crossing in walk {
        if !started {
            // The first crossing is the voxel the ray starts in — see the note above.
            started = true;
            held_since = crossing.t;
            continue;
        }
        if let Some((voxel, at)) = held.take() {
            let path_m = crossing.t - held_since;
            let Some(reached) = charge(matter, voxel, at, path_m, through) else {
                return 0.0;
            };
            through = reached;
        }
        held = Some((
            crossing.voxel,
            subvoxel_of(
                [
                    from_m[0] + direction[0] * crossing.t,
                    from_m[1] + direction[1] * crossing.t,
                    from_m[2] + direction[2] * crossing.t,
                ],
                crossing.voxel,
                direction,
            ),
        ));
        held_since = crossing.t;
    }
    // **The last segment is charged too.** The walk ends at `reach_m`, and the matter the
    // ray is inside when it gets there is still matter: leaving it uncharged would let a
    // beam reach straight through the block it stopped in, so the final voxel is charged
    // over the distance it was held.
    if let Some((voxel, at)) = held {
        let path_m = reach_m - held_since;
        let Some(reached) = charge(matter, voxel, at, path_m, through) else {
            return 0.0;
        };
        through = reached;
    }
    through
}

/// **The same beam, marched by the tree's own hierarchy** — `transmittance`, at the cost of what
/// the ray meets instead of what it passes.
///
/// [`transmittance`] walks the grid a voxel at a time because the matter it asks about is a
/// per-address question ([`Matter::subvoxel`]): the only structure it has is the walk itself. Where
/// the matter **is** an octree — a resident chunk, at the world's own addresses — the tree already
/// knows which stretches of the path hold one material and which hold nothing, and
/// [`SparseVoxelOctree::spans`] yields exactly those. So an empty metre costs a slab test rather
/// than a crossing, and a solid block costs one charge rather than one per voxel: **the shadow ray
/// is walked by the same structure the world is meshed from.**
///
/// # It is the same law, and `the_tree_walk_and_the_voxel_walk_are_one_measurement` holds the two
/// together over a battery of rays through a generated chunk: the same transmittance to `1e-12`.
/// What is identical is not the bookkeeping but the arithmetic — Beer-Lambert over a segment,
/// `exp(-σ·d)`, which composes over a span exactly as it composes over the voxels in it.
///
/// # The two things that are this walk's own, said rather than implied
///
/// * **The cell's own voxel is still not charged**, the rule [`transmittance`] states: the ray's
///   first crossing is the voxel it starts in and counting it would make every surface shadow
///   itself. The first span can be a *block* of many voxels, so the exclusion is taken to the
///   first crossing after the start rather than to the first span, or a ray inside a solid block
///   would be missing a block's worth of path.
/// * **A medium is not asked**, because this walk's matter is the tree's and a medium is not in
///   it: a cloud is a field over the world ([`Matter::medium`]'s own note), so a caller that wants
///   it adds its own transmittance — which `voxel_client::cloud_field` already measures in closed
///   form. Charging the cloud inside this walk would be asking the hierarchy a question it cannot
///   answer per address.
///
/// # The frame, which is the tree's and not the world's
///
/// `from_tree_m` and `direction` are in the **tree's own coordinates** — the box its root covers,
/// which for a chunk is `[0, 32)` on each axis. A caller holding a world point and a chunk's tree
/// subtracts the chunk's own origin first. That subtraction is the caller's rather than this
/// function's because a `SparseVoxelOctree` is a **shape and not a place**: the same tree is a
/// chunk here and a cloud cell at a kilometre's scale there (`SparseVoxelOctree::with_levels`),
/// and it does not carry which of them it is. The names say so rather than leaving it to be
/// inferred from a wrong answer.
///
/// `reach_m` is the caller's, and **what lies beyond the tree is not this walk's**: a ray that
/// leaves the box is charged to the box's edge and no further, which is the honest answer for a
/// measurement of one tree. A caller marching across chunks clips the reach to each one.
///
/// **The cell the ray starts in is excluded, exactly as the per-voxel walk excludes it**: the
/// walk begins charging at the second crossing of the grid — the far side of the caller's own
/// voxel — so a beam out of a cell is not charged for the cell it came from. One crossing is
/// asked for, which is a step rather than a march.
///
/// **A degenerate ray is `0.0` and not `1.0`** — a NaN coordinate, a zero direction — which is
/// the same refusal [`transmittance`] makes: an unmeasurable beam is not an unobstructed one.
///
/// Everything is charged **per span and never per address**: a `Uniform` node is one material by
/// construction and a leaf is one voxel, so the span's own entry address is the whole of the
/// address question and one call to [`Matter::blocking`] settles `2^level` voxels of it. A
/// material that [`Blocking::Stops`] ends the march with `0.0` — the trait's own default, and the
/// same answer the per-voxel walk gives for the same address.
pub fn transmittance_tree(
    matter: &impl Matter,
    tree: &SparseVoxelOctree,
    from_tree_m: [f64; 3],
    direction: [f64; 3],
    reach_m: f64,
) -> f64 {
    march_tree(matter, tree, from_tree_m, direction, reach_m, true)
}

/// **The same march with the voxel the ray starts in charged too.**
///
/// [`transmittance_tree`] excludes it, which is right for a measurement of a *cell's* light — the
/// beam is not charged for the cell it came from. It is wrong for a march that **resumes at a chunk
/// boundary**: the first voxel of the next chunk is matter the beam is inside, and skipping it
/// would let a wall standing exactly on a boundary leak its light. A cross-chunk march
/// (`voxel_client::field`) is the caller, and the two rules are one law told where it began.
pub fn transmittance_tree_charging_the_start(
    matter: &impl Matter,
    tree: &SparseVoxelOctree,
    from_tree_m: [f64; 3],
    direction: [f64; 3],
    reach_m: f64,
) -> f64 {
    march_tree(matter, tree, from_tree_m, direction, reach_m, false)
}

/// The span march both rules above are: `skip_own_voxel` decides whether the stretch inside the
/// voxel the ray starts in is charged.
fn march_tree(
    matter: &impl Matter,
    tree: &SparseVoxelOctree,
    from_tree_m: [f64; 3],
    direction: [f64; 3],
    reach_m: f64,
    skip_own_voxel: bool,
) -> f64 {
    let Some(spans) = tree.spans(&Ray::new(from_tree_m, direction), reach_m) else {
        return 0.0;
    };
    let start_exit = if skip_own_voxel {
        // Where the ray's own voxel ends: the second crossing of the grid walk. One crossing to
        // find, so this costs a step and not a march.
        let mut walk = match Crossings::new(&Ray::new(from_tree_m, direction), reach_m) {
            Some(walk) => walk,
            None => return 0.0,
        };
        walk.next();
        walk.next().map(|crossing| crossing.t).unwrap_or(reach_m)
    } else {
        0.0
    };
    let mut through = 1.0;
    for span in spans {
        let entry = span.entry_t.max(start_exit);
        let path_m = span.exit_t - entry;
        if path_m <= 0.0 {
            continue;
        }
        // **A `Mixed` leaf arrives already split at its own grid.** `Spans` answers a non-uniform
        // leaf cell by cell (`SUBVOXEL_SIDE` per voxel), so `material_at_entry()` here is the
        // material of the stretch this span covers and never one of sixty-four cells speaking for
        // the metre. The question is the walk's, and this is its answer being spent — not a second
        // grid walked inside this loop.
        let Some(material) = span.material_at_entry() else {
            continue;
        };
        match matter.blocking(material) {
            Blocking::Stops => return 0.0,
            Blocking::Attenuates(sigma) => {
                let Some(reached) = attenuate(sigma, path_m, through) else {
                    return 0.0;
                };
                through = reached;
            }
        }
    }
    through
}

/// One segment's worth of matter: `Some(transmittance)` through it, or `None` when it
/// stops the beam.
///
/// **The medium is asked first**, and the order is the class distinction rather than a
/// preference: an address that holds a participating medium has no interface, so there is
/// no material there to stop anything and asking the solid question first would let a
/// provider's empty `subvoxel` answer stand for a cell that is full of cloud.
fn charge(
    matter: &impl Matter,
    voxel: [i32; 3],
    at: SubvoxelIndex,
    path_m: f64,
    through: f64,
) -> Option<f64> {
    if path_m <= 0.0 {
        return Some(through);
    }
    if let Some(sigma) = matter.medium(voxel, at) {
        return attenuate(sigma, path_m, through);
    }
    match matter.subvoxel(voxel, at) {
        None => Some(through),
        Some(material) => match matter.blocking(material) {
            Blocking::Stops => None,
            Blocking::Attenuates(sigma) => attenuate(sigma, path_m, through),
        },
    }
}

/// Beer-Lambert over one segment: `through · exp(−σ·d)`, or `None` for a rate that is not
/// a rate and for a path that ends dark.
///
/// **Shared by the material's rate and the medium's**, because they are the same law: the
/// spec's `exp(−σ·d)` does not change because the thing it is applied to has a surface or
/// does not. What differs is only where the rate came from — a material id or an address —
/// and that is [`Matter`]'s question, not this one's.
fn attenuate(sigma: f64, path_m: f64, through: f64) -> Option<f64> {
    if !sigma.is_finite() || sigma < 0.0 {
        // A rate that is not a rate is a refusal, not a free pass. `0.0` is legal: it is a
        // medium that is not there, and the caller may say so instead of answering `None`.
        return None;
    }
    let reached = through * (-sigma * path_m).exp();
    if reached <= 0.0 {
        None
    } else {
        Some(reached)
    }
}

/// The subvoxel a point on a voxel's boundary belongs to — the **entered** one.
///
/// A crossing lands exactly on the boundary, where a `floor` is a coin toss between
/// the voxel the ray is leaving and the one it is entering. So the point is nudged a
/// half-subvoxel along the ray first, which puts it unambiguously inside the entered
/// voxel — and the nudge is *along the ray* rather than per axis, so a ray sliding
/// along a grid plane (a zero component in the direction) is not pushed sideways into
/// a subvoxel it does not pass through.
fn subvoxel_of(point: [f64; 3], voxel: [i32; 3], direction: [f64; 3]) -> SubvoxelIndex {
    let half_cell = 0.5 / SUBVOXEL_SIDE as f64;
    let mut at = [0u8; 3];
    for axis in 0..3 {
        let nudged = point[axis] + direction[axis] * half_cell;
        let fraction = (nudged - voxel[axis] as f64) * SUBVOXEL_SIDE as f64;
        at[axis] = fraction.floor().clamp(0.0, (SUBVOXEL_SIDE - 1) as f64) as u8;
    }
    SubvoxelIndex { x: at[0], y: at[1], z: at[2] }
}

/// [`Matter`] for the **generated world**, one voxel at a time.
///
/// The honest slow path: every address runs the generator, because there is no
/// resident chunk to read. A shadow ray asks up to [`Sampler::reach_m`] addresses, so
/// this is for a first chunk, a test, or a tool — the frame path's provider is the
/// resident octrees, whose subvoxel addressing this walks identically.
///
/// Every subvoxel of a generated voxel answers the same material, and that is not a
/// shortcut: the generator writes one material per voxel, and the subvoxel grid exists
/// for what is *authored* (dug, placed, partially built), which is exactly what the
/// resident octrees carry.
impl Matter for crate::WorldSource {
    fn subvoxel(&self, world: [i32; 3], _at: SubvoxelIndex) -> Option<MaterialId> {
        match self.material_at(world[0], world[1], world[2]) {
            Ok(id) if !id.is_empty() => Some(id),
            _ => None,
        }
    }

}

// **No `blocking` override, and that is the measurement.** The generated world's
// materials carry no absorption coefficient (`APPEARANCE.md` §5), so the trait's own
// default applies: matter stops the beam. The generated world is also **not a medium**: it
// is made of materials with interfaces, and the one participating medium this tree draws —
// a cloud — is a field over the world rather than a row in it, so [`CloudMedium`] answers
// [`Matter::medium`] above and this impl answers `None` to that question, which is the
// truth about rock.
//
// [`CloudMedium`]: ../../voxel_client/cloud_field/struct.CloudMedium.html

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A provider over hand-placed voxels: `(world, material)` pairs, with everything
    /// else air. Built from a `BTreeMap` for the reason `VoxelGrid` is: iteration is
    /// deterministic, so a test can be too.
    struct Placed {
        cells: BTreeMap<[i32; 3], MaterialId>,
        /// A rate to report for every material placed, or `None` to stop the beam.
        attenuation: Option<f64>,
    }

    impl Placed {
        fn new(cells: &[([i32; 3], u16)]) -> Self {
            Placed {
                cells: cells.iter().map(|(world, material)| (*world, MaterialId(*material))).collect(),
                attenuation: None,
            }
        }

        /// Only the **subvoxels** named are solid, in one voxel — the addressing case.
        fn partial(voxel: [i32; 3], solid: &[usize], material: MaterialId) -> SubvoxelFn {
            let mut set = Vec::new();
            for index in solid {
                set.push(*index as u8);
            }
            SubvoxelFn { voxel, set, material }
        }
    }

    /// A provider solid at exactly the subvoxels listed, in one voxel.
    struct SubvoxelFn {
        voxel: [i32; 3],
        /// Subvoxel ordinals in `0..64`, `x + 4y + 16z`.
        set: Vec<u8>,
        material: MaterialId,
    }

    impl Matter for Placed {
        fn subvoxel(&self, world: [i32; 3], _at: SubvoxelIndex) -> Option<MaterialId> {
            self.cells.get(&world).copied()
        }

        fn blocking(&self, _material: MaterialId) -> Blocking {
            match self.attenuation {
                Some(sigma) => Blocking::Attenuates(sigma),
                None => Blocking::Stops,
            }
        }
    }

    impl Matter for SubvoxelFn {
        fn subvoxel(&self, world: [i32; 3], at: SubvoxelIndex) -> Option<MaterialId> {
            if world != self.voxel {
                return None;
            }
            let ordinal = at.x as u16 + 4 * at.y as u16 + 16 * at.z as u16;
            if self.set.contains(&(ordinal as u8)) {
                Some(self.material)
            } else {
                None
            }
        }
    }

    /// Straight up, so the arithmetic in these tests is checkable by eye.
    fn sun() -> Sampler {
        Sampler::new([0.0, 1.0, 0.0], 64.0).expect("an overhead Sun")
    }

    /// A provider that is **a medium and nothing else** (C-178): solid at no subvoxel, and
    /// with one declared rate everywhere at or above `from_y`.
    struct Medium {
        sigma: f64,
        from_y: i32,
    }

    impl Matter for Medium {
        fn subvoxel(&self, _world: [i32; 3], _at: SubvoxelIndex) -> Option<MaterialId> {
            None
        }

        fn medium(&self, world: [i32; 3], _at: SubvoxelIndex) -> Option<f64> {
            if world[1] >= self.from_y {
                Some(self.sigma)
            } else {
                None
            }
        }
    }

    /// A provider that answers **both questions** at every address: a material that stops the
    /// beam *and* a medium with a rate. Built to pin which one the walk asks first.
    struct Both {
        sigma: f64,
    }

    impl Matter for Both {
        fn subvoxel(&self, _world: [i32; 3], _at: SubvoxelIndex) -> Option<MaterialId> {
            Some(MaterialId(7))
        }

        fn blocking(&self, _material: MaterialId) -> Blocking {
            Blocking::Stops
        }

        fn medium(&self, _world: [i32; 3], _at: SubvoxelIndex) -> Option<f64> {
            Some(self.sigma)
        }
    }

    #[test]
    fn a_clear_column_passes_the_whole_beam() {
        let matter = Placed::new(&[]);
        assert_eq!(sun().beam(&matter, [0.5, 0.5, 0.5]), 1.0);
        assert_eq!(sun().sky(&matter, [0.5, 0.5, 0.5]), 1.0);
    }

    #[test]
    fn a_lid_stops_the_beam_and_the_sky() {
        // **Two claims, and the second is about the dome's own shape.** One solid metre
        // of rock three metres up stops the beam — one voxel is enough, because the beam
        // is a ray. The *dome* is five rays at 45°, so the same single voxel blocks only
        // the vertical one: by the time a tilted ray climbs 2.5 m it is 2.5 m to the
        // side. Blocking all five needs a slab, so a slab is what this asserts, and the
        // seven-by-seven one is sized from the stencil's own geometry rather than guessed.
        let lid = Placed::new(&[([0, 3, 0], 7)]);
        assert_eq!(
            sun().beam(&lid, [0.5, 0.5, 0.5]),
            0.0,
            "a solid metre of rock did not stop the Sun"
        );
        assert!(
            sun().sky(&lid, [0.5, 0.5, 0.5]) < 1.0,
            "a solid metre of rock did not shade any of the dome"
        );

        let mut cells = Vec::new();
        for x in -3..=3 {
            for z in -3..=3 {
                cells.push(([x, 3, z], 7));
            }
        }
        let slab = Placed::new(&cells);
        assert_eq!(
            sun().sky(&slab, [0.5, 0.5, 0.5]),
            0.0,
            "a roof over every one of the dome's five rays left some of it visible"
        );
    }

    #[test]
    fn a_cell_is_not_shadowed_by_itself() {
        // **The classic defect, and the one that turns a world black.** The origin
        // voxel is solid — it is the surface whose light is being measured — and a
        // sampler that counted it would give every cell in the world a beam of zero
        // and every wall a sky of zero. Nothing else is placed, so the answer is the
        // open sky's.
        let matter = Placed::new(&[([0, 0, 0], 7)]);
        assert_eq!(sun().beam(&matter, [0.5, 0.5, 0.5]), 1.0);
        assert_eq!(sun().sky(&matter, [0.5, 0.5, 0.5]), 1.0);
    }

    #[test]
    fn a_partial_voxel_shadows_only_where_it_is_solid() {
        // A voxel at `y = 2` with **one subvoxel column** solid: `x = 0, z = 0`, all
        // four `y` cells (ordinals 0, 4, 8, 12). A ray straight up through `(0.125,
        // ., 0.125)` passes through that column and is stopped; a ray up through
        // `(0.875, ., 0.875)` misses every solid subvoxel and is not.
        //
        // This is the material-addressing payoff: the occluder is a *quarter* of a
        // voxel, and the shadow it casts is that quarter's, because the sampler asks
        // the world at subvoxel addresses rather than per block.
        let partial = Placed::partial([0, 2, 0], &[0, 4, 8, 12], MaterialId(7));
        assert_eq!(
            transmittance(&partial, [0.125, 0.5, 0.125], [0.0, 1.0, 0.0], 64.0),
            0.0,
            "a ray through the solid subvoxel column passed"
        );
        assert_eq!(
            transmittance(&partial, [0.875, 0.5, 0.875], [0.0, 1.0, 0.0], 64.0),
            1.0,
            "a ray through the emptied subvoxels was shadowed by the quarter beside it"
        );
    }

    /// **The tree's own walk, through a voxel that is only partly solid.**
    ///
    /// The per-voxel walk above has always asked the subvoxel address; the *tree* walk did not —
    /// `Spans` hands it one span per leaf and one subvoxel per span, so a beam that entered a
    /// half-dug voxel through an empty corner was charged nothing for the rock it crossed. That is
    /// the light leak this test is for: the solid cell sits at the **top** of the voxel, so the ray
    /// enters through an empty one and only meets the rock three quarters of a metre later.
    #[test]
    fn a_mixed_voxels_shadow_follows_its_own_subvoxel_grid() {
        use voxel_mesh::octree::local_from;
        let mut tree = SparseVoxelOctree::new();
        let voxel = local_from([4, 4, 4]).expect("a voxel");
        // The top corner of the voxel: `y` in `[4.75, 5)`, `x` and `z` in `[4, 4.25)`.
        let top = SubvoxelIndex::new(0, 3, 0).expect("a subvoxel");
        tree.set_subvoxel(voxel, top, MaterialId(7));
        let matter = ChunkMatter::new(&tree, [0, 0, 0]);

        // Straight up through the solid cell, entering the voxel through an *empty* one.
        let stopped = transmittance_tree(&matter, &tree, [4.125, 3.0, 4.125], [0.0, 1.0, 0.0], 8.0);
        assert_eq!(
            stopped, 0.0,
            "a beam through the emptied corner passed the rock above it: {stopped}"
        );
        // And the dug quarter on the other side of the same voxel is still clear.
        let clear = transmittance_tree(&matter, &tree, [4.875, 3.0, 4.875], [0.0, 1.0, 0.0], 8.0);
        assert_eq!(clear, 1.0, "the emptied quarter stopped a beam: {clear}");
    }

    #[test]
    fn an_attenuating_material_dims_the_beam_rather_than_stopping_it() {
        // Four one-metre voxels of a medium, straight up: `exp(−σ·4)`.
        let mut matter = Placed::new(&[
            ([0, 1, 0], 7),
            ([0, 2, 0], 7),
            ([0, 3, 0], 7),
            ([0, 4, 0], 7),
        ]);
        matter.attenuation = Some(0.25);
        let one = {
            let mut single = Placed::new(&[([0, 1, 0], 7)]);
            single.attenuation = Some(0.25);
            transmittance(&single, [0.5, 0.5, 0.5], [0.0, 1.0, 0.0], 64.0)
        };
        let four = transmittance(&matter, [0.5, 0.5, 0.5], [0.0, 1.0, 0.0], 64.0);
        assert!(
            (one - (-0.25f64).exp()).abs() < 1e-12,
            "one metre of the medium gave {one} where Beer-Lambert gives {}",
            (-0.25f64).exp()
        );
        assert!(
            (four - (-1.0f64).exp()).abs() < 1e-12,
            "four metres gave {four} where Beer-Lambert gives {}",
            (-1.0f64).exp()
        );
        assert!(four < one && four > 0.0, "four metres was not dimmer but still lit");
    }

    #[test]
    fn a_medium_dims_the_beam_along_the_path_rather_than_stopping_it() {
        // **The cloud's own case, at a rate checkable by eye.** Four one-metre voxels of a
        // medium at σ = 0.25/m, straight up, give `exp(−σ·4) = exp(−1)` — the same number
        // [`an_attenuating_material_dims_the_beam_rather_than_stopping_it`] measures for a
        // material at the same rate, because Beer-Lambert is one law. What differs is that
        // this rate came from **where the beam is** and not from *what it met*: a cloud has
        // no id to hang a rate on, and its density falls to zero at the layer's base and top,
        // which a per-material rate could not express.
        let uniform = Medium { sigma: 0.25, from_y: 1 };
        let got = transmittance(&uniform, [0.5, 0.5, 0.5], [0.0, 1.0, 0.0], 4.5);
        assert!(
            (got - (-1.0f64).exp()).abs() < 1e-12,
            "4 m of the medium gave {got} where Beer-Lambert gives {}",
            (-1.0f64).exp()
        );
        assert!(
            got > 0.0 && got < 1.0,
            "a medium stopped the beam ({got}) instead of dimming it"
        );
    }

    #[test]
    fn a_medium_is_asked_before_the_material_beside_it() {
        // **The order, pinned, because the alternative is the exact bug the class exists to
        // avoid.** A cloud cell answers `None` to `subvoxel` (it has no surface) and a rate to
        // `medium`; a walk that asked the solid question first would take that `None` for an
        // empty cell and let the whole cloud through. This provider answers *both*, and the
        // medium must win: three metres at σ = 0.5 reads `exp(−1.5)`, not the material's stop.
        let both = Both { sigma: 0.5 };
        let got = transmittance(&both, [0.5, 0.5, 0.5], [0.0, 1.0, 0.0], 3.5);
        assert!(
            (got - (-1.5f64).exp()).abs() < 1e-12,
            "the solid answer won over the medium: got {got}, Beer-Lambert gives {}",
            (-1.5f64).exp()
        );
    }

    #[test]
    fn a_medium_that_is_not_there_is_clear_and_a_rate_that_is_not_a_rate_is_a_refusal() {
        // `0.0` is a legal rate — a medium that is not there — and must pass the whole beam,
        // so a provider can keep its shape instead of dropping to `None`. A rate that is not a
        // rate (`NaN`, negative) is the refusal [`Blocking::Attenuates`] already makes, and it
        // is a **refusal and not a free pass** for the module's own reason: an unmeasurable
        // beam is not an unobstructed one.
        let clear = Medium { sigma: 0.0, from_y: 1 };
        assert_eq!(
            transmittance(&clear, [0.5, 0.5, 0.5], [0.0, 1.0, 0.0], 8.0),
            1.0,
            "a medium with a rate of zero dimmed the beam"
        );
        for bad in [f64::NAN, f64::INFINITY, -0.5] {
            let broken = Medium { sigma: bad, from_y: 1 };
            assert_eq!(
                transmittance(&broken, [0.5, 0.5, 0.5], [0.0, 1.0, 0.0], 8.0),
                0.0,
                "a medium with a rate of {bad} passed the beam"
            );
        }
    }

    #[test]
    fn a_degenerate_ray_is_a_refusal_and_not_a_clear_view() {
        // A NaN start point, and a zero direction. Both are answered `0.0` — the beam
        // is *unmeasurable*, which is not the same claim as *unobstructed*, and the
        // difference is a bright cell in a stone wall.
        let matter = Placed::new(&[]);
        assert_eq!(
            transmittance(&matter, [f64::NAN, 0.5, 0.5], [0.0, 1.0, 0.0], 64.0),
            0.0
        );
        assert_eq!(transmittance(&matter, [0.5, 0.5, 0.5], [0.0, 0.0, 0.0], 64.0), 0.0);
        assert!(Sampler::new([0.0, 0.0, 0.0], 64.0).is_none());
        assert!(Sampler::new([0.0, 1.0, 0.0], 0.0).is_none());
    }

    #[test]
    fn the_sun_s_angle_is_what_it_is_asked_about() {
        // A wall one metre to the east of the cell, and a Sun over the horizon: the
        // beam is stopped by it when the Sun is on the far side, and clear when the
        // Sun is on the cell's side. Same cell, same wall, only the angle differs —
        // which is the whole of "the Sun's own direction decides what is in the way".
        let matter = Placed::new(&[([1, 0, 0], 7)]);
        let from = [0.5, 0.5, 0.5];
        let low_east = Sampler::new([1.0, 0.2, 0.0], 64.0).expect("a Sun near the horizon");
        let low_west = Sampler::new([-1.0, 0.2, 0.0], 64.0).expect("the other side");
        assert_eq!(low_east.beam(&matter, from), 0.0, "the wall is between the cell and a rising Sun");
        assert_eq!(low_west.beam(&matter, from), 1.0, "a setting Sun sees past the same wall");
    }

    #[test]
    fn the_same_cell_measured_twice_is_the_same_answer() {
        // Determinism is a contract here (F1): no clock, no RNG, no resident state.
        let matter = Placed::new(&[([0, 3, 0], 7), ([0, 5, 0], 7)]);
        let first = sun().visibility(&matter, [0.25, 0.5, 0.75]);
        let second = sun().visibility(&matter, [0.25, 0.5, 0.75]);
        assert_eq!(first, second);
    }

    #[test]
    fn a_reach_that_is_too_short_is_not_a_clear_sky() {
        // The reach bounds the walk, and a ray that leaves it without meeting matter
        // is clear *within the reach*. A lid beyond the reach is therefore not seen —
        // which is why the reach has to be the world's own scale and not a guess: the
        // test states what the bound does rather than pretending it is unbounded.
        let matter = Placed::new(&[([0, 40, 0], 7)]);
        let short = Sampler::new([0.0, 1.0, 0.0], 8.0).expect("a short reach");
        let long = Sampler::new([0.0, 1.0, 0.0], 64.0).expect("a reach that arrives");
        assert_eq!(short.beam(&matter, [0.5, 0.5, 0.5]), 1.0, "the lid is past the reach");
        assert_eq!(long.beam(&matter, [0.5, 0.5, 0.5]), 0.0, "the reach that arrives sees it");
    }

    #[test]
    fn the_generated_world_shadows_itself_where_it_is_solid() {
        // **The end-to-end claim, on the real generator and the real octree.** One
        // chunk of the shipped world, built the way the mesher builds it, sampled at
        // the addresses a shadow ray walks: a cell inside the rock is dark, and a cell
        // above the surface is the open sky. Those two are the whole point of the
        // module, and the chunk's own octree answers the addresses — so this is the
        // sampler reading an octree and not a hand-written grid.
        use crate::world_source::WorldSource;
        use voxel_mesh::octree::{local_from, SparseVoxelOctree};
        use voxel_model::{ChunkPos, CHUNK_SIDE};

        // The client's own spawn column, **restated because this crate is below the
        // client**: `voxel_client::game::GAME_SPAWN_XZ` is `(0.5, -95.5)`, and a test
        // in `voxel-client` is what holds the two together.
        const SPAWN_X: i32 = 0;
        const SPAWN_Z: i32 = -95;

        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        let source = WorldSource::new(spec);

        // **Which chunk to build is a question about the world**, so it is asked rather
        // than picked: find the ground first, then build the chunk it is in. A test
        // that hard-coded the chunk would be about the picking.
        let mut surface_y = None;
        for y in -32..128 {
            let solid = source
                .material_at(SPAWN_X, y, SPAWN_Z)
                .map(|material| !material.is_empty())
                .unwrap_or(false);
            if solid {
                surface_y = Some(y);
            }
        }
        let surface_y = surface_y.expect("the spawn column has ground under it");
        let chunk = ChunkPos::new(
            SPAWN_X.div_euclid(CHUNK_SIDE),
            surface_y.div_euclid(CHUNK_SIDE),
            SPAWN_Z.div_euclid(CHUNK_SIDE),
        );
        let tree: SparseVoxelOctree = source.build_chunk(chunk).expect("a chunk of the world");
        let origin = chunk.origin();

        // The chunk's octree, at the world's own addresses — the provider a resident
        // set hands in, and the same addressing `Matter` takes. An address outside this
        // chunk is air here, which is this test's declared limit: one chunk answers.
        let matter = |world: [i32; 3], at: SubvoxelIndex| -> Option<MaterialId> {
            let local =
                local_from([world[0] - origin.x, world[1] - origin.y, world[2] - origin.z]).ok()?;
            let material = tree.material_at(local, at);
            if material.is_empty() {
                None
            } else {
                Some(material)
            }
        };

        // **A Sun 60° above the horizon, from the east.** The sampler's own input: the
        // world's noon direction is `voxel_client::skylight::sun_direction`'s, and that
        // function lives *above* this crate — a copy of the convention here would be a
        // second Sun. An oblique beam is what this test needs, so the walk crosses
        // voxels in two axes rather than sliding straight up one.
        let sampler =
            Sampler::new([0.5, 0.866_025_403_784_438_6, 0.0], 256.0).expect("an oblique Sun");

        // **The column measured is the chunk's own centre, and the surface is the
        // spawn's.** The one-chunk limit above is what decides that: the dome's tilted
        // rays leave a chunk quickly from a column one metre inside its edge (the spawn
        // column is), and an address outside the chunk answers air — so a column at the
        // centre is where the answer is about the world rather than about the boundary.
        // One voxel is one metre, so the cell's centre is `world + 0.5`.
        let column = [CHUNK_SIDE / 2, CHUNK_SIDE / 2];
        let centre = |local_y: i32| {
            [
                (origin.x + column[0]) as f64 + 0.5,
                (origin.y + local_y) as f64 + 0.5,
                (origin.z + column[1]) as f64 + 0.5,
            ]
        };
        let centre_surface = {
            let mut top = None;
            for y in -32..128 {
                let solid = source
                    .material_at(origin.x + column[0], y, origin.z + column[1])
                    .map(|material| !material.is_empty())
                    .unwrap_or(false);
                if solid {
                    top = Some(y);
                }
            }
            top.expect("the centre column has ground under it")
        };
        let buried = sampler.visibility(&matter, centre(centre_surface - origin.y - 3));
        let open = sampler.visibility(&matter, centre(centre_surface - origin.y + 4));

        println!(
            "chunk ({}, {}, {}): the spawn column ({SPAWN_X}, {SPAWN_Z}) stands on y = {surface_y};\n  \
             the chunk's centre column ({} , {}) stands on y = {centre_surface}\n  \
             three metres into the rock: beam {:.4} sky {:.4}\n  \
             four metres above it:      beam {:.4} sky {:.4}",
            chunk.x,
            chunk.y,
            chunk.z,
            origin.x + column[0],
            origin.z + column[1],
            buried.beam,
            buried.sky,
            open.beam,
            open.sky
        );
        assert_eq!(buried.beam, 0.0, "a cell inside the rock saw the Sun");
        assert_eq!(buried.sky, 0.0, "a cell inside the rock saw the sky");
        assert!(open.beam > 0.0, "a cell in the open was shadowed");
        assert!(open.sky > 0.0, "a cell in the open saw none of the dome");
    }


    /// Find the chunk the spawn column stands on, the way the module's own end-to-end test does:
    /// **which chunk to build is a question about the world**, so it is asked rather than picked.
    fn a_generated_chunk() -> (SparseVoxelOctree, voxel_model::ChunkPos) {
        use crate::world_source::WorldSource;
        use voxel_model::CHUNK_SIDE;

        const SPAWN_X: i32 = 0;
        const SPAWN_Z: i32 = -95;
        let spec = voxel_shapers::WorldSpec::new(2468, voxel_shapers::Config::default());
        let source = WorldSource::new(spec);
        let mut surface_y = None;
        for y in -32..128 {
            let solid = source
                .material_at(SPAWN_X, y, SPAWN_Z)
                .map(|material| !material.is_empty())
                .unwrap_or(false);
            if solid {
                surface_y = Some(y);
            }
        }
        let surface_y = surface_y.expect("the spawn column has ground under it");
        let chunk = voxel_model::ChunkPos::new(
            SPAWN_X.div_euclid(CHUNK_SIDE),
            surface_y.div_euclid(CHUNK_SIDE),
            SPAWN_Z.div_euclid(CHUNK_SIDE),
        );
        let tree = source.build_chunk(chunk).expect("a chunk of the world");
        (tree, chunk)
    }

    /// The chunk's own tree answering `Matter`, so the tree and the provider are **one world**:
    /// the equivalence below is between two walks and not between two worlds. The type is
    /// [`ChunkMatter`]'s, so the client's field marches through the same provider this holds to
    /// the grid walk.
    fn tree_matter<'a>(tree: &'a SparseVoxelOctree, origin: [i32; 3], sigma: Option<f64>) -> ChunkMatter<'a> {
        match sigma {
            Some(sigma) => ChunkMatter::attenuating(tree, origin, sigma),
            None => ChunkMatter::new(tree, origin),
        }
    }

    /// Deterministic origins inside one chunk, from a linear congruential generator: a battery
    /// over the whole chunk rather than the cells its author thought of, from `seed` so the case
    /// is reproducible.
    fn a_battery(origin: [i32; 3], count: usize, seed: u64) -> Vec<[f64; 3]> {
        let mut state = seed | 1;
        let mut next = move || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((state >> 33) as f64 / (1u64 << 31) as f64) * 30.0
        };
        (0..count)
            .map(|_| {
                [
                    origin[0] as f64 + next() + 0.5,
                    origin[1] as f64 + next() + 0.5,
                    origin[2] as f64 + next() + 0.5,
                ]
            })
            .collect()
    }

    /// **One measurement or two light models.** `transmittance_tree` marches the octree's own
    /// hierarchy where `transmittance` steps the grid, and the comparison is made on a **generated
    /// chunk** — the real world's own nodes at the world's own addresses — over an oblique Sun, so
    /// the walk crosses voxels in two axes instead of sliding up one.
    ///
    /// **Both of `Blocking`'s arms, because they answer differently on purpose**: stopped matter is
    /// `0.0`, while a rate is `0.0` only after an enormous optical depth. And **the battery is a
    /// battery**: it must meet rock, open sky and — with a rate — the arithmetic in between, since
    /// a fixture where every ray returned the same answer would compare two walks that never walked.
    #[test]
    fn the_tree_walk_and_the_voxel_walk_are_one_measurement() {
        let (tree, chunk) = a_generated_chunk();
        let origin = chunk.origin();
        let origin = [origin.x, origin.y, origin.z];
        let direction = [0.5, 0.866_025_403_784_438_6, 0.0];
        let reach = 20.0;

        for sigma in [None, Some(0.4)] {
            let matter = tree_matter(&tree, origin, sigma);
            let mut stopped = 0usize;
            let mut partial = 0usize;
            let mut clear = 0usize;
            for from in a_battery(origin, 200, 0x1f2e_3d4c) {
                let in_tree = [
                    from[0] - origin[0] as f64,
                    from[1] - origin[1] as f64,
                    from[2] - origin[2] as f64,
                ];
                let stepped = transmittance(&matter, from, direction, reach);
                let descended = transmittance_tree(&matter, &tree, in_tree, direction, reach);
                assert!(
                    (stepped - descended).abs() < 1e-12,
                    "the two walks measure different light at {from:?}: grid {stepped}, tree \
                     {descended} (sigma {sigma:?})"
                );
                if stepped == 0.0 {
                    stopped += 1;
                } else if stepped == 1.0 {
                    clear += 1;
                } else {
                    partial += 1;
                }
            }
            assert!(stopped + clear > 20, "the battery measured one answer: {stopped}/{clear}");
            match sigma {
                None => assert!(
                    stopped > 20 && clear > 20,
                    "the fixture met only one kind of light: {stopped} shadowed, {clear} clear"
                ),
                Some(_) => assert!(
                    partial > 20,
                    "the attenuating arm was never reached: {partial} of 200"
                ),
            }
            println!(
                "chunk ({}, {}, {}), sigma {sigma:?}: {stopped} shadowed, {partial} dimmed, \
                 {clear} clear",
                chunk.x, chunk.y, chunk.z
            );
        }
    }

    /// **The measurement the hierarchy exists for**: a beam straight up out of the spawn column,
    /// where the sky is not matter — so the grid walk steps through every voxel of it while the
    /// tree walk has nothing at all to report, and the beam is clear either way.
    #[test]
    fn the_tree_walk_costs_the_nodes_it_meets_not_the_voxels_it_passes() {
        let (tree, chunk) = a_generated_chunk();
        let origin = chunk.origin();
        let from = [
            origin.x as f64 + 16.5,
            origin.y as f64 + 30.5,
            origin.z as f64 + 16.5,
        ];
        let up = [0.0, 1.0, 0.0];
        let reach = 40.0;
        let sky = ChunkMatter::new(&tree, [origin.x, origin.y, origin.z]);
        let in_tree = [
            from[0] - origin.x as f64,
            from[1] - origin.y as f64,
            from[2] - origin.z as f64,
        ];
        let voxels = Crossings::new(&Ray::new(in_tree, up), reach)
            .expect("a ray")
            .count();
        let spans = tree.spans(&Ray::new(in_tree, up), reach).expect("a ray").count();
        assert_eq!(spans, 0, "the space above the surface reported matter");
        assert!(voxels >= 30, "the grid walk crossed hardly anything: {voxels}");
        assert_eq!(transmittance(&sky, from, up, reach), 1.0);
        assert_eq!(transmittance_tree(&sky, &tree, in_tree, up, reach), 1.0);
        println!(
            "a beam out of the spawn chunk's sky: {voxels} voxels stepped, {spans} spans descended"
        );
    }

    /// **A roof eight metres up, filling the chunk's upward half.** A cell under it is dark for
    /// both walks, and the same cell looking sideways along the roof's own underside is clear for
    /// both — because neither walk is asked about the octree's shape, only about the light. The
    /// region is `[0, 16, 0]` at level 4 (`16 = 2^4`), so it is one node: the walk's own block case.
    #[test]
    fn the_tree_walk_finds_the_wall_the_grid_walk_finds() {
        let mut tree = SparseVoxelOctree::new();
        tree.set_material_region([0, 16, 0], 4, MaterialId(7));
        let matter = ChunkMatter::new(&tree, [0, 0, 0]);
        let under = [4.5, 15.5, 4.5];
        let overhead = Sampler::new([0.0, 1.0, 0.0], 64.0).expect("an overhead Sun");
        assert_eq!(
            overhead.beam(&matter, under),
            0.0,
            "the grid walk saw the Sun through a roof"
        );
        assert_eq!(
            overhead.beam_tree(&matter, &tree, under),
            0.0,
            "the tree walk saw the Sun through the same roof"
        );
        let sideways = Sampler::new([1.0, 0.0, 0.0], 64.0).expect("a Sun along the roof");
        assert_eq!(sideways.beam(&matter, under), 1.0);
        assert_eq!(sideways.beam_tree(&matter, &tree, under), 1.0);
        assert_eq!(overhead.beam_tree(&matter, &tree, [16.5, 15.5, 4.5]), 1.0);
    }

    /// **A cell walled in by rock reads nothing, in every direction.**
    ///
    /// The property the tree walk exists to have, and the one a picture cannot check: a cell whose
    /// twenty-six neighbours are solid is *inside* the matter, so every ray out of it has met rock
    /// before it has gone a metre. `voxel_client`'s `measured-light` example found cells that broke
    /// it in a generated world — the diffuse bands came back at `3e-4` for a cell surrounded by
    /// full voxels — which is either a hole in the world (a neighbour that is only partly filled,
    /// which is a legal voxel of 64 subvoxels) or a walk that is not charging what it should.
    ///
    /// This is the half of that question the tree can answer on its own: **the world is entirely
    /// full here**, twenty-six whole voxels of stone around an empty one, so a nonzero answer is
    /// the walk's and not the world's.
    #[test]
    fn a_cell_walled_in_by_rock_reads_no_light_from_any_direction() {
        const STONE: u16 = 6;
        let mut tree = SparseVoxelOctree::new();
        let cell = [16i32; 3];
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if (dx, dy, dz) == (0, 0, 0) {
                        continue;
                    }
                    let local = voxel_model::LocalPos::new(
                        (cell[0] + dx) as u32,
                        (cell[1] + dy) as u32,
                        (cell[2] + dz) as u32,
                    )
                    .expect("a local offset inside the chunk");
                    tree.set_voxel(local, voxel_model::VoxelShape::solid(MaterialId(STONE)));
                }
            }
        }
        let matter = ChunkMatter::new(&tree, [0, 0, 0]);
        let from = [16.5, 16.5, 16.5];
        // The dome's own five rays, the vertical, and a fan of obliques — the direction a leak
        // would hide in is the one nobody thought to aim.
        let mut directions: Vec<[f64; 3]> = vec![[0.0, 1.0, 0.0]];
        for direction in SKY_STENCIL {
            directions.push(direction);
        }
        for step in 0..24 {
            let angle = step as f64 * core::f64::consts::TAU / 24.0;
            directions.push([angle.cos() * 0.3, 1.0, angle.sin() * 0.3]);
            directions.push([angle.cos(), 0.4, angle.sin()]);
            directions.push([0.0, -1.0, 0.0]);
        }
        for direction in directions {
            let through = transmittance_tree(&matter, &tree, from, direction, 160.0);
            assert_eq!(
                through, 0.0,
                "a cell walled in by rock let {through} of the beam through along {direction:?}"
            );
            let charging = transmittance_tree_charging_the_start(
                &matter, &tree, from, direction, 160.0,
            );
            assert_eq!(
                charging, 0.0,
                "the cross-chunk rule let {charging} through along {direction:?}"
            );
        }
    }

    #[test]
    fn a_degenerate_ray_is_no_light_for_the_tree_walk_too() {
        let tree = SparseVoxelOctree::new();
        let matter = ChunkMatter::new(&tree, [0, 0, 0]);
        assert_eq!(
            transmittance_tree(&matter, &tree, [0.5; 3], [0.0; 3], 10.0),
            0.0,
            "a walk that cannot be aimed is not a clear one"
        );
        assert_eq!(transmittance_tree(&matter, &tree, [f64::NAN; 3], [0.0, 1.0, 0.0], 10.0), 0.0);
        assert_eq!(transmittance_tree(&matter, &tree, [0.5; 3], [0.0, 1.0, 0.0], 0.0), 0.0);
    }
}
