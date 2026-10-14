//! **MEASUREMENT — the sixteen stages against the clock, and what one call costs.**
//!
//! This is the measurement the round ordered before the cut — *"measure twice, cut once"*
//! (the user's ruling, `hypergrilling-ledger.jsonl` event-0087: measure the sixteen stages
//! first) — taken as evidence for carrying the clock to every shaper (LEDGER C-92.0 item 3).
//!
//! Every row is one stage of `stages::ALL` and states three things:
//!
//! * **the cadence**, read from the stage itself (`Stage::cadence().period_ticks()`), never
//!   restated here, so this output cannot drift from H-8.3's ladder;
//! * **how it takes the clock**, measured by *calling the stage's own public entry point*
//!   at two ticks — an explicitly-ticked sampler at tick 0 and tick 24 000; a sampler that
//!   reads `WorldSpec::tick` through two specs; a stage whose entry points take no spec at
//!   all called with fixed arguments, where the absence of a clock argument is itself the
//!   measurement — there is no route for a tick to arrive through;
//! * **the channel its answer moves through** — `field` for a value the world samples (a
//!   change cannot be *recorded* through it), `record` for the earthworks channel, which is
//!   the only route by which a voxel's material changes (the world is the generator
//!   **composed with** the record), and `none` where nothing is implemented.
//!
//! The second table times each entry point. **The world is a cellular automata**, the
//! materials systems are going to carry machines, belts and fluidics, and a clock carried
//! to a shaper whose per-call cost nobody measured is how a tick budget dies — so cost is
//! part of the measurement, not an optimisation to be discovered later.
//!
//! The final sections re-take C-117's composite (`chunk_fields::material_at_composed`, the
//! substance of the call `WorldSource` makes) and prove the change channel: one recorded
//! `Modification` changes what the world composes.
//!
//! Run: `cargo run --offline -p voxel-shapers --example stage-clocks`

use std::time::Instant;

use voxel_model::world::{ActorId, PhysicalVerb};
use voxel_shapers::earthworks::{hand_change, Earthworks, HandChange};
use voxel_shapers::geology::Lithology;
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::stages::{Cadence, Stage, ALL};
use voxel_shapers::{
    atmosphere, caves, chunk_fields, climate, delta, ecology, erosion, geology, hydrology,
    materials, substrate, tectonics, terrain, transition, weather,
};

fn clock_name(c: Cadence) -> &'static str {
    match c {
        Cadence::EveryTick => "every tick",
        Cadence::Water => "water x4",
        Cadence::Sediment => "sediment x20",
        Cadence::Daily => "daily",
    }
}

/// "Does the tick move this answer?", asked through **two specs** — for the stages whose
/// entry point reads `WorldSpec::tick` rather than taking a tick argument.
fn two_specs<T: PartialEq>(
    probes: &[(f64, f64)],
    a: &WorldSpec,
    b: &WorldSpec,
    mut f: impl FnMut(&WorldSpec, f64, f64) -> T,
) -> (usize, T, T) {
    let mut moves = 0;
    let mut first: Option<(T, T)> = None;
    for (x, z) in probes {
        let va = f(a, *x, *z);
        let vb = f(b, *x, *z);
        if va != vb {
            moves += 1;
        }
        if first.is_none() {
            first = Some((va, vb));
        }
    }
    let (fa, fb) = first.expect("at least one probe");
    (moves, fa, fb)
}

/// The same question for the stages whose entry point takes the tick **explicitly**.
fn two_ticks<T: PartialEq>(
    probes: &[(f64, f64)],
    spec: &WorldSpec,
    t0: u64,
    t1: u64,
    mut f: impl FnMut(&WorldSpec, f64, f64, u64) -> T,
) -> (usize, T, T) {
    let mut moves = 0;
    let mut first: Option<(T, T)> = None;
    for (x, z) in probes {
        let va = f(spec, *x, *z, t0);
        let vb = f(spec, *x, *z, t1);
        if va != vb {
            moves += 1;
        }
        if first.is_none() {
            first = Some((va, vb));
        }
    }
    let (fa, fb) = first.expect("at least one probe");
    (moves, fa, fb)
}

