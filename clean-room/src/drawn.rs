//! **U4 — the drawn reading: the mesh's own rectangles to pixels, under one camera with depth.**
//!
//! # Two readings of one world, each with **one** owner
//!
//! Until this module there was one way a world became a picture, and it was the **plan reading**:
//! `surface::Surface::render` shades **one sample per column** of a [read](crate::being::WorldRead) and
//! paints a `ZOOM × ZOOM` block per column, with no depth of any kind. The mesher's own geometry — the
//! stitched rectangles U4 exists to produce — was **counted and never drawn**: the run printed its quads
//! and vertices and no frame contained one of them.
//!
//! This module is the second reading, and it is *named*: the **drawn reading** consumes the mesh's own
//! vertices and indices, projects them through one camera, and keeps the nearest rectangle per pixel in a
//! depth buffer. Each reading has exactly one owner and one entry point, and the run prints which one drew
//! the frame it describes. Two render paths standing **unlabelled** is the defect; two labelled readings,
//! one of which is the mesh's own geometry, is the seam.
//!
//! | concern | owner |
//! |---|---|
//! | the **plan reading** — a world's own columns to a block of pixels | `surface::Surface::render` |
//! | the **drawn reading** — the mesh's own rectangles to pixels, with depth | [`draw`] |
//! | the **camera** — where the eye stands and what it sees | [`Camera`] |
//! | the **subject of a frame** — which rectangles are drawn | `surface::region_mesh` (the region door) |
//! | the **transfer function** — a linear colour to a byte | [`crate::presentation::encode`] |
//!
//! # What is drawn, exactly
//!
//! Every rectangle of the mesh is submitted: the loop walks [`Mesh::quads`] **in the mesh's own declared
//! order** and does three things per rectangle, once each —
//!
//! 1. reads its material's admitted reflectance out of the registry and shades it through the contract's one
//!    [`presentation::shade`] call, with the rectangle's **own** normal (a face's normal is its axis and the
//!    way it looks) and its own direction toward the eye. A rectangle is **flat**, so one shade call per
//!    rectangle is the exact answer rather than an approximation: the picture's luminance bands are the
//!    bands of the shading the geometry really has;
//! 2. transforms its **four corners** ([`corners`] — the four vertices [`Quad::vertices`] counts) and
//!    submits its **two triangles** ([`INDICES`] — the six indices [`Quad::indices`] counts);
//! 3. rasterises each triangle against the depth buffer.
//!
//! The counts printed with the frame are the loop's **own** counts, and the frame also carries the mesh's own
//! declared counts, so the two can be compared rather than assumed ([`Drawn::counts_agree`]).
//!
//! # Depth
//!
//! The depth buffer holds the **camera-space depth** of the rectangle that owns each pixel, and the test is
//! *nearer wins*. Depth is interpolated as `1/z` (the reciprocal, the quantity that is linear in screen
//! space) rather than as `z`, so a slanted rectangle's depth is the depth of the surface at that pixel and
//! not a screen-space artefact. Trivial rejection is absent on purpose: every rectangle of the mesh is
//! submitted and the buffer decides, so *quads submitted* is the mesh's own number rather than a number a
//! culling heuristic produced. A triangle with a corner nearer than the near plane is **clipped whole and
//! counted**, and the **rectangles** the eye cannot see are counted beside the triangles
//! ([`Drawn::unseen`]) — because a camera a person drives stands *inside* the world it draws, so the eye's own
//! near plane legitimately swallows what is behind it (2026-10-14e). What matters is that the two counts
//! agree — one unseen rectangle clips exactly its own two triangles, so `clipped == 2 × unseen` — and that is
//! the claim [`crate::surface::Drawing::bars`] gates: nothing is dropped between the eye and the picture.
//!
//! # Refuse, never default (P2)
//!
//! A material whose record carries **no** reflectance refuses the frame **by name**, with the rectangle's own
//! axis and plane — never a grey rectangle. A mesh with no rectangle, a camera whose field of view is not an
//! angle, and a frame of zero pixels are each refused by name. A direction that is not a unit vector is
//! refused by [`presentation::shade`] itself, and the refusal is passed through rather than replaced by a
//! plausible colour.

use crate::digest::Digest;
use crate::material::MaterialRegistry;
use crate::mesh::{Axis, Mesh, Quad};
use crate::presentation::{self, Light, Rgb};
use crate::refusal::{Refusal, RefusalKind};

/// **The drawn frame's own size**, in pixels and square: the client's declared picture, the same
/// `DECLARED_PATCH_SIDE × ZOOM` the plan reading holds, so one window shows either reading without a resize.
pub const DRAWN_SIZE: u32 = 576;

/// **The camera's declared field of view**, in degrees — the whole vertical angle the eye sees. The eye's
/// distance is *derived* from it and the mesh's own extent ([`Camera::over`]), so this is the one number the
/// camera's placement is declared with.
pub const DRAWN_FOV_DEG: f64 = 45.0;

/// The near plane's own fraction of the **mesh's own extent**, and the far plane's own multiple of it.
///
/// Declared together because one sentence explains both: *the planes are the extent's own size and not the
/// eye's distance from it*. Until 2026-10-14e they were fractions of the eye's distance, which was the same
/// statement as long as the eye stood off the whole extent along the declared direction ([`Camera::over`]'s
/// own placement): there the nearest corner is at least `distance - radius` and the farthest at most
/// `distance + radius`. **A camera a person drives stands anywhere**, so a near plane measured from the eye
/// would swallow more of the world the closer the person flew and would leave a hole at the feet of an eye
/// standing on the ground. Measured from the extent, the near plane is **0.46 m** for this client's region
/// (radius 45.72 m) wherever the eye is, and the far plane is ten times the extent: whatever a person's own
/// eye can see of the region stands inside both.
pub const NEAR_FRACTION: f64 = 0.01;
pub const FAR_MULTIPLE: f64 = 10.0;

