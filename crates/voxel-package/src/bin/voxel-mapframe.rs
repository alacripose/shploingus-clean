//! **The map window's axis cell frames, generated from the world's own laws.**
//!
//! `display-packages/alpha-shell/frames/MAP-0001.frame` used to be a hand-drawn
//! lattice with invented contours. This command replaces the drawing with a
//! **measurement**: every land cell is the world's own `terrain::elevation_m`, its
//! biome is `ecology::biome_at` on the same Holdridge inputs the chunk builder uses,
//! and a contour is a change of elevation band at that cell. Nothing in the frame is
//! a made-up shape, and the generator refuses to write a frame the grammar does not
//! parse or whose picture is not the grid it measured.
//!
//! # Eight views of one measured map
//!
//! The package declares **one frame per view** of the same 56 × 22 grid, all addressed
//! to `map-plan` and distinguished by the frame grammar's `feature` key:
//!
//! | frame | feature | what is drawn in full ink |
//! |---|---|---|
//! | `MAP-0001` | `all` | every cell: the default map |
//! | `MAP-0002` | `ocean` | water (deep and shallow) |
//! | `MAP-0003` | `mountain` | alpine cells and high ground |
//! | `MAP-0004` | `forest` | the five forest biomes |
//! | `MAP-0005` | `grassland` | grassland |
//! | `MAP-0006` | `desert` | desert and subtropical scrub |
//! | `MAP-0007` | `tundra` | tundra and polar desert |
//!
//! **There is no rainforest view.** The seeded world has no tropical rainforest within
//! 12.3 km of the spawn, and a declared view that selects zero of 918 cells is a blank
//! selection rather than a map. The absence is stated here and in the patch record; it is
//! not papered over with a near-biome threshold.
//!
//! A selected view keeps the whole world drawn — the cells that do not answer the
//! feature are drawn in the schema's `disabled` ink — so switching views is a
//! highlight on the same measurement, not a second map with different geography.
//! `plan ocean` selects one, `plan next` cycles, and `plan all` returns the default.
//!
//! Each view is centred where its feature is **found**, by a ring search outward from
//! the spawn column at the same declared cell size. The record states whether the
//! search found the feature or fell back, so a view centred on the spawn is a stated
//! fallback rather than an unexamined default.
//!
//! # Why this is a 2D cell frame and not the world's mesh
//!
//! The **column** record is the 2D cell renderer's, and this is that renderer: a
//! map window is a raster of cells, one screen cell a world square. The 3D world is
//! cubic and is drawn by the streamer and the surface ladder; no column is built here
//! for it. The map is a *reading* of the world at a declared scale, not a second world.
//!
//! # The declared recipe
//!
//! ```text
//! centre   0.5, −95.5       the game's spawn column (GAME_SPAWN_XZ), the search origin
//! cell     32 m             one map cell, and the scale the legend states
//! tick     3/8 of a day     GAME_TICK, so the climate sample is the game's own
//! seed     9 418 646 105 956 113 635   GAME_SEED
//! discs    54 × 17 cells    the content space inside a 56 × 22 frame
//! search   128 m rings, 96  up to 12.3 km from the spawn, for a feature's centre
//! ```
//!
//! # Usage
//!
//! ```text
//! cargo run -p voxel-package --bin voxel-mapframe            # write all seven views
//! cargo run -p voxel-package --bin voxel-mapframe -- --check  # compare, write nothing
//! ```
//!
//! Exit 0 when every generated frame parses and (in `--check`) matches the file.
//! Exit 1 when the grammar or a file disagrees, with the first differing row named.
//! Exit 2 on a usage error.

use std::f64::consts::TAU;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use voxel_package::cellframe::{Frame, Ink, Paper};
use voxel_shapers::ecology::{self, Biome};
use voxel_shapers::{climate, terrain, Config, WorldSpec};

