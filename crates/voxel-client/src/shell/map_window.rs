//! **The map window, under one owner.**
//!
//! Everything that is *the map window* lives here and nothing else does: where its camera looks and
//! whether it has been placed, the survey it draws, the cell it has selected, the cell the pointer
//! is over, and how far its control gutter is scrolled. The rects those things are drawn and picked
//! in are this module's arithmetic too, so the picture and the controls cannot disagree about where
//! the window is.
//!
//! # Why the split, and where each half stands
//!
//! `mount.rs` owns the shell: which windows exist, where they are, what the pointer is doing, and
//! the order the layer composes in. It was also holding every value and every verb of one particular
//! window, which meant a map bug hid in a nine-thousand-line file about windows. The boundary drawn
//! here is:
//!
//! * **[`MapWindow`]** — the window's state, and every verb that touches it. One owner: no copy of
//!   the camera, the selection or the gutter's scroll exists anywhere else in the shell, so there is
//!   no second place for them to drift apart in.
//! * **[`MapChrome`]** — *all* the window knows about the shell around it: the content container the
//!   package's window put it in (`None` when it is closed), the viewport to fall back to, whether its
//!   chrome is shown, the frame's own declaration of the map, and the two feature tokens (see the
//!   field docs). `Mount::map_chrome` builds it, in one place, so the rects the pointer resolves
//!   against are the rects the picture is drawn in.
//! * **[`crate::map_view`]** — the drawing itself: the camera's projection, the terrain's picking and
//!   raster, the view filter. This module composes those into the shell's element language and
//!   answers the verbs; it does not re-derive a projection.
//! * **[`crate::map_survey`]** — the world-facing half: what the ground is at a plan's samples. The
//!   window asks it and holds it, so the survey has exactly one reader.
//! * **[`super::mount`]** — pointer *routing* (which region a press or a wheel belongs to), layout,
//!   the window chrome, and the package's own view selection (`select_feature`), which is why
//!   [`MapWindow::action`] answers `None` for a view token instead of inventing one.
//!
//! # Behaviour
//!
//! **The window is a viewer.** It draws the world's own ground at the plan the camera implies, marks
//! the selected cell and states where the pointer is; it is *moved and looked at* — a drag pans it,
//! the wheel scales it, and the keys the focused window owns run the same verbs the gutter's buttons
//! do ([`MAP_MOVE_STEP_DP`], [`MapWindow::action`]). **It edits nothing**: the terrain's own
//! modification is the shapers' over the world's cadence tick, and a map window that struck ground
//! was this module's misreading of what a map is for.

use super::webview::{Facts, Rendered};
use super::xml::Document;
use crate::map_survey::{MapSurvey, MapWorld, Observation};
use crate::map_view::{MapCamera, MapTerrain, ViewFilter};
use voxel_package::cellframe::MapView;
use voxel_ui::{Element, Palette, Rect};

/// **The readout toolbar's height**, dp. The strip spans the content container's width, and the
/// map and the control gutter both begin below it — one header, not two.
pub const MAP_TOOLBAR_DP: f64 = 40.0;

/// **One row of the control gutter**, dp: a 24 dp control and the 2 dp gap under it. The gutter's
/// wheel moves by whole rows, so a column scrolled to any offset still has every control it draws
/// wholly inside itself. See [`MapWindow::wheel`].
pub const MAP_CONTROLS_ROW_DP: f64 = 26.0;

/// **How far one `move` takes the view**, dp: a screen distance rather than a world one, so a key
/// carries the same share of the window at every zoom and the ground travels with it in the plain
/// direction of the key (`move up` slides the ground down the window, the way a drag down does).
/// The platform's key repeat is what makes holding one a walk. See [`MapWindow::action`].
pub const MAP_MOVE_STEP_DP: f64 = 48.0;

/// **The map's control gutter width**, dp. A proportion with a floor and a ceiling: wide enough
/// for a two-button row and never so wide that the terrain is a sliver.
fn gutter_dp(content_width: f64) -> f64 {
    (content_width * 0.30)
        .clamp(120.0, 168.0)
        .min((content_width - 80.0).max(0.0))
}