/// **How far ahead of the eye its own look is expressed**, in metres.
///
/// A camera has an eye and a target, and the drawn reading's own basis is built from the direction between
/// them. A driven eye's target is therefore a point on the ray the person is looking along — this far ahead —
/// rather than a second position the camera keeps: [`Camera::standing`] takes the look itself, and `looking
/// at` in a printed line names a point on that ray. The number is this client's, declared here and printed
/// with every frame's own camera.
pub const LOOK_M: f64 = 10.0;

/// **The six indices of a rectangle's two triangles**, in the corner order [`corners`] returns: `(0, 1, 2)`
/// and `(0, 2, 3)`. Stated once, in one place, because it is this reading's reading of [`Quad::indices`]' own
/// six.
pub const INDICES: [[usize; 3]; 2] = [[0, 1, 2], [0, 2, 3]];

/// **What the client declares about a material the record does not carry** — OpenPBR's reduced subset here is
/// `base_color`, `specular_roughness` and `base_metalness`, and this world's material records carry the first
/// alone (U1's own spec: parameters the model does not record are **owed and named absent**, never defaulted).
/// The exposure is the linear gain the frame's channels are given before the transfer function. Passed in
/// rather than read from a global, so this module owns no client constant: the drawn reading and the plan
/// reading are given the **same** declared values by their one caller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub roughness: f64,
    pub metalness: f64,
    pub exposure: f64,
}

// ---------------------------------------------------------------------------
// The geometry's own corners, and the camera.
// ---------------------------------------------------------------------------

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(v: [f64; 3]) -> f64 {
    dot(v, v).sqrt()
}

/// A direction normalised by the client's one arithmetic, or `None` when it has no direction at all.
fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let len = length(v);
    if len == 0.0 || !len.is_finite() {
        return None;
    }
    Some([v[0] / len, v[1] / len, v[2] / len])
}

/// **The rectangle's four corners in the world's own metres**, in the declared order: `(u, v)`,
/// `(u + span_u, v)`, `(u + span_u, v + span_v)`, `(u, v + span_v)` — the two coordinates [`Axis::spans`]
/// names for the rectangle's own axis, the face's plane just outside the cells that show it, exactly as
/// [`Quad`] states them. The four vertices [`Quad::vertices`] counts, and the two triangles [`INDICES`]
/// indexes.
pub fn corners(quad: &Quad) -> [[f64; 3]; 4] {
    let (u, v, plane) = (f64::from(quad.u), f64::from(quad.v), f64::from(quad.plane));
    let (span_u, span_v) = (f64::from(quad.span_u), f64::from(quad.span_v));
    let at = |du: f64, dv: f64| -> [f64; 3] {
        match quad.axis {
            Axis::X => [plane, u + du, v + dv],
            Axis::Y => [u + du, plane, v + dv],
            Axis::Z => [u + du, v + dv, plane],
        }
    };
    [
        at(0.0, 0.0),
        at(span_u, 0.0),
        at(span_u, span_v),
        at(0.0, span_v),
    ]
}

/// **The rectangle's own normal** — its axis, and the way it looks: `+1` the way the axis grows, `-1` the way
/// it falls. One normal per rectangle, because a rectangle is flat.
pub fn normal_of(quad: &Quad) -> [f64; 3] {
    let mut normal = [0.0_f64; 3];
    normal[match quad.axis {
        Axis::X => 0,
        Axis::Y => 1,
        Axis::Z => 2,
    }] = if quad.facing >= 0 { 1.0 } else { -1.0 };
    normal
}

/// **The mesh's own extent**, in the world's own coordinates: the lowest and highest corner over every
/// rectangle. `None` for a mesh that holds no rectangle — no extent exists, and an extent invented for it
/// would be a camera pointed at nothing.
pub fn bounds(mesh: &Mesh) -> Option<([f64; 3], [f64; 3])> {
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    let mut any = false;
    for quad in mesh.quads() {
        for corner in corners(quad) {
            any = true;
            for axis in 0..3 {
                low[axis] = low[axis].min(corner[axis]);
                high[axis] = high[axis].max(corner[axis]);
            }
        }
    }
    any.then_some((low, high))
}

/// **The camera's own orthonormal basis** — which way is right on the screen, which way is up on it, and
/// which way the eye looks — each a unit vector in the world's own coordinates.
pub type Basis = ([f64; 3], [f64; 3], [f64; 3]);

