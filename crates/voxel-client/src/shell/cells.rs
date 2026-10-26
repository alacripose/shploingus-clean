//! **The shell's cell raster: every interface element becomes cells, and every colour lands on the
//! Android 4 schema.**
//!
//! # Where this comes from
//!
//! `ala_package_designer`'s shell (`crates/foundry-shell/shell/`) draws its floating windows as a
//! **cell surface**: an 8 × 16 pixel cell grid between the action bar and the system bar, with one
//! function placing windows and one raster painting them. That shell's `place.go` header states the
//! rule this module keeps: *"One function decides it and nothing here draws."* The placement and the
//! paint are separate, so the raster can change without moving a window.
//!
//! This module is that raster, and **the shell's own draw path crosses it**: the app composes
//! [`Element`]s — surfaces, rules and text — and [`surface_at`] turns the whole list into cell runs
//! for the version the package declared. It is a second layout, and that is stated rather than
//! hidden: it moves a glyph the views sized and a rule the chrome placed (PATCH-071's table is the
//! measured cost). What every shell element does cross is [`retinted`], which is the schema and only
//! the schema.
//!
//! # Which drawing, and where the package says so
//!
//! A drawing is **versioned** ([`CellVersion`]), and the version is the **package's declaration**, not
//! a constant in the binary ([`declare`], `package.cells`): `cells 1` is the Holo Dark raster and
//! `cells 0` is the old drawing, the composed elements handed on unchanged. So *"bring the old
//! version back"* is one line in a package, and a package that writes the old **declaration** header
//! (`shploingus.cells/0`) is **refused by name with its file and line**, printed once, rather than
//! silently substituted — the same rule `shploingus.recipe/3` states for an old recipe header.
//!
//! **The declaration is read where the package is mounted** ([`super::Mount::load_package`]), so a
//! package that arrives while the shell runs selects its drawing — or is refused, leaving the
//! drawing and arrangement already on screen untouched — by the same act that adopts it. A
//! declaration read only at boot would never be read for the package the loader exists to mount.
//!
//! # The schema
//!
//! **Android 4's Holo Dark**, declared as twelve values and nothing else. Every paint that reaches
//! this module is re-tinted to the nearest of them, so the shell's colours are the schema's rather
//! than a second palette that resembles it. The twelve are the Holo dark background, its panel
//! surface, chrome, the bright and dark blues, the three ink steps, the divider, and the status red,
//! green and orange. Nearest is measured in the bytes the paint carries — a perceptual space would be
//! a second colour model, and this crate already has one (`voxel_ui::Rgb::linear`) for contrast.
//!
//! **A role is not a colour here.** The theme rows still name `surface`, `hud` and `quiet`; the
//! raster answers what those are *on this schema*, which is the one place the answer exists.
//!
//! # What a cell is, and what it is not
//!
//! * A cell is **8 × 16 dp**, the designer shell's own number, and 14 sp of type fits one line of a
//!   cell. Text is shaped by the HUD's glyph pipeline in a monospace family, so a run written at cell
//!   *c* lands where the cell says it does.
//! * A **surface** fills cells with paper. A **rule** is one cell row of `─`, because a rule is a
//!   horizontal line and a cell cannot be thinner than its row.
//! * A **text** run starts at the cell its rect snaps to, and is clipped at the grid's right edge.
//!   **Nothing grows the grid**: a readout longer than its window is cut, not laid out past the pane.
//!
//! # Why the raster is a module in `voxel-client`, and the declaration is not
//!
//! The raster consumes `voxel_ui::Element` and is consumed by the shell's own draw path. A crate
//! boundary would put the schema where the UI crate cannot see it and invite a second palette; here
//! it sits beside the draw it repaints.
//!
//! **The declaration is not the raster's.** What a package may declare, which header is read, and
//! the refusal that names the file and line are the format's grammar, so they live in
//! [`voxel_package::cells`] and are re-exported here below. A GPU-free reader can now check a
//! package's declaration without this crate; this module answers only what version 1 looks like.

use voxel_ui::{Align, Element, Paint, Rect, Rgb};

/// One cell's width in device-independent pixels: the designer shell's own cell.
pub const CELL_W_DP: f64 = 8.0;

/// One cell's height in device-independent pixels: the designer shell's own cell.
pub const CELL_H_DP: f64 = 16.0;

/// The type size a cell's line can hold, at the HUD's line box.
pub const CELL_TYPE_DP: f64 = 12.8;

// **The declaration moved to the format crate, and this is the one path that keeps its callers
// compiling.** `voxel_package::cells` owns `package.cells`: the header constants, the refusal
// variants, the reader, and the version vocabulary. This module owns the **raster** — what version 1
// draws — and re-exports the declaration unchanged, so `shell::cells::declare` is the format's own
// function rather than a copy of it.
pub use voxel_package::cells::{
    declare, CellRefusal, CellVersion, Declaration, DECLARATION_FILE, DECLARATION_HEADER,
    OLD_DECLARATION_HEADER,
};

