//! The HUD: `voxel-ui`'s [`Element`]s, on screen.
//!
//! # The division of labour, which is the whole design
//!
//! * [`voxel_ui`] decides everything a designer would decide: which ink step,
//!   how far apart the rows are, what a panel's surface is, and whether the text
//!   on it is legible. It answers with [`Element`]s whose colours are already
//!   **resolved** — a [`Paint`], not a role name.
//! * This module turns resolved paint into triangles and glyphs. It contains no
//!   colour arithmetic at all, and it could not invent a colour if it wanted to:
//!   there is nothing in an [`Element`] to invent one from.
//!
//! That is what makes the design system enforceable rather than aspirational. A
//! renderer that can compute a colour can compute the wrong one, and the only
//! way a component repaints itself outside the palette is by being handed
//! something other than a resolved paint.
//!
//! # What is drawn, and what is not
//!
//! Four panels — world, streamer, determinism, refusals — one in each screen
//! corner. The refusal panel is the one that matters most: this project's first
//! rule is *refuse, never default*, and a refusal that only reaches `stderr` in a
//! windowed program is a refusal nobody sees. When nothing has been refused the
//! panel carries a **note** rather than collapsing, because a view that
//! disappears when it is empty teaches a player that empty and broken look the
//! same.
//!
//! # The vector overlay is a second input, not an [`Element`]
//!
//! An element is axis-aligned by construction — a panel, a rule, a text box — and that is
//! everything the shell draws. **Not everything a view draws**: the change view's outline
//! (H-7.1/H-7.9, `crate::shell::changes`) is a set of projected world edges, and an edge runs at
//! whatever angle the camera put it at. So [`Stroke`]s arrive beside the elements and are drawn by
//! the same solid pipeline, and the shell's hierarchy reading (`crate::debug::drawing`) stays a
//! reading about the shell rather than about marks that land on no surface of it.
//!
//! # Shaping is paid for what changed, not for what is on screen
//!
//! The first version of this file reshaped every run every frame and named that as
//! the cost it accepted: a counter that changes every frame is text that changes
//! every frame. Measured, that was **2.8 ms of a 3.0 ms [`HudRenderer::prepare`]**
//! over 45 runs — the largest single item in the shell stage — and the argument was
//! wrong, because a frame changes *one* number and reshaped the 44 beside it.
//!
//! So the shaped runs have an owner ([`crate::shaped`]) with a rule of its own: a run
//! is shaped once per distinct (text, size, wrap width), however many frames or
//! windows ask for it, and released the first frame nobody does. This file keeps the
//! other half — where a run is placed, in what ink, in which band — and recomputes
//! that every frame, because placement is cheap and it is the half a window may
//! change at will. A settled frame touches no GPU buffer at all.
//!
//! # Why `glyphon` is named by this crate and not by the bin
//!
//! So the HUD can be drawn into an **offscreen** target and measured in a test —
//! see `tests::the_shell_chrome_is_legible_over_the_worst_backdrop`. A HUD that
//! exists only inside an event loop is a HUD nobody can check.

use crate::shading;
use crate::shaped::{RunKey, ShapedRuns, LINE_BOX};
use crate::ClientError;
use voxel_model::{Digest, Refusal};
use voxel_ui::{radius_dp, Align, Element, Paint, Panel, Readout, Rect, Role, WHITE};

/// The brightest thing the world can put behind the glass: an 8-bit swapchain
/// cannot present more than white. Every ink step the HUD uses is measured
/// against this, so "the HUD is legible" is a claim about the worst case rather
/// than about a screenshot.
///
/// **The limit, stated:** an HDR swapchain, or a material whose emission exceeds
/// white, would be brighter than this and would invalidate the assumption.
/// Neither exists in this build, and the constant is named so that when one does,
/// the thing to change has a name.
pub const BRIGHTEST_BACKDROP: voxel_ui::Rgb = WHITE;

/// **One stroke of the vector overlay**: a screen-space segment in dp, with its resolved paint.
///
/// A stroke's width is its own and its ends are points, not a rect, because the overlay draws
/// world geometry projected through the camera (`crate::shell::changes::Projection`) and a
/// projected edge has no axis to be aligned to. The width comes from the view that emits the
/// stroke (the change view declares one dp, the lane overlay's own pen), never from here — this
/// type carries resolved values and computes none.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Stroke {
    pub from_dp: [f64; 2],
    pub to_dp: [f64; 2],
    pub width_dp: f64,
    pub paint: Paint,
}

/// Everything the HUD is told. Every field is a value that was **measured**; the
/// HUD computes no statistic and estimates none.
pub struct HudState<'a> {
    /// The frame `Streamer::pump` returned. Taken by reference rather than copied
    /// field by field, so the HUD cannot drift from the thing it describes.
    pub frame: &'a voxel_work::Frame,
    pub adapter: &'a str,
    /// How many worker threads the mesher was given. Reported because it is a
    /// *perf knob* and never an input to the world — the digest beside it is the
    /// statement that says so.
    pub workers: usize,
    /// `None` until a reporting interval has completed. A frame rate is a
    /// **measurement over a period**, and before one has passed there is no
    /// measurement to report — `0 fps` would be a number the HUD made up.
    pub fps: Option<f64>,
    pub position_m: [f64; 3],
    /// Whether the body's feet are resting on something this frame.
    ///
    /// A readout rather than a detail: before the camera had a body, "where is the
    /// player" was a position and nothing else, so a player standing on the ground
    /// and one falling past it printed the *same three numbers*. This is the bit
    /// that says which.
    pub grounded: bool,
    /// The body's vertical speed in metres per second, signed: positive is rising.
    ///
    /// Signed, so a jump and a fall are distinguishable from a standing player by
    /// reading rather than by knowing which is which.
    pub vertical_m_s: f64,
    /// Whether the eye is inside a water voxel.
    ///
    /// Measured by the caller, which is the only place that can ask the world. The
    /// HUD renders state and does not go looking for it — the same discipline as
    /// taking the frame by reference so it cannot drift from the thing it describes.
    pub in_water: bool,
    pub quads: usize,
    pub seed: u64,
    pub tick: u64,
    /// A digest over the resident meshes. Order-independent by construction, so
    /// it is the same number at any worker count (F12).
    pub digest: Digest,
    /// The streamer's last refusal, if it has had one.
    pub refusal: Option<&'a Refusal>,
    /// What the appearance layer could and could not derive for the world's
    /// materials.
    pub shading: shading::ShadingReport,
}