/// **The eye, and what it sees** — where it stands, what it looks at, which way is up, and the four declared
/// planes of its own frustum.
///
/// The camera's **planes and its own reach are a function of the mesh's own extent** and of nothing else: the
/// radius is the extent's circumscribed sphere, the near plane a hundredth of it and the far plane ten times
/// it, so *how much of the region is drawn* never depends on where the eye happens to be. Its **eye and
/// target** are the caller's: [`Camera::over_extent`] stands the eye off along the direction toward the
/// viewer at the one distance where the extent's sphere subtends its field of view (`distance = radius /
/// sin(fov / 2)`, so no standoff is invented), and [`Camera::standing`] puts the eye where a person drove it
/// (2026-10-14e) — both through the one placement below, so the two cannot disagree about the planes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Where the eye stands, in the world's own metres.
    pub eye: [f64; 3],
    /// What it looks at — the centre of the mesh's own extent, or a point on the ray a person's own eye looks
    /// along ([`Camera::standing`], [`LOOK_M`]).
    pub target: [f64; 3],
    /// **Which way the eye looks** — a unit vector, from the eye toward what it looks at.
    ///
    /// It is stored rather than re-derived from `target - eye` on every use (2026-10-14e) because a person's
    /// own eye is placed **and aimed** by its own state (`crate::viewpoint::Viewpoint`): a look that went out
    /// through `target - eye` and back would come home differing in its last bits, and a client whose recorded
    /// frame must return byte for byte when the eye comes back cannot afford that. [`Camera::basis`] and
    /// [`Camera::toward`] read this field, and the two doors below set it from their own own arithmetic —
    /// `over_extent` from the standoff it just computed, `standing` from the person's own look.
    pub look: [f64; 3],
    /// Which way is up **in the world**: the world's own `+z`, orthonormalised against the view inside
    /// [`Camera::basis`].
    pub up: [f64; 3],
    pub fov_deg: f64,
    pub near: f64,
    pub far: f64,
    /// The radius of the extent's own bounding sphere — half its box's diagonal, the extent the planes are
    /// derived from.
    pub radius: f64,
}

impl Camera {
    /// **Place the eye over one mesh**, from the mesh's own bounds.
    pub fn over(mesh: &Mesh, toward: [f64; 3], fov_deg: f64) -> Result<Camera, Refusal> {
        let Some((low, high)) = bounds(mesh) else {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the mesh holds no rectangle: there is no extent to stand a camera over",
            ));
        };
        Camera::over_extent(low, high, toward, fov_deg)
    }

    /// **Place the eye over one extent.** `low` and `high` are the box's own corners in the world's
    /// coordinates; the direction must be a unit vector (this client's own `surface::view` is), and the field
    /// of view must be an angle strictly between zero and a half turn; a box that is not a box and a box of no
    /// size are each refused by name.
    pub fn over_extent(
        low: [f64; 3],
        high: [f64; 3],
        toward: [f64; 3],
        fov_deg: f64,
    ) -> Result<Camera, Refusal> {
        let (target, radius) = Camera::extent(low, high, fov_deg)?;
        let Some(toward) = unit(toward) else {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the declared view direction has no direction: a camera cannot stand off along it",
            ));
        };
        let distance = radius / (fov_deg / 2.0).to_radians().sin();
        let eye = [
            target[0] + toward[0] * distance,
            target[1] + toward[1] * distance,
            target[2] + toward[2] * distance,
        ];
        // The look is the direction between the two places **as computed**, not the negated argument: that is
        // the direction this door has always projected with, so a frame drawn through it does not move.
        let look = unit(sub(target, eye)).expect("the standoff eye is not its own target");
        Camera::standing_over(low, high, eye, look, target, fov_deg)
    }

    /// **Stand the eye where a person put it** — the extent's own mesh, the person's own eye, and the
    /// direction they look along ([`Camera::standing`] is the drawn reading's door for a viewpoint:
    /// `crate::viewpoint::Viewpoint` hands its own eye and look here).
    ///
    /// The planes are the extent's own ([`NEAR_FRACTION`]), so a person standing *inside* the region they are
    /// drawing still sees what is in front of them: the eye's own place is the caller's and nothing here
    /// invents a standoff for it. A look with no direction is refused by name rather than pointed at the eye.
    pub fn standing(mesh: &Mesh, eye: [f64; 3], look: [f64; 3], fov_deg: f64) -> Result<Camera, Refusal> {
        let Some((low, high)) = bounds(mesh) else {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the mesh holds no rectangle: there is no extent to stand a camera in",
            ));
        };
        let Some(look) = unit(look) else {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the person's own look has no direction: a camera cannot be pointed along nothing",
            ));
        };
        let target = [
            eye[0] + look[0] * LOOK_M,
            eye[1] + look[1] * LOOK_M,
            eye[2] + look[2] * LOOK_M,
        ];
        // **The person's own look is the camera's own**, bit for bit: it is stored, never re-derived.
        Camera::standing_over(low, high, eye, look, target, fov_deg)
    }

    /// The one placement, with the look named: [`Camera::over_extent`] and [`Camera::standing`] differ only in
    /// where the eye, the look and the target come from.
    fn standing_over(
        low: [f64; 3],
        high: [f64; 3],
        eye: [f64; 3],
        look: [f64; 3],
        target: [f64; 3],
        fov_deg: f64,
    ) -> Result<Camera, Refusal> {
        let Some(look) = unit(look) else {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the eye stands on what it looks at: the camera has no forward direction",
            ));
        };
        if (0..3).any(|axis| !eye[axis].is_finite() || !target[axis].is_finite()) {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the eye or what it looks at is not a place in the world",
            ));
        }
        let (_, radius) = Camera::extent(low, high, fov_deg)?;
        Ok(Camera {
            eye,
            target,
            look,
            up: [0.0, 0.0, 1.0],
            fov_deg,
            near: radius * NEAR_FRACTION,
            far: radius * FAR_MULTIPLE,
            radius,
        })
    }

    /// The extent's own centre and the radius of its circumscribed sphere, with the field of view checked —
    /// the one arithmetic both placements above are made of.
    fn extent(low: [f64; 3], high: [f64; 3], fov_deg: f64) -> Result<([f64; 3], f64), Refusal> {
        if !(0.0 < fov_deg && fov_deg < 180.0) {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!("a field of view of {fov_deg}° is not an angle a camera sees with"),
            ));
        }
        if (0..3).any(|axis| {
            low[axis] > high[axis] || !low[axis].is_finite() || !high[axis].is_finite()
        }) {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the extent's own corners do not describe a box: a camera has nothing to stand over",
            ));
        }
        let target = [
            (low[0] + high[0]) / 2.0,
            (low[1] + high[1]) / 2.0,
            (low[2] + high[2]) / 2.0,
        ];
        // The box's own half-diagonal: the circumscribed sphere's radius, in one arithmetic from the extent.
        let radius = length([
            (high[0] - low[0]) / 2.0,
            (high[1] - low[1]) / 2.0,
            (high[2] - low[2]) / 2.0,
        ]);
        if radius == 0.0 {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the extent's own box has no size: there is no extent to see",
            ));
        }
        Ok((target, radius))
    }

    /// The distance from the eye to what it looks at.
    pub fn distance(&self) -> f64 {
        length(sub(self.target, self.eye))
    }

    /// The unit direction **toward the viewer**, from the world — the same reading of a direction the
    /// contract's [`presentation::View`] takes, so a rectangle's own direction toward the eye is the same kind
    /// of quantity the plan reading hands to the same [`presentation::shade`].
    ///
    /// It is the eye's own look, negated exactly ([`Camera::look`]): one stored direction, so a viewpoint read
    /// off a camera and handed back to it is the same direction and not a copy that drifted.
    pub fn toward(&self) -> [f64; 3] {
        [-self.look[0], -self.look[1], -self.look[2]]
    }

    /// The camera's own orthonormal basis: `(right, up, forward)`, each a unit vector, with `forward` the
    /// direction from the eye to the target. Refused by name when the view runs along the world's own up (no
    /// right vector exists, and picking one would invent a roll).
    pub fn basis(&self) -> Result<Basis, Refusal> {
        let forward = self.look;
        let Some(right) = unit(cross(self.up, forward)) else {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "the declared view runs along the world's own up: there is no right direction, and inventing \
                 a roll would be inventing a camera",
            ));
        };
        Ok((right, cross(right, forward), forward))
    }

    /// **The one projection**: a world point to `(screen x, screen y, camera-space depth)`, or `None` when the
    /// point stands at or behind the near plane. Screen coordinates are in pixels with `x` growing to the right
    /// and `y` growing **down**, which is the order the frame's own bytes are in — one projection, so the
    /// depths the buffer holds and the pixels it keeps are the same arithmetic.
    pub fn project(&self, point: [f64; 3], size: u32) -> Result<Option<(f64, f64, f64)>, Refusal> {
        let (right, up, forward) = self.basis()?;
        let to_point = sub(point, self.eye);
        let depth = dot(to_point, forward);
        if depth < self.near || !depth.is_finite() {
            return Ok(None);
        }
        let scale = depth * (self.fov_deg / 2.0).to_radians().tan();
        let size = f64::from(size);
        let x = (0.5 + 0.5 * dot(to_point, right) / scale) * size;
        let y = (0.5 - 0.5 * dot(to_point, up) / scale) * size;
        Ok(Some((x, y, depth)))
    }

    /// The camera's own numbers, for a reader of the frame's printed line.
    pub fn described(&self) -> String {
        format!(
            "eye ({:.2}, {:.2}, {:.2}) m looking at ({:.2}, {:.2}, {:.2}) m, {:.0}° of view, radius {:.2} \
             m, distance {:.2} m, near {:.2} m, far {:.2} m",
            self.eye[0],
            self.eye[1],
            self.eye[2],
            self.target[0],
            self.target[1],
            self.target[2],
            self.fov_deg,
            self.radius,
            self.distance(),
            self.near,
            self.far
        )
    }
}