/// **The frame's own declaration of the map window**, in the fields the window reads: where it
/// opens, how big a map cell is, how many cells the picture covers, and how far down it looks.
///
/// Copied out of the package's [`MapView`] rather than borrowed, because a verb's answer has to
/// survive the mount being asked to change the very frame that declared it (`plan ocean`) while the
/// window is holding it. A copy of six numbers is cheaper than the borrow that would forbid it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MapDecl {
    /// The world column the window opens on, metres.
    pub centre_xz: [f64; 2],
    /// One map cell, metres.
    pub cell_m: f64,
    /// The frame's declared extent in cells: columns (x) and rows (z).
    pub extent: [u32; 2],
    /// The camera's downward tilt, degrees.
    pub pitch_deg: f64,
}

impl MapDecl {
    /// The window's reading of a frame's declaration.
    pub fn of(view: &MapView) -> MapDecl {
        MapDecl {
            centre_xz: view.centre_xz,
            cell_m: view.cell_m,
            extent: view.extent,
            pitch_deg: view.pitch_deg,
        }
    }
}

/// **What the map window may read of the shell around it**, and the whole of it: the mount resolves
/// these once per question (`Mount::map_chrome`) and the window never reaches back for more.
///
/// Owned rather than borrowed for the same reason [`MapDecl`] is: a verb mutates the window while
/// the answer to "where is my window, and what does my frame declare" is still in hand.
#[derive(Clone, PartialEq, Debug)]
pub struct MapChrome {
    /// The window's content container, or `None` when it is closed or minimized.
    pub content: Option<Rect>,
    /// The shell's viewport, for the one fallback the window has: a survey's body before any window
    /// is placed on screen (a capture asking for a plan, a test that never opened the window).
    pub viewport: Rect,
    /// **Whether the toolbar and the gutter are drawn.** False is the minimap form: a pinned map is
    /// HUD chrome the player keeps in view, and its chrome would be most of a small panel.
    pub chrome: bool,
    /// The frame's declaration, when the package declares a GPU map at all.
    pub decl: Option<MapDecl>,
    /// **The showing frame's own `feature` token**, which drives the view filter and the toolbar's
    /// `VIEW` fact — the frame the window is drawn from, not the shell's notion of the selection.
    pub feature: Option<String>,
    /// **The token the shell's view selection implies** (`plan ocean`), which is the button the
    /// gutter draws as selected. It is the package's vocabulary, and the mount owns it.
    pub selected: String,
}

/// **The map window's whole state.**
///
/// One value, so a frame's worth of map is one field's worth of state: a capture, a test, a scripted
/// run and the frame loop all steer the same window, and none of them can hold a private copy of
/// half of it.
pub struct MapWindow {
    /// The one value the picture comes from: where it looks, how big a metre is, which way is up the
    /// screen, and how far down it looks.
    camera: MapCamera,
    /// Whether the camera has been placed on the frame's declared world column yet.
    camera_ready: bool,
    /// **The survey under one owner**: the plan the ground was sampled for, the law's field, the
    /// composed survey, the keys they are cached under and the ground raster. The window keeps the
    /// *state* — where the camera looks, what is selected, where the pointer is — and the survey
    /// keeps the *world*.
    survey: MapSurvey,
    /// The body drag in progress: where the pointer was, and how far it has come.
    drag: Option<((f64, f64), f64)>,
    /// The map's selected cell, in world metres, set and cleared by a click on its body.
    selection: Option<[f64; 2]>,
    /// The cell the pointer is over, world `[x, y, z]`, and the cell itself for the outline — which
    /// is what the toolbar's cursor fact reads.
    cursor: Option<[f64; 3]>,
    cursor_cell: Option<(i32, i32)>,
    /// **How far the control gutter is scrolled**, in dp. The gutter's document is taller than the
    /// map window at the size this package declares (524 dp of controls in a 342 dp column), and a
    /// control the column pushes past the bottom edge is a control that does not exist: the wheel
    /// over the gutter moves this, the way the wheel over the console's own body moves its viewport.
    gutter_scroll: f64,
}

impl Default for MapWindow {
    fn default() -> MapWindow {
        MapWindow::new()
    }
}