/// How much of a digest the HUD shows. Sixteen hex characters is 64 bits of a
/// 256-bit digest: enough to see that two runs differ. The readout ends in `…`
/// because it is a prefix, and pretending sixteen characters is the digest would
/// be the sort of rounding this project refuses everywhere else.
pub const DIGEST_CHARS: usize = 16;

// **There is no `hud()` that places panels in four corners any more, and that is H-9.8's
// ruling rather than a deletion for tidiness.** The HUD *is* the pinned subset of the shell's
// window set (§1), so the four panels are hosted by four declared pinned windows and drawn by
// `crate::shell::hud::frame` — one window system, not two. The panel builders below are
// unchanged and still the only place a readout's ink, row height and note are decided.

/// Where the player is, what is around them, and how fast it is being drawn.
pub fn world_panel(state: &HudState<'_>) -> Panel {
    let f = state.frame;
    Panel::new(
        "world",
        Role::Chrome,
        "World",
        vec![
            Readout::measured(
                "position",
                format!(
                    "{:.0}, {:.0}, {:.0} m",
                    state.position_m[0], state.position_m[1], state.position_m[2]
                ),
            ),
            Readout::measured(
                "chunks",
                format!(
                    "{} / {} ({:.0}%)",
                    f.resident,
                    f.wanted,
                    f.progress() * 100.0
                ),
            ),
            Readout::measured("quads drawn", format!("{}", state.quads)),
            // The body's own state, which a position alone cannot express. A player
            // standing on the ground and one falling past it share a position.
            Readout::measured(
                "body",
                format!(
                    "{}, {:+.2} m/s",
                    if state.grounded {
                        "on the ground"
                    } else {
                        "airborne"
                    },
                    state.vertical_m_s
                ),
            ),
            Readout::measured(
                "water",
                if state.in_water { "submerged" } else { "dry" }.to_string(),
            ),
            match state.fps {
                Some(fps) => Readout::measured("frame rate", format!("{fps:.0} fps")),
                None => Readout::absent("frame rate", "no reporting interval has completed yet")
                    .expect("the note is not blank"),
            },
            Readout::measured("adapter", state.adapter.to_string()),
        ],
    )
}

/// The residency, the ceiling and the budget refusals.
pub fn streamer_panel(state: &HudState<'_>) -> Panel {
    let f = state.frame;
    let mb = |bytes: usize| bytes as f64 / (1024.0 * 1024.0);
    Panel::new(
        "streamer",
        Role::Chrome,
        "Streamer",
        vec![
            Readout::measured(
                "mesh",
                format!("{:.1} / {:.1} MB", mb(f.bytes), mb(f.ceiling)),
            ),
            Readout::measured("worker scratch", format!("{:.1} MB", mb(f.scratch_bytes))),
            Readout::measured("in flight", format!("{}", f.in_flight)),
            // The one number that has to stay at zero for "bounded" to be a
            // claim rather than a hope (`docs/LEDGER.md` C-29).
            Readout::measured("refused for budget", format!("{}", f.refused_for_budget)),
            Readout::measured("evicted / failed", format!("{} / {}", f.evicted, f.failed)),
            // Chunks on screen from a mesh the world has moved past. Non-zero
            // exactly while something rebuilds, and the direct read on the flash
            // this crate no longer has: those chunks are covered, not absent.
            Readout::measured("stale, still drawn", format!("{}", f.stale_drawn)),
        ],
    )
}

/// The readout that makes F12 visible: the world is a function of the seed and
/// the tick, and **not** of how many threads produced it.
pub fn determinism_panel(state: &HudState<'_>) -> Panel {
    let mut rows = vec![
        Readout::measured("seed", format!("{}", state.seed)),
        Readout::measured("tick", format!("{}", state.tick)),
        // The worker count is reported *beside* the digest so the pair reads as
        // one statement: this number is a performance knob, that number is the
        // world.
        Readout::measured("workers", format!("{} (a perf knob)", state.workers)),
    ];
    if state.frame.resident == 0 {
        // A digest over zero meshes is a real number and not a statement about
        // the world, so the row says why it has nothing rather than showing it.
        rows.push(
            Readout::absent("resident digest", "no chunk is resident yet")
                .expect("the note is not blank"),
        );
    } else {
        let hex = state.digest.to_hex();
        let shown: String = hex.chars().take(DIGEST_CHARS).collect();
        rows.push(Readout::measured("resident digest", format!("{shown}…")));
    }
    Panel::new("determinism", Role::Chrome, "Determinism", rows)
}