// ---------------------------------------------------------------------------
// The frame.
// ---------------------------------------------------------------------------

/// **One drawn frame** — its pixels, its depth buffer, the shaded sample of every rectangle it submitted, and
/// the counts that say what reached them.
///
/// The pixels are in the byte order the platform's bitmaps use (**BGRA, top-down, four bytes to a pixel**),
/// exactly the order `surface::Surface` hands the window, so one window presents either reading without
/// knowing which it holds. A pixel no rectangle covers keeps the zero it was made with and is counted as
/// **uncovered** rather than filled with an invented background.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    size: u32,
    pixels: Vec<u8>,
    depth: Vec<f64>,
    samples: Vec<Rgb>,
    quads: u64,
    vertices: u64,
    triangles: u64,
    clipped: u64,
    /// **The rectangles the eye could not see at all** — at least one of a rectangle's four corners stood at
    /// or behind the eye's own near plane, so neither of its triangles was drawn. Counted per rectangle, and
    /// the triangles it costs are exactly its own two ([`Drawn::clipped`]), so the pair says *nothing was
    /// dropped between the eye and the picture* — the claim `crate::surface::Drawing::bars` gates
    /// (2026-10-14e). A camera a person drives stands inside the world it draws, so this is a state and not a
    /// defect: what would be a defect is a rectangle that is neither drawn nor counted here.
    unseen: u64,
    degenerate: u64,
    declared: (u64, u64, u64, u64),
    mesh: Digest,
    mesh_state: Digest,
    camera: Camera,
    shaded: u64,
    depth_min: f64,
    depth_max: f64,
    digest: Digest,
}