impl MapWindow {
    /// A window that has never been observed: no camera placement, no survey, no cursor.
    pub fn new() -> MapWindow {
        MapWindow {
            camera: MapCamera::default(),
            camera_ready: false,
            survey: MapSurvey::new(),
            drag: None,
            selection: None,
            cursor: None,
            cursor_cell: None,
            gutter_scroll: 0.0,
        }
    }

    // ---------------------------------------------------------------------------------------
    // The rects: one arithmetic, read by the drawing and by the pointer.
    // ---------------------------------------------------------------------------------------

    /// **The readout toolbar** across the window's top: the content container's full width,
    /// [`MAP_TOOLBAR_DP`] tall. `None` in the minimap form, where the ground takes the row.
    pub fn toolbar_rect(&self, chrome: &MapChrome) -> Option<Rect> {
        if !chrome.chrome {
            return None;
        }
        let content = chrome.content?;
        Some(Rect::new(
            content.x,
            content.y,
            content.width_dp,
            MAP_TOOLBAR_DP.min(content.height_dp),
        ))
    }

    /// **The terrain's viewport inside the map window** — the content container minus the toolbar
    /// and the control gutter, or the whole container in the minimap form. A GPU map is drawn here
    /// and nowhere else, so it cannot paint over a button, the toolbar or a title bar.
    pub fn view_rect(&self, chrome: &MapChrome) -> Option<Rect> {
        let content = chrome.content?;
        if !chrome.chrome {
            return Some(Rect::new(
                content.x,
                content.y,
                content.width_dp.max(1.0),
                content.height_dp.max(1.0),
            ));
        }
        let gutter = gutter_dp(content.width_dp);
        let top = content.y + MAP_TOOLBAR_DP;
        Some(Rect::new(
            content.x,
            top,
            (content.width_dp - gutter - 1.0).max(1.0),
            (content.height_dp - MAP_TOOLBAR_DP).max(1.0),
        ))
    }

    /// **The controls' gutter**, on the map window's right edge below the toolbar.
    pub fn controls_rect(&self, chrome: &MapChrome) -> Option<Rect> {
        if !chrome.chrome {
            return None;
        }
        let content = chrome.content?;
        let gutter = gutter_dp(content.width_dp);
        Some(Rect::new(
            content.right() - gutter,
            content.y + MAP_TOOLBAR_DP,
            gutter,
            (content.height_dp - MAP_TOOLBAR_DP).max(1.0),
        ))
    }

    /// **The body's rect for a caller that only knows the window**: the terrain's own viewport, or
    /// `None` when the window is not on screen. The pointer's hit tests and the toolbar's facts both
    /// ask this, rather than each computing a rect of their own.
    pub fn body_rect(&self, chrome: &MapChrome) -> Option<Rect> {
        self.view_rect(chrome)
    }

    // ---------------------------------------------------------------------------------------
    // Reading the window.
    // ---------------------------------------------------------------------------------------

    /// **The map's zoom, in doublings** of the default scale — a reading of the camera, so the
    /// toolbar cannot report a zoom the picture is not at.
    pub fn zoom(&self) -> i32 {
        (self.camera.pixels_per_m / MapCamera::default().pixels_per_m)
            .log2()
            .round() as i32
    }

    /// The camera, for a caller that draws or tests through it.
    pub fn camera(&self) -> MapCamera {
        self.camera
    }

    /// **The live facts the toolbar reads**, from the frame the window is showing and the camera's
    /// own zoom. `None` when the package declares no GPU map.
    pub fn facts(&self, chrome: &MapChrome) -> Option<Facts> {
        let map = chrome.decl?;
        Some(Facts {
            view: chrome.feature.clone().unwrap_or_else(|| "all".to_string()),
            cell_m: map.cell_m,
            extent: map.extent,
            pitch_deg: self.camera.pitch_deg,
            zoom: self.zoom(),
            cursor: self.cursor,
        })
    }

    /// The survey the window is drawing, for a caller that reads it rather than steering it.
    pub fn terrain(&self) -> Option<&MapTerrain> {
        self.survey.terrain()
    }

    /// **The ground the survey draws at a world column**, for a caller that reads the picture rather
    /// than composing it; `None` off the survey, where the map has no answer to give.
    ///
    /// This is the same interpolated read the raster and the picking use, so a scripted run can ask
    /// *"what does the map say the ground is here?"* of the same map it is looking at — a reading
    /// taken rather than eyeballed.
    pub fn ground_at(&self, xz: [f64; 2]) -> Option<f64> {
        self.survey
            .terrain()?
            .ground_at(xz)
            .map(|(height, _)| height)
    }

