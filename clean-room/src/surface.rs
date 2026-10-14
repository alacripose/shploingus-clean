//! **The client — two named readings of one world, and the one window they are presented in.**
//!
//! # The two readings, and which one the window shows (§41, 2026-10-13)
//!
//! The client renders a world **twice over**, and both readings are **named**, each with exactly one owner
//! and one entry point:
//!
//! | reading | what it consumes | owner | entry |
//! |---|---|---|---|
//! | the **plan reading** | a [read](being::WorldRead)'s own columns — one `shade` call per column, a `ZOOM × ZOOM` block of pixels each, **no depth** | [`Surface`] | [`Surface::render`] |
//! | the **drawn reading** | the mesher's **own rectangles** — four corners and two triangles each, one `shade` call per rectangle, a depth buffer per pixel | [`crate::drawn`] | [`crate::drawn::draw`] |
//!
//! Until §41 the window showed the plan reading while U4's geometry was **counted and never drawn**: the run
//! printed the stitched mesh's quads and vertices and no frame contained one of them. **The window now shows
//! the drawn reading** — the world's own geometry, under one camera with depth ([`draw_plane`], the same call
//! the window's loop makes every frame) — and the plan reading is **kept and named**: it remains the evidence
//! run's own frame ([`measured_frame`]), where one sample per column is what the FMA bar's instrument counts
//! bands on. Neither reading is unlabelled, and no frame is drawn by two of them.
//!
//! Derived from the **documents** (Constitution P1/P7), never from the witness's source:
//!
//! | Source | What it declares here |
//! |---|---|
//! | `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 1 (and §2 **U1**) | the bar this slice reaches: *a **visible** client (never `nodraw`)*, whose check is *the band count and a screenshot* (`LEDGER.md` **C-40**: *an eye is the check that finds the most*) |
//! | `clean-room/specs/001-presentation/tasks.md` **T009** | the visible-frame half of the task — *a **visible** frame drawn, never `nodraw`* — with the **GPU pass** half **owed by name** here, because this client draws on the CPU through the contract's own `shade` call and a shader beside it is a second render path until it is checked against the same seam |
//! | `docs/APPEARANCE.md` §1, `docs/DESIGN-SPEC.md` §2 **F13** | the appearance contract the picture is drawn through: the reduced OpenPBR inputs with the four Source 2 terms over them — **read at the one [`presentation::shade`] call**, never re-implemented here |
//! | `LEDGER.md` **C-39** | a colour is carried **with its source**: every pixel is the reflectance of an **admitted material** ([`crate::material`]), or the frame is refused by name |
//! | `src/light.rs` (U5, §24), and its own state | the light is the **solver's own Sun** at the declared altitude — the atmospheric state's own read, not a constant — and its tint is that solver's own transmitted colour |
//! | `src/being.rs` (U12, §32/§35) and `src/automaton.rs` (§34, §38) | the world drawn is the game's own cells: the declared fields filled into their cubic world ([`crate::chunk`]), read through the one reading every consumer shares ([`being::WorldRead::over_cubic`]), with the law stepping **that world's own cells** one tick per frame |
//! | `docs/SHAPERS.md` §1 row 5 (`hydrology`), `src/chunk.rs`'s cells | the material of a column: **the cell the world filled**, read through [`CubicWorld::column`] — the one owner of what a column is made of. The `lithology` class plane is the *build's* input ([`ChunkSource::of`], [`crate::mesh::Ladder`]) and the appearance path never sees it (2026-10-14f), so a class cannot draw one material and mesh another: neither of them asks a class |
//!
//! # The three rulings this pass **took** rather than asked (§36)
//!
//! 1. **No crate is added.** The record's dependency rule is explicit and threefold: the clean room's own
//!    manifest declares `sha2` alone and says a dependency must name what it does that nothing else can
//!    (`Cargo.toml`), the plan's §2 repeats it, and `.github/instructions/rust.instructions.md` states it as
//!    *Zero External Dependencies* — *"Dependency changes violate the zero-external-dependencies
//!    constraint."* A windowing crate would therefore be a **policy breach**, so the window is built from
//!    fresh code **against the platform's own interface** ([`platform`], Win32), which is what the ruling
//!    recorded here required rather than what it preferred.
//! 2. **`unsafe` is licensed in exactly one module.** A window *is* a platform call and a platform call is
//!    `unsafe`, so the crate's `#![forbid(unsafe_code)]` became `#![deny(unsafe_code)]` with **one**
//!    module carrying `#[allow(unsafe_code)]` — this file's [`platform`] — because `forbid` cannot be
//!    narrowed and a rule that cannot be narrowed would force a second crate for one door. The rest of the
//!    crate is under the same denial as before, and `grep -rn "allow(unsafe_code)" clean-room/src` returns
//!    **one** line, which is the licence.
//! 3. **A frame is a tick.** The window's loop advances the law **once per frame** and draws the state at
//!    that tick, so the picture a person watches is the world the run's other lines measure — the cubic
//!    world's own cells routing into the marsh, exactly [`automaton::SETTLE_TICKS`] frames to rest (the
//!    declared count is that constant, not a second number). Since §38 the law's state is those cells
//!    themselves, so the frames a person watches are the very state the being's own plane is read from;
//!    this client's projection, its shading and its bars are untouched by that move. There is **no second
//!    scheduler**: the loop is the only clock, and nothing here ticks the world on its own.
//!
//! # One concern, one owner
//!
//! | concern | owner |
//! |---|---|
//! | the **plan reading's pixels** | [`Surface`] — one BGRA buffer, its own size, its own digest, and the shaded samples behind it |
//! | the **plan reading** — a world's columns to pixels | [`Surface::render`] — one loop over the world's own columns, one `shade` call each, each material from the column's own cell |
//! | the **drawn reading** — the mesh's rectangles to pixels, with depth | [`crate::drawn::draw`] — the camera, the depth buffer, the counts and the frame's own digest are that module's |
//! | the **drawn frame's subject** | [`region_mesh`] — the client's own world's **stitched region** (`2 × 2` chunks, `64 × 64` columns, one footprint) through the mesher's **region** door: one call, so the window's loop and the headless frame cannot draw two different meshes |
//! | the **presentation** — pixels to a window | [`platform::Window`] — one class, one window, one `StretchDIBits` per frame, and the read-back that says the screen holds the frame's own bytes |
//! | **where the eye stands and which way it looks**, and what a person is asking for | [`crate::viewpoint::Viewpoint`] — one state and one intent vocabulary, driven by the window's own keys here and by the shell's own words in `main.rs`, and the drawn camera is **derived** from it ([`draw_from`]) |
//! | the **one call site** that drives it | [`present_world`] — the world the tree already builds, one tick per frame, and the measured report |
//! | the **transfer function** — a linear colour to a byte | [`presentation::encode`] — one arithmetic, both readings |
//! | the **screenshot** | [`crate::drawn::write_ppm`] — the presented frame's own bytes as a P6 file, so the eye's check has an artifact rather than a promise |
//!
//! **Not built, and named rather than implied: a GPU pass** (a shader checked against
//! [`presentation::shade`] — T009's other half, and this client's own render path exists to be its oracle);
//! **a resizable window** (the client's size is declared, and a re-render at a new size is owed);
//! **an azimuth** for the Sun (the record's own owed §24 item: the client places the Sun in the patch's
//! own y–z plane and prints that, rather than inventing the axis the record has not declared); and **the
//! GPU pass's comparison figure**.
//!
//! **A person drives the camera (2026-10-14e).** Until then the drawn reading's own eye was a **function of the
//! region's extent** and of nothing else: the residency held the frame's cells and nobody could move them. The
//! eye is now the person's own ([`crate::viewpoint::Viewpoint`], driven from the window's own keys or the
//! shell's own words), and three things follow it: the **camera's own chunk** the residency is observed at, the
//! **region** the frame is a picture of ([`crate::viewpoint::Viewpoint::region_origin`]), and the frame itself
//! ([`draw_from`]). Coming back to the declared view reproduces the recorded frame byte for byte — that is a
//! guard, not a hope ([`crate::viewpoint`]'s tests own it) — and the drawn reading's own bars moved with the
//! camera: *every rectangle is inside the camera's own frustum* was a claim about an eye that stood off the
//! whole extent, and a person's eye stands inside the world it draws, so the two claims that matter now are
//! **the frame is drawn from the eye the person stands at** and **the rectangles the eye does not see are
//! counted** (see [`Drawing::bars`]).
//!
//! **And the drawn reading's own owed list, named where it is built**: the drawn reading and the plan reading
//! are readings of **two worlds** — the client declares its own patch (`2 × 2` chunks, `64 × 64` columns,
//! [`CLIENT_CHUNKS`]') so that a stitched region fits in it whole, while the plan reading keeps the reference
//! world's `48 × 48` — so the two extents are printed with every frame rather than left to be noticed, and
//! **one world for both readings** is the owed repair (2026-10-13, §42: before it, the drawn extent was one
//! chunk of `32 × 32` and the brief's stitched region could not be drawn at all, because a `48`-column world
//! holds one and a half chunks); **a region's own residency** (the resident set's key *is* the state, and the
//! law's state moves every tick, so no resident mesh can serve a second frame — the measured rate is what
//! that costs, and a residency keyed on the geometry rather than on the state is owed); **no back-face culling** (the depth buffer decides, so *quads submitted* stays the mesh's own
//! number, and a culled pass is a later optimisation with a figure owed); **no textures** (a material's own
//! reflectance is the colour, and the 256× addressed detail lattice is `docs/APPEARANCE.md`'s own owed
//! slice); and **one light** (the solver's own Sun — no sky term, no reflections, and no second bounce).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::automaton::{Automaton, SETTLE_TICKS};
use crate::being::{self, WorldRead};
use crate::body::{self, Body, Traversal};
use crate::chunk::{surface_fields, CubicWorld};
use crate::coordinates::{ChunkKey, CHUNK_SIDE};
use crate::digest::Digest;
use crate::drawn::{self, Drawn};
use crate::light;
use crate::material::{MaterialId, MaterialRegistry};
use crate::mesh::{Budget, Effort, Ladder, Level, Mesh, Mesher, ResidencyHorizon, WorkLedger};
use crate::presentation::{self, Light, Rgb, View};
use crate::process;
use crate::refusal::{Refusal, RefusalKind, RefusalSet};
use crate::reference;
use crate::stage::{Declared, ProcessRequest};
use crate::stream::{StreamStep, Streamer};
use crate::viewpoint::{Viewpoint, Wish};
use crate::world::Tick;

/// **The declared pixels per column.** A patch column becomes a `ZOOM × ZOOM` block, so the client's
/// picture is the world's own resolution times a declared magnification — 48 columns × 12 = **576 × 576**
/// pixel(s). The number is the client's, printed by the run, and never a size the world has to know.
pub const ZOOM: u32 = 12;

/// **The declared frame rate.** The loop's own clock: one frame per tick, thirty times a second, so two
/// hundred frames of the plane's own law take about seven seconds for a person to watch.
pub const FRAMES_PER_SECOND: u32 = 30;

/// **The declared frame count** — *one frame is one tick*, so the client presents the plane exactly to the
/// horizon the run's own lines settle at ([`SETTLE_TICKS`]). The count is that constant rather than a
/// second number, so the picture and the measurement cannot drift apart.
pub const DECLARED_FRAMES: u32 = SETTLE_TICKS as u32;

/// **The declared Sun altitude**, in degrees above the horizon — the client's own viewing condition, read
/// into U5's solver ([`crate::light`]), which is what makes the light the atmospheric state's own rather
/// than a constant. The Moon stands below the horizon in this state and no droplets hang in the air.
pub const SUN_ALTITUDE_DEG: f64 = 35.0;

/// **The declared view altitude**, in degrees above the patch's own surface — the direction *toward* the
/// viewer. A high, slightly tilted view reads the whole patch and still shows the shading response.
pub const VIEW_ALTITUDE_DEG: f64 = 15.0;

/// **The declared relief exaggeration** — the multiplier the client applies to the terrain's own gradient
/// when it reads a column's normal. The world is unchanged by it; it exists because this patch's own relief
/// is about a metre across forty-eight (the reference read is a wetland), and the FMA bar's own point is
/// that *a player sees shading response on every visible surface*. The number is printed, so the picture's
/// own reading of the terrain is a declared one.
pub const RELIEF_EXAGGERATION: f64 = 8.0;

/// **The declared exposure** — the linear gain the frame's channels are given before the transfer function
/// below. One is no gain at all, which is the honest default: the contract's terms compose in linear light
/// and this client adds nothing to them on the way to the screen.
pub const EXPOSURE: f64 = 1.0;

/// **The declared surface parameters** the material record does not carry. OpenPBR's reduced subset here is
/// `base_color`, `specular_roughness` and `base_metalness` (U1's own spec: parameters the model does not
/// record are **owed and named absent**, never defaulted) — so these two are the **client's** declared
/// values for every column, printed with the frame, and a material that ever carries its own roughness will
/// be read instead of them.
pub const ROUGHNESS: f64 = 0.55;
/// The client draws every column as a dielectric: no column's material record carries a metalness, so a
/// metal here would be invented.
pub const METALNESS: f64 = 0.0;

