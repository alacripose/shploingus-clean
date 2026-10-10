//! The client: a `Mesh` and a camera in, pixels out — **offscreen**.
//!
//! This crate is the half of `voxel-appearance` that has never existed. That
//! crate derives OpenPBR inputs from block truth and evaluates the four Valve
//! Source 2 terms; it also measures its own band count on a Fibonacci lattice of
//! view directions. What it has never done — what nothing in this repository has
//! ever done — is put a **picture on screen**. `docs/APPEARANCE.md` claims stage
//! **S0** and is accurate about it.
//!
//! # Offscreen first, and that is a decision rather than a stage of construction
//!
//! There is no window in this crate, and no windowing dependency in its
//! `Cargo.toml`. [`Renderer`] draws into a texture and reads it back. That is
//! what makes the FMA bar's procedure *runnable*: the bar advances a stage on
//! pixel analysis of a rendered frame, and a frame that can be read back is
//! analysable on a machine with no display attached, in a test, forever.
//! `winit` belongs to the `shploingus` bin, which is the only place a window is
//! ever created.
//!
//! # The shape of a frame
//!
//! 1. [`voxel_work::Streamer`] owns the residency and the budget. The client
//!    drives it — it never generates or meshes on the frame thread, because that
//!    is the whole reason the pool exists.
//! 2. [`Scene::build`] turns the meshes it holds into vertices, applying the
//!    chunk origin in `f64` and narrowing only the offset from the eye.
//! 3. [`shading::table`] derives one row of OpenPBR inputs per admitted material
//!    — refusing the whole table rather than shading a material it cannot
//!    describe.
//! 4. [`Renderer::render`] draws and reads back a [`Frame`].
//! 5. [`Frame::response`] hands the frame's luminances to
//!    `voxel_appearance::response::measure`, which is the **same** function that
//!    measured the lattice, so the two band counts are comparable.
//!
//! # What this crate does not do, stated rather than implied
//!
//! * No window. See above.
//! * No per-quad openness. The mesh merges quads across voxels, so a merged
//!   quad has no single `open`; see [`shading::MESH_DOES_NOT_CARRY_OPEN`].
//! * No reuse of GPU buffers between frames — every frame re-uploads. That is
//!   an honest first version and a named cost, not an oversight.

#![forbid(unsafe_code)]

pub mod camera;
pub mod camera_overhaul;
pub mod clock;
pub mod cloud_cubes;
pub mod cloud_field;
pub mod quad;
pub mod debug;
pub mod frame;
pub mod game;
pub mod hud;
pub mod motion;
pub mod renderer;
pub mod scene;
pub mod pattern;
pub mod shell;
pub mod shading;
pub mod skylight;
pub mod sun;
pub mod world;

pub use camera::Camera;
pub use camera_overhaul::CameraOverhaul;
pub use clock::{Clock, MAX_CATCH_UP, TICK_HZ};
pub use quad::QuadRecord;
pub use cloud_cubes::{rung_cell_m, rung_for, CloudCubes, MAX_CELL_M, MIN_CELL_M, RUNGS};
pub use cloud_field::{
    cloud_transmittance, density_of, extinction_per_m, optical_depth, CloudField, CloudMedium,
    CloudSun, CLOUD_BLOCK_SIDE, CLOUD_COLUMNS, CLOUD_SAMPLE_M, CLOUD_SPAN_M,
};
pub use frame::{luminance, Frame};
pub use game::{
    spawn_eye, GAME_CEILING_BYTES, GAME_PITCH_RAD, GAME_SEED, GAME_TICK, GAME_WINDOW,
};
pub use hud::{HudRenderer, HudState, BRIGHTEST_BACKDROP, DIGEST_CHARS};
pub use pattern::{
    address_at, eye_footprint_m, eye_phase, eye_voxel, octave_cell_m, octave_weight,
    pixel_angle_rad, quantise, quantise_cell, Pattern, DETAIL_CELL_M, FACE_COS_FLOOR,
    MAX_FEATURE_M, MIN_FEATURE_M, OCTAVE_GAIN, OCTAVE_TOP, SUBVOXEL_CELL_M, SUBVOXEL_OCTAVE,
};
pub use renderer::{offscreen_device, Renderer, DEPTH_FORMAT, TARGET_FORMAT};
pub use scene::{Scene, Vertex};
pub use shading::{
    shade_cpu, table, world_lattice_uniform, GpuRig, MaterialParams, ShadingReport,
    MESH_DOES_NOT_CARRY_OPEN,
};
pub use world::WorldPipeline;