/// The **refusals** panel: what has been refused, and what could not be derived.
///
/// This panel is the one that matters most. `refuse, never default` is this
/// project's first rule, and a refusal that only reaches `stderr` in a windowed
/// program is a refusal nobody sees. It has **two** sources of bad news, and
/// both belong here because to a player they are the same thing: something the
/// world needed was not established.
///
/// * the streamer's last refusal, if it has had one;
/// * the appearance layer's, which is the one that explains a black world. The
///   layer refuses to invent a reflectance, and **every shading term that reaches
///   a block now carries that block's own reflectance** — the ambient, the
///   half-Lambert term and the rim alike (`npr::shade`) — so a material whose
///   record carries no colour has nothing for any of them to carry, and it is
///   drawn exactly black *because nothing in the record says what it reflects*.
///   That is a named absence and not a defect, which is why the panel says so
///   instead of leaving a player to guess.
///
/// `SurfaceAlt` rather than `Chrome`: MD1 sets the alternative surface above the
/// chrome one, and this is the one panel that is meant to be noticed.
pub fn refusal_panel(state: &HudState<'_>) -> Panel {
    let mut rows = Vec::new();
    if let Some(refusal) = state.refusal {
        rows.push(Readout::measured("kind", format!("{:?}", refusal.kind())));
        rows.push(Readout::measured("detail", refusal.detail().to_string()));
    }
    let shading = &state.shading;
    if shading.with_base_colour < shading.materials {
        rows.push(Readout::measured(
            "shadeable materials",
            format!("{} of {}", shading.with_base_colour, shading.materials),
        ));
    }

    if rows.is_empty() {
        return Panel::new("refusals", Role::SurfaceAlt, "Refusals", Vec::new())
            .with_note("Nothing has been refused this session.")
            .expect("the note is not blank");
    }

    let mut panel = Panel::new("refusals", Role::SurfaceAlt, "Refused", rows);
    if shading.with_base_colour == 0 {
        panel = panel
            .with_note("No material carries a reflectance, so only the rim is drawn.")
            .expect("the note is not blank");
    }
    panel
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// The vertex layout the panel pipeline expects: eleven floats, made of the clip
/// position, the linear colour, the offset from the rect's centre, the half
/// extent, and the corner radius. Everything but the position is in pixels.
const SOLID_FLOATS: usize = 11;
const SOLID_STRIDE: u32 = (SOLID_FLOATS * 4) as u32;
const BYTES_PER_VERTEX: usize = SOLID_FLOATS * 4;
/// Two triangles and no index buffer. A rect is 264 bytes; an index buffer would
/// cost more than it saves.
const VERTICES_PER_RECT: usize = 6;
/// How many rects the buffer starts able to hold. Four panels and their rules is
/// a little over twenty, so this is the round number above that and growth is the
/// fallback rather than the path.
const INITIAL_RECTS: usize = 32;
/// **One band of the paint**: the rectangles that come before the text that follows them.
///
/// A band is drawn in one pass — its rects, then its text — so a window's body covers the text of
/// the window behind it. A single text pass above every rect cannot: it is one layer, and every
/// glyph in it lands above every surface, covered or not.
#[derive(Clone, Debug, Default)]
struct Band {
    /// Vertex indices in [`HudRenderer::vertices`]: whole rectangles, six vertices each.
    rects: std::ops::Range<u32>,
    /// Indices into [`HudRenderer::placed`]: the runs drawn after them.
    texts: std::ops::Range<usize>,
    /// Which of [`HudRenderer::texts`] carries those runs, once they are prepared.
    renderer: Option<usize>,
}

/// **One run where a view put it**: the shaped text it draws, and the box, ink, alignment and
/// type size it was placed with.
///
/// The key says *what* is drawn; everything else says *where and how*, and all of it is compared
/// frame to frame to tell a settled frame from a changed one. One struct rather than parallel
/// vectors: a run and its placement cannot drift apart if they are the same value.
#[derive(Clone, PartialEq, Debug)]
struct Placed {
    key: RunKey,
    rect: Rect,
    paint: Paint,
    align: Align,
    size_px: f32,
}

/// Glyphon's state plus the panel pipeline: everything the HUD needs to draw.
pub struct HudRenderer {
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    solid: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    vertex_capacity_bytes: usize,
    /// **The shaped runs**, held by what they say rather than by where they were drawn: reshaping
    /// is the expensive half and it does not depend on placement. See [`crate::shaped`].
    runs: ShapedRuns,
    /// This frame's runs, in the order the elements emitted them.
    placed: Vec<Placed>,
    /// The vertices handed to the GPU last time a frame was prepared, so a frame that builds the
    /// same bytes is recognised rather than re-uploaded.
    uploaded: Vec<u8>,
    /// **One text renderer a band.** A band's glyphs have to live in their own vertex buffer:
    /// a run cannot be re-uploaded between draw calls, so the paint cannot take what one renderer
    /// holds and hand it back a band at a time.
    texts: Vec<glyphon::TextRenderer>,
    /// The paint's bands, back to front: each one's rectangles and then its text.
    bands: Vec<Band>,
    /// The dp-per-pixel ratio [`Self::prepare`] measured with, so a band's text areas are built in
    /// the same units.
    scale: f32,
    /// How many frames [`Self::prepare`] recognised as repeats of the one before it.
    ///
    /// **A measurement, not state**: the draw reads nothing from it. It exists because "a settled
    /// frame costs nothing" is a claim about a path, and a path nothing counts cannot be checked.
    settled_frames: u64,
    prepared_vertices: u32,
    width: u32,
    height: u32,
}

impl HudRenderer {
    /// Build the HUD's pipelines for one target `format`. The format is the
    /// swapchain's, or the offscreen test target's — the atlas is created for it
    /// so that glyph coverage and the target agree about sRGB.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let cache = glyphon::Cache::new(device);
        let viewport = glyphon::Viewport::new(device, &cache);
        let atlas = glyphon::TextAtlas::new(device, queue, &cache, format);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("panels.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("panels.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hud panels"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let solid = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud panels"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: SOLID_STRIDE as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 24,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 32,
                            shader_location: 3,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32,
                            offset: 40,
                            shader_location: 4,
                        },
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // The panel is translucent, so the world shows through it.
                    // The hardware linearises an `*Srgb` target before blending,
                    // which is the same model `voxel_ui::Rgb::blend_over` uses.
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let vertex_capacity_bytes = INITIAL_RECTS * VERTICES_PER_RECT * BYTES_PER_VERTEX;
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud vertices"),
            size: vertex_capacity_bytes as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        HudRenderer {
            font_system: glyphon::FontSystem::new(),
            swash_cache: glyphon::SwashCache::new(),
            viewport,
            atlas,
            texts: Vec::new(),
            solid,
            vertices,
            vertex_capacity_bytes,
            runs: ShapedRuns::default(),
            placed: Vec::new(),
            uploaded: Vec::new(),
            bands: Vec::new(),
            scale: 1.0,
            settled_frames: 0,
            prepared_vertices: 0,
            width: 1,
            height: 1,
        }
    }

    pub fn extent(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// **How many runs the last [`Self::prepare`] had to shape.** Not used to draw: a HUD that
    /// found no text at all and one that found all of it draw the same nothing, so the count is
    /// what tells a test whether the shaping saw anything.
    ///
    /// It is the count of shapes, not the count of runs on screen: a settled frame places every run
    /// it did last frame and shapes none of them, and this says so.
    pub fn shaped_runs(&self) -> usize {
        self.runs.misses()
    }

    /// **How many frames [`Self::prepare`] recognised as repeats of the frame before them.** The
    /// companion measurement to [`Self::shaped_runs`]: together they say whether the settled path is
    /// the one being taken, which is a claim about a path and therefore needs a witness.
    pub fn settled_frames(&self) -> u64 {
        self.settled_frames
    }

    /// Shape every text run and build every rect, for a viewport `extent_px`
    /// physical pixels at `scale` device-independent pixels per pixel.
    ///
    /// The layout is in **dp** and only this function knows about pixels, which is
    /// the property that lets a whole design system be tested without a display.
    ///
    /// **A frame that says what the last one said is not prepared again.** Only text that changed
    /// is re-shaped (see [`crate::shaped`]), so a frame costs what moved rather than what is on
    /// screen; and when nothing was shaped *and* every rect, every placement and the extent are
    /// the ones this renderer already holds, it returns without touching the GPU at all — the
    /// vertex buffer, the glyph vertices and the bands are still what this frame would produce.
    ///
    /// **Every band or none**, and that rule is what makes the reuse safe. `glyphon`'s atlas frees
    /// a glyph that was not used in the current epoch when a new one needs its room, so a frame
    /// that prepared *some* bands could let a fresh glyph evict one that a reused band still points
    /// at. Preparing every band when anything moved, and none when nothing did, leaves no band
    /// pointing at an atlas entry the atlas believes is free.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        extent_px: (u32, u32),
        scale: f32,
        elements: &[Element],
        strokes: &[Stroke],
    ) -> Result<(), ClientError> {
        let (width, height) = (extent_px.0.max(1), extent_px.1.max(1));
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let resized = (width, height) != (self.width, self.height);
        self.width = width;
        self.height = height;

        self.viewport
            .update(queue, glyphon::Resolution { width, height });

        // ---- the bands, in element order ---------------------------------
        //
        // **A band is a run of rectangles and the text that follows it**, drawn in that order before
        // the next band starts. That is the whole of the z-order: text is not a layer above the
        // interface, it is part of the window that emitted it, so a body drawn later covers it.
        let mut bytes: Vec<u8> = Vec::new();
        let mut bands: Vec<Band> = Vec::new();
        let mut placed: Vec<Placed> = Vec::new();
        self.runs.begin();
        let mut open: Option<Band> = None;
        for element in elements {
            match element {
                Element::Surface { rect, paint } | Element::Rule { rect, paint } => {
                    // **A rect after text is painted after it**, so it opens the next band.
                    if open.as_ref().is_some_and(|band| !band.texts.is_empty()) {
                        if let Some(band) = open.take() {
                            bands.push(band);
                        }
                    }
                    if open.is_none() {
                        let at = (bytes.len() / BYTES_PER_VERTEX) as u32;
                        open = Some(Band {
                            rects: at..at,
                            texts: placed.len()..placed.len(),
                            renderer: None,
                        });
                    }
                    let radius = match element {
                        Element::Surface { .. } => radius_dp(),
                        _ => 0.0,
                    };
                    push_rect(&mut bytes, *rect, *paint, radius, scale, width, height);
                    if let Some(band) = open.as_mut() {
                        band.rects.end = (bytes.len() / BYTES_PER_VERTEX) as u32;
                    }
                }
                Element::Text { .. } => {
                    let at = (bytes.len() / BYTES_PER_VERTEX) as u32;
                    if let Some(index) = self.place_run(element, &mut placed, scale) {
                        match open.as_mut() {
                            Some(band) => band.texts.end = index + 1,
                            None => {
                                open = Some(Band {
                                    rects: at..at,
                                    texts: index..index + 1,
                                    renderer: None,
                                })
                            }
                        }
                    }
                }
            }
        }
        if let Some(band) = open.take() {
            bands.push(band);
        }
        // ---- the vector overlay ------------------------------------------
        // Strokes are drawn on top of the shell's own rects, in the order the view emitted them
        // (the change view emits in record order). **Not [`Element`]s**: see the module docs, and
        // they are their own band because nothing in the shell follows them.
        let first = (bytes.len() / BYTES_PER_VERTEX) as u32;
        for stroke in strokes {
            push_segment(&mut bytes, stroke, scale, width, height);
        }
        let after = (bytes.len() / BYTES_PER_VERTEX) as u32;
        if after > first {
            bands.push(Band {
                rects: first..after,
                texts: placed.len()..placed.len(),
                renderer: None,
            });
        }
        self.prepared_vertices = after;

        // ---- a frame that says nothing new -------------------------------
        //
        // Everything the draw depends on, and nothing else: the vertex bytes, the placement of
        // every run, the band partition, the scale it was all measured at, and the extent. If all
        // of it equals what was last prepared, this frame's picture is last frame's picture.
        let settled = !resized
            && self.runs.misses() == 0
            && scale == self.scale
            && bytes == self.uploaded
            && placed == self.placed
            && same_partition(&bands, &self.bands);
        if settled {
            self.settled_frames += 1;
            self.runs.settle();
            return Ok(());
        }

        if bytes.len() > self.vertex_capacity_bytes {
            // Grow once and keep it. A window resize changes the HUD's size; a
            // counter changing does not.
            self.vertex_capacity_bytes = bytes.len().next_power_of_two();
            self.vertices = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("hud vertices"),
                size: self.vertex_capacity_bytes as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            // The new buffer is empty: what was uploaded is gone with the old one.
            self.uploaded.clear();
        }
        if !bytes.is_empty() && bytes != self.uploaded {
            queue.write_buffer(&self.vertices, 0, &bytes);
        }
        self.uploaded = bytes;

        // ---- the glyphs, one renderer a band ------------------------------
        //
        // **Prepared before the paint starts, not during it.** A glyph run is uploaded into its
        // renderer's vertex buffer, which cannot happen inside a render pass — so each band that
        // has text gets its own renderer here, and the paint then draws them in band order.
        let mut renderer = 0usize;
        for band in bands.iter_mut() {
            if band.texts.is_empty() {
                continue;
            }
            if self.texts.len() <= renderer {
                self.texts.push(glyphon::TextRenderer::new(
                    &mut self.atlas,
                    device,
                    wgpu::MultisampleState::default(),
                    // No depth. A HUD draws in order, on top of the world.
                    None,
                ));
            }
            let areas = text_areas(&self.runs, &placed, band.texts.clone(), scale);
            self.texts[renderer]
                .prepare(
                    device,
                    queue,
                    &mut self.font_system,
                    &mut self.atlas,
                    &self.viewport,
                    areas,
                    &mut self.swash_cache,
                )
                .map_err(|e| ClientError::TextRender {
                    detail: e.to_string(),
                })?;
            band.renderer = Some(renderer);
            renderer += 1;
        }
        self.bands = bands;
        self.placed = placed;
        self.scale = scale;
        self.runs.settle();
        Ok(())
    }

    /// **One text run, shaped and placed.** The buffer belongs to the [`ShapedRuns`] store, keyed by
    /// what was said and at what size, so a run that only moved is not shaped again; this records
    /// where and how it is drawn, which is the half that may change every frame for free.
    ///
    /// `None` for an element that is not text; the caller is iterating a sequence that can hold
    /// both kinds.
    fn place_run(
        &mut self,
        element: &Element,
        placed: &mut Vec<Placed>,
        scale: f32,
    ) -> Option<usize> {
        let Element::Text {
            rect,
            text,
            paint,
            size_dp,
            align,
        } = element
        else {
            return None;
        };
        let size_px = (*size_dp as f32) * scale;
        let key = RunKey::of(text, size_px, (rect.width_dp as f32) * scale);
        self.runs.ensure(&mut self.font_system, &key);
        placed.push(Placed {
            key,
            rect: *rect,
            paint: *paint,
            align: *align,
            size_px,
        });
        Some(placed.len() - 1)
    }

    /// Draw the prepared HUD into `target`, on top of whatever is already there.
    ///
    /// **In band order, in one pass**: a band's rects, then that band's text, then the next band's
    /// rects — which is what keeps a covered window's text under the window covering it.
    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hud"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                // **Load**, not clear: the world is already in this attachment.
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        for band in &self.bands {
            if !band.rects.is_empty() {
                pass.set_pipeline(&self.solid);
                pass.set_vertex_buffer(0, self.vertices.slice(..));
                pass.draw(band.rects.clone(), 0..1);
            }
            if let Some(renderer) = band.renderer {
                if let Err(e) = self.texts[renderer].render(&self.atlas, &self.viewport, &mut pass)
                {
                    // Nothing to draw on, but an event loop must not die inside a render pass. The
                    // panic-free path is to draw what is there and say so.
                    eprintln!("shploingus: the HUD's text pass failed: {e}");
                }
            }
        }
    }

    /// Trim the glyph atlas. Called once a frame by the window, because an atlas
    /// that only grows is a leak with a nice name.
    pub fn trim(&mut self) {
        self.atlas.trim();
    }
}