/// **The drawn reading's own residency horizon** — the one declared "near the camera" the drawn path takes:
/// the chunks the camera's own residency **holds** ([`crate::stream`]), and the mesher's own limit on the
/// *derivation* of geometry. One number for both, so streaming and meshing cannot disagree about what near
/// means. It is the reference gate's own horizon (§ the record's own four horizons, whose order is held by
/// `U4 horizons (four types)`), and it is **far wider than this world**: the client's `2 × 2` chunks span
/// `45.2548` m between the two furthest chunk centres, so no camera anywhere in it can make a chunk leave the
/// horizon — which is why the residency's own eviction is measured at the world's own scale in
/// `U4 stream a walk at 45 m` and why *a world wider than its own horizon* is owed (2026-10-14e, named in
/// [`crate::controller`]'s header).
pub const DRAWN_RESIDENCY_M: f64 = 256.0;

/// **The client's own world, in chunks a side.** The drawn reading draws the brief's own subject: the
/// **stitched region** — one footprint over a rectangle of chunks, meshed in one sweep and merged once
/// ([`Mesher::derive_region`]). A region of `n` chunks needs a world that holds `n` chunks **whole**, and the
/// reference world's own patch is `48` columns — **one and a half chunks** of `32` — so the client declares
/// a patch of its own.
///
/// **Two chunks a side, and that is the smallest world that holds a region whole (2026-10-14e).** A driven
/// camera moves the region's own origin ([`crate::viewpoint::Viewpoint::region_origin`]) and the declared
/// [`DRAWN_RESIDENCY_M`] keeps whatever rectangle the eye stands at the corner of — but this world is `4`
/// chunk(s) of a `45.2548` m diagonal, so *every* chunk of it is inside the declared horizon from anywhere a
/// person can stand. Widening the world so that a walk can push a chunk out of the horizon would move the
/// 3 673 cell(s), the 1333 rectangle(s) and every recorded digest with them, so the camera's own effect on
/// the residency is measured **at the world's own scale** instead (the 45 m horizon `U4 stream` walks at),
/// and *a world wider than its own horizon* is owed — named in `U4 owed after this seam` rather than assumed
/// away here. The plan reading keeps the reference world's `48 × 48` ([`subject`]); the drawn reading's world
/// is this one, and both extents are printed with every frame rather than left to be noticed.
pub const CLIENT_CHUNKS: u32 = 2;

/// **The client's own patch side**, in columns — whole chunks, so the region below is a rectangle of the
/// world's own cells and not a rectangle over its edge.
pub const CLIENT_SIDE: u32 = CHUNK_SIDE as u32 * CLIENT_CHUNKS;

/// **The drawn reading's own subject**: the client's own world through the mesher's **region** door, over
/// [`DRAWN_CHUNKS`] chunks a side — `2 × 2` chunks of `32`, `64 × 64` columns, **one footprint, one sweep,
/// one merge, one mesh**. The chunks' own shared boundaries (at `x = 32` and `y = 32`) are **interior** to
/// the footprint, which is what *stitched* means here: the seam belongs to the region's own declaration and
/// not to any one chunk's mesh, so a frame's rectangle count is the region door's own number and never a sum
/// of per-chunk numbers.
pub const DRAWN_CHUNKS: u32 = CLIENT_CHUNKS;

/// **The region the drawn reading stitches when nobody has driven the camera** — the plan corner of the
/// rectangle of chunks, at the region's own declared base level ([`crate::viewpoint::REGION_BASE_Z`]).
///
/// The same key the drawn reading has always declared: this client's first chunk's own plan rectangle, which
/// is the rectangle the recorded frames are pictures of. A person who drives moves the region
/// ([`crate::viewpoint::Viewpoint::region_origin`]) and a person who comes home comes back to this one.
pub const DRAWN_CHUNK: ChunkKey = ChunkKey::new(0, 0, crate::viewpoint::REGION_BASE_Z);

/// **The read-back's own declared format.** Every presented frame is read back out of the window through
/// `GetDIBits` into a DIB this client declares itself: **32-bit, `BI_RGB`, top-down** — the same declared
/// layout the frame is handed to the window in, so the comparison is a **byte** comparison rather than a
/// guess about which channel the platform's colour value puts where. One frame, all of its pixels, every
/// pixel's own bytes.
pub const READBACK_BITS: u16 = 32;

/// **The window's own title** — what a person watching the screen reads.
pub const TITLE: &str = "shploingus clean room — U1's visible client (the cubic world's own cells)";

/// **The client's own pixels.** One buffer in the byte order the platform's bitmaps use (BGRA, top-down,
/// four bytes to a pixel — the bytes are handed to the platform **unchanged**), the shaded samples the
/// pixels were encoded from, and nothing else: no camera, no scene graph, no second surface.
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    samples: Vec<Rgb>,
}

impl Surface {
    /// **The one render path** — a world's own columns to this surface's pixels.
    ///
    /// Every column is shaded through [`presentation::shade`] over the appearance of the material **the
    /// column's own cell holds** ([`CubicWorld::material_at`], asked through the cells themselves),
    /// and the column's normal is read from the world's own terrain: a level upward normal stands where the
    /// world's own law stands water (a water surface has no slope of its own), and the terrain's own gradient
    /// elsewhere.
    ///
    /// **No class ladder is in this path (2026-10-14f).** Until this pass the frame derived each column's
    /// material a *second* time from a per-column `lithology` class array and the palette — while the cells
    /// the same world was built from already held that material. Two owners of *which material is here* is
    /// the shape the plan's own shape ruling refused (the column read is what reached a shader and produced
    /// the material flicker in the parent's WGSL path), so this reading now asks the cells and the class plane
    /// is nowhere in the appearance path. The only material the cells cannot answer for is the water's, which
    /// is a palette fact rather than a cell's — it is passed in as the one declared id it is.
    ///
    /// Nothing here invents a colour, a shape or a light: a material that derives no colour refuses the frame
    /// by name, and a read that is **not** the cells it was handed is refused rather than shaded — the patch
    /// must be the same and the water must stand in the same columns, because a frame drawn from one world's
    /// surface over another world's ground is a picture of a world that does not exist.
    pub fn render(
        world: &WorldRead,
        cells: &CubicWorld,
        water: MaterialId,
        registry: &MaterialRegistry,
        light: &Light,
        view: &View,
        zoom: u32,
    ) -> Result<Surface, Refusal> {
        if zoom == 0 {
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                "a zoom of 0 pixel(s) per column names no pixel to draw into",
            ));
        }
        let columns = world.cells();
        if cells.patch() != world.patch {
            let origin = cells.patch().origin();
            let world_origin = world.patch.origin();
            return Err(Refusal::new(
                RefusalKind::ParameterOutOfRange,
                format!(
                    "the cells cover {} column(s) at ({}, {}) and the read covers {} at ({}, {}): a frame \
                     is a picture of one world, and its material comes from the cells it was read from",
                    cells.patch().side(),
                    origin.x,
                    origin.y,
                    world.patch.side(),
                    world_origin.x,
                    world_origin.y
                ),
            ));
        }
        let side = world.patch.side() as usize;
        let width = world.patch.side() * zoom;
        let height = width;
        let mut pixels = vec![0_u8; (width * height * 4) as usize];
        let mut samples = Vec::with_capacity(columns);
        let (width_u, zoom_u) = (width as usize, zoom as usize);

        for cell in 0..columns {
            let Some(at) = world.patch.position(cell) else {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!("the read names column {cell}, which is not one of its own patch"),
                ));
            };
            // **The column's own cell reading** — the surface cell's material, and the water the cells above
            // it stand. One read per column, and it is the read the frame's water question is checked
            // against: the cells and the read are held together rather than trusted to agree.
            let column = cells.column(at)?;
            let wet = world.water_stands(cell);
            if wet != column.water_stands() {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!(
                        "the read and the cells disagree about the water at column {cell}: the read {} water \
                         where the cells hold {} µm of it — this frame would be a picture of one world's \
                         surface over another world's ground",
                        if wet { "stands" } else { "stands no" },
                        column.water_um
                    ),
                ));
            }
            let material_id = if wet {
                water
            } else {
                column.material.ok_or_else(|| {
                    Refusal::new(
                        RefusalKind::NoReflectance,
                        format!(
                            "column {cell} holds no solid cell, so it has no material: a colour cannot be \
                             derived from a column that is not made of anything"
                        ),
                    )
                })?
            };
            let material = registry.material(material_id)?;
            let surface = presentation::surface_of(material, ROUGHNESS, METALNESS).map_err(|kind| {
                Refusal::new(
                    RefusalKind::NoReflectance,
                    format!(
                        "material {} ({}), drawn at column {cell}, carries no reflectance: a colour \
                         cannot be derived from it (`{}`)",
                        material_id.0,
                        material.name,
                        kind.name()
                    ),
                )
            })?;
            let normal = normal_of(world, cell, wet);
            let shaded = presentation::shade(&surface, normal, light, view)
                .map_err(|kind| shade_refusal(kind, cell))?;
            samples.push(shaded);
            let encoded = presentation::encode(shaded, EXPOSURE);
            let (x, y) = (cell % side, cell / side);
            for row in 0..zoom_u {
                let start = ((y * zoom_u + row) * width_u + x * zoom_u) * 4;
                for column in 0..zoom_u {
                    let at = start + column * 4;
                    pixels[at..at + 4].copy_from_slice(&encoded);
                }
            }
        }
        Ok(Surface { width, height, pixels, samples })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The pixels **as the platform is handed them** — BGRA, top-down, `width * height * 4` bytes.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The shaded samples behind the pixels, one per column, in the patch's own order — the same
    /// [`presentation::shade`] results the frame was encoded from, so the FMA bar's instrument
    /// ([`presentation::band_count`]) counts bands on the picture the screen shows.
    pub fn samples(&self) -> &[Rgb] {
        &self.samples
    }

    /// **The FMA bar, read on this frame's own samples** — the count [`presentation::band_count`] makes.
    pub fn bands(&self) -> usize {
        presentation::band_count(&self.samples)
    }

    /// One pixel, in the buffer's own order (`[b, g, r, a]`), or nothing where the point is off the frame.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = ((y * self.width + x) * 4) as usize;
        let mut pixel = [0_u8; 4];
        pixel.copy_from_slice(&self.pixels[at..at + 4]);
        Some(pixel)
    }

    /// The frame as one digest: its size and every pixel byte, in the buffer's own order. Two frames of one
    /// world and one declared view are one digest (P3), and the run prints it.
    pub fn digest(&self) -> Digest {
        let mut bytes = Vec::with_capacity(self.pixels.len() + 8);
        bytes.extend_from_slice(&self.width.to_le_bytes());
        bytes.extend_from_slice(&self.height.to_le_bytes());
        bytes.extend_from_slice(&self.pixels);
        Digest::of(&bytes)
    }

    /// The frame's own pixels as a **P6 portable pixmap**, in RGB order — the screenshot the bar asks for,
    /// written from the bytes the window was handed rather than from a second drawing of the world. The
    /// writer itself is [`crate::drawn::write_ppm`], so both readings write one format one way.
    pub fn write_ppm(&self, path: &Path) -> Result<(), Refusal> {
        drawn::write_ppm(path, self.width, self.height, &self.pixels)
    }

    /// The frame's own darkest and brightest channel byte — two measured numbers that say the picture is
    /// not a flat fill, printed with the frame.
    pub fn range(&self) -> (u8, u8) {
        let mut dark = u8::MAX;
        let mut bright = 0_u8;
        for pixel in self.pixels.as_chunks::<4>().0 {
            for channel in &pixel[0..3] {
                dark = dark.min(*channel);
                bright = bright.max(*channel);
            }
        }
        (dark, bright)
    }
}

/// **What one drawn frame measured** — the frame's own figures, with no window in them, so the evidence run
/// and the windowed run report the same facts about the same world.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    pub width: u32,
    pub height: u32,
    /// The world's own columns, one shaded sample each.
    pub columns: usize,
    pub bands: usize,
    pub frame: Digest,
    /// The world the frame was drawn from, **read as a surface** ([`being::surface_digest`]) — a reading of
    /// the world's own cells, which have their own identity ([`crate::chunk::CubicWorld::digest`]) since the
    /// shape ruling (§37, `LEDGER.md` C-189). It is the same digest the run's own U12 lines stand on when the
    /// plane is at rest.
    pub world: Digest,
    /// The plane's own state digest at that frame ([`Automaton::digest`]) — the world's own identity, since
    /// the law's state *is* the cubic world (§38).
    pub state: Digest,
    pub ticks: u64,
    pub standing: usize,
    pub deepest: Option<(usize, i64)>,
    /// The ladder's declared class → material mapping, and the material water is drawn with.
    pub materials: Vec<(usize, u16)>,
    pub water: u16,
    pub sun_altitude_deg: f64,
    pub sun_tint: [f64; 3],
    pub dark: u8,
    pub bright: u8,
}