/// Microseconds per call, over `calls` calls with a warm-up call first.
fn us_per_call<T>(calls: u32, mut f: impl FnMut() -> T) -> f64 {
    let _ = std::hint::black_box(f());
    let start = Instant::now();
    for _ in 0..calls {
        let _ = std::hint::black_box(f());
    }
    start.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn main() {
    let probes = [(0.5, 0.5), (16.5, 16.5), (-32.5, 48.5)];
    let spec = WorldSpec::new(2468, Config::default());
    let later = spec.at_tick(24_000);

    println!("the sixteen stages, cadence read from the stage, the clock measured through its own entry points");
    println!("seed 2468, probes {probes:?}; the later clock is tick 24 000 (one world day)");
    println!();
    println!(
        "{:<15} {:<11} {:>6}  {:<30} {:<66} channel",
        "stage", "cadence", "period", "sampler", "measurement (clock 0 -> clock 24 000)"
    );
    println!("{}", "-".repeat(170));

    for stage in ALL {
        let cadence = stage.cadence();
        let (sampler, measured, channel): (&str, String, &str) = match stage {
            Stage::Tectonics => {
                let (m, a, b) = two_specs(&probes, &spec, &later, |s, x, z| {
                    let p = tectonics::sample(s, x, z);
                    (p.boundary_strength, p.plate_id)
                });
                (
                    "sample(spec; x, z)",
                    format!(
                        "boundary {:.3} -> {:.3}, plate {} -> {}; moves {m}/{}",
                        a.0,
                        b.0,
                        a.1,
                        b.1,
                        probes.len()
                    ),
                    "field",
                )
            }
            Stage::Fields => (
                "blocks_at_level / FieldSpec",
                "level arithmetic and identity only; no (x, z, tick) entry point exists".to_string(),
                "declaration",
            ),
            Stage::Geology => {
                let (m, a, b) = two_specs(&probes, &spec, &later, |s, x, z| {
                    let surface = terrain::elevation_m(s, x, z);
                    let st = geology::stratum_at(s, x, z, surface, surface - 10.0);
                    (st.lithology, st.bed)
                });
                (
                    "stratum_at(spec; x, z, surf, y)",
                    format!("lithology {:?} -> {:?}, bed {} -> {}; moves {m}/{}", a.0, b.0, a.1, b.1, probes.len()),
                    "field",
                )
            }
            Stage::Terrain => {
                let (m, a, b) = two_specs(&probes, &spec, &later, terrain::elevation_m);
                (
                    "elevation_m(spec; x, z)",
                    format!("{:.3} m -> {:.3} m (delta {:+.3} m); moves {m}/{}", a, b, b - a, probes.len()),
                    "field",
                )
            }
            Stage::Hydrology => {
                let (m, a, b) = two_specs(&probes, &spec, &later, |s, x, z| {
                    let h = hydrology::sample(s, x, z, 4);
                    (h.water_depth_m, h.order)
                });
                (
                    "sample(spec; x, z, r=4)",
                    format!(
                        "water depth {:.5} m -> {:.5} m, order {} -> {}; moves {m}/{} (radius 4 so the probe stays cheap; the declared radius changes the watershed, not this comparison)",
                        a.0, b.0, a.1, b.1, probes.len()
                    ),
                    "field",
                )
            }
            Stage::Erosion => {
                let heights: Vec<f64> = (0..1024).map(|i| 5.0 + (i as f64 * 0.11).sin()).collect();
                let a = format!("{:?}", erosion::thermal(&heights, 32, 32, 1.0, 0.7, 0.5, 1));
                let b = format!("{:?}", erosion::thermal(&heights, 32, 32, 1.0, 0.7, 0.5, 1));
                (
                    "thermal(heights, .., rate, iters)",
                    format!("patch law only: no spec or tick argument exists to carry a clock (two identical calls equal: {})", a == b),
                    "record (unwired)",
                )
            }
            Stage::Climate => {
                let (m, a, b) = two_ticks(&probes, &spec, 0, 24_000, |s, x, z, t| {
                    let c = climate::sample(s, x, z, t);
                    (c.temperature_c, c.season_phase)
                });
                let (mm, ma, mb) = two_specs(&probes, &spec, &later, climate::moisture);
                (
                    "sample(spec; x, z, tick)",
                    format!(
                        "T {:.2} C -> {:.2} C, season {:.3} -> {:.3} (explicit tick, moves {m}/{}); moisture {:.5} -> {:.5} (spec tick, moves {mm}/{})",
                        a.0, b.0, a.1, b.1, probes.len(), ma, mb, probes.len()
                    ),
                    "process",
                )
            }
            Stage::Atmosphere => {
                let (m, a, b) = two_specs(&probes, &spec, &later, |s, _x, _z| atmosphere::sample(s, 120.0).pressure_kpa);
                (
                    "sample(spec; y)",
                    format!("p {:.5} kPa -> {:.5} kPa across two specs; moves {m}/{} (the spec, and so the clock, is not read)", a, b, probes.len()),
                    "process",
                )
            }
            Stage::Weather => {
                let (m, a, b) = two_ticks(&probes, &spec, 0, 24_000, weather::precipitation_mm);
                let (wm, wa, wb) = two_specs(&probes, &spec, &later, weather::wind_bearing_rad);
                (
                    "precipitation_mm / wind_bearing_rad",
                    format!(
                        "precip {:.5} -> {:.5} (explicit tick, moves {m}/{}); wind {:.4} -> {:.4} rad (spec tick, moves {wm}/{})",
                        a, b, probes.len(), wa, wb, probes.len()
                    ),
                    "process",
                )
            }
            Stage::Caves => {
                let (m, a, b) = two_specs(&probes, &spec, &later, |s, x, z| {
                    let surface = terrain::elevation_m(s, x, z);
                    caves::void_at(s, Lithology::Sedimentary, x, surface - 20.0, z, surface, &caves::CaveParams::for_spec(s))
                });
                (
                    "void_at(spec; ., x, y, z, surf, params)",
                    format!("{a:?} -> {b:?}; moves {m}/{}", probes.len()),
                    "field",
                )
            }
            Stage::Substrate => {
                let (m, a, b) = two_specs(&probes, &spec, &later, |s, _x, _z| {
                    substrate::soil_depth_m(s, Lithology::Sedimentary, 0.3, 0.5)
                });
                (
                    "soil_depth_m(spec; lith, slope, m)",
                    format!("{:.5} m -> {:.5} m; moves {m}/{} (the baseline rule: tick-free, and measured so)", a, b, probes.len()),
                    "baseline",
                )
            }
            Stage::SoilTransport => (
                "— none",
                "declared sixteenth stage; no entry point exists, so there is no clock to carry yet (H-16.3: the delta half of substrate)".to_string(),
                "record (declared)",
            ),
            Stage::Ecology => {
                let (m, a, b) = two_ticks(&probes, &spec, 0, 24_000, |s, x, z, t| {
                    let y = terrain::elevation_m(s, x, z);
                    ecology::holdridge_inputs(s, x, z, y, t)
                });
                (
                    "holdridge_inputs(spec; x, z, y, tick)",
                    format!("biotemperature {:.3} C -> {:.3} C, mean annual precip {:.4} -> {:.4} mm; moves {m}/{}", a.0, b.0, a.1, b.1, probes.len()),
                    "process",
                )
            }
            Stage::Delta => {
                let mut da = delta::DeltaState::default();
                da.avulse(&spec);
                let mut db = delta::DeltaState::default();
                db.avulse(&later);
                (
                    "transport_capacity / DeltaState",
                    format!("laws take no tick; avulsion bearing {:.5} -> {:.5} rad across two specs (the seed carries it)", da.bearing_rad, db.bearing_rad),
                    "record (unwired)",
                )
            }
            Stage::Transition => {
                let a = transition::blend_scalar(&[(0.5, 1.0), (0.5, 3.0)]).unwrap();
                let b = transition::blend_scalar(&[(0.5, 2.0), (0.5, 4.0)]).unwrap();
                (
                    "shepard_weights / blend_scalar",
                    format!("blend law; fixed weights give {:.5} -> {:.5} for inputs 1/3 vs 2/4 (no spec argument exists)", a, b),
                    "field",
                )
            }
            Stage::Earthworks => (
                "record / hand_change",
                "the change channel: no sampler; a Modification is recorded against (x, y, z) and never sampled from the clock".to_string(),
                "record",
            ),
        };
        println!(
            "{:<15} {:<11} {:>6}  {:<30} {:<66} {}",
            stage.name(),
            clock_name(cadence),
            cadence.period_ticks(),
            sampler,
            measured,
            channel
        );
    }

    // ------------------------------------------------------------------
    // Cost: what one call takes, because the world is a CA and the tick
    // budget is declared by what a stage costs times how often it is asked.
    // ------------------------------------------------------------------
    println!();
    println!("cost of one entry-point call, measured (us/call; chunk figures are that cost times the calls a chunk needs)");
    println!(
        "{:<15} {:<10} {:<12} one 32x32x32 chunk, where the arithmetic is meaningful",
        "stage", "calls", "us/call"
    );
    println!("{}", "-".repeat(120));

    let row = |stage: Stage, calls: u32, us: f64, note: String| {
        println!("{:<15} {:<10} {:<12.3} {}", stage.name(), calls, us, note);
    };

    let us = us_per_call(5_000, || tectonics::sample(&spec, 16.5, 16.5));
    row(Stage::Tectonics, 5_000, us, format!("x1024 columns = {:.2} ms/chunk", us * 1024.0 / 1000.0));

    let us = us_per_call(5_000, || {
        let surface = terrain::elevation_m(&spec, 16.5, 16.5);
        geology::stratum_at(&spec, 16.5, 16.5, surface, surface - 10.0)
    });
    row(Stage::Geology, 5_000, us, format!("x1024 columns = {:.2} ms/chunk", us * 1024.0 / 1000.0));

    let us = us_per_call(5_000, || terrain::elevation_m(&spec, 16.5, 16.5));
    row(Stage::Terrain, 5_000, us, format!("x1024 columns = {:.2} ms/chunk", us * 1024.0 / 1000.0));

    let us = us_per_call(50, || hydrology::sample(&spec, 16.5, 16.5, 4));
    row(Stage::Hydrology, 50, us, format!("x1024 columns (radius 4, cheaper than the declared 12) = {:.1} ms/chunk", us * 1024.0 / 1000.0));

    let heights: Vec<f64> = (0..1024).map(|i| 5.0 + (i as f64 * 0.11).sin()).collect();
    let us = us_per_call(200, || erosion::thermal(&heights, 32, 32, 1.0, 0.7, 0.5, 1));
    row(Stage::Erosion, 200, us, "one 32x32 patch pass, 1 iteration".to_string());

    let us = us_per_call(1_000, || climate::sample(&spec, 16.5, 16.5, 9_000));
    row(Stage::Climate, 1_000, us, format!("x1024 columns = {:.2} ms/chunk (the sample evaluates terrain internally)", us * 1024.0 / 1000.0));

    let us = us_per_call(5_000, || atmosphere::sample(&spec, 120.0).density_kg_m3);
    row(Stage::Atmosphere, 5_000, us, format!("x1024 columns = {:.2} ms/chunk", us * 1024.0 / 1000.0));

    let us = us_per_call(500, || weather::sample(&spec, 16.5, 16.5, 9_000));
    row(Stage::Weather, 500, us, format!("x1024 columns = {:.2} ms/chunk (the gradient reads four elevations)", us * 1024.0 / 1000.0));

    let us = us_per_call(5_000, || {
        let surface = terrain::elevation_m(&spec, 16.5, 16.5);
        caves::void_at(&spec, Lithology::Sedimentary, 16.5, surface - 20.0, 16.5, surface, &caves::CaveParams::for_spec(&spec))
    });
    row(Stage::Caves, 5_000, us, format!("x32768 voxels = {:.1} ms/chunk, if a whole chunk were asked", us * 32768.0 / 1000.0));

    let us = us_per_call(50_000, || substrate::soil_depth_m(&spec, Lithology::Sedimentary, 0.3, 0.5));
    row(Stage::Substrate, 50_000, us, "per voxel below the surface in the composition".to_string());

    let us = us_per_call(500, || {
        let y = terrain::elevation_m(&spec, 16.5, 16.5);
        ecology::holdridge_inputs(&spec, 16.5, 16.5, y, 9_000)
    });
    row(Stage::Ecology, 500, us, format!("x1024 columns = {:.2} ms/chunk (twelve monthly climates per call)", us * 1024.0 / 1000.0));

    let us = us_per_call(50_000, || {
        delta::deposition_m(delta::transport_capacity(1.5), 2.0, 0.01)
    });
    row(Stage::Delta, 50_000, us, "the deposition law alone".to_string());

    let us = us_per_call(50_000, || transition::blend_scalar(&[(0.4, 2.0), (0.6, 4.0)]));
    row(Stage::Transition, 50_000, us, "four samples blended".to_string());

    let us = us_per_call(2_000, || {
        chunk_fields::material_at_composed(&spec, &Earthworks::new(), 16, 20, 16)
    });
    row(
        Stage::Earthworks,
        2_000,
        us,
        "chunk_fields::material_at_composed, the composed voxel the streamer reads".to_string(),
    );

    // ------------------------------------------------------------------
    // The change channel, proven: one recorded Modification changes the world.
    // ------------------------------------------------------------------
    println!();
    println!("the change channel, measured: one earthworks entry changes what the world composes");
    let (x, y, z) = (3, 20, 4);
    let empty = Earthworks::new();
    let from = chunk_fields::material_at_composed(&spec, &empty, x, y, z).unwrap();
    let to = if from.is_empty() { materials::IGNEOUS } else { materials::AIR };
    let change = hand_change(HandChange {
        x,
        y,
        z,
        from,
        to,
        reason: "stage-clocks measurement".to_string(),
        actor: ActorId::player(),
        verb: PhysicalVerb::Place,
    });
    match change {
        Ok(m) => {
            let mut record = Earthworks::new();
            match record.record(m) {
                Ok(()) => {
                    let after = chunk_fields::material_at_composed(&spec, &record, x, y, z).unwrap();
                    println!("  ({x}, {y}, {z})  {from:?} -> {after:?}  (asked for {to:?})  with 1 recorded entry");
                }
                Err(set) => println!("  refused by the record: {set:?}"),
            }
        }
        Err(refusal) => println!("  refused by hand_change: {refusal}"),
    }

    // ------------------------------------------------------------------
    // The composite the client actually reads, re-taken from C-117.
    // ------------------------------------------------------------------
    println!();
    println!("the composite the streamer reads, re-measured at two spec ticks:");
    let mut compared = 0u32;
    let mut differing = 0u32;
    for gx in (-96..=96).step_by(6) {
        for gz in (-96..=96).step_by(6) {
            let x = gx;
            let z = gz;
            let surface = terrain::elevation_m(&spec, x as f64 + 0.5, z as f64 + 0.5).floor() as i32;
            for y in (surface - 2)..=(surface + 2) {
                let a = chunk_fields::material_at_composed(&spec, &empty, x, y, z).unwrap();
                let b = chunk_fields::material_at_composed(&later, &empty, x, y, z).unwrap();
                compared += 1;
                if a != b {
                    differing += 1;
                }
            }
        }
    }
    println!("  {differing} of {compared} voxels differ when the spec tick moves (C-117's count, re-taken).");
    println!("  the changes above the surface tick are seed changes; the changes below it are picked up by the same call the streamer makes, so whatever the cut decides per row, this is the call it must keep honest.");
}
