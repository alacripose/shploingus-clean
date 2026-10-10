//! The cloud field — **what the GPU march reads**, and where it comes from.
//!
//! The cloud model is [`voxel_shapers::clouds`] and it exists once, in Rust: a column's base,
//! top, condensed water and kind are derived from terrain, climate, weather and the sun. What
//! travels to the GPU is that evaluated record, one texel per world column, and the shader
//! reconstitutes the vertical profile [`voxel_shapers::clouds::cloud_at`] defines. Nothing is
//! stored between refreshes (C-2).
//!
//! # The field is also a participating medium
//!
//! The cloud is not only something to be looked at: a beam of the Sun that passes through one
//! is **attenuated along the path inside it**, because a cloud is `OpticalClass::Scattering` —
//! a medium with no surface to stop anything at. [`CloudMedium`] is that answer for
//! `voxel_stream::light::Matter`, and [`cloud_transmittance`] is the same law in closed form
//! for a whole column, so the sampler's walk and a column's own water path are held to each
//! other rather than both trusted. `the_worlds_own_clouds_are_mostly_transparent` measures the
//! ruling that makes them a veil.
//!
//! # Why a field and not a per-sample shader
//!
//! The sky can be a table because it is a function of *direction* alone. A cloud is not: it
//! depends on where the column is and on the surface parcel's own state, so a per-pixel
//! evaluation of it in WGSL would mean porting terrain, climate, weather and the solar search
//! into a second language — two copies of one model, which is the drift this repository
//! refuses. So the model runs here, once per column, and the GPU integrates it.

use voxel_appearance::npr::LightRig;
use voxel_model::{MaterialId, SubvoxelIndex, SUBVOXEL_SIDE};
use voxel_shapers::{clouds, WorldSpec};
use voxel_stream::light::Matter;

/// Columns per side of one cloud block.
///
/// **It is a count, and it is the only thing a cloud block shares with a world chunk.** The
/// earlier reading here — *"the world's own chunk side, so a cloud block and a world chunk are
/// addressed the same way"* — was **false, and measurably so**. One voxel is one metre
/// (`voxel_shapers::fluidics` states the scale contract), so a chunk is
/// [`voxel_model::CHUNK_SIDE`] = **32 m**, while a cloud **column** is [`CLOUD_COLUMN_M`] =
/// **64 m** and a cloud **block** is [`CLOUD_BLOCK_M`] = **2,048 m**. A cloud block is therefore
/// **64 chunks across**, not one, and a cloud column **straddles two chunks**. The number 32 is
/// held in common; no extent is.
///
/// An address is a **cuboid cell** on the cloud's own lattice, and the three constants below say
/// what that lattice is in the world's own chunk units — which is what a reader needs instead of
/// the sentence that used to be here. `the_cloud_lattice_is_stated_in_chunks_and_is_not_the_chunk_lattice`
/// is the witness, and it also records the one place the cloud lattice and the horizon ladder
/// disagree: this field reaches further than the widest horizon the tree declares.
pub const CLOUD_BLOCK_SIDE: u32 = 32;

/// Blocks per axis in one field.
pub const CLOUD_BLOCKS_PER_AXIS: u32 = 4;

/// Columns per axis in one field.
pub const CLOUD_COLUMNS: u32 = CLOUD_BLOCK_SIDE * CLOUD_BLOCKS_PER_AXIS;

/// Metres across one block.
pub const CLOUD_BLOCK_M: f64 = 2_048.0;

/// Metres per column: 64, which is a third of the shortest cloud depth the model makes
/// ([`clouds`]'s stratus is `STRATUS_DEPTH_M` 500 m at full moisture).
pub const CLOUD_COLUMN_M: f64 = CLOUD_BLOCK_M / CLOUD_BLOCK_SIDE as f64;

/// **How many columns apart the model is actually asked** — the field's sample stride.
///
/// The model is the expensive half of a fill, and it is expensive in a way that is easy to
/// miss: [`CloudField::sample`] calls `clouds::cloud_column` once per column it is asked
/// about, and one call is terrain, climate, weather and the solar search. At one ask per
/// texel that is 16,384 calls for a field whose *own* declared sample is 64 m — the field
/// was being computed at a rate nothing then used, because the shader's march throws the
/// between-texel detail away and re-interpolates it anyway.
///
/// So the rate is declared, and the texels between two samples are **interpolated**
/// ([`CloudField::sample`] does it, and `the_between_is_interpolated_never_a_second_ask`
/// is the witness). This is the pattern `voxel_shapers::fluidics` is built on: the stored
/// state is one level per cell on a declared lattice, and the appearance layer
/// interpolates between cells rather than the state being kept at display resolution.
///
/// **2, and the number is a measurement rather than a preference**: the fill's cost falls
/// as the square, and the measurement test below prints what the stride does to the fill's
/// time, to the field's agreement with the full-rate model, and to how many columns the
/// interpolation turns cloudy that the model did not. Something coarser is a decision to
/// make against those numbers.
pub const CLOUD_SAMPLE_STRIDE: u32 = 2;

/// Samples per axis: 65 — every second column, and the field's **last** column pinned as
/// the final sample so the lattice covers the field's own extent instead of stopping one
/// stride short of its edge.
pub const CLOUD_SAMPLES_PER_AXIS: u32 = CLOUD_COLUMNS / CLOUD_SAMPLE_STRIDE + 1;

/// Metres between two columns the model is asked about: 128, **four world chunks** — and
/// the lattice stays a whole number of chunks, which is the property that lets a chunk
/// owner say which samples it needs without a fraction anywhere.
pub const CLOUD_SAMPLE_M: f64 = CLOUD_COLUMN_M * CLOUD_SAMPLE_STRIDE as f64;

/// The field's span, metres: 8,192, four blocks a side.
pub const CLOUD_SPAN_M: f64 = CLOUD_BLOCK_M * CLOUD_BLOCKS_PER_AXIS as f64;

/// **How many world chunks one cloud column spans.** `2`, exactly — which is the whole of the
/// difference between the cloud lattice and the chunk lattice, and the reason a cloud cell cannot
/// be addressed by a bare `ChunkPos` today.
pub const CLOUD_COLUMN_CHUNKS: i32 = (CLOUD_COLUMN_M as i32) / voxel_model::CHUNK_SIDE;

/// **How many world chunks one cloud block spans.** `64`.
pub const CLOUD_BLOCK_CHUNKS: i32 = (CLOUD_BLOCK_M as i32) / voxel_model::CHUNK_SIDE;

/// **The field's reach from the eye, in chunks** — the number a horizon ladder has to be compared
/// with, carried as a constant because the comparison is the interesting fact: the widest horizon
/// this tree declares is `voxel_stream::horizons`' `Horizon::Field` at a radius of **96** chunks,
/// and this field reaches **128**. So the cloud field is **not inside the horizon ladder** — it is
/// wider than the widest horizon there is, which is what makes it un-streamable as it stands:
/// there is no horizon that could bound which cells a refresh should touch.
pub const CLOUD_HALF_EXTENT_CHUNKS: i32 =
    (CLOUD_SPAN_M as i32) / 2 / voxel_model::CHUNK_SIDE;

/// **The march's step length**, metres — and it is the field's own column, because that is the
/// finest thing the model was asked about: a step coarser than a column reads past the cloud it
/// should be inside. 120 m measured as a stipple, which is aliasing rather than texture.
pub const CLOUD_STEP_M: f32 = 64.0;

/// **Extinction per metre at water `1.0`**, declared: `1/100 m`, so a saturated metre-long
/// path of the field's own water unit turns the column opaque in about a kilometre.
pub const CLOUD_EXTINCTION_PER_M: f32 = 0.01;