impl Drawn {
    /// The frame's own size, in pixels, on both axes.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// The frame's pixels **as the platform is handed them** (BGRA, top-down, `size * size * 4` bytes).
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// One pixel, in the buffer's own order (`[b, g, r, a]`), or nothing where the point is off the frame.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.size || y >= self.size {
            return None;
        }
        let at = ((y * self.size + x) * 4) as usize;
        let mut pixel = [0_u8; 4];
        pixel.copy_from_slice(&self.pixels[at..at + 4]);
        Some(pixel)
    }

    /// **The camera-space depth the frame kept at one pixel** — `None` where no rectangle reached it.
    pub fn depth_at(&self, x: u32, y: u32) -> Option<f64> {
        if x >= self.size || y >= self.size {
            return None;
        }
        let depth = self.depth[(y * self.size + x) as usize];
        depth.is_finite().then_some(depth)
    }

    /// The shaded sample of every rectangle the frame submitted, in the mesh's own declared order — the same
    /// [`presentation::shade`] results the pixels were encoded from, so the FMA bar's instrument
    /// ([`presentation::band_count`]) counts bands on the shading the picture is made of.
    pub fn samples(&self) -> &[Rgb] {
        &self.samples
    }

    /// **The FMA bar, read on this frame's own samples** — the count [`presentation::band_count`] makes.
    pub fn bands(&self) -> usize {
        presentation::band_count(&self.samples)
    }

    /// The rectangles the loop submitted.
    pub fn quads(&self) -> u64 {
        self.quads
    }

    /// The corners the loop transformed (`4` per rectangle, [`Quad::vertices`]).
    pub fn vertices(&self) -> u64 {
        self.vertices
    }

    /// The triangles the loop submitted (`2` per rectangle, [`Quad::triangles`]).
    pub fn triangles(&self) -> u64 {
        self.triangles
    }

    /// The indices those triangles are made of (`3` each — [`Quad::indices`]' own six per rectangle).
    pub fn indices(&self) -> u64 {
        self.triangles * 3
    }

    /// Triangles a corner of which stood nearer than the near plane, and were not rasterised.
    pub fn clipped(&self) -> u64 {
        self.clipped
    }

    /// **The rectangles the eye did not see at all** — each of them costing exactly its own two triangles
    /// ([`Drawn::clipped`]). A person's own eye stands inside the region it draws, so the rectangles behind it
    /// are a state rather than a defect; a rectangle that is neither drawn nor counted here is the defect, and
    /// `clipped == 2 × unseen` is that sentence as arithmetic.
    pub fn unseen(&self) -> u64 {
        self.unseen
    }

    /// Triangles with no area on the screen — a rectangle seen exactly edge-on.
    pub fn degenerate(&self) -> u64 {
        self.degenerate
    }

    /// The mesh's own declared counts: rectangles, vertices, triangles and indices. Carried beside the loop's
    /// counts so the two can be compared ([`Drawn::counts_agree`]) rather than assumed equal.
    pub fn declared(&self) -> (u64, u64, u64, u64) {
        self.declared
    }

    /// **Whether the drawing loop and the mesh agree about what was submitted** — the loop counted the corners
    /// it transformed and the triangles it submitted, the mesh declared four vertices, two triangles and six
    /// indices per rectangle, and this is the two numbers compared.
    pub fn counts_agree(&self) -> bool {
        let (quads, vertices, triangles, indices) = self.declared;
        self.quads == quads
            && self.vertices == vertices
            && self.triangles == triangles
            && self.indices() == indices
    }

    /// The pixels a rectangle reached, counted **from the frame's own depth buffer** rather than from a
    /// running total, so the number describes the picture that exists.
    pub fn shaded(&self) -> u64 {
        self.shaded
    }

    /// The frame's whole pixel count.
    pub fn pixels_count(&self) -> u64 {
        u64::from(self.size) * u64::from(self.size)
    }

    /// **The frame's coverage** — the fraction of its own pixels a rectangle reached.
    pub fn coverage(&self) -> f64 {
        self.shaded as f64 / self.pixels_count() as f64
    }

    /// The shallowest and deepest camera-space depth the frame kept, or `None` when nothing was drawn.
    pub fn depth_extent(&self) -> Option<(f64, f64)> {
        (self.shaded > 0).then_some((self.depth_min, self.depth_max))
    }

    /// The frame's own darkest and brightest channel byte over the pixels it drew — two measured numbers that
    /// say the picture is not a flat fill.
    pub fn range(&self) -> (u8, u8) {
        let mut dark = u8::MAX;
        let mut bright = 0_u8;
        for (index, pixel) in self.pixels.as_chunks::<4>().0.iter().enumerate() {
            if self.depth[index].is_finite() {
                for channel in &pixel[0..3] {
                    dark = dark.min(*channel);
                    bright = bright.max(*channel);
                }
            }
        }
        (dark, bright)
    }

    /// The camera the frame was drawn under.
    pub fn camera(&self) -> Camera {
        self.camera
    }

    /// The mesh's own geometry identity ([`Mesh::digest`]).
    pub fn mesh(&self) -> Digest {
        self.mesh
    }

    /// **The state digest the drawn frame came from** — the mesh's own key state, which for a mesh of a cell
    /// world is that world's own identity ([`crate::mesh::GeometryKey::of_state`]).
    pub fn mesh_state(&self) -> Digest {
        self.mesh_state
    }

    /// The frame's own identity: its size and every pixel byte, in the buffer's own order. Two drawings of one
    /// mesh under one camera are one digest.
    pub fn digest(&self) -> Digest {
        self.digest
    }

    /// The frame's own pixels as a **P6 portable pixmap**, in RGB order — written from the bytes the window
    /// was handed rather than from a second drawing of the world.
    pub fn write_ppm(&self, path: &std::path::Path) -> Result<(), Refusal> {
        write_ppm(path, self.size, self.size, &self.pixels)
    }
}