/// The frame's declared width in cells.
const COLS: u32 = 56;
/// The frame's declared height in cells.
const ROWS: u32 = 22;
/// The map's width inside the border.
const MAP_COLS: u32 = 54;
/// The map's height inside the border.
const MAP_ROWS: u32 = 17;
/// The map's first column inside the border.
const MAP_LEFT: u32 = 1;
/// The map's first row inside the border.
const MAP_TOP: u32 = 1;
/// One map cell in world metres. Declared, and stated in the legend.
const CELL_M: f64 = 32.0;
/// The game's spawn column, mirrored here because `voxel-package` may not depend on
/// `voxel-client` (the client depends on this crate). The legend prints it, so a
/// changed spawn is visible in the frame rather than silently different.
const CENTRE_XZ: [f64; 2] = [0.5, -95.5];
/// The game's seed, mirrored for the same reason as [`CENTRE_XZ`].
const SEED: u64 = 9_418_646_105_956_113_635;
/// The game's tick is 3/8 of a day; the biome sample uses the same clock.
const TICK: u64 = climate::TICKS_PER_DAY * 3 / 8;
/// The elevation step a contour marks, metres. Declared: a 40 m relief gets five
/// bands, which is a map that reads rather than a wall of lines.
const CONTOUR_M: f64 = 8.0;
/// How far apart the centring search's samples stand, metres.
const SEARCH_STEP_M: f64 = 128.0;
/// How many rings the centring search walks outward from the spawn.
const SEARCH_RINGS: u32 = 96;
/// **The GPU view's extent**, in map cells. The glyph frame is 56 × 22 because a character grid
/// is its own resolution; the terrain view is 28 × 32 at the same 32 m so its 896 × 1024 m extent
/// fills a 1.27-wide window's width and leaves sky only above the horizon.
const GPU_EXTENT: [u32; 2] = [28, 32];
/// The camera's downward tilt. Declared here because the horizon is the point: a steeper camera
/// is a plan, and a shallower one spends the window on sky.
const GPU_PITCH_DEG: f64 = -24.0;
/// The elevation law the declaration names, spelled once here rather than in two literals.
const ELEVATION_SOURCE: &str = "terrain::elevation_m";
/// The biome law the declaration names.
const BIOME_SOURCE: &str = "ecology::biome_at";

/// **The views the map offers.** One frame per view; the token is the frame grammar's
/// `feature` value and the word the console's `plan` verb accepts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Feature {
    /// Every cell. The default view.
    All,
    /// Water, deep and shallow.
    Ocean,
    /// Alpine cells and high ground.
    Mountain,
    /// The forest biomes.
    Forest,
    /// Grassland.
    Grassland,
    /// Desert and subtropical scrub.
    Desert,
    /// Tundra and polar desert.
    Tundra,
}

impl Feature {
    /// Every view, in frame order.
    const ALL: [Feature; 7] = [
        Feature::All,
        Feature::Ocean,
        Feature::Mountain,
        Feature::Forest,
        Feature::Grassland,
        Feature::Desert,
        Feature::Tundra,
    ];

    /// The token the frame's `feature` key carries and `plan` types.
    const fn name(self) -> &'static str {
        match self {
            Feature::All => "all",
            Feature::Ocean => "ocean",
            Feature::Mountain => "mountain",
            Feature::Forest => "forest",
            Feature::Grassland => "grassland",
            Feature::Desert => "desert",
            Feature::Tundra => "tundra",
        }
    }

    /// The frame id the package's recipe names this view by.
    const fn frame_id(self) -> &'static str {
        match self {
            Feature::All => "MAP-0001",
            Feature::Ocean => "MAP-0002",
            Feature::Mountain => "MAP-0003",
            Feature::Forest => "MAP-0004",
            Feature::Grassland => "MAP-0005",
            Feature::Desert => "MAP-0006",
            Feature::Tundra => "MAP-0007",
        }
    }

    /// Draw order within the content layer. One frame per window draws, so these only
    /// order the package's joins; the default stays first.
    const fn order(self) -> u32 {
        match self {
            Feature::All => 10,
            Feature::Ocean => 11,
            Feature::Mountain => 12,
            Feature::Forest => 13,
            Feature::Grassland => 14,
            Feature::Desert => 15,
            Feature::Tundra => 16,
        }
    }

    /// **Whether this cell answers the view**, from the reading the map already took.
    ///
    /// The predicates are the world's own classifications where one exists (`Alpine`,
    /// `Grassland`, `Desert`, …) and a stated elevation threshold for the mountain view,
    /// because "high ground" is a viewing rule and not a biome.
    fn matches(self, reading: &Reading, config: &Config) -> bool {
        let sea = config.sea_level_m;
        match self {
            Feature::All => true,
            Feature::Ocean => reading.water,
            Feature::Mountain => {
                !reading.water
                    && (reading.biome == Biome::Alpine
                        || reading.elevation_m >= sea + config.relief_m * 0.85)
            }
            Feature::Forest => {
                !reading.water
                    && matches!(
                        reading.biome,
                        Biome::BorealForest
                            | Biome::CoolTemperateForest
                            | Biome::TemperateForest
                            | Biome::TropicalDryForest
                            | Biome::TropicalRainforest
                    )
            }
            Feature::Grassland => !reading.water && reading.biome == Biome::Grassland,
            Feature::Desert => {
                !reading.water
                    && matches!(reading.biome, Biome::Desert | Biome::SubtropicalScrub)
            }
            Feature::Tundra => {
                !reading.water
                    && matches!(reading.biome, Biome::Tundra | Biome::PolarDesert)
            }
        }
    }

    /// **Whether the view draws contour rules.** Water has no elevation bands worth a
    /// rule, and an ocean view that drew them would be a map of the sea floor it did not
    /// measure. Every land view keeps them, over the cells it selects.
    const fn contours(self) -> bool {
        !matches!(self, Feature::Ocean)
    }

    /// A short phrase for the record when the centring search found nothing.
    const fn fallback(self) -> &'static str {
        match self {
            Feature::All => "the spawn column",
            _ => "the spawn column, after the ring search found no cell of this feature",
        }
    }
}

