//! **The arrangement: panes, splitters, and the tree they make.**
//!
//! # What the spec asks for, in the spec's own words
//!
//! `UI-SPEC.md` §3, *"the pane tree — panes are slots, arrangements are trees"*:
//!
//! * A **pane** is a slot holding a **window id**. A window with no split is one pane.
//! * `split(pane, h|v)` inserts into an existing splitter when the orientation agrees,
//!   **wraps** when it opposes, and wraps the root when the target is the root.
//! * `undock(pane)` produces a new standalone window; `close(pane)` collapses one-child
//!   splitters back up; **a window closes when its last pane does**.
//! * **40 × 40 floor on every splitter and pane** and proportional stretch factors — *"the
//!   reference's hard-won detail: without it, nested splitters' `minimumSizeHint`s accumulate
//!   and the layout locks."*
//! * **Free-floating windows are allowed** (H-7.24): a window may ignore the grid entirely.
//! * **The arrangement is data**, so it is **a loadable package** — *"the first thing the
//!   Mods window has to show. The shell holds only transient state."*
//!
//! # Why the reference was read and not the spec's summary
//!
//! **`docs/portal_simulator.py` is in this tree** (4,252 lines, byte-identical to
//! `F:\Documents\simulators\` per H-7.19) and its `split_pane` is the implementation the
//! spec is summarising. Reading it produced **four details the spec's prose does not carry**,
//! and every one of them is a way a re-derivation goes wrong:
//!
//! | the reference does | the spec's summary says | what a re-derivation misses |
//! |---|---|---|
//! | `setSizes([1000 // num] * num)` on insert | *"proportional stretch factors"* | **integer division**: 3 panes get `333/333/333` and one unit goes unallocated. Equal division is the rule; the rounding is Qt's. |
//! | on a wrap, `parent.setSizes(old_sizes)` after re-parenting | *"wraps when it opposes"* | **the outer split's sizes are restored**. Without that line a nested split silently resizes its ancestors — the reference's actual reason for capturing `old_sizes` first. |
//! | `minimumSizeHint` on **both** `PortalSplitter` and `WindowPane`, plus `setChildrenCollapsible(False)` | *"40 × 40 floor on every splitter and pane"* | the floor is **two classes**, and `setChildrenCollapsible(False)` is what stops a child being crushed below the floor by its parent. |
//! | `_cleanup_splitter` **recurses upward** (`self._cleanup_splitter(p)`) | *"collapses one-child splitters back up"* | *"back up"* means **recursively**, not one level. The recursion is the whole rule. |
//! | `dock_window` **reverses** the pane list when docking `before`/`top` | — | panes land in the order they were arranged; reversal is load-bearing and appears nowhere in the spec. |
//!
//! **The `[1000 // num]` rounding is modelled deliberately, not fixed.** Equal division is
//! carried as [`f32`] stretch factors, which divide exactly — so the reference's lost unit is
//! *not* reproduced, and [`Arrangement::check_divisible`] exists to name where the reference
//! would have lost one. Reproducing an integer-division artefact in a data model that has no
//! integers to divide would be cargo-culting a Qt detail into the wrong medium.
//!
//! # Ownership: the load-bearing half
//!
//! UI-SPEC §2: *"the window declares; the shell owns `WindowState` … A window that draws its
//! own chrome is a window the shell cannot theme."* This module is the **declaration side**:
//! it holds pane membership, orientation and stretch, and it has **no pixels, no geometry and
//! no drawing**. Geometry belongs to the shell's transient state, and the arrangement is data
//! precisely so that a package can carry it and the shell can hold nothing.

use std::collections::BTreeSet;

use voxel_ui::{UiRefusal, UiResult};

/// **40 × 40, the floor on every splitter and pane.** UI-SPEC §3, from the reference.
///
/// **A named constant, because it is the number the spec calls "hard-won"** and the one that
/// silently disappears the moment someone writes a layout by hand. It is a floor on *both*
/// axes: the reference's `QSize(40, 40)` is square, and a pane narrower than 40 is not a pane.
pub const MIN_EXTENT: u32 = 40;

/// How a splitter divides. **`h|v` in the spec's own notation.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Side by side. The spec's `h`.
    Horizontal,
    /// Stacked. The spec's `v`.
    Vertical,
}

impl Orientation {
    pub const ALL: [Orientation; 2] = [Orientation::Horizontal, Orientation::Vertical];

    pub const fn name(self) -> &'static str {
        match self {
            Orientation::Horizontal => "h",
            Orientation::Vertical => "v",
        }
    }

    /// The axis this orientation divides along, as the reference's `QSize` pair.
    ///
    /// **A horizontal splitter constrains width** — its children sit side by side, so each
    /// child's *width* is what the floor applies to. The reference's `minimumSizeHint`
    /// returns `QSize(40, 40)` for both axes and Qt applies it per-axis; naming which axis
    /// is which is what lets [`Arrangement::check_floor`] check the right one.
    pub const fn constrained_axis(self) -> Axis {
        match self {
            Orientation::Horizontal => Axis::Width,
            Orientation::Vertical => Axis::Height,
        }
    }
}

/// Which of the two screen axes a constraint is about.
///
/// **Re-exported from `voxel-ui`, not defined here**, because
/// [`UiRefusal::PaneBelowFloor`] has to name it and this crate depends on that one. A second
/// `Axis` would be two types with one meaning and a `PaneBelowFloor` that could not be
/// constructed from this one — D-07's collision, created by the definition being in the
/// convenient place instead of the place that needs it.
pub use voxel_ui::Axis;

/// Which side of a split the new pane goes on.
///
/// **The reference spells these `"before"`, `"left"`, `"top"` and the opposites**, and treats
/// the two spellings of each side as one case (`if position in ("before", "left", "top")`).
/// The spec says `h|v` and says nothing about sides, so **the reference's two spellings are
/// collapsed into one variant each** and the record of the collapse is here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// The reference's `"before"`, `"left"`, `"top"`.
    Before,
    /// The reference's `"after"`, `"right"`, `"bottom"`.
    After,
}

impl Side {
    pub const ALL: [Side; 2] = [Side::Before, Side::After];

    pub const fn name(self) -> &'static str {
        match self {
            Side::Before => "before",
            Side::After => "after",
        }
    }
}

/// A window's stable identity within an arrangement.
///
/// **A newtype over a `String`, and the reason is UI-SPEC §2's lesson**: *"stable identity,
/// **not a list position** — the reference shell addressed windows as `#3`, which stops meaning
/// anything the moment one closes."* So a pane holds an id and never an index, and this type
/// exists so an index cannot be passed where an id belongs.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(String);

impl WindowId {
    /// An id, or a refusal naming why it is not one.
    ///
    /// **Refused rather than trimmed.** A blank id would be a pane holding nothing, and the
    /// reference's `PanePlaceholder` exists because an empty pane is a *visible state*, not a
    /// silent one — so this returns `Err` and lets the caller decide to show a placeholder
    /// rather than fabricating an id.
    pub fn new(id: impl Into<String>) -> UiResult<Self> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(UiRefusal::EmptyViewWithoutNote {
                what: "a window id",
            });
        }
        Ok(WindowId(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// **Which node of the tree a caller means.**
///
/// **A pane holding a window is named by its window id; an empty slot has no id, so it is named
/// by its path in the tree.** The reference's empty pane is an *unnamed* `WindowPane(app=None)`
/// (2655–2685) — it exists, it lays out, it draws a header and a placeholder, and it has no name
/// at all. [`WindowId`] cannot spell "no name" (a blank id is refused by [`WindowId::new`]), so
/// the address is the identity: the path of child indices from the root — which is the walk the
/// layout pass already makes, so the drawing and the hit tests agree about it by construction.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PaneRef {
    /// A pane holding a window.
    Window(WindowId),
    /// An empty slot, by its path from the root: `[1]` is the root's second child.
    Slot(Vec<usize>),
}

impl PaneRef {
    /// The window this pane holds, if it holds one. A slot holds none.
    pub fn window(&self) -> Option<&WindowId> {
        match self {
            PaneRef::Window(id) => Some(id),
            PaneRef::Slot(_) => None,
        }
    }
}

/// A node in the arrangement tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// A pane holding a window id. **A window with no split is one pane** (§3).
    Pane(WindowId),
    /// **An empty slot** — the reference's `WindowPane(app=None)` (2677–2685).
    ///
    /// It is a pane in every way that matters: it takes a share, it lays out, it draws a header
    /// (`◧  New Split Slot`) and a placeholder, and it can be filled (the reference's `set_app`,
    /// 2697–2711). What it does not have is a window — which is why it carries no id and is
    /// addressed by [`PaneRef::Slot`]'s path instead. **The reference's own state**, not a
    /// placeholder for a missing value.
    Slot,
    /// A splitter dividing its children along one axis.
    Split {
        orientation: Orientation,
        /// `(stretch factor, child)`. **Proportional, per §3.**
        children: Vec<(f32, Node)>,
    },
}

