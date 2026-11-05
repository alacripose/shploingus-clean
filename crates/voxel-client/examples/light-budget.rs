//! **How many distinct cells one scene asks for** — the number the measurer's
//! ask size is spent against.
//!
//! ```text
//! cargo run --offline -p voxel-client --example light-budget
//! ```
//!
//! Builds a real world, meshes real chunks, and builds a vertex-format scene with a
//! recording light closure (answering `None`, so every query is recorded and nothing
//! is measured). Prints total corner queries, the distinct cells behind them, and how
//! many asks of [`voxel_client::CELLS_PER_ASK`] it takes to cover a window this size.
//!
//! **There is one grain.** This example used to print two — the subvoxel addresses the
//! scene asked for and the voxel cells the field keyed on — and the distance between
//! them was the defect: four times the address, a chunk four times out, no world under
//! it. The scene asks with `VoxelPos::of_point` now, so the two numbers are one number.

use std::cell::RefCell;

use voxel_client::Scene;
use voxel_mesh::{Chunk, MeshConfig, Mesher};
use voxel_shapers::{Config, WorldSpec};
use voxel_stream::WorldSource;

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
    let no_light = |_: [f64; 3]| None;
    let placed = Scene::build_with_light(meshes.iter(), [0.0; 3], &no_light, None, None);
    let hi = placed
        .vertices
        .iter()
        .map(|v| v.position[1] as f64)
        .fold(f64::NEG_INFINITY, f64::max);
    let eye = [16.0, hi + 2.0, -24.0];

    let seen = RefCell::new(Vec::<([i32; 3], u8)>::new());
    let record = |point: [f64; 3]| {
        if let Some(cell) = voxel_client::field::SubvoxelPos::of_point(point) {
            seen.borrow_mut().push((
                [cell.voxel.x, cell.voxel.y, cell.voxel.z],
                cell.at.flat() as u8,
            ));
        }
        None
    };
    let scene = Scene::build_with_light(meshes.iter(), eye, &record, None, None);
    let seen = seen.borrow();
    let mut cells = seen.clone();
    cells.sort_unstable();
    cells.dedup();
    println!(
        "light-budget: {} quads, {} vertices, {} corner queries",
        scene.quads,
        scene.vertices.len(),
        seen.len()
    );
    println!(
        "distinct cells asked for: {} | ask size: {} | asks to cover this window: {:.1}",
        cells.len(),
        voxel_client::CELLS_PER_ASK,
        cells.len() as f64 / voxel_client::CELLS_PER_ASK as f64
    );
    // **Every address is a cell of its own corner** — the law, checked against the geometry rather
    // than against itself: the scene's own vertex list is what the addresses were asked for, and a
    // grain that disagreed (four times the address, as the draft did) would leave corners whose own
    // voxel was never asked about.
    let mut missing = 0usize;
    for v in &scene.vertices {
        let origin = &scene.origins[v.chunk as usize];
        let world = [
            v.position[0] as f64 + origin[0] as f64,
            v.position[1] as f64 + origin[1] as f64,
            v.position[2] as f64 + origin[2] as f64,
        ];
        let at = voxel_client::field::SubvoxelPos::of_point(world).expect("a finite corner");
        let asked = ([at.voxel.x, at.voxel.y, at.voxel.z], at.at.flat() as u8);
        if cells.binary_search(&asked).is_err() {
            missing += 1;
        }
    }
    println!("corner(s) whose own cell was never asked for: {missing}");
    assert_eq!(
        missing, 0,
        "the scene and the field are on two different grains"
    );
}