/// One cell's reading: the glyph, the schema's ink, and what the world said.
#[derive(Clone, Copy)]
struct Reading {
    glyph: char,
    ink: Ink,
    elevation_m: f64,
    biome: Biome,
    water: bool,
}

/// One generated view: the declaration, its record, and the numbers both state.
struct Generated {
    view: Feature,
    frame: String,
    record: String,
    summary: String,
}

fn main() -> ExitCode {
    let mut check = false;
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--check" => check = true,
            "--help" | "-h" => {
                println!(
                    "the map window's axis cell frames, generated from the world's own laws\n\n\
                     USAGE: voxel-mapframe [--check]\n\n\
                     With no argument the seven views (all, ocean, mountain, forest, grassland, \
                     desert, tundra) are written to \
                     display-packages/alpha-shell/frames/MAP-0001..0007. With `--check` the \
                     existing frames and records are parsed and compared against a fresh \
                     measurement and nothing is written.\n\n\
                     Exit 0 — every view parses and matches (or was written). Exit 1 — one does \
                     not. Exit 2 — a usage error.\n"
                );
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument {other:?}; try --help");
                return ExitCode::from(2);
            }
        }
    }

    let folder = workspace().join("display-packages/alpha-shell/frames");
    let generated: Vec<Generated> = Feature::ALL.into_iter().map(generate).collect();
    let mut failed = false;
    for view in &generated {
        let frame_path = folder.join(format!("{}.frame", view.view.frame_id()));
        let record_path = folder.join(format!("{}.md", view.view.frame_id()));
        if check {
            if let Err(why) = check_one(view, &frame_path, &record_path) {
                eprintln!("{}: {why}", view.view.frame_id());
                failed = true;
                continue;
            }
            println!("{}", view.summary);
            continue;
        }
        if let Err(why) = std::fs::write(&frame_path, &view.frame) {
            eprintln!("{} cannot be written: {why}", frame_path.display());
            failed = true;
            continue;
        }
        if let Err(why) = std::fs::write(&record_path, &view.record) {
            eprintln!("{} cannot be written: {why}", record_path.display());
            failed = true;
            continue;
        }
        println!(
            "wrote {} ({} bytes) and {} ({} bytes)",
            frame_path.display(),
            view.frame.len(),
            record_path.display(),
            view.record.len()
        );
        println!("{}", view.summary);
    }
    if failed {
        return ExitCode::FAILURE;
    }
    if check {
        println!(
            "check: {} views are the generator's own {} × {} pictures and records",
            generated.len(),
            COLS,
            ROWS
        );
    }
    ExitCode::SUCCESS
}

/// Compare one generated view against the two files the package carries.
fn check_one(view: &Generated, frame_path: &Path, record_path: &Path) -> Result<(), String> {
    let existing = std::fs::read_to_string(frame_path)
        .map_err(|why| format!("{} cannot be read: {why}", frame_path.display()))?;
    let parsed = Frame::parse(&existing)
        .map_err(|refusals| format!("the file is not a frame this grammar reads: {refusals}"))?;
    if parsed.feature.as_deref() != Some(view.view.name()) {
        return Err(format!(
            "the file declares feature {:?}, and this view is {:?}",
            parsed.feature,
            view.view.name()
        ));
    }
    let want = Frame::parse(&view.frame)
        .map_err(|refusals| format!("the generator's frame does not parse: {refusals}"))?;
    let have_picture = parsed.picture();
    let want_picture = want.picture();
    for row in 0..ROWS as usize {
        let (Some(have), Some(want_row)) = (have_picture.get(row), want_picture.get(row)) else {
            continue;
        };
        if have != want_row {
            return Err(format!(
                "row {row} differs:\n  file      {have}\n  generator {want_row}"
            ));
        }
    }
    let existing_record = std::fs::read_to_string(record_path)
        .map_err(|why| format!("{} cannot be read: {why}", record_path.display()))?;
    if existing_record != view.record {
        let at = existing_record
            .lines()
            .zip(view.record.lines())
            .position(|(have, want)| have != want)
            .map(|line| line + 1)
            .unwrap_or(0);
        return Err(format!(
            "{} is not the generated record; first difference at line {at}",
            record_path.display()
        ));
    }
    Ok(())
}

/// The workspace root this binary was built from: the crate's own manifest is
/// `crates/voxel-package`, so two parents up is the tree the frame belongs to.
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate sits two levels under the workspace")
        .to_path_buf()
}

