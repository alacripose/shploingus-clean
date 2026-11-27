//! **A cell data frame, declared before it draws** — the package half of the shell's streamed
//! cells, and the artifact class PATCH-076 named as owed.
//!
//! # Why a frame is a file rather than a block in the recipe
//!
//! `shploingus.recipe/3` has windows, rows and ballots, and **no frame rows**. Ruling R-64.1 says a
//! document that requires a field the schema does not have is a refusal the build surfaces — not a
//! schema change and not a silent extension. So a frame is a **sibling artifact the package names by
//! id in a `row`**, exactly as a layer is: the recipe grammar is untouched, and a package that
//! declares a frame and no row that names it is refused here rather than mounted as furniture.
//!
//! # What a frame is
//!
//! A frame is a **grid of cells addressed to one declared window's content space**. It carries two
//! kinds of row:
//!
//! * `fill` — a run of paper, the surface a row of the map stands on;
//! * `run` — glyphs from a cell, drawn in one ink, clipped by the frame's own width.
//!
//! **A frame names a role, never a colour.** `accent`, `dim` and `surface` are resolved against the
//! Android 4 schema where the raster draws (PATCH-071), so the format cannot become a second palette
//! that resembles the shell's. That is the same rule the theme rows already keep: *a role is not a
//! colour here.*
//!
//! # What a frame may not do
//!
//! * It may not address anything but a window's content space. `window` is a **declared window id**
//!   and the client asks [`crate::recipe::WindowSpec::geometry`] for the extent the cells must fit
//!   inside; a frame that does not fit its window is refused by cell count rather than drawn
//!   through the window's chrome.
//! * It may not draw a cell outside its own grid. A `fill` or `run` whose last cell is past `cols`
//!   or `rows` is refused with the cell and the row that wrote it — a truncated run is a silent cut,
//!   and this crate exists against silent cuts.
//! * It may not be a declaration that draws nothing. A frame with no `fill` and no `run` is refused
//!   by name: an empty frame is a window's content erased, and an absence has to be stated as one.
//!
//! # The grammar
//!
//! ```text
//! shploingus.cellframe/1
//! frame<TAB>id                       (the row id the package's recipe names)
//!     window      <window-id>        (a window the same package declares)
//!     cols        <integer>          (the frame's width in cells)
//!     rows        <integer>          (the frame's height in cells)
//!     order       <integer>
//!     requires    <frame-id>[,...]|none
//! fill<TAB>col<TAB>row<TAB>cols<TAB>paper      (may be repeated)
//! run<TAB>col<TAB>row<TAB>ink<TAB>text          (may be repeated)
//! ```
//!
//! **Every listed key is required**, an unknown key is refused by name, and a duplicate key is
//! refused by name. The strictness is the recipe's own and `layer.rs`'s: a plausible value that
//! silently overwrote a declared one is the failure this crate exists against.
//!
//! # The two files are one declaration
//!
//! `frames/<id>.frame` is the machine part and `frames/<id>.md` is the record — the split
//! `layer.rs` already uses. [`Frame::render`] prints both as one human-readable block, and a record
//! missing one of the four patch sections is refused by the section's own name.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use voxel_model::{RefusalKind, RefusalSet};

use crate::patch;

/// The pin. A different grammar is a different file, and an older one is refused rather than read.
pub const HEADER: &str = "shploingus.cellframe/1";
/// Every version this build accepts. Exactly one, for the recipe's own reason.
pub const ACCEPTED_VERSIONS: &[&str] = &[HEADER];

/// One frame's ceiling. A declaration, not a truncation: a frame larger than this is refused.
///
/// **Interim, and lifted from 512 KiB — deliberately, not silently.** The format owner's Q-4 ruling
/// (`secure-shell-audit/cell-schema-ruling.md`) says the 512 KiB / 16 384 pair is **not** the ceiling:
/// the surface is stateful and real-time, the addressing 64-bit, the geometry subvoxel, and the exact
/// numbers are still owed. This admits that class (a package carrying its whole timeline measures
/// ~741 KB) while the owner's numbers are owed. `MAX_CELLS` below is **not** moved with it: it is a
/// declared grid ratio (`MAX_COLS * MAX_ROWS / 16`) and the ruling's cell number is part of what is
/// owed.
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;
/// The widest a frame may declare. `cols × rows` is also bounded, because a grid is memory the
/// shell allocates per frame and a number nobody bounded is a bound the allocator chooses.
pub const MAX_COLS: u32 = 512;
/// The tallest a frame may declare.
pub const MAX_ROWS: u32 = 512;
/// The most cells one frame may hold.
pub const MAX_CELLS: u32 = 16_384;
/// One cell's width in device-independent pixels. **The format's number**, because the extent
/// check below is the format's question; `voxel-client::shell::cells` re-exports it and a test
/// proves the two are one number.
pub const CELL_W_DP: u32 = 8;
/// One cell's height in device-independent pixels.
pub const CELL_H_DP: u32 = 16;

/// Every **required** key a frame carries, in grammar order. The parser, the missing-key report
/// and the renderer all read this list. The optional key is [`OPTIONAL_KEYS`].
pub const KEYS: [&str; 5] = ["window", "cols", "rows", "order", "requires"];

/// The keys a frame may carry **and may omit**. `feature` names the world feature a map frame
/// shows: the mount draws one frame per window, so a package that declares several views of the
/// same window selects between them by this token rather than by inventing a second window.
pub const OPTIONAL_KEYS: [&str; 1] = ["feature"];

/// The paper a cell stands on, **named by role**. The raster resolves it against the schema.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Paper {
    /// A window's own body.
    Surface,
    /// Chrome, one step under the surface.
    Chrome,
    /// The schema's background.
    Background,
    /// The schema's divider.
    Divider,
    /// No paper: the world shows through, and the glyphs stand on it.
    Transparent,
}

impl Paper {
    /// Every role the grammar admits.
    pub const ALL: [Paper; 5] = [
        Paper::Surface,
        Paper::Chrome,
        Paper::Background,
        Paper::Divider,
        Paper::Transparent,
    ];

    /// The word a `.frame` writes.
    pub const fn name(self) -> &'static str {
        match self {
            Paper::Surface => "surface",
            Paper::Chrome => "chrome",
            Paper::Background => "background",
            Paper::Divider => "divider",
            Paper::Transparent => "transparent",
        }
    }

    /// Read a role by its word. `None` is a refusal the caller names.
    pub fn parse(text: &str) -> Option<Paper> {
        Paper::ALL.into_iter().find(|paper| paper.name() == text)
    }
}

/// The ink a run is written in, **named by role**. The raster resolves it against the schema.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ink {
    /// Primary ink.
    Text,
    /// Secondary ink.
    Dim,
    /// Disabled ink.
    Disabled,
    /// The schema's accent.
    Accent,
    /// The schema's divider, for contour rules drawn as glyphs.
    Divider,
    /// The schema's status red.
    Red,
    /// The schema's status green.
    Green,
    /// The schema's status orange.
    Orange,
}