    /// **The map's selected cell**, world metres, if one is selected.
    pub fn selection(&self) -> Option<[f64; 2]> {
        self.selection
    }

    /// The cell the pointer is over, for the outline the map draws and the toolbar's readout.
    pub fn cursor_cell(&self) -> Option<(i32, i32)> {
        self.cursor_cell
    }

    /// How far the gutter is scrolled, in dp.
    pub fn controls_scroll(&self) -> f64 {
        self.gutter_scroll
    }

    /// **Whether the window has anything to draw at all** — a survey that has been sampled.
    pub fn observed(&self) -> bool {
        self.survey.observed()
    }

    // ---------------------------------------------------------------------------------------
    // Setting the window's state outright: for a restored view, a capture, a test.
    // ---------------------------------------------------------------------------------------

    /// **Set the camera outright** — the value the package's declaration places at open, for a
    /// caller that already knows where the window should look.
    pub fn camera_to(&mut self, camera: MapCamera) {
        self.camera = camera;
        self.camera.clamp();
        self.camera_ready = true;
    }

    /// **Set the selection directly**, for a caller that already knows the world column. `None`
    /// clears it; a value that is not finite is not a place and is refused as one.
    pub fn select(&mut self, selection: Option<[f64; 2]>) {
        self.selection = selection.filter(|xz| xz[0].is_finite() && xz[1].is_finite());
    }

    // ---------------------------------------------------------------------------------------
    // Observing the world: the one call the app makes before the layer composes.
    // ---------------------------------------------------------------------------------------

    /// **Observe the world for this window**, once per frame, before the layer composes.
    ///
    /// One call does everything the map needs from the world: it places the camera on the frame's
    /// declared column the first time, asks the survey for the plan the body and the camera imply,
    /// writes the world's own voxels over the laws' field wherever the world holds chunks, and moves
    /// the marker to the pose. **The caller holds no key and no terrain** — the survey is this
    /// window's, and the one thing the window cannot know by itself is `world.ground`.
    ///
    /// `pose` is `[x, z, bearing_deg]`: the player's world column and heading, in the units the map
    /// draws in. `dt_s` is the frame's elapsed seconds, which the survey's own gate reads.
    pub fn observe(
        &mut self,
        world: MapWorld<'_>,
        pose: Option<[f64; 3]>,
        dt_s: f64,
        chrome: &MapChrome,
    ) -> Observation {
        let Some(declaration) = chrome.decl else {
            return Observation::default();
        };
        // **The window opens on the place its package declares**, and `recentre` is the same
        // placement again: the frame's own world column, not the quantised centre of wherever the
        // camera happens to be.
        if !self.camera_ready {
            self.camera.look_at(declaration.centre_xz);
            // **And at the tilt the frame declares**: a package that asks for a shallower window
            // than the default gets one, because the pitch is the declaration's and not a taste the
            // shell keeps.
            self.camera.pitch_deg = declaration.pitch_deg.abs();
            self.camera.clamp();
            self.camera_ready = true;
        }
        let body = self.view_rect(chrome).unwrap_or(chrome.viewport);
        let plan = self.camera.survey(body, declaration.cell_m);
        let observation = self.survey.observe(world, plan, dt_s);
        if let Some([x, z, bearing_deg]) = pose {
            self.survey.follow(world.spec, [x, z], bearing_deg);
        }
        observation
    }

    // ---------------------------------------------------------------------------------------
    // The pointer: the body's own gestures, and the gutter's wheel.
    // ---------------------------------------------------------------------------------------

    /// **A body drag pans the camera**, bringing the world with the pointer.
    pub fn press(&mut self, at: (f64, f64), chrome: &MapChrome) -> bool {
        if !self.survey.observed() {
            return false;
        }
        let Some(rect) = self.view_rect(chrome) else {
            return false;
        };
        if !rect.contains(at) {
            self.drag = None;
            return false;
        }
        self.drag = Some((at, 0.0));
        true
    }

