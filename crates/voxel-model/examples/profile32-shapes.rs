//! **What a 1/32 profile looks like**, printed as a slice, with no window and no GPU.
//!
//! This is the witness for `voxel_model::profile32` before the renderer reads it: the
//! same record sampled on the 32×32 grid of one block's vertical slice, at two stages
//! and for the two organ kinds a tree is made of. `#` is a cell more than half inside
//! the body, `+` is a cell the surface crosses, `.` is air.
//!
//! Run with `cargo run -p voxel-model --example profile32-shapes`.

use voxel_model::profile32::{Shaped, PROFILE_SIDE, STAGE_ONE};
use voxel_model::profile::Profile;
use voxel_model::profile32::Axis;

fn slice(body: &Shaped, z: u32) -> Vec<String> {
    (0..PROFILE_SIDE)
        .rev()
        .map(|v| {
            (0..PROFILE_SIDE)
                .map(|u| match body.cell(u, v, z) {
                    c if c > 0.5 => '#',
                    c if c > 0.0 => '+',
                    _ => '.',
                })
                .collect()
        })
        .collect()
}

fn print_slice(title: &str, body: &Shaped) {
    println!("\n{title}");
    for row in slice(body, PROFILE_SIDE / 2) {
        println!("  {row}");
    }
}

fn main() {
    let trunk = Shaped::of(Profile::Cylinder { radius: 0.28 }, Axis::Y);
    print_slice("a trunk at a full stage (56 cm across, 1/32 cells)", &trunk);
    print_slice("the same trunk at stage 1/2 — the same form, half the size", &trunk.grown(STAGE_ONE / 2));

    let mut crown = Shaped::of(Profile::Spheroid { radius: 0.45, height: 0.45 }, Axis::Y);
    crown.void = 0.9;
    crown.ridge = 0.2;
    crown.seed = 0x5EED;
    print_slice("a crown envelope: 90 % of it is air, and the same air every time", &crown);
}