impl Ink {
    /// Every role the grammar admits.
    pub const ALL: [Ink; 8] = [
        Ink::Text,
        Ink::Dim,
        Ink::Disabled,
        Ink::Accent,
        Ink::Divider,
        Ink::Red,
        Ink::Green,
        Ink::Orange,
    ];

    /// The word a `.frame` writes.
    pub const fn name(self) -> &'static str {
        match self {
            Ink::Text => "text",
            Ink::Dim => "dim",
            Ink::Disabled => "disabled",
            Ink::Accent => "accent",
            Ink::Divider => "divider",
            Ink::Red => "red",
            Ink::Green => "green",
            Ink::Orange => "orange",
        }
    }

    /// Read a role by its word. `None` is a refusal the caller names.
    pub fn parse(text: &str) -> Option<Ink> {
        Ink::ALL.into_iter().find(|ink| ink.name() == text)
    }
}

/// One run of paper: `cols` cells from `(col, row)`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fill {
    /// The first cell's column.
    pub col: u32,
    /// The row.
    pub row: u32,
    /// How many cells wide the paper is.
    pub cols: u32,
    /// The paper's role.
    pub paper: Paper,
}

/// One run of glyphs: from `(col, row)`, one glyph per cell, in one ink.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Run {
    /// The first cell's column.
    pub col: u32,
    /// The row.
    pub row: u32,
    /// The ink's role.
    pub ink: Ink,
    /// The glyphs. **One glyph per cell**, so a wide character occupies one cell rather than two.
    pub text: String,
}

/// **One field source a GPU map frame declares** — a name the renderer knows and the law that
/// answers it. The frame names the source rather than sampling it: the map's elevation and biome
/// are the world's own laws, and a frame holding a copy of one would be a second world.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldRef {
    /// The field's name: `elevation` or `biome`.
    pub name: String,
    /// The law that answers it, as the world states it (`terrain::elevation_m`).
    pub source: String,
}

/// **A map frame's GPU declaration**: the fields and the 1/32 cell law the terrain renderer
/// consumes, plus the extent and camera it is drawn with.
///
/// The `.frame` still carries its glyph rows as the **fallback** a shell with no GPU surface
/// draws, but this record is the drawing: the ASCII raster is not the map, it is what the map
/// degrades to. `field` names terrain laws rather than cell glyphs, `shaper` names the per-axis
/// cell law (`profile32` at 32 divisions), and `projection` names the camera.
#[derive(Clone, PartialEq, Debug)]
pub struct MapView {
    /// The fields the renderer samples, in declaration order.
    pub fields: Vec<FieldRef>,
    /// The cell law's name — `profile32`.
    pub shaper: String,
    /// That law's divisions per axis: 32.
    pub shaper_cells: u32,
    /// The camera: `orthographic`.
    pub projection: String,
    /// The world column the view is centred on, metres.
    pub centre_xz: [f64; 2],
    /// One map cell, metres.
    pub cell_m: f64,
    /// The map's extent in cells: columns (x) and rows (z).
    pub extent: [u32; 2],
    /// The camera's downward tilt, degrees. Negative looks down; it is what puts the horizon in
    /// the window rather than drawing a plan from directly above.
    pub pitch_deg: f64,
}

impl MapView {
    /// The fields this build can sample. An unknown name is a refusal, not a default: a frame that
    /// asked for a field the renderer does not know would otherwise draw the one it does.
    pub const FIELDS: [&'static str; 2] = ["elevation", "biome"];
    /// The only cell law the renderer carries.
    pub const SHAPER: &'static str = "profile32";
    /// The only projection it carries.
    pub const PROJECTION: &'static str = "orthographic";

    /// The source declared for `name`, if this frame declares it.
    pub fn source_of(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.source.as_str())
    }
}

/// One frame, as declared. [`Frame::parse`] is the only constructor and it checks every field.
#[derive(Clone, PartialEq, Debug)]
pub struct Frame {
    /// The frame's id — **and the row id the package's recipe names it by**.
    pub id: String,
    /// The window whose content space this frame addresses.
    pub window: String,
    /// The frame's width in cells.
    pub cols: u32,
    /// The frame's height in cells.
    pub rows: u32,
    /// Draw order within the content layer. Lower draws first.
    pub order: u32,
    /// Frames that must be registered before this one, by id.
    pub requires: Vec<String>,
    /// **The world feature this frame shows**, if it declares one — `all`, `ocean`, `mountain`,
    /// and so on. `None` is a frame with no claim on the world (a panel's cells, a test fixture).
    /// The token is what the console's `plan` verb selects; it is not a window id.
    pub feature: Option<String>,
    /// **The GPU declaration**, when the frame draws through the terrain renderer. `None` is a
    /// frame of glyphs alone, which is every frame the shell drew before the map surface existed.
    pub map: Option<MapView>,
    /// The paper runs, in declaration order.
    pub fills: Vec<Fill>,
    /// The glyph runs, in declaration order — the fallback picture when no GPU surface exists.
    pub runs: Vec<Run>,
}