/// The pane tree, and **data, per §3's "a loadable package"**.
///
/// **There is deliberately no `new`.** A window's id is its only constructor argument, and an
/// empty arrangement is not a state the shell can be in — §3 says the arrangement is a
/// package, and a package with no window in it is a package that describes nothing. This is
/// the same rule as `Board::from_chain` and `Feature::witness_of`'s exhaustive match: the
/// illegal state does not compile.
#[derive(Clone, Debug, PartialEq)]
pub struct Arrangement {
    root: Node,
}

impl Arrangement {
    /// An arrangement holding exactly one window.
    pub fn of(window: WindowId) -> Self {
        Arrangement {
            root: Node::Pane(window),
        }
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    /// Every window in the arrangement, in tree order.
    ///
    /// **`BTreeSet`, not a `Vec`.** The reference's `get_all_panes` returns a list whose order
    /// depends on traversal, and `dock_window` then *reverses* it — so an order-sensitive
    /// collection here would make the reversal's effect invisible until it was already wrong.
    /// A set makes "which windows" answerable without pretending to answer "in what order",
    /// which is the question actually asked of a membership test.
    ///
    /// **Slots are not windows**, so they are absent from this set. `get_all_panes` in the
    /// reference returns empty panes too; this is the question "which windows", and a caller
    /// that wants every pane asks [`Arrangement::panes`].
    pub fn windows(&self) -> BTreeSet<WindowId> {
        let mut out = BTreeSet::new();
        collect_panes(&self.root, &mut out);
        out
    }

    /// How many panes the arrangement holds — **windows and empty slots alike**, because the
    /// reference counts panes and an empty slot is one (its `update_titlebar_and_taskbar`
    /// shows `⇱` when `len(panes) > 1`, empty panes included, 3141–3164).
    pub fn panes(&self) -> usize {
        count_panes(&self.root)
    }

    /// `split(pane, orientation, side)` — **the spec's `split(pane, h|v)`**, with the side the
    /// reference's `position` carries.
    ///
    /// **The three cases are the reference's three cases**, because the spec's summary of them
    /// ("inserts when the orientation agrees, wraps when it opposes, and wraps the root when the
    /// target is the root") is exactly three and they are not the same three:
    ///
    /// 1. **the root is a pane** → the new pane and the old one become a fresh splitter, and
    ///    the tree gains a root. The reference sizes it `[500, 500]`.
    /// 2. **the root is a splitter of the same orientation** → **insert** at the front or back
    ///    of the root's children. The reference re-divides the whole root.
    /// 3. **the target sits inside a splitter of the opposite orientation** → **wrap**: the
    ///    target is lifted out into a fresh sub-splitter, and that sub-splitter is put back at
    ///    the target's old index. **The outer split's stretch is preserved**, which is the
    ///    reference's `parent.setSizes(old_sizes)` and the one line a re-derivation drops.
    ///
    /// **`None` for the target means the root**, as in the reference's `split_pane(None, …)`.
    pub fn split(
        &mut self,
        target: Option<&WindowId>,
        orientation: Orientation,
        side: Side,
        new_pane: WindowId,
    ) -> UiResult<WindowId> {
        let target = target.cloned().map(PaneRef::Window);
        let created = self.split_pane(target.as_ref(), orientation, side, Some(&new_pane))?;
        debug_assert!(
            matches!(&created, PaneRef::Window(id) if id == &new_pane),
            "a window split creates the pane that window was named for"
        );
        Ok(new_pane)
    }

    /// **The one split, of either kind of pane** — the reference's `split_pane(target_pane,
    /// orientation, position, new_pane)` (2919–3012), where both the target and the new pane can
    /// be a window pane or an empty one (2920–2921).
    ///
    /// * **The target** is `None` for the root (the reference's `split_pane(None, …)`), a
    ///   window pane by its id, or a slot by its path. A target the tree does not hold is
    ///   refused *by name*: the reference falls back to an outer split when `target_pane` is not
    ///   found, because a Qt widget that has been re-parented away is a thing that happens
    ///   silently — a data model can refuse instead.
    /// * **The new pane** is `Some(id)` for a pane holding that window or `None` for an empty
    ///   [`Node::Slot`]: the pane header's `◧`/`⬓` and the outer edge triggers create slots
    ///   (2534–2552, 2874–2883), and a dock creates panes holding windows.
    ///
    /// The returned [`PaneRef`] is the new pane's **address in the tree that was built** — for
    /// a slot, the path the layout will stamp — so a caller can fill or address it without a
    /// second walk.
    pub fn split_pane(
        &mut self,
        target: Option<&PaneRef>,
        orientation: Orientation,
        side: Side,
        new_window: Option<&WindowId>,
    ) -> UiResult<PaneRef> {
        if let Some(id) = new_window {
            // **Refused, because the reference does not check and should have.** Two panes
            // holding one window id means the arrangement says the window is in two places,
            // and `close` would then have to guess which. This is a case where reading the
            // reference is not enough — the reference is a UI and can render a contradiction;
            // a data model cannot.
            if self.windows().contains(id) {
                return Err(UiRefusal::DuplicateWindow {
                    window: id.as_str().to_string(),
                });
            }
        }
        // **The target must be somewhere, or the split has nothing to split.**
        match target {
            Some(PaneRef::Window(id)) if !self.windows().contains(id) => {
                return Err(UiRefusal::UnknownWindow {
                    window: id.as_str().to_string(),
                });
            }
            Some(PaneRef::Slot(path)) if self.slot_at(path).is_none() => {
                return Err(UiRefusal::UnknownSlot { slot: path.clone() });
            }
            _ => {}
        }
        // **The target was checked to be a pane the tree holds**, so the walk cannot miss it:
        // a missing target is the bug the checks above exist to refuse, not a case here.
        let aim = Aim::of(target);
        let (root, created) = split_at(
            &self.root,
            aim.as_ref(),
            orientation,
            side,
            new_window,
            &mut Vec::new(),
        )
        .expect("the target was checked to be in the tree, so the split resolves");
        self.root = root;
        Ok(created)
    }

    /// `split(pane, h|v)` **with an empty new pane** — the reference's `split_pane` with
    /// `new_pane=None` (2920–2921), which is what both the pane header's split controls
    /// (`◧`/`⬓`, 2534–2552) and the outer edge triggers (2874–2883) ask for. The new pane is a
    /// [`Node::Slot`]: it holds no window, it lays out and draws like any pane, and it is filled
    /// by [`Arrangement::fill`] or by a drop.
    ///
    /// **`None` for the target means the root**, as in the reference's `split_pane(None, …)`.
    pub fn split_slot(
        &mut self,
        target: Option<&PaneRef>,
        orientation: Orientation,
        side: Side,
    ) -> UiResult<PaneRef> {
        self.split_pane(target, orientation, side, None)
    }

    /// The empty slot at `path`, if the tree holds one there. **The address a [`PaneRef::Slot`]
    /// carries is resolved against the tree, not trusted** — a path is only as good as the tree
    /// it was stamped from.
    pub fn slot_at(&self, path: &[usize]) -> Option<&Node> {
        node_at(&self.root, path).filter(|node| matches!(node, Node::Slot))
    }

    /// **Fill an empty slot with a window** — the reference's `set_app` (2697–2711), which
    /// replaces a placeholder pane's widget with the app's. Here the slot stops being a slot and
    /// becomes a pane holding that window.
    ///
    /// Two refusals, and both are a state the caller can genuinely be in: the path may name no
    /// slot (the tree moved under the offer grid), and the window may already be somewhere in
    /// this tree — two panes holding one window is the contradiction [`Arrangement::split`]
    /// refuses for the same reason.
    pub fn fill(&mut self, path: &[usize], window: WindowId) -> UiResult<()> {
        if self.windows().contains(&window) {
            return Err(UiRefusal::DuplicateWindow {
                window: window.as_str().to_string(),
            });
        }
        let Some(target) = node_at_mut(&mut self.root, path) else {
            return Err(UiRefusal::UnknownSlot {
                slot: path.to_vec(),
            });
        };
        if !matches!(target, Node::Slot) {
            return Err(UiRefusal::UnknownSlot {
                slot: path.to_vec(),
            });
        }
        *target = Node::Pane(window);
        Ok(())
    }

    /// `undock(pane)` — remove one window pane and return an arrangement holding just it.
    ///
    /// **An empty slot cannot be undocked here**: a slot has no window id, and this model's
    /// registry names windows by id. The reference undocks an empty `WindowPane` into a
    /// standalone window (3040–3085); that is owed, and named in `SHELL-PORT.md` §3.
    ///
    /// **A window becomes free-floating** (H-7.24: *"a window may ignore the grid entirely"*).
    /// The reference re-parents the pane out of its splitter and leaves the window standing; so
    /// the arrangement is one window again and the caller decides where it lives.
    pub fn undock(&self, target: &WindowId) -> Option<Arrangement> {
        self.windows()
            .contains(target)
            .then(|| Arrangement::of(target.clone()))
    }

    /// `close(pane)` — remove one pane, **collapsing one-child splitters back up, recursively**.
    ///
    /// **A pane is addressed by [`PaneRef`]**, so an empty slot closes like any pane. **`None`
    /// when the last pane closes**, per §3: *"a window closes when its last pane does"* — and a
    /// slot is a pane, so a window of one window-pane and one slot survives closing either one
    /// (the reference's `remaining = self.get_all_panes()`, 3105–3110, which empty panes are
    /// counted in).
    ///
    /// **A pane the tree does not hold is the reference's own no-op**: `if pane not in panes:
    /// return` (3087–3088). It leaves the tree alone rather than refusing, because a close is
    /// forgiving — the caller's address came from a frame that may have been one tick old.
    ///
    /// **The collapse is the reference's `_cleanup_splitter`, including its recursion.** A
    /// splitter that drops to one child is replaced by that child; a splitter that drops to
    /// zero cannot exist because its parent already took its place. The recursion upward is
    /// the rule, and stopping at one level leaves a chain of one-child splitters — which is
    /// exactly the accumulation §3 calls the layout's hard-won detail.
    pub fn close(&self, target: &PaneRef) -> Option<Arrangement> {
        let present = match target {
            PaneRef::Window(id) => self.windows().contains(id),
            PaneRef::Slot(path) => self.slot_at(path).is_some(),
        };
        if !present {
            return Some(self.clone());
        }
        // **The window closes when its last pane does**, and a slot is a pane: closing one of
        // two panes always leaves the other, whether it holds a window or not.
        if self.panes() == 1 {
            return None;
        }
        let mut out = Arrangement {
            root: match target {
                PaneRef::Window(id) => remove_pane(&self.root, id),
                PaneRef::Slot(path) => remove_slot(&self.root, path),
            },
        };
        out.root = collapse(out.root);
        debug_assert_eq!(
            out.panes(),
            self.panes() - 1,
            "closing a pane removes exactly that pane"
        );
        Some(out)
    }

    /// Whether every split of `span` can satisfy the 40 × 40 floor.
    ///
    /// **The check the reference gets for free and a data model does not.** Qt enforces
    /// `minimumSizeHint` when it lays out; a tree of stretch factors has no layout pass, so
    /// **the arithmetic has to be done somewhere and this is it.** A pane tree whose root is
    /// 100 px wide and split three ways cannot give every child 40 — and the answer must be
    /// available before anything is drawn, not after a window locks.
    pub fn check_floor(&self, span: u32) -> UiResult<()> {
        check_floor_node(&self.root, span)
    }

    /// Where the reference's `[1000 // num] * num` would lose a unit.
    ///
    /// **Named, not reproduced** (see the module docs). The reference divides 1000 among `n`
    /// children with integer division and never distributes the remainder, so `n` children of
    /// 3 leave one unit unallocated. Equal `f32` factors do not have that problem — and this
    /// function reports the case where the reference's rule would have, so the difference is
    /// visible rather than a silent improvement nobody can find.
    pub fn check_divisible(&self) -> Vec<usize> {
        let mut out = Vec::new();
        collect_uneven(&self.root, &mut out);
        out
    }

    /// Every splitter's child shares, in tree order, outermost first — **the tree's own
    /// stretch factors, read rather than guessed**.
    ///
    /// A seam addresses a splitter and a boundary; this is how a caller learns which splitter
    /// that is without walking the tree itself. An empty list means a bare pane: a tree with no
    /// splitter has no shares to move.
    pub fn shares(&self) -> Vec<Vec<f32>> {
        let mut out = Vec::new();
        collect_shares(&self.root, &mut out);
        out
    }

    /// **Move one boundary of one splitter, from the shares the drag started at.** The seam
    /// drag's mutation, and the only writer of stretch factors after `split`.
    ///
    /// `splitter` addresses a splitter by its index in [`Arrangement::shares`]; `boundary`
    /// addresses the seam **between child `boundary` and child `boundary + 1`**; `span` is the
    /// splitter's extent along its own axis, in dp; `from` is that child pair's shares when the
    /// drag started; and `delta_dp` is the pointer's total travel along the axis.
    ///
    /// **The start pair is read every move, so a move never compounds** — the same rule
    /// `PortalWindow::resize_from` states for a resize, and the reason a seam dragged to its
    /// floor and back comes back with the pointer rather than lagging by the clamp.
    ///
    /// **The clamp is the floor the layout enforces, done in shares.** The reference relies on
    /// Qt's `setChildrenCollapsible(False)` plus the 40 × 40 `minimumSizeHint` to stop a handle
    /// crushing a child; there is no layout engine here to enforce it mid-drag, so the two
    /// children the boundary separates are each held to [`MIN_EXTENT`] — the seam stops rather
    /// than moving a boundary the floor forbids. Qt's handle stops the same way, which is why
    /// this is a clamp and not a refusal.
    ///
    /// **Both shares change by the same amount**: the splitter's total is preserved, so moving
    /// one boundary never resizes the splitter's siblings or its parent (the same property
    /// `a_wrap_preserves_the_outer_splits_shares` pins for `split`).
    ///
    /// # The two refusals
    ///
    /// * **The indices are checked, not trusted.** `splitter` and `boundary` arrive from a
    ///   layout that was drawn before the drag began, and a tree that moved under the drag gets
    ///   [`UiRefusal::UnknownSeam`] rather than a write to whichever splitter now sits at that
    ///   index.
    /// * **A pair already below the floor cannot be dragged at all.** A window resized narrower
    ///   after a drag can leave two adjacent children whose combined extent is under
    ///   `2 × MIN_EXTENT` — the layout's own floor check is the average, `span / count`, so the
    ///   tree can be drawn and still hold such a pair — and no `delta` satisfies both floors.
    ///   That refuses with [`UiRefusal::PaneBelowFloor`] and the pair's numbers, which is the
    ///   layout's own arithmetic rather than a panic inside a clamp.
    pub fn drag_seam(
        &mut self,
        splitter: usize,
        boundary: usize,
        span: f64,
        from: (f32, f32),
        delta_dp: f64,
    ) -> UiResult<()> {
        if !span.is_finite() {
            return Err(UiRefusal::NonFiniteValue {
                what: "a splitter's span",
                value: span,
            });
        }
        if !delta_dp.is_finite() {
            return Err(UiRefusal::NonFiniteValue {
                what: "a seam drag",
                value: delta_dp,
            });
        }
        if !from.0.is_finite() || !from.1.is_finite() {
            return Err(UiRefusal::NonFiniteValue {
                what: "a seam's start share",
                value: from.0 as f64,
            });
        }
        // **The splitter count is read before the mutable walk**, so the refusal can name how
        // many there were without a second borrow of the tree the walk is holding.
        let splitters = self.shares().len();
        let (orientation, children) =
            nth_split_mut(&mut self.root, splitter).ok_or(UiRefusal::UnknownSeam {
                seam: splitter,
                seams: splitters,
            })?;
        if boundary + 1 >= children.len() {
            return Err(UiRefusal::UnknownSeam {
                seam: boundary,
                seams: children.len().saturating_sub(1),
            });
        }
        children[boundary].0 = from.0;
        children[boundary + 1].0 = from.1;
        let count: f64 = children.iter().map(|(share, _)| *share as f64).sum();
        let total = if count <= 0.0 { 1.0 } else { count };
        // The two children the boundary separates, as extents in dp — the numbers the floor
        // check and any refusal are stated in.
        let left_dp = span * from.0 as f64 / total;
        let right_dp = span * from.1 as f64 / total;
        let low = MIN_EXTENT as f64 - left_dp;
        let high = right_dp - MIN_EXTENT as f64;
        if low > high {
            return Err(UiRefusal::PaneBelowFloor {
                span: (left_dp + right_dp).round() as u32,
                count: 2,
                floor: MIN_EXTENT,
                axis: orientation.constrained_axis(),
            });
        }
        let moved = delta_dp.clamp(low, high);
        let moved_share = (moved / span * total) as f32;
        children[boundary].0 += moved_share;
        children[boundary + 1].0 -= moved_share;
        Ok(())
    }
}

/// The `index`th splitter in tree order, as its orientation and children — the seam index's
/// resolution. **The same walk [`Arrangement::shares`] makes**, so the two cannot disagree
/// about which splitter a seam names.
fn nth_split_mut(node: &mut Node, index: usize) -> Option<(Orientation, &mut Vec<(f32, Node)>)> {
    // A preorder counter: every splitter the walk passes is one position, whether it is the
    // node itself or one of its descendants. Recursing on a subtree only after the node's own
    // position has been spent is what keeps the numbering identical to `collect_shares`'.
    let mut remaining = index;
    nth_split_from(node, &mut remaining)
}

fn nth_split_from<'a>(
    node: &'a mut Node,
    remaining: &mut usize,
) -> Option<(Orientation, &'a mut Vec<(f32, Node)>)> {
    match node {
        Node::Pane(_) | Node::Slot => None,
        Node::Split {
            orientation,
            children,
        } => {
            if *remaining == 0 {
                return Some((*orientation, children));
            }
            *remaining -= 1;
            for (_, child) in children.iter_mut() {
                if let Some(found) = nth_split_from(child, remaining) {
                    return Some(found);
                }
            }
            None
        }
    }
}