/// The cloud's single-scattering albedo in the visible — a real number for water droplets,
/// and the reason a cloud is white rather than grey.
pub const CLOUD_ALBEDO: f64 = 0.9;

/// **Surfaces above this altitude are not marched to**, metres: a ray that has travelled
/// 8 km has crossed the whole field and is behind the fog the sky already draws.
pub const CLOUD_MAX_DISTANCE_M: f32 = 8_000.0;

/// One field: the columns, and the slab they occupy.
///
/// `PartialEq` is written out rather than derived, and that is the determinism contract
/// rather than a style: [`CloudField::filled_micros`] is a **measurement of this machine**, and
/// a derived `==` would compare it, so two calls to [`CloudField::sample`] with identical
/// inputs would be unequal whenever the clock differed by a microsecond — which is always.
/// Equality is what the model produced, not how long it took to produce it.
#[derive(Clone, Debug)]
pub struct CloudField {
    /// The south-west corner of the field, metres, **snapped to a column** so a field that
    /// follows the eye does not slide its own texels under the world.
    pub origin_m: [f64; 2],
    /// One texel per column: `[water, base_m, top_m, kind]`, kind indexed into
    /// [`clouds::CloudKind::ALL`].
    pub columns: Vec<[f32; 4]>,
    /// The lowest cloud base in the field, metres, or `0.0` when the field is clear.
    pub slab_base_m: f64,
    /// The highest cloud top in the field, metres, or `0.0` when the field is clear.
    pub slab_top_m: f64,
    /// How many columns hold a cloud — the field's own report, so a picture of an empty sky
    /// can say whether the model or the pass is responsible.
    pub cloudy: usize,
    /// How long the fill took, microseconds. Carried because it is the cost of the
    /// atmosphere's cadence, and a number that is measured can be argued with.
    pub filled_micros: u128,
}

impl PartialEq for CloudField {
    fn eq(&self, other: &Self) -> bool {
        self.origin_m == other.origin_m
            && self.columns == other.columns
            && self.slab_base_m == other.slab_base_m
            && self.slab_top_m == other.slab_top_m
            && self.cloudy == other.cloudy
    }
}

impl CloudField {
    /// The field the model makes around `centred_on_m`, at `tick`.
    ///
    /// **Two lattices, one ask each.** The model is asked on the coarse sample lattice —
    /// [`CLOUD_SAMPLES_PER_AXIS`]² columns, one per [`CLOUD_SAMPLE_STRIDE`] texels — and
    /// every texel between two samples is **[`blend`]ed from the four around it**. A texel
    /// that *is* a sample carries the model's column exactly (the test compares it with
    /// `clouds::cloud_column`, float for float); a texel between two carries an
    /// interpolation of them. What the interpolation may and may not do is `blend`'s own
    /// rule: **the coverage blends, the layer does not**.
    ///
    /// The field's cost is therefore quadratic in the rate: 4,225 asks at the declared
    /// stride against 16,384 at one ask per texel, for the same 8,192 m field.
    pub fn sample(spec: &WorldSpec, tick: u64, centred_on_m: [f64; 2]) -> Self {
        let started = std::time::Instant::now();
        let origin_m = [
            snap(centred_on_m[0] - CLOUD_SPAN_M / 2.0),
            snap(centred_on_m[1] - CLOUD_SPAN_M / 2.0),
        ];
        let n = CLOUD_COLUMNS as usize;
        let m = CLOUD_SAMPLES_PER_AXIS as usize;

        // ---- the sample lattice: the model, asked once per sample ---------------
        let mut samples = vec![[0.0f32; 4]; m * m];
        let mut asks = 0usize;
        for jk in 0..m {
            for ik in 0..m {
                asks += 1;
                let i = sample_index(ik, n);
                let j = sample_index(jk, n);
                let x = origin_m[0] + (i as f64 + 0.5) * CLOUD_COLUMN_M;
                let z = origin_m[1] + (j as f64 + 0.5) * CLOUD_COLUMN_M;
                // The model's own existence test. A column it declines is not a cloud, so the
                // sample stays clear rather than carrying base and top for a layer that is not
                // there.
                let Some(column) = clouds::cloud_column(spec, x, z, tick) else {
                    continue;
                };
                samples[jk * m + ik] = [
                    column.water as f32,
                    column.base_m as f32,
                    column.top_m as f32,
                    kind_code(column.kind) as f32,
                ];
            }
        }
        // The rate is the point of the stride, so it is pinned where it is spent rather
        // than left to a reader to count. Debug-only: it is a property of this loop, and
        // a release frame should not pay for it.
        debug_assert_eq!(
            asks,
            m * m,
            "the sample loop must ask the model once per sample and no more"
        );

        // ---- the field's texels: interpolated, never a second ask --------------
        let mut columns = vec![[0.0f32; 4]; n * n];
        let mut cloudy = 0usize;
        let mut slab_base_m = f64::INFINITY;
        let mut slab_top_m = f64::NEG_INFINITY;
        for j in 0..n {
            let (j0, j1, t_j) = corner(j, n, m);
            for i in 0..n {
                let (i0, i1, t_i) = corner(i, n, m);
                let texel = blend(
                    samples[j0 * m + i0],
                    samples[j0 * m + i1],
                    samples[j1 * m + i0],
                    samples[j1 * m + i1],
                    t_i,
                    t_j,
                );
                if texel[0] <= 0.0 {
                    continue;
                }
                columns[j * n + i] = texel;
                cloudy += 1;
                slab_base_m = slab_base_m.min(texel[1] as f64);
                slab_top_m = slab_top_m.max(texel[2] as f64);
            }
        }
        if cloudy == 0 {
            slab_base_m = 0.0;
            slab_top_m = 0.0;
        }
        CloudField {
            origin_m,
            columns,
            slab_base_m,
            slab_top_m,
            cloudy,
            filled_micros: started.elapsed().as_micros(),
        }
    }

    /// The texel for the column at grid position `(i, j)`, or `None` off the field.
    pub fn texel(&self, i: u32, j: u32) -> Option<[f32; 4]> {
        if i >= CLOUD_COLUMNS || j >= CLOUD_COLUMNS {
            return None;
        }
        Some(self.columns[j as usize * CLOUD_COLUMNS as usize + i as usize])
    }

    /// **The field asked between its own texels**, at a world `(x, z)`.
    ///
    /// The rule is [`blend`]'s, applied to the texel grid rather than to the sample lattice: the
    /// **nearest texel decides whether there is a cloud at all** — coverage is a step, because the
    /// model's own existence test is one and a bilinear blend of it invents a haze in every
    /// address that merely neighbours a cloud — and the wet texels around the address carry their
    /// density and their layer **bilinearly, re-normalised over the wet ones**, so a clear texel
    /// cannot drag a cloud's base toward an altitude nobody measured.
    ///
    /// **Why a second interpolation exists.** The texels are on a 64 m grid and the model's
    /// samples on a 128 m one, so a cloud drawn at its texels' own resolution has 64 m steps in its
    /// outline — which is what the first picture showed, columns of cloud with square corners. The
    /// geometry may not ask the model (that is a fill's cost), and it must not invent a cloud
    /// (that is what the nearest-texel rule prevents), but it may ask the field *here*, and a cut
    /// on this value follows the field smoothly inside a cloud's footprint.
    ///
    /// Off the field it is clear, which is the same answer the field's own edge has: the field is
    /// `CLOUD_SPAN_M` across and there is nothing measured outside it.
    pub fn at_m(&self, x_m: f64, z_m: f64) -> [f32; 4] {
        if !x_m.is_finite() || !z_m.is_finite() {
            return [0.0; 4];
        }
        // Texel centres are at `origin + (i + 0.5) · column`, so the fractional index of a texel
        // *centre* is this; the corners around the address are then `floor` and `floor + 1`.
        let fi = (x_m - self.origin_m[0]) / CLOUD_COLUMN_M - 0.5;
        let fj = (z_m - self.origin_m[1]) / CLOUD_COLUMN_M - 0.5;
        let (i0, j0) = (fi.floor(), fj.floor());
        let t_i = (fi - i0).clamp(0.0, 1.0);
        let t_j = (fj - j0).clamp(0.0, 1.0);
        let corner = |di: f64, dj: f64| -> [f32; 4] {
            let (i, j) = (i0 + di, j0 + dj);
            if i < 0.0 || j < 0.0 || i >= CLOUD_COLUMNS as f64 || j >= CLOUD_COLUMNS as f64 {
                return [0.0; 4];
            }
            self.texel(i as u32, j as u32).unwrap_or([0.0; 4])
        };
        blend(
            corner(0.0, 0.0),
            corner(1.0, 0.0),
            corner(0.0, 1.0),
            corner(1.0, 1.0),
            t_i,
            t_j,
        )
    }

