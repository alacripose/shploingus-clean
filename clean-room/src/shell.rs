//! **The shell — the window, the mount, the design system, and the frame.**
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 (U8) and §5's slice 7 (*"U8: the window, the
//! mount, the design system"*, whose acceptance check is *a **visibly** drawn frame; pointer ownership
//! read once*), `docs/SHELL-PORT.md` §1 and §4 (the machinery's shape, and the four-bucket rule for
//! every value that gets drawn), `docs/UI-SPEC.md` §1–§3 and §8 (one kind of window; the shell and the
//! HUD as **two layers of one window set**; the pane tree; the four buckets), `docs/DESIGN-SPEC.md` §3
//! (the slot roster is content, scheduled after the world is playable) and the design-doctrine rulings
//! `docs/UI-SPEC.md` §8 quotes from `docs/LEDGER.md` (C-35, C-37, C-55 §55.1).
//!
//! **What this module is, and what it refuses to be.** It is the clean room's own window machinery: a
//! [`Mount`]-shaped pane tree driven by a declared [`Recipe`] (the windows are **data** the shell
//! mounts, not a class with a draw method — `docs/UI-SPEC.md` §1), the shell/HUD **layer toggle**, the
//! one pointer-ownership decision, the admitted [`CONSTANTS`] table, and a [`Canvas`] that draws the
//! frame. It is **not** a second registry — the recipe is the only window list, and `Mount` adds no
//! window it was not handed; **not** a second pane tree — this module *is* the tree, the way
//! `docs/SHELL-PORT.md` §4.2 warns a duplicate would be the defect; and **not** a renderer of the
//! world: `docs/SHELL-PORT.md` §6 D-3 rules that *"the rendered world is the background; the desktop
//! draws nothing"*, so this frame draws chrome and readouts and names the world rather than faking one.
//!
//! The panes read the **systems already built** through their own public constructors and accessors —
//! the reference world, the material registry, the stage roster, the geometry ladder and the light
//! table — so what a person sees through the frame is real state, and a refusal is printed **by name**
//! rather than replaced by a plausible number (P2). Nothing here imports the legacy tree (P7): the
//! witness is `crates/shploingus`, `crates/voxel-ui` and `crates/voxel-client/src/shell`, and not one
//! line of them was read.
//!
//! **Owed, named, and not silently missing:** the undock operation, the splitter-drag resize, the
//! console's five graded levels, the toast's final form, and the `gholtzap/lintapple` **P8** gate —
//! which the record already assigns to **U13** (*"the shell completed, and its gholtzap/lintapple
//! gate"*, `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 13), and which this slice therefore does **not**
//! take: U8 is the shell, U13 is *the shell completed* plus that gate. `clean-room/specs/010-u8-shell/`
//! states that ownership plainly, so it is deferred on the record rather than silently.

use std::collections::BTreeMap;

use crate::digest::Digest;
use crate::light::{
    CHANNEL_COUNT, MOON_LUMENS_PER_WATT, NIGHT_SKY_ILLUMINANCE_LX, SOLAR_CONSTANT_W_M2,
    VISIBLE_HI_NM, VISIBLE_LO_NM,
};
use crate::material::MaterialId;
use crate::mesh::{Budget, Effort, Ladder, Level, Mesher, Resident, ResidencyHorizon, WorkLedger, MAX_LEVEL};
use crate::process;
use crate::reference;
use crate::refusal::{Refusal, RefusalKind, RefusalSet};
use crate::systems;
use crate::world::World;

// ---------------------------------------------------------------------------------------------------
// The design system — four buckets, counted by name (`docs/UI-SPEC.md` §8).
// ---------------------------------------------------------------------------------------------------

/// One of the **four buckets** every drawn value must be admitted into (`docs/UI-SPEC.md` §8, following
/// `voxel-ui`'s C-35). A value in none of them is not drawn: it is refused by name. The fourth bucket is
/// the one that can grow silently, so the module pins **its** count in a test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bucket {
    /// An MD1 rule, with the rule named in the row's source.
    Md1Rule,
    /// An MD1 example — a worked value MD1 itself prints.
    Md1Example,
    /// A platform translation: a value this build's target forces, declared rather than implied.
    PlatformTranslation,
    /// A project extension: a value this project adds, **counted** so it cannot grow unnoticed.
    ProjectExtension,
}

impl Bucket {
    /// The four buckets, in the order `docs/UI-SPEC.md` §8 states them.
    pub const ALL: [Bucket; 4] = [
        Bucket::Md1Rule,
        Bucket::Md1Example,
        Bucket::PlatformTranslation,
        Bucket::ProjectExtension,
    ];
}

/// One admitted constant: the value, the bucket it was admitted into, and the **source** that admits it.
/// A row with no source is not a row (P1).
#[derive(Clone, Copy, Debug)]
pub struct Constant {
    pub name: &'static str,
    pub value: f64,
    pub bucket: Bucket,
    pub source: &'static str,
}