/// **The one P6 writer** — a frame's own bytes as a portable pixmap, in RGB order, used by both readings so a
/// screenshot cannot be a second drawing of the world.
pub fn write_ppm(
    path: &std::path::Path,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<(), Refusal> {
    let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
    bytes.reserve(pixels.len() / 4 * 3);
    for pixel in pixels.as_chunks::<4>().0 {
        bytes.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
    }
    std::fs::write(path, &bytes).map_err(|error| {
        Refusal::new(
            RefusalKind::DocumentUnwritable,
            format!("the frame cannot be written at {}: {error}", path.display()),
        )
    })
}

/// The refusal `shade` made, named as the **contract's** own refusal with the drawn reading's rectangle it
/// happened at — a refusal is passed through rather than replaced by a plausible colour.
fn shade_refusal(kind: presentation::RefusalKind, quad: &Quad) -> Refusal {
    let named = match kind {
        presentation::RefusalKind::NoReflectance => RefusalKind::NoReflectance,
        // A direction this reading built was not a unit vector: the fault is this reading's arithmetic, and
        // the contract's own name is carried in the detail.
        presentation::RefusalKind::NotAUnitVector => RefusalKind::ParameterOutOfRange,
        // The **ported** appearance contract's own refusal (P7 v2.0.0): carried through as this reading's
        // parameter fault, with the ported kind's own name in the detail rather than a plausible colour.
        presentation::RefusalKind::AppearanceRefused => RefusalKind::ParameterOutOfRange,
    };
    Refusal::new(
        named,
        format!(
            "the one shade call refused `{}` at the {} rectangle on plane {} at ({}, {})",
            kind.name(),
            quad.axis.name(),
            quad.plane,
            quad.u,
            quad.v
        ),
    )
}

/// **A triangle's own screen area**, as the cross product of two of its edges — the sign is the winding, and
/// zero is a rectangle seen exactly edge-on (no pixel of it exists).
fn screen_area(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (c.0 - a.0) * (b.1 - a.1)
}

/// The barycentric coverage of a screen point by a triangle, as `(w0, w1, w2)` over the triangle's own
/// corners. The three weights sum to one, so the interpolation of any vertex property at the point is
/// `w0·a + w1·b + w2·c`.
fn barycentric(
    (x0, y0): (f64, f64),
    (x1, y1): (f64, f64),
    (x2, y2): (f64, f64),
    (px, py): (f64, f64),
) -> (f64, f64, f64) {
    let inv = 1.0 / ((x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0));
    let w0 = ((y1 - y2) * (px - x2) + (x2 - x1) * (py - y2)) * inv;
    let w1 = ((y2 - y0) * (px - x2) + (x0 - x2) * (py - y2)) * inv;
    (w0, w1, 1.0 - w0 - w1)
}

/// **One triangle, into the depth buffer** — the pixel centres of the triangle's own screen bounding box are
/// tested against its barycentric coverage, their depth is interpolated as `1/z`, and a pixel keeps a colour
/// and a depth only where it is the **shallowest** surface seen so far. Corners off the frame are clipped by
/// the bounding box, so no pixel outside the frame is ever computed.
fn raster(
    pixels: &mut [u8],
    depth: &mut [f64],
    size: u32,
    a: (f64, f64, f64),
    b: (f64, f64, f64),
    c: (f64, f64, f64),
    colour: [u8; 4],
) {
    let last = f64::from(size - 1);
    let low_x = a.0.min(b.0).min(c.0).floor().clamp(0.0, last) as u32;
    let high_x = a.0.max(b.0).max(c.0).ceil().clamp(0.0, last) as u32;
    let low_y = a.1.min(b.1).min(c.1).floor().clamp(0.0, last) as u32;
    let high_y = a.1.max(b.1).max(c.1).ceil().clamp(0.0, last) as u32;
    for y in low_y..=high_y {
        for x in low_x..=high_x {
            let point = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let (w0, w1, w2) = barycentric((a.0, a.1), (b.0, b.1), (c.0, c.1), point);
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let z = 1.0 / (w0 / a.2 + w1 / b.2 + w2 / c.2);
            let at = (y * size + x) as usize;
            if z < depth[at] {
                depth[at] = z;
                pixels[at * 4..at * 4 + 4].copy_from_slice(&colour);
            }
        }
    }
}

/// **Draw one mesh** — the drawn reading's one entry point.
///
/// Every rectangle of the mesh is submitted, in the mesh's own declared order; each is shaded once, with its
/// own normal and its own direction toward the eye; its four corners are transformed and its two triangles
/// rasterised against a depth buffer that keeps the **nearest** surface per pixel. The colour a pixel is given
/// is the linear colour the one `shade` call returned, put through the caller's own declared transfer function
/// — so the picture's bytes and the measurement's numbers are the same colours.
pub fn draw(
    mesh: &Mesh,
    registry: &MaterialRegistry,
    light: &Light,
    camera: &Camera,
    size: u32,
    look: Look,
    encode: fn(Rgb, f64) -> [u8; 4],
) -> Result<Drawn, Refusal> {
    if size == 0 {
        return Err(Refusal::new(
            RefusalKind::ParameterOutOfRange,
            "a frame of 0 pixel(s) names no pixel to draw into",
        ));
    }
    let total = (size as usize) * (size as usize);
    let mut pixels = vec![0_u8; total * 4];
    let mut depth = vec![f64::INFINITY; total];
    let mut samples = Vec::with_capacity(mesh.quads().len());
    let mut quads = 0_u64;
    let mut vertices = 0_u64;
    let mut triangles = 0_u64;
    let mut clipped = 0_u64;
    let mut unseen = 0_u64;
    let mut degenerate = 0_u64;

    for quad in mesh.quads() {
        let material = registry.material(quad.material)?;
        // **The material's own appearance**, through the contract's one conversion (P5): which material
        // stands here is the cell's fact and the mesh's own rectangle ([`Quad::material`]); what it looks
        // like is the contract's. Where it derives no colour the refusal names the material and the rectangle
        // it was asked for rather than shading a grey rectangle.
        let surface = presentation::surface_of(material, look.roughness, look.metalness).map_err(|kind| {
            Refusal::new(
                RefusalKind::NoReflectance,
                format!(
                    "material {} ({}), drawn as the {} rectangle on plane {} at ({}, {}), carries no \
                     reflectance: a colour cannot be derived from it (`{}`)",
                    quad.material.0,
                    material.name,
                    quad.axis.name(),
                    quad.plane,
                    quad.u,
                    quad.v,
                    kind.name()
                ),
            )
        })?;
        let corners = corners(quad);
        let centre = [
            (corners[0][0] + corners[2][0]) / 2.0,
            (corners[0][1] + corners[2][1]) / 2.0,
            (corners[0][2] + corners[2][2]) / 2.0,
        ];
        // The rectangle's own direction toward the eye: the view is the camera's, so a rectangle at the edge
        // of a wide frame is shaded edge-on and one under the eye is shaded face-on.
        let Some(view) = unit(sub(camera.eye, centre)) else {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "the eye stands on the {} rectangle on plane {} at ({}, {}): there is no direction \
                     toward the viewer",
                    quad.axis.name(),
                    quad.plane,
                    quad.u,
                    quad.v
                ),
            ));
        };
        let shaded = presentation::shade(
            &surface,
            normal_of(quad),
            light,
            &presentation::View { toward: view },
        )
        .map_err(|kind| shade_refusal(kind, quad))?;
        samples.push(shaded);
        let colour = encode(shaded, look.exposure);

        // The corners, projected: a corner at or behind the near plane clips its rectangle's triangles whole
        // rather than being moved onto the plane, which would draw geometry at a depth it does not have.
        let mut screen = [(0.0_f64, 0.0_f64, 0.0_f64); 4];
        let mut in_front = true;
        for (at, corner) in corners.iter().enumerate() {
            vertices += 1;
            match camera.project(*corner, size)? {
                Some(projected) => screen[at] = projected,
                None => in_front = false,
            }
        }
        quads += 1;
        if !in_front {
            unseen += 1;
        }
        for triangle in INDICES {
            triangles += 1;
            if !in_front {
                clipped += 1;
                continue;
            }
            let (a, b, c) = (screen[triangle[0]], screen[triangle[1]], screen[triangle[2]]);
            if screen_area((a.0, a.1), (b.0, b.1), (c.0, c.1)) == 0.0 {
                degenerate += 1;
                continue;
            }
            raster(&mut pixels, &mut depth, size, a, b, c, colour);
        }
    }

    let mut shaded = 0_u64;
    let (mut depth_min, mut depth_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for value in &depth {
        if value.is_finite() {
            shaded += 1;
            depth_min = depth_min.min(*value);
            depth_max = depth_max.max(*value);
        }
    }
    let mut bytes = Vec::with_capacity(pixels.len() + 8);
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&pixels);
    Ok(Drawn {
        size,
        pixels,
        depth,
        samples,
        quads,
        vertices,
        triangles,
        clipped,
        unseen,
        degenerate,
        declared: (
            mesh.quads().len() as u64,
            mesh.vertices(),
            mesh.triangles(),
            mesh.indices(),
        ),
        mesh: mesh.digest(),
        mesh_state: mesh.key().state,
        camera: *camera,
        shaded,
        depth_min,
        depth_max,
        digest: Digest::of(&bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rectangle on the ground plane, one metre square, at the world's origin — enough geometry for the
    /// arithmetic below to be checked without a world.
    fn quad() -> Quad {
        Quad {
            layer: crate::mesh::Layer::Solid,
            axis: Axis::Z,
            facing: 1,
            plane: 1,
            u: 0,
            v: 0,
            span_u: 1,
            span_v: 1,
            material: crate::material::MaterialId(1),
        }
    }

    #[test]
    fn a_rectangles_four_corners_are_the_vertices_and_its_two_triangles_the_indices() {
        let corners = corners(&quad());
        assert_eq!(corners[0], [0.0, 0.0, 1.0]);
        assert_eq!(corners[1], [1.0, 0.0, 1.0]);
        assert_eq!(corners[2], [1.0, 1.0, 1.0]);
        assert_eq!(corners[3], [0.0, 1.0, 1.0]);
        assert_eq!(corners.len() as u64, quad().vertices());
        // The declared index list is two triangles of three indices, and every corner is used by one.
        let mut used = [false; 4];
        for triangle in INDICES {
            for at in triangle {
                used[at] = true;
            }
        }
        assert!(used.iter().all(|used| *used));
        assert_eq!((INDICES.len() * 3) as u64, quad().indices());
        assert_eq!(INDICES.len() as u64, quad().triangles());
    }

    #[test]
    fn a_rectangles_normal_is_its_axis_and_the_way_it_looks() {
        assert_eq!(normal_of(&quad()), [0.0, 0.0, 1.0]);
        assert_eq!(normal_of(&Quad { facing: -1, ..quad() }), [0.0, 0.0, -1.0]);
        assert_eq!(
            normal_of(&Quad { axis: Axis::X, facing: -1, ..quad() }),
            [-1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn a_camera_stands_off_so_the_extents_own_sphere_subtends_its_own_field_of_view() {
        // The camera's own arithmetic, without a world: the distance is `radius / sin(fov / 2)` and the eye
        // stands that far from the extent's centre, along the direction toward the viewer.
        let camera = Camera::over_extent([0.0, 0.0, 1.0], [8.0, 1.0, 1.0], [0.0, 0.0, 1.0], 90.0)
            .expect("the extent has a size");
        assert_eq!(camera.target, [4.0, 0.5, 1.0]);
        assert!((camera.radius - (16.0_f64 + 0.25).sqrt()).abs() < 1e-12, "{}", camera.radius);
        assert!((camera.distance() - camera.radius / 45.0_f64.to_radians().sin()).abs() < 1e-9);
        // **The planes are the extent's own** (2026-10-14e): a hundredth of its circumscribed sphere and ten
        // times it, so where the eye stands does not decide what it can see.
        assert!((camera.far - camera.radius * FAR_MULTIPLE).abs() < 1e-9);
        assert!((camera.near - camera.radius * NEAR_FRACTION).abs() < 1e-9);
        assert!((camera.eye[2] - (camera.target[2] + camera.distance())).abs() < 1e-9);
        assert_eq!(camera.eye[0], camera.target[0]);
        assert_eq!(camera.eye[1], camera.target[1]);
    }

    /// **A person's own eye is where the camera stands** — the door the drawn reading takes a viewpoint
    /// through: the eye is the person's (never a standoff invented from the extent), the look is theirs, and
    /// the planes are still the extent's own, so an eye *inside* the region it draws has the same near plane
    /// as one standing off it.
    #[test]
    fn a_standing_camera_keeps_the_persons_own_eye_and_the_extents_own_planes() {
        let mesh = camera_test_mesh();
        let eye = [4.0, 4.0, 40.0];
        let look = unit([0.0, 1.0, -1.0]).expect("a direction");
        let camera = Camera::standing(&mesh, eye, look, 60.0).expect("the person's own eye");
        assert_eq!(camera.eye, eye, "the eye is the person's own");
        for axis in 0..3 {
            assert!(
                (camera.target[axis] - (eye[axis] + look[axis] * LOOK_M)).abs() < 1e-12,
                "the target is not {LOOK_M} m along the person's own look"
            );
        }
        // The same extent as `over` sees it, so the two doors cannot disagree about the planes.
        let over = Camera::over(&mesh, [0.0, 0.0, 1.0], 60.0).expect("the declared view");
        assert_eq!(camera.radius, over.radius, "the two doors stand over one extent");
        assert_eq!(camera.near, over.near);
        assert_eq!(camera.far, over.far);
    }

    /// A small mesh with real geometry, for the camera's own arithmetic: two rectangles one metre apart on
    /// the world's own up.
    fn camera_test_mesh() -> Mesh {
        let mut quads = vec![
            Quad { plane: 0, span_u: 4, span_v: 4, ..quad() },
            Quad { plane: 4, ..quad() },
        ];
        quads[1].span_u = 4;
        quads[1].span_v = 4;
        Mesh::of(
            crate::mesh::GeometryKey::of_state(
                Digest::of(b"a mesh for the camera's own arithmetic"),
                crate::coordinates::ChunkKey::new(0, 0, 0),
                crate::mesh::Level::new(0).expect("level zero"),
            ),
            quads,
        )
    }

    #[test]
    fn a_camera_refuses_an_angle_that_is_not_one_a_view_with_no_direction_and_no_extent() {
        let view = [0.0, 0.0, 1.0];
        for fov in [0.0, 180.0, -45.0, f64::NAN] {
            assert_eq!(
                Camera::over_extent([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], view, fov)
                    .unwrap_err()
                    .kind(),
                RefusalKind::ParameterOutOfRange,
                "a field of view of {fov}° is not one"
            );
        }
        assert_eq!(
            Camera::over_extent([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [0.0, 0.0, 0.0], 45.0)
                .unwrap_err()
                .kind(),
            RefusalKind::ParameterOutOfRange
        );
        assert_eq!(
            Camera::over_extent([1.0, 0.0, 0.0], [0.0, 1.0, 1.0], view, 45.0)
                .unwrap_err()
                .kind(),
            RefusalKind::ParameterOutOfRange
        );
        assert_eq!(
            Camera::over_extent([1.0, 1.0, 1.0], [1.0, 1.0, 1.0], view, 45.0)
                .unwrap_err()
                .kind(),
            RefusalKind::ParameterOutOfRange
        );
        // A person's own look with no direction is refused the same way, and not pointed at their own eye.
        let mesh = camera_test_mesh();
        assert_eq!(
            Camera::standing(&mesh, [4.0, 4.0, 40.0], [0.0, 0.0, 0.0], 60.0)
                .unwrap_err()
                .kind(),
            RefusalKind::ParameterOutOfRange
        );
    }

    #[test]
    fn the_projection_puts_the_target_at_the_frames_own_centre() {
        // The view is tilted off the world's own up, as the client's declared view is: a view straight down
        // has no right direction, and that is a refusal this camera states rather than a roll it invents.
        let camera = Camera::over_extent([0.0, 0.0, 0.0], [4.0, 4.0, 2.0], [0.0, 0.2588, 0.9659], 60.0)
            .expect("the extent has a size");
        let (x, y, depth) =
            camera.project(camera.target, 100).expect("a projection").expect("in front");
        assert!((x - 50.0).abs() < 1e-9, "{x}");
        assert!((y - 50.0).abs() < 1e-9, "{y}");
        assert!((depth - camera.distance()).abs() < 1e-9);
        // A point at or behind the near plane is not projected at all.
        let behind = [camera.eye[0], camera.eye[1], camera.eye[2] + camera.near];
        assert!(camera.project(behind, 100).expect("a projection").is_none());
    }
}
