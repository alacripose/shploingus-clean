//! **The map window: one camera, one world scale, and the player's place in it.**
//!
//! [`MapCamera`] is the whole view: where it looks (a world centre), how big a metre is
//! (`pixels_per_m`), which way is up the screen (`bearing_deg`) and how far down it looks
//! (`pitch_deg`). Every pixel of the map comes from that one value — the window's body, the pan a
//! drag makes, the anchor a wheel keeps still, the silhouette the ground is drawn against, and the
//! marks. **A resize changes how much world is revealed, never the scale**: the same camera draws
//! the same metre at the same pixel size in a 480 dp window and a maximised one.
//!
//! # How the picture is made
//!
//! **The ground is the world's own where the streamer holds it**, and the laws' field everywhere
//! else ([`crate::map_survey`]). This module owns the camera, the marks and the raster: the survey
//! — where its surface came from and which world it is — is `map_survey`'s.
//!
//! The ground is drawn **by the column**, which is the 2D cell renderer's own path: for each screen
//! column the picture is walked from the near edge to the far edge, the terrain's height is sampled
//! by inverting the camera at that row, and the rows the surface covers become runs of colour. A
//! run is one rectangle, so a hill's worth of pixels costs a handful of elements rather than one
//! each, and the ground is smooth because every run is sampled from the interpolated survey rather
//! than from a cell.
//!
//! **Sky is what the ground does not cover.** The survey is finite, so beyond its edge the column
//! shows sky above the last terrain and the void colour below it — the honest reading of "no data
//! here", rather than a stretched copy of the edge or a flat band.
//!
//! **Everything on the map is a mark, drawn through the same camera**: the player's glyph and its
//! cuboid frame, the heading arrow (the nearest of eight lobes to the player's bearing), the
//! selected cell, the route between them, and the edge indicators for a player or selection the
//! view cannot show.

use voxel_shapers::ecology::Biome;
use voxel_shapers::WorldSpec;
use voxel_ui::{Align, Element, Paint, Rect, Rgb};

/// **Samples per declared cell, in each direction.** The frame declares 32 m cells, which is the
/// right grid for a *selection*; the picture is sampled four times as finely and interpolated, so
/// the surface is the height field's own slope rather than a quilt of its cell corners.
pub const SUB: usize = 4;

/// **Cells across a survey.** A frame's extent decides the grid; these are the ceiling it clamps to.
pub const MAX_COLUMNS: usize = 192;
/// **Cells down a survey.**
pub const MAX_ROWS: usize = 192;
/// **Cells across the climate grid.** A climate column costs about ten thousand height samples, so
/// the biomes are read on the frame's own cells and the colour between them is blended — and when a
/// survey reaches further than that grid can describe, the grid coarsens with it.
pub const MAX_BIOME: usize = 64;

/// **The width of a raster column**, in device-independent pixels — the 2D cell renderer's own
/// column, and the unit the map is drawn in.
pub const COLUMN_DP: f64 = 4.0;
/// **How finely a column is walked**, in device-independent pixels. Half a pixel would be a sample
/// per pixel for no visible gain once runs are merged.
pub const ROW_STEP_DP: f64 = 1.5;
/// **Colour quantisation for run merging**: channels are rounded to this step before two rows are
/// called the same run, so a gradient still merges and a biome boundary still splits.
pub const RUN_STEP: u8 = 12;
/// **How far a run overlaps its neighbour's column**, in device-independent pixels — half of one
/// [`COLUMN_DP`]. Two rects that merely abut leave a seam wherever a renderer rounds their edges,
/// and a seam in the ground is the sky showing through it, which is exactly what a dotted field of
/// pale dashes in the water turned out to be.
pub const RUN_OVERLAP_DP: f64 = 0.5;

/// **The player glyph, its cuboid frame and the edge indicators.**
pub const PLAYER_GLYPH: char = '@';
pub const FRAME: [&str; 4] = ["┌", "┐", "└", "┘"];
pub const PLAYER_EDGE: char = '◆';
pub const SELECTION_EDGE: char = '◇';
pub const SELECTION_GLYPH: char = '◇';
/// The player's mark and the plate behind it, so the glyph reads on bright ground.
pub const PLAYER_INK: Rgb = Rgb::new(255, 255, 255);
pub const MARKER_BACKDROP: Rgb = Rgb::new(12, 14, 16);
/// The route's ink and the selected cell's.
pub const ROUTE_INK: Rgb = Rgb::new(96, 212, 255);
pub const ROUTE_HEAD_INK: Rgb = Rgb::new(255, 236, 150);
/// The offscreen indicator's ink.
pub const INDICATOR_INK: Rgb = Rgb::new(255, 120, 96);
/// The cursor's outline.
pub const CURSOR_INK: Rgb = Rgb::new(255, 255, 255);

/// **The map's own colours.** A biome is a fact about the ground, and the palette has no role for
/// "boreal forest".
pub const WATER_DEEP: Rgb = Rgb::new(26, 52, 84);
pub const WATER_SHALLOW: Rgb = Rgb::new(44, 84, 120);
pub const VOID: Rgb = Rgb::new(16, 20, 28);
pub const ALPINE: Rgb = Rgb::new(146, 146, 154);
pub const TUNDRA: Rgb = Rgb::new(154, 162, 146);
pub const BOREAL: Rgb = Rgb::new(66, 96, 70);
pub const TEMPERATE: Rgb = Rgb::new(92, 124, 76);
pub const COOL_TEMPERATE: Rgb = Rgb::new(82, 116, 74);
pub const GRASSLAND: Rgb = Rgb::new(122, 142, 82);
pub const DESERT: Rgb = Rgb::new(184, 158, 102);
pub const SCRUB: Rgb = Rgb::new(146, 136, 82);
pub const TROPICAL_DRY: Rgb = Rgb::new(128, 122, 70);
pub const RAINFOREST: Rgb = Rgb::new(50, 106, 60);
pub const POLAR_DESERT: Rgb = Rgb::new(198, 202, 204);
/// The sky above the survey, high and at the horizon.
pub const SKY_HIGH: Rgb = Rgb::new(92, 138, 186);
pub const SKY_LOW: Rgb = Rgb::new(176, 206, 232);

/// **The eight cells a heading rounds to**, clockwise from a bearing looking north (−Z).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrow {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Arrow {
    /// The lobes in compass order, which is also the order [`arrow_of_degrees`] rounds into.
    pub const ALL: [Arrow; 8] = [
        Arrow::North,
        Arrow::NorthEast,
        Arrow::East,
        Arrow::SouthEast,
        Arrow::South,
        Arrow::SouthWest,
        Arrow::West,
        Arrow::NorthWest,
    ];

    /// The glyphs the map draws in place of a rotated arrow: one per lobe, the head pointing the
    /// way the bearing points on screen.
    pub const GLYPH: [&'static str; 8] = ["▲", "◥", "▶", "◢", "▼", "◣", "◀", "◤"];

    /// The arrow's index in [`Arrow::ALL`].
    pub fn lobe(self) -> usize {
        Arrow::ALL
            .iter()
            .position(|lobe| *lobe == self)
            .unwrap_or(0)
    }

    /// The glyph for a lobe.
    pub fn glyph(self) -> &'static str {
        Arrow::GLYPH[self.lobe()]
    }
}

/// **The lobe a compass bearing rounds to** (degrees clockwise from north).
pub fn arrow_of_degrees(bearing: f64) -> Arrow {
    if !bearing.is_finite() {
        return Arrow::North;
    }
    Arrow::ALL[(((bearing.rem_euclid(360.0)) / 45.0) + 0.5) as usize % Arrow::ALL.len()]
}

/// **A camera's compass bearing**, in degrees clockwise from north, from the direction it looks.
pub fn bearing_of(eye: [f64; 3], look: [f64; 3]) -> f64 {
    let (dx, dz) = (look[0] - eye[0], look[2] - eye[2]);
    if !dx.is_finite() || !dz.is_finite() || (dx == 0.0 && dz == 0.0) {
        return 0.0;
    }
    dx.atan2(-dz).to_degrees().rem_euclid(360.0)
}

/// **The map's camera** — the value the whole picture comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapCamera {
    /// The world column at the centre of the body, metres.
    pub centre_m: [f64; 2],
    /// **How big a metre is**, in device-independent pixels. One scale for both axes: a metre east
    /// and a metre north are the same length on screen, whatever the window is doing.
    pub pixels_per_m: f64,
    /// Which way up the screen is, degrees clockwise from north.
    pub bearing_deg: f64,
    /// How far down the camera looks. `90` is straight down; smaller skims the ground.
    pub pitch_deg: f64,
}

impl Default for MapCamera {
    fn default() -> Self {
        MapCamera {
            centre_m: [0.0, 0.0],
            pixels_per_m: 0.45,
            bearing_deg: 0.0,
            pitch_deg: 24.0,
        }
    }
}

impl MapCamera {
    /// The pitch range: shallow enough to see the shape of a valley, steep enough that the ground
    /// never collapses into a line.
    pub const MIN_PITCH_DEG: f64 = 8.0;
    pub const MAX_PITCH_DEG: f64 = 84.0;
    /// The scale range — about 2 km across a 1000 px body down to 40 m.
    pub const MIN_PIXELS_PER_M: f64 = 0.05;
    pub const MAX_PIXELS_PER_M: f64 = 24.0;
    /// **Pitch finer than this is refused**, so a pitch step never lands on a degenerate view.
    pub const PITCH_STEP_DEG: f64 = 6.0;
    /// **Bearing finer than this is refused.**
    pub const BEARING_STEP_DEG: f64 = 15.0;

    /// A camera at a world column, at the default scale and pitch.
    pub fn at(centre_m: [f64; 2]) -> MapCamera {
        MapCamera {
            centre_m,
            ..MapCamera::default()
        }
    }

    /// The scale in the horizontal and vertical screen directions. A metre north is foreshortened
    /// by the pitch, which is what gives the picture a horizon.
    fn axes(&self) -> (f64, f64, f64, f64) {
        let pitch = self.pitch_deg.to_radians().clamp(
            Self::MIN_PITCH_DEG.to_radians(),
            Self::MAX_PITCH_DEG.to_radians(),
        );
        let bearing = self.bearing_deg.to_radians();
        (pitch.sin(), pitch.cos(), bearing.sin(), bearing.cos())
    }

    /// **A world point to a body pixel.** Height lifts a point up the screen by `cos(pitch)`.
    pub fn project(&self, rect: Rect, world: [f64; 3]) -> [f64; 2] {
        let (sin_pitch, cos_pitch, sin_b, cos_b) = self.axes();
        let dx = world[0] - self.centre_m[0];
        let dz = world[2] - self.centre_m[1];
        let right = dx * cos_b + dz * sin_b;
        let forward = -dx * sin_b + dz * cos_b;
        [
            rect.x + rect.width_dp / 2.0 + right * self.pixels_per_m,
            rect.y + rect.height_dp / 2.0 + forward * self.pixels_per_m * sin_pitch
                - world[1] * self.pixels_per_m * cos_pitch,
        ]
    }

    /// **The ground plane point a body pixel looks at**, ignoring terrain height — the first step of
    /// both the raster and picking. `forward` is metres along the camera's own forward axis, which
    /// is what [`MapCamera::depth_of`] reads a pixel back to.
    pub fn ground_point(&self, rect: Rect, px: f64, forward: f64) -> [f64; 2] {
        let (_, _, sin_b, cos_b) = self.axes();
        let right = (px - (rect.x + rect.width_dp / 2.0)) / self.pixels_per_m.max(1e-9);
        // The inverse of `project`'s rotation: bearing measures the camera's own turn, so the
        // screen's "up" axis is the camera's, and the world is turned back by it.
        let x = right * cos_b - forward * sin_b;
        let z = right * sin_b + forward * cos_b;
        [self.centre_m[0] + x, self.centre_m[1] + z]
    }