/// **The admitted table.** Every value the frame draws comes from here, so there are no literals in the
/// drawing and an unadmitted name refuses by name ([`constant`]) rather than being filled.
pub const CONSTANTS: &[Constant] = &[
    Constant {
        name: "ink_primary_percent",
        value: 100.0,
        bucket: Bucket::Md1Rule,
        source: "docs/UI-SPEC.md §8.1 (the ink ladder's first step)",
    },
    Constant {
        name: "ink_secondary_percent",
        value: 70.0,
        bucket: Bucket::Md1Rule,
        source: "docs/UI-SPEC.md §8.1",
    },
    Constant {
        name: "ink_tertiary_percent",
        value: 50.0,
        bucket: Bucket::Md1Rule,
        source: "docs/UI-SPEC.md §8.1",
    },
    Constant {
        name: "divider_percent",
        value: 12.0,
        bucket: Bucket::Md1Rule,
        source: "docs/UI-SPEC.md §8.1",
    },
    Constant {
        name: "contrast_floor_ratio",
        value: 4.5,
        bucket: Bucket::Md1Rule,
        source: "docs/UI-SPEC.md §8.1 (MD1 rule, measured with WCAG arithmetic: LEDGER C-37)",
    },
    Constant {
        name: "corner_radius_dp",
        value: 2.0,
        bucket: Bucket::Md1Rule,
        source: "docs/UI-SPEC.md §8.1",
    },
    Constant {
        name: "target_dp",
        value: 48.0,
        bucket: Bucket::Md1Rule,
        source: "docs/UI-SPEC.md §8.1 (a touch target is at least 48 dp)",
    },
    Constant {
        name: "separation_dp",
        value: 8.0,
        bucket: Bucket::Md1Rule,
        source: "docs/UI-SPEC.md §8.1",
    },
    Constant {
        name: "pointer_target_dp",
        value: 40.0,
        bucket: Bucket::Md1Example,
        source: "docs/SHELL-PORT.md §6 D-1 (MD1 §4.5: a 20 dp glyph in a 40 dp target)",
    },
    Constant {
        name: "pointer_glyph_dp",
        value: 20.0,
        bucket: Bucket::Md1Example,
        source: "docs/SHELL-PORT.md §6 D-1 (MD1 §4.5)",
    },
    Constant {
        name: "panel_opacity",
        value: 0.93,
        bucket: Bucket::ProjectExtension,
        source: "docs/UI-SPEC.md §8.1 (derived: the least opacity keeping the ladder legible; C-35)",
    },
    Constant {
        name: "elev_step",
        value: 0.05,
        bucket: Bucket::ProjectExtension,
        source: "docs/UI-SPEC.md §8.2 (LEDGER C-55 §55.1: raised = lighter)",
    },
    Constant {
        name: "elev_max_level",
        value: 4.0,
        bucket: Bucket::ProjectExtension,
        source: "docs/UI-SPEC.md §8.2 (LEDGER C-55 §55.1: above 4 refused, never clamped)",
    },
    Constant {
        name: "pane_floor_dp",
        value: 40.0,
        bucket: Bucket::ProjectExtension,
        source: "docs/UI-SPEC.md §3 (the reference's 40 × 40 floor, adopted)",
    },
    Constant {
        name: "bar_height_dp",
        value: 48.0,
        bucket: Bucket::ProjectExtension,
        source: "docs/UI-SPEC.md §5 (the shell is the upper bar, 48 dp; D-2 ruled one surface, top)",
    },
    Constant {
        name: "cell_px_assumed",
        value: 8.0,
        bucket: Bucket::PlatformTranslation,
        source: "this build draws into a character grid; the cell's size is a declared assumption",
    },
];

/// The admitted value of a named constant, or a refusal **by name**. This is the one door to the table,
/// so a drawing that wants a number it cannot find is refused rather than filled with a literal.
pub fn constant(name: &str) -> Result<f64, Refusal> {
    CONSTANTS
        .iter()
        .find(|row| row.name == name)
        .map(|row| row.value)
        .ok_or_else(|| {
            Refusal::new(
                RefusalKind::ConstantUnadmitted,
                format!(
                    "`{name}` is in none of docs/UI-SPEC.md §8's four buckets, so it is refused \
                     rather than drawn"
                ),
            )
        })
}

/// How many rows each bucket holds. The extension count is pinned by a test, because it is the bucket
/// that can grow without anyone noticing (`docs/UI-SPEC.md` §8).
pub fn bucket_counts() -> [usize; 4] {
    Bucket::ALL.map(|bucket| CONSTANTS.iter().filter(|row| row.bucket == bucket).count())
}

// ---------------------------------------------------------------------------------------------------
// The declaration — a window is data, not a class with a draw method (`docs/UI-SPEC.md` §1–§2).
// ---------------------------------------------------------------------------------------------------

/// One of the **six** lane hues a window reports through (`docs/UI-SPEC.md` §2, H-7.25). A text frame
/// names the lane rather than colouring it, which is `docs/UI-SPEC.md` §9's rule — *"every view has a
/// text form"* — applied to the lanes themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lane {
    Frame,
    Control,
    Tab,
    World,
    Beacon,
    Motion,
}

impl Lane {
    /// The six hues, in the order `docs/UI-SPEC.md` §2 names them. The six **is** the ruling.
    pub const ALL: [Lane; 6] = [
        Lane::Frame,
        Lane::Control,
        Lane::Tab,
        Lane::World,
        Lane::Beacon,
        Lane::Motion,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Lane::Frame => "Frame",
            Lane::Control => "Control",
            Lane::Tab => "Tab",
            Lane::World => "World",
            Lane::Beacon => "Beacon",
            Lane::Motion => "Motion",
        }
    }
}

/// What a window shows. A closed enum, so a window cannot invent a content kind the shell has no reader
/// for — and each variant reads a **system already built** ([`Content::lines`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Content {
    /// `systems` — the unified-systems registry.
    Registry,
    /// `reference` + `world` — the reference world's identity and digests.
    World,
    /// `reference` — the world's one material registry.
    Materials,
    /// `process` — the declared stage roster.
    Stages,
    /// `mesh` — the render ladder and a derived chunk's geometry.
    Geometry,
    /// `light` — the band table's declared solar and night constants.
    Light,
}

impl Content {
    /// Read the system this window shows, as lines. A refusal from the system is returned **by name**,
    /// never replaced by a plausible value.
    pub fn lines(self) -> Result<Vec<String>, Refusal> {
        match self {
            Content::Registry => Ok(registry_lines()),
            Content::World => world_lines(),
            Content::Materials => Ok(materials_lines()),
            Content::Stages => stages_lines(),
            Content::Geometry => geometry_lines(),
            Content::Light => Ok(light_lines()),
        }
    }
}

/// A window's stable identity — **not a list position** (`docs/UI-SPEC.md` §2, C-51 §51.3's lesson).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(&'static str);

impl WindowId {
    pub const fn new(name: &'static str) -> Self {
        WindowId(name)
    }

    pub const fn name(self) -> &'static str {
        self.0
    }
}

/// A window: data the shell mounts. The shell owns every piece of state (`docs/UI-SPEC.md` §2's
/// load-bearing half), so this struct carries no behaviour beyond its own declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowSpec {
    pub id: WindowId,
    pub title: &'static str,
    pub lane: Lane,
    pub content: Content,
    /// May this window live in the **HUD layer** (`docs/UI-SPEC.md` §4, H-9.8)? A pinned window stays
    /// in the user's view while the shell is hidden.
    pub pinnable: bool,
}

/// **The declared recipe** — the window list the shell mounts. A `Recipe` is the *only* window list; the
/// shell adds no window it was not handed, which is what makes it a mount rather than a hard-coded app
/// registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipe {
    windows: Vec<WindowSpec>,
}