/// **The counts a run of drawn frames accumulated** — summed over every frame the run drew, so the run's
/// report describes the whole run while the frame's own counters describe its last frame. Nothing here is
/// derived from a declaration: every number is read off a frame that exists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawnCounts {
    pub frames: u64,
    pub quads: u64,
    pub vertices: u64,
    pub triangles: u64,
    pub indices: u64,
    pub clipped: u64,
    pub degenerate: u64,
    pub shaded: u64,
}

impl DrawnCounts {
    /// Add one drawn frame's own counters to the run's.
    pub fn add(&mut self, frame: &Drawn) {
        self.frames += 1;
        self.quads += frame.quads();
        self.vertices += frame.vertices();
        self.triangles += frame.triangles();
        self.indices += frame.indices();
        self.clipped += frame.clipped();
        self.degenerate += frame.degenerate();
        self.shaded += frame.shaded();
    }
}

/// **What a run of drawn frames measured** — the drawn reading's own report, with the world's own facts
/// beside it: the counts summed over every frame drawn, the **last** frame itself (its pixels, its depth,
/// its camera and its digest), the state digest that frame's mesh came from, and the plane's own state at
/// that tick. The window's own report and the evidence run both print this, so neither can describe a
/// picture the other did not draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    pub counts: DrawnCounts,
    pub last: Drawn,
    /// The world the frame was drawn from, read as a surface ([`being::surface_digest`]).
    pub world: Digest,
    /// The plane's own state digest at that frame — the same digest the frame's mesh is keyed by.
    pub state: Digest,
    pub ticks: u64,
    pub standing: usize,
    pub deepest: Option<(usize, i64)>,
    pub materials: Vec<(usize, u16)>,
    pub water: u16,
    pub sun_altitude_deg: f64,
    pub sun_tint: [f64; 3],
    /// The columns the **world** holds — the plan reading's own extent — and the columns the **drawn region**
    /// covers. Two extents, both declared, printed together because they are not the same number.
    pub world_side: u32,
    pub region_side: u32,
    /// **Where the person's own eye stood for this frame** (2026-10-14e), the chunk the residency was observed
    /// at, and the plan corner of the rectangle of chunks the frame is a picture of. The eye is the drawn
    /// camera's own by construction ([`draw_from`]) and the second bar says so; the anchor and the region are
    /// read off the same viewpoint, so a printed frame names its own place rather than a declared one.
    pub viewpoint: Viewpoint,
    pub anchor: ChunkKey,
    pub region: ChunkKey,
    /// **The headless drawn frame of this run's own last state** — the same state, the same call, no window —
    /// as a *picture* digest. The drawn reading's fifth bar is the two pictures being one picture
    /// ([`Drawing::bars`]); where a run *is* the headless frame ([`drawn_frame`]) the number is that frame's
    /// own, and it is the windowed runs where the two can differ.
    pub headless: Digest,
    /// **The residency the last frame was drawn from**: how many chunks it held, how many cells those are, and
    /// its own identity — which is the `mesh state digest` above, because the mesh is keyed by the identity of
    /// the cells it was handed. Printed together so a frame says *which cells it is a picture of* with numbers
    /// rather than with a promise.
    pub resident: usize,
    pub resident_cells: usize,
    pub residency: Digest,
}

impl Drawing {
    /// The last frame's own luminance bands, read on the shading the picture was made of.
    pub fn bands(&self) -> usize {
        self.last.bands()
    }

    /// The last frame's own darkest and brightest channel byte.
    pub fn range(&self) -> (u8, u8) {
        self.last.range()
    }