    /// **The forward distance a body pixel looks at, in metres, for ground at the datum.**
    ///
    /// `project` **foreshortens the ground by the pitch** — a metre of forward is
    /// `pixels_per_m · sin(pitch)` pixels up the body — so a pixel is read back by that same
    /// ratio and not by the scale alone. A surface `h` metres above the datum sits
    /// `h · pixels_per_m · cos(pitch)` higher again, which the caller adds back to the pixel before
    /// reading it (the refinement in [`MapTerrain::ground_elements`] and [`MapTerrain::cell_at_pointer`]).
    fn depth_of(&self, rect: Rect, py: f64) -> f64 {
        let (sin_pitch, _, _, _) = self.axes();
        (py - (rect.y + rect.height_dp / 2.0)) / (self.pixels_per_m.max(1e-9) * sin_pitch.max(1e-3))
    }

    /// **Drag the world with the pointer.** A drag to the right brings the world to the right, so a
    /// place stays under the finger rather than sliding away from it.
    pub fn pan_by_screen(&mut self, dx: f64, dy: f64, rect: Rect) {
        let base = self.ground_point(rect, rect.x + rect.width_dp / 2.0, 0.0);
        let across = self.ground_point(rect, rect.x + rect.width_dp / 2.0 + dx, 0.0);
        let down = self.ground_point(
            rect,
            rect.x + rect.width_dp / 2.0,
            self.depth_of(rect, rect.y + rect.height_dp / 2.0 + dy),
        );
        // **The world moves with the pointer, not away from it**: a drag right shows ground to the
        // west, so the centre steps back by the world the drag covered.
        self.centre_m[0] -= across[0] - base[0];
        self.centre_m[1] -= down[1] - base[1];
        self.clamp();
    }

    /// **Scale about an anchor pixel**, so the world under the pointer stays under it.
    pub fn zoom_by(&mut self, factor: f64, anchor: [f64; 2], rect: Rect) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let before = self.ground_point(rect, anchor[0], self.depth_of(rect, anchor[1]));
        self.pixels_per_m =
            (self.pixels_per_m * factor).clamp(Self::MIN_PIXELS_PER_M, Self::MAX_PIXELS_PER_M);
        let after = self.ground_point(rect, anchor[0], self.depth_of(rect, anchor[1]));
        self.centre_m[0] += before[0] - after[0];
        self.centre_m[1] += before[1] - after[1];
        self.clamp();
    }

    /// Turn the view. **A step is a step**: finer angles are refused, so the picture's up direction
    /// is one of the headings a person can name.
    pub fn rotate_by(&mut self, steps: f64) {
        self.bearing_deg = (self.bearing_deg + steps * Self::BEARING_STEP_DEG).rem_euclid(360.0);
    }

    /// Tilt the view, clamped so the ground never collapses into a line.
    pub fn pitch_by(&mut self, steps: f64) {
        self.pitch_deg = (self.pitch_deg + steps * Self::PITCH_STEP_DEG)
            .clamp(Self::MIN_PITCH_DEG, Self::MAX_PITCH_DEG);
    }

    /// Bring the camera back to a world column without touching scale or heading.
    pub fn look_at(&mut self, centre_m: [f64; 2]) {
        self.centre_m = centre_m;
        self.clamp();
    }

    /// Keep the camera state inside its declared ranges, and its numbers real.
    pub fn clamp(&mut self) {
        if !self.centre_m[0].is_finite() || !self.centre_m[1].is_finite() {
            self.centre_m = [0.0, 0.0];
        }
        if !self.pixels_per_m.is_finite() {
            self.pixels_per_m = MapCamera::default().pixels_per_m;
        }
        self.pixels_per_m = self
            .pixels_per_m
            .clamp(Self::MIN_PIXELS_PER_M, Self::MAX_PIXELS_PER_M);
        if !self.bearing_deg.is_finite() {
            self.bearing_deg = 0.0;
        }
        self.bearing_deg = self.bearing_deg.rem_euclid(360.0);
        if !self.pitch_deg.is_finite() {
            self.pitch_deg = MapCamera::default().pitch_deg;
        }
        self.pitch_deg = self
            .pitch_deg
            .clamp(Self::MIN_PITCH_DEG, Self::MAX_PITCH_DEG);
    }

    /// A key for a cached raster: the camera's own numbers as bits, because this is equality and not
    /// distance — a camera that moved by a millimetre is a different picture.
    pub fn key(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let mut mix = |value: u64| {
            hash ^= value;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        };
        for value in [
            self.centre_m[0].to_bits(),
            self.centre_m[1].to_bits(),
            self.pixels_per_m.to_bits(),
            self.bearing_deg.to_bits(),
            self.pitch_deg.to_bits(),
        ] {
            mix(value);
        }
        hash
    }
}

/// **Where the window's ground is centred and how much of it to sample.** The camera's controls move
/// this value: a pan moves the centre, and zooming out asks for more cells — which is the map's own
/// streaming request, answered from the world's laws until the streamer answers it from chunks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurveyPlan {
    /// The world column the survey is centred on, quantised to whole declared cells so a pan
    /// resamples once per cell rather than once per pixel.
    pub centre_xz: [f64; 2],
    /// One **sample**, metres. A declared cell divided by [`SUB`] when the camera is close enough
    /// for that to mean anything, and coarser when the window shows more ground than
    /// [`MAX_COLUMNS`] samples of it can describe.
    pub sample_m: f64,
    /// **The declared cell**, metres — what the world's climate is read on, and what the map's
    /// selection is measured in. A sample names its cell; the cell is what a person selects.
    pub declared_m: f64,
    /// Samples across and down.
    pub columns: usize,
    pub rows: usize,
}

impl MapCamera {
    /// **The survey a body needs at this camera**: centred where the camera looks, sized to what
    /// the body can see. Widening the window, or zooming out, asks for more ground; panning asks for
    /// the same number somewhere else. **The survey always covers the body** — a picture that stops
    /// short of the window's edge is a survey that stopped short of it.
    pub fn survey(&self, rect: Rect, declared_m: f64) -> SurveyPlan {
        let declared_m = declared_m.max(1.0);
        let (sin_pitch, _, _, _) = self.axes();
        let pixels_per_m = self.pixels_per_m.max(1e-9);
        // **The far half of the body is foreshortened ground**, so the survey reaches it by dividing
        // the body's height by `sin(pitch)` as well as by the scale — otherwise the top of the
        // window falls off the survey and the map letterboxes itself with void.
        let span_x = rect.width_dp / pixels_per_m;
        let span_z = rect.height_dp / (pixels_per_m * sin_pitch.max(1e-3));
        // **The one streaming rule**: as fine as a declared cell subdivided, and never so fine that
        // the survey cannot reach the edges. Zooming out coarsens the sample rather than truncating
        // the ground, which is why the picture keeps its edges at every scale.
        let sample_m = (declared_m / SUB as f64)
            .max(span_x / MAX_COLUMNS as f64)
            .max(span_z / MAX_ROWS as f64)
            .max(1.0);
        SurveyPlan {
            centre_xz: [
                (self.centre_m[0] / declared_m).round() * declared_m,
                (self.centre_m[1] / declared_m).round() * declared_m,
            ],
            sample_m,
            declared_m,
            columns: ((span_x / sample_m).ceil() as usize + 2).clamp(8, MAX_COLUMNS),
            rows: ((span_z / sample_m).ceil() as usize + 2).clamp(8, MAX_ROWS),
        }
    }
}

/// **The declared cell a world column falls in**, so a sample's position can be named as the cell a
/// person selects rather than as the pixel a pointer landed on.
fn snap_to_cell(value_m: f64, cell_m: f64) -> f64 {
    (value_m / cell_m).floor() * cell_m + cell_m / 2.0
}

/// **A view filter as one bit pattern**, for the raster's cache key: two selections that keep
/// different ground must not share a cached picture.
pub fn filter_key(filter: &ViewFilter) -> u64 {
    match filter {
        ViewFilter::Any => 0,
        ViewFilter::Water => 1,
        ViewFilter::Biomes(kinds) => kinds
            .iter()
            .fold(2u64, |bits, kind| bits | 1 << (*kind as u32)),
    }
}

/// **What a view selection keeps lit.** `all` keeps everything; a biome token keeps that ground and
/// shadows the rest; `ocean` keeps the water. An unknown token is `Any`, because a control that
/// filters nothing must not look like it filtered something.
#[derive(Clone, Debug, PartialEq)]
pub enum ViewFilter {
    Any,
    Water,
    Biomes(Vec<Biome>),
}

impl ViewFilter {
    /// The filter a frame's `feature` token names.
    pub fn of_token(token: Option<&str>) -> ViewFilter {
        match token.unwrap_or("all").trim().to_ascii_lowercase().as_str() {
            "ocean" | "sea" | "water" => ViewFilter::Water,
            "mountain" | "alpine" => ViewFilter::Biomes(vec![Biome::Alpine]),
            "forest" => ViewFilter::Biomes(vec![
                Biome::BorealForest,
                Biome::CoolTemperateForest,
                Biome::TemperateForest,
                Biome::TropicalRainforest,
            ]),
            "grassland" => ViewFilter::Biomes(vec![Biome::Grassland]),
            "desert" => ViewFilter::Biomes(vec![Biome::Desert, Biome::PolarDesert]),
            "tundra" => ViewFilter::Biomes(vec![Biome::Tundra]),
            "scrub" => ViewFilter::Biomes(vec![Biome::SubtropicalScrub]),
            "dry" => ViewFilter::Biomes(vec![Biome::TropicalDryForest]),
            _ => ViewFilter::Any,
        }
    }

    /// Whether a sample stays lit: `kind` is its biome, or `None` for water.
    pub fn lit(&self, kind: Option<Biome>) -> bool {
        match self {
            ViewFilter::Any => true,
            ViewFilter::Water => kind.is_none(),
            ViewFilter::Biomes(kinds) => kind.is_some_and(|kind| kinds.contains(&kind)),
        }
    }
}

/// **The player's place on the map.**
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// The player's world column, metres.
    pub xz: [f64; 2],
    /// The ground the marker stands on, metres above the datum.
    pub y_m: f64,
    /// The heading, degrees clockwise from north.
    pub bearing_deg: f64,
}

/// **One window's map**: a grid of ground — the world's own voxels where the streamer holds chunks,
/// the laws' field beyond them — the camera over it, and the player's pose. The window's *selection*
/// is the mount's, because selecting a cell is a shell act; this value answers what a cell is and
/// what the camera sees.
#[derive(Clone, Debug)]
pub struct MapTerrain {
    /// Samples across the survey.
    pub columns: usize,
    pub rows: usize,
    /// The biome samples' own grid — climate is expensive, so biomes are sampled on the declared
    /// cells and blended; heights are cheap and sampled finely.
    pub biome_columns: usize,
    pub biome_rows: usize,
    /// The highest ground on each sample, metres. Rows of [`MapTerrain::columns`].
    pub heights: Vec<f32>,
    /// **The biome's own colour of each sample**, interpolated between the declared cells'
    /// biomes — **before relief**. The ink a viewer sees is [`MapTerrain::relief`]'s: it is read
    /// from the sample's own height at the moment it is drawn, so a survey the world has written
    /// over is shaded by the world's surface and not by the law's.
    pub colours: Vec<Rgb>,
    pub highest_m: f64,
    pub lowest_m: f64,
    /// The world column the grid is centred on, metres.
    pub centre_xz: [f64; 2],
    /// One map sample, metres east–west, and north–south.
    pub cell_m: f64,
    pub cell_row_m: f64,
    /// **One declared cell**, metres: what a selection is measured in and what the climate grid is
    /// read on.
    pub declared_m: f64,
    pub sea_level_m: f64,
    /// **What each sample is** — its biome, or `None` for water below sea level. This is what a view
    /// selection filters on; the drawn colour is [`MapTerrain::colours`].
    pub kinds: Vec<Option<Biome>>,
    /// The player's pose, when there is one.
    pub pose: Option<Pose>,
    /// Bumped by every sample, so a cached raster knows its ground moved.
    pub revision: u64,
}