/// Holo Dark's background — the schema's black.
pub const HOLO_BACKGROUND: Rgb = Rgb::new(0x00, 0x00, 0x00);
/// Holo Dark's window surface.
pub const HOLO_SURFACE: Rgb = Rgb::new(0x1A, 0x1A, 0x1A);
/// Holo Dark's chrome, one step under the surface.
pub const HOLO_CHROME: Rgb = Rgb::new(0x0D, 0x0D, 0x0D);
/// The schema's accent: Holo blue.
pub const HOLO_BLUE: Rgb = Rgb::new(0x33, 0xB5, 0xE5);
/// The accent's darker step, used by the schema for pressed and selected states.
pub const HOLO_BLUE_DARK: Rgb = Rgb::new(0x00, 0x99, 0xCC);
/// Primary ink.
pub const HOLO_TEXT: Rgb = Rgb::new(0xFF, 0xFF, 0xFF);
/// Secondary ink.
pub const HOLO_TEXT_DIM: Rgb = Rgb::new(0xB3, 0xB3, 0xB3);
/// Disabled ink.
pub const HOLO_TEXT_DISABLED: Rgb = Rgb::new(0x80, 0x80, 0x80);
/// The schema's divider.
pub const HOLO_DIVIDER: Rgb = Rgb::new(0x33, 0x33, 0x33);
/// The schema's status red.
pub const HOLO_RED: Rgb = Rgb::new(0xFF, 0x44, 0x44);
/// The schema's status green.
pub const HOLO_GREEN: Rgb = Rgb::new(0x99, 0xCC, 0x00);
/// The schema's status orange.
pub const HOLO_ORANGE: Rgb = Rgb::new(0xFF, 0xBB, 0x33);

/// **The Android 4 schema, as the eleven values it is.** A paint is retinted to the nearest of
/// these; nothing else in the shell states a colour.
pub const HOLO: [Rgb; 12] = [
    HOLO_BACKGROUND,
    HOLO_SURFACE,
    HOLO_CHROME,
    HOLO_BLUE,
    HOLO_BLUE_DARK,
    HOLO_TEXT,
    HOLO_TEXT_DIM,
    HOLO_TEXT_DISABLED,
    HOLO_DIVIDER,
    HOLO_RED,
    HOLO_GREEN,
    HOLO_ORANGE,
];

/// A paint the schema admits, with the alpha it arrived with.
///
/// **The nearest value wins, and distance is squared bytes.** A schema with no near answer is not a
/// refusal: every colour has a nearest, and the alternative is a shell that draws nothing.
pub fn retint(paint: Paint) -> Paint {
    let mut best = HOLO[0];
    let mut best_distance = i32::MAX;
    for value in HOLO {
        let dr = i32::from(paint.rgb.r) - i32::from(value.r);
        let dg = i32::from(paint.rgb.g) - i32::from(value.g);
        let db = i32::from(paint.rgb.b) - i32::from(value.b);
        let distance = dr * dr + dg * dg + db * db;
        if distance < best_distance {
            best_distance = distance;
            best = value;
        }
    }
    Paint { rgb: best, alpha: paint.alpha }
}

/// **The schema applied to a frame's elements, with every rect left where it was drawn.**
///
/// This is the whole of what the shell's draw path asks of this module: the paint lands on the nearest
/// Holo Dark value and the geometry is the views'.
pub fn retinted(elements: &[Element]) -> Vec<Element> {
    elements
        .iter()
        .map(|element| match element {
            Element::Surface { rect, paint } => Element::Surface {
                rect: *rect,
                paint: retint(*paint),
            },
            Element::Rule { rect, paint } => Element::Rule {
                rect: *rect,
                paint: retint(*paint),
            },
            Element::Text {
                rect,
                text,
                paint,
                size_dp,
                align,
            } => Element::Text {
                rect: *rect,
                text: text.clone(),
                paint: retint(*paint),
                size_dp: *size_dp,
                align: *align,
            },
        })
        .collect()
}

/// **A graded frame, placed in the content space of the window it was declared for.**
///
/// The frame is the format's own type ([`voxel_package::cellframe::Frame`]) and its `rect` is the
/// **origin of a declared window's content container**, computed by [`super::Mount::frame_space`]
/// from the same `window_body` arithmetic the window's own drawing uses. A frame is therefore
/// addressable nowhere but a content space: there is no constructor here that takes an arbitrary
/// rect, and the mount is the only producer.
pub struct Placed<'a> {
    /// The frame the package declared and the mount graded.
    pub frame: &'a voxel_package::cellframe::Frame,
    /// The window content container the frame is drawn from.
    pub rect: Rect,
}

impl Placed<'_> {
    /// **Where the frame's cells stand**: its own `cols × rows`, from the content origin.
    ///
    /// A frame smaller than its window leaves the rest of the content to the composed elements, so
    /// the cells claim exactly the rect they declare and never the window's whole body.
    pub fn footprint(&self) -> Rect {
        Rect::new(
            self.rect.x,
            self.rect.y,
            f64::from(self.frame.cols) * CELL_W_DP,
            f64::from(self.frame.rows) * CELL_H_DP,
        )
    }
}

/// A paper role resolved against the schema. **The one place a role becomes a colour.**
pub fn paper_of(paper: voxel_package::cellframe::Paper) -> Paint {
    use voxel_package::cellframe::Paper;
    match paper {
        Paper::Surface => Paint::opaque(HOLO_SURFACE),
        Paper::Chrome => Paint::opaque(HOLO_CHROME),
        Paper::Background => Paint::opaque(HOLO_BACKGROUND),
        Paper::Divider => Paint::opaque(HOLO_DIVIDER),
        Paper::Transparent => Paint { rgb: HOLO_BACKGROUND, alpha: 0.0 },
    }
}

/// An ink role resolved against the schema. The same schema [`retint`] snaps every composed paint
/// to, so a frame's cells and the shell's elements cannot land on two palettes.
pub fn ink_of(ink: voxel_package::cellframe::Ink) -> Paint {
    use voxel_package::cellframe::Ink;
    Paint::opaque(match ink {
        Ink::Text => HOLO_TEXT,
        Ink::Dim => HOLO_TEXT_DIM,
        Ink::Disabled => HOLO_TEXT_DISABLED,
        Ink::Accent => HOLO_BLUE,
        Ink::Divider => HOLO_DIVIDER,
        Ink::Red => HOLO_RED,
        Ink::Green => HOLO_GREEN,
        Ink::Orange => HOLO_ORANGE,
    })
}