    /// **The drawn reading's own six bars**, named, in one place: the drawing loop's counts against the
    /// mesh's own declaration, **the eye the frame was drawn from** against the person's own (`2026-10-14e`),
    /// **the rectangles the eye does not see** against the triangles it clipped (`clipped == 2 × unseen`), the
    /// picture against the FMA bar, the frame's own mesh state against the residency it was drawn from, and the
    /// **pictures**: this run's last frame against the headless drawn frame of this run's own state
    /// (2026-10-13, §42). The last is a *picture* comparison, so a window whose last frame is not the picture a
    /// windowless draw of the same state produces **refuses by name** rather than printing a sentence that can
    /// read false while the run exits 0.
    ///
    /// **The second and third moved with the camera.** Until 2026-10-14e the second was *every rectangle is
    /// inside the camera's own frustum* (`clipped == 0`), which was true of an eye that stood off the whole
    /// extent along the client's one declared direction — and false the moment a person stands **inside** the
    /// world they are drawing, where the rectangles behind the eye are a state and not a defect. The claim that
    /// matters about a driven frame is *which eye drew it* (the person's own, bit for bit: the look is stored
    /// on the camera rather than re-derived from a target, [`crate::drawn::Camera::look`]), and the third keeps
    /// the accounting **total**: one unseen rectangle costs exactly its own two triangles, so no rectangle can
    /// be dropped between the eye and the picture without a count going wrong.
    pub fn bars(&self) -> [(&'static str, bool); 6] {
        [
            (
                "the loop's own counts are the mesh's own",
                self.last.counts_agree(),
            ),
            (
                "the frame is drawn from the eye the person stands at",
                self.last.camera().eye == self.viewpoint.eye()
                    && self.last.camera().look == self.viewpoint.look(),
            ),
            (
                "the rectangles the eye does not see are counted: nothing is dropped between the eye and the \
                 picture",
                self.last.clipped() == 2 * self.last.unseen(),
            ),
            (
                "the picture is not a flat fill",
                self.last.shaded() > 0
                    && self.last.bands() >= presentation::BAR_MINIMUM_BANDS
                    && self.range().0 < self.range().1,
            ),
            (
                "the frame's own mesh state is the cells the residency holds",
                self.last.mesh_state() == self.residency,
            ),
            (
                "the headless drawn frame of this run's own state is this run's last frame",
                self.last.digest() == self.headless,
            ),
        ]
    }

    /// Whether every one of those bars holds.
    pub fn bars_hold(&self) -> bool {
        self.bars().iter().all(|(_, holds)| *holds)
    }

    /// The bars as one printed clause, each named beside its own answer.
    pub fn bars_described(&self) -> String {
        self.bars()
            .iter()
            .map(|(name, holds)| format!("{name} = {holds}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// **The one description of a drawn run** — the counts, the last frame's own coverage, depth extents and
    /// bands, the digests, the two extents and the camera, from one place, so the window's report and the
    /// evidence run cannot describe two different pictures.
    pub fn described(&self) -> String {
        let depth = match self.last.depth_extent() {
            Some((near, far)) => format!("depth {near:.2}..{far:.2} m"),
            None => "no pixel drawn".to_string(),
        };
        let (dark, bright) = self.range();
        format!(
            "the drawn reading: {} frame(s) of the world's own geometry — quads submitted {}, vertices {}, \
             triangles {} (indices {}), clipped {}, degenerate {} — the last frame drew {} of {} pixel(s) \
             ({:.1}%), {depth}, {} luminance band(s), channels 0x{dark:02x}..0x{bright:02x}; frame digest {}, \
             mesh digest {}, mesh state digest {}; **the camera's own residency** held {} chunk(s) / {} cell(s) \
             of this state, its identity {}, and the mesh's state is the residency's own = {}; the region is \
             {} × {} column(s) of the world's own {} × {}; camera {}",
            self.counts.frames,
            self.counts.quads,
            self.counts.vertices,
            self.counts.triangles,
            self.counts.indices,
            self.counts.clipped,
            self.counts.degenerate,
            self.last.shaded(),
            self.last.pixels_count(),
            self.last.coverage() * 100.0,
            self.last.bands(),
            self.last.digest().to_hex(),
            self.last.mesh().to_hex(),
            self.last.mesh_state().to_hex(),
            self.resident,
            self.resident_cells,
            self.residency.to_hex(),
            self.last.mesh_state() == self.residency,
            self.region_side,
            self.region_side,
            self.world_side,
            self.world_side,
            self.last.camera().described()
        )
    }
}

/// **What a windowed run measured** — the drawn run's own figures plus what only the window can answer: how
/// many frames it presented, how many ticks that was, and how many of the window's own sampled pixels agreed
/// with the frame's own bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Presented {
    pub drawing: Drawing,
    /// The declared count the run was asked for, and how many it actually presented (a person who closes
    /// the window early stops the loop, which is the one way the two can differ).
    pub frames: u32,
    pub presented: u32,
    /// **The run's own wall-clock seconds and the rate it actually presented at** — measured between the
    /// first frame's own tick and the last frame's, by the platform's own monotonic clock. The declared rate
    /// is the loop's **pacing target**; this number is what the picture kept, and both are printed.
    pub seconds: f64,
    pub rate: f64,
    /// Whether the **person** closed the window while the loop was running: the one honest way the frames
    /// presented can be fewer than the frames declared.
    pub closed: bool,
    pub settled: bool,
    /// Every pixel of the window's own client area, read back through the declared format, and how many of
    /// them are the frame's own bytes. The pair is the client's whole claim about the screen.
    pub pixels: usize,
    pub agreeing: usize,
    pub dpi_aware: bool,
    pub screenshot: Option<PathBuf>,
    /// **The residency's own work over the run, counted**: how many held chunks were released because the
    /// law's own state moved between frames (`stale`) and how many were derived through the keyed door
    /// (`derived`). A still state serves its chunks; a stepping one cannot, and these are the measured numbers
    /// of that cost rather than a sentence about it.
    pub stale: u64,
    pub derived: u64,
    /// **What the person's own input did over the run** (2026-10-14e): how many frames carried a held key,
    /// how far the eye travelled in metres and how far its look turned in degrees. A run nobody touched has
    /// zero of each, and the window's own report prints them rather than leaving *was anything driven* to be
    /// assumed from the picture.
    pub driven_frames: u32,
    pub travelled_m: f64,
    pub turned_deg: f64,
    /// **The person the window opened as** (2026-10-14g) — the body's own first state
    /// ([`crate::body::Body::described`]), so the report says which body the keys below were driving, and
    /// whether it was walking on the ground rather than at the instrument's own altitude.
    pub opened_as: String,
    /// **What the body's own steps did over the run** — summed.
    pub body: BodyTotals,
}

/// **What a body's own steps did over a window run** — summed, so a report says what a person's hands did
/// with the body rather than only that the picture changed. A run nobody touched is a run of zeros, and the
/// line prints them rather than leaving a reader to wonder whether the walk happened and was lost.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyTotals {
    /// How far the feet travelled in plan, in metres.
    pub plan_m: f64,
    /// How far the body rose (a jump's own rise, or a flight's), in metres.
    pub rose_m: f64,
    /// How far gravity brought it down, in metres.
    pub fell_m: f64,
    /// How far a blocked horizontal move was raised by the step-up retry, in metres — **zero on this world**,
    /// where voxels are one metre and `STEP_UP` is `0.55` (§95.5), and measured rather than assumed.
    pub climbed_m: f64,
    /// How many jumps were taken.
    pub jumps: u64,
    /// How many frames a horizontal move was refused outright — the frames the world itself stopped the body.
    pub blocked_frames: u64,
    /// Whether the feet were on the floor at the end.
    pub grounded_end: bool,
    /// How the body was traversing at the end.
    pub traversal_end: Traversal,
}

/// **One frame drawn from the world the tree already builds**, with no window: the being's own read, the
/// plane's own law stepped to the declared rest ([`SETTLE_TICKS`]), and the frame that state draws. This is
/// the evidence run's own subject, and the window's last frame is the same world by construction.
pub fn measured_frame() -> Result<Measured, Refusal> {
    let (cubic, ladder) = subject()?;
    let registry = reference::registry();
    let light = light();
    let view = view();
    let mut plane = Automaton::over(&cubic);
    plane.steps(SETTLE_TICKS)?;
    let read = read_of(&plane)?;
    // **The cells the read was taken from** — the read's own world, not the world the plane was built on:
    // the frame's materials come from the cells it is drawing, and the pass's own guard is that those cells
    // and that read stand the same water in the same columns.
    let surface = Surface::render(&read, plane.world(), ladder.water(), registry, &light, &view, ZOOM)?;
    Ok(measure(&read, &plane, &surface, &ladder))
}

/// **The world's own reading of the law's state** — `over_cubic` of the world the law is stepping (§38):
/// the one reading every consumer shares, and the only way to read the stepped plane.
fn read_of(plane: &Automaton) -> Result<WorldRead, Refusal> {
    being::WorldRead::over_cubic(
        being::Organisation::first(),
        "the first mob",
        plane.world(),
    )
}

/// **The drawn reading's own subject: the cells the camera's own residency holds** — the region's own
/// `CLIENT_CHUNKS × CLIENT_CHUNKS` chunks (`96 × 96` columns of a `3 × 3` world, of which the footprint is the
/// `2 × 2` rectangle `origin` names) through the mesher's **region** door: one footprint over
/// [`residency`](Streamer::world), one sweep, one merge, one mesh ([`Mesher::derive_region`]). The
/// mesh is named by the chunk it starts at **and by the identity of the cells it was drawn from**, so a
/// frame's mesh state digest is the **residency's own** digest: the number moves exactly when a held cell's
/// water does, and geometry from cells the residency does not hold is not reachable through it — a residency
/// holding a part of the region is **refused by the region's own name**, carrying the count of chunks it does
/// hold, and one holding none is refused the same way (measured: `U4 stream the frame follows the residency`),
/// so a frame is never a picture of less than the region.
///
/// **The region's own origin is an argument since 2026-10-14e.** It used to be the declared constant
/// [`DRAWN_CHUNK`], because the region *was* the world and the camera could not move: a person's own place now
/// decides it ([`crate::viewpoint::Viewpoint::region_origin`]), and the camera stands at the region's own
/// corner column, so the rectangle a frame is a picture of follows the person. This is the one call that
/// decides *what* a drawn frame is a picture of, so the window's loop and the headless frame cannot draw two
/// different meshes.
pub fn region_mesh(residency: &Streamer, origin: ChunkKey) -> Result<Mesh, Refusal> {
    let roster = roster()?;
    let ladder = reference::ladder()?;
    // The mesher's own limit on the derivation of geometry, at the residency's own declared horizon: one
    // number for "near the camera", so streaming and meshing cannot disagree about it.
    let horizon = ResidencyHorizon::m(DRAWN_RESIDENCY_M)?;
    // Level 0 is the render ladder's own first rung; a refusal here is the ladder refusing itself.
    let level = Level::new(0)?;
    let mesher = Mesher::new(&roster, ladder, horizon);
    let mut ledger = WorkLedger::default();
    mesher.derive_region(
        residency.world(),
        origin,
        DRAWN_CHUNKS,
        level,
        Effort::new(Budget::unlimited(), 1),
        &mut ledger,
    )
}

/// **The person's own eye in the region the residency holds** — the client's own declared view
/// ([`view`], [`VIEW_ALTITUDE_DEG`], [`drawn::DRAWN_FOV_DEG`]) read as a viewpoint
/// ([`crate::viewpoint::Viewpoint::of`]).
///
/// This is *home*: the eye the drawn reading has always stood at, standing off the region's own extent along
/// the declared direction and looking back at its centre. It is derived **from the mesh** rather than declared
/// a second time, which is what makes *the recorded frame comes back byte for byte* a property of the code
/// path rather than of two numbers agreeing (`crate::viewpoint`'s guards hold it).
pub fn home_viewpoint(mesh: &Mesh) -> Result<Viewpoint, Refusal> {
    Ok(Viewpoint::of(&drawn::Camera::over(
        mesh,
        view().toward,
        drawn::DRAWN_FOV_DEG,
    )?))
}

/// **One frame of the client's own clock, in seconds** — the step of time every step of this client's own
/// motion is taken over ([`FRAMES_PER_SECOND`], the declared rate). One door, so a walking body's own speed in
/// metres a second ([`crate::body::WALK_M_PER_S`]) and a flying eye's own metres a frame
/// ([`crate::viewpoint::FLY_M_PER_FRAME`]) cannot disagree about how long a frame is.
pub fn frame_seconds() -> f64 {
    1.0 / f64::from(FRAMES_PER_SECOND)
}

/// **The world's own floor under a plan position** — the top of the highest solid cell under it, in metres,
/// found by [`crate::body::floor_under`] from the top of the world's own span downwards: the world's own
/// answer to *what is a person standing on here*, and the number `person_of` stands a body on.
pub fn ground_under(world: &CubicWorld, x: f64, y: f64) -> Result<f64, Refusal> {
    let cells = body::Cells::over(world);
    let top = f64::from(world.span_z().1);
    body::floor_under(&cells, [x, y], top).ok_or_else(|| {
        body::no_ground_refusal(&format!(
            "the column ({x:.2}, {y:.2}) m of the world the client shows, whose own span reaches {top:.0}"
        ))
    })
}

/// **How far from the declared view's own plan column the search for a place to stand reaches**, in metres —
/// a body is `0.6` m across and this world's voxels are one metre, so a place whose disc fits and whose floor is
/// level this far out is a plateau the session's own walk stays on.
const STANDING_REACH_M: f64 = 1.5;

/// **How many cells out the search for a place to stand goes** before it stands a body at the nearest column
/// whose disc fits, level or not — a bound, so a world of cliffs refuses rather than searching forever.
const STANDING_SEARCH_CELLS: i32 = 6;

/// **Whether a place is level ground a body can walk out of**: the body's own disc fits at the same floor
/// [`STANDING_REACH_M`] in each plan direction — which is what the session's own walk needs, and what a stand
/// on a one-cell ledge is not.
fn level_ground(cells: &body::Cells<'_>, feet: [f64; 3]) -> bool {
    let here = feet[2];
    [
        (STANDING_REACH_M, 0.0),
        (-STANDING_REACH_M, 0.0),
        (0.0, STANDING_REACH_M),
        (0.0, -STANDING_REACH_M),
    ]
    .into_iter()
    .all(|(dx, dy)| {
        let (x, y) = (feet[0] + dx, feet[1] + dy);
        body::floor_under(cells, [x, y], here + 0.5).is_some_and(|floor| (floor - here).abs() < 0.5)
            && !body::disc_hits(cells, [x, y, here])
    })
}

/// **The person the client opens as: a place to stand near the declared view's own plan column, walking**
/// (2026-10-14g).
///
/// The lens and the person share a **plan column** and differ in exactly the one way they must: the
/// instrument's own eye ([`home_viewpoint`]) stands at its own altitude and is a **flying** body (that eye is a
/// vantage, not a person's feet), while the person stands on the world's own floor and **walks**
/// ([`Body::standing`], [`Traversal::Walking`]) — so *the picture follows the body* is a walk away from where
/// the recorded frames are drawn, and the recorded frames stay the instrument's own view rather than becoming a
/// second client's.
///
/// **The column is searched, not assumed, and the reason is the world's own geometry.** The lens's own column
/// is where the *camera* stands, and a camera has no width: a body there has a `0.6` m disc, and if the next
/// column of terrain is a whole metre higher the body is inside it before it takes a step — every move refused,
/// which would look like a client that cannot move rather than a body placed badly. So the search takes the
/// nearest column to the lens's own (ring by ring, in the world's own plan) whose floor a body fits on, and
/// **prefers** one whose floor is level out to [`STANDING_REACH_M`] — the place a person would actually choose
/// to stand — falling back to the nearest fitting one when the terrain offers no level ground at all. Where it
/// chose is printed by [`crate::controller`], so *the person opens somewhere else* is a measurement rather than
/// something a reader has to infer.
///
/// The look is the lens's own look: a person opens looking at what the recorded frame is a picture of, and
/// turns from there.
pub fn person_of(mesh: &Mesh, world: &CubicWorld) -> Result<Viewpoint, Refusal> {
    let lens = home_viewpoint(mesh)?;
    let eye = lens.eye();
    let cells = body::Cells::over(world);
    let top = f64::from(world.span_z().1);
    let patch = world.patch();
    let origin = patch.origin();
    let side = patch.side() as i32;
    let (lens_x, lens_y) = (eye[0].floor() as i32, eye[1].floor() as i32);
    let mut fitting: Option<[f64; 3]> = None;
    let mut level: Option<[f64; 3]> = None;
    'rings: for ring in 0..=STANDING_SEARCH_CELLS {
        for dx in -ring..=ring {
            for dy in -ring..=ring {
                if dx.abs() != ring && dy.abs() != ring {
                    continue;
                }
                let (cell_x, cell_y) = (lens_x + dx, lens_y + dy);
                if !(origin.x..origin.x + side).contains(&cell_x)
                    || !(origin.y..origin.y + side).contains(&cell_y)
                {
                    continue;
                }
                let (x, y) = (f64::from(cell_x) + 0.5, f64::from(cell_y) + 0.5);
                let Some(floor) = body::floor_under(&cells, [x, y], top) else {
                    continue;
                };
                let feet = [x, y, floor];
                if body::disc_hits(&cells, feet) {
                    continue;
                }
                if fitting.is_none() {
                    fitting = Some(feet);
                }
                if level_ground(&cells, feet) {
                    level = Some(feet);
                    break 'rings;
                }
            }
        }
    }
    let feet = level.or(fitting).ok_or_else(|| {
        body::no_ground_refusal(&format!(
            "any column within {STANDING_SEARCH_CELLS} cell(s) of the declared view's own plan column \
             ({lens_x}, {lens_y}), which is where a person the client can open as would have to stand"
        ))
    })?;
    // **A person standing on the ground looks at the horizon, not down at the region's centre from above.**
    // The lens's own look is `75°` below the plan (an instrument looking *down* at the region it draws), and a
    // body on the ground that kept it would be looking at its own feet with the whole region behind its near
    // plane — which is exactly what the drawn frame measured before this line existed (0 of 331776 pixel(s)
    // shaded, every rectangle of the region cut by the near plane). So the person's look is the lens's own
    // **plan direction**, flattened to the horizon: the same way the instrument faces, with the feet on the
    // ground, which is what makes the frame a picture of the ground ahead rather than of nothing at all.
    let lens_look = lens.look();
    let look = if (lens_look[0] * lens_look[0] + lens_look[1] * lens_look[1]).sqrt() > 1e-9 {
        unit([lens_look[0], lens_look[1], 0.0])
    } else {
        // A lens looking straight down has no plan direction to face, so the person keeps its look: a client
        // that invented a heading here would be inventing one, and the recorded view's own direction is 15° off
        // the plan rather than under it.
        lens_look
    };
    Ok(Viewpoint::new(Body::standing(feet), look))
}

/// **The one call that draws the cells a camera's residency holds, from where a person stands** — the mesh
/// through [`region_mesh`], the camera the person's own state stands at ([`drawn::Camera::standing`]), and the
/// frame through [`crate::drawn::draw`] under the client's declared look. The window's own loop and the
/// shell's own drive both come through here, so the picture a person watches and the picture a run's own line
/// measures are drawn by **the same call** and not by two — and a frame cannot be drawn from cells no
/// residency holds, because this door takes the residency itself.
pub fn draw_from(
    residency: &Streamer,
    viewpoint: &Viewpoint,
    origin: ChunkKey,
) -> Result<Drawn, Refusal> {
    frame_of(&region_mesh(residency, origin)?, viewpoint)
}

/// **What one step of driving did** — the eye's own movement, the two keys the step decided (the chunk the
/// residency was observed at and the region the frame is a picture of) and what came of them: the residency's
/// own work and the frame drawn from the eye's new place.
#[derive(Clone, Debug, PartialEq)]
pub struct DrivenStep {
    /// What the wish did to the eye: how far it moved and how far its look turned.
    pub movement: crate::viewpoint::Step,
    /// **The chunk the eye stands over** — the anchor [`Streamer::observe`] was asked about, from
    /// [`Viewpoint::chunk`], so a person who crosses a chunk's own plan boundary pays a derivation and a
    /// person who does not pays nothing.
    pub anchor: ChunkKey,
    /// **The region the frame is a picture of** — the plan corner of the chunk rectangle, from
    /// [`Viewpoint::region_origin`].
    pub region: ChunkKey,
    /// What the residency had to do to answer: derived, served, evicted, stale.
    pub stream: StreamStep,
    /// The frame drawn from the eye's own place of that region.
    pub drawn: Drawn,
}

impl DrivenStep {
    /// Whether the step was **free**: the eye did not move, so no chunk was derived and none released.
    pub fn was_free(&self) -> bool {
        self.movement.was_still() && self.stream.was_free()
    }
}

/// **One step of a driven client, and the one door every input comes through** — a person's own [`Wish`] is
/// turned into the whole client's next state: the eye moves ([`Viewpoint::step`]), the two keys are read off
/// the eye's own new place, the residency is observed at that anchor through its own door, and the frame is
/// drawn from the eye's own place. **One door**, so the window's own message loop and the shell's own words
/// cannot advance the client two different ways, and neither can draw a frame the other would not.
///
/// **The law is not ticked here.** This is the camera's own step: it moves a person over the state the client
/// shows, which is what makes *the frame comes back byte for byte when the eye comes back* a property of the
/// code rather than of the state standing still. The window's own loop ticks the law **beside** this call,
/// one tick a frame, because a frame is a tick (§36 ruling 3).
pub fn step_driven(
    plane: &Automaton,
    residency: &mut Streamer,
    viewpoint: &mut Viewpoint,
    wish: &Wish,
    dt_s: f64,
) -> Result<DrivenStep, Refusal> {
    // **The world's own cells are what stops the body** (2026-10-14g): the person's own walk is blocked by
    // the same cells the frame is drawn from, so *the drawn world stops it* is one world's answer rather than
    // a collision model standing beside it.
    let movement = viewpoint.step(wish, dt_s, &body::Cells::over(plane.world()));
    let world = plane.world();
    let anchor = viewpoint.chunk(world)?;
    let region = viewpoint.region_origin(world, DRAWN_CHUNKS)?;
    let stream = residency.observe(world, anchor)?;
    let drawn = draw_from(residency, viewpoint, region)?;
    Ok(DrivenStep {
        movement,
        anchor,
        region,
        stream,
        drawn,
    })
}

/// **The declared view's own frame of the cells a residency holds** — [`draw_from`] at *home*: the mesh, the
/// viewpoint [`home_viewpoint`] reads off it, and the frame that eye draws. The viewpoint is returned beside
/// the frame, so a run prints the eye the picture was drawn from rather than naming it twice.
pub fn draw_plane(residency: &Streamer, origin: ChunkKey) -> Result<(Drawn, Viewpoint), Refusal> {
    let mesh = region_mesh(residency, origin)?;
    let viewpoint = home_viewpoint(&mesh)?;
    Ok((frame_of(&mesh, &viewpoint)?, viewpoint))
}

/// **One frame of one mesh from one eye** — the camera, the client's declared look and the one draw call, in
/// one place, so a driven eye and the declared view differ in nothing but the eye.
fn frame_of(mesh: &Mesh, viewpoint: &Viewpoint) -> Result<Drawn, Refusal> {
    let camera = drawn::Camera::standing(
        mesh,
        viewpoint.eye(),
        viewpoint.look(),
        drawn::DRAWN_FOV_DEG,
    )?;
    drawn::draw(
        mesh,
        reference::registry(),
        &light(),
        &camera,
        drawn::DRAWN_SIZE,
        drawn::Look {
            roughness: ROUGHNESS,
            metalness: METALNESS,
            exposure: EXPOSURE,
        },
        presentation::encode,
    )
}

/// **The world the client shows** — the client's own world ([`client_world`]) after the plane's own law has
/// run to the declared rest ([`SETTLE_TICKS`]): the state a drawn frame is a picture of and the state the
/// camera's own residency streams. One door, so the evidence run's own stream lines, the drawn frame and the
/// window's last frame cannot measure two different worlds — and the law's own work is inside it, which is
/// the `3 673` of `131 072` cell(s) the declared surface and the shown state differ by.
pub fn client_state() -> Result<Automaton, Refusal> {
    let cubic = client_world()?;
    let mut plane = Automaton::over(&cubic);
    plane.steps(SETTLE_TICKS)?;
    Ok(plane)
}

/// **The anchor the client's own declared view is observed at** — the chunk the drawn reading's camera has
/// always stood over.
///
/// The region [`DRAWN_CHUNK`] names is a rectangle of [`DRAWN_CHUNKS`] chunks a side, and the declared view
/// stands at the **column where those chunks meet**: one chunk in from the rectangle's own plan origin in both
/// axes, which is `CLIENT_CHUNKS - 1` chunks in. [`crate::viewpoint::Viewpoint::region_origin`] is the same
/// relation read the other way (the region is one chunk back from the camera's own chunk), and the world's own
/// key law answers which of its chunks is there ([`crate::viewpoint::chunk_over_plan`]) — one rule, so the
/// declared view and a person driving it cannot name two different chunks. A guard in `crate::viewpoint`
/// holds the two directions together.
pub fn home_anchor(world: &CubicWorld) -> Result<ChunkKey, Refusal> {
    let corner = DRAWN_CHUNK.x + DRAWN_CHUNKS as i32 - 1;
    let corner_y = DRAWN_CHUNK.y + DRAWN_CHUNKS as i32 - 1;
    crate::viewpoint::chunk_over_plan(world, corner, corner_y)
}

/// **A residency of one state, observed at one anchor** — the client's declared horizon
/// ([`DRAWN_RESIDENCY_M`]) around the chunk a camera stands in, through the one door
/// ([`Streamer::observe`]). A person's own anchor comes from
/// [`crate::viewpoint::Viewpoint::chunk`], so a driven run and the declared view ask the same door.
pub fn residency_at(world: &CubicWorld, anchor: ChunkKey) -> Result<Streamer, Refusal> {
    let mut residency = Streamer::over(world, ResidencyHorizon::m(DRAWN_RESIDENCY_M)?);
    residency.observe(world, anchor)?;
    Ok(residency)
}

/// **The camera's own residency of one state, at the declared view's own anchor** — what the headless frame,
/// the guards and the shell's own drive all come through, so none of them declares a second residency.
pub fn residency_of(world: &CubicWorld) -> Result<Streamer, Refusal> {
    residency_at(world, home_anchor(world)?)
}

/// **One drawn frame of the world the tree already builds**, with no window: the being's own read, the plane's
/// own law stepped to the declared rest ([`SETTLE_TICKS`]), **the camera's own residency of that state**, and
/// the drawn reading's frame of what it holds — from the **declared view** ([`draw_plane`]), because this is
/// the frame the recorded baselines are pictures of. This is the evidence run's own drawn subject, and **the
/// window's first frame is this frame by construction** — the same world at the same tick, the same residency
/// door, the same camera and the same one draw call.
pub fn drawn_frame() -> Result<Drawing, Refusal> {
    let plane = client_state()?;
    let residency = residency_of(plane.world())?;
    let (frame, viewpoint) = draw_plane(&residency, DRAWN_CHUNK)?;
    let mut counts = DrawnCounts::default();
    counts.add(&frame);
    // **This run *is* the headless frame**, so the fifth bar is that frame against itself: it is the
    // windowed runs where the two pictures can differ, and the bar is the same bar there.
    let headless = frame.digest();
    drawing_of(&plane, &residency, &viewpoint, counts, frame, headless)
}

/// **A run's own report, from its last frame, the residency it was drawn from and the run's own counts** —
/// the world's facts read once, at the tick the last frame was drawn at, so the window's report and the
/// headless frame describe the same kinds of numbers from the same sources. The **viewpoint**, the **anchor**
/// and the **region** are the run's own (2026-10-14e): which eye drew the picture, which chunk the residency
/// was observed at and which rectangle of chunks the frame is a picture of — all three read off the one
/// viewpoint, so a printed frame says where its own person stood.
fn drawing_of(
    plane: &Automaton,
    residency: &Streamer,
    viewpoint: &Viewpoint,
    counts: DrawnCounts,
    last: Drawn,
    headless: Digest,
) -> Result<Drawing, Refusal> {
    let read = read_of(plane)?;
    let ladder = reference::ladder()?;
    let world = plane.world();
    Ok(Drawing {
        counts,
        last,
        headless,
        viewpoint: *viewpoint,
        anchor: viewpoint.chunk(world)?,
        region: viewpoint.region_origin(world, DRAWN_CHUNKS)?,
        resident: residency.len(),
        resident_cells: residency.cells(),
        residency: residency.digest(),
        world: being::surface_digest(&read),
        state: plane.digest(),
        ticks: plane.ticks(),
        standing: plane.standing(),
        deepest: plane.deepest(),
        materials: ladder
            .mapping()
            .into_iter()
            .map(|(class, id)| (class, id.0))
            .collect(),
        water: ladder.water().0,
        sun_altitude_deg: SUN_ALTITUDE_DEG,
        sun_tint: light().tint,
        world_side: plane.world().patch().side(),
        region_side: CHUNK_SIDE as u32 * DRAWN_CHUNKS,
    })
}

/// **The one call site that drives the client from the world the tree already builds** — and the window's own
/// loop: **the window shows the drawn reading** (§41) **from the person's own eye** (2026-10-14e). One frame
/// is one tick of the plane's own law, so the picture a person watches is the world routing itself into the
/// marsh *and* the world's own geometry being drawn under a camera with depth; every frame the person's own
/// held keys are read as one [`Wish`] ([`platform::Window::wish`]), the eye takes one step
/// ([`step_driven`]), and the picture is drawn **from the eye's own place** of the cells the residency holds
/// around the chunk the eye now stands in — which is why a moving world re-derives its chunks and the run
/// prints those counts; the loop is **bounded** by `frames` and returns when it is spent, when the person
/// closes the window, or when the platform refuses a call by name.
pub fn present_world(frames: u32, screenshot: Option<&Path>) -> Result<Presented, Refusal> {
    if frames == 0 {
        return Err(Refusal::new(
            RefusalKind::ParameterOutOfRange,
            "a window that presents 0 frame(s) is not a client: a frame count is at least one",
        ));
    }
    let cubic = client_world()?;
    let mut plane = Automaton::over(&cubic);
    // **The person's own viewpoint, and the residency it streams** — the viewpoint is the client's declared
    // view ([`home_viewpoint`] of the drawn region, which is what makes *the recorded frame comes back* a
    // property of the code), and the residency is built over the world's own store and observed again every
    // frame at **the chunk the eye now stands in** ([`step_driven`]), so the anchor is the person's own and
    // not the declared constant.
    let mut residency = residency_of(plane.world())?;
    // **The person the window opens as** (2026-10-14g): the declared view's own plan column, the feet on the
    // world's own ground, **walking** — so every frame below is drawn from an eye derived from that body
    // ([`Body::eye`]) and the drawn world is what the walk is stopped by.
    let mut viewpoint = person_of(&region_mesh(&residency, DRAWN_CHUNK)?, plane.world())?;
    let opened_as = viewpoint.body().described();
    // The window is opened for the frame's own declared size, before any frame is drawn: the read-back
    // compares the window's own client area with the bytes of a frame, so the two must be the same size.
    let blank = vec![0_u8; (drawn::DRAWN_SIZE as usize) * (drawn::DRAWN_SIZE as usize) * 4];
    let mut window = platform::Window::open(TITLE, drawn::DRAWN_SIZE, drawn::DRAWN_SIZE, &blank)?;
    let gap = Duration::from_millis(1_000 / u64::from(FRAMES_PER_SECOND));

    let mut counts = DrawnCounts::default();
    let mut last = None;
    let mut presented = 0_u32;
    let mut pixels = 0_usize;
    let mut agreeing = 0_usize;
    let mut first_mismatch = None;
    let mut closed = false;
    let mut stale = 0_u64;
    let mut derived = 0_u64;
    // **What the person's own input did**, counted over the run: how many frames carried a held key, how far
    // the eye travelled, how far its look turned, and the last step's own two keys. A run that nobody touched
    // is a run of still steps, and these numbers say so rather than leaving it to be assumed.
    let mut driven_frames = 0_u32;
    let mut travelled_m = 0.0_f64;
    let mut turned_deg = 0.0_f64;
    // **What the body's own steps did** — the parent's own quantities, summed over the run.
    let mut body_totals = BodyTotals::default();
    let mut anchor = viewpoint.chunk(plane.world())?;
    // **The loop's own clock, measured.** `gap` is the declared pacing target; the deadlines below are what
    // the loop aims at, and `seconds` is what it took. A frame that costs longer than the gap cannot be
    // paced — the loop then runs as fast as the picture allows and the measured rate says so, rather than
    // declaring a rate the picture does not keep.
    let start = Instant::now();
    'frames: for frame in 0..frames {
        if window.pump()? {
            closed = true;
            break 'frames;
        }
        plane.step()?;
        // **The window's own traversal** — `F` takes up flight on the press and `F` again puts it down
        // ([`platform::Window::traversal`]), so the option is a key a person holds rather than a state the
        // client is in, and the same one step below carries either body.
        viewpoint.traverse(window.traversal());
        let step = step_driven(
            &plane,
            &mut residency,
            &mut viewpoint,
            &window.wish(),
            frame_seconds(),
        )?;
        stale += step.stream.stale as u64;
        derived += step.stream.derived as u64;
        if !step.movement.was_still() {
            driven_frames += 1;
        }
        travelled_m += step.movement.moved_m;
        turned_deg += step.movement.turned_deg;
        body_totals.plan_m += step.movement.body.plan_m;
        body_totals.rose_m += step.movement.body.rose_m;
        body_totals.fell_m += step.movement.body.fell_m;
        body_totals.climbed_m += step.movement.body.climbed_m;
        body_totals.jumps += u64::from(step.movement.body.jumped);
        body_totals.blocked_frames += u64::from(step.movement.body.blocked);
        body_totals.grounded_end = step.movement.body.grounded;
        body_totals.traversal_end = step.movement.body.traversal;
        anchor = step.anchor;
        let drawn = step.drawn;
        window.present(drawn.pixels())?;
        let read_back = window.client_agrees(drawn.pixels())?;
        pixels += read_back.pixels;
        agreeing += read_back.agreeing;
        if first_mismatch.is_none() {
            first_mismatch = read_back.first;
        }
        counts.add(&drawn);
        last = Some(drawn);
        presented += 1;
        if frame + 1 < frames {
            let deadline = start + gap * (frame + 1);
            let now = Instant::now();
            if now < deadline {
                std::thread::sleep(deadline - now);
            }
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    let rate = if seconds > 0.0 { f64::from(presented) / seconds } else { 0.0 };
    // **The screen holds the frame or this client says so by name.** Every pixel of the window's own client
    // area was read back and compared with the frame's own byte; one that differs is a picture the window did
    // not show whole (a minimized or hidden window answers this way), and the refusal names the first pixel
    // and both values rather than reporting a frame nobody saw.
    if pixels > 0 && agreeing != pixels {
        let detail = match first_mismatch {
            Some((x, y, ours, theirs)) => format!(
                "at ({x},{y}) the frame's own bytes are {ours:?} and the window's are {theirs:?}"
            ),
            None => "no first disagreement was recorded, which is itself a defect".to_string(),
        };
        return Err(Refusal::new(
            RefusalKind::PlatformRefused,
            format!(
                "the window's own client area is the drawn frame at {agreeing} of {pixels} pixel(s) — \
                 {detail}"
            ),
        ));
    }
    let last = match last {
        Some(drawn) => drawn,
        None => {
            // A person closed the window before a frame was drawn: the report still describes a picture, and
            // it is this world's own — the residency observed at the tick the loop stopped at, from the eye
            // the person's own state stands at.
            residency.observe(plane.world(), anchor)?;
            let region = viewpoint.region_origin(plane.world(), DRAWN_CHUNKS)?;
            draw_from(&residency, &viewpoint, region)?
        }
    };
    // **The headless drawn frame of *this run's own last state, from this run's own eye*** — the same state,
    // the same eye, the same region, the same call, no window, and a **fresh residency** of it — and the drawn
    // reading's last bar is the two **pictures** being one picture. 2026-10-13 (§42): this line used to
    // compare `headless.state == drawing.state`, a **state** digest under a label claiming the picture, so it
    // could read `false` on a legal invocation and the run still exited 0. It now compares the frames' own
    // digests and it **gates** (`Drawing::bars`), so a window whose last frame is not what a windowless draw
    // of the same state and the same eye produces refuses by name — and the fresh residency makes the bar say
    // *the residency path is a function of the state* rather than *one residency called twice*. 2026-10-14e:
    // the eye is the run's own, because a person who has driven the camera is not at the declared view and a
    // bar that compared against the declared view would fail a legal run.
    let headless = draw_from(
        &residency_of(plane.world())?,
        &viewpoint,
        viewpoint.region_origin(plane.world(), DRAWN_CHUNKS)?,
    )?
    .digest();
    let drawing = drawing_of(&plane, &residency, &viewpoint, counts, last, headless)?;
    let screenshot = match screenshot {
        Some(path) => {
            drawing.last.write_ppm(path)?;
            Some(path.to_path_buf())
        }
        None => None,
    };
    Ok(Presented {
        drawing,
        frames,
        presented,
        seconds,
        rate,
        closed,
        settled: plane.ticks() >= SETTLE_TICKS as u64,
        pixels,
        agreeing,
        dpi_aware: window.dpi_aware(),
        screenshot,
        stale,
        derived,
        driven_frames,
        travelled_m,
        turned_deg,
        opened_as,
        body: body_totals,
    })
}

/// **The reference world's own ladder** for the client: the same class → material mapping the mesher and
/// the shell draw a chunk with ([`reference::ladder`]), so the picture and the geometry cannot disagree
/// about which material a class is.
///
/// Since §38 the subject is the **cubic world itself** — §37's ruling 4 (*"a plan-view instrument over a
/// cubic world"*) made literal: the declared fields are filled into their world once, the law's state is
/// that world's own cells, and every frame draws the world's own reading of it. The client's projection,
/// shading and bars are untouched; what moved with the law is the input to its frames.
/// **The client's own world** — the drawn reading's subject, declared in one place: the reference
/// declaration's own roster, identity and ladder, the same three fields, filled into a patch **widened to
/// whole chunks** ([`CLIENT_CHUNKS`] of them a side, [`CLIENT_SIDE`] columns) so the region below is a
/// rectangle of the world's own cells rather than half a rectangle over an edge the world does not own. The
/// plan reading keeps the reference request's own patch ([`process::declared_request`], `48 × 48`); this one
/// is the client's, and its size is printed with every frame through [`Drawing::world_side`].
///
/// §42's own finding, stated here where the number lives: the drawn reading could not draw a stitched region
/// **because this world was `48` columns — one and a half chunks** — so "the world's own first chunk" was
/// the only whole chunk it held. A region needs whole chunks of world, and the world is the one thing this
/// client may declare for itself (the being, the cell law and the extent contract are untouched by it).
pub fn client_world() -> Result<CubicWorld, Refusal> {
    let roster = roster()?;
    let identity = reference::identity()?;
    let ladder = reference::ladder()?;
    let origin = process::declared_request().patch()?.origin();
    let request = ProcessRequest::Region { origin, side: CLIENT_SIDE };
    // The world through **the world's own door** ([`CubicWorld::from_fields`]): one read of the declared
    // surface ([`surface_fields`]), one fill. This client declares a wider request; it does not restate the
    // build.
    CubicWorld::from_fields(&roster, identity, Tick(0), request, &ladder, 1)
}

/// The plan reading's own subject: the reference world's patch filled from its declared surface, and the
/// ladder the frame's **water** is drawn with.
///
/// The frame's materials are no longer read from the `lithology` class plane (2026-10-14f): the cells the
/// world was filled with hold them, and the frame asks the cells. So this door reads the surface itself
/// ([`surface_fields`], the world's one read), fills the world from it — one construction, the same
/// [`CubicWorld::from_surface`] every other consumer uses — and returns the ladder for the one palette fact
/// the cells cannot answer ([`Ladder::water`]).
fn subject() -> Result<(CubicWorld, Ladder), Refusal> {
    let roster = roster()?;
    let identity = reference::identity()?;
    let request = process::declared_request();
    let patch = request.patch()?;
    let ladder = reference::ladder()?;
    let values = surface_fields(&roster, identity, Tick(0), request, 1)?;
    let cubic = CubicWorld::from_surface(patch, &values[0], &values[1], &values[2], &ladder)?;
    Ok((cubic, ladder))
}

fn roster() -> Result<Declared, Refusal> {
    process::roster().map_err(|refusals: RefusalSet| {
        refusals.joined().unwrap_or_else(|| {
            Refusal::new(
                RefusalKind::StageUndeclared,
                "the declared roster refused without naming a refusal, which is itself a defect",
            )
        })
    })
}

/// **The client's own light** — U5's solver in the declared state, read for the Sun's own transmitted
/// colour: the tint is the solver's spectra response over its own `sun_transmittance`, so a lower Sun is a
/// redder one *because its own air column says so*, never because this client picked a colour.
pub fn light() -> Light {
    let solver = light::Solver::new(
        light::Spectra::standard(),
        light::State::clear(SUN_ALTITUDE_DEG, -20.0, 0.0),
    );
    let read = solver.spectra().response(&solver.sun_transmittance());
    // The solver's response is a **chromaticity**, not an irradiance — `light.rs` says so itself ("a 5 778 K
    // body renders near the model's reference white") — so this client keeps the **shape** and normalises it
    // to its own brightest channel: the Sun's colour still reddens as its air column thickens at a lower
    // altitude, and the table's own absolute scale never dims the frame. The tint the light carries is
    // printed by the run, so the number this decision makes is a measured one.
    let brightest = read.iter().copied().fold(0.0_f64, f64::max);
    let tint = if brightest > 0.0 {
        [read[0] / brightest, read[1] / brightest, read[2] / brightest]
    } else {
        [1.0, 1.0, 1.0]
    };
    let altitude = SUN_ALTITUDE_DEG.to_radians();
    Light {
        toward: unit([0.0, altitude.sin(), altitude.cos()]),
        tint,
    }
}

/// **The client's own view** — the declared direction toward the viewer, normalised here so the contract's
/// own `not_a_unit_vector` refusal is a fact about this client's arithmetic and not about the world.
pub fn view() -> View {
    let altitude = VIEW_ALTITUDE_DEG.to_radians();
    View { toward: unit([0.0, altitude.sin(), altitude.cos()]) }
}

/// A column's own normal: level where the world's own law stands water (a water surface has no slope of its
/// own), and the terrain's own gradient elsewhere — read from the elevation field the world carries, with
/// the declared [`RELIEF_EXAGGERATION`] the only thing this client adds to it.
pub fn normal_of(world: &WorldRead, cell: usize, wet: bool) -> [f64; 3] {
    if wet {
        return [0.0, 0.0, 1.0];
    }
    let side = world.patch.side() as usize;
    if side == 0 || world.elevation.len() != side * side {
        return [0.0, 0.0, 1.0];
    }
    let (x, y) = (cell % side, cell / side);
    let at = |x: usize, y: usize| world.elevation[y * side + x] * RELIEF_EXAGGERATION;
    let left = at(x.saturating_sub(1), y);
    let right = at((x + 1).min(side - 1), y);
    let down = at(x, y.saturating_sub(1));
    let up = at(x, (y + 1).min(side - 1));
    // Neighbouring columns are one metre apart on the world's own lattice (`DESIGN-SPEC` §5's signed
    // metre lattice), so the central difference is over two metres and needs no invented spacing.
    let change_x = (right - left) / 2.0;
    let change_y = (up - down) / 2.0;
    unit([-change_x, -change_y, 1.0])
}

/// **The one measurement of a frame** — its own pixels, the world they were drawn from, and the plane's own
/// state and ladder. Both the evidence run and the windowed run report these facts, so neither can describe
/// a picture the other did not draw.
fn measure(read: &WorldRead, plane: &Automaton, surface: &Surface, ladder: &Ladder) -> Measured {
    let (dark, bright) = surface.range();
    Measured {
        width: surface.width(),
        height: surface.height(),
        columns: read.cells(),
        bands: surface.bands(),
        frame: surface.digest(),
        world: being::surface_digest(read),
        state: plane.digest(),
        ticks: plane.ticks(),
        standing: plane.standing(),
        deepest: plane.deepest(),
        materials: ladder.mapping().into_iter().map(|(class, id)| (class, id.0)).collect(),
        water: ladder.water().0,
        sun_altitude_deg: SUN_ALTITUDE_DEG,
        sun_tint: light().tint,
        dark,
        bright,
    }
}

/// A direction normalised to the unit vector the contract requires — one place, so every direction this
/// client hands to `shade` is normalised by the same arithmetic.
fn unit(direction: [f64; 3]) -> [f64; 3] {
    let length = (direction[0] * direction[0]
        + direction[1] * direction[1]
        + direction[2] * direction[2])
        .sqrt();
    if length == 0.0 {
        return [0.0, 0.0, 1.0];
    }
    [direction[0] / length, direction[1] / length, direction[2] / length]
}

/// The refusal `shade` made, named as the **contract's** own refusal with this client's column it happened
/// at — a refusal is passed through rather than replaced by a plausible colour.
fn shade_refusal(kind: presentation::RefusalKind, cell: usize) -> Refusal {
    let named = match kind {
        presentation::RefusalKind::NoReflectance => RefusalKind::NoReflectance,
        // A direction this client built was not a unit vector: the fault is this client's arithmetic, and
        // the contract's own name for it is carried in the detail.
        presentation::RefusalKind::NotAUnitVector => RefusalKind::ParameterOutOfRange,
        // **The port's own refusal.** `presentation::shade` is now the parent's shader (P7 v2.0.0), and
        // when that contract refuses an input this client's vocabulary cannot name, the refusal is
        // passed through as this client's parameter fault with the ported contract's kind in the detail.
        presentation::RefusalKind::AppearanceRefused => RefusalKind::ParameterOutOfRange,
    };
    Refusal::new(
        named,
        format!(
            "the one shade call refused `{}` at column {cell}",
            kind.name()
        ),
    )
}

/// The platform's own window door, and the **one** module licensed to call it.
///
/// A `#![deny(unsafe_code)]` crate with `#[allow(unsafe_code)]` here: the licence is one line, it is this
/// module, and it exists because a window is a platform interface and a platform interface is `unsafe`.
/// Every declaration below is the platform's own published signature, transcribed as a type (Constitution
/// P7: no line is copied from any implementation; an interface's shape is not an implementation).
#[cfg(windows)]
mod platform {
    #![allow(unsafe_code)]

    use std::ffi::c_void;

    use crate::refusal::{Refusal, RefusalKind};
    use crate::viewpoint::Wish;

    type Handle = *mut c_void;
    type Instance = *mut c_void;
    type Device = *mut c_void;
    type Brush = *mut c_void;
    type Icon = *mut c_void;
    type Cursor = *mut c_void;
    type Procedure = Option<unsafe extern "system" fn(Handle, u32, usize, isize) -> isize>;

    #[repr(C)]
    struct WindowClass {
        size: u32,
        style: u32,
        procedure: Procedure,
        class_extra: i32,
        window_extra: i32,
        instance: Instance,
        icon: Icon,
        cursor: Cursor,
        background: Brush,
        menu_name: *const u16,
        class_name: *const u16,
        icon_small: Icon,
    }

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct Message {
        window: Handle,
        message: u32,
        w_param: usize,
        l_param: isize,
        time: u32,
        point: Point,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct PaintStruct {
        device: Device,
        erase: i32,
        paint: Rect,
        restore: i32,
        inc_update: i32,
        reserved: [u8; 32],
    }

    #[repr(C)]
    struct BitmapHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bits: u16,
        compression: u32,
        image_size: u32,
        x_pels: i32,
        y_pels: i32,
        colours_used: u32,
        colours_important: u32,
    }

    #[repr(C)]
    struct BitmapInfo {
        header: BitmapHeader,
        colours: [u32; 1],
    }

    #[link(name = "user32")]
    extern "system" {
        fn RegisterClassExW(class: *const WindowClass) -> u16;
        fn CreateWindowExW(
            ex_style: u32,
            class_name: *const u16,
            window_name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Handle,
            menu: *mut c_void,
            instance: Instance,
            param: *mut c_void,
        ) -> Handle;
        fn DefWindowProcW(window: Handle, message: u32, w_param: usize, l_param: isize) -> isize;
        fn GetModuleHandleW(name: *const u16) -> Instance;
        fn AdjustWindowRect(rect: *mut Rect, style: u32, menu: i32) -> i32;
        fn ShowWindow(window: Handle, command: i32) -> i32;
        fn UpdateWindow(window: Handle) -> i32;
        fn PeekMessageW(
            message: *mut Message,
            window: Handle,
            low: u32,
            high: u32,
            remove: u32,
        ) -> i32;
        fn TranslateMessage(message: *const Message) -> i32;
        fn DispatchMessageW(message: *const Message) -> isize;
        fn GetDC(window: Handle) -> Device;
        fn ReleaseDC(window: Handle, device: Device) -> i32;
        fn GetDIBits(
            device: Device,
            bitmap: Handle,
            start: u32,
            lines: u32,
            bits: *mut c_void,
            info: *mut BitmapInfo,
            usage: u32,
        ) -> i32;
        fn CreateCompatibleDC(device: Device) -> Device;
        fn CreateCompatibleBitmap(device: Device, width: i32, height: i32) -> Handle;
        fn SelectObject(device: Device, object: Handle) -> Handle;
        fn BitBlt(
            dest: Device,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            source: Device,
            x_src: i32,
            y_src: i32,
            rop: u32,
        ) -> i32;
        fn DeleteObject(object: Handle) -> i32;
        fn DeleteDC(device: Device) -> i32;
        fn DestroyWindow(window: Handle) -> i32;
        fn PostQuitMessage(code: i32);
        fn GetClientRect(window: Handle, rect: *mut Rect) -> i32;
        fn SetWindowLongPtrW(window: Handle, index: i32, value: isize) -> isize;
        fn GetWindowLongPtrW(window: Handle, index: i32) -> isize;
        fn SetProcessDPIAware() -> i32;

        fn BeginPaint(window: Handle, paint: *mut PaintStruct) -> Device;
        fn EndPaint(window: Handle, paint: *const PaintStruct) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn StretchDIBits(
            device: Device,
            x_dest: i32,
            y_dest: i32,
            width_dest: i32,
            height_dest: i32,
            x_src: i32,
            y_src: i32,
            width_src: i32,
            height_src: i32,
            bits: *const c_void,
            info: *const BitmapInfo,
            usage: u32,
            rop: u32,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetLastError() -> u32;
    }

    const MESSAGE_DESTROY: u32 = 0x0002;
    const MESSAGE_ERASE_BACKGROUND: u32 = 0x0014;
    const MESSAGE_PAINT: u32 = 0x000F;
    const MESSAGE_QUIT: u32 = 0x0012;
    // **The key messages (2026-10-14e)**, transcribed as the interface's own numbers: the platform's
    // `WM_KEYDOWN`/`WM_KEYUP` and their system-key twins carry the virtual-key code in the low word of
    // `w_param`, and `WM_CHAR` is what `TranslateMessage` makes of them (which this client discards — it reads
    // **intent**, never text). `WM_KILLFOCUS` is the one that matters for a held key: a person who alt-tabs
    // mid-step is not still holding it, and a client that thought so would fly on its own.
    const MESSAGE_KEY_DOWN: u32 = 0x0100;
    const MESSAGE_KEY_UP: u32 = 0x0101;
    const MESSAGE_CHAR: u32 = 0x0102;
    const MESSAGE_SYS_KEY_DOWN: u32 = 0x0104;
    const MESSAGE_SYS_KEY_UP: u32 = 0x0105;
    const MESSAGE_KILL_FOCUS: u32 = 0x0021;
    const STYLE_OVERLAPPED: u32 = 0x00C8_0000; // caption and a system menu: a declared size, no drag frame
    const STYLE_VISIBLE: u32 = 0x1000_0000;
    const SHOW: i32 = 5;
    const PEEK_REMOVE: u32 = 1;
    const DIB_RGB_COLOURS: u32 = 0;
    const COPY: u32 = 0x00CC_0020;
    const USER_DATA: i32 = -21;
    const CLASS_ALREADY_EXISTS: u32 = 1410;
    const COLOUR_WINDOW_BRUSH: Brush = 6 as Brush;
    /// The message drain is **bounded**: a frame loop that could be held hostage by a flood of messages
    /// would be a hang, and this client is required not to have one.
    const DRAIN_LIMIT: usize = 64;

    // **The keys a person drives with**, by the platform's own names for them — `VK_LEFT` and its
    // neighbours. The parent's own client is the source of the mapping
    // (`crates/shploingus/src/main.rs`: `KeyW`/`KeyS` forward and back, `KeyD`/`KeyA` to the eye's own right
    // and left, `Space`/`ControlLeft` up and down, at `FLY_M_PER_S`), and where it turns its look with a
    // **mouse motion** this client turns it with a key, because it has no device event of its own: the arrow
    // keys, which is the one thing here the parent has no number for (`crate::viewpoint` says so where the
    // turn's own size is declared).
    const VK_CONTROL: usize = 0x11;
    const VK_SPACE: usize = 0x20;
    /// The key that toggles the person's own traversal between walking (the default) and flight.
    const VK_F: usize = 0x46;
    const VK_LEFT: usize = 0x25;
    const VK_UP: usize = 0x26;
    const VK_RIGHT: usize = 0x27;
    const VK_DOWN: usize = 0x28;
    const VK_A: usize = 0x41;
    const VK_D: usize = 0x44;
    const VK_S: usize = 0x53;
    const VK_W: usize = 0x57;
    /// The virtual-key code space, and so the size of the held-key table below.
    const KEYS: usize = 256;

    /// What the window procedure needs to paint: the bytes of the frame being presented, and their header.
    /// It is owned by the [`Window`] below, and the window procedure reads it through the window's own user
    /// data — only ever on the thread that owns the `Window`, so no two runs of it overlap.
    struct Presenting {
        pixels: Vec<u8>,
        info: BitmapInfo,
    }

    /// **The one window** — created for a frame of a declared size, presented to once per frame, and
    /// destroyed when it is dropped. It holds the memory device and bitmap the read-back copies the window's
    /// own client area into, so a frame's read-back allocates nothing.
    pub(super) struct Window {
        window: Handle,
        presenting: Box<Presenting>,
        readback: Device,
        bitmap: Handle,
        previous: Handle,
        pixels: Vec<u8>,
        info: BitmapInfo,
        dpi_aware: bool,
        width: i32,
        height: i32,
        /// **Which keys a person is holding**, by the platform's own virtual-key code — the one piece of
        /// input state this client keeps, and the only place a key is ever named ([`Window::wish`] turns it
        /// into the intent `crate::viewpoint::Wish`, and nothing above this module knows a `VK_`). Cleared
        /// whole when the window loses focus.
        held: [bool; KEYS],
        /// Whether the person has taken up flight with `F` ([`Window::traversal`]) — the one piece of the
        /// traversal's own state the window owns, **false** at the start because walking is the default.
        flying: bool,
    }

    /// What one frame's read-back measured: how many pixels the client area holds, how many of them are the
    /// frame's own bytes, and — when one is not — the first disagreement, so a refusal can name it.
    pub(super) struct ReadBack {
        pub pixels: usize,
        pub agreeing: usize,
        pub first: Option<(u32, u32, [u8; 4], [u8; 4])>,
    }

    unsafe extern "system" fn procedure(
        window: Handle,
        message: u32,
        w_param: usize,
        l_param: isize,
    ) -> isize {
        match message {
            MESSAGE_PAINT => {
                // A paint that is not ended leaves the window's own update region unvalidated, and then the
                // platform paints again, and again: `BeginPaint`/`EndPaint` is what makes one paint one
                // paint.
                let mut paint: PaintStruct = std::mem::zeroed();
                let device = BeginPaint(window, &mut paint);
                let presenting = GetWindowLongPtrW(window, USER_DATA) as *const Presenting;
                if !device.is_null() && !presenting.is_null() {
                    let presenting = &*presenting;
                    StretchDIBits(
                        device,
                        0,
                        0,
                        presenting.info.header.width,
                        -presenting.info.header.height,
                        0,
                        0,
                        presenting.info.header.width,
                        -presenting.info.header.height,
                        presenting.pixels.as_ptr() as *const c_void,
                        &presenting.info,
                        DIB_RGB_COLOURS,
                        COPY,
                    );
                }
                EndPaint(window, &paint);
                0
            }
            // The frame covers every pixel of the client area, so the background never needs erasing: saying
            // so is what keeps a presented frame from flashing.
            MESSAGE_ERASE_BACKGROUND => 1,
            // **A key is intent, and `pump` has already read it** (2026-10-14e): the procedure consumes the
            // key messages and the character the platform makes of them, so the platform's own default
            // behaviour — which beeps at an unhandled key — never runs for a key a person is driving with.
            MESSAGE_KEY_DOWN | MESSAGE_KEY_UP | MESSAGE_SYS_KEY_DOWN | MESSAGE_SYS_KEY_UP | MESSAGE_CHAR => 0,
            MESSAGE_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, message, w_param, l_param),
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn info_for(width: i32, height: i32) -> BitmapInfo {
        BitmapInfo {
            header: BitmapHeader {
                size: std::mem::size_of::<BitmapHeader>() as u32,
                width,
                // A negative height declares the rows **top-down**, which is the order the surface's own
                // pixels are in: no row is flipped anywhere in this client.
                height: -height,
                planes: 1,
                bits: 32,
                compression: 0, // BI_RGB
                image_size: (width * height * 4) as u32,
                x_pels: 0,
                y_pels: 0,
                colours_used: 0,
                colours_important: 0,
            },
            colours: [0],
        }
    }

    impl Window {
        /// **Open the one window** for a frame's own size: the class is registered once, the window is created
        /// with a client area of exactly `width × height` pixel(s) (the frame *is* the client area, which is
        /// what makes the read-back exact), and shown. Every failure is refused by name.
        pub(super) fn open(
            title: &str,
            width: u32,
            height: u32,
            pixels: &[u8],
        ) -> Result<Window, Refusal> {
            if width == 0 || height == 0 {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!("a window of {width} x {height} pixel(s) has nothing to show"),
                ));
            }
            // The bytes handed in are the frame the window opens on, so they must be that frame's own size: a
            // window opened for a frame of another size could not compare its client area with one.
            if pixels.len() != (width as usize) * (height as usize) * 4 {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!(
                        "a {width} x {height} pixel(s) window was handed {} byte(s) rather than {}",
                        pixels.len(),
                        (width as usize) * (height as usize) * 4
                    ),
                ));
            }
            let width = width as i32;
            let height = height as i32;
            let class_name = wide("shploingus-clean-room-visible-client");
            let title = wide(title);
            let presenting = Box::new(Presenting {
                pixels: pixels.to_vec(),
                info: info_for(width, height),
            });
            let window: Handle;
            let readback: Device;
            let bitmap: Handle;
            let previous: Handle;
            let dpi_aware: bool;
            // SAFETY: every call below is the platform's own interface, made on this thread, with a window
            // the platform owns; the pointer kept in the window's user data is the box above, which
            // outlives the window (it is dropped after the window is destroyed, below).
            unsafe {
                dpi_aware = SetProcessDPIAware() != 0;
                let instance = GetModuleHandleW(std::ptr::null());
                let class = WindowClass {
                    size: std::mem::size_of::<WindowClass>() as u32,
                    style: 0,
                    procedure: Some(procedure),
                    class_extra: 0,
                    window_extra: 0,
                    instance,
                    icon: std::ptr::null_mut(),
                    cursor: std::ptr::null_mut(),
                    background: COLOUR_WINDOW_BRUSH,
                    menu_name: std::ptr::null(),
                    class_name: class_name.as_ptr(),
                    icon_small: std::ptr::null_mut(),
                };
                if RegisterClassExW(&class) == 0 && GetLastError() != CLASS_ALREADY_EXISTS {
                    return Err(Refusal::new(
                        RefusalKind::PlatformRefused,
                        format!(
                            "the platform refused to register the client's own window class \
                             (error {})",
                            GetLastError()
                        ),
                    ));
                }
                let mut rect = Rect { left: 0, top: 0, right: width, bottom: height };
                if AdjustWindowRect(&mut rect, STYLE_OVERLAPPED | STYLE_VISIBLE, 0) == 0 {
                    return Err(Refusal::new(
                        RefusalKind::PlatformRefused,
                        "the platform refused to size the client's window frame",
                    ));
                }
                window = CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    title.as_ptr(),
                    STYLE_OVERLAPPED | STYLE_VISIBLE,
                    40,
                    40,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    instance,
                    std::ptr::null_mut(),
                );
                if window.is_null() {
                    return Err(Refusal::new(
                        RefusalKind::PlatformRefused,
                        format!(
                            "the platform refused to create the client's window (error {})",
                            GetLastError()
                        ),
                    ));
                }
                // The client area must be the frame's own size, or a presented frame would not cover the
                // window and the read-back below would be sampling something the surface does not own.
                let mut client = Rect { left: 0, top: 0, right: 0, bottom: 0 };
                if GetClientRect(window, &mut client) == 0
                    || client.right - client.left != width
                    || client.bottom - client.top != height
                {
                    DestroyWindow(window);
                    return Err(Refusal::new(
                        RefusalKind::PlatformRefused,
                        format!(
                            "the platform made a {} x {} pixel(s) client area for a {width} x {height} 
                             frame: the frame is the client area or it is not the picture",
                            client.right - client.left,
                            client.bottom - client.top
                        ),
                    ));
                }
                // The read-back's own memory device and bitmap, made once for this window's size and reused
                // by every frame. The bitmap is made compatible with a **colour** device — the screen —
                // because a fresh memory device's own bitmap is one monochrome pixel, and a copy into that
                // would arrive black.
                let screen = GetDC(std::ptr::null_mut());
                readback = CreateCompatibleDC(screen);
                bitmap = CreateCompatibleBitmap(screen, width, height);
                ReleaseDC(std::ptr::null_mut(), screen);
                if readback.is_null() || bitmap.is_null() {
                    DestroyWindow(window);
                    return Err(Refusal::new(
                        RefusalKind::PlatformRefused,
                        "the platform refused a memory device to read the client's own area back through",
                    ));
                }
                previous = SelectObject(readback, bitmap);
                SetWindowLongPtrW(window, USER_DATA, (&*presenting) as *const Presenting as isize);
                ShowWindow(window, SHOW);
                UpdateWindow(window);
            }
            Ok(Window {
                window,
                presenting,
                readback,
                bitmap,
                previous,
                pixels: vec![0; (width * height * 4) as usize],
                info: info_for(width, height),
                dpi_aware,
                width,
                height,
                held: [false; KEYS],
                flying: false,
            })
        }