impl MapTerrain {
    /// **Sample the law's own field for a plan**, once per distinct view: the smooth world the
    /// voxels were placed from. This is the far ground and the base a survey writes the streamer's
    /// own columns over — **not** the picture, except where nothing is resident.
    pub fn sample(spec: &WorldSpec, tick: u64, plan: SurveyPlan) -> MapTerrain {
        let columns = plan.columns.clamp(1, MAX_COLUMNS);
        let rows = plan.rows.clamp(1, MAX_ROWS);
        // The survey keeps the planned span whatever resolution it is sampled at, so a finer sample
        // is a sharper picture of the same place.
        let cell_m = plan.sample_m.max(1.0);
        let cell_row_m = cell_m;
        let declared_m = plan.declared_m.max(cell_m);
        let centre_xz = plan.centre_xz;
        // **The climate grid is the declared one**, coarsening with the survey when the window
        // reaches further than that grid can describe: two grids over one span, so the mapping
        // between them is a ratio and not a second centre.
        let span_x = columns as f64 * cell_m;
        let span_z = rows as f64 * cell_m;
        let biome_columns = ((span_x / declared_m).ceil() as usize).clamp(2, MAX_BIOME);
        let biome_rows = ((span_z / declared_m).ceil() as usize).clamp(2, MAX_BIOME);
        let biome_m_x = span_x / biome_columns as f64;
        let biome_m_z = span_z / biome_rows as f64;
        let biome_at_sample = |column: usize, row: usize| {
            (
                (column as f64 + 0.5) * (cell_m / biome_m_x) - 0.5,
                (row as f64 + 0.5) * (cell_m / biome_m_z) - 0.5,
            )
        };
        let mut heights = Vec::with_capacity(columns * rows);
        let mut highest_m = f64::MIN;
        let mut lowest_m = f64::MAX;
        for row in 0..rows {
            for column in 0..columns {
                let x = centre_xz[0] + (column as f64 + 0.5 - columns as f64 / 2.0) * cell_m;
                let z = centre_xz[1] + (row as f64 + 0.5 - rows as f64 / 2.0) * cell_row_m;
                let height = voxel_shapers::terrain::elevation_m(spec, x, z);
                highest_m = highest_m.max(height);
                lowest_m = lowest_m.min(height);
                heights.push(height as f32);
            }
        }
        let highest_m = if highest_m.is_finite() {
            highest_m
        } else {
            0.0
        };
        let lowest_m = if lowest_m.is_finite() { lowest_m } else { 0.0 };
        // **The biomes are sampled on the declared cells**, because a climate column costs about ten
        // thousand times a height sample; the colour between them is blended here, once, so the
        // raster's own sampling is a bilinear read of this and nothing more.
        let mut biomes = Vec::with_capacity(biome_columns * biome_rows);
        for row in 0..biome_rows {
            for column in 0..biome_columns {
                let x =
                    centre_xz[0] + (column as f64 + 0.5 - biome_columns as f64 / 2.0) * biome_m_x;
                let z = centre_xz[1] + (row as f64 + 0.5 - biome_rows as f64 / 2.0) * biome_m_z;
                let height = voxel_shapers::terrain::elevation_m(spec, x, z);
                biomes.push(biome_at(spec, tick, x, z, height));
            }
        }
        let sea_level_m = spec.config.sea_level_m;
        let mut colours = Vec::with_capacity(columns * rows);
        let mut kinds = Vec::with_capacity(columns * rows);
        for row in 0..rows {
            for column in 0..columns {
                let height = f64::from(heights[row * columns + column]);
                // **The view's own reading of water** stays the law's, so `plan ocean` selects the
                // same cells whatever the residency; the *ink* is [`MapTerrain::relief`]'s and
                // follows the height the map is actually drawing.
                kinds.push(if height < sea_level_m {
                    None
                } else {
                    let (fx, fy) = biome_at_sample(column, row);
                    nearest_biome(&biomes, biome_columns, biome_rows, fx, fy)
                });
                let (fx, fy) = biome_at_sample(column, row);
                colours.push(blend_biomes(&biomes, biome_columns, biome_rows, fx, fy));
            }
        }
        MapTerrain {
            columns,
            rows,
            biome_columns,
            biome_rows,
            heights,
            colours,
            highest_m,
            lowest_m,
            centre_xz,
            cell_m,
            cell_row_m,
            declared_m,
            sea_level_m,
            kinds,
            pose: None,
            revision: next_revision(),
        }
    }

    /// **The ink a sample of this height is drawn in**: water below sea level, deeper with depth,
    /// and land in its own colour, shaded by how far it stands above the survey's lowest ground.
    ///
    /// **Read from the height, not baked into the colour.** A survey the world's own voxels have
    /// been written over has heights the laws never produced, and the shading has to follow them or
    /// a dug pit is drawn in a law's relief.
    pub fn relief(&self, height: f64, colour: Rgb) -> Rgb {
        let span = (self.highest_m - self.lowest_m).max(1.0);
        if height < self.sea_level_m {
            let depth = ((self.sea_level_m - height) / span).clamp(0.0, 1.0);
            mix(WATER_SHALLOW, WATER_DEEP, depth)
        } else {
            let rise = ((height - self.lowest_m) / span).clamp(0.0, 1.0);
            shade(colour, 0.72 + 0.56 * rise)
        }
    }

    /// **Re-measure the survey's own vertical span** after a write over its heights: the relief's
    /// ink and the water's depth are read from these two numbers.
    pub fn refit_extent(&mut self) {
        let mut highest = f64::MIN;
        let mut lowest = f64::MAX;
        for height in &self.heights {
            let height = f64::from(*height);
            if height.is_finite() {
                highest = highest.max(height);
                lowest = lowest.min(height);
            }
        }
        self.highest_m = if highest.is_finite() { highest } else { 0.0 };
        self.lowest_m = if lowest.is_finite() { lowest } else { 0.0 };
    }

    /// **Publish a new ground**: re-measure the span and move the revision, so a cached raster
    /// knows the ground under it changed.
    pub fn republish(&mut self) {
        self.refit_extent();
        self.revision = next_revision();
    }

    /// **Move the player's marker over the survey**, without resampling the ground.
    pub fn follow(&mut self, xz: [f64; 2], y_m: f64, bearing_deg: f64) {
        if !xz[0].is_finite() || !xz[1].is_finite() || !bearing_deg.is_finite() {
            self.pose = None;
            return;
        }
        self.pose = Some(Pose {
            xz,
            y_m,
            bearing_deg: bearing_deg.rem_euclid(360.0),
        });
    }

    /// The grid's cell count.
    pub fn cells(&self) -> usize {
        self.columns * self.rows
    }

    /// **The fractional sample a world column falls on.**
    pub fn cell_of(&self, xz: [f64; 2]) -> (f64, f64) {
        (
            (xz[0] - self.centre_xz[0]) / self.cell_m + self.columns as f64 / 2.0 - 0.5,
            (xz[1] - self.centre_xz[1]) / self.cell_row_m + self.rows as f64 / 2.0 - 0.5,
        )
    }

    /// **The world column a sample is centred at** — the column its height and its colour were
    /// read at. This is the survey's own arithmetic and not a reading of a cell: a test that
    /// compares the survey with the law it sampled has to ask at the column that was sampled.
    pub fn sample_xz(&self, cell: (i32, i32)) -> [f64; 2] {
        [
            self.centre_xz[0] + (cell.0 as f64 + 0.5 - self.columns as f64 / 2.0) * self.cell_m,
            self.centre_xz[1] + (cell.1 as f64 + 0.5 - self.rows as f64 / 2.0) * self.cell_row_m,
        ]
    }

    /// **The world column a sample is centred at, named as the declared cell it falls in**: a
    /// pointer picks a sample, but what a person selects — and what the toolbar and the route name —
    /// is one of the map's own cells.
    pub fn world_of(&self, cell: (i32, i32)) -> [f64; 2] {
        let [x, z] = self.sample_xz(cell);
        [
            snap_to_cell(x, self.declared_m),
            snap_to_cell(z, self.declared_m),
        ]
    }

    /// The highest ground on a sample, metres.
    pub fn height(&self, cell: (i32, i32)) -> Option<f64> {
        if cell.0 < 0 || cell.1 < 0 {
            return None;
        }
        self.heights
            .get(cell.1 as usize * self.columns + cell.0 as usize)
            .filter(|height| height.is_finite())
            .map(|height| f64::from(*height))
    }

    /// **The world position a sample names** — what the toolbar reads under the cursor.
    pub fn world_xyz(&self, cell: (i32, i32)) -> Option<[f64; 3]> {
        let height = self.height(cell)?;
        let [x, z] = self.world_of(cell);
        Some([x, height, z])
    }

    /// **The interpolated ground at a world column**: the height and the drawn colour, or `None`
    /// outside the survey — which is the honest answer for ground nobody has sampled.
    pub fn ground_at(&self, xz: [f64; 2]) -> Option<(f64, Rgb)> {
        let (fx, fy) = self.cell_of(xz);
        let inside = |v: f64, of: usize| v >= -0.5 && v <= of as f64 - 0.5;
        if !inside(fx, self.columns) || !inside(fy, self.rows) {
            return None;
        }
        let x0 = fx.floor().max(0.0) as usize;
        let y0 = fy.floor().max(0.0) as usize;
        let x1 = (x0 + 1).min(self.columns - 1);
        let y1 = (y0 + 1).min(self.rows - 1);
        let tx = (fx - x0 as f64).clamp(0.0, 1.0);
        let ty = (fy - y0 as f64).clamp(0.0, 1.0);
        let corner = |x: usize, y: usize, w: f64| {
            let index = y * self.columns + x;
            let height = self.heights.get(index).copied().unwrap_or(0.0);
            let colour = self.colours.get(index).copied().unwrap_or(GRASSLAND);
            (f64::from(height) * w, scale_colour(colour, w))
        };
        let (h00, c00) = corner(x0, y0, (1.0 - tx) * (1.0 - ty));
        let (h10, c10) = corner(x1, y0, tx * (1.0 - ty));
        let (h01, c01) = corner(x0, y1, (1.0 - tx) * ty);
        let (h11, c11) = corner(x1, y1, tx * ty);
        let height = h00 + h10 + h01 + h11;
        let colour = add(add(c00, c10), add(c01, c11));
        Some((height, self.relief(height, colour)))
    }

