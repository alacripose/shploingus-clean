//! Trees, counted and cut open — `cargo run -p voxel-shapers --example flora-census`
//!
//! The counts come from the flora lattice itself; the slice is read through
//! `chunk_fields::material_at`, which is **the path the client streams**, so what is
//! printed is what will stand in the world. `W` is trunk wood, `C` conifer needles, `L`
//! broadleaf, `#` the ground the tree is rooted in, `.` air.

use voxel_shapers::chunk_fields;
use voxel_shapers::column::materials;
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::{flora, terrain};

fn main() {
    let spec = WorldSpec::new(2468, Config::default());

    // The spawn window's stand, counted by cells.
    let cells = 512 / flora::TREE_CELL_M;
    let (mut conifer, mut broadleaf) = (0u32, 0u32);
    for cell_x in -cells..=cells {
        for cell_z in -cells..=cells {
            if let Some(tree) = flora::memoized_tree_of_cell(&spec, cell_x, cell_z) {
                if tree.conifer {
                    conifer += 1;
                } else {
                    broadleaf += 1;
                }
            }
        }
    }
    println!(
        "within ±512 m: {conifer} conifer, {broadleaf} broadleaf ({} m cells)",
        flora::TREE_CELL_M
    );

    // The first conifer the lattice grows near the origin, and a slice through it.
    let mut found = None;
    'scan: for radius in 1..cells {
        for cell_z in -radius..=radius {
            for cell_x in [-radius, radius] {
                if let Some(tree) = flora::memoized_tree_of_cell(&spec, cell_x, cell_z) {
                    if tree.conifer {
                        found = Some(tree);
                        break 'scan;
                    }
                }
            }
        }
    }
    let tree = found.expect("no conifer within 512 m of the spawn");
    println!(
        "a conifer at column ({}, {}) on ground {} m: trunk {} voxels",
        tree.base_x, tree.base_z, tree.base_y, tree.trunk_height
    );

    let x = tree.base_x;
    let zs = tree.base_z - 10..=tree.base_z + 10;
    let top = tree.base_y + tree.trunk_height + 1;
    println!("slice through it at x = {x} (z runs left to right, one column each):");
    for y in (tree.base_y - 1..=top).rev() {
        let mut row = String::new();
        for z in zs.clone() {
            let material = chunk_fields::material_at(&spec, x, y, z).unwrap_or(materials::AIR);
            row.push(match material {
                materials::WOOD => 'W',
                materials::CONIFER => 'C',
                materials::LEAF => 'L',
                materials::AIR => '.',
                _ => '#',
            });
        }
        println!("  y {y:>4} {row}");
    }

    // And the whole ±6 km world, by cell, so the stand's scale is visible.
    let (mut world_conifer, mut world_broadleaf) = (0u32, 0u32);
    for cell_x in -750..750 {
        for cell_z in -750..750 {
            if let Some(tree) = flora::memoized_tree_of_cell(&spec, cell_x, cell_z) {
                if tree.conifer {
                    world_conifer += 1;
                } else {
                    world_broadleaf += 1;
                }
            }
        }
    }
    println!("whole ±6 km world: {world_conifer} conifer, {world_broadleaf} broadleaf");
    let _ = terrain::elevation_m(&spec, 0.5, 0.5);
}