        /// **What the person is asking for**, read off the keys they are holding — the window's own
        /// translation from the platform's virtual-key codes to the one intent vocabulary
        /// ([`crate::viewpoint::Wish`]). It is the **only** place in this crate where a key is named, so the
        /// shell's own words and a person's own hands cannot move the camera two different ways: both produce
        /// a `Wish`, and both are handed to the same step (`crate::surface::step_driven`).
        ///
        /// The three moves are the parent's own (`W`/`S`, `D`/`A`, `Space`/`Control`), and the turn is the
        /// arrow keys — the declared quarter turn a second, which the parent does with a mouse motion this
        /// client has no event for.
        pub(super) fn wish(&self) -> Wish {
            let held = |code: usize| if self.held[code] { 1.0 } else { 0.0 };
            Wish {
                forward: held(VK_W) - held(VK_S),
                strafe: held(VK_D) - held(VK_A),
                rise: held(VK_SPACE) - held(VK_CONTROL),
                turn: held(VK_RIGHT) - held(VK_LEFT),
                tilt: held(VK_UP) - held(VK_DOWN),
                // **The walk's own vertical intent** (2026-10-14g): `Space` is the parent's jump, held like
                // its own. A flying body ignores it and rises along `rise` instead, which is why the two are
                // separate intents rather than one.
                jump: self.held[VK_SPACE],
            }
        }