/// The text areas of one band, in the units [`HudRenderer::prepare`] measured with.
///
/// One area per [`Placed`] run: the shaped layout comes from the [`ShapedRuns`] store by key, and
/// the position, ink and alignment come from the placement. Keeping them apart is what lets a run
/// be shaped once and placed anywhere.
///
/// Right alignment is computed here because a shaping engine does not do it: it is the run's own
/// measured width that has to be subtracted, not the box's. The rect is sized to the type and the
/// line box is taller, so the box is pulled up by half the difference — that centres the glyphs in
/// the rect the layout gave them, which is what makes the layout's 48 dp row mean anything to a
/// reader.
fn text_areas<'a>(
    runs: &'a ShapedRuns,
    placed: &[Placed],
    texts: std::ops::Range<usize>,
    scale: f32,
) -> Vec<glyphon::TextArea<'a>> {
    texts
        .map(|index| {
            let run = &placed[index];
            let buffer = runs.get(&run.key);
            let (rect, paint, align, size_px) = (run.rect, run.paint, run.align, run.size_px);
            let line_w = buffer
                .layout_runs()
                .next()
                .map(|run| run.line_w)
                .unwrap_or(0.0);
            let left = match align {
                Align::Start => (rect.x as f32) * scale,
                Align::End => ((rect.x + rect.width_dp) as f32) * scale - line_w,
            };
            let top = (rect.y as f32) * scale - (size_px * LINE_BOX - size_px) / 2.0;
            glyphon::TextArea {
                buffer,
                left,
                top,
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: (rect.x as f32 * scale) as i32,
                    top: ((rect.y as f32) * scale) as i32,
                    right: (((rect.x + rect.width_dp) as f32) * scale).ceil() as i32,
                    bottom: (((rect.y + rect.height_dp) as f32) * scale).ceil() as i32,
                },
                default_color: glyphon::Color::rgba(
                    paint.rgb.r,
                    paint.rgb.g,
                    paint.rgb.b,
                    (255.0 * paint.alpha.clamp(0.0, 1.0)) as u8,
                ),
                custom_glyphs: &[],
            }
        })
        .collect()
}