    /// **The ground as the shell draws it**: columns of runs, painted into `rect` through `camera`.
    ///
    /// The body is filled with sky and then walked column by column from near to far; the terrain's
    /// own silhouette decides where the ground stops, and each column's ground is merged into runs.
    /// **This is the one place the picture is made**, so a window and a test read the same raster.
    pub fn ground_elements(
        &self,
        rect: Rect,
        camera: &MapCamera,
        filter: &ViewFilter,
    ) -> Vec<Element> {
        let mut elements = Vec::new();
        if rect.width_dp <= 0.0 || rect.height_dp <= 0.0 || self.columns == 0 || self.rows == 0 {
            return elements;
        }
        // The sky, in bands, so the picture has a horizon rather than one flat blue. Each band
        // overlaps the next by a pixel to hide the seam, and **the last one stops at the body's
        // own edge**: the overlap is for a seam inside the picture, and the map does not paint
        // outside the window its package declared for it.
        let bands = 6;
        let band = rect.height_dp / bands as f64;
        for index in 0..bands {
            let y = rect.y + index as f64 * band;
            elements.push(Element::Surface {
                rect: Rect::new(
                    rect.x,
                    y,
                    rect.width_dp,
                    (band + 1.0).min(rect.bottom() - y),
                ),
                paint: Paint::opaque(mix(SKY_HIGH, SKY_LOW, index as f64 / (bands - 1) as f64)),
            });
        }
        // **The pitch enters through the projection, not here**: `ground_point` already carries
        // `sin(pitch)` for the ray, and the surface's screen lift is `cos(pitch)` per metre.
        let (_, cos_pitch, _, _) = camera.axes();
        let columns = ((rect.width_dp / COLUMN_DP).ceil() as usize).max(1);
        for column in 0..columns {
            let px = rect.x + column as f64 * COLUMN_DP + COLUMN_DP / 2.0;
            let mut runs: Vec<(f64, f64, Rgb)> = Vec::new();
            let mut y = rect.bottom();
            while y > rect.y - ROW_STEP_DP {
                let py = y - ROW_STEP_DP / 2.0;
                let depth = camera.depth_of(rect, py);
                // **Four refinement steps**: the ground point of a pixel depends on the height of
                // the ground under it, so the inverse is iterated to the surface.
                let mut world = camera.ground_point(rect, px, depth);
                let mut sampled = self.ground_at(world);
                for _ in 0..3 {
                    let Some((height, _)) = sampled else { break };
                    // **The sign matters**: a surface `h` above the datum projects `h·px·cos` up the
                    // screen, so the ray point that reaches this row sits that much deeper.
                    let corrected =
                        camera.depth_of(rect, py + height * camera.pixels_per_m * cos_pitch);
                    let candidate = camera.ground_point(rect, px, corrected);
                    if let Some(next) = self.ground_at(candidate) {
                        world = candidate;
                        sampled = Some(next);
                    } else {
                        break;
                    }
                }
                // **Every row this column reaches is painted**: each row's own inverse has already
                // resolved to the ground under it, so a row that the survey answers is a row of
                // ground and nothing is left for the sky to show through. A row the survey cannot
                // answer is left alone — that is the one honest edge of the picture.
                if let Some((_, colour)) = sampled {
                    // **A view selection keeps its own ground bright and the rest in shadow**, so
                    // `plan mountain` shows the mountain rather than a caption over the same
                    // picture. `all` filters nothing.
                    let lit = filter.lit(self.ground_kind(world).unwrap_or(None));
                    let colour = if lit { colour } else { shade(colour, 0.22) };
                    match runs.last_mut() {
                        Some((start, end, run)) if same_run(*run, colour) => {
                            *start = py;
                            let _ = end;
                        }
                        _ => runs.push((py, py, colour)),
                    }
                }
                y -= ROW_STEP_DP;
            }
            for (start, end, colour) in runs {
                let top = start.min(end);
                let bottom = start.max(end);
                // **A run overlaps its neighbours by half a row step, and its column by half a
                // column**: two rects that merely abut leave a seam wherever a renderer rounds
                // their edges, and a seam in the ground is the sky showing through it. The paint
                // is opaque, so the overlap costs nothing but a few tenths of a pixel.
                //
                // **And the run is clipped to the body it is drawn in.** The overlap exists to
                // cover a seam *inside* the picture; past the body's edge there is no seam to
                // hide, and a spill there is the map painting outside the window its package
                // declared for it.
                let x = rect.x + column as f64 * COLUMN_DP;
                let y = (top - ROW_STEP_DP / 2.0).max(rect.y);
                let width = (COLUMN_DP + RUN_OVERLAP_DP).min(rect.right() - x);
                let height = (bottom - top + ROW_STEP_DP * 2.0)
                    .clamp(ROW_STEP_DP, (rect.bottom() - y).max(ROW_STEP_DP));
                elements.push(Element::Surface {
                    rect: Rect::new(x, y, width.max(0.0), height),
                    paint: Paint::opaque(colour),
                });
            }
        }
        elements
    }

    /// The kind of ground at a world column: its biome, or `None` for water. `Some(None)` is water,
    /// and `None` is off the survey — the two are different answers.
    pub fn ground_kind(&self, xz: [f64; 2]) -> Option<Option<Biome>> {
        let (fx, fy) = self.cell_of(xz);
        if fx < -0.5 || fy < -0.5 || fx > self.columns as f64 - 0.5 || fy > self.rows as f64 - 0.5 {
            return None;
        }
        let index = fy.round().max(0.0) as usize * self.columns + fx.round().max(0.0) as usize;
        Some(
            self.kinds
                .get(index)
                .copied()
                .unwrap_or(Some(Biome::Grassland)),
        )
    }

    /// **The cell under a pointer**, through the camera. The inverse is iterated to the surface, so a
    /// click on a hillside lands on the hill; a point beyond the survey is no cell at all.
    pub fn cell_at_pointer(
        &self,
        rect: Rect,
        at: (f64, f64),
        camera: &MapCamera,
    ) -> Option<(i32, i32)> {
        if !rect.contains(at) || rect.width_dp <= 0.0 || rect.height_dp <= 0.0 {
            return None;
        }
        let (_, cos_pitch, _, _) = camera.axes();
        let mut depth = camera.depth_of(rect, at.1);
        let mut world = camera.ground_point(rect, at.0, depth);
        let mut inside = self.ground_at(world).is_some();
        for _ in 0..6 {
            let Some((height, _)) = self.ground_at(world) else {
                break;
            };
            let candidate = camera.ground_point(
                rect,
                at.0,
                camera.depth_of(rect, at.1 + height * camera.pixels_per_m * cos_pitch),
            );
            // The inverse only moves while it is over sampled ground; a point off the survey is the
            // answer "no cell", not a reason to walk off the world.
            if self.ground_at(candidate).is_none() {
                break;
            }
            world = candidate;
            inside = true;
        }
        if !inside {
            return None;
        }
        let (fx, fy) = self.cell_of(world);
        if fx < -0.5 || fy < -0.5 || fx > self.columns as f64 - 0.5 || fy > self.rows as f64 - 0.5 {
            return None;
        }
        Some((
            fx.round().clamp(0.0, self.columns as f64 - 1.0) as i32,
            fy.round().clamp(0.0, self.rows as f64 - 1.0) as i32,
        ))
    }

    /// The player's sample, rounding down.
    pub fn player_cell(&self) -> Option<(i32, i32)> {
        let pose = self.pose?;
        let (column, row) = self.cell_of(pose.xz);
        Some((column.floor() as i32, row.floor() as i32))
    }

    /// The arrow the player's heading rounds to.
    pub fn arrow(&self) -> Arrow {
        arrow_of_degrees(self.pose.map(|pose| pose.bearing_deg).unwrap_or(0.0))
    }

    /// **The cells a route to `selection` crosses**, from the player's sample to the selection's,
    /// clipped to the survey.
    pub fn route(&self, selection: Option<[f64; 2]>) -> Vec<(i32, i32)> {
        let (Some(player), Some(selected)) =
            (self.player_cell(), selection.map(|xz| self.cell_of(xz)))
        else {
            return Vec::new();
        };
        line_cells(
            player,
            (selected.0.round() as i32, selected.1.round() as i32),
            self.columns,
            self.rows,
        )
    }

    /// **The marks**: the player, its frame and arrow, the selection, the route and any edge
    /// indicators — every one of them projected through the same camera as the ground.
    pub fn mark_elements(
        &self,
        rect: Rect,
        camera: &MapCamera,
        selection: Option<[f64; 2]>,
        cursor: Option<(i32, i32)>,
    ) -> Vec<Element> {
        let mut elements = Vec::with_capacity(32);
        if rect.width_dp <= 0.0 || rect.height_dp <= 0.0 {
            return elements;
        }
        // The route, under everything that marks a place. Each sample of the line is projected, so a
        // route over a hill is a route over the hill.
        for cell in self.route(selection) {
            let Some([x, height, z]) = self.world_xyz(cell) else {
                continue;
            };
            let at = camera.project(rect, [x, height, z]);
            if !rect.contains((at[0], at[1])) {
                continue;
            }
            elements.push(Element::Surface {
                rect: Rect::new(at[0] - 2.0, at[1] - 2.0, 4.0, 4.0),
                paint: Paint::opaque(ROUTE_INK),
            });
        }
        if let Some(selected) = selection {
            let whole = self.cell_of(selected);
            let cell = (whole.0.round() as i32, whole.1.round() as i32);
            if let Some([x, height, z]) = self.world_xyz(cell) {
                self.mark(
                    &mut elements,
                    rect,
                    camera,
                    [x, height, z],
                    SELECTION_GLYPH,
                    ROUTE_HEAD_INK,
                    12.0,
                );
            }
        }
        if let Some(cell) = cursor {
            if let Some([x, height, z]) = self.world_xyz(cell) {
                let at = camera.project(rect, [x, height, z]);
                if rect.contains((at[0], at[1])) {
                    let outline = Rect::new(at[0] - 6.0, at[1] - 6.0, 12.0, 12.0);
                    let paint = Paint::opaque(CURSOR_INK);
                    for strip in [
                        Rect::new(outline.x, outline.y, outline.width_dp, 1.0),
                        Rect::new(outline.x, outline.bottom() - 1.0, outline.width_dp, 1.0),
                        Rect::new(outline.x, outline.y, 1.0, outline.height_dp),
                        Rect::new(outline.right() - 1.0, outline.y, 1.0, outline.height_dp),
                    ] {
                        elements.push(Element::Rule { rect: strip, paint });
                    }
                }
            }
        }
        if let Some(pose) = self.pose {
            let player = [pose.xz[0], pose.y_m, pose.xz[1]];
            let at = camera.project(rect, player);
            if rect.contains((at[0], at[1])) {
                elements.push(Element::Surface {
                    rect: Rect::new(at[0] - 9.0, at[1] - 9.0, 18.0, 18.0),
                    paint: Paint::opaque(MARKER_BACKDROP),
                });
                for (dx, dy, glyph) in [
                    (-1.0, -1.0, FRAME[0]),
                    (1.0, -1.0, FRAME[1]),
                    (-1.0, 1.0, FRAME[2]),
                    (1.0, 1.0, FRAME[3]),
                ] {
                    elements.push(Element::Text {
                        rect: Rect::new(
                            at[0] + dx * 10.0 - 5.0,
                            at[1] + dy * 10.0 - 5.0,
                            10.0,
                            10.0,
                        ),
                        text: glyph.to_string(),
                        paint: Paint::opaque(PLAYER_INK),
                        size_dp: 10.0,
                        align: Align::Start,
                    });
                }
                elements.push(Element::Text {
                    rect: Rect::new(at[0] - 7.0, at[1] - 7.0, 16.0, 16.0),
                    text: PLAYER_GLYPH.to_string(),
                    paint: Paint::opaque(PLAYER_INK),
                    size_dp: 14.0,
                    align: Align::Start,
                });
                elements.push(Element::Text {
                    rect: Rect::new(at[0] + 3.0, at[1] - 21.0, 12.0, 12.0),
                    text: self.arrow().glyph().to_string(),
                    paint: Paint::opaque(PLAYER_INK),
                    size_dp: 13.0,
                    align: Align::Start,
                });
            } else {
                let edge = clamp_to_body(rect, at);
                elements.push(Element::Text {
                    rect: Rect::new(edge[0] - 7.0, edge[1] - 7.0, 14.0, 14.0),
                    text: PLAYER_EDGE.to_string(),
                    paint: Paint::opaque(INDICATOR_INK),
                    size_dp: 13.0,
                    align: Align::Start,
                });
            }
        }
        if let Some(selected) = selection {
            let whole = self.cell_of(selected);
            let cell = (whole.0.round() as i32, whole.1.round() as i32);
            if let Some([x, height, z]) = self.world_xyz(cell) {
                let at = camera.project(rect, [x, height, z]);
                if !rect.contains((at[0], at[1])) {
                    let edge = clamp_to_body(rect, at);
                    elements.push(Element::Text {
                        rect: Rect::new(edge[0] - 7.0, edge[1] - 7.0, 14.0, 14.0),
                        text: SELECTION_EDGE.to_string(),
                        paint: Paint::opaque(ROUTE_HEAD_INK),
                        size_dp: 13.0,
                        align: Align::Start,
                    });
                }
            }
        }
        elements
    }

