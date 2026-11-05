//! **Every term that reaches a pixel, in the units it is computed in** — the light budget of one
//! frame at a real tick, over the shipped world and through the real worker.
//!
//! ```text
//! cargo run --offline --release -p voxel-client --example light-terms
//! ```
//!
//! The frame has two branches (`shading.wgsl`, `which light`): a corner the field measured reads
//! the field's six bands, and a corner it did not reads the rig. The question this example answers
//! is not *which* branch a corner takes but **what each one adds up to**, because a picture that is
//! too bright is a sum and not a branch. So it does what the fragment does, in the same order and
//! the same units, and prints the terms rather than the total alone.
//!
//! # What is measured, and what is transcribed
//!
//! * **The field is the real one**: [`CellField`] over the shipped world's own streamed window, fed
//!   by [`Measurer`]'s worker thread — the same handshake the frame runs, with the trees travelling
//!   by handle.
//! * **The rig is the real one**: `daylight_of` at the same tick, exposed by `exposed_at` with the
//!   frame's own grade — the same expression `shploingus` hands the uniform.
//! * **The arithmetic is a transcription** of `fs_main` between `// 4. Ambient` and the emission
//!   block, term by term, with each line naming the shader line it copies. A transcription and not
//!   the shader, because what is being audited is a *sum*, and the shader cannot print its terms;
//!   everything it is summed from is the real measurement.
//!
//! # The three claims the numbers are read against
//!
//! 1. **A measured corner's beam is the rig's own beam.** For a cell with nothing in front of it the
//!    two branches must deliver the same light, because `WorldLight` divides by the same
//!    `peak_sun_luminance` the rig does. They are *not* the same triple — the field carries bands
//!    and the rig carries a chromaticity times an intensity — so the comparison is per channel and
//!    the ratio is printed.
//! 2. **The sky on a measured corner is one sky, not two.** The fragment adds the rig's ambient
//!    before the branch and the field's own sky inside it; this prints the second term as a share
//!    of the first on every measured corner.
//! 3. **A shaded corner is darker than an open one and never brighter.** The rig has no idea where
//!    the cell is, so a measured corner in shade is the only place the world can be darker than the
//!    rig would make it — and a corner that is *brighter* than the rig is a term counted twice.
//!
//! A failure exits non-zero and says which claim and by how much.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use voxel_appearance::npr::LightRig;
use voxel_appearance::openpbr::LinearRgb;
use voxel_appearance::sky::SkyInputs;
use voxel_client::field::REACH_M;
use voxel_client::skylight::{daylight_of, exposed_at, exposed_bands};
use voxel_client::{Ask, CellField, Measurer};
use voxel_model::{SubvoxelIndex, VoxelPos, CHUNK_SIDE};
use voxel_shapers::{Config, WorldSpec};
use voxel_stream::light::{ChunkMatter, Matter};
use voxel_stream::WorldSource;
use voxel_work::{ChunkWindow, Streamer};

/// The window this example streams, one chunk each way — the smallest that lets a ray leave the
/// cell's own chunk, which is the half of the march a single tree cannot show.
const WINDOW: ChunkWindow = ChunkWindow {
    radius_chunks: 1,
    vertical_chunks: 1,
};

/// The streamer's ceiling for meshes and trees together (a window of 27 chunks).
const CEILING_BYTES: usize = 256 * 1024 * 1024;

/// **The surface this example shades.** A mid-grey, matte, opaque OpenPBR surface with no coat, no
/// boundary weight, no transmission and no emission — every term that is not the light itself is
/// switched off, so the printed numbers are the *light budget* and not the material's opinion of
/// it. `0.5` is declared here rather than sampled because the material is not what is under audit.
const BASE: f64 = 0.5;

/// The luminance weights `display.wgsl` tone-maps with (`LUMINANCE_WEIGHTS`).
const LUMINANCE: [f64; 3] = [0.2126, 0.7152, 0.0722];

