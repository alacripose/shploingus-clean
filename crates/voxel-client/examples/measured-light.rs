//! **The world's own light, measured, over the world's own chunks** — the check that the frame is
//! lit by the world rather than by the rig, and that the night's light is the Moon's.
//!
//! Run from the workspace root:
//!
//! ```text
//! cargo run --offline --release -p voxel-client --example measured-light
//! ```
//!
//! `voxel_client::CellField` is what the game's frame now reads — see `shploingus`'s `light_field`
//! and its `light-field` line — and `voxel_client::skylight::WorldLight::bands_across` is the solver
//! under it. Neither is visible in a picture alone: a scene lit by a rig and a scene lit by a field
//! both come out shaded. What *is* visible is the difference between cells, and this prints it as
//! numbers:
//!
//! * **an air cell above the ground** reads the beam the sky above it delivers — near the top of
//!   the model's own beam, since nothing is in the way;
//! * **a cell inside the ground under the same column** reads **exactly zero** in all six bands,
//!   because the matter in the way stops the beam rather than dimming it — a measurement of rock,
//!   and a number a rig-lit world cannot produce at all;
//! * **the same measurement at night** is the Moon's, not a second lamp: it is the Sun's own bands
//!   reflected off the Moon's declared reflectance, delivered from where the Moon's own orbit puts
//!   it, and it is orders dimmer than the day — which is what a body that reflects sunlight is.
//!
//! Every reading is taken **against the same tick's own delivery through an empty world** — the
//! model's beam with nothing to stop it — so the numbers are transmittances and not two constants
//! being compared. That reference is what makes the claim falsifiable rather than photographic.
//!
//! It also times the marches, because the frame measures cells under a budget
//! (`voxel_client::field::CELLS_PER_FRAME`): the milliseconds a cell costs here are what turns that
//! budget into a number of milliseconds, and a claim about a frame's light that does not carry its
//! own cost is half a claim.
//!
//! **A refusal is a refusal.** A world whose open cells are dark, whose buried cells are lit, or
//! whose night is lit by nothing exits non-zero and says which — an example that printed the
//! numbers and exited zero whatever they were would be a report nobody could fail.

use std::time::Instant;

use voxel_appearance::sky::SkyInputs;
use voxel_client::field::{CellField, ChunkTrees, REACH_M};
use voxel_mesh::{MeshConfig, SparseVoxelOctree};
use voxel_model::{ChunkPos, SubvoxelIndex, VoxelPos, CHUNK_SIDE};
use voxel_shapers::{Config, WorldSpec};
use voxel_stream::light::{ChunkMatter, Matter};
use voxel_stream::WorldSource;
use voxel_work::{ChunkWindow, Streamer};

/// The window this example streams: one chunk each way, one up and one down.
///
/// **Small on purpose, and big enough for the march to mean something.** A shadow ray has to be
/// able to leave the cell's own chunk and be charged by the next one — that is the half of the
/// solver a single-chunk walk cannot show — and radius one puts eight neighbours around the centre
/// chunk for it to cross into.
const WINDOW: ChunkWindow = ChunkWindow { radius_chunks: 1, vertical_chunks: 1 };

/// The ceiling for the streamer's meshes and trees, in bytes. This example reads the trees, and the
/// mesher's own output is charged to the same ceiling (`Frame::bytes`), so the number covers both —
/// the default the game ships with is larger, and this is a window of 27 chunks.
const CEILING_BYTES: usize = 256 * 1024 * 1024;

/// **A world of nothing**, so the same tick's beam can be measured with no world in the way.
///
/// Every chunk is an empty tree, which is a legal chunk of air: the march charges nothing and the
/// transmittance comes out at exactly `1.0`. What the bands are then is **the model's own delivery
/// at this tick and this address** — the number the world's cells are a fraction of, and the reason
/// no reading below is compared against a constant.
struct Empty(std::sync::Arc<SparseVoxelOctree>);

impl ChunkTrees for Empty {
    fn tree_at(&self, _pos: ChunkPos) -> Option<std::sync::Arc<SparseVoxelOctree>> {
        Some(std::sync::Arc::clone(&self.0))
    }
}