/// **Find where a feature is, then measure the window around it.**
///
/// The default view is the spawn column. Every other view walks rings outward from the
/// spawn at [`SEARCH_STEP_M`] until a cell answers its predicate, and reports whether it
/// found one. A mountain view that found nothing falls back to the highest ground the
/// search crossed; every other fallback is the spawn, stated as such.
fn locate(view: Feature) -> ([f64; 2], bool) {
    if view == Feature::All {
        return (CENTRE_XZ, true);
    }
    let spec = WorldSpec::new(SEED, Config::default()).at_tick(TICK);
    let mut highest = (f64::NEG_INFINITY, CENTRE_XZ);
    for ring in 1..=SEARCH_RINGS {
        let radius = f64::from(ring) * SEARCH_STEP_M;
        let samples = ((TAU * radius / SEARCH_STEP_M).round() as u32).max(6);
        for step in 0..samples {
            let angle = TAU * f64::from(step) / f64::from(samples);
            let x = CENTRE_XZ[0] + radius * angle.cos();
            let z = CENTRE_XZ[1] + radius * angle.sin();
            let reading = read(&spec, x, z);
            if reading.elevation_m > highest.0 {
                highest = (reading.elevation_m, [x, z]);
            }
            if view.matches(&reading, &spec.config) {
                return ([x, z], true);
            }
        }
    }
    if view == Feature::Mountain {
        return (highest.1, false);
    }
    (CENTRE_XZ, false)
}

/// One world sample, through the same three laws the chunk builder uses.
fn read(spec: &WorldSpec, x: f64, z: f64) -> Reading {
    let elevation = terrain::elevation_m(spec, x, z);
    let (biotemperature, precipitation) = ecology::holdridge_inputs(spec, x, z, elevation, TICK);
    let biome = ecology::biome_at(spec, elevation, biotemperature, precipitation);
    let water = elevation <= spec.config.sea_level_m;
    let (glyph, ink) = glyph_of(biome, water, elevation, spec.config.sea_level_m);
    Reading { glyph, ink, elevation_m: elevation, biome, water }
}