impl Frame {
    /// **The only constructor that checks.** A wrong header, an unknown or duplicate key, a missing
    /// key, a row outside the grid, or a frame that draws nothing refuses here, with every problem
    /// named in one pass rather than one per round-trip.
    pub fn parse(source: &str) -> Result<Frame, RefusalSet> {
        let mut errors = RefusalSet::new();
        if source.len() > MAX_FRAME_BYTES {
            errors.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "frame declaration has {} bytes; the ceiling is {MAX_FRAME_BYTES}",
                    source.len()
                ),
            );
            return Err(errors);
        }

        let all: Vec<&str> = source.lines().collect();
        let header = all.first().copied().unwrap_or_default();
        if !ACCEPTED_VERSIONS.contains(&header) {
            errors.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "unknown cellframe schema {header:?}; this build accepts {ACCEPTED_VERSIONS:?}. A \
                     recipe header is not a frame, and reading one as the other would be a \
                     plausible substitute for a declared grammar"
                ),
            );
            return Err(errors);
        }

        let Some(declaration) = all.get(1).copied() else {
            errors.refuse(
                RefusalKind::MalformedRecord,
                "the frame declaration ends at the header; a frame needs a `frame<TAB>id` row",
            );
            return Err(errors);
        };
        let fields: Vec<&str> = declaration.split('\t').collect();
        let id = match fields.as_slice() {
            ["frame", id] if crate::layer::row_identifier(id) => id.to_string(),
            _ => {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "expected `frame<TAB>id` after the header, found {declaration:?}; the id is \
                         the row id the package's recipe names, so it is the record's id shape \
                         (`MAP-0001`), never prose"
                    ),
                );
                String::new()
            }
        };

        let mut window: Option<String> = None;
        let mut cols: Option<u32> = None;
        let mut rows: Option<u32> = None;
        let mut order: Option<u32> = None;
        let mut requires: Option<Vec<String>> = None;
        let mut feature: Option<String> = None;
        // **The GPU declaration's own keys** — optional together, and one declaration when any is
        // present. `map_declared` distinguishes "no map keys" from "map keys that did not parse",
        // so a broken map is refused rather than silently drawn as glyphs.
        let mut map_declared = false;
        let mut fields: Vec<FieldRef> = Vec::new();
        let mut shaper: Option<(String, u32)> = None;
        let mut projection: Option<String> = None;
        let mut centre: Option<[f64; 2]> = None;
        let mut cell_m: Option<f64> = None;
        let mut extent: Option<[u32; 2]> = None;
        let mut pitch: Option<f64> = None;
        let mut fills: Vec<Fill> = Vec::new();
        let mut runs: Vec<Run> = Vec::new();

        for (index, line) in all.iter().enumerate().skip(2) {
            let number = index + 1;
            let trimmed = line.trim_end_matches('\r');
            if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
                continue;
            }
            // **A key row is indented** (`\twindow\tmap-plan`), so the line's leading tab is not a
            // field. A cell row is never indented, so trimming the start cannot eat a run's text.
            let cells: Vec<&str> = trimmed.trim_start().split('\t').collect();
            match cells.as_slice() {
                // **A cell row is read before the key match**, because its last field is prose.
                ["fill", col, row, wide, paper] => {
                    match (
                        number_of(col),
                        number_of(row),
                        number_of(wide),
                        Paper::parse(paper),
                    ) {
                        (Some(col), Some(row), Some(wide), Some(paper)) => fills.push(Fill {
                            col,
                            row,
                            cols: wide,
                            paper,
                        }),
                        _ => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `fill` is \
                                     `fill<TAB>col<TAB>row<TAB>cols<TAB>paper` with a whole number \
                                     in each count and one of {:?}, found {trimmed:?}",
                                    Paper::ALL.map(Paper::name)
                                ),
                            );
                        }
                    }
                }
                ["run", col, row, ink, text] => {
                    match (number_of(col), number_of(row), Ink::parse(ink)) {
                        (Some(col), Some(row), Some(ink)) if !text.is_empty() => {
                            runs.push(Run { col, row, ink, text: (*text).to_string() })
                        }
                        _ => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `run` is \
                                     `run<TAB>col<TAB>row<TAB>ink<TAB>text` with a whole number in \
                                     each count, one of {:?}, and text that is not empty; found \
                                     {trimmed:?}",
                                    Ink::ALL.map(Ink::name)
                                ),
                            );
                        }
                    }
                }
                // **The GPU map declaration.** These rows name laws rather than cells, so the
                // map renderer can sample the world instead of reading a glyph grid: the frame
                // says *what* answers the field, never the answer itself.
                ["field", name, source] => {
                    map_declared = true;
                    if !MapView::FIELDS.contains(name) {
                        errors.refuse(
                            RefusalKind::UnadmittedFieldSpec,
                            format!(
                                "frame {id:?} line {number}: `field {name}` is not a field this \
                                 renderer samples; the fields are {:?}",
                                MapView::FIELDS
                            ),
                        );
                    } else if fields.iter().any(|field| field.name == *name) {
                        errors.refuse(
                            RefusalKind::MalformedRecord,
                            format!(
                                "frame {id:?} line {number}: `field {name}` is declared twice; a \
                                 field has one source"
                            ),
                        );
                    } else if !source_token(source) {
                        errors.refuse(
                            RefusalKind::MalformedRecord,
                            format!(
                                "frame {id:?} line {number}: `field {name}` takes a law name such \
                                 as `terrain::elevation_m`, found {source:?}"
                            ),
                        );
                    } else {
                        fields.push(FieldRef {
                            name: (*name).to_string(),
                            source: (*source).to_string(),
                        });
                    }
                }
                ["shaper", name, divisions] => {
                    map_declared = true;
                    let divisions = number_of(divisions);
                    if *name != MapView::SHAPER {
                        errors.refuse(
                            RefusalKind::UnadmittedFieldSpec,
                            format!(
                                "frame {id:?} line {number}: `shaper {name}` is not a cell law this \
                                 renderer carries; the law is {}",
                                MapView::SHAPER
                            ),
                        );
                    } else if shaper.is_some() {
                        errors.refuse(
                            RefusalKind::MalformedRecord,
                            format!(
                                "frame {id:?} line {number}: `shaper` is declared twice; a frame \
                                 states one cell law"
                            ),
                        );
                    } else if divisions != Some(32) {
                        errors.refuse(
                            RefusalKind::MalformedRecord,
                            format!(
                                "frame {id:?} line {number}: `shaper {}` takes 32 divisions per \
                                 axis, found {divisions:?}",
                                MapView::SHAPER
                            ),
                        );
                    } else {
                        shaper = Some(((*name).to_string(), 32));
                    }
                }
                ["projection", name] => {
                    map_declared = true;
                    if *name != MapView::PROJECTION {
                        errors.refuse(
                            RefusalKind::UnadmittedFieldSpec,
                            format!(
                                "frame {id:?} line {number}: `projection {name}` is not a camera \
                                 this renderer carries; the projection is {}",
                                MapView::PROJECTION
                            ),
                        );
                    } else if projection.is_some() {
                        errors.refuse(
                            RefusalKind::MalformedRecord,
                            format!(
                                "frame {id:?} line {number}: `projection` is declared twice; a \
                                 frame is drawn through one camera"
                            ),
                        );
                    } else {
                        projection = Some((*name).to_string());
                    }
                }
                ["centre", x, z] => {
                    map_declared = true;
                    match (number_f64(x), number_f64(z)) {
                        (Some(x), Some(z)) if centre.is_none() => centre = Some([x, z]),
                        (Some(_), Some(_)) => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `centre` is declared twice; a \
                                     view is centred on one column"
                                ),
                            );
                        }
                        _ => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `centre` is \
                                     `centre<TAB>x<TAB>z` in metres, found {trimmed:?}"
                                ),
                            );
                        }
                    }
                }
                ["cell", metres] => {
                    map_declared = true;
                    match number_f64(metres) {
                        Some(m) if m > 0.0 && cell_m.is_none() => cell_m = Some(m),
                        Some(_) => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `cell` is zero, negative, or \
                                     already declared; one cell is one positive number of metres"
                                ),
                            );
                        }
                        None => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `cell` takes a positive number \
                                     of metres, found {metres:?}"
                                ),
                            );
                        }
                    }
                }
                ["extent", cols_span, rows_span] => {
                    map_declared = true;
                    match (number_of(cols_span), number_of(rows_span)) {
                        (Some(cols_span), Some(rows_span))
                            if cols_span > 0 && rows_span > 0 && extent.is_none() =>
                        {
                            extent = Some([cols_span, rows_span])
                        }
                        (Some(_), Some(_)) => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `extent` is empty or already \
                                     declared; a map is at least one cell and stated once"
                                ),
                            );
                        }
                        _ => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `extent` is \
                                     `extent<TAB>cols<TAB>rows` with whole numbers, found \
                                     {trimmed:?}"
                                ),
                            );
                        }
                    }
                }
                ["pitch", degrees] => {
                    map_declared = true;
                    match number_f64(degrees) {
                        Some(pitch_deg)
                            if (-80.0..=-1.0).contains(&pitch_deg) && pitch.is_none() =>
                        {
                            pitch = Some(pitch_deg)
                        }
                        Some(_) => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `pitch` is declared twice or \
                                     outside −80°..−1°; a map camera looks down and never at the \
                                     pole"
                                ),
                            );
                        }
                        None => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `pitch` takes a downward tilt \
                                     in degrees, found {degrees:?}"
                                ),
                            );
                        }
                    }
                }
                [key, value] | [key, value, ..] => match *key {
                    "window" => match *value {
                        "" => {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!("frame {id:?} line {number}: `window` takes a window id"),
                            );
                        }
                        id => {
                            if window.is_some() {
                                errors.refuse(
                                    RefusalKind::MalformedRecord,
                                    format!(
                                        "frame {id:?} line {number}: `window` is declared twice; \
                                         a frame addresses one window"
                                    ),
                                );
                            }
                            window = Some(id.to_string());
                        }
                    },
                    "cols" | "rows" | "order" => {
                        let taken = match *key {
                            "cols" => cols.is_some(),
                            "rows" => rows.is_some(),
                            _ => order.is_some(),
                        };
                        if taken {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `{key}` is declared twice, and a \
                                     key states one value"
                                ),
                            );
                            continue;
                        }
                        match number_of(value) {
                            Some(number) => match *key {
                                "cols" => cols = Some(number),
                                "rows" => rows = Some(number),
                                _ => order = Some(number),
                            },
                            None => {
                                errors.refuse(
                                    RefusalKind::MalformedRecord,
                                    format!(
                                        "frame {id:?} line {number}: `{key}` takes a whole number, \
                                         not {value:?}"
                                    ),
                                );
                            }
                        }
                    }
                    "feature" => {
                        if feature.is_some() {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `feature` is declared twice; a \
                                     frame shows one view"
                                ),
                            );
                            continue;
                        }
                        if feature_token(value) {
                            feature = Some((*value).to_string());
                        } else {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `feature` takes a lowercase \
                                     world-feature token such as `all`, `ocean` or `mountain`, \
                                     found {value:?}"
                                ),
                            );
                        }
                    }
                    "requires" => {
                        if requires.is_some() {
                            errors.refuse(
                                RefusalKind::MalformedRecord,
                                format!(
                                    "frame {id:?} line {number}: `requires` is declared twice; a \
                                     frame states its prerequisites once"
                                ),
                            );
                            continue;
                        }
                        if *value == "none" {
                            requires = Some(Vec::new());
                            continue;
                        }
                        let mut refused = false;
                        let mut named = Vec::new();
                        for dependency in value.split(',') {
                            if !crate::layer::row_identifier(dependency) {
                                refused = true;
                                errors.refuse(
                                    RefusalKind::MalformedRecord,
                                    format!(
                                        "frame {id:?} line {number}: `requires` takes frame ids \
                                         or `none`, and {dependency:?} is not an id"
                                    ),
                                );
                            } else {
                                named.push(dependency.to_string());
                            }
                        }
                        if !refused {
                            requires = Some(named);
                        }
                    }
                    other => {
                        errors.refuse(
                            RefusalKind::UnadmittedFieldSpec,
                            format!(
                                "frame {id:?} line {number} declares key {other:?}, which is not in \
                                 the grammar; the required keys are {KEYS:?} and the optional \
                                 key is {OPTIONAL_KEYS:?}"
                            ),
                        );
                    }
                },
                _ => {
                    errors.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "frame {id:?} line {number}: {trimmed:?} is not a key row, a `fill` row \
                             or a `run` row"
                        ),
                    );
                }
            }
        }

        let missing: Vec<&str> = [
            (window.is_none(), "window"),
            (cols.is_none(), "cols"),
            (rows.is_none(), "rows"),
            (order.is_none(), "order"),
            (requires.is_none(), "requires"),
        ]
        .into_iter()
        .filter_map(|(absent, key)| absent.then_some(key))
        .collect();
        for key in missing {
            errors.refuse(
                RefusalKind::MalformedRecord,
                format!("frame {id:?} declares no `{key}`, and every key is required"),
            );
        }

        let width = cols.unwrap_or(0);
        let height = rows.unwrap_or(0);
        for (label, limit, declared) in [("cols", MAX_COLS, width), ("rows", MAX_ROWS, height)] {
            if declared > limit {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "frame {id:?} declares {label} {declared}; the ceiling is {limit}"
                    ),
                );
            }
        }
        if width > 0 && height > 0 && width.saturating_mul(height) > MAX_CELLS {
            errors.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "frame {id:?} declares {} cells ({width} × {height}); the ceiling is \
                     {MAX_CELLS}",
                    width.saturating_mul(height)
                ),
            );
        }
        // **The GPU declaration, checked as one record.** A map frame is drawn by the terrain
        // renderer, so its fields, law, camera and extent are all required together: a frame that
        // named `orthographic` and no elevation is not a map with a default, it is a declaration
        // missing the thing it draws.
        let map = if map_declared {
            for name in MapView::FIELDS {
                if !fields.iter().any(|field| field.name == name) {
                    errors.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "frame {id:?} declares a GPU map but no `field {name}`; the renderer \
                             samples both {:?}",
                            MapView::FIELDS
                        ),
                    );
                }
            }
            if shaper.is_none() {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "frame {id:?} declares a GPU map but no `shaper {} 32`; a map cell is a \
                         per-axis cell and the law is what shapes it",
                        MapView::SHAPER
                    ),
                );
            }
            if projection.is_none() {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "frame {id:?} declares a GPU map but no `projection {}`",
                        MapView::PROJECTION
                    ),
                );
            }
            for (absent, key) in [
                (centre.is_none(), "centre"),
                (cell_m.is_none(), "cell"),
                (extent.is_none(), "extent"),
                (pitch.is_none(), "pitch"),
            ] {
                if absent {
                    errors.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "frame {id:?} declares a GPU map but no `{key}`; a view states where \
                             it looks, how wide one cell is, and how far"
                        ),
                    );
                }
            }
            if let Some([cols_span, rows_span]) = extent {
                if cols_span > MAX_COLS || rows_span > MAX_ROWS {
                    errors.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "frame {id:?} declares a {cols_span} × {rows_span} cell map; the \
                             ceiling is {MAX_COLS} × {MAX_ROWS}"
                        ),
                    );
                }
            }
            match (shaper, projection, centre, cell_m, extent, pitch) {
                (
                    Some((shaper, shaper_cells)),
                    Some(projection),
                    Some(centre_xz),
                    Some(cell_m),
                    Some(extent),
                    Some(pitch_deg),
                ) => Some(MapView {
                    fields,
                    shaper,
                    shaper_cells,
                    projection,
                    centre_xz,
                    cell_m,
                    extent,
                    pitch_deg,
                }),
                _ => None,
            }
        } else {
            None
        };
        if fills.is_empty() && runs.is_empty() && map.is_none() {
            errors.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "frame {id:?} carries no `fill`, no `run` and no GPU map, so it draws nothing; \
                     a window whose content is erased says so with an empty declaration, not with \
                     an empty frame"
                ),
            );
        }
        for fill in &fills {
            if fill.cols == 0 {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "frame {id:?}: `fill` at ({}, {}) is zero cells wide",
                        fill.col, fill.row
                    ),
                );
            }
            if width > 0 && (fill.col >= width || fill.row >= height) {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "frame {id:?}: `fill` starts at ({}, {}) and the grid is {width} × {height}",
                        fill.col, fill.row
                    ),
                );
            } else if width > 0 && fill.col.saturating_add(fill.cols) > width {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "frame {id:?}: `fill` from ({}, {}) runs {} cells and the grid is {width} \
                         wide",
                        fill.col, fill.row, fill.cols
                    ),
                );
            }
        }
        for run in &runs {
            let glyphs = run.text.chars().count() as u32;
            if width > 0 && (run.col >= width || run.row >= height) {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "frame {id:?}: `run` starts at ({}, {}) and the grid is {width} × {height}",
                        run.col, run.row
                    ),
                );
            } else if width > 0 && run.col.saturating_add(glyphs) > width {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "frame {id:?}: `run` from ({}, {}) is {glyphs} glyphs and the grid is \
                         {width} wide; a run the frame cannot hold is a cut nobody stated",
                        run.col, run.row
                    ),
                );
            }
        }

        errors.into_result(Frame {
            id,
            window: window.unwrap_or_default(),
            cols: width,
            rows: height,
            order: order.unwrap_or(0),
            requires: requires.unwrap_or_default(),
            feature,
            map,
            fills,
            runs,
        })
    }

    /// **The cells as rows of glyphs**, for a reader and for the rendered block.
    ///
    /// Paper does not print: a fill is a surface, and a picture of a cell grid is its glyphs. A run
    /// is clipped to the frame's own width rather than refused here — [`Frame::parse`] already
    /// refused a run the frame cannot hold, so this cannot cut a declaration, only a rendering.
    pub fn picture(&self) -> Vec<String> {
        let mut rows = vec![vec![' '; self.cols as usize]; self.rows as usize];
        for run in &self.runs {
            if run.row as usize >= rows.len() {
                continue;
            }
            for (offset, glyph) in run.text.chars().enumerate() {
                let col = run.col as usize + offset;
                if col >= self.cols as usize {
                    break;
                }
                rows[run.row as usize][col] = glyph;
            }
        }
        rows.into_iter().map(|cells| cells.into_iter().collect()).collect()
    }

    /// The frame as one line: `MAP-0001 · map-plan · 56 × 22 · 4 fills · 18 runs`.
    ///
    /// A frame that declares a world feature says so, because two frames addressed to the
    /// same window are otherwise one line each with nothing to tell them apart.
    pub fn describe(&self) -> String {
        let feature = match &self.feature {
            Some(feature) => format!(" · feature {feature}"),
            None => String::new(),
        };
        let map = match &self.map {
            Some(map) => format!(
                " · {} {} · {} field(s) · {} m/cell {} × {} · pitch {}°",
                map.projection,
                map.shaper,
                map.fields.len(),
                map.cell_m,
                map.extent[0],
                map.extent[1],
                map.pitch_deg
            ),
            None => String::new(),
        };
        format!(
            "{} · {} · {} × {} · {} fills · {} runs{feature}{map}",
            self.id,
            self.window,
            self.cols,
            self.rows,
            self.fills.len(),
            self.runs.len()
        )
    }

    /// **The human-readable block, from the two files and nothing else.**
    ///
    /// The first line names the frame, its window and its extent; the fenced block repeats every
    /// field in grammar order, so a reader can check the declaration against the file; the record's
    /// four sections follow verbatim. A record missing one of [`patch::SECTIONS`] is refused by name.
    pub fn render(&self, record: &str) -> Result<String, RefusalSet> {
        let mut errors = RefusalSet::new();
        let mut sections: Vec<(&'static str, String)> = Vec::new();
        for heading in patch::SECTIONS {
            match section_of(record, heading) {
                Some(body) if !body.trim().is_empty() => sections.push((heading, body)),
                Some(_) => {
                    errors.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "frame {}'s record has an empty `{heading}` section, and a heading with \
                             nothing under it is a heading that was not written",
                            self.id
                        ),
                    );
                }
                None => {
                    errors.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "frame {}'s record declares no `{heading}` section; every record \
                             carries all four",
                            self.id
                        ),
                    );
                }
            }
        }
        errors.into_result(self.render_with(&sections))
    }

    fn render_with(&self, sections: &[(&'static str, String)]) -> String {
        let requires = if self.requires.is_empty() {
            "none".to_string()
        } else {
            self.requires.join(",")
        };
        let mut out = format!(
            "api-cellframe · {} · window {} · {} × {} cells · fills {} · runs {}\n\n```api-cellframe\n",
            self.id,
            self.window,
            self.cols,
            self.rows,
            self.fills.len(),
            self.runs.len()
        );
        out.push_str(&format!("id          {}\n", self.id));
        out.push_str(&format!("window      {}\n", self.window));
        out.push_str(&format!("cols        {}\n", self.cols));
        out.push_str(&format!("rows        {}\n", self.rows));
        out.push_str(&format!("order       {}\n", self.order));
        out.push_str(&format!("requires    {requires}\n"));
        if let Some(feature) = &self.feature {
            out.push_str(&format!("feature     {feature}\n"));
        }
        if let Some(map) = &self.map {
            for field in &map.fields {
                out.push_str(&format!(
                    "field       {} {}\n",
                    field.name, field.source
                ));
            }
            out.push_str(&format!(
                "shaper      {} {}\nprojection  {}\ncentre      {} {}\ncell        {}\nextent      {} {}\npitch       {}\n",
                map.shaper,
                map.shaper_cells,
                map.projection,
                map.centre_xz[0],
                map.centre_xz[1],
                map.cell_m,
                map.extent[0],
                map.extent[1],
                map.pitch_deg
            ));
        }
        for fill in &self.fills {
            out.push_str(&format!(
                "fill        {} {} {} {}\n",
                fill.col,
                fill.row,
                fill.cols,
                fill.paper.name()
            ));
        }
        for ink in Ink::ALL {
            let runs = self.runs.iter().filter(|run| run.ink == ink).count();
            if runs > 0 {
                out.push_str(&format!("ink         {} {}\n", ink.name(), runs));
            }
        }
        out.push_str("```\n\n");
        // **The cells as the picture they are**, not three hundred run rows: a reader checks a map
        // by looking at it, and a dump of `run 12 4 divider ...` is data no one reads.
        for row in self.picture() {
            out.push_str(row.trim_end());
            out.push('\n');
        }
        for (heading, body) in sections {
            out.push('\n');
            out.push_str(heading);
            out.push('\n');
            out.push_str(body.trim_end());
            out.push('\n');
        }
        out
    }
}

/// A frame and the record that explains it, as a folder holds them.
#[derive(Clone, PartialEq, Debug)]
pub struct Declared {
    /// The parsed frame.
    pub frame: Frame,
    /// The record's text, exactly as `frames/<id>.md` holds it.
    pub record: String,
    /// The `.frame` file this was read from.
    pub file: PathBuf,
}

/// A display package's frames, checked against the recipe that names them.
#[derive(Clone, PartialEq, Debug)]
pub struct Package {
    /// The folder this was read from; empty when the frames came from a package's artifact map.
    pub folder: PathBuf,
    /// The frames the recipe names, each with its record, in id order.
    pub frames: Vec<Declared>,
}

/// **Every frame in a display package, with its record**, checked against the package's recipe.
///
/// The recipe is **passed in rather than read again**: the caller has already parsed it (the mount
/// at its package's load, `voxel-layers` at its render), and a second parse would be a second answer
/// to what the package is.
///
/// **Nine refusals are this reader's own, beyond the grammar's row checks**: an unreadable
/// `frames/`, a frame that is not UTF-8, a file whose stem is not its frame's id, a frame the recipe
/// names by no row, a window the package does not declare, a grid over that window's declared
/// extent, a missing record, a record missing one of the four sections, and a record with no frame.
/// The five bullets below are the join's, which is the half a folder adds to the grammar:
///
/// * a `.frame` with no `frames/<id>.md` is refused by path — the machine part without the record is
///   half a declaration;
/// * a `.md` with no `.frame` is refused too, because a record that explains a frame the package does
///   not carry is prose about nothing;
/// * a frame the recipe names by **no row** is refused, because a frame nothing names is a frame no
///   reader can find and no gate can check (M10.1 item 3);
/// * a frame addressed to a window the package **does not declare** is refused by both ids, because
///   there is no content space for it to draw in;
/// * a frame larger than that window's declared extent is refused by cell count, so a frame cannot
///   reach past the window it belongs to.
pub fn read(folder: &Path, recipe: &crate::recipe::Recipe) -> Result<Package, RefusalSet> {
    let artifacts = match crate::recipe::artifacts_in(folder) {
        Ok(artifacts) => artifacts,
        Err(detail) => {
            let mut errors = RefusalSet::new();
            errors.refuse(RefusalKind::MalformedRecord, detail);
            return Err(errors);
        }
    };
    let mut package = read_artifacts(&artifacts, recipe)?;
    package.folder = folder.to_path_buf();
    // Keep the on-disk path a folder reader has always reported; the map's key stays relative.
    for declared in &mut package.frames {
        declared.file = folder.join(&declared.file);
    }
    Ok(package)
}

/// **The same join, read from the files a compiled package carries.**
///
/// A mounted package grades its cell frames from the artifact map `recipe::load` produced, so a
/// shipped frame is checked by exactly the rules a folder is: the file names its frame, the recipe
/// names it by a row, the window is declared, the grid fits, and the record carries the four
/// sections. **There is no second reader and no second grammar.**
pub fn read_artifacts(
    artifacts: &BTreeMap<String, Vec<u8>>,
    recipe: &crate::recipe::Recipe,
) -> Result<Package, RefusalSet> {
    let mut errors = RefusalSet::new();
    let mut names: Vec<&String> = artifacts
        .keys()
        .filter(|key| {
            key.strip_prefix("frames/")
                .is_some_and(|name| name.ends_with(".frame"))
        })
        .collect();
    names.sort();

    let mut declared = Vec::new();
    for key in names {
        let key = key.as_str();
        let file = Path::new(key);
        let name = key.strip_prefix("frames/").unwrap_or(key).to_string();
        let text = match std::str::from_utf8(artifacts.get(key).map(Vec::as_slice).unwrap_or_default())
        {
            Ok(text) => text,
            Err(why) => {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!("{key} is not UTF-8 text ({why})"),
                );
                continue;
            }
        };
        let frame = match Frame::parse(text) {
            Ok(frame) => frame,
            Err(found) => {
                for refusal in found.all() {
                    errors.refuse(
                        refusal.kind(),
                        format!("{}: {}", file.display(), refusal.detail()),
                    );
                }
                continue;
            }
        };
        let stem = name.trim_end_matches(".frame");
        if stem != frame.id {
            errors.refuse(
                RefusalKind::MalformedRecord,
                format!(
                    "{} declares frame {:?}: a file names its own frame, and a frame named twice is \
                     a frame a reader will look for in the wrong place",
                    file.display(),
                    frame.id
                ),
            );
        }
        if !recipe.rows.iter().any(|row| row.id == frame.id) {
            errors.refuse(
                RefusalKind::UnadmittedFieldSpec,
                format!(
                    "{} declares frame {} but the package's recipe names it by no row; a frame the \
                     recipe cannot name is a frame the package does not carry",
                    file.display(),
                    frame.id
                ),
            );
        }
        match recipe.windows.iter().find(|window| window.id == frame.window) {
            None => {
                errors.refuse(
                    RefusalKind::UndeclaredName,
                    format!(
                        "{} addresses window {:?}, which the package does not declare; the cell \
                         format may draw only in a declared window's content space",
                        file.display(),
                        frame.window
                    ),
                );
            }
            Some(window) => {
                let width = window.geometry[2];
                let height = window.geometry[3];
                let needed_w = frame.cols.saturating_mul(CELL_W_DP);
                let needed_h = frame.rows.saturating_mul(CELL_H_DP);
                if needed_w > width || needed_h > height {
                    errors.refuse(
                        RefusalKind::MalformedRecord,
                        format!(
                            "{} declares {} × {} cells ({} × {} dp) and window {:?} is declared \
                             {} × {} dp; a frame larger than its window is drawn through chrome",
                            file.display(),
                            frame.cols,
                            frame.rows,
                            needed_w,
                            needed_h,
                            frame.window,
                            width,
                            height
                        ),
                    );
                }
            }
        }
        // **A `/`-separated key, not `Path::join`.** The artifact map is package-relative and uses
        // `/` on every host; a Windows separator here would look for a file the package never has.
        let record_key = format!("frames/{}.md", frame.id);
        let record = match artifacts
            .get(&record_key)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        {
            Some(record) => record.to_string(),
            None => {
                errors.refuse(
                    RefusalKind::MalformedRecord,
                    format!(
                        "{} declares frame {} and {} is not in the package; the machine part \
                         without the record is half a declaration",
                        file.display(),
                        frame.id,
                        record_key
                    ),
                );
                continue;
            }
        };
        if let Err(found) = frame.render(&record) {
            for refusal in found.all() {
                errors.refuse(
                    refusal.kind(),
                    format!("{record_key}: {}", refusal.detail()),
                );
            }
            continue;
        }
        declared.push(Declared {
            frame,
            record,
            file: file.to_path_buf(),
        });
    }

    // **A record with no frame is half a declaration too**, and it is the half a reader is most
    // likely to trust: a `.md` that explains a frame nobody can mount reads as a feature.
    let mut strays: Vec<&String> = artifacts
        .keys()
        .filter(|key| key.starts_with("frames/") && key.ends_with(".md"))
        .filter(|key| {
            let name = key.strip_prefix("frames/").unwrap_or(key);
            let stem = name.trim_end_matches(".md");
            !declared.iter().any(|entry| entry.frame.id == stem)
        })
        .collect();
    strays.sort();
    for stray in strays {
        errors.refuse(
            RefusalKind::MalformedRecord,
            format!(
                "{stray} is a frame's record and no `frames/<id>.frame` declares that frame; prose \
                 about a frame the package does not carry is a record of nothing"
            ),
        );
    }

    declared.sort_by(|left, right| left.frame.id.cmp(&right.frame.id));
    errors.into_result(Package {
        folder: PathBuf::new(),
        frames: declared,
    })
}