    /// **The local water at one world altitude, at an address the caller picks** — the model's
    /// profile, asked through [`density_of`] at a point finer than the field's lattice.
    ///
    /// This is the number a drawn cloud's edge is cut on: `clouds`' own doc says the rendered edge
    /// is *"where the appearance layer chooses to cut it, not a step in this function"*, and this is
    /// the function it means. Zero outside the layer and outside the field, so a cut on it can
    /// never draw a cell where the model has no cloud.
    pub fn density_at_m(&self, x_m: f64, z_m: f64, y_m: f64) -> f64 {
        density_of(self.at_m(x_m, z_m), y_m)
    }

    /// **The water path one cell stands for**, metres: the profile `water · sin(π · through)`
    /// integrated **exactly** over the cell's own `[y, y_top]`.
    ///
    /// Analytic rather than a midpoint sample, and that is what makes the cells' shares
    /// **telescope**: over the whole layer the cosines cancel to `2`, so the sum is
    /// `water · 2 · L / π` — `clouds::column_water_path_m`'s own closed form — however the layer is
    /// cut into cells. A midpoint reading would leave a percent or so of the path behind at the
    /// profile's own curvature, and the picture would then owe the shadow a number it never quite
    /// pays. A `sample` outside the layer gives `0`.
    pub fn cell_water_path_m(sample: [f32; 4], y_m: f64, y_top_m: f64) -> f64 {
        let water = sample[0] as f64;
        let base_m = sample[1] as f64;
        let top_m = sample[2] as f64;
        if water <= 0.0 || top_m <= base_m {
            return 0.0;
        }
        let layer_m = top_m - base_m;
        let pi = std::f64::consts::PI;
        let through = |y: f64| ((y - base_m) / layer_m).clamp(0.0, 1.0);
        water * layer_m / pi * ((pi * through(y_m)).cos() - (pi * through(y_top_m)).cos())
    }

    /// The world `(x, z)` of the centre of the column at `(i, j)`.
    pub fn column_centre_m(&self, i: u32, j: u32) -> [f64; 2] {
        [
            self.origin_m[0] + (i as f64 + 0.5) * CLOUD_COLUMN_M,
            self.origin_m[1] + (j as f64 + 0.5) * CLOUD_COLUMN_M,
        ]
    }
}

/// The sample lattice index a texel's own axis position reads, clamped to the field's last
/// column so the final sample is pinned inside the field rather than one stride outside it.
fn sample_index(k: usize, n: usize) -> usize {
    (k * CLOUD_SAMPLE_STRIDE as usize).min(n - 1)
}

/// The two samples a texel sits between, and how far it is from the first — `t` in
/// `[0, 1]`, and `0.0` for the degenerate cell at the field's edge.
fn corner(i: usize, n: usize, m: usize) -> (usize, usize, f64) {
    let k = (i / CLOUD_SAMPLE_STRIDE as usize).min(m - 1);
    let k1 = (k + 1).min(m - 1);
    let first = sample_index(k, n) as f64;
    let second = sample_index(k1, n) as f64;
    let t = if second > first {
        (i as f64 - first) / (second - first)
    } else {
        0.0
    };
    (k, k1, t)
}

/// **The value at a texel, from the four samples around it.**
///
/// The rule, and every clause of it was chosen against the measurement
/// (`the_stride_is_reported_against_the_full_rate_model` builds the field both ways and
/// prints what each costs):
///
/// * **Coverage is the nearest sample's, not a blend of cloudy and clear.** The model's own
///   existence test is a **step** — `clouds::cloud_column` answers `None`, not a small
///   number — so a bilinear blend of it invents a thin cloud in every texel that merely
///   *neighbours* a sample with a cloud in it. Measured at stride 2, that widened a 28 %-covered
///   field to 43 %: 3,030 texels called cloudy that the model does not cloud, each one a
///   haze where the model has an edge. The nearest sample decides instead, and the cost is
///   the honest one: a texel whose nearest sample is clear is **clear**, even if a further
///   corner in the same cell holds a cloud — which is the model's edge landing up to half a
///   stride from where the model put it.
/// * **The density and the layer are the wet samples'**, weighted by the same bilinear
///   weights and **re-normalised over the corners that hold a cloud**. Air has no base and
///   no density, so a clear corner contributes nothing instead of dragging a cloud's base
///   toward an altitude nobody measured. A texel whose dominant sample is the only wet
///   corner therefore takes that sample's own column **exactly** — density, base, top and
///   class.
/// * **A class is never a blend of two classes.** `kind` is the **dominant wet corner's
///   own**, because `CloudKind::ALL`'s order is not a scale: the midpoint of fog and stratus
///   is `1u`, which is cumulus, a class neither sample named. Ties go to the earlier corner,
///   for the reason `SamplingPolicy::Nearest` breaks its ties toward the greater coordinate:
///   a deterministic answer is the requirement, the direction is the declaration.
fn blend(
    c00: [f32; 4],
    c10: [f32; 4],
    c01: [f32; 4],
    c11: [f32; 4],
    t_i: f64,
    t_j: f64,
) -> [f32; 4] {
    let corners = [c00, c10, c01, c11];
    let weights = [
        (1.0 - t_i) * (1.0 - t_j),
        t_i * (1.0 - t_j),
        (1.0 - t_i) * t_j,
        t_i * t_j,
    ];
    // The nearest sample: the corner this texel sits closest to, ties to the earlier one.
    let mut dominant = -1.0f64;
    let mut nearest = 0usize;
    for (index, weight) in weights.iter().enumerate() {
        if *weight > dominant {
            dominant = *weight;
            nearest = index;
        }
    }
    if corners[nearest][0] <= 0.0 {
        return [0.0; 4];
    }
    let wet: f64 = weights
        .iter()
        .zip(corners.iter())
        .filter(|(_, corner)| corner[0] > 0.0)
        .map(|(weight, _)| weight)
        .sum();
    let mut water = 0.0f64;
    let mut base_m = 0.0f64;
    let mut top_m = 0.0f64;
    let mut kind = corners[nearest][3];
    let mut heaviest = -1.0f64;
    for (weight, corner) in weights.iter().zip(corners.iter()) {
        if corner[0] <= 0.0 {
            continue;
        }
        let share = weight / wet;
        water += share * corner[0] as f64;
        base_m += share * corner[1] as f64;
        top_m += share * corner[2] as f64;
        if *weight > heaviest {
            heaviest = *weight;
            kind = corner[3];
        }
    }
    [water as f32, base_m as f32, top_m as f32, kind]
}

