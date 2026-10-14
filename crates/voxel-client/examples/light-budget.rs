//! How many distinct light cells one frame asks for, at subvoxel grain versus
//! voxel grain — the number the frame's 1 024-cell measurement budget is spent
//! against.
//!
//! ```text
//! cargo run --offline -p voxel-client --example light-budget
//! ```
//!
//! Builds a real world, meshes real chunks, and builds a vertex-format scene
//! with a recording light closure (answering `None`, so every query is
//! recorded and nothing is measured). Prints total corner queries, unique keys
//! at the granularity `Scene` feeds the field (subvoxel cells), and unique
//! keys at the granularity the field's key was designed for (voxels).

use std::cell::RefCell;

use voxel_appearance::DetailLattice;
use voxel_client::Scene;
use voxel_mesh::{Chunk, MeshConfig, Mesher};
use voxel_model::SUBVOXEL_SIDE;
use voxel_stream::WorldSource;
use voxel_shapers::{Config, WorldSpec};

fn main() {
    let source = WorldSource::new(WorldSpec::new(2468, Config::default()));
    let registry = voxel_shapers::materials::registry().expect("the registry");
    let mesh_config = MeshConfig::from_registry(&registry).expect("a mesh config");
    let mut mesher = Mesher::new();
    let mut meshes = Vec::new();
    for cy in 0..2 {
        for cx in 0..3 {
            for cz in 0..3 {
                let pos = voxel_model::ChunkPos::new(cx, cy, cz);
                let Ok(tree) = source.build_chunk(pos) else {
                    continue;
                };
                let chunk = Chunk::from_tree(pos, tree);
                let neighborhood = source.neighborhood(pos);
                if let Ok(mesh) = mesher.build(&chunk, &mesh_config, Some(&neighborhood)) {
                    meshes.push(mesh);
                }
            }
        }
    }
    assert!(!meshes.is_empty(), "the world produced no chunks to mesh");

    // Eye above the terrain's top, looking across it: place from a quick build.
    let lattice = DetailLattice::new(0);
    let no_light = |_: i32, _: i32, _: i32| None;
    let placed = Scene::build_with_light(meshes.iter(), [0.0; 3], &lattice, &no_light, None, None);
    let hi = placed
        .vertices
        .iter()
        .map(|v| v.position[1] as f64)
        .fold(f64::NEG_INFINITY, f64::max);
    let eye = [16.0, hi + 2.0, -24.0];

    let seen = RefCell::new(Vec::<[i32; 3]>::new());
    let record = |x: i32, y: i32, z: i32| {
        seen.borrow_mut().push([x, y, z]);
        None
    };
    let scene = Scene::build_with_light(meshes.iter(), eye, &lattice, &record, None, None);
    let seen = seen.borrow();
    let mut subvoxel_keys = seen.clone();
    subvoxel_keys.sort_unstable();
    subvoxel_keys.dedup();
    let side = SUBVOXEL_SIDE as i32;
    let mut voxel_keys: Vec<[i32; 3]> = seen
        .iter()
        .map(|c| {
            [
                c[0].div_euclid(side),
                c[1].div_euclid(side),
                c[2].div_euclid(side),
            ]
        })
        .collect();
    voxel_keys.sort_unstable();
    voxel_keys.dedup();
    println!(
        "light-budget: {} quads, {} vertices, {} corner queries",
        scene.quads,
        scene.vertices.len(),
        seen.len()
    );
    println!(
        "unique subvoxel keys: {} | unique voxel keys: {} | budget: {}",
        subvoxel_keys.len(),
        voxel_keys.len(),
        voxel_client::field::CELLS_PER_FRAME
    );
    println!(
        "frames to cover at budget: {:.1} (subvoxel) vs {:.1} (voxel)",
        subvoxel_keys.len() as f64 / voxel_client::field::CELLS_PER_FRAME as f64,
        voxel_keys.len() as f64 / voxel_client::field::CELLS_PER_FRAME as f64
    );
}