/// The text under one heading, up to the next heading of the same kind — or `None`.
fn section_of(record: &str, heading: &str) -> Option<String> {
    let lines: Vec<&str> = record.lines().collect();
    let start = lines.iter().position(|line| line.trim_end() == heading)?;
    let mut body = Vec::new();
    for line in lines.iter().skip(start + 1) {
        if patch::SECTIONS.contains(&line.trim_end()) {
            break;
        }
        body.push(*line);
    }
    Some(body.join("\n"))
}

/// **A world-feature token**: lowercase ASCII, digits and a hyphen, at most 24 characters.
///
/// The narrowness is the point: this token is typed at the console (`plan ocean`), so a value
/// with a space in it would be a selection no verb could name and a value with capitals would be
/// a second spelling of the same feature.
fn feature_token(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 24
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// A whole number, or `None` — the one reading every count in the grammar uses.
fn number_of(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// A signed decimal, or `None`. The map's centre, cell size and pitch are the only numbers in this
/// grammar that are not counts, and `-0` is a value rather than a typo.
fn number_f64(text: &str) -> Option<f64> {
    if text.is_empty()
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E'))
    {
        return None;
    }
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// A law's name: `terrain::elevation_m`. No whitespace, because a row splits on tabs and a source
/// with a space in it would be two tokens the grammar did not declare.
fn source_token(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 96
        && text.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'.' | b'-' | b'/')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(extra: &str) -> String {
        format!(
            "{HEADER}\nframe\tMAP-0001\n\twindow\tmap-plan\n\tcols\t8\n\trows\t2\n\torder\t10\n\t\
             requires\tnone\n{extra}"
        )
    }

    fn refusals(source: &str) -> Vec<String> {
        Frame::parse(source)
            .expect_err("the declaration is not this build's")
            .all()
            .iter()
            .map(|refusal| refusal.detail().to_string())
            .collect()
    }

    #[test]
    fn a_frame_parses_every_key_and_both_row_kinds() {
        let parsed = Frame::parse(&frame(
            "fill\t0\t0\t8\tsurface\nrun\t1\t1\taccent\ttopo\nrun\t5\t1\tdim\t--\n",
        ))
        .expect("a well-formed frame");
        assert_eq!(parsed.id, "MAP-0001");
        assert_eq!(parsed.window, "map-plan");
        assert_eq!((parsed.cols, parsed.rows), (8, 2));
        assert_eq!(parsed.fills.len(), 1);
        assert_eq!(parsed.runs.len(), 2);
        assert_eq!(parsed.runs[0].ink, Ink::Accent);
        assert_eq!(parsed.runs[1].text, "--");
        assert_eq!(
            parsed.describe(),
            "MAP-0001 · map-plan · 8 × 2 · 1 fills · 2 runs"
        );
    }

    #[test]
    fn a_frame_may_declare_the_world_feature_it_shows_and_the_key_is_optional() {
        let parsed = Frame::parse(&frame(
            "\tfeature\tocean\nfill\t0\t0\t8\tsurface\nrun\t1\t1\taccent\ttopo\n",
        ))
        .expect("a frame with a feature parses");
        assert_eq!(parsed.feature.as_deref(), Some("ocean"));
        assert!(
            parsed.describe().ends_with("· feature ocean"),
            "the description does not name the view: {}",
            parsed.describe()
        );

        // It is optional: the fixture every other test uses declares none.
        let plain = Frame::parse(&frame("fill\t0\t0\t8\tsurface\nrun\t1\t1\taccent\ttopo\n"))
            .expect("a frame without a feature parses");
        assert_eq!(plain.feature, None);

        let twice = refusals(&frame(
            "\tfeature\tocean\n\tfeature\tmountain\nfill\t0\t0\t8\tsurface\n",
        ));
        assert!(
            twice.iter().any(|detail| detail.contains("`feature` is declared twice")),
            "{twice:?}"
        );
        let bad = refusals(&frame("\tfeature\tOcean Reef\nfill\t0\t0\t8\tsurface\n"));
        assert!(
            bad.iter().any(|detail| detail.contains("lowercase world-feature token")),
            "{bad:?}"
        );
    }

    #[test]
    fn an_unknown_key_is_refused_by_name() {
        let found = refusals(&frame("fill\t0\t0\t8\tsurface\ncolour\tblue\n"));
        assert!(
            found.iter().any(|detail| detail.contains("\"colour\"")
                && detail.contains("not in the grammar")),
            "{found:?}"
        );
    }

    #[test]
    fn a_duplicate_key_is_refused_by_name() {
        let found = refusals(&frame("fill\t0\t0\t8\tsurface\n\torder\t2\n"));
        assert!(
            found.iter().any(|detail| detail.contains("`order` is declared twice")),
            "{found:?}"
        );
    }

    #[test]
    fn every_required_key_is_required() {
        for key in KEYS {
            let mut source = frame("fill\t0\t0\t8\tsurface\n");
            let needle = format!("\t{key}\t");
            source = source
                .lines()
                .filter(|line| !line.starts_with(&needle))
                .collect::<Vec<_>>()
                .join("\n");
            let found = refusals(&source);
            assert!(
                found
                    .iter()
                    .any(|detail| detail.contains(&format!("declares no `{key}`"))),
                "dropping {key} was not refused: {found:?}"
            );
        }
    }

    #[test]
    fn a_row_outside_the_grid_is_refused_with_its_cell() {
        let found = refusals(&frame("run\t6\t0\ttext\tfour\n"));
        assert!(
            found.iter().any(|detail| detail.contains("4 glyphs") && detail.contains("8 wide")),
            "{found:?}"
        );
        let found = refusals(&frame("fill\t0\t2\t8\tsurface\n"));
        assert!(
            found.iter().any(|detail| detail.contains("starts at (0, 2)")),
            "{found:?}"
        );
    }

    #[test]
    fn a_frame_that_draws_nothing_is_refused() {
        let found = refusals(&frame(""));
        assert!(
            found.iter().any(|detail| detail.contains("draws nothing")),
            "{found:?}"
        );
    }

    #[test]
    fn an_old_schema_or_a_recipe_header_is_refused() {
        for header in ["shploingus.cellframe/0", "shploingus.recipe/3"] {
            let source = frame("fill\t0\t0\t8\tsurface\n")
                .replacen(HEADER, header, 1);
            let found = refusals(&source);
            assert!(
                found.iter().any(|detail| detail.contains("unknown cellframe schema")),
                "{header}: {found:?}"
            );
        }
    }

    #[test]
    fn a_role_outside_the_grammar_is_refused_by_word() {
        let found = refusals(&frame("run\t0\t0\tscarlet\thi\n"));
        assert!(
            found.iter().any(|detail| detail.contains("\"scarlet\"") || detail.contains("scarlet")),
            "{found:?}"
        );
    }

    #[test]
    fn the_render_is_byte_stable_and_carries_every_field() {
        let parsed = Frame::parse(&frame(
            "fill\t0\t0\t8\tsurface\nrun\t1\t1\taccent\ttopo\n",
        ))
        .unwrap();
        let record = "# Change\n\nreasons.\n\n# Acceptance\n\ntested.\n\n# Evidence\n\nread.\n\n# Rollback\n\ndeleted.\n";
        let first = parsed.render(record).expect("a complete record");
        let second = parsed.render(record).expect("a complete record");
        assert_eq!(first, second);
        for needle in [
            "api-cellframe · MAP-0001",
            "window      map-plan",
            "cols        8",
            "rows        2",
            "fill        0 0 8 surface",
            "ink         accent 1",
            " topo",
            "requires    none",
            "# Rollback",
        ] {
            assert!(first.contains(needle), "{needle} is missing from\n{first}");
        }
    }

    #[test]
    fn a_record_missing_a_section_is_refused_by_the_section() {
        let parsed = Frame::parse(&frame("fill\t0\t0\t8\tsurface\n")).unwrap();
        let record = "# Change\n\nreasons.\n\n# Acceptance\n\ntested.\n\n# Evidence\n\nread.\n";
        let found = parsed
            .render(record)
            .expect_err("no rollback section")
            .all()
            .iter()
            .map(|refusal| refusal.detail().to_string())
            .collect::<Vec<_>>();
        assert!(
            found.iter().any(|detail| detail.contains("# Rollback")),
            "{found:?}"
        );
    }

    fn recipe() -> crate::recipe::Recipe {
        crate::recipe::Recipe::parse(
            b"shploingus.recipe/3\n\
              package\talpha-shell\tv1\n\
              row\tMAP-0001\tthe map window\n\
              window\tmap-plan\n\
              \ttitle\tMap\n\
              \tcontent\tmap-plan\n\
              \tlane\tframe\n\
              \textent\tsquare-pixels\n\
              \tgeometry\t0 0 480 384\n\
              \tmin-size\t240 192\n\
              \topacity\t930\n\
              \televation\t1\n\
              \ttheme-role\tsurface\n\
              \tpinnable\tfalse\n\
              \tlayout\tclose\n\
              \tbehavior\tclose\n\
              \trecord-id\twindow-map-plan\n",
        )
        .expect("the test recipe parses")
    }

    fn packaged() -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([
            (
                "frames/MAP-0001.frame".to_string(),
                frame("fill\t0\t0\t8\tsurface\n").into_bytes(),
            ),
            (
                "frames/MAP-0001.md".to_string(),
                b"# Change\n\ncells.\n\n# Acceptance\n\ntested.\n\n# Evidence\n\nread.\n\n# Rollback\n\ndeleted.\n"
                    .to_vec(),
            ),
        ])
    }

    #[test]
    fn the_same_frame_grades_from_the_package_map_as_from_a_folder() {
        let package = read_artifacts(&packaged(), &recipe()).expect("a shipped frame grades");
        assert_eq!(package.frames.len(), 1);
        assert_eq!(package.frames[0].frame.id, "MAP-0001");
        assert_eq!(package.frames[0].file, PathBuf::from("frames/MAP-0001.frame"));
    }

    #[test]
    fn a_package_frame_missing_its_record_or_naming_an_undeclared_window_is_refused() {
        let mut missing = packaged();
        missing.remove("frames/MAP-0001.md");
        let refusals = read_artifacts(&missing, &recipe()).expect_err("half a declaration");
        assert!(
            refusals
                .all()
                .iter()
                .any(|refusal| refusal.detail().contains("half a declaration")),
            "{refusals}"
        );

        let mut ghost = packaged();
        ghost.insert(
            "frames/MAP-0001.frame".to_string(),
            frame("fill\t0\t0\t8\tsurface\n")
                .replace("\twindow\tmap-plan", "\twindow\tno-such-window")
                .into_bytes(),
        );
        let refusals = read_artifacts(&ghost, &recipe()).expect_err("no content space");
        assert!(
            refusals
                .all()
                .iter()
                .any(|refusal| refusal.detail().contains("does not declare")),
            "{refusals}"
        );
    }

    #[test]
    fn the_cells_are_one_number_between_the_format_and_the_drawing() {
        // The client's constants are the same two numbers; the test that proves it lives where the
        // raster is, so this one states the format's half without reaching into the UI crate.
        assert_eq!((CELL_W_DP, CELL_H_DP), (8, 16));
        assert_eq!(MAX_CELLS, MAX_COLS * MAX_ROWS / 16);
    }
}