    /// A pointer move while a body press is held: pan, and remember how far it has come, so a press
    /// that never moved can still mean "select this cell".
    pub fn drag(&mut self, at: (f64, f64), chrome: &MapChrome) -> bool {
        let Some((last, travelled)) = self.drag else {
            return false;
        };
        let Some(rect) = self.view_rect(chrome) else {
            return false;
        };
        self.camera
            .pan_by_screen(at.0 - last.0, at.1 - last.1, rect);
        self.drag = Some((at, travelled + (at.0 - last.0).hypot(at.1 - last.1)));
        true
    }

    /// **A body press that did not travel selects the cell under it** (or clears it, when it is the
    /// same cell); a press that dragged was a pan and selects nothing.
    pub fn release(&mut self, at: (f64, f64), chrome: &MapChrome) -> bool {
        let Some((_, travelled)) = self.drag.take() else {
            return false;
        };
        if travelled > 4.0 {
            return true;
        }
        let picked = self.view_rect(chrome).and_then(|rect| {
            let terrain = self.survey.terrain()?;
            let cell = terrain.cell_at_pointer(rect, at, &self.camera)?;
            Some((cell, terrain.world_xyz(cell), terrain.world_of(cell)))
        });
        let Some((cell, cursor, world)) = picked else {
            self.cursor = None;
            self.cursor_cell = None;
            return true;
        };
        // **A click is also a hover**: the pointer is over the cell it just selected, so the
        // toolbar's readout names that cell rather than the one it was over before.
        self.cursor_cell = Some(cell);
        self.cursor = cursor;
        self.selection = if self.selection == Some(world) {
            None
        } else {
            Some(world)
        };
        true
    }

    /// **The wheel over the body scales the camera about the pointer**, so the ground under the
    /// cursor stays put — and **the wheel over the gutter scrolls the gutter**, because the column
    /// of controls is taller than the map window this package declares. One entry point for both, so
    /// the app routes one region and the map decides what a wheel there means: zoom over the ground,
    /// scroll over the controls, exactly as the console's own body scrolls its rows.
    pub fn wheel(&mut self, at: (f64, f64), lines: f64, chrome: &MapChrome) -> bool {
        if !lines.is_finite() || lines == 0.0 {
            return false;
        }
        // The gutter first: it is drawn over the body's right edge, so a point inside it must not
        // also zoom the camera the picture behind it is drawn by.
        if let Some(gutter) = self.controls_rect(chrome) {
            if gutter.contains(at) {
                let step = MAP_CONTROLS_ROW_DP * lines;
                let most = self.controls_scroll_max(chrome);
                // **A step lands on the row grid and the end is the end**: a column of controls
                // scrolls a row at a time (so nothing is left straddling an edge it cannot be drawn
                // across), and the last step reaches the document's own bottom whatever the grid
                // says.
                let wanted = if step > 0.0 {
                    (self.gutter_scroll / MAP_CONTROLS_ROW_DP + step / MAP_CONTROLS_ROW_DP).ceil()
                        * MAP_CONTROLS_ROW_DP
                } else {
                    (self.gutter_scroll / MAP_CONTROLS_ROW_DP + step / MAP_CONTROLS_ROW_DP).floor()
                        * MAP_CONTROLS_ROW_DP
                };
                if wanted >= most {
                    self.gutter_scroll = most;
                } else {
                    self.gutter_scroll = wanted.max(0.0);
                }
                return true;
            }
        }
        let Some(rect) = self.view_rect(chrome) else {
            return false;
        };
        if !rect.contains(at) {
            return false;
        }
        self.camera.zoom_by(1.2f64.powf(lines), [at.0, at.1], rect);
        true
    }

    /// **The pointer's cell** for the toolbar's readout, through the camera.
    pub fn hover(&mut self, at: (f64, f64), chrome: &MapChrome) -> bool {
        let picked = self.view_rect(chrome).and_then(|rect| {
            let terrain = self.survey.terrain()?;
            let cell = terrain.cell_at_pointer(rect, at, &self.camera)?;
            Some((cell, terrain.world_xyz(cell)))
        });
        match picked {
            Some((cell, cursor)) => {
                self.cursor = cursor;
                self.cursor_cell = Some(cell);
                true
            }
            None => {
                self.cursor = None;
                self.cursor_cell = None;
                false
            }
        }
    }

