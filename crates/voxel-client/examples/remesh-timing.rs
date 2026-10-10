//! **Where the mesh step's time goes**: chunk generation against the mesher, over the same chunks.
//!
//! Run from the workspace root:
//!
//! ```text
//! cargo run --offline --release -p voxel-client --example remesh-timing
//! ```
//!
//! `mesher.build` does the raster, the face rule and the greedy merge together, and the merge
//! is the only part a GPU remesh would replace. This example times the generation
//! (`source.build_chunk`, the octree) and the mesher separately so the merge's share is a
//! measured number rather than a guess. The mesher's internal split is not timed here, because
//! that would need the crate to expose its phases.

use std::time::Instant;

use voxel_mesh::{Chunk, MeshConfig, Mesher};
use voxel_shapers::{Config, WorldSpec};
use voxel_stream::WorldSource;

/// The same window `cloud-picture` meshes, so the two numbers describe one world.
const CHUNKS_SIDE: i32 = 6;
const CHUNKS_UP: i32 = 3;

fn main() {
    let spec = WorldSpec::new(voxel_client::GAME_SEED, Config::default());
    let source = WorldSource::new(spec);
    let registry = voxel_shapers::column::materials::registry().expect("the registry");
    let config = MeshConfig::from_registry(&registry).expect("a mesh config");

    let mut mesher = Mesher::new();
    let mut generation = 0.0_f64;
    let mut meshing = 0.0_f64;
    let mut chunks = 0usize;
    let mut quads = 0usize;
    let mut loops = 0usize;
    let mut corners = 0usize;

    // Generate every chunk first, timed on its own, then mesh them, timed on its own. The two
    // loops touch the same chunks in the same order, so neither timing includes the other.
    let mut built = Vec::new();
    for cy in 0..CHUNKS_UP {
        for cx in 0..CHUNKS_SIDE {
            for cz in 0..CHUNKS_SIDE {
                let pos = voxel_model::ChunkPos::new(cx, cy, cz);
                let started = Instant::now();
                let Ok(tree) = source.build_chunk(pos) else {
                    continue;
                };
                generation += started.elapsed().as_secs_f64() * 1000.0;
                built.push((pos, tree));
            }
        }
    }
    for (pos, tree) in built {
        let chunk = Chunk::from_tree(pos, tree);
        let neighborhood = source.neighborhood(pos);
        let started = Instant::now();
        if let Ok(mesh) = mesher.build(&chunk, &config, Some(&neighborhood)) {
            meshing += started.elapsed().as_secs_f64() * 1000.0;
            quads += mesh.quads.len();
            loops += mesh.loops.len();
            corners += mesh.loops.iter().map(|l| l.corners.len()).sum::<usize>();
            chunks += 1;
        }
    }

    println!("chunks: {chunks}, quads: {quads}, loops: {loops}, corners: {corners}");
    if quads > 0 {
        println!(
            "one loop per {:.2} quads, {:.2} corners a loop",
            quads as f64 / loops as f64,
            corners as f64 / loops as f64
        );
    }
    println!("generation: {generation:.1} ms");
    println!("mesher.build (raster + face rule + greedy merge): {meshing:.1} ms");
    println!(
        "merge's share is at most the mesher's own time: {:.0} % of the mesh step is meshing, \
         {:.0} % is generation",
        100.0 * meshing / (meshing + generation),
        100.0 * generation / (meshing + generation),
    );
}