/// **A graded frame's cells, as elements at its own origin.**
///
/// This is the whole of what a frame draws, and it reuses the grid the connected module already
/// uses: paper runs first, then glyph runs, run-length encoded by [`Grid::elements`]. There is no
/// second renderer stage — the frame is cells, and the cells become the same element list the
/// shell's own composition produces.
pub fn frame(frame: &voxel_package::cellframe::Frame, origin: Rect) -> Vec<Element> {
    let mut grid = Grid::new(frame.cols as usize, frame.rows as usize);
    for fill in &frame.fills {
        grid.fill(
            fill.col as usize,
            fill.row as usize,
            fill.cols as usize,
            paper_of(fill.paper),
        );
    }
    for run in &frame.runs {
        grid.text(run.col as usize, run.row as usize, &run.text, ink_of(run.ink));
    }
    grid.elements(origin)
}

/// One cell: what it says, the ink it says it in, and the paper behind it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// The glyph, or a space.
    pub glyph: char,
    /// The glyph's colour.
    pub ink: Paint,
    /// The surface behind the glyph. Transparent paper is the world showing through.
    pub paper: Paint,
}

impl Cell {
    /// An empty cell: nothing written, nothing behind it.
    pub fn empty() -> Cell {
        Cell { glyph: ' ', ink: Paint { rgb: HOLO_TEXT, alpha: 0.0 }, paper: Paint { rgb: HOLO_BACKGROUND, alpha: 0.0 } }
    }

    /// Whether anything about this cell would draw.
    pub fn is_drawn(self) -> bool {
        (self.glyph != ' ' && self.ink.alpha > 0.0) || self.paper.alpha > 0.0
    }
}

/// A grid of cells, addressed from its own low corner.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
}

impl Grid {
    /// `cols × rows` empty cells. A zero dimension is an empty grid, not a refusal: a pane the
    /// caller measured as nothing has nothing to paint.
    pub fn new(cols: usize, rows: usize) -> Grid {
        Grid { cols, rows, cells: vec![Cell::empty(); cols * rows] }
    }

    /// The grid's width in cells.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// The grid's height in cells.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// The cell at `(col, row)`, or `None` outside the grid.
    pub fn cell(&self, col: usize, row: usize) -> Option<&Cell> {
        if col >= self.cols || row >= self.rows {
            return None;
        }
        self.cells.get(row * self.cols + col)
    }

    /// Write one cell. **Out-of-range writes are dropped**, never grown: a window's content is
    /// bounded by the window, which is the whole reason a grid exists.
    pub fn put(&mut self, col: usize, row: usize, cell: Cell) {
        if col < self.cols && row < self.rows {
            self.cells[row * self.cols + col] = cell;
        }
    }

    /// Fill a run of cells with paper.
    pub fn fill(&mut self, col: usize, row: usize, len: usize, paper: Paint) {
        for offset in 0..len {
            let Some(cell) = self.cell(col + offset, row).copied() else { break };
            self.put(col + offset, row, Cell { paper, ..cell });
        }
    }

    /// Write text from `(col, row)`, one glyph per cell, clipping at the right and bottom edges.
    pub fn text(&mut self, col: usize, row: usize, text: &str, ink: Paint) {
        self.text_within(col, row, text, ink, usize::MAX);
    }

    /// The same, clipped to `limit` cells — the element's own width, not the grid's.
    ///
    /// **A readout's value column is the reason.** The element's rect is nineteen characters wide
    /// at 14 sp; a longer string would otherwise paint into the column beside it, which is the
    /// "prose that gets silently cut off" the readout rule already names.
    fn text_within(&mut self, col: usize, row: usize, text: &str, ink: Paint, limit: usize) {
        for (offset, glyph) in text.chars().take(limit).enumerate() {
            let cell = col + offset;
            if cell >= self.cols {
                break;
            }
            let Some(existing) = self.cell(cell, row).copied() else { break };
            self.put(cell, row, Cell { glyph, ink, ..existing });
        }
    }

    /// Rasterise elements into a grid of `cols × rows`.
    ///
    /// **The element's own rect decides its cells.** A surface fills every cell it touches; text
    /// starts at the cell its alignment and its left edge snap to, and a run too long for them is
    /// elided rather than cut mid-word. A rule is drawn **after** both, in the nearest row clear of
    /// glyphs — see [`draw_rule`]. Every paint is retinted to the schema on the way in, so the grid
    /// holds schema colours and the emitter has nothing left to decide.
    pub fn of(elements: &[Element], cols: usize, rows: usize) -> Grid {
        let mut grid = Grid::new(cols, rows);
        // **Where the glyphs will be**, measured before anything is drawn: a rule has to know
        // whether a row carries text, and it must know it without being moved to a later pass —
        // a divider drawn after every surface would be painted over the window that occludes it.
        let mut occupied = vec![false; cols * rows];
        for element in elements {
            let Element::Text { rect, text, align, .. } = element else { continue };
            let Some((left, top, right, _)) = cell_bounds(*rect) else { continue };
            let width = right.saturating_sub(left);
            let glyphs = fit_to(text, width).chars().count();
            // **The same start the drawing computes**, end alignment included: a mask that
            // marked the wrong end of a right-aligned column would let a divider be placed in
            // the row it actually occupies.
            let start = match align {
                Align::Start => left,
                Align::End => left + width.saturating_sub(glyphs),
            };
            for col in start..(start + glyphs).min(cols) {
                if top < rows {
                    occupied[top * cols + col] = true;
                }
            }
        }
        for element in elements {
            match element {
                Element::Surface { rect, paint } => {
                    let paper = retint(*paint);
                    if let Some((left, top, right, bottom)) = cell_bounds(*rect) {
                        for row in top..bottom.min(rows) {
                            for col in left..right.min(cols) {
                                let Some(cell) = grid.cell(col, row).copied() else { break };
                                // **Paper is opaque to the layer below it.** A surface is a
                                // window's own body (or a panel's, or the bar's), and a window
                                // beneath it is occluded rather than ghosted through: the
                                // glyphs under the paper are cleared, so no two runs can share
                                // a cell, and the paint's own alpha blends with the *world* —
                                // which is the only thing the declared opacity is about. A
                                // transparent surface clears nothing.
                                let glyph = if paper.alpha > 0.0 { ' ' } else { cell.glyph };
                                grid.put(col, row, Cell { glyph, paper, ..cell });
                            }
                        }
                    }
                }
                Element::Text { rect, text, paint, align, .. } => {
                    let ink = retint(*paint);
                    let Some((left, top, right, _)) = cell_bounds(*rect) else { continue };
                    let width = right.saturating_sub(left);
                    let fitted = fit_to(text, width);
                    let start = match align {
                        Align::Start => left,
                        Align::End => left + width.saturating_sub(fitted.chars().count()),
                    };
                    grid.text_within(start, top, &fitted, ink, width);
                }
                Element::Rule { rect, paint } => {
                    draw_rule(&mut grid, &occupied, *rect, retint(*paint));
                }
            }
        }
        grid
    }