    /// **Forget the pointer's cell** when it leaves the map body, so the toolbar cannot keep
    /// describing a cell the pointer is no longer over.
    pub fn pointer_left(&mut self) {
        self.cursor = None;
        self.cursor_cell = None;
        self.drag = None;
    }

    // ---------------------------------------------------------------------------------------
    // The controls: the toolbar's strip and the gutter's column.
    // ---------------------------------------------------------------------------------------

    /// **The map's controls, laid out from the XML document**, scrolled to the gutter's own offset.
    /// `None` when the map window is not open or the document does not lay out; the terrain still
    /// draws, so a broken control document cannot take the map with it.
    pub fn controls(&self, chrome: &MapChrome) -> Option<Rendered> {
        let rect = self.controls_rect(chrome)?;
        let document = Document::parse(super::webview::MAP_CONTROLS).ok()?;
        super::webview::render(
            &document,
            rect,
            &Palette::declared(),
            Some(&chrome.selected),
            self.gutter_scroll,
        )
        .ok()
    }

    /// **How far the gutter can be scrolled**: the document's own height less the column it is drawn
    /// in, or zero when it already fits. Read from the layout rather than kept as a second number, so
    /// the clamp cannot disagree with what the renderer drew.
    pub fn controls_scroll_max(&self, chrome: &MapChrome) -> f64 {
        let Some(rect) = self.controls_rect(chrome) else {
            return 0.0;
        };
        let Ok(document) = Document::parse(super::webview::MAP_CONTROLS) else {
            return 0.0;
        };
        match super::webview::render(
            &document,
            rect,
            &Palette::declared(),
            Some(&chrome.selected),
            0.0,
        ) {
            // The renderer's content box is the gutter less its own 8 dp padding top and bottom,
            // which `controls_rect` does not know about.
            Ok(rendered) => (rendered.content_dp - (rect.height_dp - 16.0)).max(0.0),
            Err(_) => 0.0,
        }
    }

    /// The action under `at`, if the point is on a map control.
    pub fn control_at(&self, chrome: &MapChrome, at: (f64, f64)) -> Option<String> {
        self.controls(chrome)?
            .buttons
            .into_iter()
            .find(|button| button.rect.contains(at))
            .map(|button| button.action)
    }

    /// **The toolbar as shell elements**, or a closed window's empty list. A toolbar that cannot lay
    /// out is an error, for the same reason the controls' is.
    pub fn toolbar_elements(
        &self,
        palette: &Palette,
        chrome: &MapChrome,
    ) -> Result<Vec<Element>, String> {
        let Some(rect) = self.toolbar_rect(chrome) else {
            return Ok(Vec::new());
        };
        let Some(facts) = self.facts(chrome) else {
            return Ok(Vec::new());
        };
        let document = Document::parse(super::webview::MAP_TOOLBAR)?;
        super::webview::render_toolbar(&document, rect, palette, &facts)
            .map(|rendered| rendered.elements)
    }

    /// The controls as shell elements, for a caller that composes them **after** the cell raster. The
    /// gutter is outside every frame's footprint, but drawing after the raster keeps the two
    /// orderings the same whatever a package declares.
    ///
    /// **A closed map is an empty list; a control that cannot lay out is an error.** The old
    /// signature answered `Vec` in both cases, so a palette refusal became a silently blank gutter —
    /// the failure looked exactly like a window with no controls.
    pub fn controls_elements(
        &self,
        palette: &Palette,
        chrome: &MapChrome,
    ) -> Result<Vec<Element>, String> {
        let Some(rect) = self.controls_rect(chrome) else {
            return Ok(Vec::new());
        };
        let document = Document::parse(super::webview::MAP_CONTROLS)?;
        super::webview::render(
            &document,
            rect,
            palette,
            Some(&chrome.selected),
            self.gutter_scroll,
        )
        .map(|rendered| rendered.elements)
    }

    // ---------------------------------------------------------------------------------------
    // Drawing the body.
    // ---------------------------------------------------------------------------------------