/// **Whether two band lists cut the frame into the same layers**, ignoring which renderer each
/// band was given.
///
/// The partition is part of what a frame draws — a rect before a run is under it, the same rect
/// after it covers it — and it is not recoverable from the vertex bytes, so it is compared. The
/// renderers are what a prepare *assigns*, so comparing them would compare this frame against the
/// work it is trying not to redo.
fn same_partition(left: &[Band], right: &[Band]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.rects == right.rects && left.texts == right.texts)
}

/// Append one rectangle as two triangles, in the layout [`SOLID_FLOATS`]
/// describes.
/// Append one stroke as two triangles, in the same layout [`push_rect`] writes — and **in the
/// segment's own frame**: the quad's corners are the segment's two ends swept by half its width,
/// and each vertex carries its offset from the segment's centre *and* the segment's half extents,
/// so the shader's rounded-rect coverage becomes a stadium cap. A degenerate segment (no length,
/// or no width) writes nothing: there is no mark to make.
fn push_segment(bytes: &mut Vec<u8>, stroke: &Stroke, scale: f32, width: u32, height: u32) {
    let [ax, ay] = [
        stroke.from_dp[0] as f32 * scale,
        stroke.from_dp[1] as f32 * scale,
    ];
    let [bx, by] = [
        stroke.to_dp[0] as f32 * scale,
        stroke.to_dp[1] as f32 * scale,
    ];
    let (dx, dy) = (bx - ax, by - ay);
    let length = (dx * dx + dy * dy).sqrt();
    let half_width = (stroke.width_dp as f32) * scale / 2.0;
    // **Finite and positive**, refused rather than drawn: a NaN or a zero-length segment is not a
    // mark, and a quad built from one would be a stripe with no direction.
    if !length.is_finite() || length <= 0.0 || !half_width.is_finite() || half_width <= 0.0 {
        return;
    }
    let (ux, uy) = (dx / length, dy / length);
    let (vx, vy) = (-uy, ux);
    let (cx, cy) = ((ax + bx) / 2.0, (ay + by) / 2.0);
    let half = [length / 2.0, half_width];
    let [r, g, b] = stroke.paint.rgb.linear();
    let colour = [r as f32, g as f32, b as f32, stroke.paint.alpha as f32];
    let clip = |px: f32, py: f32| {
        [
            px / width as f32 * 2.0 - 1.0,
            1.0 - py / height as f32 * 2.0,
        ]
    };
    // (offset along the segment, offset across it) → position, in the segment's own frame.
    let corners = [
        (-half[0], -half[1]),
        (half[0], -half[1]),
        (half[0], half[1]),
        (-half[0], -half[1]),
        (half[0], half[1]),
        (-half[0], half[1]),
    ];
    for (along, across) in corners {
        let px = cx + ux * along + vx * across;
        let py = cy + uy * along + vy * across;
        for value in clip(px, py)
            .into_iter()
            .chain(colour)
            .chain([along, across])
            .chain(half)
            .chain([half_width])
        {
            let bits = value.to_bits();
            bytes.extend_from_slice(&[
                bits as u8,
                (bits >> 8) as u8,
                (bits >> 16) as u8,
                (bits >> 24) as u8,
            ]);
        }
    }
}