fn collect_shares(node: &Node, out: &mut Vec<Vec<f32>>) {
    if let Node::Split { children, .. } = node {
        out.push(children.iter().map(|(share, _)| *share).collect());
        for (_, child) in children {
            collect_shares(child, out);
        }
    }
}

fn collect_panes(node: &Node, out: &mut BTreeSet<WindowId>) {
    match node {
        Node::Pane(id) => {
            out.insert(id.clone());
        }
        Node::Slot => {}
        Node::Split { children, .. } => {
            for (_, child) in children {
                collect_panes(child, out);
            }
        }
    }
}

fn count_panes(node: &Node) -> usize {
    match node {
        Node::Pane(_) | Node::Slot => 1,
        Node::Split { children, .. } => children.iter().map(|(_, child)| count_panes(child)).sum(),
    }
}

/// Overwrite every factor in a child list with an equal share.
///
/// **A list's *factors* are replaced, not its *children*.** The reference re-divides the
/// splitter it inserted into and leaves every other splitter alone, so this takes and returns
/// the same children in the same order and touches nothing but the numbers.
fn with_equal_shares(mut children: Vec<(f32, Node)>) -> Vec<(f32, Node)> {
    let share = 1.0 / children.len() as f32;
    for (factor, _) in &mut children {
        *factor = share;
    }
    children
}

