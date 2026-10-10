//! The process roster, **driven** on the cadence tick.
//!
//! `ProcessScheduler::step` is the runtime the roster was built for, and until this
//! example it had no caller outside tests. This runs it over several ticks and
//! prints the evidence it produces: which stages were due, which **ran** and how
//! many readings they gave, and which **refused** and why. It is the honest answer
//! to *"do the shapers work on the tick?"* — a stage that owes a body says so here
//! rather than silently doing nothing.
//!
//! **The request carries both a `unit` and a `region`.** They are the two region
//! forms the structural and neighbour-reading bodies were owed: `Fields` and
//! `Erosion` read the unit's own area, while `SoilTransport` and `Transition` are
//! laws about neighbours. Supplying only one leaves the stages that read the other
//! refusing by name — which is why the earlier cut of this example showed 11 of 16
//! running rather than 14.
//!
//! `cargo run --offline -p voxel-shapers --example process-tick`

use voxel_shapers::process::{
    ProcessEvidence, ProcessRequest, ProcessScheduler, RegionSample, Site, UnitBox,
};
use voxel_shapers::spec::{Config, WorldSpec};

fn main() {
    let world = WorldSpec::new(2468, Config::default());
    let mut scheduler = ProcessScheduler::world_defaults().expect("the roster validates");
    // One unit of work: a 16³ region at octree level 4, carrying the site it centres
    // on. The structural stages (`fields`, `erosion`) read this region, so without it
    // they refuse by name and the whole chain behind them refuses on their dependency.
    let unit = UnitBox::new([0, 0, 0], [16, 16, 16], 4);
    let site = unit.centre_site();
    // The neighbours a neighbour-reading stage blends: the four cardinal columns of the
    // site's own cross, each at its true distance. `SoilTransport` reads them as
    // pairwise slopes and `Transition` as a partition of unity over their strata.
    let region = [
        RegionSample { site: Site::new(site.x + 4, site.y, site.z), distance_m: 4.0 },
        RegionSample { site: Site::new(site.x - 4, site.y, site.z), distance_m: 4.0 },
        RegionSample { site: Site::new(site.x, site.y, site.z + 4), distance_m: 4.0 },
        RegionSample { site: Site::new(site.x, site.y, site.z - 4), distance_m: 4.0 },
    ];

    // Tick 0 (the whole pipeline), the ×4 rung, the ×20 rung, and a mid-day tick.
    for tick in [0u64, 1, 4, 8, 20, 21] {
        let request = ProcessRequest {
            site,
            tick,
            hydro_radius: 4,
            region: &region,
            unit: Some(unit),
        };
        let evidence = scheduler.step(&world, &request).expect("the tick advances");
        report(tick, &evidence);
    }
}

fn report(tick: u64, evidence: &[ProcessEvidence]) {
    let ran = evidence.iter().filter(|e| e.ran()).count();
    let refused = evidence.len() - ran;
    println!("\ntick {tick}: {} due, {ran} ran, {refused} refused", evidence.len());
    for e in evidence {
        match &e.refusal {
            None => println!(
                "  {:<14} ran    {} reading(s)",
                e.process.stage().name(),
                e.readings.len()
            ),
            Some(condition) => println!(
                "  {:<14} {:<28} {}",
                e.process.stage().name(),
                format!("{condition:?}"),
                e.detail.as_deref().unwrap_or("")
            ),
        }
    }
}