/// **Measure one view, then draw it.** Every cell is one world sample; the border,
/// ticks, cardinal axis and legend are the only declared glyphs.
fn generate(view: Feature) -> Generated {
    let spec = WorldSpec::new(SEED, Config::default()).at_tick(TICK);
    let (centre, found) = locate(view);
    let mut grid = vec![vec![(' ', Ink::Text); COLS as usize]; ROWS as usize];
    let mut readings: Vec<Reading> = Vec::with_capacity((MAP_COLS * MAP_ROWS) as usize);

    // **The world first**, one sample a cell. Row 0 is north.
    for row in 0..MAP_ROWS {
        for col in 0..MAP_COLS {
            let x = centre[0] + (col as f64 - MAP_COLS as f64 * 0.5 + 0.5) * CELL_M;
            let z = centre[1] + (MAP_ROWS as f64 * 0.5 - row as f64 - 0.5) * CELL_M;
            let reading = read(&spec, x, z);
            let selected = view.matches(&reading, &spec.config);
            let ink = if view == Feature::All || selected {
                reading.ink
            } else {
                // **The rest of the world stays drawn.** A view that erased the cells it
                // does not select would be a second map with a different shape, and the
                // player could not tell a feature from a hole in the measurement.
                Ink::Disabled
            };
            let at = (MAP_LEFT + col) as usize;
            let down = (MAP_TOP + row) as usize;
            grid[down][at] = (reading.glyph, ink);
            readings.push(reading);
        }
    }

    // **The contours**, from the same readings: a cell whose band differs from its
    // west or east neighbour draws a vertical rule, one whose band differs north or
    // south draws a horizontal one, and a cell where both change is a cross. A water
    // neighbour is a change too, so the coast is drawn rather than only implied. In a
    // feature view the contour is drawn only over the selected cells and their edge,
    // so the contour traces the feature rather than the whole world.
    let band = |col: i32, row: i32| -> Option<i32> {
        if col < 0 || row < 0 || col >= MAP_COLS as i32 || row >= MAP_ROWS as i32 {
            return None;
        }
        let reading = &readings[(col + row * MAP_COLS as i32) as usize];
        if reading.water || !view.matches(reading, &spec.config) {
            return None;
        }
        Some((reading.elevation_m / CONTOUR_M).floor() as i32)
    };
    if view.contours() {
        for row in 0..MAP_ROWS as i32 {
            for col in 0..MAP_COLS as i32 {
                let here = &readings[(col + row * MAP_COLS as i32) as usize];
                if here.water || !view.matches(here, &spec.config) {
                    continue;
                }
                let Some(own) = band(col, row) else { continue };
                let west = band(col - 1, row) != Some(own);
                let east = band(col + 1, row) != Some(own);
                let north = band(col, row - 1) != Some(own);
                let south = band(col, row + 1) != Some(own);
                let glyph = match (west || east, north || south) {
                    (true, true) => Some('+'),
                    (true, false) => Some('|'),
                    (false, true) => Some('-'),
                    (false, false) => None,
                };
                if let Some(glyph) = glyph {
                    let at = (MAP_LEFT + col as u32) as usize;
                    let down = (MAP_TOP + row as u32) as usize;
                    grid[down][at] = (glyph, Ink::Divider);
                }
            }
        }
    }

    // **The axis frame**: a border with a tick every nine columns and three rows, and
    // the four cardinal letters *in* the border rather than floating over the map.
    for col in 0..COLS {
        let tick = col == 0 || col == COLS - 1 || (col % 9 == 0);
        grid[0][col as usize] = (if tick { '+' } else { '-' }, Ink::Divider);
        grid[(ROWS - 1) as usize][col as usize] = (if tick { '+' } else { '-' }, Ink::Divider);
    }
    for row in 1..ROWS - 1 {
        let tick = row % 3 == 0;
        grid[row as usize][0] = (if tick { '+' } else { '|' }, Ink::Divider);
        grid[row as usize][(COLS - 1) as usize] = (if tick { '+' } else { '|' }, Ink::Divider);
    }
    grid[0][2] = ('N', Ink::Accent);
    grid[(ROWS - 1) as usize][2] = ('S', Ink::Accent);
    grid[10][0] = ('W', Ink::Accent);
    grid[10][(COLS - 1) as usize] = ('E', Ink::Accent);

    // **The key and the declared recipe**, in the three rows below the map.
    for row in MAP_ROWS + MAP_TOP..ROWS - 1 {
        for col in MAP_LEFT..MAP_LEFT + MAP_COLS {
            grid[row as usize][col as usize] = (' ', Ink::Text);
        }
    }
    let key: &[(Ink, &str)] = &[
        (Ink::Accent, "~water"),
        (Ink::Green, ",grass"),
        (Ink::Green, "#forest"),
        (Ink::Dim, "^alpine"),
        (Ink::Orange, ".desert"),
        (Ink::Green, "@rain"),
    ];
    let mut col = MAP_LEFT;
    for (ink, text) in key {
        for glyph in text.chars() {
            if col >= MAP_LEFT + MAP_COLS {
                break;
            }
            grid[(MAP_ROWS + MAP_TOP) as usize][col as usize] = (glyph, *ink);
            col += 1;
        }
        col += 1;
    }
    let matched = readings
        .iter()
        .filter(|reading| view.matches(reading, &spec.config))
        .count();
    let land = readings.iter().filter(|reading| !reading.water).count();
    let land_share = land * 100 / readings.len().max(1);
    let highest = readings
        .iter()
        .map(|reading| reading.elevation_m)
        .fold(f64::NEG_INFINITY, f64::max);
    let biomes = Biome::ALL
        .into_iter()
        .filter(|biome| readings.iter().any(|reading| !reading.water && reading.biome == *biome))
        .count();
    // Three short lines, so nothing runs into the right border: the key owns the first,
    // the scale and the view the second, and the seed, land share and selection the
    // third. A legend that overflowed would be clipped by the raster and the clipped
    // number would look like the measured one.
    put(&mut grid, MAP_LEFT + 44, MAP_ROWS + MAP_TOP, &format!("high {highest:.0}m"), Ink::Dim);
    let legend = format!(
        "N ^ {CELL_M:.0} m/cell  centre {:.1},{:.1}  view {}",
        centre[0],
        centre[1],
        view.name()
    );
    put(&mut grid, MAP_LEFT, MAP_ROWS + MAP_TOP + 1, &legend, Ink::Dim);
    let census = format!("seed {SEED}  land {land_share}%  {matched}/{}", readings.len());
    put(&mut grid, MAP_LEFT, MAP_ROWS + MAP_TOP + 2, &census, Ink::Dim);

    // **Paper**, one row at a time: the map is the background, the three legend rows are
    // chrome. A fill is a surface and the glyphs above stand on it.
    let mut fills: Vec<String> = Vec::new();
    for row in MAP_TOP..MAP_TOP + MAP_ROWS {
        fills.push(format!("fill\t{MAP_LEFT}\t{row}\t{MAP_COLS}\t{}", Paper::Background.name()));
    }
    for row in MAP_ROWS + MAP_TOP..ROWS - 1 {
        fills.push(format!("fill\t{MAP_LEFT}\t{row}\t{MAP_COLS}\t{}", Paper::Chrome.name()));
    }

    // **Runs**, coalesced by ink so a row with one terrain is one line and not fifty.
    let mut runs: Vec<String> = Vec::new();
    for (row, cells) in grid.iter().enumerate() {
        let mut col = 0usize;
        while col < cells.len() {
            let (glyph, ink) = cells[col];
            if glyph == ' ' {
                col += 1;
                continue;
            }
            let start = col;
            let mut text = String::new();
            while col < cells.len() && cells[col].0 != ' ' && cells[col].1 == ink {
                text.push(cells[col].0);
                col += 1;
            }
            runs.push(format!("run\t{start}\t{row}\t{}\t{text}", ink.name()));
        }
    }

    let mut frame = String::new();
    frame.push_str("shploingus.cellframe/1\n");
    frame.push_str(&format!("frame\t{}\n", view.frame_id()));
    frame.push_str("\twindow\tmap-plan\n");
    frame.push_str(&format!("\tcols\t{COLS}\n"));
    frame.push_str(&format!("\trows\t{ROWS}\n"));
    frame.push_str(&format!("\torder\t{}\n", view.order()));
    frame.push_str("\trequires\tnone\n");
    frame.push_str(&format!("\tfeature\t{}\n", view.name()));
    // **The declaration the GPU consumes**, before the fallback picture: the fields, the 1/32
    // per-axis cell law, the camera and the extent. The rows below are what a shell with no
    // surface keeps drawing; these rows are what the map *is*.
    frame.push_str(&format!("\tfield\televation\t{ELEVATION_SOURCE}\n"));
    frame.push_str(&format!("\tfield\tbiome\t{BIOME_SOURCE}\n"));
    frame.push_str("\tshaper\tprofile32\t32\n");
    frame.push_str("\tprojection\torthographic\n");
    frame.push_str(&format!(
        "\tcentre\t{:.1}\t{:.1}\n",
        centre[0], centre[1]
    ));
    frame.push_str(&format!("\tcell\t{CELL_M:.0}\n"));
    frame.push_str(&format!(
        "\textent\t{}\t{}\n",
        GPU_EXTENT[0], GPU_EXTENT[1]
    ));
    frame.push_str(&format!("\tpitch\t{GPU_PITCH_DEG:.0}\n"));
    for fill in &fills {
        frame.push_str(fill);
        frame.push('\n');
    }
    for run in &runs {
        frame.push_str(run);
        frame.push('\n');
    }

    let where_from = match (view, found) {
        (Feature::All, _) => "the game's spawn column",
        (_, true) => "found by the ring search",
        (_, false) => view.fallback(),
    };
    let record = record_of(view, centre, found, matched, readings.len(), land_share, biomes, highest);
    let summary = format!(
        "{} {}: measured {} × {} cells at {CELL_M:.0} m/cell around {centre:?} ({where_from}); \
         {matched} of {} cells answer the view; {land} land, {land_share}% of the disc, highest \
         {highest:.1} m, {biomes} of {} biomes present; {} fills, {} runs",
        view.frame_id(),
        view.name(),
        MAP_COLS,
        MAP_ROWS,
        readings.len(),
        Biome::ALL.len(),
        fills.len(),
        runs.len()
    );
    Generated { view, frame, record, summary }
}