impl Recipe {
    /// Declare a recipe from a window list.
    pub fn of(windows: Vec<WindowSpec>) -> Recipe {
        Recipe { windows }
    }

    /// **The clean room's own recipe**, declared here as data. Six windows, one for each system this
    /// build has something real to show through: the registry, the reference world, its materials, the
    /// stage roster, a derived chunk's geometry, and the light table. Nothing here is a stub — every
    /// window's reader is a call into a built system.
    pub fn declared() -> Recipe {
        Recipe::of(vec![
            WindowSpec {
                id: WindowId::new("registry"),
                title: "systems",
                lane: Lane::Frame,
                content: Content::Registry,
                pinnable: false,
            },
            WindowSpec {
                id: WindowId::new("world"),
                title: "world",
                lane: Lane::World,
                content: Content::World,
                pinnable: true,
            },
            WindowSpec {
                id: WindowId::new("materials"),
                title: "materials",
                lane: Lane::World,
                content: Content::Materials,
                pinnable: false,
            },
            WindowSpec {
                id: WindowId::new("stages"),
                title: "stages",
                lane: Lane::World,
                content: Content::Stages,
                pinnable: false,
            },
            WindowSpec {
                id: WindowId::new("geometry"),
                title: "geometry",
                lane: Lane::World,
                content: Content::Geometry,
                pinnable: false,
            },
            WindowSpec {
                id: WindowId::new("light"),
                title: "light",
                lane: Lane::Beacon,
                content: Content::Light,
                pinnable: true,
            },
        ])
    }

    pub fn windows(&self) -> &[WindowSpec] {
        &self.windows
    }

    pub fn window(&self, id: WindowId) -> Option<&WindowSpec> {
        self.windows.iter().find(|window| window.id == id)
    }
}

// ---------------------------------------------------------------------------------------------------
// The mount — one pane tree, the shell/HUD layers, and the one pointer decision.
// ---------------------------------------------------------------------------------------------------

/// A pane's identity, distinct from the window it holds: a window may occupy several panes, and a
/// window closes when its **last** pane does (`docs/UI-SPEC.md` §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaneId(pub u64);

/// Which way a split runs. `Horizontal` puts the two children **side by side**; `Vertical` stacks them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// The pane tree. One pane, or a split of two subtrees — never a list, because the arrangement **is**
/// data (`docs/UI-SPEC.md` §3) and a list could not carry the nesting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Pane(PaneId),
    Split {
        orientation: Orientation,
        a: Box<Node>,
        b: Box<Node>,
    },
}

/// The two **layers of one window set** (`docs/UI-SPEC.md` §1, H-9.8): the shell, and the HUD that holds
/// the windows declared `pinnable`. Toggling moves between them; it does not build a second window set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Shell,
    Hud,
}

impl Layer {
    pub fn toggled(self) -> Layer {
        match self {
            Layer::Shell => Layer::Hud,
            Layer::Hud => Layer::Shell,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Layer::Shell => "shell",
            Layer::Hud => "hud",
        }
    }
}

/// Who consumes a pointer event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerOwner {
    /// The shell: a shown layer holds the pointer.
    Shell,
    /// The world: the shell is hidden, so the pointer belongs to the game behind it.
    World,
}

impl PointerOwner {
    pub const fn name(self) -> &'static str {
        match self {
            PointerOwner::Shell => "shell",
            PointerOwner::World => "world",
        }
    }
}

/// **The one pointer-ownership decision**, as a value. It is read once per frame ([`Mount::read_pointer`])
/// and then **passed by value** into both consumers — the drawing, which decides the glyph's visibility,
/// and the event handling, which decides the grab — so neither can recompute it and disagree. A frame
/// that showed a cursor it did not own, or owned one it did not show, is unrepresentable here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pointer {
    pub owner: PointerOwner,
    /// Whether the pointer glyph is drawn. It is a consequence of the **same** read: the glyph belongs
    /// to whoever grabs.
    pub visible: bool,
}

impl Pointer {
    /// Whether this decision grants the shell the grab.
    pub fn grabs(self) -> bool {
        self.owner == PointerOwner::Shell
    }
}

/// A rectangle of the frame's character grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
}

impl Area {
    pub const fn new(row: usize, col: usize, rows: usize, cols: usize) -> Area {
        Area { row, col, rows, cols }
    }

    fn split_half(self, orientation: Orientation) -> (Area, Area) {
        match orientation {
            Orientation::Horizontal => {
                let left = self.cols / 2;
                (
                    Area::new(self.row, self.col, self.rows, left),
                    Area::new(self.row, self.col + left, self.rows, self.cols - left),
                )
            }
            Orientation::Vertical => {
                let top = self.rows / 2;
                (
                    Area::new(self.row, self.col, top, self.cols),
                    Area::new(self.row + top, self.col, self.rows - top, self.cols),
                )
            }
        }
    }
}

/// **The mount** — the shell's own state over a declared recipe: one pane tree, the panes' windows, the
/// focus, the layer, and whether a grab is in effect. It is the clean room's single pane tree; there is
/// no second.
#[derive(Clone, Debug)]
pub struct Mount {
    recipe: Recipe,
    root: Option<Node>,
    /// Which window each pane holds. Kept beside the tree because a pane is a **slot** for a window
    /// (`docs/UI-SPEC.md` §3) and a split adds a pane of a window already declared.
    windows: BTreeMap<PaneId, WindowId>,
    focus: Option<PaneId>,
    layer: Layer,
    grab: bool,
    next_pane: u64,
}

impl Mount {
    /// Mount a declared recipe. Every window in the recipe is placed in the tree; a recipe with no
    /// window mounts an empty desktop rather than a fabricated pane.
    pub fn of(recipe: &Recipe) -> Mount {
        let mut mount = Mount {
            recipe: recipe.clone(),
            root: None,
            windows: BTreeMap::new(),
            focus: None,
            layer: Layer::Shell,
            grab: false,
            next_pane: 0,
        };
        let mut panes = Vec::new();
        for window in recipe.windows() {
            let pane = PaneId(mount.next_pane);
            mount.next_pane += 1;
            mount.windows.insert(pane, window.id);
            panes.push(pane);
        }
        mount.root = mount_tree(&panes);
        mount.focus = panes.first().copied();
        mount
    }

    /// The windows the recipe declared — the only window list this mount knows.
    pub fn windows(&self) -> &[WindowSpec] {
        self.recipe.windows()
    }