    fn mark(
        &self,
        out: &mut Vec<Element>,
        rect: Rect,
        camera: &MapCamera,
        world: [f64; 3],
        glyph: char,
        ink: Rgb,
        size: f64,
    ) {
        let at = camera.project(rect, world);
        if !rect.contains((at[0], at[1])) {
            return;
        }
        out.push(Element::Text {
            rect: Rect::new(
                at[0] - size / 2.0,
                at[1] - size / 2.0,
                size + 2.0,
                size + 2.0,
            ),
            text: glyph.to_string(),
            paint: Paint::opaque(ink),
            size_dp: size,
            align: Align::Start,
        });
    }
}

/// **The radii a brush offers**, metres. One ladder, read in both directions, so "wider" and
/// "narrower" cannot disagree about the sizes available and a caller cannot name a size the ladder
/// cannot reach.
pub const BRUSH_RADII_M: [f64; 4] = [2.0, 4.0, 8.0, 16.0];
/// The radius a brush starts at, metres — the ladder's second step.
pub const BRUSH_DEFAULT_M: f64 = 4.0;
/// **The furthest one stroke lifts or drops**, metres.
///
/// A stroke is one recorded voxel edit per column per metre, made on the frame's own thread
/// because the world is the app's and it is not handed across a thread for a button. The bound is
/// what keeps one press a stroke rather than a stall: the widest brush at its limit is about six
/// thousand voxels, and anything a person means by "raise the ground a little" is far below it.
pub const MAX_HAMMER_LIFT: i32 = 8;
/// **How many points a ring is drawn with.** Enough that its gaps read as a dotted outline at the
/// closest zoom the camera allows, and few enough that a ring is a few dozen elements rather than
/// a raster of its own.
const RING_POINTS: usize = 48;
/// The brush's own ink, and the ink of the footprint it has marked — two colours, because a brush
/// that follows the pointer and a target that stays put are two different places on a map.
pub const BRUSH_INK: Rgb = Rgb::new(255, 170, 88);
pub const MARKED_INK: Rgb = Rgb::new(255, 238, 120);

/// **One stroke, as the world can record it**: the whole-metre columns it covers and the voxels it
/// moves at each.
///
/// Columns rather than a centre and a radius, so what the world edits is exactly the footprint that
/// was modelled. **Nothing on the map stages one any more** — the terrain's modification is the
/// shapers' over the world's cadence tick — and this is the shape
/// [`Streamer::shape_columns`](../../voxel-work/src/stream.rs) takes if that path hands it a
/// stroke.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MapEdit {
    /// The world's own 1 m columns, each named by its lower corner `[x, z]`.
    pub columns: Vec<[i32; 2]>,
    /// Voxels to add above each column's surface (`+`) or remove from it (`−`).
    pub lift: i32,
}

/// **The stroke modeller**: where a brush is, how wide, what it has marked, and how far the next
/// stroke lifts.
///
/// **Parked.** The map window draws and moves and edits nothing, so no gutter button and no key
/// drives this; its tests are what keep it honest until the shapers' own editing path collects it.
///
/// **The brush is the caller's, unless it has been marked.** `set_at` puts it where a pointer is;
/// `mark` freezes that footprint as the target, so the pointer can be moved, the ground read
/// elsewhere, and the stroke still land where it was meant to. Without a mark a stroke lands on the
/// brush's own footprint, because a hammer that refused to strike until it had been marked would be
/// a hammer with a ceremony in front of it.
///
/// The radius is in **metres**, and the stroke's footprint is every whole-metre column within it —
/// the world's own lattice, not the survey's samples: a sample can be eight metres of ground, and
/// raising "the patch under the ring" must raise the patch and not eight metres of everything.
#[derive(Clone, Debug, PartialEq)]
pub struct MapBrush {
    /// The pointer's cell, world metres — the centre a stroke lands on until a mark moves it.
    centre: Option<[f64; 2]>,
    /// The brush's radius, metres: one of [`BRUSH_RADII_M`].
    radius_m: f64,
    /// **The marked footprint's centre**, and the radius it was marked at — a brush resized after
    /// the mark does not move the target, because the mark is the place a person chose.
    target: Option<[f64; 2]>,
    target_radius_m: f64,
    /// Voxels staged for the next stroke, metres: `+` raises, `−` lowers.
    lift: i32,
}

impl Default for MapBrush {
    fn default() -> Self {
        MapBrush::new()
    }
}

impl MapBrush {
    /// A brush at the ladder's default size, with nothing marked and nothing staged.
    pub fn new() -> MapBrush {
        MapBrush {
            centre: None,
            radius_m: BRUSH_DEFAULT_M,
            target: None,
            target_radius_m: BRUSH_DEFAULT_M,
            lift: 0,
        }
    }

    /// Where the pointer last put the brush, world metres.
    pub fn centre(&self) -> Option<[f64; 2]> {
        self.centre
    }

    /// **Put the brush at a column.** A non-finite column is no brush at all, for the same
    /// reason a non-finite selection is no selection.
    pub fn set_at(&mut self, xz: [f64; 2]) {
        if xz[0].is_finite() && xz[1].is_finite() {
            self.centre = Some(xz);
        }
    }

    /// The brush's radius, metres.
    pub fn radius_m(&self) -> f64 {
        self.radius_m
    }

    /// Voxels staged for the next stroke.
    pub fn lift(&self) -> i32 {
        self.lift
    }

    /// Whether a footprint has been marked.
    pub fn marked(&self) -> bool {
        self.target.is_some()
    }

    /// The footprint's centre and radius: the mark when there is one, the brush otherwise.
    pub fn target(&self) -> Option<([f64; 2], f64)> {
        self.target
            .map(|centre| (centre, self.target_radius_m))
            .or_else(|| self.centre.map(|centre| (centre, self.radius_m)))
    }

    /// **Step the brush up or down the ladder**, answering the radius it landed on. At either end
    /// the radius stays put: a button that reports a size it did not reach is worse than one that
    /// does nothing.
    fn step(&mut self, wider: bool) -> f64 {
        let at = BRUSH_RADII_M
            .iter()
            .position(|radius| *radius >= self.radius_m - 1e-9)
            .unwrap_or(BRUSH_RADII_M.len() - 1);
        let next = if wider {
            (at + 1).min(BRUSH_RADII_M.len() - 1)
        } else {
            at.saturating_sub(1)
        };
        self.radius_m = BRUSH_RADII_M[next];
        self.radius_m
    }

    /// Widen the brush one ladder step; the radius it is now at.
    pub fn wider(&mut self) -> f64 {
        self.step(true)
    }

    /// Narrow the brush one ladder step; the radius it is now at.
    pub fn narrower(&mut self) -> f64 {
        self.step(false)
    }

    /// **Mark the brush's footprint as the target**, answering where and how wide it is. Refused
    /// when the pointer has never been over the body, because a mark is a place and there is none.
    pub fn mark(&mut self) -> Result<([f64; 2], f64), String> {
        let centre = self.centre.ok_or_else(|| {
            "there is no brush to mark: the pointer has not been over the map body".to_string()
        })?;
        self.target = Some(centre);
        self.target_radius_m = self.radius_m;
        Ok((centre, self.radius_m))
    }

    /// **Stage one metre of lift**, answering the staged total.
    pub fn raise(&mut self) -> Result<i32, String> {
        self.target().ok_or_else(|| {
            "the hammer has no target: put the pointer on the map body, or mark a brush".to_string()
        })?;
        if self.lift >= MAX_HAMMER_LIFT {
            return Err(format!(
                "the hammer is already staged at +{MAX_HAMMER_LIFT} m, its limit"
            ));
        }
        self.lift += 1;
        Ok(self.lift)
    }

    /// **Stage one metre of drop**, answering the staged total.
    pub fn lower(&mut self) -> Result<i32, String> {
        self.target().ok_or_else(|| {
            "the hammer has no target: put the pointer on the map body, or mark a brush".to_string()
        })?;
        if self.lift <= -MAX_HAMMER_LIFT {
            return Err(format!(
                "the hammer is already staged at −{MAX_HAMMER_LIFT} m, its limit"
            ));
        }
        self.lift -= 1;
        Ok(self.lift)
    }

    /// **Forget the mark and anything staged.** The brush keeps its place and its size: clearing is
    /// about the stroke, not about where the person is looking.
    pub fn clear(&mut self) {
        self.target = None;
        self.lift = 0;
    }

    /// **The stroke staged for the target**, and the staging resets — a stroke is recorded once.
    /// `None` when there is nothing to record: no target, or nothing staged.
    pub fn edit(&mut self) -> Option<MapEdit> {
        let (centre, radius_m) = self.target()?;
        if self.lift == 0 {
            return None;
        }
        let lift = std::mem::replace(&mut self.lift, 0);
        Some(MapEdit {
            columns: columns_within(centre, radius_m),
            lift,
        })
    }

    /// **What the toolbar says about the hammer**: the brush's width and, when something is
    /// staged, how far. One string, because the toolbar has one cell for it.
    pub fn readout(&self) -> String {
        match self.lift {
            0 => format!("{:.0} m", self.radius_m),
            lift => format!("{:.0} m {:+}", self.radius_m, lift),
        }
    }

    /// **The rings**: the brush's footprint where the pointer put it, and the marked footprint on
    /// top of it. Each point is projected at the ground under it through the survey's own camera,
    /// so a ring over a hillside bends with the hill instead of floating over it.
    pub fn elements(&self, terrain: &MapTerrain, rect: Rect, camera: &MapCamera) -> Vec<Element> {
        let mut elements = Vec::with_capacity(RING_POINTS * 2 + 1);
        if let Some(centre) = self.centre {
            ring(
                &mut elements,
                terrain,
                rect,
                camera,
                centre,
                self.radius_m,
                BRUSH_INK,
            );
        }
        if let Some(centre) = self.target {
            ring(
                &mut elements,
                terrain,
                rect,
                camera,
                centre,
                self.target_radius_m,
                MARKED_INK,
            );
        }
        elements
    }
}