/// **The local water at one world altitude**, as the shader recomputes it from a texel:
/// `water · sin(π · through)`, and zero outside the layer.
///
/// This is the reference the WGSL profile is a transcription of, and it is held to the model
/// itself by `the_field_reproduces_the_models_own_point_query`, which compares it against
/// `clouds::cloud_at` — so a drift in either the shaping or the reconstruction lands on a test.
pub fn density_of(texel: [f32; 4], y_m: f64) -> f64 {
    let water = texel[0] as f64;
    let base_m = texel[1] as f64;
    let top_m = texel[2] as f64;
    if water <= 0.0 || top_m <= base_m || y_m < base_m || y_m >= top_m {
        return 0.0;
    }
    let through = (y_m - base_m) / (top_m - base_m);
    water * (std::f64::consts::PI * through).sin()
}

/// **The cloud's extinction at one local water content**, per metre: the declared
/// [`CLOUD_EXTINCTION_PER_M`] scaled by how much water the address holds.
///
/// Linear in the water, because a participating medium's extinction is proportional to how
/// much of it there is; the reconstruction of the water from a field texel at an altitude is
/// [`density_of`]'s. The rate belongs to the **address** and not to a material id, which is
/// why `voxel_stream::light::Matter` asks a second question for it — see [`CloudMedium`].
pub fn extinction_per_m(water: f64) -> f64 {
    CLOUD_EXTINCTION_PER_M as f64 * water.max(0.0)
}

/// **The optical depth of a column's water**: `σ · LWP`, dimensionless.
///
/// `LWP` is the *liquid water path* — the integral of [`density_of`] over the layer, which
/// `voxel_shapers::clouds::column_water_path_m` evaluates in closed form — so the depth is a
/// plain product and there is no second profile to keep in step.
pub fn optical_depth(water_path_m: f64) -> f64 {
    if !water_path_m.is_finite() || water_path_m < 0.0 {
        return f64::INFINITY;
    }
    CLOUD_EXTINCTION_PER_M as f64 * water_path_m
}

/// **The Sun's own transmittance through `water_path_m` metres of cloud water** — the
/// spec's Beer-Lambert, `exp(−σ·LWP)`, and a fraction in `[0, 1]` like everything
/// `voxel_stream::light` returns.
///
/// **This is where the cloud's "mostly transparent" is decided, and it is a number rather
/// than a mood.** The user's ruling (2026-10-09) is that a cloud reads as a veil and not a
/// wall. Over this world's own columns the law gives `exp(−0.335) = 0.72` at the measured p90
/// water path (33.5 m, `examples/aerosol-column.rs`) and `exp(−1.25) = 0.29` at the deepest
/// (125 m) — most of the Sun crosses an ordinary cloud, and even the deepest passes a
/// quarter. `the_worlds_own_clouds_are_mostly_transparent` measures that on the shipped model
/// rather than trusting this arithmetic, and `the_medium_and_the_closed_form_are_one_law`
/// holds the walk to it.
pub fn cloud_transmittance(water_path_m: f64) -> f64 {
    let depth = optical_depth(water_path_m);
    if !depth.is_finite() {
        return 0.0;
    }
    (-depth).exp()
}

/// **The cloud's own OpenPBR row**, as the appearance layer states it (C-179).
///
/// A cloud has no row in the material registry and cannot have one, which is `OpticalClass::
/// Scattering`'s own claim: a medium has no surface, so there is no `Material` for `openpbr::
/// derive` to read. The row is therefore stated here, once, and it is the reference the shader's
/// own `cloud_row` is held to — the same arrangement `shade_cpu` has with `shading.wgsl`'s
/// fragment terms, and for the same reason: a second copy of a shading input is how two copies
/// drift.
///
/// Every parameter a *surface* would carry is **absent and not zero**: no roughness, no specular
/// boundary, no coat, no emission, no transmission. That is the medium's own statement — a
/// whole-cube medium has no surface to reflect off — and it is why the rim and the specular warp
/// are drawn nowhere on a cloud while the diffuse terms, lit by the same rig as everything else,
/// are drawn everywhere on it.
pub fn cloud_inputs() -> voxel_appearance::openpbr::OpenPbrInputs {
    use voxel_appearance::openpbr::{Colour, Emission, LinearRgb, OpenPbrInputs};
    OpenPbrInputs {
        base_color: Colour::Admitted(LinearRgb::new(CLOUD_ALBEDO, CLOUD_ALBEDO, CLOUD_ALBEDO)),
        base_metalness: 0.0,
        specular_roughness: None,
        specular_ior: None,
        specular_weight: None,
        coat_weight: 0.0,
        coat_roughness: None,
        transmission_weight: 0.0,
        transmission_depth: None,
        subsurface_weight: None,
        emission: Emission::None,
    }
}

/// **The cloud as the light sampler's participating medium** — the answer to
/// `voxel_stream::light::Matter::medium` over a [`CloudField`].
///
/// A provider rather than a plain function, so the sampler's own walk is used unchanged:
/// `voxel_stream::light::Sampler` aims the beam, `Crossings` visits the voxels, and this
/// answers *how thick the cloud is* at each address the ray is in. `voxel_model::
/// OpticalClass::Scattering` names what this is — a medium, not a material — and the class's
/// own note says why: a cloud has no surface, so a beam that meets one is neither stopped at
/// an interface nor bent by one. `subvoxel` therefore answers `None` at every address, and
/// the **only** thing this provider can do to a beam is attenuate it along the path.
pub struct CloudMedium<'a> {
    field: &'a CloudField,
}

impl<'a> CloudMedium<'a> {
    pub fn new(field: &'a CloudField) -> Self {
        CloudMedium { field }
    }

    /// The field's local water at one world address, or `None` where the field holds no cloud
    /// there.
    ///
    /// The address is taken at its **subvoxel centre** — the point the segment's rate is
    /// evaluated at — and off the field's own extent the answer is `None` rather than a wrap,
    /// for the reason `CloudField::texel` refuses an index outside the field: a cloud the
    /// field does not reach is not a cloud this provider can place.
    pub fn water_at(&self, world: [i32; 3], at: SubvoxelIndex) -> Option<f64> {
        let cell = 1.0 / SUBVOXEL_SIDE as f64;
        let x = world[0] as f64 + (at.x as f64 + 0.5) * cell;
        let y = world[1] as f64 + (at.y as f64 + 0.5) * cell;
        let z = world[2] as f64 + (at.z as f64 + 0.5) * cell;
        let i = (x - self.field.origin_m[0]) / CLOUD_COLUMN_M;
        let j = (z - self.field.origin_m[1]) / CLOUD_COLUMN_M;
        if !(0.0..CLOUD_COLUMNS as f64).contains(&i) || !(0.0..CLOUD_COLUMNS as f64).contains(&j) {
            return None;
        }
        let texel = self.field.texel(i.floor() as u32, j.floor() as u32)?;
        let water = density_of(texel, y);
        if water > 0.0 {
            Some(water)
        } else {
            None
        }
    }
}

impl Matter for CloudMedium<'_> {
    fn subvoxel(&self, _world: [i32; 3], _at: SubvoxelIndex) -> Option<MaterialId> {
        // A medium has no surface at any address. This is the class's whole claim, and it is
        // why the beam is dimmed rather than stopped: see `voxel_stream::light::Matter`.
        None
    }

    fn medium(&self, world: [i32; 3], at: SubvoxelIndex) -> Option<f64> {
        Some(extinction_per_m(self.water_at(world, at)?))
    }
}

/// The kind as the field's fourth channel carries it: an index into
/// [`clouds::CloudKind::ALL`], so the encoding cannot silently reorder.
pub fn kind_code(kind: clouds::CloudKind) -> u32 {
    clouds::CloudKind::ALL
        .iter()
        .position(|k| *k == kind)
        .unwrap_or(0) as u32
}