        /// **How the person is traversing** — walking unless they have taken up flight with `F`, which is
        /// toggled on the **press** (so a held key does not flip the mode every frame). Walking is where a
        /// person starts, which is what makes flight an option rather than a state.
        pub(super) const fn traversal(&self) -> crate::body::Traversal {
            if self.flying {
                crate::body::Traversal::Flying
            } else {
                crate::body::Traversal::Walking
            }
        }

        pub(super) const fn dpi_aware(&self) -> bool {
            self.dpi_aware
        }

        /// **Present one frame** — the frame's own bytes into the window's own client area, in one blit, with
        /// the platform's own answer checked: a blit that copied fewer scan lines than the frame has is a
        /// picture that did not reach the screen, and that is refused by name.
        ///
        /// The frame's bytes are copied into the window's own buffer first, so the window procedure always
        /// paints one complete frame — the box the procedure reads is never the frame a caller still holds.
        pub(super) fn present(&mut self, pixels: &[u8]) -> Result<(), Refusal> {
            if pixels.len() != self.presenting.pixels.len() {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!(
                        "the window was opened for a {} x {} pixel(s) frame ({} byte(s)) and this one is {} \
                         byte(s)",
                        self.width,
                        self.height,
                        self.presenting.pixels.len(),
                        pixels.len()
                    ),
                ));
            }
            self.presenting.pixels.copy_from_slice(pixels);
            let lines;
            // SAFETY: the window is open (the handle was checked at creation and this method takes `&mut
            // self`, which cannot outlive the `Window` that destroys it), the device context is released
            // below, and the box the window procedure reads was written above.
            unsafe {
                let device = GetDC(self.window);
                if device.is_null() {
                    return Err(Refusal::new(
                        RefusalKind::PlatformRefused,
                        "the platform refused the client's window a device to draw on",
                    ));
                }
                let bits = self.presenting.pixels.as_ptr() as *const c_void;
                lines = StretchDIBits(
                    device,
                    0,
                    0,
                    self.width,
                    self.height,
                    0,
                    0,
                    self.width,
                    self.height,
                    bits,
                    &self.presenting.info,
                    DIB_RGB_COLOURS,
                    COPY,
                );
                ReleaseDC(self.window, device);
            }
            if lines != self.height {
                return Err(Refusal::new(
                    RefusalKind::PlatformRefused,
                    format!(
                        "the platform's own blit copied {lines} of {} scan line(s): the frame did not \
                         reach the screen whole",
                        self.height
                    ),
                ));
            }
            Ok(())
        }

        /// **Read the window's own client area back and compare it with the frame, pixel for pixel.**
        ///
        /// The window's content is copied into this client's own memory bitmap and then written out through
        /// `GetDIBits` into a DIB **this client declares** (32-bit, `BI_RGB`, top-down — [`super::READBACK_BITS`]'s
        /// format), so the comparison is a **byte** comparison under one declared layout rather than a
        /// reading of the platform's colour value, whose channel order is a convention rather than a
        /// measurement. Every pixel of the client area is compared: this is the fact behind *the screen holds
        /// what the surface owns*, and one differing pixel is enough to say it does not.
        pub(super) fn client_agrees(&mut self, pixels: &[u8]) -> Result<ReadBack, Refusal> {
            let mut first = None;
            let mut agreeing = 0_usize;
            // SAFETY: `present`'s invariants hold (the window is open and outlives this call), the two
            // devices are this `Window`'s own, the borrowed pair is released before returning, and the
            // buffer written is this `Window`'s own `pixels`.
            unsafe {
                let device = GetDC(self.window);
                if device.is_null() {
                    return Err(Refusal::new(
                        RefusalKind::PlatformRefused,
                        "the platform refused the client's window a device to read back from",
                    ));
                }
                let copied = BitBlt(
                    self.readback,
                    0,
                    0,
                    self.width,
                    self.height,
                    device,
                    0,
                    0,
                    COPY,
                );
                ReleaseDC(self.window, device);
                if copied == 0 {
                    return Err(Refusal::new(
                        RefusalKind::PlatformRefused,
                        "the platform refused to copy the client's own client area for reading back",
                    ));
                }
                let lines = GetDIBits(
                    self.readback,
                    self.bitmap,
                    0,
                    self.height as u32,
                    self.pixels.as_mut_ptr() as *mut c_void,
                    &mut self.info,
                    DIB_RGB_COLOURS,
                );
                if lines != self.height {
                    return Err(Refusal::new(
                        RefusalKind::PlatformRefused,
                        format!(
                            "the platform wrote {lines} of {} scan line(s) of the client's own area into \
                             the declared format",
                            self.height
                        ),
                    ));
                }
            }
            let total = (self.width * self.height) as usize;
            if pixels.len() != total * 4 {
                return Err(Refusal::new(
                    RefusalKind::ParameterOutOfRange,
                    format!(
                        "the window was opened for a {} x {} pixel(s) frame ({} byte(s)) and this one is {} \
                         byte(s)",
                        self.width,
                        self.height,
                        total * 4,
                        pixels.len()
                    ),
                ));
            }
            for pixel in 0..total {
                let at = pixel * 4;
                if self.pixels[at..at + 3] == pixels[at..at + 3] {
                    agreeing += 1;
                    continue;
                }
                if first.is_none() {
                    let (x, y) = ((pixel as u32) % self.width as u32, (pixel as u32) / self.width as u32);
                    let ours = [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]];
                    let theirs = [self.pixels[at], self.pixels[at + 1], self.pixels[at + 2], 0];
                    first = Some((x, y, ours, theirs));
                }
            }
            Ok(ReadBack { pixels: total, agreeing, first })
        }

        /// **Drain the window's own messages**, bounded — returning whether the client has been asked to
        /// stop (the person closed the window, or the platform posted a quit). A bounded drain is what keeps
        /// a flood of messages from holding the frame loop hostage.
        ///
        /// **It is where a key becomes input (2026-10-14e).** Each drained message is read before it is
        /// dispatched: a key pressed is held, a key released is let go, and the window losing focus **lets go
        /// of all of them** — one function, so the held set can only change here and a key can only mean what
        /// [`Window::wish`] says it means.
        pub(super) fn pump(&mut self) -> Result<bool, Refusal> {
            let mut quit = false;
            for _ in 0..DRAIN_LIMIT {
                let mut message: Message = unsafe { std::mem::zeroed() };
                // SAFETY: the message is this thread's own, and it is dispatched on this thread.
                unsafe {
                    if PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PEEK_REMOVE) == 0 {
                        break;
                    }
                    if message.message == MESSAGE_QUIT {
                        quit = true;
                        break;
                    }
                    let key = message.w_param & 0xff;
                    match message.message {
                        MESSAGE_KEY_DOWN | MESSAGE_SYS_KEY_DOWN => {
                            // **Flight is taken up, not fallen into** (2026-10-14g): `F` toggles the person's
                            // traversal on the **press** — a held key does not flip it every frame — and
                            // walking is where a person starts. One key, so the option is reached by a hand
                            // rather than removed from the client.
                            if key == VK_F && !self.held[key] {
                                self.flying = !self.flying;
                            }
                            self.held[key] = true;
                        }
                        MESSAGE_KEY_UP | MESSAGE_SYS_KEY_UP => self.held[key] = false,
                        MESSAGE_KILL_FOCUS => self.held = [false; KEYS],
                        _ => {}
                    }
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            Ok(quit)
        }
    }

    impl Drop for Window {
        /// Destroy the window, and clear the user data so no message can read a pointer that is about to go.
        /// A `Drop` body runs **before** the fields are dropped, so the box the window procedure paints from
        /// outlives the window it paints in.
        fn drop(&mut self) {
            // SAFETY: the window is this `Window`'s own and is destroyed once, here.
            unsafe {
                SetWindowLongPtrW(self.window, USER_DATA, 0);
                DestroyWindow(self.window);
                // The read-back's own pair goes after the window it read, and the bitmap is put back the way
                // the memory device was handed to this client.
                SelectObject(self.readback, self.previous);
                DeleteObject(self.bitmap);
                DeleteDC(self.readback);
            }
        }
    }
}

