//! The cadence tick, driven for real.
//!
//! This is the demonstration the missing component needed: a [`TickDriver`] over
//! the full pipeline, advanced tick by tick over a chunk's **real** octree units,
//! printing each tick's receipt — which stages the ladder says are due, how much
//! work that is, and the digest of the plan. Two drivers over the same ticks are
//! compared so the determinism is measured and not asserted.
//!
//! `cargo run --offline -p voxel-work --example tick-driver`

use voxel_mesh::octree::SparseVoxelOctree;
use voxel_model::{ChunkPos, LocalPos, MaterialId, VoxelShape};
use voxel_shapers::stages::Stage;
use voxel_work::{Scheduler, TickDriver, UnitOfWork};

fn main() {
    // One chunk with a little matter in it, so the octree has units at level 4.
    let chunk = ChunkPos::new(0, 0, 0);
    let mut tree = SparseVoxelOctree::new();
    for (x, y, z) in [(0, 0, 0), (7, 9, 11), (16, 16, 16), (31, 31, 31)] {
        tree.set_voxel(
            LocalPos::new(x, y, z).expect("the corner is in the chunk"),
            VoxelShape::solid(MaterialId(3)),
        );
    }

    // Level 4 is the 8-region rung: a chunk offers eight 16³ units there. Stated
    // rather than assumed, so a changed root size has to be argued for.
    let units: Vec<UnitOfWork> =
        Scheduler::units_at(&tree, chunk, 4).expect("level 4 is in the octree");
    println!("unit of work: {} regions of 16³ voxels (halo {} voxel)", units.len(), voxel_work::HALO_VOXELS);

    let mut driver = TickDriver::full().expect("the default pipeline validates");
    let mut second = TickDriver::full().expect("the default pipeline validates");
    println!("tick  due stages                                                     work  digest");
    for tick in 0..=24u64 {
        let receipt = driver.step(tick, &units).expect("the tick advances");
        let again = second.step(tick, &units).expect("the tick advances");
        assert_eq!(receipt, again, "tick {tick} differed between two drivers");
        let names = receipt
            .due
            .iter()
            .map(|s| s.name())
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{tick:>4}  {:<64} {:>4}  {}",
            names,
            receipt.work.len(),
            short(receipt.digest.digest().as_bytes())
        );
    }

    // The two things the driver refuses, both measured rather than described.
    let refused = driver.step(23, &units).expect_err("a rewind is refused");
    println!("\na rewind: {}", refused.detail());
    let same = driver.step(24, &units).expect("the same tick is idempotent");
    println!("the same tick twice: {} work items, cursor still {:?}", same.work.len(), driver.last_tick());

    // And the day's stage is reached on the day and not a tick early.
    let day = voxel_shapers::climate::TICKS_PER_DAY;
    let clim = driver.scheduler().due_at(day - 1).contains(&Stage::Climate);
    println!(
        "climate due on tick {}: {clim}; on the day ({day}): {}",
        day - 1,
        driver.scheduler().due_at(day).contains(&Stage::Climate)
    );
}

fn short(bytes: &[u8]) -> String {
    bytes.iter().take(4).map(|b| format!("{b:02x}")).collect()
}