/// **The beam a cloud scatters**: the sun's direction and colour, in the units the model
/// states them in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CloudSun {
    /// Unit vector **toward** the sun, world frame.
    pub toward: [f64; 3],
    /// The sun's own colour, linear, before any exposure.
    pub colour: [f64; 3],
    /// The sun light's intensity, as the rig carries it.
    pub intensity: f64,
    /// The hemisphere light the cloud also receives, linear: the sky's own diffuse.
    pub ambient: [f64; 3],
}

impl CloudSun {
    /// The sun out of a light rig — **the first light**, which is the rig's own convention for
    /// the beam (`shading.wgsl` reads `rig.lights[0]` for the sky's own beam).
    ///
    /// `None` when the rig carries no light at all: a night with no moon has no beam, and the
    /// clouds are then lit by [`CloudSun::ambient`] alone rather than by a direction nobody
    /// measured.
    pub fn from_rig(rig: &LightRig) -> Option<Self> {
        let light = rig.lights.first()?;
        Some(CloudSun {
            toward: [light.direction.x, light.direction.y, light.direction.z],
            colour: [light.colour.r, light.colour.g, light.colour.b],
            intensity: light.intensity,
            ambient: [rig.ambient.r, rig.ambient.g, rig.ambient.b],
        })
    }

    /// A faint sun overhead, for a caller with no rig: enough to draw a cloud, and named as a
    /// fixture rather than a measurement.
    pub fn fixture() -> Self {
        CloudSun {
            toward: [0.35, 0.86, 0.36],
            colour: [0.99, 0.97, 0.94],
            intensity: 1.0,
            ambient: [0.10, 0.12, 0.16],
        }
    }
}