fn main() {
    let spec = WorldSpec::new(voxel_client::game::GAME_SEED, Config::default());
    let source = WorldSource::new(spec);
    let registry = voxel_shapers::materials::registry().expect("the registry");
    let config = MeshConfig::from_registry(&registry).expect("a mesh config");

    // **The window is centred on the player's own spawn**, not on the origin: the shipped world's
    // `(0, 0, 0)` is not where the game puts a body, and a check that measured a different column
    // than the one the player stands in would be a check about a world nobody plays.
    let eye = voxel_client::game::spawn_eye(&voxel_client::game::World::shipped());
    let centre = VoxelPos::new(eye[0] as i32, eye[1] as i32, eye[2] as i32).chunk();
    let mut streamer = Streamer::new(source, config, 2, CEILING_BYTES)
        .expect("a streamer")
        .with_window(WINDOW)
        .expect("the window validates");
    streamer.set_centre(centre);
    let settled = streamer.settle().expect("the window settles");
    println!(
        "window: radius {} chunks, {} vertical, centred on {:?} — {} of {} chunks resident",
        WINDOW.radius_chunks,
        WINDOW.vertical_chunks,
        centre,
        settled.resident,
        settled.wanted
    );

    let inputs = SkyInputs::default();
    let empty = Empty(std::sync::Arc::new(SparseVoxelOctree::new()));
    let mut failures: Vec<String> = Vec::new();

    // ---- by day ----------------------------------------------------------
    // **The highest the Sun gets in the first day, found in the model's own year rather than
    // chosen.** This world's day does not reach 45 degrees — the search returned nothing when it
    // asked for that — so the tick is the day's own maximum rather than a number this example
    // picked to make its own picture come out.
    let day = voxel_shapers::climate::TICKS_PER_DAY;
    let altitude = |t: u64| voxel_shapers::sky::solar_position(&spec, t).altitude_deg;
    let day_tick = (0..day)
        .max_by(|a, b| altitude(*a).total_cmp(&altitude(*b)))
        .expect("a tick in the first day");
    let sun = voxel_shapers::sky::solar_position(&spec, day_tick);
    println!(
        "\nday: tick {day_tick}, the sun {:.1} deg up at azimuth {:.1} deg",
        sun.altitude_deg, sun.azimuth_deg
    );

    let mut day_field = CellField::of(&spec, day_tick, inputs, 0, None)
        .expect("the shipped atmosphere is described");
    assert!(
        !day_field.light().beam_is_moon(),
        "the sun is up, so the day's beam is not the moon's"
    );
    day_field.begin_frame();
    let origin = centre.origin();
    let columns = measure(&mut day_field, &streamer, origin);
    // **A second field, for the empty world.** A field is a measurement of *one* world — its cells
    // are cached by address and a second world's answers would be indistinguishable from the
    // first's — so the reference is its own field at the same tick rather than the same field asked
    // twice about two worlds.
    let mut day_reference = CellField::of(&spec, day_tick, inputs, 0, None)
        .expect("the shipped atmosphere is described");
    let delivery = day_reference
        .measure(&empty, VoxelPos::new(origin.x, origin.y + CHUNK_SIDE - 1, origin.z).into())
        .map(|(bands, _)| bands)
        .expect("an empty world always has a measurement");
    let delivered_direct = delivery[0] + delivery[1] + delivery[2];
    println!(
        "the same tick with no world in the way: direct {:.4} of the sky's own beam, diffuse {:.4}",
        delivered_direct,
        delivery[3] + delivery[4] + delivery[5]
    );
    let brightest = columns.brightest_direct;
    println!(
        "{} solid column(s) of the centre chunk, read at {:.2} ms/cell ({:.1} ms for {} cells, \
         reach {REACH_M:.0} m)",
        columns.solid,
        columns.millis_per_cell(),
        columns.elapsed_ms,
        columns.cells
    );
    println!(
        "open air: mean direct {:.4} ({:.1} % of what the sky delivers), brightest {:.4} ({:.1} %), \
         mean diffuse {:.4}",
        columns.mean_direct(),
        100.0 * columns.mean_direct() as f64 / delivered_direct.max(1e-9) as f64,
        brightest,
        100.0 * brightest as f64 / delivered_direct.max(1e-9) as f64,
        columns.mean_diffuse(),
    );
    if columns.enclosed == 0 {
        failures.push("no cell in the centre chunk was enclosed by rock on all twenty-six sides"
            .into());
    }
    println!(
        "enclosed by rock on all twenty-six sides: {} of {} cell(s) read exactly zero in all six \
         bands ({:.0} % of the {} column(s))",
        columns.buried_zero,
        columns.buried,
        100.0 * columns.buried as f64 / columns.solid.max(1) as f64,
        columns.solid
    );
    println!(
        "open but unlit cells (a roof or the terrain's own shadow): {}",
        columns.shaded
    );
    if columns.solid == 0 {
        failures.push("no column of the centre chunk had its surface inside the chunk".into());
    }
    if (brightest as f64) < 0.9 * delivered_direct.max(1e-9) as f64 {
        failures.push(format!(
            "the brightest open cell reads {brightest:.4} against the {delivered_direct:.4} the sky \
             delivers with the sun {:.1} deg up — the world is not being lit by the sun",
            sun.altitude_deg
        ));
    }
    if columns.buried_zero != columns.buried {
        failures.push(format!(
            "{} of {} cells enclosed by rock let light through — matter is not stopping the beam",
            columns.buried - columns.buried_zero,
            columns.buried
        ));
        // **What the leak is made of, printed rather than guessed at.** A cell whose twenty-six
        // neighbours all hold matter in their *centre subvoxel* and that still reads light is either
        // a subvoxel-resolution hole in one of them — the stored level is a voxel of 64 cells, and a
        // neighbour can be a quarter full — or a walk that is not charging what it should. The
        // per-neighbour fill counts tell the two apart: a minimum below 64 of 64 subvoxels means the
        // light went through a partly-filled neighbour, and 64 means it did not.
        for (cell, bands) in &columns.leaks {
            let fills: Vec<usize> = (-1..=1)
                .flat_map(|dy| {
                    (-1..=1).flat_map(move |dz| (-1..=1).map(move |dx| (dx, dy, dz)))
                })
                .filter(|(dx, dy, dz)| !(*dx == 0 && *dy == 0 && *dz == 0))
                .map(|(dx, dy, dz)| filled(&streamer, cell[0] + dx, cell[1] + dy, cell[2] + dz))
                .collect();
            eprintln!(
                "  leak at {cell:?}: bands {bands:?}; the twenty-six neighbours hold {:?} of their \
                 64 subvoxels (min {})",
                {
                    let mut sorted = fills.clone();
                    sorted.sort_unstable();
                    sorted.dedup();
                    sorted
                },
                fills.iter().copied().min().unwrap_or(0)
            );
            // **Where the leak is, in two measurements rather than one argument.** The same cell,
            // marched with a reach that cannot leave its own neighbourhood (1.5 m — the first
            // neighbour every way is within a metre) and with the frame's own 160 m. A leak that
            // appears in the short reach is the first chunk's own march; one that appears only in
            // the long reach is the **hop** between chunks, which is the half of the solver a
            // single-chunk tree cannot show.
            let centre_m = [cell[0] as f64 + 0.5, cell[1] as f64 + 0.5, cell[2] as f64 + 0.5];
            for reach in [1.5f64, REACH_M] {
                match day_field.light().bands_across(&streamer, centre_m, reach) {
                    Some((bands, absent)) => eprintln!(
                        "  reach {reach:.1} m: direct {:.6}, diffuse {:.6}, {absent} hop(s) past a \
                         chunk not resident",
                        bands[0] + bands[1] + bands[2],
                        bands[3] + bands[4] + bands[5]
                    ),
                    None => eprintln!("  reach {reach:.1} m: no measurement at all"),
                }
            }
            // And the walk itself, over the cell's own tree with the first hop's rule: the five
            // stencil directions and the vertical, from the cell's own centre.
            let pos = VoxelPos::new(cell[0], cell[1], cell[2]);
            if let Some(tree) = streamer.tree(pos.chunk()) {
                let corner = pos.chunk().origin();
                let matter = ChunkMatter::new(tree, [corner.x, corner.y, corner.z]);
                let from = [
                    cell[0] as f64 + 0.5 - corner.x as f64,
                    cell[1] as f64 + 0.5 - corner.y as f64,
                    cell[2] as f64 + 0.5 - corner.z as f64,
                ];
                let mut walked = Vec::new();
                for direction in [[0.0, 1.0, 0.0], [-1.0, 1.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, -1.0], [0.0, 1.0, 1.0]] {
                    walked.push(voxel_stream::light::transmittance_tree(
                        &matter, tree, from, direction, REACH_M,
                    ));
                }
                eprintln!("  one tree, first-hop rule: {walked:?}");
            }
        }
    }
    if columns.mean_diffuse() <= 0.0 {
        failures.push("the open cells read no sky at all, so the dome is not reaching them".into());
    }

    // ---- the frame's budget, and the field's own reuse --------------------
    // **The same cells asked again.** The second pass is the cache's, and it is the number that says
    // whether a frame's corners cost marches or hash lookups — a frame asks at every corner of every
    // quad it builds, so this difference is the difference between a usable frame and a slideshow.
    let marched = day_field.adopted();
    day_field.begin_frame();
    let again = Instant::now();
    let repeated = measure(&mut day_field, &streamer, origin);
    let repeat_ms = again.elapsed().as_secs_f64() * 1000.0;
    println!(
        "\nthe same {} cell(s) asked again: {:.2} ms against {:.2} ms, with {} answered from the \
         field's own measurements and {} measured",
        repeated.cells,
        repeat_ms,
        columns.elapsed_ms,
        day_field.answered(),
        day_field.adopted()
    );
    if day_field.answered() == 0 {
        failures.push("the second pass over the same cells measured every one of them again".into());
    }
    // **And the cost is the worker's, not the frame's**: the frame reads a hash map, and the bytes
    // that used to be a per-frame budget are now an ask's worth of marches on another thread.
    println!(
        "the measurement is off the frame: {} ask(s) of {} cells carry {:.1} ms a march here",
        marched.div_ceil(voxel_client::CELLS_PER_ASK as u64).max(1),
        voxel_client::CELLS_PER_ASK,
        columns.millis_per_cell() * voxel_client::CELLS_PER_ASK as f64
    );

    // ---- by night --------------------------------------------------------
    // **The first full Moon of the model's own year that is up in the dark**, found by asking the
    // orbit rather than by picking a tick: the phase moves over a synodic month, so a search inside
    // one day either finds a lit Moon or finds none.
    let mut night = None;
    'year: for d in 0..voxel_shapers::climate::DAYS_PER_YEAR {
        let start = d * day;
        if voxel_shapers::sky::moon_phase_fraction(start, 0) < 0.9 {
            continue;
        }
        let mut best: Option<(f64, u64)> = None;
        for t in start..start + day {
            let moon = voxel_shapers::sky::moon_position(&spec, t, 0);
            let up = !voxel_shapers::sky::SkyPhase::sun_is_up(altitude(t));
            if up && best.map(|(a, _)| moon.altitude_deg > a).unwrap_or(true) {
                best = Some((moon.altitude_deg, t));
            }
        }
        if let Some((altitude_deg, tick)) = best {
            if altitude_deg > 20.0 {
                night = Some((tick, altitude_deg));
                break 'year;
            }
        }
    }
    let (night_tick, moon_altitude) = night.unwrap_or_else(|| {
        failures.push("no night in the model's own year had a lit Moon more than 20 deg up".into());
        (day_tick, 0.0)
    });
    let moon = voxel_shapers::sky::moon_position(&spec, night_tick, 0);
    println!(
        "\nnight: tick {night_tick}, the Moon {moon_altitude:.1} deg up at azimuth {:.1} deg, \
         {:.0} % of its disc lit, elongation {:.1} deg",
        moon.azimuth_deg,
        100.0 * moon.phase_fraction,
        moon.elongation_deg
    );
    let mut night_field = CellField::of(&spec, night_tick, inputs, 0, None)
        .expect("the shipped atmosphere is described");
    if !night_field.light().beam_is_moon() {
        failures.push("the moon is up in the dark and the beam is not the moon's".into());
    }
    night_field.begin_frame();
    let night_columns = measure(&mut night_field, &streamer, origin);
    let mut night_reference = CellField::of(&spec, night_tick, inputs, 0, None)
        .expect("the shipped atmosphere is described");
    let night_delivery = night_reference
        .measure(&empty, VoxelPos::new(origin.x, origin.y + CHUNK_SIDE - 1, origin.z).into())
        .map(|(bands, _)| bands)
        .expect("an empty world always has a measurement");
    let moon_direct = (night_delivery[0] + night_delivery[1] + night_delivery[2]) as f64;
    println!(
        "the moonlight at the same address: direct {moon_direct:.3e} of the sky's own beam — \
         {:.2e} of the day's {delivered_direct:.4}",
        moon_direct / delivered_direct.max(1e-9) as f64
    );
    println!(
        "the Moon's own bands (B3, B4, B5), as the sun's spectrum times its reflectance: \
         {:.3e} {:.3e} {:.3e}",
        night_delivery[0], night_delivery[1], night_delivery[2]
    );
    println!(
        "open air under the Moon: mean direct {:.3e}, brightest {:.3e}, and {} of {} cell(s) \
         enclosed by rock read exactly zero",
        night_columns.mean_direct(),
        night_columns.brightest_direct,
        night_columns.buried_zero,
        night_columns.buried
    );
    if moon_direct <= 0.0 {
        failures.push("the night's sky delivered no light from the Moon at all".into());
    }
    if night_columns.buried_zero != night_columns.buried {
        failures.push(format!(
            "{} of {} cells enclosed by rock let moonlight through",
            night_columns.buried - night_columns.buried_zero,
            night_columns.buried
        ));
    }
    if night_delivery[0] + night_delivery[1] + night_delivery[2] > delivery[0] + delivery[1] + delivery[2]
    {
        failures.push("the Moon delivered more than the Sun at the same address".into());
    }

    // ---- the verdict -----------------------------------------------------
    if !failures.is_empty() {
        for failure in &failures {
            eprintln!("REFUSED: {failure}");
        }
        std::process::exit(1);
    }
    println!(
        "\nthe world's own light is a measurement: every band above came from the march, and the \
         night's came from the Moon's own orbit"
    );
}