/// **What a split is aimed at**: the root (`None`), a window pane, or a slot by its path.
///
/// The three are one question — *which pane does this split happen at* — asked of the two
/// identities a pane can have ([`PaneRef`]). Asking it in one place is what keeps the root
/// case, the insert-beside case and the wrap case from drifting between a pane that holds a
/// window and one that does not.
#[derive(Clone, Copy, Debug)]
enum Aim<'a> {
    /// A pane holding a window, by its id.
    Window(&'a WindowId),
    /// An empty slot, by its path from the root (`[]` is the root).
    Slot(&'a [usize]),
}

impl<'a> Aim<'a> {
    fn of(target: Option<&'a PaneRef>) -> Option<Aim<'a>> {
        match target {
            None => None,
            Some(PaneRef::Window(id)) => Some(Aim::Window(id)),
            Some(PaneRef::Slot(path)) => Some(Aim::Slot(path)),
        }
    }
}

/// The tree with one pane split, or `None` when the aim names a pane the tree does not hold.
///
/// `new_window` is the pane to create: `Some(id)` is a pane holding that window (the dock
/// path), `None` is an **empty slot** — the reference's `WindowPane(app=None)`, which both the
/// pane header's split controls and the outer edge triggers create (2919–2921). The returned
/// [`PaneRef`] is the new pane's address in the tree that was built: for a slot, the path the
/// layout will stamp, so the caller can address it without a second walk.
fn split_at(
    node: &Node,
    aim: Option<&Aim<'_>>,
    orientation: Orientation,
    side: Side,
    new_window: Option<&WindowId>,
    path: &mut Vec<usize>,
) -> Option<(Node, PaneRef)> {
    // Case 1 — no aim at all, or the aim *is* the root pane.
    //
    // **The first version of this also treated a `Split` root with any `Some` target as a root
    // split, and the wrap test caught it.** That is the reference's
    // `target_pane == self.layout_root` branch — but the reference compares a *widget* to its
    // root, and this type's target is a *window id*, which is either inside the tree or it is
    // not. Treating "the root happens to be a splitter" as "the target is the root" sent every
    // nested split to the root case, so `split(gamma, Vertical, …)` split the whole window
    // instead of nesting — the outer shares went from `[1.0, 1.0]` to `[⅓, ⅓, ⅓]`.
    //
    // A `Split` root therefore always means **search inside**, and a target that is not found is
    // a refusal at the call site rather than a silent fallback to the root.
    let aim_is_root = match (aim, node) {
        (None, _) => true,
        (Some(Aim::Window(id)), Node::Pane(root)) => *id == root,
        // **An empty path addresses the root**, so a lone slot is the reference's
        // `target_pane == self.layout_root` with an unnamed pane.
        (Some(Aim::Slot(slot)), Node::Slot) => slot.is_empty(),
        (Some(_), _) => false,
    };
    if aim_is_root {
        return Some(split_root(node, orientation, side, new_window, path));
    }

    // Case 4 — the aim is a pane inside a splitter.
    //
    // **`aim` is `Some` here and the compiler does not know it.** The root case above returns,
    // so the only way to reach this arm is a non-root aim — but `aim` is still typed
    // `Option<&Aim>`, so it is unwrapped *by name* rather than by a control-flow fact the
    // reader has to reconstruct. An `.expect` on a case that cannot fail is better than a
    // silent `None` that would drop the split.
    match aim.expect("the root case returns above, so the aim is Some here") {
        Aim::Window(id) => split_beside(node, id, orientation, side, new_window, path),
        Aim::Slot(slot) => split_beside_slot(node, slot, orientation, side, new_window, path),
    }
}

/// The root cases: **insert** into a same-orientation root splitter, or **wrap** the root.
///
/// The reference's `split_pane` case 1 (2930–2976), and the three shapes it covers — a bare
/// pane, a root splitter that agrees and one that opposes — are the `match` below. `path` is
/// the path to the node being wrapped or inserted into (empty at the root), and it is what a
/// new slot's own path is stamped from.
fn split_root(
    node: &Node,
    orientation: Orientation,
    side: Side,
    new_window: Option<&WindowId>,
    path: &[usize],
) -> (Node, PaneRef) {
    let new_node = new_node(new_window);
    match node {
        Node::Split {
            orientation: root,
            children,
        } if *root == orientation => {
            // Case 2 — same orientation: **insert**, and re-divide the whole root.
            let mut next = children.clone();
            let at = match side {
                Side::Before => 0,
                Side::After => next.len(),
            };
            next.insert(at, (1.0, new_node));
            (
                Node::Split {
                    orientation,
                    children: with_equal_shares(next),
                },
                new_ref(path, at, new_window),
            )
        }
        other => {
            // Case 3 — opposing orientation, or a bare pane: **wrap the root**.
            let (children, at) = match (other, side) {
                (Node::Pane(existing), Side::Before) => (
                    vec![(1.0, new_node), (1.0, Node::Pane(existing.clone()))],
                    0,
                ),
                (Node::Pane(existing), Side::After) => (
                    vec![(1.0, Node::Pane(existing.clone())), (1.0, new_node)],
                    1,
                ),
                // **A slot is wrapped like a pane**: it is the old pane, and it keeps its side.
                (Node::Slot, Side::Before) => (vec![(1.0, new_node), (1.0, Node::Slot)], 0),
                (Node::Slot, Side::After) => (vec![(1.0, Node::Slot), (1.0, new_node)], 1),
                // **A wrapped root splitter is re-nested, not flattened.** The reference builds
                // `new_root` around `old_root` wholesale; flattening would silently reorder
                // every pane in the old tree.
                (Node::Split { .. }, Side::Before) => {
                    let mut next = vec![(1.0, new_node)];
                    next.push((1.0, other.clone()));
                    (next, 0)
                }
                (Node::Split { .. }, Side::After) => {
                    (vec![(1.0, other.clone()), (1.0, new_node)], 1)
                }
            };
            (
                Node::Split {
                    orientation,
                    children,
                },
                new_ref(path, at, new_window),
            )
        }
    }
}

/// **The reference's case 2, and it is about the target's *parent*.** `portal_simulator.py`
/// 2919–3012 walks up from the target to the nearest `QSplitter`, then:
///
/// * **the parent agrees on orientation** → *insert beside the target* (`insertWidget(idx ± 1)`)
///   and re-divide **that** splitter; ancestors keep their shares (`setSizes(old_sizes)`);
/// * **the parent disagrees** → *wrap the target alone* in a fresh sub-splitter, put the
///   sub-splitter back at the target's index, and restore the parent's shares.
///
/// **The first version of this wrapped the child subtree instead of the target, and inserted
/// only when the target was itself a same-orientation splitter.** Both diverge from the
/// reference: `H[a, b]` split at `a` horizontally must become `H[a, new, b]`, not
/// `H[H[a, new], b]`, and `H[V[a, b], c]` split at `a` horizontally must become
/// `H[V[H[a, new], b], c]` — the pane is lifted out of `V`, not `V` wrapped whole. The first
/// divergence was found by `splitter.rs`'s three-across floor test, which expected three panes
/// in a row and got two plus a nesting.
///
/// `path` is the path to the node this call is looking at, and it is carried only so a new
/// slot's own path can be stamped: the walk knows the index of every step it takes.
fn split_beside(
    node: &Node,
    target: &WindowId,
    orientation: Orientation,
    side: Side,
    new_window: Option<&WindowId>,
    path: &mut Vec<usize>,
) -> Option<(Node, PaneRef)> {
    let Node::Split {
        orientation: parent,
        children,
    } = node
    else {
        // A bare pane or a slot cannot hold a window: there is no parent to split beside.
        return None;
    };
    // Is the target a **direct** child of this splitter? Then this is its parent.
    if let Some(index) = children
        .iter()
        .position(|(_, child)| matches!(child, Node::Pane(id) if id == target))
    {
        return Some(split_in_children(
            children,
            index,
            *parent,
            orientation,
            side,
            new_window,
            path,
        ));
    }
    // Otherwise descend into the one child that holds the target. The parent that decides is
    // found at the level where the target is a direct child, so the recursion carries no state
    // — the splitter that answers is the one that sees it.
    for (index, (share, child)) in children.iter().enumerate() {
        if !holds(child, target) {
            continue;
        }
        path.push(index);
        let found = split_beside(child, target, orientation, side, new_window, path);
        path.pop();
        let (next, created) = found?;
        let mut rebuilt = children.clone();
        rebuilt[index] = (*share, next);
        return Some((
            Node::Split {
                orientation: *parent,
                children: rebuilt,
            },
            created,
        ));
    }
    None
}

/// **The same case for a slot**, which is addressed by its path rather than searched for: the
/// path's last index says which child it is, and the splitter that holds it is the one that
/// decides — exactly the parent rule above, with the position given instead of found. The walk
/// is the same walk [`node_at`] makes, so a slot's [`PaneRef`] and this split resolve the same
/// node.
fn split_beside_slot(
    node: &Node,
    slot: &[usize],
    orientation: Orientation,
    side: Side,
    new_window: Option<&WindowId>,
    path: &mut Vec<usize>,
) -> Option<(Node, PaneRef)> {
    let (first, rest) = slot.split_first()?;
    let Node::Split {
        orientation: parent,
        children,
    } = node
    else {
        return None;
    };
    let (share, child) = children.get(*first)?;
    if rest.is_empty() {
        // The slot is a direct child of this splitter: the splitter decides, as above.
        if !matches!(child, Node::Slot) {
            return None;
        }
        return Some(split_in_children(
            children,
            *first,
            *parent,
            orientation,
            side,
            new_window,
            path,
        ));
    }
    path.push(*first);
    let found = split_beside_slot(child, rest, orientation, side, new_window, path);
    path.pop();
    let (next, created) = found?;
    let mut rebuilt = children.clone();
    rebuilt[*first] = (*share, next);
    Some((
        Node::Split {
            orientation: *parent,
            children: rebuilt,
        },
        created,
    ))
}

/// **The pane the new split creates**, from the caller's request: `Some` a window id, `None` an
/// empty slot — the reference's own two possibilities (`WindowPane(app=…)` and
/// `WindowPane(app=None)`, 2853–2857).
fn new_node(new_window: Option<&WindowId>) -> Node {
    match new_window {
        Some(id) => Node::Pane(id.clone()),
        None => Node::Slot,
    }
}

/// The address of the pane the split has just created: the window's id when it holds one, and
/// otherwise **the path the node was inserted at** — `path` is the path to the splitter the
/// insert happened in, and `at` is the new child's index in it.
fn new_ref(path: &[usize], at: usize, new_window: Option<&WindowId>) -> PaneRef {
    match new_window {
        Some(id) => PaneRef::Window(id.clone()),
        None => {
            let mut slot = path.to_vec();
            slot.push(at);
            PaneRef::Slot(slot)
        }
    }
}

/// **The insert-beside-or-wrap decision at the target's own parent**, shared by a window target
/// and a slot target: `parent` is the splitter's orientation, `index` the target's index in it,
/// and `path` the path to this splitter (so a new slot's own path can be stamped).
fn split_in_children(
    children: &[(f32, Node)],
    index: usize,
    parent: Orientation,
    orientation: Orientation,
    side: Side,
    new_window: Option<&WindowId>,
    path: &[usize],
) -> (Node, PaneRef) {
    let new_node = new_node(new_window);
    if parent == orientation {
        // Insert beside the target, and re-divide this splitter only.
        let mut next = children.to_vec();
        let at = match side {
            Side::Before => index,
            Side::After => index + 1,
        };
        next.insert(at, (1.0, new_node));
        (
            Node::Split {
                orientation: parent,
                children: with_equal_shares(next),
            },
            new_ref(path, at, new_window),
        )
    } else {
        // Wrap the target alone, in place, and keep this splitter's shares untouched.
        let mut rebuilt = children.to_vec();
        let at = match side {
            Side::Before => 0,
            Side::After => 1,
        };
        rebuilt[index] = (
            children[index].0,
            Node::Split {
                orientation,
                children: match side {
                    Side::Before => vec![(1.0, new_node), (1.0, children[index].1.clone())],
                    Side::After => vec![(1.0, children[index].1.clone()), (1.0, new_node)],
                },
            },
        );
        // The new pane's path inside the fresh sub-splitter: this splitter's path, the
        // target's index, then the new pane's own place.
        let mut slot = path.to_vec();
        slot.push(index);
        slot.push(at);
        (
            Node::Split {
                orientation: parent,
                children: rebuilt,
            },
            match new_window {
                Some(id) => PaneRef::Window(id.clone()),
                None => PaneRef::Slot(slot),
            },
        )
    }
}

fn holds(node: &Node, target: &WindowId) -> bool {
    match node {
        Node::Pane(id) => id == target,
        // **A slot holds no window**, so no id can be in it.
        Node::Slot => false,
        Node::Split { children, .. } => children.iter().any(|(_, child)| holds(child, target)),
    }
}

fn remove_pane(node: &Node, target: &WindowId) -> Node {
    match node {
        Node::Pane(id) if id == target => Node::Pane(WindowId(String::new())),
        Node::Pane(id) => Node::Pane(id.clone()),
        // **A slot is not the target**: a window's id is never a slot's, so the walk leaves it.
        Node::Slot => Node::Slot,
        Node::Split {
            orientation,
            children,
        } => Node::Split {
            orientation: *orientation,
            children: children
                .iter()
                .map(|(share, child)| (*share, remove_pane(child, target)))
                .collect(),
        },
    }
}

/// Remove the empty slot at `path` — **the node becomes a hole** the collapse drops, exactly as
/// [`remove_pane`] leaves one. The walk is the same walk [`node_at`] makes, so a slot's address
/// means to the removal what it means to the reader.
fn remove_slot(node: &Node, path: &[usize]) -> Node {
    match path.split_first() {
        None => Node::Pane(WindowId(String::new())),
        Some((first, rest)) => match node {
            Node::Split {
                orientation,
                children,
            } => Node::Split {
                orientation: *orientation,
                children: children
                    .iter()
                    .enumerate()
                    .map(|(index, (share, child))| {
                        (
                            *share,
                            if index == *first {
                                remove_slot(child, rest)
                            } else {
                                child.clone()
                            },
                        )
                    })
                    .collect(),
            },
            other => other.clone(),
        },
    }
}

/// The node at a path from the root — **the same walk the layout stamps**, so a path means to
/// the tree what it means to the drawing. `[]` is the root.
fn node_at<'a>(node: &'a Node, path: &[usize]) -> Option<&'a Node> {
    match path.split_first() {
        None => Some(node),
        Some((first, rest)) => match node {
            Node::Split { children, .. } => children
                .get(*first)
                .and_then(|(_, child)| node_at(child, rest)),
            _ => None,
        },
    }
}