/// The field's corner for a span centred on `at`, snapped to whole columns.
fn snap(at: f64) -> f64 {
    (at / CLOUD_COLUMN_M).floor() * CLOUD_COLUMN_M
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_shapers::spec::{Config, WorldSpec};

    fn spec() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    fn noon() -> u64 {
        voxel_shapers::climate::TICKS_PER_DAY / 2
    }

    #[test]
    fn the_field_can_be_asked_between_its_own_texels() {
        // **The interpolation the geometry cuts on.** A drawn cloud's edge is where the field's own
        // water crosses a declared cut, so the value the geometry reads has to be the field *between*
        // its texels — otherwise every cell's fate is its texel's, and the picture is 64 m columns.
        // Three properties, and each is used: the lattice itself is exact, an address between two
        // texels answers with a value strictly between them, and **coverage is still a step** — a
        // clear address stays clear, so bending the density can never invent a cloud.
        let n = CLOUD_COLUMNS as usize;
        let mut columns = vec![[0.0f32; 4]; n * n];
        columns[10 * n + 10] = [0.2, 1_000.0, 1_400.0, 1.0];
        columns[10 * n + 11] = [0.6, 1_000.0, 1_400.0, 1.0];
        let field = CloudField {
            origin_m: [0.0, 0.0],
            columns,
            slab_base_m: 1_000.0,
            slab_top_m: 1_400.0,
            cloudy: 2,
            filled_micros: 0,
        };
        let centre = |i: u32| field.column_centre_m(i, 10);
        let z = centre(10)[1];
        // On the lattice it is the texel, exactly.
        assert_eq!(field.at_m(centre(10)[0], z), [0.2, 1_000.0, 1_400.0, 1.0]);
        // Halfway between two wet texels it is halfway between their water — the profile the cut
        // reads therefore runs continuously from a thin texel to a thick one.
        let mid = (centre(10)[0] + centre(11)[0]) / 2.0;
        let between = field.at_m(mid, z);
        assert!(
            (between[0] - 0.4).abs() < 1e-6,
            "the midpoint reads {} where the two texels hold 0.2 and 0.6",
            between[0]
        );
        assert!(between[0] > 0.2 && between[0] < 0.6);
        assert_eq!((between[1], between[2]), (1_000.0, 1_400.0));
        // **But existence is not bent.** Past the midpoint toward the clear texel the answer is
        // clear, even though a cloud is one texel away: the model's own existence test is a step and
        // an interpolated one would put a haze in every address that merely neighbours a cloud.
        assert_eq!(
            field.at_m(centre(11)[0] + CLOUD_COLUMN_M / 2.0 + 1.0, z)[0],
            0.0
        );
        // And off the field there is nothing measured, so there is no cloud.
        assert_eq!(field.at_m(-10_000.0, z), [0.0; 4]);
        assert_eq!(field.at_m(f64::NAN, z), [0.0; 4]);
    }

    #[test]
    fn the_model_is_asked_on_its_own_lattice_and_the_between_is_interpolated() {
        // **Two lattices, and both are checked against the model rather than against the
        // field's own arithmetic.**
        //
        // A texel the model was asked about **is** the model's column: the same three
        // numbers, the same class, and the profile `cloud_at`'s, float for float. A texel
        // between two samples is an interpolation of them, and what holds there is what
        // `blend` claims: coverage blends, the layer is one of the sampled layers, and no
        // class is ever a blend of two classes.
        let spec = spec();
        let field = CloudField::sample(&spec, noon(), [0.0, 0.0]);
        assert!(field.cloudy > 0, "the field is clear; nothing is being tested");
        let n = CLOUD_COLUMNS as usize;
        let m = CLOUD_SAMPLES_PER_AXIS as usize;

        // The model at every sample of the coarse lattice, in the field's own indexing.
        let mut samples = vec![[0.0f32; 4]; m * m];
        for jk in 0..m {
            for ik in 0..m {
                let i = sample_index(ik, n);
                let j = sample_index(jk, n);
                let [x, z] = field.column_centre_m(i as u32, j as u32);
                let Some(column) = clouds::cloud_column(&spec, x, z, noon()) else {
                    continue;
                };
                samples[jk * m + ik] = [
                    column.water as f32,
                    column.base_m as f32,
                    column.top_m as f32,
                    kind_code(column.kind) as f32,
                ];
            }
        }
        assert!(
            samples.iter().any(|sample| sample[0] > 0.0),
            "the sample lattice is clear; nothing is being tested"
        );

        let mut on_lattice = 0usize;
        let mut between = 0usize;
        for j in 0..n {
            for i in 0..n {
                let texel = field.texel(i as u32, j as u32).expect("inside the field");
                let (i0, i1, t_i) = corner(i, n, m);
                let (j0, j1, t_j) = corner(j, n, m);
                let corners = [
                    samples[j0 * m + i0],
                    samples[j0 * m + i1],
                    samples[j1 * m + i0],
                    samples[j1 * m + i1],
                ];
                // A texel that sits exactly on one sample has that sample's weight alone,
                // and those are the m × m texels the model was actually asked about.
                let weights = [
                    (1.0 - t_i) * (1.0 - t_j),
                    t_i * (1.0 - t_j),
                    (1.0 - t_i) * t_j,
                    t_i * t_j,
                ];
                if let Some(exact) = weights.iter().position(|weight| *weight == 1.0) {
                    on_lattice += 1;
                    let sample = corners[exact];
                    if sample[0] <= 0.0 {
                        assert_eq!(texel, [0.0; 4], "a clear sample produced a cloud");
                        continue;
                    }
                    assert_eq!(texel, sample, "a sampled texel drifted from the model");
                    let [x, z] = field.column_centre_m(i as u32, j as u32);
                    let model = clouds::cloud_column(&spec, x, z, noon()).expect("a cloudy column");
                    assert_eq!(texel[0], model.water as f32);
                    assert_eq!(texel[1], model.base_m as f32);
                    assert_eq!(texel[2], model.top_m as f32);
                    assert_eq!(texel[3] as usize, kind_code(model.kind) as usize);
                    for fraction in [0.2, 0.5, 0.8] {
                        let y = model.base_m + (model.top_m - model.base_m) * fraction;
                        let mine = density_of(texel, y);
                        let theirs = clouds::cloud_at(&spec, x, z, y, noon())
                            .expect("inside the layer")
                            .water;
                        assert!(
                            (mine - theirs).abs() < 1e-6,
                            "profile drift at ({x}, {z}) y={y}: {mine} vs {theirs}"
                        );
                    }
                    continue;
                }
                if texel[0] <= 0.0 {
                    continue;
                }
                between += 1;
                // **Coverage is the nearest sample's**: a texel whose nearest sample is clear
                // is clear, so the model's edge moves by at most half a stride and no haze
                // appears beside a cloud.
                let nearest = weights
                    .iter()
                    .enumerate()
                    .fold((0usize, -1.0f64), |best, (index, weight)| {
                        if *weight > best.1 {
                            (index, *weight)
                        } else {
                            best
                        }
                    })
                    .0;
                assert!(
                    corners[nearest][0] > 0.0,
                    "a cloudy texel whose nearest sample is clear"
                );
                // **The density is the wet corners'**: inside their own range.
                let water_lo = corners.iter().map(|c| c[0]).fold(f32::INFINITY, f32::min);
                let water_hi = corners
                    .iter()
                    .map(|c| c[0])
                    .fold(f32::NEG_INFINITY, f32::max);
                assert!(
                    texel[0] >= water_lo && texel[0] <= water_hi,
                    "the interpolated density left its corners' range"
                );
                // **A class is never a blend of two classes**: it is a corner's own code.
                assert!(
                    corners.iter().any(|c| c[0] > 0.0 && c[3] == texel[3]),
                    "the interpolated class {} is one no sample named",
                    texel[3]
                );
                // **The layer never blends with air**: the base and the top stay inside the
                // range of the *wet* corners, so a clear corner's zeros cannot drag a
                // cloud's base to an altitude nobody measured.
                let wet: Vec<[f32; 4]> = corners.iter().copied().filter(|c| c[0] > 0.0).collect();
                assert!(!wet.is_empty(), "a cloudy texel whose corners are all clear");
                let base_lo = wet.iter().map(|c| c[1]).fold(f32::INFINITY, f32::min);
                let base_hi = wet.iter().map(|c| c[1]).fold(f32::NEG_INFINITY, f32::max);
                assert!(
                    texel[1] >= base_lo && texel[1] <= base_hi,
                    "the interpolated base left its wet corners' range"
                );
                let top_lo = wet.iter().map(|c| c[2]).fold(f32::INFINITY, f32::min);
                let top_hi = wet.iter().map(|c| c[2]).fold(f32::NEG_INFINITY, f32::max);
                assert!(
                    texel[2] >= top_lo && texel[2] <= top_hi,
                    "the interpolated top left its wet corners' range"
                );
                // And covered by exactly one sample, the texel takes that sample's own layer
                // rather than an average of it and the empty air beside it.
                if wet.len() == 1 {
                    assert_eq!(
                        [texel[1], texel[2], texel[3]],
                        [wet[0][1], wet[0][2], wet[0][3]],
                        "a singly-covered texel averaged its layer with the air"
                    );
                }
            }
        }
        assert_eq!(on_lattice, m * m, "the sample lattice is not m x m");
        assert!(between > 0, "nothing was interpolated; the stride is not a stride");
    }

    #[test]
    fn the_profile_is_zero_outside_the_layer() {
        let texel = [0.5f32, 1_000.0, 1_500.0, 1.0];
        assert_eq!(density_of(texel, 999.9), 0.0);
        assert_eq!(density_of(texel, 1_500.0), 0.0);
        assert!(density_of(texel, 1_250.0) > 0.4, "the middle of the layer is its deepest");
    }

    #[test]
    fn the_kind_encoding_is_injective_over_every_kind() {
        let codes: Vec<u32> = clouds::CloudKind::ALL.iter().copied().map(kind_code).collect();
        assert_eq!(codes, vec![0, 1, 2]);
    }

    #[test]
    fn the_worlds_own_clouds_are_mostly_transparent() {
        // **The user's ruling, measured on the shipped model** (2026-10-09): a cloud reads
        // as a veil and not a wall. The claim is about a picture, so it is held to the
        // world's own clouds rather than to a rate typed here — the model's column water
        // paths are read over a grid of columns and the transmittance they imply is what a
        // beam sees. A veil is dimmer than clear air and brighter than a wall; a cloud that
        // dimmed the Sun to nothing would be a wall, and one that dimmed nothing at all
        // would mean the extinction was not wired.
        let spec = spec();
        let tick = noon();
        let mut dimmed: Vec<f64> = Vec::new();
        for x in (-1_024..1_024).step_by(61) {
            for z in (-1_024..1_024).step_by(67) {
                let path = clouds::column_water_path_m(&spec, x as f64, z as f64, tick);
                if path > 0.0 {
                    dimmed.push(cloud_transmittance(path));
                }
            }
        }
        assert!(
            dimmed.len() >= 16,
            "the sampled region is nearly clear ({} cloudy columns), so the ruling is not \
             measured here",
            dimmed.len()
        );
        dimmed.sort_by(f64::total_cmp);
        let median = dimmed[dimmed.len() / 2];
        let deepest = dimmed[0];
        println!(
            "cloud transmittance over {} cloudy columns: median {median:.3}, deepest {deepest:.3}",
            dimmed.len()
        );
        assert!(
            median > 0.5,
            "the median cloud passed only {median:.3} of the Sun — that is a wall, not a veil"
        );
        assert!(
            deepest > 0.1,
            "the deepest cloud passed only {deepest:.3} — a cloud this world draws is opaque"
        );
        assert!(
            deepest < 1.0,
            "no cloud dimmed the Sun at all, so the extinction is not wired"
        );
        // **The rate is declared and pinned.** A later edit that thins or thickens the world's
        // clouds has to change this line and say so: it is the number the ruling was given in.
        assert_eq!(CLOUD_EXTINCTION_PER_M, 0.01, "the cloud's extinction moved");
    }

    #[test]
    fn the_medium_and_the_closed_form_are_one_law() {
        // **The walk and the closed form, held together.** `column_water_path_m` evaluates the
        // column's own integral of the profile; the sampler integrates it again, step by step,
        // through [`CloudMedium`]. Two derivations of one quantity is the drift this repository
        // refuses, so they are compared rather than *both* trusted: a vertical beam through the
        // cloudiest column the field has must leave with the transmittance that column's own
        // water path implies.
        //
        // The one declared difference is the step: the walk charges each metre of the layer
        // with the rate at the subvoxel it entered, so it is a left Riemann sum of the profile
        // at `1 m` against the integral. Over this world's layers that is under a part in a
        // million — a lot less than the tolerance below, which is sized for the field's own
        // interpolation rather than for the sum.
        let spec = spec();
        let field = CloudField::sample(&spec, noon(), [0.0, 0.0]);
        let mut best: Option<((u32, u32), [f32; 4], f64)> = None;
        for j in 0..CLOUD_COLUMNS {
            for i in 0..CLOUD_COLUMNS {
                let Some(texel) = field.texel(i, j) else { continue };
                if texel[0] <= 0.0 {
                    continue;
                }
                let path =
                    texel[0] as f64 * 2.0 * (texel[2] - texel[1]) as f64 / std::f64::consts::PI;
                if best.map_or(true, |(_, _, deepest)| path > deepest) {
                    best = Some(((i, j), texel, path));
                }
            }
        }
        let ((i, j), texel, lwp) = best.expect("the field has no cloud in it");
        let [x, z] = field.column_centre_m(i, j);
        let base_m = texel[1] as f64;
        let top_m = texel[2] as f64;
        let from_m = [x, base_m - 8.0, z];
        let reach_m = (top_m - from_m[1]) + 8.0;
        let sampler =
            voxel_stream::light::Sampler::new([0.0, 1.0, 0.0], reach_m).expect("a vertical beam");
        let open = voxel_stream::light::transmittance(
            &CloudMedium::new(&field),
            from_m,
            [0.0, 1.0, 0.0],
            4.0,
        );
        let walk = sampler.beam(&CloudMedium::new(&field), from_m);
        let closed = cloud_transmittance(lwp);
        assert_eq!(open, 1.0, "a beam that stops short of the layer was dimmed by it");
        assert!(
            closed > 0.0 && closed < 1.0,
            "the chosen column is not a dimmer: {closed}"
        );
        assert!(
            (walk - closed).abs() / closed < 1e-3,
            "the walk gave {walk} where the column's own water path ({lwp:.1} m) gives {closed}"
        );
    }

    #[test]
    fn above_the_cloud_the_beam_is_clear_and_below_it_is_dimmed() {
        // **The vertical extent, which is the field's own profile.** A beam that starts above
        // the layer crosses no water and leaves whole; one that starts under it crosses the
        // whole column. That is `density_of`'s `y_m < base_m || y_m >= top_m` clause seen from
        // the sampler's side, and it is the difference between a cloud that is a layer and one
        // that is a whole column of sky.
        let spec = spec();
        let field = CloudField::sample(&spec, noon(), [0.0, 0.0]);
        let mut found = None;
        'scan: for j in 0..CLOUD_COLUMNS {
            for i in 0..CLOUD_COLUMNS {
                if let Some(texel) = field.texel(i, j) {
                    if texel[0] > 0.0 && texel[2] - texel[1] > 40.0 {
                        found = Some((i, j, texel));
                        break 'scan;
                    }
                }
            }
        }
        let (i, j, texel) = found.expect("a cloud with a layer deeper than 40 m");
        let [x, z] = field.column_centre_m(i, j);
        let medium = CloudMedium::new(&field);
        let base_m = texel[1] as f64;
        let top_m = texel[2] as f64;
        // Above the top, looking up crosses nothing, which is what an eye over the cloud sees
        // of the Sun.
        let above = voxel_stream::light::transmittance(
            &medium,
            [x, top_m + 4.0, z],
            [0.0, 1.0, 0.0],
            64.0,
        );
        let under = voxel_stream::light::transmittance(
            &medium,
            [x, base_m - 4.0, z],
            [0.0, 1.0, 0.0],
            (top_m - base_m) + 16.0,
        );
        assert_eq!(
            above, 1.0,
            "a beam above the cloud was dimmed by the layer below it"
        );
        assert!(
            under > 0.0 && under < 1.0,
            "a beam under the cloud was not dimmed: {under}"
        );
    }

    #[test]
    fn the_field_is_snapped_to_its_own_columns() {
        // Two eyes inside one column get **the same field**, byte for byte, so following the
        // eye does not slide the texels under the world.
        let spec = spec();
        let a = CloudField::sample(&spec, noon(), [1_000.0, -2_000.0]);
        let b = CloudField::sample(&spec, noon(), [1_000.0 + CLOUD_COLUMN_M / 4.0, -2_000.0]);
        assert_eq!(a.origin_m, b.origin_m);
        assert_eq!(a.columns, b.columns);
        assert_eq!(a, b);

        // And a column further on does move it.
        let c = CloudField::sample(&spec, noon(), [1_000.0 + CLOUD_COLUMN_M, -2_000.0]);
        assert_ne!(a.origin_m, c.origin_m);
    }

    /// **What one field fill costs, and what it produces** — reported, not asserted.
    ///
    /// [`CloudField::filled_micros`] and [`CloudField::cloudy`] are carried on every fill and
    /// were read by nothing. That is the shape of a measurement that is not one: the fill runs
    /// on the **frame thread**, on the atmosphere's cadence, so its cost is a frame's cost —
    /// and nobody could see it. This prints it, in the world's own units as well as in metres,
    /// so the constants above can be argued with instead of assumed.
    ///
    /// The times are printed and not asserted, for the reason the renderer's own cost test
    /// states: nobody has ruled what this is allowed to cost on hardware that is not this
    /// machine, so a threshold here would be an invented number wearing an assertion's clothes.
    #[test]
    fn the_fill_reports_what_it_costs_the_cadence() {
        let spec = spec();
        let mut best = f64::INFINITY;
        let mut field = CloudField::sample(&spec, noon(), [0.0, 0.0]);
        for _ in 0..3 {
            let started = std::time::Instant::now();
            field = CloudField::sample(&spec, noon(), [0.0, 0.0]);
            std::hint::black_box(&field);
            best = best.min(started.elapsed().as_secs_f64() * 1000.0);
        }
        let chunk_m = voxel_model::CHUNK_SIDE as f64;
        let columns = CLOUD_COLUMNS * CLOUD_COLUMNS;
        let asks = CLOUD_SAMPLES_PER_AXIS * CLOUD_SAMPLES_PER_AXIS;
        println!(
            "cloud field: {CLOUD_COLUMNS} x {CLOUD_COLUMNS} columns ({columns} addressed) at \
             {CLOUD_COLUMN_M:.1} m each, spanning {CLOUD_SPAN_M:.0} m = {:.0} chunks across\n\
\x20  the sample rate: {asks} asks at {CLOUD_SAMPLE_M:.0} m ({CLOUD_SAMPLES_PER_AXIS} per \
             axis, stride {CLOUD_SAMPLE_STRIDE}) against {columns} texels = {:.2}x fewer asks\n\
\x20  the model's own instrument: {:.2} ms to fill (recorded {:.2} ms), {} of {columns} \
             columns cloudy ({:.0} %)\n\
\x20  slab {:.1} m .. {:.1} m above the datum = {:.1} .. {:.1} chunks; the terrain band is \
             {} .. {} m\n\
\x20  march: a step of {CLOUD_STEP_M:.0} m and a ceiling of {:.0} steps = {:.0} m of ray, \
             against a field {CLOUD_SPAN_M:.0} m across",
            CLOUD_SPAN_M / chunk_m,
            columns as f64 / asks as f64,
            best,
            field.filled_micros as f64 / 1000.0,
            field.cloudy,
            100.0 * field.cloudy as f64 / columns as f64,
            field.slab_base_m,
            field.slab_top_m,
            field.slab_base_m / chunk_m,
            field.slab_top_m / chunk_m,
            -16,
            48,
            CLOUD_MAX_DISTANCE_M / CLOUD_STEP_M,
            CLOUD_SPAN_M,
        );
        assert!(field.cloudy > 0, "a clear field is not a field this test can report");
    }

    /// **What the stride costs the picture** — reported, not asserted, for the reason every
    /// threshold in this repository is reported rather than invented: nobody has ruled what
    /// a cloud is allowed to be wrong by.
    ///
    /// The field is built twice: once the way [`CloudField::sample`] now does it (the model
    /// asked on the sample lattice, the texels between interpolated) and once at one ask
    /// per texel, which is what the field did before the stride existed. The two are then
    /// compared per texel. The numbers that decide the stride are the worst density it
    /// moved, the worst layer boundary it moved, and how many texels it calls cloudy that
    /// the model did not — the last of which is the interpolation *widening* a cloud, which
    /// is the price a lower rate is paid in.
    #[test]
    fn the_stride_is_reported_against_the_full_rate_model() {
        let spec = spec();
        let sampled = CloudField::sample(&spec, noon(), [0.0, 0.0]);
        let n = CLOUD_COLUMNS as usize;
        // The rate this replaced, timed in the same test so the two are one measurement
        // rather than a claim and an assumption.
        let full_rate_started = std::time::Instant::now();
        let mut full_rate = sampled.clone();
        for j in 0..n {
            for i in 0..n {
                let [x, z] = sampled.column_centre_m(i as u32, j as u32);
                full_rate.columns[j * n + i] = match clouds::cloud_column(&spec, x, z, noon()) {
                    Some(column) => [
                        column.water as f32,
                        column.base_m as f32,
                        column.top_m as f32,
                        kind_code(column.kind) as f32,
                    ],
                    None => [0.0; 4],
                };
            }
        }
        let mut worst_water = 0.0f64;
        let mut worst_layer_m = 0.0f64;
        let mut widened = 0usize;
        let mut thinned = 0usize;
        let mut full_rate_cloudy = 0usize;
        for j in 0..n {
            for i in 0..n {
                let mine = sampled.columns[j * n + i];
                let theirs = full_rate.columns[j * n + i];
                if theirs[0] > 0.0 {
                    full_rate_cloudy += 1;
                    worst_water = worst_water.max((mine[0] - theirs[0]).abs() as f64);
                    if mine[0] > 0.0 {
                        worst_layer_m = worst_layer_m
                            .max((mine[1] - theirs[1]).abs() as f64)
                            .max((mine[2] - theirs[2]).abs() as f64);
                    }
                }
                match (mine[0] > 0.0, theirs[0] > 0.0) {
                    (true, false) => widened += 1,
                    (false, true) => thinned += 1,
                    _ => {}
                }
            }
        }
        let full_rate_ms = full_rate_started.elapsed().as_secs_f64() * 1000.0;
        println!(
            "cloud stride {CLOUD_SAMPLE_STRIDE}: worst |{{water}}| = {:.5} of the field's own \
             unit, worst layer boundary moved = {:.1} m\n\
\x20  cost: {:.2} ms to ask every texel against {:.2} ms on the sample lattice\n\
\x20  coverage: the model draws {full_rate_cloudy} cloudy texels of {columns}; the field at \
             this stride calls {widened} cloudy that the model did not and {thinned} clear that \
             the model clouded\n\
\x20  the lattice is exact: every texel the model was asked about equals the model's own \
             column (the interpolation test is the witness)",
            worst_water,
            worst_layer_m,
            full_rate_ms,
            sampled.filled_micros as f64 / 1000.0,
            columns = n * n,
        );
        assert!(
            full_rate_cloudy > 0,
            "a full-rate field with no cloud is not a field this test can report"
        );
        assert!(
            widened + thinned < n * n,
            "the comparison disagrees with the model everywhere; it is measuring nothing"
        );
    }

    #[test]
    fn equality_ignores_the_measurement() {
        // The other direction of the same contract: two fields that differ **only** in how long
        // the fill took are the same field. Without this, a derived `==` would put the clock
        // back into the equality and the determinism tests above would go red again.
        let spec = spec();
        let a = CloudField::sample(&spec, noon(), [0.0, 0.0]);
        let mut b = a.clone();
        b.filled_micros = a.filled_micros.wrapping_add(1);
        assert_eq!(a, b, "the measurement is not part of the field's identity");

        // And a field that differs in what it produced is not equal, so this is not a
        // comparison that says yes to everything.
        let mut c = a.clone();
        c.slab_top_m += 1.0;
        assert_ne!(a, c);
    }

    #[test]
    fn the_cloud_lattice_is_stated_in_chunks_and_is_not_the_chunk_lattice() {
        // **The correction, as a checked fact.** One voxel is one metre, so a chunk is 32 m; the
        // cloud's column is 2 chunks and its block is 64. The header used to claim a cloud block
        // and a world chunk "are addressed the same way", which shared a *number* with
        // `CHUNK_SIDE` and no extent at all — so every ratio is pinned here rather than restated.
        let chunk_m = voxel_model::CHUNK_SIDE as f64;
        assert_eq!(chunk_m, 32.0, "one voxel is one metre, so a chunk is 32 m");
        assert_eq!(CLOUD_COLUMN_CHUNKS, 2, "a cloud column straddles exactly two chunks");
        assert_eq!(CLOUD_BLOCK_CHUNKS, 64, "a cloud block is 64 chunks across, not one");
        assert_eq!(CLOUD_SPAN_M / chunk_m, 256.0, "the field is 256 chunks across");

        // And the exactness the two integer constants depend on: those divisions must not be
        // truncating a fraction, or a "column" would not be a whole number of chunks and the
        // lattice would not be a refinement of the chunk grid at all.
        assert_eq!(CLOUD_COLUMN_M % chunk_m, 0.0);
        assert_eq!(CLOUD_BLOCK_M % chunk_m, 0.0);

        // **And the sample lattice is declared too, in the same units**, because the rate is
        // the thing the stride spends: the model is asked every 128 m — 4 chunks, a whole
        // number of chunks, so a chunk owner can name the samples it needs — and the texels
        // between two samples are interpolated rather than asked.
        assert_eq!(CLOUD_SAMPLE_M, 128.0, "the sample rate moved");
        assert_eq!(CLOUD_SAMPLE_M / chunk_m, 4.0, "the sample lattice is a chunk lattice");
        assert_eq!(CLOUD_SAMPLE_M % chunk_m, 0.0);
        assert_eq!(CLOUD_SAMPLES_PER_AXIS, 65, "65 samples a side, the last pinned to the edge");
        assert_eq!(
            CLOUD_SAMPLES_PER_AXIS * CLOUD_SAMPLES_PER_AXIS,
            4_225,
            "the ask count the fill pays"
        );
        // **3.9× fewer asks, not 4×, and the difference is the pinned last sample**: 65² is
        // 4,225 against the field's 128² = 16,384 texels (16,384 / 4,225 = 3.88). The
        // assertion is the direction, because the exact ratio moves with any change to
        // either constant and a hard-coded ratio would be a second place to edit.
        assert!(
            CLOUD_SAMPLES_PER_AXIS * CLOUD_SAMPLES_PER_AXIS < CLOUD_COLUMNS * CLOUD_COLUMNS,
            "a sample rate that is not lower than one ask per texel is not a lower rate"
        );

        // **The number the horizon ladder has to be compared with, and the disagreement.** The
        // widest horizon `voxel_stream::horizons` declares is `Horizon::Field`, radius 96 chunks;
        // this field reaches 128. So the field is wider than the widest horizon there is, and no
        // horizon can bound which cells a refresh should touch — which is the structural reason
        // the fill is one full re-sample rather than a streamed set of cells.
        assert_eq!(CLOUD_HALF_EXTENT_CHUNKS, 128);
        assert!(
            CLOUD_HALF_EXTENT_CHUNKS > 96,
            "if this stops holding, the field has been brought inside the field horizon and the \
             comment above is stale rather than wrong"
        );
    }

    #[test]
    fn the_field_is_deterministic_and_the_span_is_four_blocks() {
        let spec = spec();
        let a = CloudField::sample(&spec, noon(), [0.0, 0.0]);
        let b = CloudField::sample(&spec, noon(), [0.0, 0.0]);
        assert_eq!(a, b);
        assert_eq!(a.columns.len(), (CLOUD_COLUMNS * CLOUD_COLUMNS) as usize);
        assert_eq!(CLOUD_SPAN_M / CLOUD_BLOCK_M, CLOUD_BLOCKS_PER_AXIS as f64);
        assert!(a.slab_top_m >= a.slab_base_m, "the slab is ordered");
        assert!(a.slab_top_m <= clouds::TROPOPAUSE_M + 1e-9);
    }
}