    /// The panes, in tree order.
    pub fn panes(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            collect_panes(root, &mut out);
        }
        out
    }

    /// The window a pane holds, if the pane is mounted.
    pub fn window_of(&self, pane: PaneId) -> Option<WindowId> {
        self.windows.get(&pane).copied()
    }

    /// The window declaration a pane reads through.
    pub fn spec_of(&self, pane: PaneId) -> Option<&WindowSpec> {
        let window = self.window_of(pane)?;
        self.recipe.window(window)
    }

    pub fn focus(&self) -> Option<PaneId> {
        self.focus
    }

    pub fn layer(&self) -> Layer {
        self.layer
    }

    pub fn grab(&self) -> bool {
        self.grab
    }

    /// Turn the grab on or off. The grab is the shell's only transient input state; the pointer decision
    /// below reads it (once) rather than each consumer reading it.
    pub fn set_grab(&mut self, on: bool) {
        self.grab = on;
    }

    /// Toggle the two layers. The pinned layer is one window set, so this moves the view and nothing else.
    pub fn toggle_layer(&mut self) {
        self.layer = self.layer.toggled();
    }

    /// **The one pointer-ownership decision.** Read once per frame; the value is then passed into the
    /// drawing and the event handling, so the two cannot disagree. The shell owns the pointer only while
    /// its layer is shown, a pane exists to grab, and a grab is in effect; otherwise the world does —
    /// which is the reference's own recorded defect, fixed: *the surfaces survive the shell being hidden*
    /// (`docs/UI-SPEC.md` §1), and so does the world's pointer.
    pub fn read_pointer(&self) -> Pointer {
        let owner = if self.layer == Layer::Shell && !self.panes().is_empty() && self.grab {
            PointerOwner::Shell
        } else {
            PointerOwner::World
        };
        Pointer { owner, visible: owner == PointerOwner::Shell }
    }

    /// Split the pane holding `target`, putting a second pane of the **same** window beside it. The new
    /// pane takes focus. An unknown pane is refused **by name**.
    pub fn split(&mut self, target: PaneId, orientation: Orientation) -> Result<PaneId, Refusal> {
        let window = self
            .window_of(target)
            .ok_or_else(|| pane_unknown(target))?;
        let pane = PaneId(self.next_pane);
        self.next_pane += 1;
        self.windows.insert(pane, window);
        if let Some(root) = self.root.take() {
            self.root = Some(split_at(root, target, pane, orientation));
        }
        self.focus = Some(pane);
        Ok(pane)
    }

    /// Close one pane, collapsing a splitter left with one child. A window whose last pane closes is
    /// closed with it (`docs/UI-SPEC.md` §3).
    pub fn close(&mut self, target: PaneId) -> Result<(), Refusal> {
        if self.window_of(target).is_none() {
            return Err(pane_unknown(target));
        }
        let root = self.root.take().expect("a mounted pane implies a root");
        self.root = remove_pane(root, target);
        self.windows.remove(&target);
        let panes = self.panes();
        if self.focus.map(|p| !panes.contains(&p)).unwrap_or(true) {
            self.focus = panes.first().copied();
        }
        Ok(())
    }

    /// Move focus to a pane holding a window. Refuses an unknown window **by name**.
    pub fn focus_window(&mut self, window: WindowId) -> Result<PaneId, Refusal> {
        let pane = if self.recipe.window(window).is_some() {
            self.panes()
                .into_iter()
                .find(|pane| self.window_of(*pane) == Some(window))
        } else {
            None
        };
        match pane {
            Some(pane) => {
                self.focus = Some(pane);
                Ok(pane)
            }
            None => Err(pane_unknown_window(window)),
        }
    }

    /// The panes this layer draws: the shell layer draws them all, the HUD layer draws the windows
    /// declared `pinnable` (`docs/UI-SPEC.md` §4).
    pub fn visible_panes(&self) -> Vec<PaneId> {
        self.panes()
            .into_iter()
            .filter(|pane| self.is_visible(*pane))
            .collect()
    }

    fn is_visible(&self, pane: PaneId) -> bool {
        match self.layer {
            Layer::Shell => true,
            Layer::Hud => self.spec_of(pane).map(|spec| spec.pinnable).unwrap_or(false),
        }
    }

    /// Lay the visible panes out over `area`, refusing a split that leaves a pane below the declared
    /// **40 × 40 floor** (`docs/UI-SPEC.md` §3) — refused rather than clamped, because a silently
    /// smaller pane is a layout the person asked for and did not get.
    pub fn layout(&self, area: Area) -> Result<Vec<(PaneId, Area)>, Refusal> {
        let floor = floor_cells()?;
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            self.layout_node(root, area, floor, &mut out)?;
        }
        Ok(out)
    }

    fn layout_node(
        &self,
        node: &Node,
        area: Area,
        floor: usize,
        out: &mut Vec<(PaneId, Area)>,
    ) -> Result<(), Refusal> {
        match node {
            Node::Pane(pane) => {
                if self.is_visible(*pane) {
                    if area.rows < floor || area.cols < floor {
                        return Err(Refusal::new(
                            RefusalKind::SplitFloorBreached,
                            format!(
                                "pane #{} would be {} × {} cells, below the declared {} × {} floor \
                                 ({} dp at the assumed {} px per cell) — refused rather than clamped",
                                pane.0,
                                area.cols,
                                area.rows,
                                floor,
                                floor,
                                constant("pane_floor_dp")?,
                                constant("cell_px_assumed")?
                            ),
                        ));
                    }
                    out.push((*pane, area));
                }
                Ok(())
            }
            Node::Split { orientation, a, b } => {
                let (ra, rb) = area.split_half(*orientation);
                self.layout_node(a, ra, floor, out)?;
                self.layout_node(b, rb, floor, out)?;
                Ok(())
            }
        }
    }
}

/// The pane floor in character cells, read out of the admitted table rather than written as a literal.
fn floor_cells() -> Result<usize, Refusal> {
    let floor_dp = constant("pane_floor_dp")?;
    let cell_px = constant("cell_px_assumed")?;
    Ok((floor_dp / cell_px).round() as usize)
}

/// The bar's height in character cells, likewise from the table.
pub fn bar_cells() -> Result<usize, Refusal> {
    let bar_dp = constant("bar_height_dp")?;
    let cell_px = constant("cell_px_assumed")?;
    Ok((bar_dp / cell_px).round() as usize)
}

