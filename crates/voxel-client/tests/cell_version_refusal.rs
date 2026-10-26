//! **The cell-version refusal, through the real path** — a package's own declaration, the mount the
//! game calls, the shell's own frame, and the two drawings.
//!
//! # What this test is for
//!
//! The user asked for the refusal to be observable the way this corpus observes refusals: triggered
//! by a package, refused by name with the file and line that declared it, carried as a diagnostic,
//! and — the point of it — the shell keeps drawing the version it had. And for the other half: the
//! old drawing is brought back by a declared version, not by editing the module.
//!
//! **The declaration is read where the package is mounted**, so the second test mounts a display
//! package that arrives *after* boot through the same loader the running game uses: its `cells`
//! line selects the drawing, and a package written under the old declaration header is refused
//! before anything is adopted — the arrangement and the drawing already on screen stay exactly as
//! they were. That is the case the boot-time read could not answer.
//!
//! So nothing here stubs the refusal enum. The declaration is read from real files, the package is
//! compiled and mounted by the same functions the game calls, and the elements are composed by
//! `shell::hud::frame` — the frame the app draws — before `cells::raster` is asked for a version.

use std::path::{Path, PathBuf};

use voxel_client::hud::HudState;
use voxel_client::shell::{self, cells, hud};
use voxel_model::Digest;
use voxel_ui::{Element, Palette, Rect};

/// The package the game embeds, from the same source `build.rs` compiles.
const SHELL: &str = include_str!("../../../packages/alpha-shell.shspec");

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate sits two levels under the workspace")
        .to_path_buf()
}

/// **The shell as the game opens it**: the embedded package, then the display package's folder
/// mounted through the loader — which is what reads `package.cells`. Not a separate resolver: the
/// declaration belongs to mounting, and this is the mounting the game does.
fn mounted_shell() -> shell::Mount {
    let package = voxel_package::recipe::compile(SHELL.as_bytes()).expect("the shell compiles");
    let loaded = voxel_package::recipe::load(&package).expect("the shell loads");
    let mut mount = shell::Mount::new(&loaded).expect("the shell mounts");
    mount.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
    mount
        .load_package(&workspace().join("display-packages/alpha-shell/package.shspec"))
        .expect("the shell's display package mounts, and its declaration is read there");
    // **The map window is opened, because a frame draws only in an open window's content.**
    // The boot arrangement opens the pinned HUD windows and leaves the tabs closed; this is the
    // test asking for the window whose content is a frame, not a different arrangement.
    mount.open("map-plan").expect("the map window opens");
    mount
}

/// The frame the app draws for this mount, through `shell::hud::frame`.
fn composed(mount: &mut shell::Mount) -> Vec<Element> {
    let frame = voxel_work::Frame {
        wanted: 605,
        resident: 291,
        bytes: 71 * 1024 * 1024,
        ceiling: 256 * 1024 * 1024,
        scratch_bytes: 72 * 1024 * 1024,
        in_flight: 324,
        ..Default::default()
    };
    let state = HudState {
        frame: &frame,
        adapter: "test adapter",
        workers: 15,
        fps: Some(60.0),
        position_m: [0.0, 45.0, -160.0],
        grounded: true,
        vertical_m_s: 0.0,
        in_water: false,
        quads: 43_794,
        seed: 2468,
        tick: 9_259,
        digest: Digest::of(b"the resident world"),
        refusal: None,
        shading: voxel_client::shading::ShadingReport { materials: 7, with_base_colour: 7 },
    };
    let views = hud::views(&state, mount.console(), mount.api_view().cloned());
    let viewport = mount.viewport();
    hud::frame(mount, &Palette::declared(), viewport, &views).expect("the shell's frame composes")
}

/// **The frame's one raster act at the version the mount holds** — the call the app's draw makes.
fn drawn(mount: &shell::Mount, elements: &[Element]) -> Vec<Element> {
    cells::raster(elements, &mount.frame_space(), mount.cell_version().number())
        .expect("the version in force is one this shell draws")
}

/// A directory a package is copied into **after boot** — the arrival is a real folder on disk, not
/// the fixture path the test already knows.
fn staging(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shploingus-cells-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the arrival directory");
    dir
}