fn node_at_mut<'a>(node: &'a mut Node, path: &[usize]) -> Option<&'a mut Node> {
    match path.split_first() {
        None => Some(node),
        Some((first, rest)) => match node {
            Node::Split { children, .. } => children
                .get_mut(*first)
                .and_then(|(_, child)| node_at_mut(child, rest)),
            _ => None,
        },
    }
}

/// **The recursive collapse, and it is the whole of `close`.**
///
/// A splitter with one child *is* that child. Doing it repeatedly, bottom-up, is what
/// "collapses one-child splitters **back up**" means — the reference's `_cleanup_splitter`
/// recurses into the parent, and a single-level version leaves the chain behind.
fn collapse(node: Node) -> Node {
    match node {
        Node::Split {
            orientation,
            children,
        } => {
            let children: Vec<(f32, Node)> = children
                .into_iter()
                .map(|(share, child)| (share, collapse(child)))
                .collect();
            // **A pane emptied by `close` is dropped, not kept as a hole.** The reference
            // reparents and deletes it; **an empty-id pane is this walk's own internal hole**,
            // never a state a caller can construct ([`WindowId::new`] refuses a blank id), and
            // a `Node::Slot` — the visible empty pane a split creates — is a pane like any
            // other and survives this filter.
            let children: Vec<(f32, Node)> = children
                .into_iter()
                .filter(|(_, child)| !is_hole(child))
                .collect();
            match children.len() {
                0 => Node::Pane(WindowId(String::new())),
                1 => children.into_iter().next().expect("one child").1,
                _ => Node::Split {
                    orientation,
                    children,
                },
            }
        }
        pane => pane,
    }
}