use voxel_model::{Refusal, RefusalSet};
use voxel_ui::UiRefusal;

/// Why a frame could not be produced.
///
/// This is deliberately **not** a `RefusalKind`. `voxel-model`'s ten variants
/// are statements about *values in the model* — a coordinate out of range, a
/// ratio that does not sum, an unadmitted material — and "this machine has no
/// adapter" is not one of those. Adding a GPU variant there would put an
/// environment's accident into the world's vocabulary. So the client carries the
/// model's refusals through unchanged, in [`ClientError::Refusal`] and
/// [`ClientError::Refusals`], and names its own environment failures alongside
/// them. A new class of refusal is still a new variant; it is just this enum
/// here rather than that one there.
#[derive(Clone, PartialEq, Debug)]
pub enum ClientError {
    /// A single model refusal, carried through with its original kind.
    Refusal(Refusal),
    /// A set of them, so a caller sees all of it at once.
    Refusals(RefusalSet),
    /// The host has no adapter the requested backends can use.
    NoAdapter { detail: String },
    /// An adapter exists but would not give a device.
    NoDevice { detail: String },
    /// A frame has to be some pixels wide.
    ZeroExtent { width: u32, height: u32 },
    /// More lights than the rig uniform holds.
    TooManyLights { given: usize, holds: usize },
    /// A material id outside the table the shader was given.
    UnknownMaterial { id: u32, rows: u32 },
    /// Reading the rendered pixels back failed.
    Readback { detail: String },
    /// The **design system** refused a screen. Carried through with its own
    /// type for the same reason [`ClientError::Refusal`] carries the model's: a
    /// HUD that cannot be drawn legibly is a refusal about a layout, and it does
    /// not belong in the world's vocabulary or in this enum's environment
    /// failures.
    Ui(UiRefusal),
    /// Glyph shaping or the glyph atlas failed.
    TextRender { detail: String },
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Refusal(r) => write!(f, "refused: {r}"),
            ClientError::Refusals(s) => write!(f, "refused: {s}"),
            ClientError::NoAdapter { detail } => {
                write!(f, "no usable GPU adapter: {detail}")
            }
            ClientError::NoDevice { detail } => {
                write!(f, "the adapter would not give a device: {detail}")
            }
            ClientError::ZeroExtent { width, height } => {
                write!(f, "a {width}x{height} frame is not a frame")
            }
            ClientError::TooManyLights { given, holds } => {
                write!(f, "the rig has {given} lights and the uniform holds {holds}")
            }
            ClientError::UnknownMaterial { id, rows } => {
                write!(f, "material {id} is outside the {rows}-row table")
            }
            ClientError::Readback { detail } => write!(f, "readback failed: {detail}"),
            ClientError::Ui(refusal) => write!(f, "the design system refused: {refusal}"),
            ClientError::TextRender { detail } => write!(f, "text rendering failed: {detail}"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<Refusal> for ClientError {
    fn from(value: Refusal) -> Self {
        ClientError::Refusal(value)
    }
}

impl From<RefusalSet> for ClientError {
    fn from(value: RefusalSet) -> Self {
        ClientError::Refusals(value)
    }
}

impl From<UiRefusal> for ClientError {
    fn from(value: UiRefusal) -> Self {
        ClientError::Ui(value)
    }
}