    /// **The window's ground, its marks and the player's own marker**, in the order they compose.
    ///
    /// A window with no survey yet draws nothing: the frame's declared cells are the fallback
    /// picture, and they are still in `Mount::frame_space` for the raster to draw.
    pub fn ground_elements(&mut self, chrome: &MapChrome) -> Vec<Element> {
        let Some(rect) = self.view_rect(chrome) else {
            return Vec::new();
        };
        // **The showing frame's own feature token drives the view filter**, so the picture and the
        // toolbar's `VIEW` fact always name the same selection.
        let filter = ViewFilter::of_token(chrome.feature.as_deref());
        self.survey.body_elements(
            &self.camera,
            rect,
            &filter,
            self.selection,
            self.cursor_cell,
        )
    }

    // ---------------------------------------------------------------------------------------
    // The verbs.
    // ---------------------------------------------------------------------------------------

    /// **Run one of the map window's own verbs**, or answer `None` for an action that is not this
    /// window's to run.
    ///
    /// The one vocabulary is the gutter's, the keyboard's, the console's and the scripted pointer's:
    /// a button's `action`, the key the app mapped to it, a `map <verb>` and a step in a scripted
    /// run all land here, so a key and a button cannot mean two different moves. The camera's own
    /// verbs are this window's; a **view token** (`plan ocean`, or a bare `ocean`) belongs to the
    /// package's frames and is [`super::mount::Mount`]'s to select, which is why it comes back as
    /// `None` rather than as a refusal.
    pub fn action(&mut self, action: &str, chrome: &MapChrome) -> Option<Result<String, String>> {
        let body = self.view_rect(chrome);
        let centre = body.map(|rect| [rect.x + rect.width_dp / 2.0, rect.y + rect.height_dp / 2.0]);
        Some(match action {
            "zoom in" | "zoom +" => {
                if let (Some(body), Some(centre)) = (body, centre) {
                    self.camera.zoom_by(2.0, centre, body);
                }
                Ok(format!("zoom {}", self.zoom()))
            }
            "zoom out" | "zoom -" => {
                if let (Some(body), Some(centre)) = (body, centre) {
                    self.camera.zoom_by(0.5, centre, body);
                }
                Ok(format!("zoom {}", self.zoom()))
            }
            "zoom reset" => {
                self.camera.pixels_per_m = MapCamera::default().pixels_per_m;
                Ok("zoom 0".to_string())
            }
            // **Moving the view over the map**, in the screen's own two axes: the ground slides the
            // way the key points, which is the same gesture as a pointer drag the *other* way
            // ([`MapCamera::pan_by_screen`]) — and one step is a screen distance, so a key carries
            // the same share of the window at every zoom.
            "move up" | "move down" | "move left" | "move right" => {
                let Some(body) = body else {
                    return Some(Err(
                        "the map window is not placed: there is nothing to move over".to_string(),
                    ));
                };
                let (dx, dy) = match action {
                    "move up" => (0.0, MAP_MOVE_STEP_DP),
                    "move down" => (0.0, -MAP_MOVE_STEP_DP),
                    "move left" => (MAP_MOVE_STEP_DP, 0.0),
                    _ => (-MAP_MOVE_STEP_DP, 0.0),
                };
                self.camera.pan_by_screen(dx, dy, body);
                Ok(format!(
                    "centre {:.0},{:.0} m",
                    self.camera.centre_m[0], self.camera.centre_m[1]
                ))
            }
            "rotate left" => {
                self.camera.rotate_by(-1.0);
                Ok(format!("bearing {:.0}°", self.camera.bearing_deg))
            }
            "rotate right" => {
                self.camera.rotate_by(1.0);
                Ok(format!("bearing {:.0}°", self.camera.bearing_deg))
            }
            "pitch up" => {
                self.camera.pitch_by(-1.0);
                Ok(format!("pitch {:.0}°", self.camera.pitch_deg))
            }
            "pitch down" => {
                self.camera.pitch_by(1.0);
                Ok(format!("pitch {:.0}°", self.camera.pitch_deg))
            }
            "recenter" => {
                // **The frame's declared column again** — the same placement the window made when it
                // opened, which is why `recentre` answers with the place the package names and not
                // with wherever the camera had drifted to.
                self.camera_ready = false;
                Ok("recentre".to_string())
            }
            _ => return None,
        })
    }
}