/// A pane whose id is empty is a **hole** — the placeholder `remove_pane` left behind.
fn is_hole(node: &Node) -> bool {
    matches!(node, Node::Pane(id) if id.as_str().is_empty())
}

fn check_floor_node(node: &Node, span: u32) -> UiResult<()> {
    let children = match node {
        Node::Pane(_) | Node::Slot => return Ok(()),
        Node::Split {
            orientation,
            children,
        } => (orientation, children),
    };
    let (orientation, children) = children;
    let count = children.len() as u32;
    if span / count < MIN_EXTENT {
        return Err(UiRefusal::PaneBelowFloor {
            span,
            count,
            floor: MIN_EXTENT,
            axis: orientation.constrained_axis(),
        });
    }
    for (_, child) in children {
        match child {
            // **The nested span is what makes the floor accumulate.** The reference's warning
            // is that nested `minimumSizeHint`s add up; here each level divides what the
            // level above allowed, so a three-deep tree needs 40 × 2³ = 320 px of root span.
            Node::Split { .. } => check_floor_node(child, span / count)?,
            Node::Pane(_) | Node::Slot => {}
        }
    }
    Ok(())
}

fn collect_uneven(node: &Node, out: &mut Vec<usize>) {
    if let Node::Split { children, .. } = node {
        let count = children.len();
        if 1000 % count != 0 {
            out.push(count);
        }
        for (_, child) in children {
            collect_uneven(child, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> WindowId {
        WindowId::new(name).expect("a window id")
    }

    /// **The spec's own sentence, as a test.** §3: *"A window with no split is one pane."*
    #[test]
    fn a_window_with_no_split_is_one_pane() {
        let arrangement = Arrangement::of(id("alpha"));
        assert_eq!(arrangement.panes(), 1);
        assert!(arrangement.windows().contains(&id("alpha")));
        assert_eq!(arrangement.check_floor(400), Ok(()));
    }

    /// **§3: "wraps the root when the target is the root."**
    #[test]
    fn splitting_a_bare_root_wraps_it() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        assert_eq!(arrangement.panes(), 2);
        match arrangement.root() {
            Node::Split {
                orientation,
                children,
            } => {
                assert_eq!(*orientation, Orientation::Horizontal);
                assert_eq!(children.len(), 2);
                assert!(
                    holds(&children[0].1, &id("alpha")),
                    "the old pane keeps its side"
                );
                assert!(
                    holds(&children[1].1, &id("beta")),
                    "the new pane arrives after"
                );
            }
            other => panic!("the root did not become a splitter: {other:?}"),
        }
    }

    /// **§3: "inserts into an existing splitter when the orientation agrees."** The reference
    /// re-divides the whole splitter on insert, and the two children get equal shares.
    #[test]
    fn a_matching_orientation_inserts_and_re_divides() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("gamma"))
            .expect("splits");
        assert_eq!(arrangement.panes(), 3);
        match arrangement.root() {
            Node::Split { children, .. } => {
                assert_eq!(children.len(), 3, "inserted, not wrapped");
                for (share, _) in children {
                    assert!(
                        (share - 1.0 / 3.0).abs() < 1e-6,
                        "re-divided equally, got {share}"
                    );
                }
            }
            other => panic!("the root stopped being a splitter: {other:?}"),
        }
    }

    /// **The detail a re-derivation drops: the outer split keeps its shares.**
    ///
    /// The reference captures `old_sizes` before re-parenting and restores them after
    /// (`parent.setSizes(old_sizes)`), so nesting a split inside an existing one does not
    /// resize the window around it. **If this test fails, the wrap is re-dividing its
    /// ancestors** — which is what the reference's own comment is guarding against.
    #[test]
    fn a_wrap_preserves_the_outer_splits_shares() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("gamma"))
            .expect("splits");
        let outer_before: Vec<f32> = match arrangement.root() {
            Node::Split { children, .. } => children.iter().map(|(s, _)| *s).collect(),
            other => panic!("not a splitter: {other:?}"),
        };

        // **Split `gamma` vertically** — opposing the horizontal root, so this is a wrap.
        arrangement
            .split(
                Some(&id("gamma")),
                Orientation::Vertical,
                Side::After,
                id("delta"),
            )
            .expect("splits");

        let outer_after: Vec<f32> = match arrangement.root() {
            Node::Split { children, .. } => children.iter().map(|(s, _)| *s).collect(),
            other => panic!("not a splitter: {other:?}"),
        };
        assert_eq!(
            outer_before, outer_after,
            "**the outer split's shares must survive a nested wrap** — the reference restores \
             `old_sizes` for exactly this, and a layout that re-divides on every nest makes a \
             deep tree's panes drift toward nothing"
        );
        assert_eq!(arrangement.panes(), 4);
    }

    /// **§3: "`close(pane)` collapses one-child splitters back up."** Three levels deep, so a
    /// one-level collapse would leave a chain behind.
    #[test]
    fn closing_collapses_one_child_splitters_all_the_way_up() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        arrangement
            .split(
                Some(&id("beta")),
                Orientation::Vertical,
                Side::After,
                id("gamma"),
            )
            .expect("splits");

        let after = arrangement
            .close(&PaneRef::Window(id("gamma")))
            .expect("two panes remain");
        assert_eq!(
            after.panes(),
            2,
            "alpha and beta both survive closing gamma"
        );
        assert!(
            !contains_split(after.root()),
            "**no one-child splitter survives a close** — the collapse is recursive, and this \
             is the case the reference's `_cleanup_splitter(p)` recursion exists for"
        );
    }

    /// **§3: "a window closes when its last pane does."**
    #[test]
    fn closing_the_last_pane_has_no_arrangement_left() {
        let arrangement = Arrangement::of(id("alpha"));
        assert_eq!(
            arrangement.close(&PaneRef::Window(id("alpha"))),
            None,
            "**an arrangement with no windows describes nothing**, so the type has no way to \
             spell that state — the same rule as there being no `Arrangement::new`"
        );
    }

    /// **§3: "40 × 40 floor on every splitter and pane … nested splitters' `minimumSizeHint`s
    /// accumulate and the layout locks."** The accumulation is the point: 40 × 2³ = 320.
    #[test]
    fn the_floor_accumulates_through_nesting() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        arrangement
            .split(
                Some(&id("beta")),
                Orientation::Vertical,
                Side::After,
                id("gamma"),
            )
            .expect("splits");
        arrangement
            .split(
                Some(&id("gamma")),
                Orientation::Horizontal,
                Side::After,
                id("delta"),
            )
            .expect("splits");

        assert!(
            arrangement.check_floor(1000).is_ok(),
            "1000 px clears three levels"
        );
        assert!(
            arrangement.check_floor(200).is_err(),
            "**200 px cannot give four panes 40 each**, and the refusal names the arithmetic \
             rather than letting a layout pass discover it after the window locks"
        );
    }

    /// **The reference's insert case, at the pane that is actually targeted:** the parent's
    /// orientation agrees, so the new pane lands **beside the target** — `H[a, b]` split at
    /// `a` becomes `H[a, new, b]`, not `H[H[a, new], b]`.
    #[test]
    fn a_same_orientation_parent_inserts_beside_the_target() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        arrangement
            .split(
                Some(&id("alpha")),
                Orientation::Horizontal,
                Side::After,
                id("gamma"),
            )
            .expect("splits");
        match arrangement.root() {
            Node::Split { children, .. } => {
                assert_eq!(
                    children.len(),
                    3,
                    "three panes in one row, got {children:?}"
                );
                assert!(holds(&children[0].1, &id("alpha")));
                assert!(
                    holds(&children[1].1, &id("gamma")),
                    "the new pane sits beside its target"
                );
                assert!(holds(&children[2].1, &id("beta")));
            }
            other => panic!("the root stopped being a splitter: {other:?}"),
        }
    }

    /// **The reference's wrap case, at a target that is deeper than one level:** the pane is
    /// lifted **alone** into a fresh sub-splitter inside its own parent — `H[V[a, b], c]` split
    /// at `a` horizontally becomes `H[V[H[a, new], b], c]`, not `H[H[V[a, b], new], c]`.
    #[test]
    fn a_deeper_target_is_lifted_out_of_its_own_parent_not_the_whole_subtree() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("charlie"))
            .expect("splits");
        arrangement
            .split(
                Some(&id("alpha")),
                Orientation::Vertical,
                Side::After,
                id("beta"),
            )
            .expect("wraps alpha");
        // Now H[V[alpha, beta], charlie]; split alpha horizontally, which its parent (V) opposes.
        arrangement
            .split(
                Some(&id("alpha")),
                Orientation::Horizontal,
                Side::After,
                id("delta"),
            )
            .expect("splits");
        assert_eq!(arrangement.panes(), 4);
        match arrangement.root() {
            Node::Split {
                orientation,
                children,
            } if *orientation == Orientation::Horizontal => {
                assert_eq!(children.len(), 2, "the outer split still has two children");
                match &children[0].1 {
                    Node::Split {
                        orientation,
                        children,
                    } if *orientation == Orientation::Vertical => {
                        assert_eq!(
                            children.len(),
                            2,
                            "the vertical slot still holds two children"
                        );
                        match &children[0].1 {
                            Node::Split {
                                orientation,
                                children,
                            } if *orientation == Orientation::Horizontal => {
                                assert_eq!(children.len(), 2);
                                assert!(holds(&children[0].1, &id("alpha")));
                                assert!(holds(&children[1].1, &id("delta")));
                            }
                            other => panic!("alpha and delta were not wrapped together: {other:?}"),
                        }
                        assert!(
                            holds(&children[1].1, &id("beta")),
                            "beta stays in the vertical slot"
                        );
                    }
                    other => panic!("the vertical sub-splitter disappeared: {other:?}"),
                }
                assert!(holds(&children[1].1, &id("charlie")));
            }
            other => panic!("the root stopped being a horizontal splitter: {other:?}"),
        }
    }

    /// **The side is the side of the target, in the parent's own order:** `Before` inserts at
    /// the target's index, so `H[a, b]` split at `b` before becomes `H[a, new, b]`.
    #[test]
    fn an_insert_before_lands_at_the_targets_own_index() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        arrangement
            .split(
                Some(&id("beta")),
                Orientation::Horizontal,
                Side::Before,
                id("gamma"),
            )
            .expect("splits");
        match arrangement.root() {
            Node::Split { children, .. } => {
                assert_eq!(children.len(), 3);
                assert!(holds(&children[1].1, &id("gamma")));
                assert!(holds(&children[2].1, &id("beta")));
            }
            other => panic!("the root stopped being a splitter: {other:?}"),
        }
    }

    #[test]
    fn a_window_cannot_be_in_two_panes() {
        // **The case the reference does not check, because the reference can draw a
        // contradiction and a data model cannot.** Two panes holding one id would make
        // `close` guess which one to remove.
        let mut arrangement = Arrangement::of(id("alpha"));
        let refused = arrangement
            .split(None, Orientation::Horizontal, Side::After, id("alpha"))
            .expect_err("a window cannot be in two panes");
        assert!(
            refused.to_string().contains("alpha"),
            "the refusal names the window: {refused}"
        );
    }

    /// The proportions a splitter's shares describe, for assertions: shares are proportional
    /// (a fresh wrap carries `1.0 / 1.0`), so what a reader checks is the division, not the
    /// factors.
    fn proportions(shares: &[f32]) -> Vec<f32> {
        let total: f32 = shares.iter().sum();
        shares.iter().map(|share| share / total).collect()
    }

    /// **The seam drag's own arithmetic, from the start pair.** A boundary moved by a quarter
    /// of the splitter's span in a two-way split is a 75 / 25 splitter — and every move reads
    /// the start pair, so moving the pointer back undoes it exactly.
    #[test]
    fn a_seam_drag_moves_a_boundary_and_keeps_the_splitters_total() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        // A fresh wrap carries 1.0 / 1.0; the division is what matters, not the factors.
        assert_eq!(arrangement.shares(), vec![vec![1.0, 1.0]]);
        let start = (1.0f32, 1.0f32);
        arrangement
            .drag_seam(0, 0, 400.0, start, 100.0)
            .expect("a quarter of the span is well inside the floor");
        let shares = arrangement.shares();
        assert_eq!(shares.len(), 1);
        let p = proportions(&shares[0]);
        assert!((p[0] - 0.75).abs() < 1e-6, "got {:?}", shares[0]);
        assert!((p[1] - 0.25).abs() < 1e-6, "got {:?}", shares[0]);
        // **The total is preserved**, which is what keeps the outer tree's shares untouched.
        let total: f32 = shares[0].iter().sum();
        assert!(
            (total - 2.0).abs() < 1e-6,
            "the splitter's total moved: {total}"
        );
        // The same drag applied from the start pair, back at the origin, restores the halves
        // exactly — the property a compounding drag would lose.
        arrangement
            .drag_seam(0, 0, 400.0, start, 0.0)
            .expect("back to the start");
        assert_eq!(arrangement.shares(), vec![vec![1.0, 1.0]]);
    }

    /// **The 40 dp floor is the clamp, and a pair already under it refuses.** The reference
    /// stops the handle at `minimumSizeHint`; this is the same stop, and the refusal names the
    /// arithmetic when the pair the drag separates cannot satisfy it at all.
    #[test]
    fn a_seam_drag_stops_at_the_floor_and_refuses_a_pair_already_under_it() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        // 100 dp split in half: 50 each. A drag of 80 dp left would leave 10 on the left, under
        // the 40 dp floor — so it stops at 40, i.e. 0.4 of the span on the left.
        arrangement
            .drag_seam(0, 0, 100.0, (1.0, 1.0), -80.0)
            .expect("clamped, not refused");
        let p = proportions(&arrangement.shares()[0]);
        assert!(
            (p[0] - 0.4).abs() < 1e-6,
            "the seam stopped at the floor: {p:?}"
        );
        assert!((p[1] - 0.6).abs() < 1e-6, "{p:?}");
        // A pair whose combined extent is under 2 × 40 cannot be dragged at all — the layout
        // can draw it (its floor check is the average) and the drag refuses with the numbers.
        let mut narrow = Arrangement::of(id("alpha"));
        narrow
            .split(None, Orientation::Vertical, Side::After, id("beta"))
            .expect("splits");
        let refused = narrow
            .drag_seam(0, 0, 70.0, (1.0, 1.0), 1.0)
            .expect_err("35 + 35 dp cannot give both children 40");
        assert!(
            matches!(
                refused,
                UiRefusal::PaneBelowFloor {
                    span: 70,
                    count: 2,
                    floor: 40,
                    ..
                }
            ),
            "the refusal carries the pair's own arithmetic: {refused}"
        );
    }

    /// **The seam indices are facts about a layout, and a tree that moved refuses.** A stale
    /// index writes no splitter: the seam drag's whole safety is that the layout and the tree
    /// are checked to agree before a share moves.
    #[test]
    fn a_seam_the_tree_does_not_hold_is_refused_by_name() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        let err = arrangement
            .drag_seam(7, 0, 400.0, (1.0, 1.0), 0.0)
            .expect_err("there is no eighth splitter");
        assert!(
            matches!(err, UiRefusal::UnknownSeam { seam: 7, .. }),
            "{err}"
        );
        let err = arrangement
            .drag_seam(0, 4, 400.0, (1.0, 1.0), 0.0)
            .expect_err("a two-child splitter has one boundary");
        assert!(
            matches!(err, UiRefusal::UnknownSeam { seam: 4, .. }),
            "{err}"
        );
        // And nothing moved on the way to either refusal.
        assert_eq!(arrangement.shares(), vec![vec![1.0, 1.0]]);
    }

    /// **A nested splitter is addressed by the same walk `shares` reads**, so the seam index
    /// cannot mean one splitter to the reader and another to the writer.
    #[test]
    fn the_seam_index_walks_nested_splitters_in_tree_order() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("charlie"))
            .expect("splits");
        arrangement
            .split(
                Some(&id("alpha")),
                Orientation::Vertical,
                Side::After,
                id("beta"),
            )
            .expect("wraps alpha");
        // H[V[alpha, beta], charlie]: splitter 0 is the outer H, splitter 1 the nested V.
        assert_eq!(arrangement.shares().len(), 2);
        let outer = arrangement.shares()[0].clone();
        arrangement
            .drag_seam(1, 0, 200.0, (1.0, 1.0), 50.0)
            .expect("the nested splitter's seam");
        let p = proportions(&arrangement.shares()[1]);
        assert!(
            (p[0] - 0.75).abs() < 1e-6,
            "the nested splitter moved: {p:?}"
        );
        assert_eq!(
            arrangement.shares()[0],
            outer,
            "the outer splitter's shares are untouched"
        );
    }

    /// **A slot split creates the reference's unnamed pane** — `WindowPane(app=None)`, the
    /// pane the pane header's `◧`/`⬓` and the outer edge triggers make (2920–2921). It takes a
    /// share, it counts as a pane, and it holds no window.
    #[test]
    fn a_slot_split_creates_the_reference_s_unnamed_pane() {
        let mut arrangement = Arrangement::of(id("alpha"));
        let slot = arrangement
            .split_slot(None, Orientation::Horizontal, Side::After)
            .expect("splits");
        assert_eq!(
            slot,
            PaneRef::Slot(vec![1]),
            "the new pane is the root's second child"
        );
        assert_eq!(arrangement.panes(), 2, "a slot is a pane");
        assert_eq!(
            arrangement.windows(),
            [id("alpha")].into_iter().collect(),
            "a slot holds no window"
        );
        assert!(arrangement.slot_at(&[1]).is_some());
        assert!(
            arrangement.check_floor(80).is_ok(),
            "two 40 dp panes fit in 80"
        );
    }

    /// **A slot is addressed by its path**, and the path is the tree's: a nested split's slot is
    /// `[1, 0]`, and the same path resolves to the same node for the reader, the writer and the
    /// layout.
    #[test]
    fn a_slot_is_addressed_by_the_path_the_tree_walks() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        // H[alpha, beta] wrapped vertically: V[H[alpha, beta], slot].
        let slot = arrangement
            .split_slot(None, Orientation::Vertical, Side::After)
            .expect("wraps the root");
        assert_eq!(slot, PaneRef::Slot(vec![1]));
        // Split the slot itself, opposing its parent's orientation: the slot is wrapped alone.
        let nested = arrangement
            .split_slot(Some(&slot), Orientation::Horizontal, Side::Before)
            .expect("splits the slot");
        assert_eq!(nested, PaneRef::Slot(vec![1, 0]));
        assert!(arrangement.slot_at(&[1, 0]).is_some());
        // **The split adds a pane**: V[H[alpha, beta], H[slot, new]] is four panes.
        assert_eq!(arrangement.panes(), 4);
        // **A path the tree does not hold is refused by name**, not resolved to a neighbour.
        let refused = arrangement
            .split_slot(
                Some(&PaneRef::Slot(vec![9])),
                Orientation::Horizontal,
                Side::After,
            )
            .expect_err("there is no tenth child");
        assert!(
            matches!(refused, UiRefusal::UnknownSlot { .. }),
            "{refused}"
        );
    }

    /// **Filling a slot is `set_app`** (2697–2711): the slot stops being empty and holds the
    /// window; a second fill of the same slot, and a window already in the tree, both refuse.
    #[test]
    fn filling_a_slot_gives_it_the_window_once() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split_slot(None, Orientation::Horizontal, Side::After)
            .expect("splits");
        arrangement
            .fill(&[1], id("beta"))
            .expect("the slot takes the window");
        assert!(arrangement.windows().contains(&id("beta")));
        assert!(arrangement.slot_at(&[1]).is_none(), "the slot is gone");
        // A different window cannot fill the pane that is no longer a slot either — the path
        // names a window pane, and a split does not replace one.
        let refused = arrangement
            .fill(&[1], id("gamma"))
            .expect_err("the path is not a slot");
        assert!(
            matches!(refused, UiRefusal::UnknownSlot { .. }),
            "{refused}"
        );
        // Filling the same window twice is the duplicate case, which the tree checks first.
        let refused = arrangement
            .fill(&[1], id("beta"))
            .expect_err("beta is already in the tree");
        assert!(
            matches!(refused, UiRefusal::DuplicateWindow { .. }),
            "{refused}"
        );
        // A window the tree already holds cannot be placed anywhere in it.
        let mut other = Arrangement::of(id("alpha"));
        other
            .split_slot(None, Orientation::Horizontal, Side::After)
            .expect("splits");
        let refused = other
            .fill(&[1], id("alpha"))
            .expect_err("alpha is already in the tree");
        assert!(
            matches!(refused, UiRefusal::DuplicateWindow { .. }),
            "{refused}"
        );
    }

    /// **A slot is a pane for `close`**: closing the window pane of a window-and-slot leaves
    /// the slot (the window survives as an empty pane, 3105–3110), and closing the last pane
    /// closes the window. The slot's address moves with the tree, which is why it is a path.
    #[test]
    fn a_slot_is_a_pane_for_close() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split_slot(None, Orientation::Horizontal, Side::After)
            .expect("splits");
        let windowless = arrangement
            .close(&PaneRef::Window(id("alpha")))
            .expect("the slot is still a pane");
        assert_eq!(windowless.panes(), 1);
        assert!(windowless.windows().is_empty());
        // **The lone slot is the root**, so its path is empty — the same tree, re-addressed.
        assert!(windowless.slot_at(&[]).is_some());
        assert_eq!(
            windowless.close(&PaneRef::Slot(vec![])),
            None,
            "the last pane closing closes the window"
        );
        // A pane the tree does not hold is the reference's own no-op, not a refusal.
        assert_eq!(
            arrangement.close(&PaneRef::Window(id("zz"))),
            Some(arrangement.clone())
        );
    }

    #[test]
    fn undock_leaves_the_window_free_floating() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement
            .split(None, Orientation::Horizontal, Side::After, id("beta"))
            .expect("splits");
        let floating = arrangement
            .undock(&id("beta"))
            .expect("beta is in the arrangement");
        assert_eq!(
            floating.panes(),
            1,
            "H-7.24: a window may ignore the grid entirely"
        );
        assert!(floating.windows().contains(&id("beta")));
    }

    fn contains_split(node: &Node) -> bool {
        match node {
            Node::Pane(_) | Node::Slot => false,
            Node::Split { children, .. } => children.iter().any(|(_, c)| contains_split(c)),
        }
    }
}