/// Copy one fixture display package into `into`: its recipe and its declaration, together, because
/// a display package is the folder and the declaration is part of it.
fn arrive(fixture: &str, into: &Path) {
    let source = workspace().join("crates/voxel-client/tests/fixtures").join(fixture);
    for file in ["package.shspec", "package.cells"] {
        std::fs::copy(source.join(file), into.join(file)).expect("the package arrives");
    }
}

#[test]
fn an_old_declaration_is_refused_by_name_and_the_version_in_force_keeps_drawing() {
    // 1. **The shell's own package declares the drawing in force**, and the declaration is a real
    //    file with a real line — not a constant in this test.
    let shell_folder = workspace().join("display-packages/alpha-shell");
    let declaration = cells::declare(&shell_folder)
        .expect("the shell's declaration is one this build reads")
        .expect("the shell's package carries a declaration");
    assert_eq!(declaration.version, cells::CellVersion::Holo);
    assert_eq!(declaration.line, 2);
    assert!(declaration.file.ends_with("alpha-shell/package.cells"));

    // 2. **The old declaration is refused with its site.** The fixture is a package folder on disk,
    //    written under the pre-current header.
    let old_declaration =
        workspace().join("crates/voxel-client/tests/fixtures/old-cell-declaration");
    let refusal = cells::declare(&old_declaration).expect_err("the old header is not read");
    let message = refusal.to_string();
    assert!(
        message.contains("old-cell-declaration/package.cells:1"),
        "the refusal does not name its file and line: {message}"
    );
    assert!(
        message.contains("shploingus.cells/0"),
        "the refusal does not name the declared version: {message}"
    );
    assert!(
        message.contains("version 0"),
        "the refusal does not say how to bring the old drawing back: {message}"
    );

    // 3. **Mounting that package is refused, and the refusal is the declaration's.** The package
    //    frames — it declares no window, so the console-window rule would also turn it away — and
    //    the message is the declaration's own sentence, because the declaration is read before
    //    anything is adopted. That ordering is what makes a bad declaration cost no screen.
    let mut mount = mounted_shell();
    assert_eq!(
        mount.cell_version(),
        cells::CellVersion::Holo,
        "mounting the shell's display package did not select its declared drawing"
    );
    let why = mount
        .load_package(&old_declaration.join("package.shspec"))
        .expect_err("a package whose declaration is not read is not mounted");
    assert_eq!(why, message, "the mount's refusal is not the declaration's sentence");
    assert_eq!(
        mount.cell_version(),
        cells::CellVersion::Holo,
        "the refused package changed the drawing in force"
    );

    // 4. **It draws** — at the version the declaration selected, and that drawing is the raster,
    //    not the composed elements. This is the half the refusal is for: the screen keeps the shell
    //    it had.
    // **The frame reached the mount and is placed in its window's content space.**
    assert!(
        mount.frame_of("map-plan").is_some(),
        "the mounted package's declared frame was not graded into the mount"
    );
    let spaces = mount.frame_space();
    assert_eq!(spaces.len(), 1, "one declared frame, one placed frame");
    assert_eq!(spaces[0].frame.id, "MAP-0001");
    assert_eq!(
        spaces[0].footprint().width_dp,
        f64::from(spaces[0].frame.cols) * cells::CELL_W_DP,
        "a frame's footprint is its own grid"
    );

    let elements = composed(&mut mount);
    assert!(!elements.is_empty(), "the frame composed no elements at all");
    let drawn = drawn(&mount, &elements);
    assert_eq!(
        drawn,
        cells::frames_in(&elements, &mount.frame_space()),
        "the version in force is not the raster"
    );
    assert_ne!(drawn, elements, "the raster handed the composed elements back");

    // 5. **And the old drawing is a declaration away**: the same frame, version 0, unchanged.
    let old_folder = workspace().join("crates/voxel-client/tests/fixtures/old-drawing");
    let old = cells::declare(&old_folder)
        .expect("the current declaration grammar is this build's")
        .expect("the fixture declares a drawing");
    assert_eq!(old.version, cells::CellVersion::Elements);
    let old_drawn = cells::raster(&elements, &mount.frame_space(), old.version.number())
        .expect("version 0 is one this shell draws");
    assert_eq!(
        old_drawn, elements,
        "the old drawing did not return the elements as composed"
    );
}