/// One fragment's radiance, split into the terms `fs_main` adds it from.
///
/// Fields are in **the shader's own units**: the uploaded rig, i.e. physical × `EXPOSURE × grade`,
/// and the field's bands after the same scale (`exposed_bands`). They are compared only with each
/// other and with the tone map, which is the only consumer.
#[derive(Clone, Copy, Default)]
struct Terms {
    /// `// 4. Ambient`: `base * rig.ambient.rgb * gain`.
    ambient: [f64; 3],
    /// The diffuse beam: `base * (beams * (wrap * opaque * substrate))`.
    beam: [f64; 3],
    /// The dome: `base * (sky * opaque)` in the field branch, zero in the rig's.
    sky: [f64; 3],
    /// `// 2. Rim`: `rim_reflectance * (beams * (beam_share * silhouette))`.
    rim: [f64; 3],
    /// `// 2b. The coat` and `// 3. The specular warp` — zero on this material, printed so the
    /// budget is complete rather than merely large.
    coat_and_spec: [f64; 3],
    /// `// Emission is not a Source 2 term` — zero on this material.
    emission: [f64; 3],
}

impl Terms {
    fn total(&self) -> [f64; 3] {
        let mut out = [0.0; 3];
        for term in [
            self.ambient,
            self.beam,
            self.sky,
            self.rim,
            self.coat_and_spec,
            self.emission,
        ] {
            for i in 0..3 {
                out[i] += term[i];
            }
        }
        out
    }

    fn luminance(&self) -> f64 {
        luminance_of(self.total())
    }
}

fn luminance_of(rgb: [f64; 3]) -> f64 {
    LUMINANCE[0] * rgb[0] + LUMINANCE[1] * rgb[1] + LUMINANCE[2] * rgb[2]
}