/// One tick's worth of readings over the centre chunk's own columns.
#[derive(Default)]
struct Columns {
    solid: usize,
    enclosed: usize,
    cells: usize,
    buried: usize,
    buried_zero: usize,
    shaded: usize,
    direct: Vec<f32>,
    diffuse: Vec<f32>,
    /// The enclosed cells that were **not** dark, and their bands — the evidence a refusal is
    /// argued from rather than a count.
    leaks: Vec<([i32; 3], [f32; 6])>,
    brightest_direct: f32,
    elapsed_ms: f64,
}

impl Columns {
    fn mean_direct(&self) -> f32 {
        mean(&self.direct)
    }

    fn mean_diffuse(&self) -> f32 {
        mean(&self.diffuse)
    }

    fn millis_per_cell(&self) -> f64 {
        self.elapsed_ms / self.cells.max(1) as f64
    }
}

/// **How many of a voxel's 64 subvoxels hold matter** — the stored level's own composition, which
/// is the question a partly-filled neighbour is hiding behind.
fn filled(streamer: &Streamer, x: i32, y: i32, z: i32) -> usize {
    let pos = VoxelPos::new(x, y, z);
    let Some(tree) = streamer.tree(pos.chunk()) else {
        return 0;
    };
    let local = pos.local();
    let Ok(local) = voxel_model::LocalPos::new(local.x, local.y, local.z) else {
        return 0;
    };
    let mut filled = 0;
    for flat in 0..voxel_model::SUBVOXEL_COUNT {
        let Ok(at) = SubvoxelIndex::from_flat(flat) else {
            continue;
        };
        if !tree.material_at(local, at).is_empty() {
            filled += 1;
        }
    }
    filled
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        (values.iter().map(|v| *v as f64).sum::<f64>() / values.len() as f64) as f32
    }
}