fn push_rect(
    bytes: &mut Vec<u8>,
    rect: Rect,
    paint: Paint,
    radius_dp: f64,
    scale: f32,
    width: u32,
    height: u32,
) {
    let x = (rect.x as f32) * scale;
    let y = (rect.y as f32) * scale;
    let w = (rect.width_dp as f32) * scale;
    let h = (rect.height_dp as f32) * scale;
    let [r, g, b] = paint.rgb.linear();
    let colour = [r as f32, g as f32, b as f32, paint.alpha as f32];
    let radius = (radius_dp as f32) * scale;
    let half = [w / 2.0, h / 2.0];
    let clip = |px: f32, py: f32| {
        [
            px / width as f32 * 2.0 - 1.0,
            1.0 - py / height as f32 * 2.0,
        ]
    };

    let corners = [
        (x, y, -half[0], -half[1]),
        (x + w, y, half[0], -half[1]),
        (x + w, y + h, half[0], half[1]),
        (x, y, -half[0], -half[1]),
        (x + w, y + h, half[0], half[1]),
        (x, y + h, -half[0], half[1]),
    ];
    for (px, py, lx, ly) in corners {
        for value in clip(px, py)
            .into_iter()
            .chain(colour)
            .chain([lx, ly])
            .chain(half)
            .chain([radius])
        {
            let bits = value.to_bits();
            bytes.extend_from_slice(&[
                bits as u8,
                (bits >> 8) as u8,
                (bits >> 16) as u8,
                (bits >> 24) as u8,
            ]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_ui::{panel_opacity, Palette, Rgb};

    fn frame(resident: usize) -> voxel_work::Frame {
        voxel_work::Frame {
            wanted: 605,
            resident,
            bytes: 2 * 1024 * 1024,
            ceiling: 256 * 1024 * 1024,
            scratch_bytes: 72 * 1024 * 1024,
            in_flight: 14,
            ..Default::default()
        }
    }

    fn state<'a>(frame: &'a voxel_work::Frame, refusal: Option<&'a Refusal>) -> HudState<'a> {
        HudState {
            frame,
            adapter: "NVIDIA GeForce RTX 2070 SUPER (Vulkan)",
            workers: 15,
            fps: Some(61.5),
            position_m: [0.0, 45.0, -160.0],
            grounded: true,
            vertical_m_s: 0.0,
            in_water: false,
            quads: 21_526,
            seed: 2468,
            tick: 0,
            digest: Digest::of(b"the resident world"),
            refusal,
            shading: shading::ShadingReport {
                materials: 7,
                with_base_colour: 0,
            },
        }
    }

    #[test]
    fn a_stroke_is_a_quad_in_the_segment_s_own_frame() {
        // Why `Stroke` is not an `Element`: this quad is rotated, and no axis-aligned rect can be
        // it. Two triangles in the same layout the rects use, and each vertex carries the
        // segment's own frame, which is what the shader's coverage needs to make a stadium.
        let stroke = Stroke {
            from_dp: [0.0, 0.0],
            to_dp: [10.0, 0.0],
            width_dp: 2.0,
            paint: Paint {
                rgb: WHITE,
                alpha: 1.0,
            },
        };
        let mut bytes = Vec::new();
        push_segment(&mut bytes, &stroke, 1.0, 100, 100);
        assert_eq!(bytes.len(), 6 * BYTES_PER_VERTEX, "two triangles");
        let floats: Vec<f32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| {
                f32::from_bits(
                    b[0] as u32
                        | ((b[1] as u32) << 8)
                        | ((b[2] as u32) << 16)
                        | ((b[3] as u32) << 24),
                )
            })
            .collect();
        let at = |vertex: usize, field: usize| floats[vertex * SOLID_FLOATS + field];
        // Position: the quad's ends are the segment's ends, so x spans 0..10 dp.
        assert!(
            (at(0, 0) + 1.0).abs() < 1e-5,
            "the first corner is at x = 0 dp"
        );
        assert!(
            (at(2, 0) + 0.8).abs() < 1e-5,
            "and the far side at x = 10 dp"
        );
        // The local frame: ±5 dp along the segment, ±1 dp across it.
        assert_eq!(at(0, 6), -5.0);
        assert_eq!(at(1, 6), 5.0);
        assert_eq!(at(0, 7), -1.0);
        assert_eq!(at(2, 7), 1.0);
        // Half extents and radius are the stadium's, in pixels.
        assert_eq!(at(0, 8), 5.0);
        assert_eq!(at(0, 9), 1.0);
        assert_eq!(at(0, 10), 1.0);
        // A degenerate segment writes nothing rather than a dot of zero area.
        let mut none = Vec::new();
        push_segment(
            &mut none,
            &Stroke {
                from_dp: [3.0, 3.0],
                to_dp: [3.0, 3.0],
                ..stroke
            },
            1.0,
            100,
            100,
        );
        assert!(none.is_empty(), "a segment with no length draws nothing");
    }

    #[test]
    fn the_refusal_panel_is_a_note_when_there_is_nothing_to_report() {
        let f = frame(605);
        let mut quiet = state(&f, None);
        // The world shades, so there is genuinely nothing to say.
        quiet.shading = shading::ShadingReport {
            materials: 7,
            with_base_colour: 7,
        };
        let none = refusal_panel(&quiet);
        assert!(none.is_empty(), "an empty view is empty, and says so");
        assert!(none.validate().is_empty());
        assert!(none.note.is_some());
        assert!(none
            .note
            .as_deref()
            .unwrap()
            .contains("Nothing has been refused"));

        let refusal = Refusal::owned(voxel_model::RefusalKind::UnknownComponent, "no such shaper");
        let some = refusal_panel(&state(&f, Some(&refusal)));
        assert!(!some.is_empty());
        assert_eq!(some.rows.len(), 3, "kind, detail, and the shading count");
        assert!(some.rows[1].value.contains("no such shaper"));
        // A refusal is the thing to look at, so it gets the alternative surface.
        assert_eq!(some.role, Role::SurfaceAlt);
    }

    #[test]
    fn a_world_with_no_reflectance_says_why_it_is_black() {
        // The regression guard for the finding the user made: a black world is a
        // **stated** state, not a mystery. `0 of 7` plus a sentence, on screen.
        let f = frame(605);
        let panel = refusal_panel(&state(&f, None));
        assert_eq!(panel.role, Role::SurfaceAlt);
        let row = panel
            .rows
            .iter()
            .find(|r| r.label == "shadeable materials")
            .expect("the count is a row");
        assert_eq!(row.value, "0 of 7");
        let note = panel.note.as_deref().expect("and the reason is a note");
        assert!(note.contains("reflectance"), "{note}");
        // A note is a whole row, so the panel is one row taller than its rows.
        assert_eq!(panel.drawn_rows(), panel.rows.len() + 1);
        // And it is not the empty view, so the note is secondary text rather
        // than the empty state's ink.
        assert!(panel.validate().is_empty());
    }

    #[test]
    fn a_partly_shading_world_reports_the_count_without_the_note() {
        // Something is shadeable, so the world is visible; the count is still
        // worth reporting, and a sentence about a black world would be wrong.
        let f = frame(605);
        let mut st = state(&f, None);
        st.shading = shading::ShadingReport {
            materials: 7,
            with_base_colour: 3,
        };
        let panel = refusal_panel(&st);
        assert_eq!(panel.rows.len(), 1);
        assert_eq!(panel.rows[0].value, "3 of 7");
        assert!(panel.note.is_none());
    }

    #[test]
    fn the_digest_readout_is_absent_rather_than_wrong_when_nothing_is_resident() {
        let empty = frame(0);
        let panel = determinism_panel(&state(&empty, None));
        let row = panel
            .rows
            .iter()
            .find(|r| r.label == "resident digest")
            .expect("the row exists either way");
        assert_eq!(row.state, voxel_ui::State::Empty);
        assert!(row.value.contains("no chunk is resident yet"));

        let full = frame(605);
        let panel = determinism_panel(&state(&full, None));
        let row = panel
            .rows
            .iter()
            .find(|r| r.label == "resident digest")
            .unwrap();
        let hex = Digest::of(b"the resident world").to_hex();
        assert_eq!(row.value, format!("{}…", &hex[..DIGEST_CHARS]));
        assert_ne!(row.value, hex, "the readout says it is a prefix");
    }

    #[test]
    fn the_worker_count_is_reported_beside_the_digest_as_a_knob() {
        let f = frame(605);
        let panel = determinism_panel(&state(&f, None));
        let workers = panel.rows.iter().find(|r| r.label == "workers").unwrap();
        assert!(workers.value.contains("perf knob"), "{}", workers.value);
        // And the digest is in the same panel, so the two read together.
        assert!(panel.rows.iter().any(|r| r.label == "resident digest"));
    }

    #[test]
    fn the_panel_geometry_is_the_grid_the_design_system_declares() {
        let panel = world_panel(&state(&frame(605), None));
        let spacing = 8.0;
        let row = 48.0;
        let rule = 1.0;
        let rows = panel.rows.len() as f64;
        assert_eq!(
            panel.height_dp(),
            2.0 * spacing + row + rule + rows * row + (rows - 1.0) * rule
        );
    }

    // -----------------------------------------------------------------------
    // The GPU measurement
    // -----------------------------------------------------------------------

    /// An offscreen target in an sRGB format — the format a swapchain is most
    /// likely to offer, and the one `glyphon`'s atlas wants.
    const TEST_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    struct Target {
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        readback: wgpu::Buffer,
        width: u32,
        height: u32,
        padded_row_bytes: u32,
    }

    fn target(device: &wgpu::Device, width: u32, height: u32) -> Target {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hud target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TEST_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let unpadded = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row_bytes = unpadded.div_ceil(align) * align;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud readback"),
            size: padded_row_bytes as u64 * height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Target {
            texture,
            view,
            readback,
            width,
            height,
            padded_row_bytes,
        }
    }

    fn read_back(device: &wgpu::Device, target: &Target) -> Vec<[u8; 4]> {
        let slice = target.readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("poll");
        receiver
            .recv()
            .expect("the map callback runs")
            .expect("the map succeeds");
        let data = slice.get_mapped_range().expect("the range is mapped");
        let stride = target.padded_row_bytes as usize;
        let mut out = Vec::with_capacity((target.width * target.height) as usize);
        for y in 0..target.height as usize {
            for x in 0..target.width as usize {
                let i = y * stride + x * 4;
                out.push([data[i], data[i + 1], data[i + 2], data[i + 3]]);
            }
        }
        drop(data);
        target.readback.unmap();
        out
    }

    fn at(pixels: &[[u8; 4]], width: u32, x: u32, y: u32) -> Rgb {
        let p = pixels[(y * width + x) as usize];
        Rgb::new(p[0], p[1], p[2])
    }

    #[test]
    fn the_shell_chrome_is_legible_over_the_worst_backdrop() {
        // The design system's contrast floor, checked on pixels a GPU actually
        // produced — and checked on the **shell's** chrome rather than on four
        // corner panels, because H-9.8 replaced the corner HUD with pinned
        // windows. The backdrop is **white**, the brightest a swapchain can be,
        // and the bar is translucent, so this is the drawn thing at its worst.
        let (device, queue, adapter) = crate::renderer::offscreen_device().expect("a GPU");
        println!("adapter: {adapter}");

        let (width, height) = (1280u32, 720u32);
        let target = target(&device, width, height);
        let mut hud_renderer = HudRenderer::new(&device, &queue, TEST_FORMAT);

        let palette = Palette::declared();
        let f = frame(605);
        // Scale 1.0, so one dp is one pixel and a layout rect is a pixel rect.
        // Stated rather than assumed: the assertions below index pixels with
        // layout coordinates.
        let scale = 1.0f32;
        let mut mount = {
            let package = voxel_package::recipe::compile(MEASURE_RECIPE.as_bytes())
                .expect("the measure recipe compiles");
            let loaded = voxel_package::recipe::load(&package).expect("the measure package loads");
            let mut mount = crate::shell::Mount::new(&loaded).expect("the measure package mounts");
            mount.set_viewport(Rect::new(0.0, 0.0, width as f64, height as f64));
            mount
        };
        let views =
            crate::shell::hud::views(&state(&f, None), mount.console(), mount.api_view().cloned());
        let viewport = mount.viewport();
        let elements = crate::shell::hud::frame(&mut mount, &palette, viewport, &views)
            .expect("the shell layer lays out");
        hud_renderer
            .prepare(&device, &queue, (width, height), scale, &elements, &[])
            .expect("the shell prepares");
        // **A measured guard, re-measured.** The pinned chrome this fixture lays out
        // carries **15** text runs (the world window's title, its close glyph, the
        // pinned/roll glyphs, the panel title, four readouts and their values, the
        // held-rows note, and the Mods tab). The old `> 20` predates H-9.8's pinned
        // windows and was never re-measured against them; the guard's job is to catch
        // a chrome that has lost its text, which is what the count is for — the
        // assertion's subject is the contrast below, not a number.
        assert!(
            hud_renderer.shaped_runs() > 10,
            "the shell's chrome is text and glyphs: only {} runs were shaped out of {} elements",
            hud_renderer.shaped_runs(),
            elements.len()
        );
        assert!(
            hud_renderer.prepared_vertices > 0,
            "the shell's chrome is rects"
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("hud test"),
        });
        {
            // A white world: the worst case the HUD is designed against.
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("backdrop"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        hud_renderer.draw(&mut encoder, &target.view);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &target.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(target.padded_row_bytes),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        let pixels = read_back(&device, &target);

        // A pixel outside every panel is untouched white.
        assert_eq!(
            at(&pixels, width, width - 2, height / 2),
            WHITE,
            "the world shows through"
        );

        // The bar's own colour: the most common colour inside the strip, which is
        // the surface every tab's glyph sits on.
        let panel = Rect::new(0.0, 0.0, width as f64, voxel_ui::bar_height_dp());
        let inside: Vec<Rgb> = (panel.y as u32..panel.bottom() as u32)
            .flat_map(|y| (panel.x as u32..panel.right() as u32).map(move |x| (x, y)))
            .map(|(x, y)| at(&pixels, width, x, y))
            .collect();
        assert!(!inside.is_empty());
        let surface = most_common(&inside);

        // The picture is the design: the panel really is chrome at the declared
        // opacity over white, and the glyphs really are white ink on it.
        let expected = palette
            .get(Role::Chrome)
            .expect("chrome is declared")
            .blend_over(WHITE, panel_opacity());
        println!("panel drawn: {surface:?}, designed: {expected:?}");
        for (drawn, want) in [
            (surface.r, expected.r),
            (surface.g, expected.g),
            (surface.b, expected.b),
        ] {
            assert!(
                (drawn as i32 - want as i32).abs() <= 2,
                "the drawn panel is {surface:?} and the design says {expected:?}"
            );
        }
        assert!(
            surface.luminance() < 0.5,
            "the panel must darken the white world: {surface:?}"
        );

        // The measurement the design system's floor is about: the **brightest**
        // glyph pixel inside the panel, against the panel it sits on.
        let brightest = inside
            .iter()
            .copied()
            .max_by(|a, b| a.luminance().partial_cmp(&b.luminance()).expect("finite"))
            .expect("non-empty");
        let contrast = brightest.contrast(surface);
        println!("brightest glyph pixel: {brightest:?}, contrast {contrast:.2}:1");
        assert!(
            contrast >= 4.5,
            "the HUD's own text is under the WCAG floor on real pixels: {contrast:.2}:1"
        );

        // And there really are glyphs, rather than one white rectangle.
        let distinct: std::collections::BTreeSet<[u8; 3]> =
            inside.iter().map(|c| [c.r, c.g, c.b]).collect();
        println!("{} distinct colours inside the panel", distinct.len());
        assert!(
            distinct.len() > 8,
            "only {} colours — is any text being drawn?",
            distinct.len()
        );
    }

    /// The smallest package that can measure the shell's chrome: one pinned window
    /// (so the pinned layer is exercised) and one tab (so the bar carries a label).
    const MEASURE_RECIPE: &str = "shploingus.recipe/3\n\
package\tmeasure\tv0\n\
window\tworld\n\
\ttitle\tWorld\n\
\tcontent\tworld-view\n\
\tlane\tworld\n\
\textent\tsquare-pixels\n\
\tgeometry\t16 48 336 360\n\
\tmin-size\t240 192\n\
\topacity\t930\n\
\televation\t0\n\
\ttheme-role\thud\n\
\tpinnable\ttrue\n\
\tlayout\tclose,title-position\n\
\tbehavior\tdrag\n\
\trecord-id\twindow-world\n\
window\tmods\n\
\ttitle\tMods\n\
\tcontent\tmods\n\
\tlane\ttab\n\
\textent\tsquare-pixels\n\
\tgeometry\t400 96 480 384\n\
\tmin-size\t240 192\n\
\topacity\t930\n\
\televation\t1\n\
\ttheme-role\tsurface\n\
\tpinnable\tfalse\n\
\tlayout\tclose,title-position\n\
\tbehavior\tdrag\n\
\trecord-id\twindow-mods\n\
tab\tmods\n\
pin\tworld\n";

    fn most_common(colours: &[Rgb]) -> Rgb {
        let mut counts: std::collections::BTreeMap<(u8, u8, u8), usize> = Default::default();
        for c in colours {
            *counts.entry((c.r, c.g, c.b)).or_default() += 1;
        }
        let ((r, g, b), _) = counts.iter().max_by_key(|(_, n)| **n).expect("non-empty");
        Rgb::new(*r, *g, *b)
    }
}
