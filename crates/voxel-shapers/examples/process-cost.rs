//! **What does one turn of the process roster cost?** — `cargo run --offline -p
//! voxel-shapers --example process-cost`
//!
//! The game steps the roster **once per tick, on the render thread** (`App::drive_processes`
//! in `crates/shploingus/src/main.rs`), and the clock runs at 20 Hz, so one turn has
//! **50 ms** to spend before it costs the world a frame. This measures that turn rather
//! than guessing at it, and splits it two ways: per tick (the cadence rungs are ×1, ×4
//! and ×20, so a tick's cost is not one number) and per stage (so the answer names *which*
//! body to move rather than "the roster is slow").
//!
//! The request is the game's own: a 16³ unit box at octree level 4 centred on the eye, and
//! the four cardinal neighbour columns at 4 m. A different request measures a different
//! roster, so the two are kept in one place by naming it here: if `drive_processes` changes
//! its unit or its region, this example is measuring yesterday's question.
//!
//! ```text
//! cargo run --offline -p voxel-shapers --example process-cost
//! cargo run --offline --release -p voxel-shapers --example process-cost
//! ```

use std::time::{Duration, Instant};

use voxel_shapers::process::{
    ProcessRequest, ProcessScheduler, RegionSample, Site, UnitBox, FIELD_PLANE_HYDRO_RADIUS,
};
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::stages::Stage;

/// The half-width of the game's unit box, in voxels (`drive_processes`' own `HALF`).
const HALF: i32 = 8;
/// How many consecutive ticks are timed. Forty covers the ×4 rung ten times and the ×20
/// rung twice; tick 0 is the one turn that runs the whole chain.
const TICKS: u64 = 40;
/// The clock the client runs the world at (`voxel_client::clock::TICK_HZ`), and therefore
/// the wall-clock budget one turn has.
const TICK_HZ: f64 = 20.0;

/// Every stage that has a body, in the roster's own order.
const BODIES: [Stage; 15] = [
    Stage::Tectonics,
    Stage::Fields,
    Stage::Geology,
    Stage::Terrain,
    Stage::Hydrology,
    Stage::Erosion,
    Stage::Climate,
    Stage::Atmosphere,
    Stage::Weather,
    Stage::Substrate,
    Stage::Delta,
    Stage::Transition,
    Stage::SoilTransport,
    Stage::Ecology,
    Stage::Earthworks,
];

fn main() {
    let spec = WorldSpec::new(2468, Config::default());
    let mut scheduler = ProcessScheduler::world_defaults().expect("the roster validates");

    let site = Site::new(0, 0, 0);
    let unit = UnitBox::new(
        [site.x - HALF, site.y - HALF, site.z - HALF],
        [site.x + HALF, site.y + HALF, site.z + HALF],
        4,
    );
    let region = [
        RegionSample { site: Site::new(site.x + 4, site.y, site.z), distance_m: 4.0 },
        RegionSample { site: Site::new(site.x - 4, site.y, site.z), distance_m: 4.0 },
        RegionSample { site: Site::new(site.x, site.y, site.z + 4), distance_m: 4.0 },
        RegionSample { site: Site::new(site.x, site.y, site.z - 4), distance_m: 4.0 },
    ];

    // The roster is stepped as the game steps it: once per tick, in order, from tick 0. A
    // tick's cost is a function of the cadence it lands on, and stepping one tick twice is
    // the roster's documented no-op rather than a second measurement.
    let mut per_tick: Vec<(u64, Duration, usize, usize)> = Vec::new();
    // Per stage, summed over the same ticks. `step` does not hand out its own internal
    // split, so each body is timed on its own through `body_of` — the share is read as a
    // share, and its sum is **not** the turn's cost (the roster also computes five
    // identities per process, which this does not).
    let mut stage_sum = [Duration::ZERO; BODIES.len()];
    let mut total = Duration::ZERO;
    for tick in 0..=TICKS {
        let request = ProcessRequest {
            site,
            tick,
            hydro_radius: FIELD_PLANE_HYDRO_RADIUS,
            region: &region,
            unit: Some(unit),
            entries: &[],
        };
        let started = Instant::now();
        let evidence = scheduler.step(&spec, &request).expect("the tick advances");
        let elapsed = started.elapsed();
        total += elapsed;
        let ran = evidence.iter().filter(|e| e.ran()).count();
        per_tick.push((tick, elapsed, ran, evidence.len()));

        for (slot, stage) in BODIES.iter().enumerate() {
            let Some(body) = voxel_shapers::process::body_of(*stage) else { continue };
            let started = Instant::now();
            let readings = body(&spec, &request);
            stage_sum[slot] += started.elapsed();
            std::hint::black_box(&readings);
        }
    }

    let budget = std::time::Duration::from_secs_f64(1.0 / TICK_HZ);
    let ticks = per_tick.len() as u32;
    println!(
        "the game's own request: 16³ unit box (level 4), four cardinal columns at 4 m, \
         {TICKS} consecutive ticks from 0"
    );
    println!(
        "the clock runs {TICK_HZ:.0} Hz, so one turn may spend {:.0} ms",
        budget.as_secs_f64() * 1000.0
    );
    println!("--- per tick ---");
    for (tick, elapsed, ran, due) in &per_tick {
        let ms = elapsed.as_secs_f64() * 1000.0;
        let over = if *elapsed > budget { "  <-- over budget" } else { "" };
        println!("  t{tick:<3} {ms:>9.3} ms  {ran} of {due} ran{over}");
    }
    let worst = per_tick.iter().max_by_key(|(_, d, _, _)| *d).expect("at least one tick");
    let steady: Duration =
        per_tick.iter().skip(1).map(|(_, d, _, _)| *d).sum::<Duration>() / (ticks - 1);
    println!(
        "\n{ticks} turns in {total:?}; mean of t1..t{TICKS} {:.3} ms, worst t{} {:.3} ms \
         ({:.1}× the {:.0} ms budget)",
        steady.as_secs_f64() * 1000.0,
        worst.0,
        worst.1.as_secs_f64() * 1000.0,
        worst.1.as_secs_f64() / budget.as_secs_f64(),
        budget.as_secs_f64() * 1000.0
    );
    println!("--- each body over the same request, mean per tick ---");
    let mut ranked: Vec<(Stage, Duration)> =
        BODIES.iter().copied().zip(stage_sum).collect();
    ranked.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
    let body_total: f64 = ranked.iter().map(|(_, d)| d.as_secs_f64()).sum();
    for (stage, d) in &ranked {
        let per = d.as_secs_f64() / f64::from(ticks) * 1000.0;
        println!(
            "  {:<14} {per:>8.3} ms  ({:.0} % of the bodies' own time)",
            stage.name(),
            100.0 * d.as_secs_f64() / body_total
        );
    }
    let bodies_per_turn = body_total / f64::from(ticks) * 1000.0;
    println!(
        "\nthe bodies sum to {bodies_per_turn:.3} ms per turn; the turn itself was {:.3} ms",
        steady.as_secs_f64() * 1000.0
    );
}