/// **Every fourth column of the centre chunk: what the sky delivers to its surface, and what an
/// enclosed cell under it reads.**
///
/// The surface cell is the column's **topmost solid voxel**, found by the question the march itself
/// asks ([`Matter::subvoxel`]), and the air cell above it is what the sky delivers there.
///
/// The enclosed cell is a cell with **all twenty-six of its neighbours solid** — a full 3×3×3 of
/// rock — and that requirement is the whole of the care in this function. The first version of this
/// example asked only that two voxels directly above the cell be solid, and **15 of 63 cells "in the
/// ground" read light**: the Sun was 22° up, so a ray out of the cell's centre leaves its own column
/// after half a metre and is charged by the *neighbours*, which nobody had asked about. A cell is
/// underground for a beam when the matter is all around it, and the twenty-six sides are the honest
/// statement of that at the voxel's own scale.
/// **The frame's own path to a cell**: answer from the field if it has a measurement, and march it
/// on this thread if it does not.
///
/// The game splits these two — the frame only ever asks, and [`voxel_client::Measurer`] does the
/// marching — but an example measuring a field has no worker and no frame to protect.
fn asked(field: &mut CellField, streamer: &Streamer, cell: VoxelPos) -> Option<[f32; 6]> {
    let cell = voxel_client::field::SubvoxelPos::from(cell);
    match field.at(cell) {
        Some(bands) => Some(bands),
        None => field.measure(streamer, cell).map(|(bands, _)| bands),
    }
}