fn pane_unknown(pane: PaneId) -> Refusal {
    Refusal::new(
        RefusalKind::PaneUnknown,
        format!(
            "pane #{} is not mounted: a pane is addressed by its own identity, never by a list \
             position",
            pane.0
        ),
    )
}

fn pane_unknown_window(window: WindowId) -> Refusal {
    Refusal::new(
        RefusalKind::PaneUnknown,
        format!(
            "no pane holds the window `{}`: the recipe declares no such window, or its last pane has \
             closed",
            window.name()
        ),
    )
}

/// The mount's first tree: the panes chunked into columns of three, each column a vertical stack, the
/// columns side by side. A grid rather than a list, so the declared windows fit a normal frame above the
/// pane floor (`docs/UI-SPEC.md` §3).
fn mount_tree(panes: &[PaneId]) -> Option<Node> {
    let mut columns: Vec<Node> = panes.chunks(3).map(stack).collect();
    let mut root = columns.pop()?;
    while let Some(column) = columns.pop() {
        root = Node::Split {
            orientation: Orientation::Horizontal,
            a: Box::new(column),
            b: Box::new(root),
        };
    }
    Some(root)
}

fn stack(panes: &[PaneId]) -> Node {
    let mut it = panes.iter().copied();
    let first = it.next().expect("a chunk is never empty");
    let mut node = Node::Pane(first);
    for pane in it {
        node = Node::Split {
            orientation: Orientation::Vertical,
            a: Box::new(node),
            b: Box::new(Node::Pane(pane)),
        };
    }
    node
}

fn collect_panes(node: &Node, out: &mut Vec<PaneId>) {
    match node {
        Node::Pane(pane) => out.push(*pane),
        Node::Split { a, b, .. } => {
            collect_panes(a, out);
            collect_panes(b, out);
        }
    }
}

fn contains(node: &Node, target: PaneId) -> bool {
    match node {
        Node::Pane(pane) => *pane == target,
        Node::Split { a, b, .. } => contains(a, target) || contains(b, target),
    }
}

/// Insert `pane` **beside** `target` (`docs/UI-SPEC.md` §3): the target's own node becomes a split of the
/// two, so the new pane lands next to what was split rather than at the end of the tree.
fn split_at(node: Node, target: PaneId, pane: PaneId, orientation: Orientation) -> Node {
    match node {
        Node::Pane(existing) if existing == target => Node::Split {
            orientation,
            a: Box::new(Node::Pane(existing)),
            b: Box::new(Node::Pane(pane)),
        },
        Node::Pane(existing) => Node::Pane(existing),
        Node::Split { orientation: own, a, b } => {
            if contains(&a, target) {
                Node::Split {
                    orientation: own,
                    a: Box::new(split_at(*a, target, pane, orientation)),
                    b,
                }
            } else if contains(&b, target) {
                Node::Split {
                    orientation: own,
                    a,
                    b: Box::new(split_at(*b, target, pane, orientation)),
                }
            } else {
                Node::Split { orientation: own, a, b }
            }
        }
    }
}