/// **Every whole-metre column within `radius_m` of `centre`**, named by its lower corner.
///
/// A column's centre is `x + 0.5`, so the disk a ring draws and the columns a stroke records are
/// the same set read at the same place: the ring is the boundary of this list.
pub fn columns_within(centre: [f64; 2], radius_m: f64) -> Vec<[i32; 2]> {
    let radius = radius_m.max(0.5);
    let mut columns = Vec::new();
    let low_x = (centre[0] - radius).ceil() as i32;
    let high_x = (centre[0] + radius).floor() as i32;
    let low_z = (centre[1] - radius).ceil() as i32;
    let high_z = (centre[1] + radius).floor() as i32;
    for x in low_x..=high_x {
        for z in low_z..=high_z {
            let dx = (f64::from(x) + 0.5) - centre[0];
            let dz = (f64::from(z) + 0.5) - centre[1];
            if dx * dx + dz * dz <= radius * radius {
                columns.push([x, z]);
            }
        }
    }
    columns
}

/// Draw one footprint's outline as a dotted ring, projected through the camera.
fn ring(
    out: &mut Vec<Element>,
    terrain: &MapTerrain,
    rect: Rect,
    camera: &MapCamera,
    centre: [f64; 2],
    radius_m: f64,
    ink: Rgb,
) {
    let paint = Paint::opaque(ink);
    let middle = centre_height(terrain, centre);
    if let Some(height) = middle {
        let at = camera.project(rect, [centre[0], height, centre[1]]);
        if rect.contains((at[0], at[1])) {
            out.push(Element::Surface {
                rect: Rect::new(at[0] - 2.0, at[1] - 2.0, 5.0, 5.0),
                paint,
            });
        }
    }
    for point in 0..RING_POINTS {
        let angle = point as f64 / RING_POINTS as f64 * std::f64::consts::TAU;
        let xz = [
            centre[0] + radius_m * angle.cos(),
            centre[1] + radius_m * angle.sin(),
        ];
        let Some(height) = centre_height(terrain, xz) else {
            continue;
        };
        let at = camera.project(rect, [xz[0], height, xz[1]]);
        if !rect.contains((at[0], at[1])) {
            continue;
        }
        out.push(Element::Surface {
            rect: Rect::new(at[0] - 1.5, at[1] - 1.5, 3.0, 3.0),
            paint,
        });
    }
}

/// The ground under one column, as the survey draws it — `None` off the survey, where a ring's
/// point is not on the map at all.
fn centre_height(terrain: &MapTerrain, xz: [f64; 2]) -> Option<f64> {
    terrain.ground_at(xz).map(|(height, _)| height)
}

/// **A point clamped onto a body's edge**, for a mark the view cannot show.
fn clamp_to_body(rect: Rect, at: [f64; 2]) -> [f64; 2] {
    [
        at[0].clamp(rect.x + 8.0, rect.right() - 8.0),
        at[1].clamp(rect.y + 8.0, rect.bottom() - 8.0),
    ]
}

/// A colour scaled by a weight, for the survey's bilinear read.
fn scale_colour(colour: Rgb, w: f64) -> Rgb {
    Rgb::new(
        (f64::from(colour.r) * w).round() as u8,
        (f64::from(colour.g) * w).round() as u8,
        (f64::from(colour.b) * w).round() as u8,
    )
}

fn add(a: Rgb, b: Rgb) -> Rgb {
    Rgb::new(
        a.r.saturating_add(b.r),
        a.g.saturating_add(b.g),
        a.b.saturating_add(b.b),
    )
}

/// Two colours that quantise to the same run.
fn same_run(a: Rgb, b: Rgb) -> bool {
    let step = |v: u8| v / RUN_STEP;
    step(a.r) == step(b.r) && step(a.g) == step(b.g) && step(a.b) == step(b.b)
}

/// **The nearest declared-cell biome to a sample.** The colour between two declared cells is a
/// blend, but what a view token selects is one of them: `plan mountain` keeps the mountain's own
/// columns rather than every column the mountain's colour leaked into.
fn nearest_biome(biomes: &[Biome], columns: usize, rows: usize, fx: f64, fy: f64) -> Option<Biome> {
    let x = fx.round().clamp(0.0, columns.saturating_sub(1) as f64) as usize;
    let y = fy.round().clamp(0.0, rows.saturating_sub(1) as f64) as usize;
    biomes.get(y * columns + x).copied()
}

/// The revision counter: every sample is a new ground, so a cached raster knows to rebuild.
fn next_revision() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// **The biome at a column**, from the world's own climate and altitude laws.
fn biome_at(spec: &WorldSpec, tick: u64, x: f64, z: f64, elevation_m: f64) -> Biome {
    let (biotemperature_c, precipitation_mm) =
        voxel_shapers::ecology::holdridge_inputs(spec, x, z, elevation_m, tick);
    voxel_shapers::ecology::biome_at(spec, elevation_m, biotemperature_c, precipitation_mm)
}

/// The colour a biome's ground is drawn with.
pub fn biome_colour(biome: Biome) -> Rgb {
    match biome {
        Biome::PolarDesert => POLAR_DESERT,
        Biome::Tundra => TUNDRA,
        Biome::BorealForest => BOREAL,
        Biome::CoolTemperateForest => COOL_TEMPERATE,
        Biome::TemperateForest => TEMPERATE,
        Biome::Grassland => GRASSLAND,
        Biome::Desert => DESERT,
        Biome::SubtropicalScrub => SCRUB,
        Biome::TropicalDryForest => TROPICAL_DRY,
        Biome::TropicalRainforest => RAINFOREST,
        Biome::Alpine => ALPINE,
    }
}

/// **A biome colour blended between the four declared cells** a sample sits among. Nearest-neighbour
/// colour was a mosaic; the ground is not a mosaic.
fn blend_biomes(biomes: &[Biome], columns: usize, rows: usize, fx: f64, fy: f64) -> Rgb {
    let x0 = fx.floor().max(0.0) as usize;
    let y0 = fy.floor().max(0.0) as usize;
    let x1 = (x0 + 1).min(columns.saturating_sub(1));
    let y1 = (y0 + 1).min(rows.saturating_sub(1));
    let tx = (fx - x0 as f64).clamp(0.0, 1.0);
    let ty = (fy - y0 as f64).clamp(0.0, 1.0);
    let at = |x: usize, y: usize| {
        biomes
            .get(y * columns + x)
            .copied()
            .map(biome_colour)
            .unwrap_or(GRASSLAND)
    };
    let top = mix(at(x0, y0), at(x1, y0), tx);
    let bottom = mix(at(x0, y1), at(x1, y1), tx);
    mix(top, bottom, ty)
}

/// One colour scaled in brightness, for a survey's relief.
fn shade(rgb: Rgb, factor: f64) -> Rgb {
    let channel = |value: u8| (f64::from(value) * factor).round().clamp(0.0, 255.0) as u8;
    Rgb::new(channel(rgb.r), channel(rgb.g), channel(rgb.b))
}

/// One colour mixed toward another.
fn mix(from: Rgb, to: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let channel = |a: u8, b: u8| (f64::from(a) * (1.0 - t) + f64::from(b) * t).round() as u8;
    Rgb::new(
        channel(from.r, to.r),
        channel(from.g, to.g),
        channel(from.b, to.b),
    )
}