/// The display transform, transcribed from `display.wgsl`'s `tone_map` — the same two lines, so the
/// "range a surface should occupy" is measured through the function that decides it.
fn tone_map(rgb: [f64; 3]) -> [f64; 3] {
    let l = luminance_of(rgb);
    if !(l > 0.0) {
        return [0.0; 3];
    }
    let brightest = rgb[0].max(rgb[1]).max(rgb[2]);
    let scale = (1.0 / (1.0 + l)).min(1.0 / brightest);
    [rgb[0] * scale, rgb[1] * scale, rgb[2] * scale]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalise(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt();
    if l > 0.0 && l.is_finite() {
        [a[0] / l, a[1] / l, a[2] / l]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// The rig branch: `fs_main`'s `else`, one light at a time.
///
/// This is the branch a corner takes when the field has no measurement for the cell it stands in —
/// **and it is where a refusal is visible**, because the rig's beam does not know the cell is under
/// a tree.
fn rig_terms(rig: &LightRig, normal: [f64; 3], view: [f64; 3]) -> Terms {
    let normal = normalise(normal);
    let view = normalise(view);
    let ndotv = dot(normal, view).clamp(-1.0, 1.0);
    let silhouette = (1.0 - ndotv).powf(5.0);
    let mut terms = Terms::default();
    // `// 4. Ambient, first` — before the branch, so **both branches have it**.
    for i in 0..3 {
        terms.ambient[i] = BASE * channel(rig.ambient, i);
    }
    for light in &rig.lights {
        let toward = normalise([light.direction.x, light.direction.y, light.direction.z]);
        let ndotl = dot(normal, toward);
        let beam_share = (1.0 - ndotl.clamp(0.0, 1.0)).powf(5.0);
        let wrap = ndotl * 0.5 + 0.5;
        for i in 0..3 {
            let colour = channel(light.colour, i);
            terms.beam[i] += BASE * colour * (wrap * light.intensity);
            terms.rim[i] += BASE * colour * (beam_share * silhouette * light.intensity);
        }
    }
    terms
}

/// The field branch: `fs_main`'s `if (in.light_present == 1u)`.
///
/// `bands` is what the vertex carries — `exposed_bands` of the worker's measurement — in the
/// vertex format's order (direct B3/B4/B5, then diffuse), and the band reversal is the shader's.
fn field_terms(bands: [f32; 6], rig: &LightRig, normal: [f64; 3], view: [f64; 3]) -> Terms {
    let normal = normalise(normal);
    let view = normalise(view);
    let ndotv = dot(normal, view).clamp(-1.0, 1.0);
    let silhouette = (1.0 - ndotv).powf(5.0);
    // **A band triple is not yet a colour**: (B3, B4, B5) = blue, green, red.
    let beams = [bands[2] as f64, bands[1] as f64, bands[0] as f64];
    let sky = [bands[5] as f64, bands[4] as f64, bands[3] as f64];
    let mut terms = Terms::default();
    for i in 0..3 {
        terms.ambient[i] = BASE * channel(rig.ambient, i);
    }
    // `var beam_dir = vec3(0.0, 1.0, 0.0)` and the rig's first light when it has one.
    let beam_dir = rig
        .lights
        .first()
        .map(|l| normalise([l.direction.x, l.direction.y, l.direction.z]))
        .unwrap_or([0.0, 1.0, 0.0]);
    let ndotl = dot(normal, beam_dir).clamp(-1.0, 1.0);
    let beam_share = (1.0 - ndotl.clamp(0.0, 1.0)).powf(5.0);
    let wrap = ndotl * 0.5 + 0.5;
    for i in 0..3 {
        terms.beam[i] = BASE * beams[i] * wrap;
        terms.sky[i] = BASE * sky[i];
        terms.rim[i] = BASE * beams[i] * (beam_share * silhouette);
    }
    terms
}

fn channel(colour: LinearRgb, i: usize) -> f64 {
    match i {
        0 => colour.r,
        1 => colour.g,
        _ => colour.b,
    }
}

fn main() {
    let spec = WorldSpec::new(voxel_client::game::GAME_SEED, Config::default());
    let source = WorldSource::new(spec);
    let registry = voxel_shapers::materials::registry().expect("the registry");
    let config = voxel_mesh::MeshConfig::from_registry(&registry).expect("a mesh config");
    let eye = voxel_client::game::spawn_eye(&voxel_client::game::World::shipped());
    let centre = VoxelPos::new(eye[0] as i32, eye[1] as i32, eye[2] as i32).chunk();
    let mut streamer = Streamer::new(source, config, 2, CEILING_BYTES)
        .expect("a streamer")
        .with_window(WINDOW)
        .expect("the window validates");
    streamer.set_centre(centre);
    let settled = streamer.settle().expect("the window settles");
    println!(
        "window: {} of {} chunks resident around {centre:?}",
        settled.resident, settled.wanted
    );
    assert!(
        settled.resident > 0,
        "nothing is resident, so nothing can be measured"
    );

    // ---- the tick, the rig, and the grade the frame would use ------------------
    let inputs = SkyInputs::default();
    let day = voxel_shapers::climate::TICKS_PER_DAY;
    let altitude = |t: u64| voxel_shapers::sky::solar_position(&spec, t).altitude_deg;
    let tick = (0..day)
        .max_by(|a, b| altitude(*a).total_cmp(&altitude(*b)))
        .expect("a tick in the first day");
    let daylight = daylight_of(&spec, tick, inputs, 0).expect("the shipped atmosphere");
    let grade = daylight.grade;
    let rig = exposed_at(&daylight.rig, grade);
    println!(
        "tick {tick}: the sun {:.1} deg up, grade {grade:.4}, {} light(s)",
        daylight.sun_altitude_deg,
        rig.lights.len()
    );
    println!(
        "rig ambient (uploaded) r {:.4} g {:.4} b {:.4}",
        rig.ambient.r, rig.ambient.g, rig.ambient.b
    );
    for light in &rig.lights {
        println!(
            "rig light: direction [{:.4}, {:.4}, {:.4}], colour [{:.4}, {:.4}, {:.4}], intensity {:.4}",
            light.direction.x,
            light.direction.y,
            light.direction.z,
            light.colour.r,
            light.colour.g,
            light.colour.b,
            light.intensity
        );
    }

    // ---- the field, over the real worker --------------------------------------
    // The cells a frame would ask for: the open cell above each column's surface, and one cell
    // enclosed by matter under it. Every fourth column, as `measured-light` does.
    let at = SubvoxelIndex::new(2, 2, 2).expect("the middle of a voxel");
    let origin = centre.origin();
    let top = origin.y + CHUNK_SIDE - 1;
    let matter_at = |x: i32, y: i32, z: i32| -> bool {
        let pos = VoxelPos::new(x, y, z);
        let Some(tree) = streamer.tree(pos.chunk()) else {
            return false;
        };
        let corner = pos.chunk().origin();
        ChunkMatter::new(tree, [corner.x, corner.y, corner.z])
            .subvoxel([x, y, z], at)
            .is_some()
    };
    let mut open_cells = Vec::new();
    let mut enclosed_cells = Vec::new();
    // **Cells with nothing above them**, for the one comparison the surface cannot make: a beam
    // that leaves the window's trees behind is the model's own delivery, and that is the number the
    // rig's beam has to equal. Forty metres up is above every tree in this window and inside the
    // streamed chunks, so the cell's own chunk is resident and the march starts.
    let mut sky_cells = Vec::new();
    for dx in (0..CHUNK_SIDE).step_by(4) {
        for dz in (0..CHUNK_SIDE).step_by(4) {
            let (wx, wz) = (origin.x + dx, origin.z + dz);
            let Some(surface) = (origin.y..=top).rev().find(|y| matter_at(wx, *y, wz)) else {
                continue;
            };
            if surface < top {
                open_cells.push(VoxelPos::new(wx, surface + 1, wz));
            }
            let enclosed = (origin.y + 1..surface - 1).rev().find(|y| {
                (-1..=1).all(|dy| {
                    (-1..=1).all(|dz| (-1..=1).all(|dx| matter_at(wx + dx, *y + dy, wz + dz)))
                })
            });
            if let Some(y) = enclosed {
                enclosed_cells.push(VoxelPos::new(wx, y, wz));
            }
            if dx % 8 == 0 && dz % 8 == 0 {
                sky_cells.push(VoxelPos::new(wx, origin.y + 40, wz));
            }
        }
    }
    println!(
        "cells: {} open above a surface, {} enclosed by matter, {} high above both",
        open_cells.len(),
        enclosed_cells.len(),
        sky_cells.len()
    );

    let mut measurer = Measurer::new();
    let mut field = CellField::of(&spec, tick, inputs, 0, None).expect("a field at this tick");
    // **Two models of one Sun, which is why the beam's colour comes from the rig.** The solver's
    // visible bands and the rig's rgb are both irradiances over the same normaliser, and they are
    // not the same spectrum at the same scale — the number the direct triple is now the rig's for.
    let visible =
        |slot: usize| field.light().beam_bands()[voxel_client::skylight::VISIBLE_BANDS[slot]];
    println!(
        "the solver's visible beam (blue, green, red) {:.1}, {:.1}, {:.1} W/m2 over a peak of \
         {:.1}; the rig's sun ({:.1}, {:.1}, {:.1})",
        visible(0),
        visible(1),
        visible(2),
        voxel_client::skylight::peak_sun_luminance(&spec, inputs),
        daylight.sky.sun.r,
        daylight.sky.sun.g,
        daylight.sky.sun.b
    );
    let cells: Vec<VoxelPos> = open_cells
        .iter()
        .chain(enclosed_cells.iter())
        .chain(sky_cells.iter())
        .copied()
        .collect();
    // The trees the worker may need, by handle: every chunk of the window.
    let mut resident = BTreeSet::new();
    for pos in &cells {
        let c = pos.chunk();
        for dy in -1..=1 {
            for dz in -2..=2 {
                for dx in -2..=2 {
                    let asked = voxel_model::ChunkPos::new(c.x + dx, c.y + dy, c.z + dz);
                    if streamer.tree(asked).is_some() {
                        resident.insert(asked);
                    }
                }
            }
        }
    }
    let residency: Vec<voxel_model::ChunkPos> = resident.into_iter().collect();
    measurer.ask_for(
        Ask {
            spec,
            tick,
            inputs,
            new_moon_tick: 0,
            generation: 0,
            eye,
        },
        cells.iter().map(|cell| (*cell).into()).collect(),
        residency,
    );
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut got = Vec::new();
    while got.len() < cells.len() && Instant::now() < deadline {
        measurer.service(64, |pos| streamer.tree_shared(pos));
        got.extend(measurer.adopt(4_096));
        std::thread::yield_now();
    }
    for m in &got {
        field.adopt(m.cell, m.bands, m.toward);
    }
    println!(
        "the worker answered {} of {} cells; the field holds {} (reach {REACH_M:.0} m)",
        got.len(),
        cells.len(),
        field.held()
    );
    // **An empty answer is a refusal, not a pass**: a window with nothing measured has no budget
    // to audit, and a run that printed zeroes and exited zero would be a report nobody could fail.
    if got.is_empty() {
        eprintln!("REFUSED: the worker measured no cell, so there is no light budget to audit");
        std::process::exit(1);
    }
    assert_eq!(
        got.len(),
        cells.len(),
        "the worker did not answer every cell"
    );

    // ---- the two branches, term by term ---------------------------------------
    // **One geometry, two ways of shading it**: an up-facing surface seen from above. `ndotv` is
    // `1`, so the silhouette — and therefore the rim — is zero, which leaves the three terms that
    // a unit mismatch would move: ambient, beam, sky.
    let normal = [0.0, 1.0, 0.0];
    let view = [0.0, 1.0, 0.0];
    let rig_terms = rig_terms(&rig, normal, view);

    let report = |name: &str, terms: &Terms, bands: [f32; 6]| {
        let total = terms.total();
        let mapped = tone_map(total);
        println!(
            "\n{name}: bands direct({:.5e}, {:.5e}, {:.5e}) diffuse({:.5e}, {:.5e}, {:.5e})",
            bands[0], bands[1], bands[2], bands[3], bands[4], bands[5]
        );
        println!(
            "  ambient {:.5}  beam {:.5}  sky {:.5}  rim {:.5}  coat+spec {:.5}  emission {:.5}",
            luminance_of(terms.ambient),
            luminance_of(terms.beam),
            luminance_of(terms.sky),
            luminance_of(terms.rim),
            luminance_of(terms.coat_and_spec),
            luminance_of(terms.emission)
        );
        println!(
            "  total luminance {:.5} (rgb {:.5}, {:.5}, {:.5}) -> tone-mapped {:.5}",
            luminance_of(total),
            total[0],
            total[1],
            total[2],
            luminance_of(mapped)
        );
    };

    // **The rig's beam as the shader multiplies it**: colour × intensity, whose luminance is the
    // number a measured beam has to match on a cell with nothing in front of it.
    let rig_beam = match rig.lights.first() {
        Some(light) => [
            channel(light.colour, 0) * light.intensity,
            channel(light.colour, 1) * light.intensity,
            channel(light.colour, 2) * light.intensity,
        ],
        None => [0.0; 3],
    };
    let rig_beam_luminance = luminance_of(rig_beam);
    assert!(
        rig_beam_luminance > 0.0,
        "the tick this example picks has no light in the rig, so there is no beam to compare"
    );
    let mut failures: Vec<String> = Vec::new();
    let mut worst_ratio: f64 = 0.0;
    let mut brightest_open = 0.0f64;
    let mut brightest_shaded = 0.0f64;
    let mut max_delivered_share = 0.0f64;
    let mut brightest_cell = None;
    let mut shown_open = false;
    let mut shown_shaded = false;
    let mut sky_share: f64 = 0.0;
    for m in &got {
        let bands = exposed_bands(m.bands, grade);
        let terms = field_terms(bands, &rig, normal, view);
        let total = luminance_of(terms.total());
        let field_beam = [bands[2] as f64, bands[1] as f64, bands[0] as f64];
        let delivered = luminance_of(field_beam);
        let share = delivered / rig_beam_luminance;
        if share > max_delivered_share {
            max_delivered_share = share;
            brightest_cell = Some((m.cell, field_beam, bands));
        }
        if delivered > 0.9 * rig_beam_luminance {
            // Unobstructed: the two branches must agree. The ratio is field / rig.
            for i in 0..3 {
                if rig_beam[i] > 1e-6 {
                    let ratio = field_beam[i] / rig_beam[i];
                    worst_ratio = worst_ratio.max((ratio - 1.0).abs());
                    if !shown_open {
                        println!(
                            "\nfirst unobstructed cell at {:?}: field beam [{:.5}, {:.5}, {:.5}] \
                             against the rig's [{:.5}, {:.5}, {:.5}] — ratio {:.3}",
                            m.cell,
                            field_beam[0],
                            field_beam[1],
                            field_beam[2],
                            rig_beam[0],
                            rig_beam[1],
                            rig_beam[2],
                            ratio
                        );
                        report("  the field's own terms", &terms, bands);
                        report("  the same geometry on the rig", &rig_terms, [0.0; 6]);
                        shown_open = true;
                    }
                }
            }
            brightest_open = brightest_open.max(total);
            sky_share =
                sky_share.max(luminance_of(terms.sky) / luminance_of(terms.ambient).max(1e-30));
        } else if delivered < 0.01 * rig_beam_luminance {
            brightest_shaded = brightest_shaded.max(total);
            if !shown_shaded {
                report(
                    &format!("first shaded cell at {:?} (field)", m.cell),
                    &terms,
                    bands,
                );
                report("  the same geometry on the rig", &rig_terms, [0.0; 6]);
                shown_shaded = true;
            }
        }
    }
    // **A window with no unobstructed cell cannot answer the question this example exists for.**
    // Said plainly rather than reported as a 100 % shortfall, which is what dividing by a beam no
    // cell ever received would look like.
    if !shown_open {
        eprintln!(
            "REFUSED: none of the {} measured cells had an unobstructed beam, so the seam between \
             the two branches is not measurable in this window",
            got.len()
        );
        std::process::exit(1);
    }
    // ---- the reports, as numbers ----------------------------------------------
    let rig_total = rig_terms.luminance();
    let rig_mapped = tone_map(rig_terms.total());
    println!(
        "\nthe rig's own total for this geometry: luminance {rig_total:.5} -> tone-mapped {:.5}",
        luminance_of(rig_mapped)
    );
    println!(
        "brightest measured open cell {brightest_open:.5}, brightest measured shaded cell \
         {brightest_shaded:.5}, the rig would give {rig_total:.5}"
    );
    println!(
        "the field's sky as a share of the rig's ambient on a measured cell: {:.3} %",
        100.0 * sky_share
    );
    if let Some((cell, beam, bands)) = brightest_cell {
        println!(
            "brightest beam in the window: {cell:?}, {:.1} % of the rig's — field {beam:?} against \
             the rig's {rig_beam:?}, bands direct({:.5e}, {:.5e}, {:.5e}) diffuse({:.5e}, {:.5e}, \
             {:.5e})",
            100.0 * max_delivered_share,
            bands[0],
            bands[1],
            bands[2],
            bands[3],
            bands[4],
            bands[5]
        );
    }

    // 1. The two branches deliver one light on an unobstructed cell.
    if worst_ratio > 0.02 {
        failures.push(format!(
            "a measured corner's beam is {:.1} % away from the rig's own beam — the two branches \
             are in different units",
            100.0 * worst_ratio
        ));
    }
    // 2. A shaded measured corner is darker than the rig, never brighter.
    if brightest_shaded > rig_total + 1e-9 {
        failures.push(format!(
            "a measured shaded cell reads {brightest_shaded:.5} against the rig's {rig_total:.5} — \
             the shadow path is brighter than the light path"
        ));
    }
    // 3. **The seam**: a measured clear corner draws the rig's own light. This is the number the
    //    frame's brightness is made of, so it holds the beam and the double-added sky together —
    //    the sky's share is printed above and is the whole of the difference here (about one per
    //    cent of the frame, one 8-bit step after the tone map).
    let against_rig = (brightest_open - rig_total) / rig_total;
    if against_rig.abs() > 0.02 {
        failures.push(format!(
            "a measured clear corner draws {:.1} % {} the light the rig would give it \
             ({brightest_open:.5} against {rig_total:.5}) — the two branches are two suns",
            100.0 * against_rig.abs(),
            if against_rig > 0.0 {
                "more than"
            } else {
                "less than"
            }
        ));
    }
    // 4. Nothing the frame draws leaves the range the display transform occupies.
    let mapped_open = tone_map([brightest_open; 3]);
    if !mapped_open.iter().all(|v| (0.0..=1.0).contains(v)) {
        failures.push(format!(
            "a measured open cell maps outside [0, 1]: {mapped_open:?}"
        ));
    }

    if failures.is_empty() {
        println!("\nevery term is inside the range it should occupy");
    } else {
        for failure in &failures {
            eprintln!("REFUSED: {failure}");
        }
        std::process::exit(1);
    }
}