/// **The same door on a platform this client does not build for**: it refuses by name rather than pretending
/// a window exists. The refusal is the honest answer — a client that cannot be shown is not a client.
#[cfg(not(windows))]
mod platform {
    use crate::refusal::{Refusal, RefusalKind};
    use crate::viewpoint::Wish;

    pub(super) struct Window;

    /// The same reading as the built platform's own: nothing was presented, so nothing was read back.
    pub(super) struct ReadBack {
        pub pixels: usize,
        pub agreeing: usize,
        pub first: Option<(u32, u32, [u8; 4], [u8; 4])>,
    }

    impl Window {
        pub(super) fn open(
            _title: &str,
            _width: u32,
            _height: u32,
            _pixels: &[u8],
        ) -> Result<Window, Refusal> {
            Err(Refusal::new(
                RefusalKind::PlatformRefused,
                "this platform's own window interface is not the one this client is built against: the \
                 client refuses by name rather than opening nothing",
            ))
        }

        pub(super) const fn dpi_aware(&self) -> bool {
            false
        }

        pub(super) fn present(&self, _pixels: &[u8]) -> Result<(), Refusal> {
            Err(Refusal::new(
                RefusalKind::PlatformRefused,
                "no window to present in on this platform",
            ))
        }

        pub(super) fn client_agrees(&mut self, _pixels: &[u8]) -> Result<ReadBack, Refusal> {
            Ok(ReadBack { pixels: 0, agreeing: 0, first: None })
        }

        /// No window here, so no key is ever held: the same reading as the built platform's own of an
        /// untouched window.
        pub(super) fn pump(&mut self) -> Result<bool, Refusal> {
            Ok(true)
        }

        pub(super) fn wish(&self) -> Wish {
            Wish::STILL
        }

        /// No key is ever held here either, so the traversal never leaves its default: walking.
        pub(super) const fn traversal(&self) -> crate::body::Traversal {
            crate::body::Traversal::Walking
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_direction_is_normalised_by_the_one_arithmetic_this_client_uses() {
        let normal = unit([0.0, 0.0, 3.0]);
        assert!(presentation::is_unit(normal), "{normal:?}");
        let sun = unit([0.0, 1.0, 1.0]);
        assert!(presentation::is_unit(sun));
    }
}