/// **The cells a line crosses between two endpoints** (a Bresenham walk), clipped to a grid of
/// `columns` by `rows` cells.
pub fn line_cells(
    from: (i32, i32),
    to: (i32, i32),
    columns: usize,
    rows: usize,
) -> Vec<(i32, i32)> {
    if columns == 0 || rows == 0 {
        return Vec::new();
    }
    let (mut x, mut y) = from;
    let (dx, dy) = ((to.0 - from.0).abs(), -(to.1 - from.1).abs());
    let (sx, sy) = (
        if from.0 < to.0 { 1 } else { -1 },
        if from.1 < to.1 { 1 } else { -1 },
    );
    let mut error = dx + dy;
    let mut cells = Vec::new();
    loop {
        if x >= 0 && y >= 0 && (x as usize) < columns && (y as usize) < rows {
            cells.push((x, y));
        }
        if (x, y) == to || cells.len() > columns * rows {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tilted survey, built directly so the camera can be tested without the world's laws: 32 × 24
    /// samples of one metre, rising eastward, centred on the origin.
    fn grid() -> MapTerrain {
        let (columns, rows) = (32, 24);
        let mut heights = Vec::with_capacity(columns * rows);
        for row in 0..rows {
            for column in 0..columns {
                heights.push((column as f32) * 0.2 - (row as f32) * 0.05);
            }
        }
        let colours = vec![GRASSLAND; columns * rows];
        // **All land, and one biome the tests can select**: a selection test needs ground a token
        // can name and ground it must not name.
        let mut kinds = vec![Some(Biome::Grassland); columns * rows];
        for row in (rows / 2)..rows {
            for column in (columns / 2)..columns {
                kinds[row * columns + column] = Some(Biome::Alpine);
            }
        }
        MapTerrain {
            columns,
            rows,
            biome_columns: columns / SUB,
            biome_rows: rows / SUB,
            heights,
            colours,
            kinds,
            highest_m: 6.2,
            lowest_m: -1.15,
            centre_xz: [0.0, 0.0],
            cell_m: 1.0,
            cell_row_m: 1.0,
            declared_m: 1.0,
            sea_level_m: 0.0,
            pose: None,
            revision: 1,
        }
    }

    fn rect(width: f64, height: f64) -> Rect {
        Rect::new(100.0, 50.0, width, height)
    }

    fn text_at<'a>(elements: &'a [Element], needle: &str) -> Vec<&'a Rect> {
        elements
            .iter()
            .filter_map(|element| match element {
                Element::Text { rect, text, .. } if text == needle => Some(rect),
                _ => None,
            })
            .collect()
    }

    /// The ground elements' pixels, for the fill test.
    fn ground_pixels(elements: &[Element]) -> f64 {
        elements
            .iter()
            .map(|element| match element {
                Element::Surface { rect, .. } => rect.width_dp * rect.height_dp,
                _ => 0.0,
            })
            .sum()
    }

    #[test]
    fn the_scale_is_one_number_and_a_resize_does_not_change_it() {
        let camera = MapCamera::at([0.0, 0.0]);
        // The same world point projects the same distance from the body's centre in both sizes.
        let small = rect(320.0, 240.0);
        let large = rect(1280.0, 900.0);
        let point = [40.0, 0.0, -20.0];
        for body in [small, large] {
            let at = camera.project(body, point);
            assert!(
                ((at[0] - (body.x + body.width_dp / 2.0)) - 40.0 * camera.pixels_per_m).abs()
                    < 1e-9
            );
        }
        // A metre is the same number of pixels in both axes of both bodies: the pitch decides the
        // vertical foreshortening, and nothing about the window does.
        let pitched = MapCamera {
            pitch_deg: 60.0,
            ..camera
        };
        let a = pitched.project(large, [1.0, 0.0, 0.0]);
        let b = pitched.project(large, [0.0, 0.0, 1.0]);
        let centre = [
            large.x + large.width_dp / 2.0,
            large.y + large.height_dp / 2.0,
        ];
        // One metre east and one metre north, both at the datum: the same scale, the second
        // foreshortened by the pitch and nothing else.
        let sin_pitch = 60f64.to_radians().sin();
        assert!(((a[0] - centre[0]) - pitched.pixels_per_m).abs() < 1e-9);
        assert!(((b[1] - centre[1]) - pitched.pixels_per_m * sin_pitch).abs() < 1e-9);
    }

    #[test]
    fn pan_zoom_rotate_and_pitch_move_the_camera_and_nothing_else() {
        let body = rect(640.0, 480.0);
        let mut camera = MapCamera::at([0.0, 0.0]);
        let start = camera;

        // **Pan** brings the world with the pointer: a drag right shows ground to the west.
        camera.pan_by_screen(64.0, 0.0, body);
        assert!(
            camera.centre_m[0] < start.centre_m[0],
            "{:?}",
            camera.centre_m
        );
        assert!((camera.centre_m[1] - start.centre_m[1]).abs() < 1e-9);
        camera.centre_m = start.centre_m;

        // **Zoom** about an anchor keeps the ground under it: the world point at the anchor is the
        // same before and after.
        let anchor = [body.x + 120.0, body.y + 300.0];
        let before = camera.ground_point(body, anchor[0], camera.depth_of(body, anchor[1]));
        camera.zoom_by(2.0, anchor, body);
        let after = camera.ground_point(body, anchor[0], camera.depth_of(body, anchor[1]));
        assert!((camera.pixels_per_m - start.pixels_per_m * 2.0).abs() < 1e-9);
        assert!((before[0] - after[0]).abs() < 1e-6 && (before[1] - after[1]).abs() < 1e-6);
        camera.pixels_per_m = start.pixels_per_m;

        // **Rotate** turns the heading a step at a time and wraps.
        camera.rotate_by(1.0);
        assert_eq!(camera.bearing_deg, MapCamera::BEARING_STEP_DEG);
        let east = camera.project(body, [100.0, 0.0, 0.0]);
        camera.bearing_deg = 0.0;
        let east_unrotated = camera.project(body, [100.0, 0.0, 0.0]);
        assert!(
            east[1] < east_unrotated[1] + 1e-9,
            "turning brings east up-screen"
        );

        // **Pitch** is clamped, and steeper pitch shows more ground for the same depth.
        for _ in 0..40 {
            camera.pitch_by(1.0);
        }
        assert_eq!(camera.pitch_deg, MapCamera::MAX_PITCH_DEG);
        for _ in 0..40 {
            camera.pitch_by(-1.0);
        }
        assert_eq!(camera.pitch_deg, MapCamera::MIN_PITCH_DEG);
    }

    #[test]
    fn the_raster_fills_the_body_and_the_marks_ride_the_same_camera() {
        let mut map = grid();
        map.follow([0.5, 0.5], 1.0, 90.0);
        let body = rect(320.0, 240.0);
        let camera = MapCamera::at([0.0, 0.0]);
        let ground = map.ground_elements(body, &camera, &ViewFilter::Any);
        let sky = 6.0;
        let paint = ground_pixels(&ground);
        let body_area = body.width_dp * body.height_dp;
        assert!(
            paint >= body_area * 0.98,
            "the raster leaves dead bands: {paint} of {body_area}"
        );
        assert!(
            ground.len() > sky as usize,
            "the ground is drawn, not only sky"
        );

        let marks = map.mark_elements(body, &camera, Some([6.5, -4.5]), None);
        assert_eq!(text_at(&marks, "@").len(), 1, "the marker is drawn");
        assert_eq!(
            text_at(&marks, "▶").len(),
            1,
            "the arrow follows the bearing"
        );
        assert_eq!(text_at(&marks, "◇").len(), 1, "the selection is drawn");
        assert!(
            marks.iter().any(|element| matches!(element, Element::Surface { paint, .. } if *paint == Paint::opaque(ROUTE_INK))),
            "the route is drawn in its own ink"
        );

        // A player the view cannot show is an edge indicator, not a missing marker.
        map.follow([4000.0, 0.0], 1.0, 0.0);
        let marks = map.mark_elements(body, &camera, None, None);
        assert!(text_at(&marks, "@").is_empty());
        assert_eq!(text_at(&marks, "◆").len(), 1);
        let edge = text_at(&marks, "◆")[0];
        assert!(body.contains((edge.x + 7.0, edge.y + 7.0)));
    }

    /// A flat survey, wide enough to cover a body at the camera's default scale, with a gentle
    /// slope so the surface's screen position moves from column to column.
    fn wide_grid(cell_m: f64, columns: usize, rows: usize) -> MapTerrain {
        let mut heights = Vec::with_capacity(columns * rows);
        for row in 0..rows {
            for column in 0..columns {
                heights.push(((column as f64) * 0.05 - (row as f64) * 0.02) as f32);
            }
        }
        MapTerrain {
            columns,
            rows,
            biome_columns: 8,
            biome_rows: 8,
            heights,
            colours: vec![GRASSLAND; columns * rows],
            kinds: vec![Some(Biome::Grassland); columns * rows],
            highest_m: 1.2,
            lowest_m: -0.6,
            centre_xz: [0.0, 0.0],
            cell_m,
            cell_row_m: cell_m,
            declared_m: 32.0,
            sea_level_m: -1000.0,
            pose: None,
            revision: 1,
        }
    }

    #[test]
    fn the_raster_covers_every_pixel_of_the_body() {
        // **No holes between the columns**: every pixel below the silhouette is the ground's, so
        // the sky shows only where the survey has no surface to draw. A raster that leaves a
        // column's row unpainted shows the sky through the ground, which is what a dot of
        // `SKY_LOW` in the middle of a field is.
        for (width, height) in [(320.0, 240.0), (960.0, 540.0)] {
            for pitch in [MapCamera::MIN_PITCH_DEG, 24.0, MapCamera::MAX_PITCH_DEG] {
                let body = rect(width, height);
                let camera = MapCamera {
                    pitch_deg: pitch,
                    ..MapCamera::at([0.0, 0.0])
                };
                let columns = ((body.width_dp / camera.pixels_per_m) / 20.0).ceil() as usize + 8;
                let rows = 512;
                let map = wide_grid(20.0, columns.max(8), rows);
                let ground = map.ground_elements(body, &camera, &ViewFilter::Any);
                // The body, painted in order, remembering only what the sky still owns.
                let (across, down) = (width as usize, height as usize);
                let mut sky = vec![false; across * down];
                for element in &ground {
                    let Element::Surface { rect, paint } = element else {
                        continue;
                    };
                    let band =
                        (0..6).any(|step| paint.rgb == mix(SKY_HIGH, SKY_LOW, step as f64 / 5.0));
                    let x0 = (rect.x - body.x).max(0.0) as usize;
                    let x1 = ((rect.right() - body.x).min(width as f64)).max(0.0) as usize;
                    let y0 = (rect.y - body.y).max(0.0) as usize;
                    let y1 = ((rect.bottom() - body.y).min(height as f64)).max(0.0) as usize;
                    for y in y0..y1.min(down) {
                        for x in x0..x1.min(across) {
                            sky[y * across + x] = band;
                        }
                    }
                }
                // **A sky pixel below the highest ground pixel of its own column is a hole**: the
                // ground is drawn from the silhouette down, so the sky has no business below it.
                let mut holes = 0usize;
                let mut examples = Vec::new();
                for x in 0..across {
                    let Some(top) = (0..down).find(|y| !sky[y * across + x]) else {
                        continue;
                    };
                    for y in top..down {
                        if sky[y * across + x] {
                            holes += 1;
                            if examples.len() < 4 {
                                examples.push((x, y));
                            }
                        }
                    }
                }
                assert_eq!(
                    holes, 0,
                    "{width}×{height} at {pitch}°: {holes} sky pixels below the ground; {examples:?}"
                );
            }
        }
    }

    #[test]
    fn the_survey_reaches_the_body_at_every_size_and_scale() {
        // **The streaming law**: whatever the window and whatever the scale, the survey covers the
        // ground the body can see, so the picture never stops short of the window's edge.
        for body in [rect(320.0, 240.0), rect(480.0, 440.0), rect(1920.0, 1080.0)] {
            for scale in [
                MapCamera::MIN_PIXELS_PER_M,
                0.45,
                2.0,
                MapCamera::MAX_PIXELS_PER_M,
            ] {
                for pitch in [MapCamera::MIN_PITCH_DEG, 24.0, MapCamera::MAX_PITCH_DEG] {
                    let camera = MapCamera {
                        pixels_per_m: scale,
                        pitch_deg: pitch,
                        ..MapCamera::at([123.0, -456.0])
                    };
                    let plan = camera.survey(body, 32.0);
                    let across = plan.columns as f64 * plan.sample_m;
                    let down = plan.rows as f64 * plan.sample_m;
                    assert!(
                        across + 1e-9 >= body.width_dp / scale,
                        "{body:?} at {scale} px/m shows {across} m of the {:.0} m it can see",
                        body.width_dp / scale
                    );
                    let foreshortened = pitch.to_radians().sin();
                    assert!(
                        down + 1e-9 >= body.height_dp / (scale * foreshortened),
                        "{body:?} at {scale} px/m and {pitch}° shows {down} m of the {:.0} m it can see",
                        body.height_dp / (scale * foreshortened)
                    );
                    // A cell is never sampled finer than a declared cell subdivided, so a close
                    // camera is sharp rather than merely small.
                    assert!(plan.sample_m + 1e-9 >= 32.0 / SUB as f64);
                }
            }
        }
    }

    #[test]
    fn a_view_selection_lights_its_own_ground_and_shadows_the_rest() {
        // The vocabulary itself: what each token keeps.
        let land = Some(Biome::Grassland);
        assert!(ViewFilter::of_token(None).lit(land));
        assert!(
            ViewFilter::of_token(Some("all")).lit(None),
            "the default view filters nothing"
        );
        assert!(
            !ViewFilter::of_token(Some("ocean")).lit(land),
            "ocean is the water, not the coast"
        );
        assert!(
            ViewFilter::of_token(Some("ocean")).lit(None),
            "water is what ocean keeps"
        );
        assert!(ViewFilter::of_token(Some("mountain")).lit(Some(Biome::Alpine)));
        assert!(!ViewFilter::of_token(Some("mountain")).lit(land));
        assert!(
            ViewFilter::of_token(Some("nonsense")).lit(land),
            "a token that selects nothing must not look like it selected something"
        );

        // And on the raster: a selection changes the picture, and keeps less of it. **The survey
        // covers the body**: 320 × 240 dp at 0.45 px/m against a 24° tilt is 711 m across and
        // 1 311 m down the ground, so a fixture of metres would be a picture of sky.
        let mut map = wide_grid(32.0, 25, 43);
        for row in (map.rows / 2)..map.rows {
            for column in (map.columns / 2)..map.columns {
                map.kinds[row * map.columns + column] = Some(Biome::Alpine);
            }
        }
        let body = rect(320.0, 240.0);
        let camera = MapCamera::at([0.0, 0.0]);
        let everything = map.ground_elements(body, &camera, &ViewFilter::Any);
        let alpine = map.ground_elements(body, &camera, &ViewFilter::of_token(Some("mountain")));
        assert_ne!(
            everything, alpine,
            "the selection left the picture unchanged"
        );
        // **The ground the selection names keeps its own ink, and the rest is put into shadow** —
        // read at a grassland cell and an alpine one, on the ground element that cell projects to.
        // **A selection dims the picture and lights nothing it did not already have**: the
        // brightest ink is the same in both, because the ground the selection names keeps it, and
        // the average is darker, because everything else is in shadow. The sky is not ground.
        let ground_of = |elements: &[Element]| -> Vec<(Rect, f64)> {
            elements
                .iter()
                .filter_map(|element| match element {
                    Element::Surface { rect, paint }
                        if !(0..6)
                            .any(|step| paint.rgb == mix(SKY_HIGH, SKY_LOW, step as f64 / 5.0)) =>
                    {
                        Some((*rect, f64::from(paint.rgb.luminance())))
                    }
                    _ => None,
                })
                .collect()
        };
        // **The ground a view keeps, counted by the row steps it reaches in each column** rather
        // than by summing rectangles: a raster merges neighbouring rows that share an ink and its
        // runs overlap on purpose, so a sum of areas moves with the picture's *ink* — two views of
        // one ground would disagree by the merge. The covered steps are what the selection must
        // not change: an ink is not a place.
        let covered = |elements: &[Element]| -> usize {
            let mut steps = 0usize;
            for (rect, _) in ground_of(elements) {
                // A run's rectangle is padded by half a row step at each end and by a whole one
                // below (the overlap that hides the seam), so its own rows are read back out of
                // it: the first and last row centres it was merged from.
                let first = (rect.y + ROW_STEP_DP / 2.0 - body.y) / ROW_STEP_DP;
                let last = (rect.y + rect.height_dp - ROW_STEP_DP * 1.5 - body.y) / ROW_STEP_DP;
                let (first, last) = (first.round(), last.round());
                if last >= first {
                    steps += (last - first + 1.0) as usize;
                }
            }
            steps
        };
        let measures = |elements: &[Element]| -> (f64, f64) {
            let ground = ground_of(elements);
            let area: f64 = ground
                .iter()
                .map(|(rect, _)| rect.width_dp * rect.height_dp)
                .sum();
            let sum: f64 = ground
                .iter()
                .map(|(rect, lum)| rect.width_dp * rect.height_dp * lum)
                .sum();
            let brightest = ground.iter().map(|(_, lum)| *lum).fold(0.0f64, f64::max);
            (sum / area.max(1e-9), brightest)
        };
        let lit = |elements: &[Element], brightest: f64| -> f64 {
            ground_of(elements)
                .iter()
                .filter(|(_, lum)| *lum >= brightest - 1e-9)
                .map(|(rect, _)| rect.width_dp * rect.height_dp)
                .sum()
        };
        let (everything_mean, everything_brightest) = measures(&everything);
        let (alpine_mean, alpine_brightest) = measures(&alpine);
        assert_eq!(
            covered(&alpine),
            covered(&everything),
            "a selection moves ground out of the picture"
        );

        assert!(
            alpine_brightest <= everything_brightest + 1e-9,
            "a selection lights the survey brighter than it already was: {alpine_brightest} against {everything_brightest}"
        );
        assert!(
            alpine_brightest > everything_mean,
            "the ground the selection names is drawn, and brighter than the survey's average: \
             {alpine_brightest} against {everything_mean}"
        );
        assert!(
            alpine_mean < everything_mean,
            "a selection that changed no ink is not a selection: {alpine_mean} against {everything_mean}"
        );
        // **The ink that survives is exactly the named ground's**: every sample the selection keeps
        // at full brightness is one the survey itself calls alpine, which the fixture paints in a
        // quadrant — so a raster that kept the wrong ground would show an ink the survey never had
        // there, and the count below would be the whole picture rather than a quarter of it.
        let lit_all = lit(&everything, alpine_brightest);
        let lit_alpine = lit(&alpine, alpine_brightest);
        assert!(lit_alpine > 0.0, "the selection lit nothing at all");
        assert!(
            lit_alpine < lit_all * 0.9,
            "a selection that keeps nine tenths of the picture is not a selection: \
             {lit_alpine} of {lit_all}"
        );
    }

    /// **A pointer over the ground names the place it is over, at any tilt.** The projection
    /// foreshortens the ground by the pitch, so the screen is not a plan: at 8 px/m against a 24°
    /// tilt a metre of forward is over three pixels up the body, and an inverse that treated a
    /// pixel as a fixed number of metres would answer with a place six times nearer the camera
    /// than the one the pointer is on — a click landing in the wrong cell, a readout naming a
    /// column the viewer is not pointing at.
    #[test]
    fn the_pointer_picks_the_world_column_a_tilted_camera_is_pointing_at() {
        let map = wide_grid(1.0, 400, 400);
        let body = rect(320.0, 240.0);
        let camera = MapCamera {
            pixels_per_m: 8.0,
            ..MapCamera::at([0.0, 0.0])
        };
        for cell in [(185, 220), (215, 190), (200, 215), (190, 185)] {
            // The sample's own column, not the declared cell it is named as: this is a test of the
            // projection, and the naming is a separate act.
            let [x, z] = map.sample_xz(cell);
            let height = map.height(cell).expect("ground");
            let at = camera.project(body, [x, height, z]);
            assert!(
                body.contains((at[0], at[1])),
                "cell {cell:?} is in the body"
            );
            let picked = map
                .cell_at_pointer(body, (at[0], at[1]), &camera)
                .expect("a cell under the pointer");
            assert_eq!(
                picked, cell,
                "the pointer at {at:?} names {picked:?}, not the column {cell:?} under it"
            );
        }
    }

    #[test]
    fn the_pointer_picks_through_the_camera_and_the_readout_is_the_ground_it_lands_on() {
        let map = grid();
        let body = rect(320.0, 240.0);
        let camera = MapCamera::at([0.0, 0.0]);
        // Put the pointer on a sample's projected position, and the pick must name that sample.
        let cell = (16, 12);
        let world = map.world_xyz(cell).expect("a world position");
        let at = camera.project(body, world);
        let picked = map
            .cell_at_pointer(body, (at[0], at[1]), &camera)
            .expect("a cell");
        assert_eq!(picked, cell);
        assert_eq!(map.world_xyz(picked), Some(world));
        // Off the body is no cell; far outside the survey is no cell either.
        assert_eq!(map.cell_at_pointer(body, (0.0, 0.0), &camera), None);
        let far = camera.project(body, [9000.0, 0.0, 0.0]);
        assert_eq!(map.cell_at_pointer(body, (far[0], far[1]), &camera), None);
    }

    /// **The brush is the pointer's, the mark freezes it, and the stroke is the disk it would
    /// draw** — the whole ladder, read through the state a gutter button moves.
    #[test]
    fn the_hammer_stages_a_disk_and_a_mark_freezes_where_it_lands() {
        let mut brush = MapBrush::new();
        // No pointer has been over the body: the verbs that need a place refuse by name rather
        // than staging a stroke at the origin.
        assert!(brush.centre().is_none());
        assert!(brush.target().is_none());
        assert!(brush.mark().unwrap_err().contains("no brush"));
        assert!(brush.raise().unwrap_err().contains("no target"));

        brush.set_at([4.5, -2.5]);
        // The ladder is one list read both ways, and its ends hold.
        assert_eq!(brush.wider(), 8.0);
        assert_eq!(brush.narrower(), 4.0);
        assert_eq!(brush.narrower(), 2.0);
        assert_eq!(
            brush.narrower(),
            2.0,
            "the narrowest brush is the ladder's first"
        );
        assert_eq!(brush.wider(), 4.0);
        assert_eq!(brush.target(), Some(([4.5, -2.5], 4.0)));

        // **A mark is a place, not a live view**: the pointer moves on and the stroke stays where
        // the person put it.
        let marked = brush.mark().expect("a brush is marked");
        brush.set_at([40.0, 40.0]);
        assert_eq!(brush.target(), Some(marked));

        assert_eq!(brush.raise(), Ok(1));
        assert_eq!(brush.raise(), Ok(2));
        let stroke = brush.edit().expect("a stroke is staged");
        assert_eq!(stroke.lift, 2);
        assert_eq!(
            stroke.columns,
            columns_within([4.5, -2.5], 4.0),
            "the stroke is the ring's own columns"
        );
        // Every column's own centre is inside the radius, and the list is the whole disk: a ring
        // that drew 50 m and struck 40 would be a control that lied.
        assert!(
            stroke.columns.len() > 40,
            "{} columns",
            stroke.columns.len()
        );
        for [x, z] in &stroke.columns {
            let dx = f64::from(*x) + 0.5 - 4.5;
            let dz = f64::from(*z) + 0.5 + 2.5;
            assert!(
                dx * dx + dz * dz <= 16.0 + 1e-9,
                "column {x},{z} is off the disk"
            );
        }
        // A stroke is staged once, and after it the buttons say so rather than repeating it.
        assert_eq!(brush.lift(), 0);
        assert!(brush.edit().is_none());
        assert!(brush.raise().is_ok(), "the mark is still the target");
        assert_eq!(brush.lift(), 1);
        brush.clear();
        assert!(!brush.marked(), "clear forgets the mark");
        assert!(brush.edit().is_none(), "and the stroke");

        // **The staged lift is bounded**, because one press is a stroke and not a batch job.
        for _ in 0..MAX_HAMMER_LIFT {
            let _ = brush.raise();
        }
        let refusal = brush.raise().unwrap_err();
        assert!(
            refusal.contains(&format!("+{MAX_HAMMER_LIFT}")),
            "{refusal}"
        );
        assert_eq!(brush.lift(), MAX_HAMMER_LIFT);
    }

    /// **The rings are drawn on the ground the stroke would edit**: the brush's own width in the
    /// picture, the mark's on top of it, and every point inside the body.
    #[test]
    fn the_hammer_draws_the_brush_and_the_mark_at_their_own_widths() {
        let map = grid();
        let body = rect(640.0, 480.0);
        let camera = MapCamera::at([0.0, 0.0]);
        let mut brush = MapBrush::new();
        brush.set_at([0.5, 0.5]);
        brush.wider();
        assert_eq!(brush.radius_m(), 8.0);
        let brush_rings = brush.elements(&map, body, &camera);
        assert!(!brush_rings.is_empty());
        for element in &brush_rings {
            let at = element.rect();
            assert!(
                body.contains((at.x, at.y)) && body.contains((at.right(), at.bottom())),
                "a ring point escapes the body: {element:?}"
            );
            assert!(
                matches!(element, Element::Surface { paint, .. } if *paint == Paint::opaque(BRUSH_INK)),
                "{element:?}"
            );
        }
        // **The ring is the brush's width in the picture**, not a glyph fixed at any zoom: an
        // 8 m brush at the default scale spans 2·8·0.45 pixels east to west, plus the dot it is
        // drawn with.
        let span = brush_rings
            .iter()
            .map(|element| element.rect())
            .fold(f64::MIN, |wide, at| wide.max(at.right()))
            - brush_rings
                .iter()
                .map(|element| element.rect())
                .fold(f64::MAX, |wide, at| wide.min(at.x));
        let expected = 2.0 * 8.0 * camera.pixels_per_m;
        assert!(
            (span - expected).abs() < 4.0,
            "an 8 m brush spans {span:.1} px of {expected:.1}"
        );

        // The mark is a second ring, in its own ink, over the first.
        brush.mark().expect("marked");
        let marked = brush.elements(&map, body, &camera);
        assert!(marked.len() > brush_rings.len(), "the mark added no ring");
        assert!(marked
            .iter()
            .any(|element| matches!(element, Element::Surface { paint, .. } if *paint == Paint::opaque(MARKED_INK))));
    }

    #[test]
    fn the_arrow_rounds_each_bearing_to_its_own_lobe() {
        for (bearing, arrow) in [
            (0.0, Arrow::North),
            (45.0, Arrow::NorthEast),
            (90.0, Arrow::East),
            (135.0, Arrow::SouthEast),
            (180.0, Arrow::South),
            (225.0, Arrow::SouthWest),
            (270.0, Arrow::West),
            (315.0, Arrow::NorthWest),
            (350.0, Arrow::North),
            (-90.0, Arrow::West),
            (f64::NAN, Arrow::North),
        ] {
            assert_eq!(arrow_of_degrees(bearing), arrow, "{bearing}");
        }
    }
}