    /// **The grid as elements**, in row order: paper runs first, then glyph runs.
    ///
    /// A run is broken when the paper or the ink changes, so a row with two inks is two text runs
    /// and a row with two surfaces is two rectangles. The result is the same list kind the shell
    /// composed, which is what keeps the renderer unchanged.
    pub fn elements(&self, origin: Rect) -> Vec<Element> {
        let mut out = Vec::new();
        for row in 0..self.rows {
            let mut col = 0;
            while col < self.cols {
                let Some(cell) = self.cell(col, row).copied() else { break };
                if cell.paper.alpha <= 0.0 {
                    col += 1;
                    continue;
                }
                let paper = cell.paper;
                let start = col;
                while col < self.cols
                    && self
                        .cell(col, row)
                        .is_some_and(|next| next.paper == paper && next.paper.alpha > 0.0)
                {
                    col += 1;
                }
                out.push(Element::Surface {
                    rect: cell_rect(origin, start, row, col - start),
                    paint: paper,
                });
            }
            let mut col = 0;
            while col < self.cols {
                let Some(cell) = self.cell(col, row).copied() else { break };
                if cell.glyph == ' ' || cell.ink.alpha <= 0.0 {
                    col += 1;
                    continue;
                }
                let ink = cell.ink;
                let start = col;
                let mut text = String::new();
                while col < self.cols {
                    let Some(next) = self.cell(col, row).copied() else { break };
                    if next.ink != ink || next.ink.alpha <= 0.0 {
                        break;
                    }
                    text.push(next.glyph);
                    col += 1;
                }
                out.push(Element::Text {
                    rect: Rect::new(
                        origin.x + start as f64 * CELL_W_DP,
                        origin.y + row as f64 * CELL_H_DP,
                        (col - start) as f64 * CELL_W_DP,
                        CELL_H_DP,
                    ),
                    text,
                    paint: ink,
                    size_dp: CELL_TYPE_DP,
                    align: Align::Start,
                });
            }
        }
        out
    }

    /// How many cells would draw, which is the one number a test needs to tell "empty" from
    /// "painted the background".
    pub fn drawn(&self) -> usize {
        self.cells.iter().filter(|cell| cell.is_drawn()).count()
    }
}

/// **The shell's one raster act**: elements in, cell runs out.
///
/// `viewport` is the whole surface the elements were composed for, and it decides the grid — so
/// every panel, rule and line in the shell is snapped to the same cells rather than each to its own.
///
/// This is the current version's drawing; [`surface_at`] is the same act with the version named.
pub fn surface(elements: &[Element], viewport: Rect) -> Vec<Element> {
    let grid = Grid::of(elements, columns(viewport.width_dp), rows(viewport.height_dp));
    grid.elements(viewport)
}