/// **The package arrives after boot, and the screen answers it.**
///
/// The boot mount declares Holo. A display package is then copied into a fresh folder and mounted
/// through the app's own loader (`load_package`, the `file` verb's and the watch's entry): its
/// `cells 0` line must bring the old drawing back *and* its arrangement must take the shell. Then a
/// second package arrives whose `package.cells` is written under the old header. It must be refused
/// by file and line, and **nothing on screen may move** — not the arrangement, not the version, and
/// therefore not one raster run of the frame: the drawing already on screen keeps drawing.
#[test]
fn a_display_package_that_arrives_after_boot_selects_its_drawing_and_a_refused_one_changes_nothing() {
    let mut mount = mounted_shell();
    assert_eq!(mount.cell_version(), cells::CellVersion::Holo, "the boot package declares Holo");
    let boot = composed(&mut mount);
    let boot_drawing = drawn(&mount, &boot);
    assert_eq!(boot_drawing, cells::frames_in(&boot, &mount.frame_space()));
    assert_ne!(boot_drawing, boot, "the boot drawing is the composed elements");

    // ---- the package arrives, after boot ----------------------------------
    let arriving = staging("arriving");
    arrive("arriving-old-drawing", &arriving);
    let loaded = mount
        .load_package(&arriving.join("package.shspec"))
        .expect("the arriving display package mounts");
    assert_eq!(
        loaded.recipe.name, "arriving-old-drawing",
        "the mounted package is not the one that arrived"
    );
    assert_eq!(
        mount.cell_version(),
        cells::CellVersion::Elements,
        "the arriving package's `cells 0` did not select the old drawing"
    );
    let windows: Vec<&str> = mount.windows().map(|window| window.spec.id.as_str()).collect();
    assert_eq!(
        windows,
        vec!["console"],
        "the arriving package's arrangement did not take the shell, so its drawing was not mounted"
    );

    let arrived = composed(&mut mount);
    assert!(!arrived.is_empty(), "the arriving arrangement composed no elements");
    let arrived_drawing = drawn(&mount, &arrived);
    assert_eq!(
        arrived_drawing, arrived,
        "the arriving package's version 0 did not draw the elements as composed"
    );
    assert_ne!(
        arrived_drawing, boot_drawing,
        "the arriving declaration did not change the drawing at all"
    );

    // ---- a package whose declaration this build cannot read ---------------
    let refused = staging("refused");
    arrive("arriving-old-declaration", &refused);
    let why = mount
        .load_package(&refused.join("package.shspec"))
        .expect_err("a package written under the old declaration header is refused");
    assert!(
        why.contains("refused/package.cells:1"),
        "the refusal does not name the arriving declaration's file and line: {why}"
    );
    assert!(
        why.contains("shploingus.cells/0"),
        "the refusal does not name the declaration the package wrote: {why}"
    );
    assert!(
        why.contains("version 0"),
        "the refusal does not say which declaration brings the old drawing back: {why}"
    );

    // **What happens to what is already on screen**: nothing. The declaration was read before the
    // arrangement was adopted, so the version in force, the windows, the composed frame and the
    // raster runs are the ones the previous package left — a refused arrival costs a sentence, not
    // a screen.
    assert_eq!(
        mount.cell_version(),
        cells::CellVersion::Elements,
        "the refused package took the drawing in force"
    );
    let still: Vec<&str> = mount.windows().map(|window| window.spec.id.as_str()).collect();
    assert_eq!(still, vec!["console"], "the refused package took the arrangement");
    let after = composed(&mut mount);
    assert_eq!(after, arrived, "the refused package changed the frame on screen");
    assert_eq!(
        drawn(&mount, &after),
        arrived_drawing,
        "the refused package changed the drawing on screen"
    );

    let _ = std::fs::remove_dir_all(&arriving);
    let _ = std::fs::remove_dir_all(&refused);
}
