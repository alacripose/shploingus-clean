//! **Print the ratchet's reading of this tree.**
//!
//! `cargo run -p voxel-axis-verify --example ratchet -- <head>` prints one line per act and per
//! baseline, then the totals. With no argument it uses the commit the baselines were measured at,
//! so a tree that has not moved reads as current.

fn main() {
    let root = voxel_package::features::workspace_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("this example sits inside the workspace");
    let head = std::env::args().nth(1).unwrap_or_else(|| "6049414".to_string());
    let reading = voxel_axis_verify::ratchet(&root, &head);
    for line in reading.lines() {
        println!("{line}");
    }
    println!(
        "{} cpu act(s), {} rendering act(s)",
        reading.of(voxel_axis_matrix::Task::Cpu).count(),
        reading.of(voxel_axis_matrix::Task::Rendering).count()
    );
    if !reading.verified() {
        std::process::exit(1);
    }
}
