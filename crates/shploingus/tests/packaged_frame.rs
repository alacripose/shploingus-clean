//! **The frame the build embeds is graded from the package alone.**
//!
//! `build.rs` compiles `display-packages/alpha-shell` into `alpha-shell.shpkg` and this test
//! reads those bytes, not the folder: the frame is an artifact in `SHPKG003`'s table, and
//! [`Mount::new`] grades it from `Loaded::artifacts`. If the frame stopped being packaged, the
//! mount would carry none and this test would fail even with `display-packages/` still on disk.

use voxel_client::shell::Mount;

const SHELL_PACKAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/alpha-shell.shpkg"));

#[test]
fn the_embedded_shell_package_carries_and_grades_its_cell_frame() {
    let loaded = voxel_package::recipe::load(SHELL_PACKAGE).expect("the embedded package loads");
    assert!(
        loaded
            .artifacts
            .contains_key("frames/MAP-0001.frame"),
        "the package carries the frame, not only the recipe: {:?}",
        loaded.artifacts.keys().collect::<Vec<_>>()
    );
    let mount = Mount::new(&loaded).expect("the embedded package grades its frame and mounts");
    assert_eq!(
        mount.frame_of("map-plan").expect("the map window's frame").id,
        "MAP-0001"
    );
}