fn measure(field: &mut CellField, streamer: &Streamer, origin: VoxelPos) -> Columns {
    let at = SubvoxelIndex::new(2, 2, 2).expect("the middle of a voxel");
    let top = origin.y + CHUNK_SIDE - 1;
    let mut out = Columns::default();
    let started = Instant::now();
    for dx in (0..CHUNK_SIDE).step_by(4) {
        for dz in (0..CHUNK_SIDE).step_by(4) {
            let (wx, wz) = (origin.x + dx, origin.z + dz);
            // **Matter, asked of whichever chunk holds the voxel**, exactly as the march asks it:
            // the 3×3×3 neighbourhood below crosses chunk boundaries, and a neighbour read from the
            // wrong chunk's tree would be this example measuring a different world.
            let matter = |x: i32, y: i32, z: i32| -> bool {
                let Some(tree) = streamer.tree(VoxelPos::new(x, y, z).chunk()) else {
                    return false;
                };
                let corner = VoxelPos::new(x, y, z).chunk().origin();
                ChunkMatter::new(tree, [corner.x, corner.y, corner.z])
                    .subvoxel([x, y, z], at)
                    .is_some()
            };
            let solid = |y: i32| matter(wx, y, wz);
            let Some(surface) = (origin.y..=top).rev().find(|y| solid(*y)) else {
                continue;
            };
            if surface >= top {
                continue;
            }
            out.solid += 1;
            let open = asked(field, streamer, VoxelPos::new(wx, surface + 1, wz))
                .expect("the open cell");
            // The enclosed cell: the first voxel of the column, from the surface down, whose own
            // 3×3×3 neighbourhood is all matter. Its own cell is excluded from its own beam — that
            // is the march's first rule — so what has to block the light is the rock around it.
            let enclosed = (origin.y + 1..surface - 1).rev().find(|y| {
                (-1..=1).all(|dy| {
                    (-1..=1).all(|dz| (-1..=1).all(|dx| matter(wx + dx, y + dy, wz + dz)))
                })
            });
            let Some(buried_cell) = enclosed else {
                continue;
            };
            let buried = asked(field, streamer, VoxelPos::new(wx, buried_cell, wz))
                .expect("the enclosed cell");
            out.enclosed += 1;
            out.cells += 2;
            let direct = open[0] + open[1] + open[2];
            out.direct.push(direct);
            out.diffuse.push(open[3] + open[4] + open[5]);
            out.brightest_direct = out.brightest_direct.max(direct);
            if direct < 1e-3 {
                out.shaded += 1;
            }
            out.buried += 1;
            if buried.iter().all(|band| *band == 0.0) {
                out.buried_zero += 1;
            } else if out.leaks.len() < 4 {
                out.leaks.push(([wx, buried_cell, wz], buried));
            }
        }
    }
    out.elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    out
}