/// Remove one pane, collapsing a splitter that is left with a single child.
fn remove_pane(node: Node, target: PaneId) -> Option<Node> {
    match node {
        Node::Pane(pane) => {
            if pane == target {
                None
            } else {
                Some(Node::Pane(pane))
            }
        }
        Node::Split { orientation, a, b } => {
            let a = remove_pane(*a, target);
            let b = remove_pane(*b, target);
            match (a, b) {
                (Some(a), Some(b)) => Some(Node::Split {
                    orientation,
                    a: Box::new(a),
                    b: Box::new(b),
                }),
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// The canvas and the frame.
// ---------------------------------------------------------------------------------------------------

/// A character grid. The frame is drawn into it and rendered as text, so it is **visible** when the
/// binary runs: `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 7's check is *a visibly drawn frame*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canvas {
    rows: usize,
    cols: usize,
    cells: Vec<char>,
}

impl Canvas {
    pub fn new(rows: usize, cols: usize) -> Canvas {
        Canvas { rows, cols, cells: vec![' '; rows * cols] }
    }

    fn put(&mut self, row: usize, col: usize, ch: char) {
        if row < self.rows && col < self.cols {
            self.cells[row * self.cols + col] = ch;
        }
    }

    fn text(&mut self, row: usize, col: usize, text: &str) {
        for (offset, ch) in text.chars().enumerate() {
            self.put(row, col + offset, ch);
        }
    }

    /// Draw a box around `area` in box-drawing characters: the visible title bar and body of a window.
    fn border(&mut self, area: Area) {
        if area.rows < 2 || area.cols < 2 {
            return;
        }
        let right = area.col + area.cols - 1;
        let bottom = area.row + area.rows - 1;
        self.put(area.row, area.col, '┌');
        self.put(area.row, right, '┐');
        self.put(bottom, area.col, '└');
        self.put(bottom, right, '┘');
        for col in area.col + 1..right {
            self.put(area.row, col, '─');
            self.put(bottom, col, '─');
        }
        for row in area.row + 1..bottom {
            self.put(row, area.col, '│');
            self.put(row, right, '│');
        }
    }

    /// The frame as text: one line per row, each padded to the canvas width so the drawn frame is
    /// rectangular and a person sees the box rather than a ragged edge.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(self.rows * (self.cols + 1));
        for row in 0..self.rows {
            let line: String = self.cells[row * self.cols..(row + 1) * self.cols].iter().collect();
            out.push_str(&line);
            out.push('\n');
        }
        out
    }
}

/// **Draw the frame.** The pointer decision is **taken by value**, not recomputed here, which is what
/// makes "read once" a property of the code's shape rather than a promise: the drawing cannot re-read
/// what it was given, so the glyph's visibility and the grab cannot disagree.
pub fn frame(mount: &Mount, area: Area, pointer: Pointer) -> Result<String, Refusal> {
    let bar = bar_cells()?;
    if area.rows <= bar {
        return Err(Refusal::new(
            RefusalKind::SplitFloorBreached,
            format!("a frame of {} rows cannot hold the {bar}-cell bar and any pane", area.rows),
        ));
    }
    let mut canvas = Canvas::new(area.rows, area.cols);
    draw_bar(mount, &mut canvas, area, pointer)?;
    if pointer.visible {
        // The glyph is drawn by whoever owns it — the same read that granted the grab.
        canvas.text(
            area.row + 2,
            area.col,
            &format!(
                "pointer: {} (owned by the same read that grants the grab)",
                pointer.owner.name()
            ),
        );
    }
    let mut rects = mount.layout(Area::new(area.row + bar, area.col, area.rows - bar, area.cols))?;
    rects.sort_by_key(|(pane, _)| *pane);
    for (pane, rect) in rects {
        draw_pane(mount, &mut canvas, pane, rect)?;
    }
    Ok(canvas.render())
}

fn draw_bar(
    mount: &Mount,
    canvas: &mut Canvas,
    area: Area,
    pointer: Pointer,
) -> Result<(), Refusal> {
    let bar = bar_cells()?;
    let bar_dp = constant("bar_height_dp")?;
    let cell_px = constant("cell_px_assumed")?;
    let floor = floor_cells()?;
    let title = format!(
        "shploingus — the shell (slice 7). layer={} pointer_owner={} glyph={} grab={} | windows {} \
         panes {} focus {}",
        mount.layer().name(),
        pointer.owner.name(),
        pointer.visible,
        mount.grab(),
        mount.windows().len(),
        mount.panes().len(),
        mount
            .focus()
            .map(|pane| format!("#{}", pane.0))
            .unwrap_or_else(|| "none".to_string()),
    );
    canvas.text(area.row, area.col, &title);
    // The two layers are named on the bar, because a text frame cannot colour them and a hidden shell
    // must still say where the person is (docs/UI-SPEC.md §9: every view has a text form) — and beside
    // them the bar states its own geometry's **derivation from the table**, so a reader can check it.
    let second = format!(
        "layers: shell | hud ({} pinnable) — Escape/F1 | bar {bar} cells ({bar_dp} dp ÷ {cell_px} px) \
         floor {floor}",
        mount.windows().iter().filter(|w| w.pinnable).count()
    );
    canvas.text(area.row + 1, area.col, &second);
    Ok(())
}

fn draw_pane(mount: &Mount, canvas: &mut Canvas, pane: PaneId, area: Area) -> Result<(), Refusal> {
    if area.rows < 3 || area.cols < 4 {
        return Ok(());
    }
    canvas.border(area);
    let title = match mount.spec_of(pane) {
        Some(window) => format!(
            " {} · lane={} · pinnable={} ",
            window.title,
            window.lane.name(),
            window.pinnable
        ),
        None => format!(" pane #{} ", pane.0),
    };
    canvas.text(area.row, area.col + 2, &truncate(&title, area.cols.saturating_sub(4)));
    // The focused pane is marked on its title bar, since a text frame cannot raise it.
    if mount.focus() == Some(pane) && area.cols > 6 {
        canvas.put(area.row, area.col + area.cols - 3, '●');
    }
    let inner_rows = area.rows.saturating_sub(2);
    let inner_cols = area.cols.saturating_sub(2);
    let lines = match mount.spec_of(pane).map(|window| window.content.lines()) {
        Some(Ok(lines)) => lines,
        Some(Err(refusal)) => vec![format!("refused by name: {refusal}")],
        None => vec!["no window".to_string()],
    };
    for (index, line) in lines.iter().take(inner_rows).enumerate() {
        canvas.text(area.row + 1 + index, area.col + 1, &truncate(line, inner_cols));
    }
    if lines.len() > inner_rows && inner_rows > 0 {
        canvas.text(
            area.row + area.rows - 2,
            area.col + 1,
            &truncate(
                &format!(
                    "… {} more line(s) (the pane is a window, not a report)",
                    lines.len() - inner_rows
                ),
                inner_cols,
            ),
        );
    }
    Ok(())
}

fn truncate(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

// ---------------------------------------------------------------------------------------------------
// The readers — each one a call into a system already built, never a mock.
// ---------------------------------------------------------------------------------------------------

fn joined(refusals: RefusalSet) -> Refusal {
    refusals.joined().unwrap_or_else(|| {
        Refusal::new(
            RefusalKind::BodyOwed,
            "a system refused without naming a refusal, which is itself a defect",
        )
    })
}

fn registry_lines() -> Vec<String> {
    let mut out = vec![format!(
        "{} unified systems (docs/CLEAN-ROOM-PLAN-2026-10-11.md §2):",
        systems::SYSTEM_COUNT
    )];
    for system in systems::SYSTEMS {
        let state = system.status.split_whitespace().next().unwrap_or("?");
        out.push(format!("  [{}] {} — {state}", system.id, system.name));
    }
    out
}

fn world_lines() -> Result<Vec<String>, Refusal> {
    let identity = reference::identity()?;
    let world = World::new(identity);
    let ruleset = reference::ruleset();
    Ok(vec![
        format!(
            "identity: seed={} period={}s tick={}",
            identity.seed,
            identity.period.get(),
            world.tick().0
        ),
        format!("truth_digest: {}", world.truth_digest().to_hex()),
        format!("residency_digest: {}", world.residency_digest().to_hex()),
        format!("resident chunks: {}", world.resident_count()),
        format!("ruleset: {}.{}", ruleset.ruleset, ruleset.revision),
    ])
}

fn materials_lines() -> Vec<String> {
    let registry = reference::registry();
    let ids = registry.ids();
    vec![
        format!("admitted materials: {}", registry.len()),
        format!(
            "ids: {}",
            ids.iter().map(|id| id.0.to_string()).collect::<Vec<_>>().join(" ")
        ),
        format!("registry digest: {}", registry.digest().to_hex()),
        "an id the registry has not admitted is refused by name".to_string(),
    ]
}

fn stages_lines() -> Result<Vec<String>, Refusal> {
    let roster = process::roster().map_err(joined)?;
    let names: Vec<&str> = roster.stages().iter().map(|stage| stage.name).take(6).collect();
    Ok(vec![
        format!("declared stages: {}", roster.len()),
        format!("names: {}", names.join(" ")),
        format!("roster digest: {}", roster.digest().to_hex()),
        "a stage body that is owed refuses by name, never zeroes".to_string(),
    ])
}

fn geometry_lines() -> Result<Vec<String>, Refusal> {
    let roster = process::roster().map_err(joined)?;
    let registry = reference::registry();
    let palette = [MaterialId(1), MaterialId(2), MaterialId(3), MaterialId(4), MaterialId(6)];
    let ladder = Ladder::over_palette(registry, &palette, 6, MaterialId(5))?;
    let residency = ResidencyHorizon::m(256.0)?;
    let identity = reference::identity()?;
    let world = World::new(identity);
    let chunk = crate::coordinates::ChunkKey::new(0, 0, 0);
    let level = Level::new(0)?;
    let mesher = Mesher::new(&roster, ladder.clone(), residency);
    let mut resident = Resident::new();
    let mut ledger = WorkLedger::default();
    let mesh = mesher.derive(
        &world,
        chunk,
        level,
        Effort::new(Budget::unlimited(), 1),
        &mut resident,
        &mut ledger,
    )?;
    Ok(vec![
        format!(
            "render ladder: {} levels (max {MAX_LEVEL}, tile up to {})",
            Level::ALL.len(),
            Level::ALL.last().map(|l| l.tile()).unwrap_or(1)
        ),
        format!("palette classes: {}", ladder.classes()),
        format!("chunk (0,0,0): {} faces → {} quads", ledger.faces, mesh.quads().len()),
        format!("{} cells swept, factor {:.2}", ledger.cells, ledger.merge_factor()),
        format!("mesh digest: {}", mesh.digest().to_hex()),
    ])
}

fn light_lines() -> Vec<String> {
    vec![
        format!("solar constant: {SOLAR_CONSTANT_W_M2:.3} W/m²"),
        format!("night sky: {NIGHT_SKY_ILLUMINANCE_LX} lx"),
        format!("channels: {CHANNEL_COUNT} over {VISIBLE_LO_NM:.0}–{VISIBLE_HI_NM:.0} nm"),
        format!("moon: {MOON_LUMENS_PER_WATT:.0} lm/W"),
        "a quantity that cannot be weighed is refused by name (`not_weighable`)".to_string(),
    ]
}

/// The frame's own identity — a digest over the drawn cells. Two frames that differ by one character
/// differ here, so a report can say *what was drawn* rather than *that something was*.
pub fn frame_digest(rendered: &str) -> Digest {
    Digest::of(rendered.as_bytes())
}

/// **The declared frame extent** this build draws into: 100 columns × 30 rows of characters. The
/// terminal's own size is the caller's business; this is the declaration the `shell` mode, the evidence
/// and the tests share, so `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §5 row 7's check is run on the same
/// frame every time and two runs can be compared by digest.
pub fn declared_area() -> Area {
    Area::new(0, 0, 30, 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Area {
        declared_area()
    }

    #[test]
    fn the_constants_are_bucketed_and_the_extension_count_is_pinned() {
        assert_eq!(Bucket::ALL.len(), 4, "the four buckets are the ruling (UI-SPEC §8)");
        assert_eq!(Lane::ALL.len(), 6, "the six lane hues are the ruling (H-7.25)");
        let counts = bucket_counts();
        // Pinned by name, not by a total: the bucket that can grow silently is the one that cannot.
        assert_eq!(counts[0], 8, "md1-rule rows");
        assert_eq!(counts[1], 2, "md1-example rows (the dense-pointer pair)");
        assert_eq!(counts[2], 1, "platform-translation rows (the character cell)");
        assert_eq!(counts[3], 5, "project-extension rows — counted, so it cannot grow unnoticed");
        for row in CONSTANTS {
            assert!(!row.source.is_empty(), "{} is admitted with no source", row.name);
            assert!(row.value.is_finite(), "{} is not a finite value", row.name);
        }
    }

    #[test]
    fn an_unadmitted_constant_is_refused_by_name() {
        let refusal = constant("no_such_row").expect_err("an unadmitted name is refused");
        assert_eq!(refusal.kind(), RefusalKind::ConstantUnadmitted);
        assert_eq!(refusal.kind().name(), "constant_unadmitted");
        assert!(refusal.detail().contains("no_such_row"));
        // And the admitted ones come back with their own values.
        assert_eq!(constant("pane_floor_dp").expect("admitted"), 40.0);
        assert_eq!(constant("cell_px_assumed").expect("admitted"), 8.0);
    }

    #[test]
    fn the_frame_geometry_comes_from_the_table_and_not_from_a_literal() {
        assert_eq!(bar_cells().expect("admitted"), 6, "48 dp ÷ 8 px per cell");
        assert_eq!(floor_cells().expect("admitted"), 5, "40 dp ÷ 8 px per cell");
        let mount = Mount::of(&Recipe::declared());
        let drawn = frame(&mount, area(), mount.read_pointer()).expect("a legal frame");
        assert!(drawn.contains("bar 6 cells (48 dp ÷ 8 px)"), "{drawn}");
        assert!(drawn.contains("floor 5"), "{drawn}");
    }

    #[test]
    fn the_recipe_is_the_only_window_list() {
        let recipe = Recipe::declared();
        let mount = Mount::of(&recipe);
        assert_eq!(mount.windows().len(), recipe.windows().len());
        for window in recipe.windows() {
            assert!(
                mount.windows().iter().any(|w| w.id == window.id),
                "the mount dropped {}, so it is adding its own list, not mounting one",
                window.id.name()
            );
        }
        assert_eq!(mount.panes().len(), recipe.windows().len(), "every declared window is placed");
    }

    #[test]
    fn a_split_puts_a_pane_beside_its_target_and_close_collapses_it() {
        let mut mount = Mount::of(&Recipe::declared());
        let before = mount.panes().len();
        let target = mount.panes()[0];
        let new = mount.split(target, Orientation::Vertical).expect("a known pane splits");
        assert_ne!(new, target);
        assert_eq!(mount.panes().len(), before + 1);
        assert_eq!(mount.focus(), Some(new), "the new pane takes focus");
        assert_eq!(
            mount.window_of(new),
            mount.window_of(target),
            "a split adds a second pane of the same window"
        );
        mount.close(new).expect("the new pane closes");
        assert_eq!(mount.panes().len(), before, "closing collapses what the split added");
        assert!(mount.window_of(new).is_none());
    }

    #[test]
    fn an_unknown_pane_is_refused_by_name() {
        let mut mount = Mount::of(&Recipe::declared());
        let ghost = PaneId(9_999);
        let refusal = mount.split(ghost, Orientation::Horizontal).expect_err("unknown");
        assert_eq!(refusal.kind(), RefusalKind::PaneUnknown);
        assert!(refusal.detail().contains("9999"));
        let refusal = mount.close(ghost).expect_err("unknown");
        assert_eq!(refusal.kind().name(), "pane_unknown");
        let refusal = mount
            .focus_window(WindowId::new("nowhere"))
            .expect_err("a window the recipe never declared");
        assert_eq!(refusal.kind(), RefusalKind::PaneUnknown);
        assert!(refusal.detail().contains("nowhere"));
    }

    #[test]
    fn a_split_below_the_declared_floor_is_refused_by_name() {
        let mut mount = Mount::of(&Recipe::declared());
        for _ in 0..3 {
            let target = mount.panes()[0];
            mount.split(target, Orientation::Horizontal).expect("splits while there is room");
        }
        let narrow = Area::new(0, 0, 30, 12);
        let refusal = mount.layout(narrow).expect_err("too narrow for nine panes at the floor");
        assert_eq!(refusal.kind(), RefusalKind::SplitFloorBreached);
        assert!(
            refusal.detail().contains("below the declared 5 × 5 floor"),
            "{}",
            refusal.detail()
        );
        // And the same mount lays out inside the floor when the area is big enough.
        assert!(mount.layout(area()).is_ok());
    }

    #[test]
    fn the_pointer_decision_is_one_read_that_governs_both_the_grab_and_the_glyph() {
        let mut mount = Mount::of(&Recipe::declared());
        // Hidden layer: the world owns the pointer, and the glyph follows the owner.
        mount.toggle_layer();
        let hud = mount.read_pointer();
        assert_eq!(hud.owner, PointerOwner::World);
        assert!(!hud.visible, "a glyph the shell does not own is not drawn");
        assert!(!hud.grabs());
        mount.toggle_layer();
        // Shown layer, no grab: still the world's pointer, because nothing is grabbed.
        assert_eq!(mount.read_pointer().owner, PointerOwner::World);
        mount.set_grab(true);
        let shell = mount.read_pointer();
        assert_eq!(shell.owner, PointerOwner::Shell);
        assert!(shell.visible, "the owner draws its own glyph");
        // **Read once**: it is a pure value, and the drawing takes it by value rather than re-reading.
        assert_eq!(mount.read_pointer(), shell);
        let drawn = frame(&mount, area(), shell).expect("a legal frame");
        assert!(drawn.contains("pointer: shell"), "{drawn}");
        assert!(drawn.contains("(owned by the same read that grants the grab)"), "{drawn}");
        // With the world owning the pointer the glyph is absent from the frame entirely.
        mount.set_grab(false);
        let world = mount.read_pointer();
        assert_eq!(world.owner, PointerOwner::World);
        assert!(!world.visible);
        assert!(!frame(&mount, area(), world).expect("legal").contains("pointer: shell"));
    }

    #[test]
    fn the_frame_is_visibly_drawn_and_shows_real_system_state() {
        let mount = Mount::of(&Recipe::declared());
        let drawn = frame(&mount, area(), mount.read_pointer()).expect("a legal frame");
        // A visible frame: a rectangular canvas, box-drawing borders, and the title bar.
        assert_eq!(drawn.lines().count(), 30);
        for line in drawn.lines() {
            assert_eq!(line.chars().count(), 100, "the frame is rectangular");
        }
        for glyph in ['┌', '┐', '└', '┘', '─', '│'] {
            assert!(drawn.contains(glyph), "the frame draws its windows with {glyph}");
        }
        assert!(drawn.contains("shploingus — the shell"), "the bar names the view");
        // **Real state, not a mock**: the world's own seed and the geometry's derived numbers come
        // through the panes exactly as the systems report them.
        let identity = reference::identity().expect("the reference identity");
        let truth = World::new(identity).truth_digest().to_hex();
        assert!(drawn.contains(&format!("seed={}", identity.seed)), "{drawn}");
        // The digest is longer than a pane is wide, so the pane shows its head; the head is enough to
        // be a digest nobody else could have drawn.
        assert!(
            drawn.contains(&truth[..32]),
            "the frame shows the world's own truth digest: {}",
            &truth[..32]
        );
        // The geometry pane's own lines, as the systems report them, are what the frame draws: the pane
        // is a window, so its first three lines' heads are what is visible, and they carry the
        // derivation's own numbers rather than a literal's.
        for line in geometry_lines().expect("the geometry pane's own lines").iter().take(3) {
            let head = &line[..line.len().min(30)];
            assert!(drawn.contains(head), "the frame omits `{head}`: {drawn}");
        }
        assert!(drawn.contains("admitted materials:"), "{drawn}");
        assert!(drawn.contains("solar constant: 1361.000 W/m²"), "{drawn}");
        // Every one of the six declared windows is drawn, with its title from the recipe.
        for window in Recipe::declared().windows() {
            assert!(drawn.contains(window.title), "the frame omits the {} window", window.title);
        }
    }

    #[test]
    fn the_layers_toggle_moves_the_pinnable_windows_and_nothing_else() {
        let recipe = Recipe::declared();
        let mut mount = Mount::of(&recipe);
        let all = mount.visible_panes().len();
        assert_eq!(all, recipe.windows().len(), "the shell layer draws every window");
        mount.toggle_layer();
        assert_eq!(mount.layer(), Layer::Hud);
        let pinned = recipe.windows().iter().filter(|w| w.pinnable).count();
        assert_eq!(mount.visible_panes().len(), pinned, "the hud layer draws the pinnable windows");
        assert!(pinned < all, "the two layers are not the same window set");
        let drawn = frame(&mount, area(), mount.read_pointer()).expect("a hud frame");
        assert!(drawn.contains("layer=hud"), "{drawn}");
        assert!(drawn.contains("layers: shell | hud (2 pinnable)"), "{drawn}");
        mount.toggle_layer();
        assert_eq!(mount.visible_panes().len(), all, "the toggle is a move, not a rebuild");
    }

    #[test]
    fn the_same_state_draws_the_same_frame() {
        let mount = Mount::of(&Recipe::declared());
        let pointer = mount.read_pointer();
        let first = frame(&mount, area(), pointer).expect("legal");
        let second = frame(&mount, area(), pointer).expect("legal");
        assert_eq!(first, second, "the frame is a pure function of the mount's state");
        assert_eq!(frame_digest(&first), frame_digest(&second));
        assert_ne!(frame_digest(&first), frame_digest(&(first.clone() + " ")));
    }
}