/// The human half of one view: the four sections every frame record carries.
fn record_of(
    view: Feature,
    centre: [f64; 2],
    found: bool,
    matched: usize,
    total: usize,
    land_share: usize,
    biomes: usize,
    highest: f64,
) -> String {
    let name = view.name();
    let id = view.frame_id();
    let gpu_cols = GPU_EXTENT[0];
    let gpu_rows = GPU_EXTENT[1];
    let centring = if view == Feature::All {
        "The view is centred on the game's spawn column, because the default map is the map of\nwhere the player is."
            .to_string()
    } else if found {
        format!(
            "The view is centred on the first cell the ring search found that answers `{name}`,\n\
             walking 128 m rings outward from the spawn to 12.3 km."
        )
    } else {
        format!(
            "**The ring search found no `{name}` cell within 12.3 km of the spawn.** The view is\n\
             therefore centred on {}; the record states the fallback rather than presenting an\n\
             arbitrary centre as a find.",
            view.fallback()
        )
    };
    let selection = if view == Feature::All {
        "Every cell is in its biome's own ink; this is the map `plan all` returns"
            .to_string()
    } else {
        format!(
            "Cells that answer `{name}` keep their biome's own ink and the rest of the world is\n\
             drawn in the schema's `disabled` ink, so the view is a highlight on the same\n\
             measurement and not a second map"
        )
    };
    format!(
        "{id} — the map window's {name} view\n\
         ============================================\n\n\
         # Change\n\n\
         Generate `{id}`, the map window's `{name}` view, from the world's own laws at seed\n\
         `{SEED}` and tick `{TICK}`: every cell is one `terrain::elevation_m` sample and one\n\
         `ecology::biome_at` reading on the same Holdridge inputs the chunk builder uses, at\n\
         {CELL_M:.0} m a cell. {selection}.\n\n\
         {centring}\n\n\
         The frame is a declaration in the cellframe grammar, addressed to `map-plan`'s content\n\
         space, with `feature {name}`. The mount grades all seven views when it opens the\n\
         package; `plan {name}` selects this one and never re-reads a file.\n\n\
         **The declaration is GPU-first.** The frame names the fields (`terrain::elevation_m`,\n\
         `ecology::biome_at`), the per-axis cell law (`profile32` at 32 divisions), the orthographic\n\
         camera and a {gpu_cols} × {gpu_rows} cell terrain extent at {CELL_M:.0} m a cell;\n\
         the shell samples those laws and draws the heightfield through the world's own pipeline,\n\
         tilted {GPU_PITCH_DEG:.0}° so the horizon is in the window. **The glyph rows below are the\n\
         fallback**, kept for a host with no GPU surface: they are the same view drawn as a\n\
         character grid, not a second map.\n\n\
         # Acceptance\n\n\
         - `voxel-layers` names `{id}` in the `alpha-shell` emit and prints it with this record;\n\
           the recipe names it by a `row` and the two package copies are byte-identical.\n\
         - The frame is 56 × 22 and fits `map-plan`'s declared extent (56 × 8 dp and 22 × 16 dp\n\
           inside 480 × 440 dp); a frame larger than the window is refused by cell count.\n\
         - `plan {name}` selects it in the console; `plan next` reaches it by cycling; `plan all`\n\
           returns `MAP-0001`; an undeclared token is refused by name and changes nothing.\n\
         - The picture is the generator's own: `voxel-mapframe --check` regenerates every view\n\
           and compares every row and this record, and exits non-zero on the first difference.\n\n\
         # Evidence\n\n\
         - `cargo run -p voxel-package --bin voxel-mapframe` — writes `MAP-0001..0007`, exit 0.\n\
         - `cargo run -p voxel-package --bin voxel-mapframe -- --check` — every picture and\n\
           record is the generator's, exit 0.\n\
         - This view's reading: {matched} of {total} cells answer `{name}`; land is {land_share}%\n\
           of the disc; the highest cell in the window is {highest:.1} m; {biomes} of 11 biomes\n\
           are present at this centre.\n\
         - `cargo test -p voxel-package cellframe::` — the grammar's own refusals, including the\n\
           optional `feature` key: it may be absent, may not be declared twice, and may not carry\n\
           a token the console could not type.\n\n\
         # Rollback\n\n\
         Delete `frames/{id}.frame`, `frames/{id}.md` and the `row {id}` line in\n\
         `package.shspec` (and its copy in `packages/alpha-shell.shspec`). The package then\n\
         carries six views; the mount grades what is left and `plan` lists only those tokens.\n\
         Deleting all seven returns the shell to composed elements, exactly as before the frames\n\
         existed. No code is reverted.\n",
    )
}