/// **The interface drawn at a version the caller names**, which is how the old drawing stays
/// reachable: version 0 hands the composed elements on unchanged, version 1 is [`frames_in`].
///
/// **`placed` is every frame the mount graded, and it is the only place cells draw.** The shell's
/// main drawables — the bar, the title bars, the borders, the panels' chrome, the console's frame —
/// are handed on exactly as composed. A frame's footprint is a rect inside a declared window's
/// content container ([`super::Mount::frame_space`] builds it from `window_body`), so the cell
/// format cannot address the chrome even if a package asked it to. A caller that has no graded
/// frame gets the element drawing and no cells at all, which is a shell with a chrome rather than a
/// grid pretending to be one.
pub fn surface_at(
    elements: &[Element],
    placed: &[Placed<'_>],
    version: CellVersion,
) -> Vec<Element> {
    match version {
        // **The old version is not a fallback here, it is a drawing.** The elements were composed
        // against the views' own geometry, and this is the list the shell drew before the raster
        // existed — returned as it arrived, so "bring the old version back" is a version number and
        // not a reimplementation.
        CellVersion::Elements => elements.to_vec(),
        CellVersion::Holo => frames_in(elements, placed),
    }
}

/// Whether `rect` lies wholly inside `space`.
///
/// **Whole, not overlapping.** An element that straddles a footprint's edge is partly outside the
/// cells the frame declares, and replacing it would move the part the frame did not draw. A rect
/// with no area is not content either: a zero-height rule composed at a title bar's edge belongs to
/// the bar.
fn inside(rect: Rect, space: Rect) -> bool {
    rect.width_dp > 0.0
        && rect.height_dp > 0.0
        && rect.x >= space.x
        && rect.y >= space.y
        && rect.right() <= space.right()
        && rect.bottom() <= space.bottom()
}

/// **The shell's raster: every graded frame's cells, and nothing else.**
///
/// The composed list is walked once per frame. The elements standing inside a frame's footprint are
/// replaced by that frame's own cells — emitted where the first of them was, so the content keeps
/// the z-position the composer gave it — and everything else is copied through untouched. **A
/// frame's grid is its own rect's**, the content container the mount placed it in, so the cells can
/// begin where the window's body begins and not where the whole surface does; a shared viewport
/// origin would let a frame's run start outside the container it declared.
///
/// **An element outside every footprint is not rasterised**, which is the security property and not
/// a rendering choice: a package's declared frame is the only thing that may become cells, so a
/// package that declares none draws its content as composed elements. Frames are placed in `order`,
/// so a frame drawn later stands over one drawn earlier when their footprints overlap.
pub fn frames_in(elements: &[Element], placed: &[Placed<'_>]) -> Vec<Element> {
    if placed.is_empty() {
        return elements.to_vec();
    }
    let mut ordered: Vec<&Placed<'_>> = placed.iter().collect();
    ordered.sort_by_key(|entry| entry.frame.order);
    let mut out = Vec::with_capacity(elements.len());
    // **A frame is emitted once**, where its first claimed element stood; the rest of the elements
    // its footprint claims are dropped, because the frame's own cells are what stands there.
    let mut emitted = vec![false; ordered.len()];
    for element in elements {
        let rect = rect_of(element);
        let mut claimed = false;
        for (index, entry) in ordered.iter().enumerate() {
            if inside(rect, entry.footprint()) {
                claimed = true;
                if !emitted[index] {
                    out.extend(frame(entry.frame, entry.rect));
                    emitted[index] = true;
                }
            }
        }
        if !claimed {
            out.push(element.clone());
        }
    }
    // **A frame whose window composed nothing still draws.** A window whose content is entirely the
    // frame may compose no elements at all, and the cells are the content rather than a replacement
    // for it; a frame that claimed nothing is appended in its own order.
    for (index, entry) in ordered.iter().enumerate() {
        if !emitted[index] {
            out.extend(frame(entry.frame, entry.rect));
        }
    }
    out
}

/// An element's own rectangle, the thing a frame's footprint is tested against.
fn rect_of(element: &Element) -> Rect {
    match element {
        Element::Surface { rect, .. } | Element::Text { rect, .. } | Element::Rule { rect, .. } => {
            *rect
        }
    }
}

/// [`surface_at`] for a version written as its number, refused by name when this shell does not
/// draw it.
pub fn raster(
    elements: &[Element],
    placed: &[Placed<'_>],
    version: u16,
) -> Result<Vec<Element>, CellRefusal> {
    Ok(surface_at(elements, placed, CellVersion::of(version)?))
}

/// How many columns a width holds, rounded up: a half-covered cell is a cell that draws.
pub fn columns(width_dp: f64) -> usize {
    if width_dp <= 0.0 {
        return 0;
    }
    (width_dp / CELL_W_DP).ceil() as usize
}

/// How many rows a height holds, rounded up.
pub fn rows(height_dp: f64) -> usize {
    if height_dp <= 0.0 {
        return 0;
    }
    (height_dp / CELL_H_DP).ceil() as usize
}

/// **The run as many cells can hold — and when it cannot, an ellipsis says so.**
///
/// The shell's rects were measured for proportional type; a cell holds one glyph of a
/// monospace run, so a tab declared at 96 dp has twelve cells and a nineteen-character label
/// does not fit it. Cutting at the twelfth character drew `Working / cr`, which reads as a
/// broken label rather than a shortened one. The last cell a cut run owns is the ellipsis, so
/// every shortened run says so in its own ink and a reader can tell the two apart.
fn fit_to(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let glyphs: Vec<char> = text.chars().collect();
    if glyphs.len() <= width {
        return text.to_string();
    }
    let mut out: String = glyphs[..width - 1].iter().collect();
    out.push('…');
    out
}

/// **One divider, drawn where it does not strike through text.**
///
/// A 1 dp rule is not a cell: it is a line at a boundary, and the boundary it is nearest may
/// fall in a row a title bar's glyphs already own. The composite's title bar is 48 dp with the
/// title 11 dp into it, so the bar's bottom divider and the next strip's title land in one
/// 16 dp row — and writing `─` across that row drew a dashed line *through* the title, once on
/// each side of it. The rule is therefore placed in the row its own edge falls in when that row
/// is clear, and otherwise in the neighbouring row nearest its edge; a rule with no clear row
/// within one cell is **not drawn**, because a missing divider is a quieter defect than a line
/// through a word.
fn draw_rule(grid: &mut Grid, occupied: &[bool], rect: Rect, ink: Paint) {
    let Some((left, top, right, _)) = cell_bounds(rect) else { return };
    let left = left.min(grid.cols);
    let right = right.min(grid.cols);
    if left >= right {
        return;
    }
    let below = (rect.y / CELL_H_DP).fract() >= 0.5;
    let mut rows = vec![top];
    if below {
        rows.push(top + 1);
        rows.push(top.saturating_sub(1));
    } else {
        rows.push(top.saturating_sub(1));
        rows.push(top + 1);
    }
    for row in rows {
        if row >= grid.rows {
            continue;
        }
        let taken = (left..right)
            .any(|col| col + row * grid.cols < occupied.len() && occupied[col + row * grid.cols]);
        if taken {
            continue;
        }
        for col in left..right {
            let Some(cell) = grid.cell(col, row).copied() else { break };
            grid.put(col, row, Cell { glyph: '─', ink, ..cell });
        }
        return;
    }
}

fn cell_bounds(rect: Rect) -> Option<(usize, usize, usize, usize)> {
    if rect.width_dp <= 0.0 || rect.height_dp <= 0.0 {
        return None;
    }
    let left = (rect.x.max(0.0) / CELL_W_DP).floor() as usize;
    let top = (rect.y.max(0.0) / CELL_H_DP).floor() as usize;
    let right = ((rect.x + rect.width_dp).max(0.0) / CELL_W_DP).ceil() as usize;
    let bottom = ((rect.y + rect.height_dp).max(0.0) / CELL_H_DP).ceil() as usize;
    Some((left, top, right.max(left + 1), bottom.max(top + 1)))
}

fn cell_rect(origin: Rect, col: usize, row: usize, cols: usize) -> Rect {
    Rect::new(
        origin.x + col as f64 * CELL_W_DP,
        origin.y + row as f64 * CELL_H_DP,
        cols as f64 * CELL_W_DP,
        CELL_H_DP,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paper(rgb: Rgb) -> Paint {
        Paint::opaque(rgb)
    }

    fn viewport() -> Rect {
        Rect::new(0.0, 0.0, 24.0, 32.0)
    }

    #[test]
    fn every_paint_lands_on_the_schema_and_keeps_its_alpha() {
        let blue = Paint { rgb: Rgb::new(0x30, 0xB0, 0xE0), alpha: 0.5 };
        assert_eq!(retint(blue), Paint { rgb: HOLO_BLUE, alpha: 0.5 });
        assert_eq!(retint(paper(Rgb::new(0xFE, 0xFF, 0xFE))).rgb, HOLO_TEXT);
        assert_eq!(retint(paper(Rgb::new(0x30, 0x30, 0x30))).rgb, HOLO_DIVIDER);
        for value in HOLO {
            assert_eq!(retint(paper(value)).rgb, value, "the schema maps to itself");
        }
    }

    #[test]
    fn a_surface_becomes_whole_cells_and_a_rule_is_one_cell_high() {
        let elements = vec![
            Element::Surface { rect: Rect::new(0.0, 0.0, 24.0, 16.0), paint: paper(HOLO_SURFACE) },
            Element::Rule { rect: Rect::new(0.0, 16.0, 24.0, 1.0), paint: paper(HOLO_DIVIDER) },
        ];
        let grid = Grid::of(&elements, 3, 2);
        assert_eq!(grid.cell(0, 0).expect("a cell").paper.rgb, HOLO_SURFACE);
        assert_eq!(grid.cell(1, 0).expect("a cell").paper.rgb, HOLO_SURFACE);
        assert_eq!(grid.cell(0, 1).expect("a cell").glyph, '─');
        assert_eq!(grid.cell(0, 1).expect("a cell").ink.rgb, HOLO_DIVIDER);
        let out = grid.elements(viewport());
        let surfaces: Vec<&Element> = out
            .iter()
            .filter(|element| matches!(element, Element::Surface { .. }))
            .collect();
        assert_eq!(surfaces.len(), 1, "one row of one paper is one run: {out:#?}");
        let Element::Surface { rect, .. } = surfaces[0] else { unreachable!() };
        assert_eq!(rect.width_dp, 24.0);
        assert_eq!(rect.height_dp, CELL_H_DP);
    }

    #[test]
    fn text_is_written_one_glyph_per_cell_and_clipped_at_the_edge() {
        let element = Element::Text {
            rect: Rect::new(0.0, 0.0, 32.0, 16.0),
            text: "abcdefgh".to_string(),
            paint: paper(HOLO_TEXT),
            size_dp: 12.0,
            align: Align::Start,
        };
        let grid = Grid::of(&[element], 3, 1);
        let written: String = (0..3)
            .filter_map(|col| grid.cell(col, 0).map(|cell| cell.glyph))
            .collect();
        assert_eq!(written, "abc", "the fourth glyph is outside the window");
        let out = grid.elements(viewport());
        let text = out.iter().find_map(|element| match element {
            Element::Text { text, .. } => Some(text.clone()),
            _ => None,
        });
        assert_eq!(text.as_deref(), Some("abc"));
    }

    #[test]
    fn an_empty_grid_draws_nothing_and_a_zero_pane_has_no_cells() {
        let grid = Grid::of(&[], 4, 4);
        assert_eq!(grid.drawn(), 0);
        assert!(grid.elements(viewport()).is_empty());
        assert_eq!(columns(0.0), 0);
        assert_eq!(rows(-4.0), 0);
        let zero = Grid::of(&[], 0, 0);
        assert!(zero.elements(viewport()).is_empty());
    }

    #[test]
    fn the_same_elements_produce_the_same_cells() {
        let elements = vec![
            Element::Surface { rect: Rect::new(0.0, 0.0, 24.0, 32.0), paint: paper(HOLO_CHROME) },
            Element::Text {
                rect: Rect::new(8.0, 0.0, 16.0, 16.0),
                text: "ok".to_string(),
                paint: paper(HOLO_GREEN),
                size_dp: 12.0,
                align: Align::Start,
            },
        ];
        let first = Grid::of(&elements, 3, 2);
        let second = Grid::of(&elements, 3, 2);
        assert_eq!(first, second);
        assert_eq!(first.elements(viewport()), second.elements(viewport()));
    }

    #[test]
    fn a_text_run_is_clipped_to_its_own_rect_not_only_the_grid() {
        // A value column is two cells wide; six glyphs would paint through the column beside it.
        let element = Element::Text {
            rect: Rect::new(0.0, 0.0, 16.0, 16.0),
            text: "abcdef".to_string(),
            paint: paper(HOLO_TEXT),
            size_dp: 12.0,
            align: Align::Start,
        };
        let grid = Grid::of(&[element], 6, 1);
        let written: String = (0..6)
            .filter_map(|col| grid.cell(col, 0).map(|cell| cell.glyph))
            .collect();
        // **The cut cell says it was cut.** A label cut at the twelfth character read as a
        // broken label — `Working / cr` — rather than as a shortened one.
        assert_eq!(written.trim_end(), "a…");
        assert_eq!(grid.cell(2, 0).expect("a cell").glyph, ' ');
    }

    #[test]
    fn a_rule_does_not_draw_through_a_text_row() {
        // The composite's title bar is 48 dp and the title 11 dp into it, so the bar's bottom
        // divider and the next strip's title share a 16 dp row. The divider is drawn in the
        // nearest row that is clear instead — or not at all.
        let title = Element::Text {
            rect: Rect::new(8.0, 16.0, 64.0, 16.0),
            text: "Refusals".to_string(),
            paint: paper(HOLO_TEXT),
            size_dp: 12.0,
            align: Align::Start,
        };
        let divider = Element::Rule {
            rect: Rect::new(0.0, 24.0, 80.0, 1.0),
            paint: paper(HOLO_DIVIDER),
        };
        // Row 1 holds the title at cell 1; the rule's own row is 1, and rows 0 and 2 are clear.
        let grid = Grid::of(&[title, divider], 10, 3);
        let row: String = (0..10)
            .filter_map(|col| grid.cell(col, 1).map(|cell| cell.glyph))
            .collect();
        assert!(row.contains("Refusals"), "the title survives: {row:?}");
        let ruled: Vec<usize> = (0..3)
            .filter(|row| (0..10).any(|col| grid.cell(col, *row).is_some_and(|cell| cell.glyph == '─')))
            .collect();
        assert_eq!(ruled, vec![2], "the divider moved to the clear row below it");
    }

    #[test]
    fn a_surface_occludes_the_text_beneath_it() {
        // **A window beneath is occluded, not ghosted through.** Two windows at opacity 930
        // over one another used to leave both bodies of text legible in the same row; the
        // paper of the window on top is opaque to the one under it, and only the world shows
        // through its own alpha.
        let under = Element::Text {
            rect: Rect::new(0.0, 0.0, 80.0, 16.0),
            text: "underneath".to_string(),
            paint: paper(HOLO_TEXT),
            size_dp: 12.0,
            align: Align::Start,
        };
        let over = Element::Surface {
            rect: Rect::new(16.0, 0.0, 32.0, 16.0),
            paint: Paint { rgb: HOLO_SURFACE, alpha: 0.93 },
        };
        let grid = Grid::of(&[under, over], 10, 1);
        for col in 2..6 {
            let cell = grid.cell(col, 0).expect("a cell");
            assert_eq!(cell.glyph, ' ', "the covered glyph is gone at column {col}");
            assert_eq!(cell.paper.rgb, HOLO_SURFACE);
        }
        // Outside the surface the run is untouched, and the surface keeps its own alpha: the
        // world is what it blends with, not the window underneath.
        assert_eq!(grid.cell(0, 0).expect("a cell").glyph, 'u');
        assert_eq!(grid.cell(2, 0).expect("a cell").paper.alpha, 0.93);
    }

    /// A frame the grammar accepts, for the tests that need one: eight cells wide, two tall, with
    /// paper under a run.
    fn a_frame() -> voxel_package::cellframe::Frame {
        voxel_package::cellframe::Frame::parse(concat!(
            "shploingus.cellframe/1\n",
            "frame\tMAP-0001\n",
            "\twindow\tmap-plan\n",
            "\tcols\t8\n",
            "\trows\t2\n",
            "\torder\t10\n",
            "\trequires\tnone\n",
            "fill\t0\t0\t8\tsurface\n",
            "run\t1\t0\taccent\tholo\n",
        ))
        .expect("a frame the grammar accepts")
    }

    /// **The old drawing is a version, not dead code.** Version 0 returns the elements the shell
    /// composed, unchanged; version 1 draws the graded frame's cells. The test needs both, because a
    /// "legacy" path that quietly ran the frame would pass every other test in this file.
    #[test]
    fn version_zero_is_the_elements_as_composed_and_version_one_is_the_frame() {
        // A rect that does not land on the cell grid, so the two versions cannot agree by accident:
        // the frame draws its own cells there and the old drawing keeps its 12.5 dp.
        let elements = vec![Element::Surface {
            rect: Rect::new(0.0, 0.0, 12.5, 16.0),
            paint: paper(Rgb::new(0x1B, 0x19, 0x22)),
        }];
        let declared = a_frame();
        let placed = [Placed { frame: &declared, rect: viewport() }];
        let old = raster(&elements, &placed, CellVersion::Elements.number())
            .expect("version 0 is this shell's own drawing");
        assert_eq!(old, elements, "the old version did not draw what the caller composed");
        let new = raster(&elements, &placed, CellVersion::Holo.number())
            .expect("version 1 is this shell's own drawing");
        assert_ne!(
            new, elements,
            "the cell raster returned the composed elements, so the two versions are one drawing"
        );
        assert_eq!(new, frame(&declared, viewport()), "version 1 is the frame's own cells");
    }

    /// **A version this shell does not draw is refused by number, and the refusal lists the two it
    /// does** — which is the whole repair: an interface is never drawn at a version the binary has
    /// never heard of, and the number that would bring the old drawing back is in the message.
    #[test]
    fn an_unimplemented_cell_version_is_refused_and_the_message_names_the_ones_that_exist() {
        let refusal = raster(&[], &[], 2).expect_err("version 2 is not this shell's");
        assert_eq!(
            refusal,
            CellRefusal::VersionNotImplemented { asked: 2, implemented: [0, 1] }
        );
        let message = refusal.to_string();
        assert!(message.contains("version 2"), "the asked number is missing: {message}");
        assert!(message.contains("version 0"), "the old version is missing: {message}");
        assert!(message.contains("version 1"), "the current version is missing: {message}");
        for version in CellVersion::IMPLEMENTED {
            assert_eq!(
                CellVersion::of(version.number()),
                Ok(version),
                "a version the shell lists does not round-trip through its own number"
            );
        }
    }

    /// **The shell's own package is the declaration in force**, read from the folder `build.rs`
    /// compiles — so the version the game draws is the package's line, not this test's constant.
    #[test]
    fn the_shell_package_declares_the_version_in_force() {
        let folder = workspace().join("display-packages/alpha-shell");
        let declaration = declare(&folder)
            .expect("the shell's declaration is one this build reads")
            .expect("the shell's package carries a declaration");
        assert_eq!(declaration.version, CellVersion::Holo);
        assert_eq!(declaration.line, 2, "the `cells` line is line 2 of the file");
        assert!(
            declaration.file.ends_with("alpha-shell/package.cells"),
            "{}",
            declaration.file.display()
        );
    }

    /// **A package that predates the declaration is a declared absence, not a refusal**: the caller
    /// keeps the version in force, and nothing is invented for it.
    #[test]
    fn a_package_without_a_declaration_is_a_declared_absence() {
        let folder = workspace().join("display-packages/horizon-lod");
        assert_eq!(declare(&folder), Ok(None));
    }

    /// **The other half of the user's request**: a package written under the current grammar that
    /// declares `cells 0` brings the old drawing back, and the drawing is the elements as composed.
    #[test]
    fn a_current_declaration_brings_the_old_drawing_back() {
        let folder = workspace().join("crates/voxel-client/tests/fixtures/old-drawing");
        let declaration = declare(&folder)
            .expect("the current grammar is this build's")
            .expect("the package declares a drawing");
        assert_eq!(declaration.version, CellVersion::Elements);
        let elements = vec![Element::Text {
            rect: Rect::new(0.0, 0.0, 80.0, 16.0),
            text: "old".to_string(),
            paint: paper(HOLO_TEXT),
            size_dp: 12.0,
            align: Align::Start,
        }];
        assert_eq!(
            raster(&elements, &[], declaration.version.number()).expect("version 0"),
            elements,
            "the old drawing did not return the composed elements"
        );
    }

    fn workspace() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("the crate sits two levels under the workspace")
            .to_path_buf()
    }

    #[test]
    fn a_rule_with_no_clear_row_near_it_is_not_drawn() {
        let text = |row: u32| Element::Text {
            rect: Rect::new(0.0, f64::from(row) * 16.0, 80.0, 16.0),
            text: "xxxxxxxxxx".to_string(),
            paint: paper(HOLO_TEXT),
            size_dp: 12.0,
            align: Align::Start,
        };
        // Every row of the grid carries text, so the divider has nowhere to go.
        let grid = Grid::of(
            &[
                text(0),
                text(1),
                text(2),
                Element::Rule { rect: Rect::new(0.0, 24.0, 80.0, 1.0), paint: paper(HOLO_DIVIDER) },
            ],
            10,
            3,
        );
        assert!(!(0..3).any(|row| (0..10)
            .any(|col| grid.cell(col, row).is_some_and(|cell| cell.glyph == '─'))));
    }

    /// **A frame draws its own cells and nothing else, and only where it was placed.**
    ///
    /// The bar and the body are the same surface and the same paint; the difference is *where*
    /// they stand. The bar is outside the frame's footprint and must come out of the raster
    /// exactly as it went in — same rect, same paint, same place in the z-order. The body, inside
    /// it, is claimed by the frame: its composed runs are replaced by the frame's cells. And with
    /// no graded frame the shell draws its composed elements — **content inside a window's content
    /// space is not cells by itself**, which is the security property PATCH-076 states and this
    /// patch keeps.
    #[test]
    fn a_frame_draws_its_cells_and_content_without_one_stays_composed() {
        let bar = Element::Surface {
            rect: Rect::new(0.0, 0.0, 240.0, 16.0),
            paint: paper(HOLO_SURFACE),
        };
        let body = Element::Surface {
            rect: Rect::new(16.0, 32.0, 64.0, 32.0),
            paint: paper(HOLO_SURFACE),
        };
        let text = Element::Text {
            rect: Rect::new(16.0, 32.0, 64.0, 16.0),
            text: "hi".to_string(),
            paint: paper(HOLO_TEXT),
            size_dp: 12.0,
            align: Align::Start,
        };
        let elements = vec![bar.clone(), body.clone(), text.clone()];
        let declared = a_frame();
        let placed = [Placed { frame: &declared, rect: Rect::new(16.0, 32.0, 0.0, 0.0) }];

        let out = frames_in(&elements, &placed);
        assert_eq!(out.first(), Some(&bar), "the bar was snapped to cells");
        assert!(out.contains(&bar), "the bar did not survive the raster unchanged");
        assert!(
            !out.contains(&body),
            "a surface inside the frame's footprint was passed through instead of drawn as cells"
        );
        assert!(
            !out.contains(&text),
            "the composed text survived inside the frame's footprint"
        );
        assert!(
            out.iter().any(|element| matches!(element, Element::Text { text, .. } if text == "holo")),
            "the frame's run did not reach the element list"
        );
        assert!(
            out.iter().all(|element| rect_of(element).y >= 32.0 || element == &bar),
            "the frame drew above its own footprint"
        );

        // **No frame is no cells**, even inside a content space: the format is the package's.
        assert_eq!(
            frames_in(&elements, &[]),
            elements,
            "with no graded frame the shell must draw as composed"
        );
    }
}