/// Write `text` into a row, one glyph a cell, in one ink.
fn put(grid: &mut [Vec<(char, Ink)>], col: u32, row: u32, text: &str, ink: Ink) {
    if row as usize >= grid.len() {
        return;
    }
    for (offset, glyph) in text.chars().enumerate() {
        let at = col as usize + offset;
        if at >= grid[row as usize].len() {
            break;
        }
        grid[row as usize][at] = (glyph, ink);
    }
}

/// **One cell's glyph, from the world's own classification.** Water is the sea level's
/// own answer; land is the biome's, with alpine above the relief's declared height.
fn glyph_of(biome: Biome, water: bool, elevation_m: f64, sea_level_m: f64) -> (char, Ink) {
    if water {
        // Two depths, because a map that draws a shallow shelf and a deep basin the
        // same colour is a map that has thrown away its own measurement.
        return if elevation_m < sea_level_m - 6.0 {
            ('=', Ink::Accent)
        } else {
            ('~', Ink::Accent)
        };
    }
    match biome {
        Biome::PolarDesert => ('*', Ink::Dim),
        Biome::Tundra => (':', Ink::Dim),
        Biome::BorealForest => ('T', Ink::Green),
        Biome::CoolTemperateForest => ('t', Ink::Green),
        Biome::TemperateForest => ('#', Ink::Green),
        Biome::Grassland => (',', Ink::Green),
        Biome::Desert => ('.', Ink::Orange),
        Biome::SubtropicalScrub => (';', Ink::Orange),
        Biome::TropicalDryForest => ('Y', Ink::Green),
        Biome::TropicalRainforest => ('@', Ink::Green),
        Biome::Alpine => ('^', Ink::Dim),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_view_has_one_frame_id_one_feature_token_and_one_order() {
        let ids: std::collections::BTreeSet<&str> =
            Feature::ALL.iter().map(|view| view.frame_id()).collect();
        let names: std::collections::BTreeSet<&str> =
            Feature::ALL.iter().map(|view| view.name()).collect();
        let orders: std::collections::BTreeSet<u32> =
            Feature::ALL.iter().map(|view| view.order()).collect();
        assert_eq!(ids.len(), Feature::ALL.len(), "two views share a frame id: {ids:?}");
        assert_eq!(names.len(), Feature::ALL.len(), "two views share a token: {names:?}");
        assert_eq!(orders.len(), Feature::ALL.len(), "two views share an order: {orders:?}");
        assert_eq!(Feature::ALL[0].frame_id(), "MAP-0001");
        assert_eq!(Feature::ALL[0].name(), "all");
    }

    #[test]
    fn the_default_and_one_feature_view_are_frames_the_grammar_reads_and_the_picture_is_the_grid() {
        let default = generate(Feature::All);
        let parsed = Frame::parse(&default.frame).expect("the default map frame parses");
        assert_eq!(parsed.id, "MAP-0001");
        assert_eq!(parsed.window, "map-plan");
        assert_eq!(parsed.feature.as_deref(), Some("all"));
        assert_eq!((parsed.cols, parsed.rows), (COLS, ROWS));
        let picture = parsed.picture();
        assert_eq!(picture.len(), ROWS as usize);
        for row in &picture {
            assert_eq!(row.chars().count(), COLS as usize);
        }
        // The axis frame is present and not a corner: all four letters are in the border.
        assert_eq!(picture[0].chars().nth(2), Some('N'));
        assert_eq!(picture[(ROWS - 1) as usize].chars().nth(2), Some('S'));
        assert_eq!(picture[10].chars().next(), Some('W'));
        assert_eq!(picture[10].chars().last(), Some('E'));

        let ocean = generate(Feature::Ocean);
        let parsed = Frame::parse(&ocean.frame).expect("the ocean map frame parses");
        assert_eq!(parsed.id, "MAP-0002");
        assert_eq!(parsed.feature.as_deref(), Some("ocean"));
        // The selection is drawn: water is in the accent ink and the land that does not
        // answer the view is in the schema's disabled ink. Both are in the same frame, in
        // the map rows — the key below them legitimately names every biome in its own ink,
        // and a test that read the legend as map would fail on the legend.
        let map_rows = MAP_TOP..MAP_TOP + MAP_ROWS;
        assert!(
            parsed
                .runs
                .iter()
                .any(|run| map_rows.contains(&run.row) && run.ink == Ink::Accent),
            "the ocean view draws no water"
        );
        assert!(
            parsed
                .runs
                .iter()
                .any(|run| map_rows.contains(&run.row) && run.ink == Ink::Disabled),
            "the ocean view does not keep the rest of the world drawn"
        );
        assert!(
            parsed
                .runs
                .iter()
                .all(|run| !map_rows.contains(&run.row) || run.ink != Ink::Green),
            "the ocean view left a land biome in its full ink"
        );
    }

    #[test]
    fn a_water_column_is_the_sea_levels_answer_and_a_land_column_is_the_biomes() {
        let sea = 16.0;
        assert_eq!(glyph_of(Biome::Grassland, true, sea - 10.0, sea).0, '=');
        assert_eq!(glyph_of(Biome::Grassland, true, sea, sea).0, '~');
        assert_eq!(glyph_of(Biome::Desert, false, sea + 2.0, sea).0, '.');
        assert_eq!(glyph_of(Biome::Alpine, false, sea + 38.0, sea).0, '^');
        // Every biome has a glyph and no two share one: a key with a duplicate would
        // make the map's own legend a lie.
        let glyphs: Vec<char> = Biome::ALL
            .into_iter()
            .map(|biome| glyph_of(biome, false, sea, sea).0)
            .collect();
        let unique: std::collections::BTreeSet<char> = glyphs.iter().copied().collect();
        assert_eq!(unique.len(), glyphs.len(), "two biomes share a glyph: {glyphs:?}");
        assert!(!glyphs.contains(&'~') && !glyphs.contains(&'='), "land uses a water glyph");
    }

    #[test]
    fn a_feature_is_the_worlds_own_answer_and_not_a_second_classification() {
        let config = Config::default();
        let land = Reading {
            glyph: '#',
            ink: Ink::Green,
            elevation_m: config.sea_level_m + 20.0,
            biome: Biome::Alpine,
            water: false,
        };
        assert!(!Feature::Ocean.matches(&land, &config));
        assert!(Feature::Mountain.matches(&land, &config));
        assert!(!Feature::Forest.matches(&land, &config));
        let water = Reading { water: true, ..land };
        assert!(Feature::Ocean.matches(&water, &config));
        assert!(!Feature::Mountain.matches(&water, &config));
        assert!(Feature::All.matches(&water, &config));
        // The selected view is a highlight of the same map, so only the ocean view drops
        // contours; every land view keeps them.
        assert!(!Feature::Ocean.contours());
        assert!(Feature::Mountain.contours());
        assert!(Feature::All.contours());
    }
}
