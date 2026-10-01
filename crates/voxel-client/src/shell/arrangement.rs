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
            return Err(UiRefusal::EmptyViewWithoutNote { what: "a window id" });
        }
        Ok(WindowId(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A node in the arrangement tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// A slot holding a window id. **A window with no split is one pane** (§3).
    Pane(WindowId),
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
        Arrangement { root: Node::Pane(window) }
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
    pub fn windows(&self) -> BTreeSet<WindowId> {
        let mut out = BTreeSet::new();
        collect_panes(&self.root, &mut out);
        out
    }

    /// How many panes the arrangement holds.
    pub fn panes(&self) -> usize {
        self.windows().len()
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
        if self.windows().contains(&new_pane) {
            // **Refused, because the reference does not check and should have.** Two panes
            // holding one window id means the arrangement says the window is in two places,
            // and `close` would then have to guess which. This is a case where reading the
            // reference is not enough — the reference is a UI and can render a contradiction;
            // a data model cannot.
            return Err(UiRefusal::DuplicateWindow {
                window: new_pane.as_str().to_string(),
            });
        }
        // **The target must be somewhere, or the split has nothing to split.** The reference
        // falls back to an outer split when `target_pane` is not found, because a Qt widget
        // that has been re-parented away is a thing that happens silently. A data model can
        // refuse instead, and refusing is why this check is here rather than in the caller.
        if let Some(target) = target {
            if !self.windows().contains(target) {
                return Err(UiRefusal::UnknownWindow {
                    window: target.as_str().to_string(),
                });
            }
        }
        self.root = split_node(&self.root, target, orientation, side, new_pane.clone());
        Ok(new_pane)
    }

    /// `undock(pane)` — remove one pane and return an arrangement holding just it.
    ///
    /// **A window becomes free-floating** (H-7.24: *"a window may ignore the grid entirely"*).
    /// The reference re-parents the pane out of its splitter and leaves the window standing; so
    /// the arrangement is one window again and the caller decides where it lives.
    pub fn undock(&self, target: &WindowId) -> Option<Arrangement> {
        self.windows().contains(target).then(|| Arrangement::of(target.clone()))
    }

    /// `close(pane)` — remove one pane, **collapsing one-child splitters back up, recursively**.
    ///
    /// **`None` when the last pane closes**, per §3: *"a window closes when its last pane
    /// does."* An arrangement with no windows describes nothing, so the type has no way to
    /// spell that state — the same rule as no `new`.
    ///
    /// **The collapse is the reference's `_cleanup_splitter`, including its recursion.** A
    /// splitter that drops to one child is replaced by that child; a splitter that drops to
    /// zero cannot exist because its parent already took its place. The recursion upward is
    /// the rule, and stopping at one level leaves a chain of one-child splitters — which is
    /// exactly the accumulation §3 calls the layout's hard-won detail.
    pub fn close(&self, target: &WindowId) -> Option<Arrangement> {
        let remaining: BTreeSet<WindowId> =
            self.windows().into_iter().filter(|id| id != target).collect();
        if remaining.is_empty() {
            return None;
        }
        let mut out = Arrangement { root: remove_pane(&self.root, target) };
        out.root = collapse(out.root);
        debug_assert_eq!(out.windows(), remaining, "closing a pane removes exactly that pane");
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
}

fn collect_panes(node: &Node, out: &mut BTreeSet<WindowId>) {
    match node {
        Node::Pane(id) => {
            out.insert(id.clone());
        }
        Node::Split { children, .. } => {
            for (_, child) in children {
                collect_panes(child, out);
            }
        }
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

fn split_node(
    node: &Node,
    target: Option<&WindowId>,
    orientation: Orientation,
    side: Side,
    new_pane: WindowId,
) -> Node {
    // Case 1 — no target at all, or the target *is* the root pane.
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
    let target_is_root = match (target, node) {
        (None, _) => true,
        (Some(id), Node::Pane(root)) => id == root,
        (Some(_), Node::Split { .. }) => false,
    };
    if target_is_root {
        return match node {
            Node::Split { orientation: root, children } if *root == orientation => {
                // Case 2 — same orientation: **insert**, and re-divide the whole root.
                let mut next = children.clone();
                let at = match side {
                    Side::Before => 0,
                    Side::After => next.len(),
                };
                next.insert(at, (1.0, Node::Pane(new_pane)));
                Node::Split { orientation, children: with_equal_shares(next) }
            }
            other => {
                // Case 3 — opposing orientation, or a bare pane: **wrap the root**.
                Node::Split {
                    orientation,
                    children: match (other, side) {
                        (Node::Pane(existing), Side::Before) => vec![
                            (1.0, Node::Pane(new_pane)),
                            (1.0, Node::Pane(existing.clone())),
                        ],
                        (Node::Pane(existing), Side::After) => vec![
                            (1.0, Node::Pane(existing.clone())),
                            (1.0, Node::Pane(new_pane)),
                        ],
                        // **A wrapped root splitter is re-nested, not flattened.** The
                        // reference builds `new_root` around `old_root` wholesale; flattening
                        // would silently reorder every pane in the old tree.
                        (Node::Split { .. }, Side::Before) => {
                            let mut next = vec![(1.0, Node::Pane(new_pane))];
                            next.push((1.0, other.clone()));
                            next
                        }
                        (Node::Split { .. }, Side::After) => {
                            vec![(1.0, other.clone()), (1.0, Node::Pane(new_pane))]
                        }
                    },
                }
            }
        };
    }

    // Case 4 — the target is a pane inside a splitter.
    //
    // **`target` is `Some` here and the compiler does not know it.** The root cases above all
    // return, so the only way to reach this arm is a non-root target — but `target` is still
    // typed `Option<&WindowId>`, so it is unwrapped *by name* rather than by a control-flow
    // fact the reader has to reconstruct. A `.expect` on a case that cannot fail is better
    // than a silent `None` that would drop the split.
    let target = target.expect("the root cases return above, so the target is Some here");
    match node {
        Node::Pane(_) => Node::Pane(new_pane),
        Node::Split { orientation: parent, children } => {
            if let Some(index) = children.iter().position(|(_, child)| holds(child, target)) {
                let (outer, child) = &children[index];
                match &child {
                    // Same orientation: **insert beside it**, and re-divide this splitter.
                    Node::Split { orientation: inner, children: inner_children }
                        if *inner == orientation =>
                    {
                        let mut next = inner_children.clone();
                        let at = match side {
                            Side::Before => 0,
                            Side::After => next.len(),
                        };
                        next.insert(at, (1.0, Node::Pane(new_pane)));
                        let mut rebuilt = children.clone();
                        rebuilt[index] = (
                            *outer,
                            Node::Split {
                                orientation: *inner,
                                children: with_equal_shares(next),
                            },
                        );
                        // **The parent's shares are returned untouched, and that is the whole
                        // point.** Only the splitter that *gained* a child re-divides; the
                        // reference calls `setSizes` on that one and leaves its ancestors
                        // alone. A `re_divide` here would resize every ancestor on every
                        // nested insert, which is the drift §3's floor note is about.
                        Node::Split { orientation: *parent, children: rebuilt }
                    }
                    // Opposing orientation, or a bare pane: **wrap it**, and keep the outer
                    // split's shares untouched — the reference's `setSizes(old_sizes)`.
                    _ => {
                        let mut rebuilt = children.clone();
                        rebuilt[index] = (
                            *outer,
                            Node::Split {
                                orientation,
                                children: match side {
                                    Side::Before => vec![
                                        (1.0, Node::Pane(new_pane)),
                                        (1.0, child.clone()),
                                    ],
                                    Side::After => {
                                        vec![(1.0, child.clone()), (1.0, Node::Pane(new_pane))]
                                    }
                                },
                            },
                        );
                        Node::Split { orientation: *parent, children: rebuilt }
                    }
                }
            } else {
                // **The target is not in this tree, and that is a refusal the caller has
                // already prevented** — `split` checks the tree for the new pane's id, and the
                // target is validated there too. Reaching here means the two checks disagree,
                // so the tree is returned untouched rather than split somewhere arbitrary.
                Node::Split { orientation: *parent, children: children.clone() }
            }
        }
    }
}

fn holds(node: &Node, target: &WindowId) -> bool {
    match node {
        Node::Pane(id) => id == target,
        Node::Split { children, .. } => children.iter().any(|(_, child)| holds(child, target)),
    }
}

fn remove_pane(node: &Node, target: &WindowId) -> Node {
    match node {
        Node::Pane(id) if id == target => Node::Pane(WindowId(String::new())),
        Node::Pane(id) => Node::Pane(id.clone()),
        Node::Split { orientation, children } => Node::Split {
            orientation: *orientation,
            children: children
                .iter()
                .map(|(share, child)| (*share, remove_pane(child, target)))
                .collect(),
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
        Node::Split { orientation, children } => {
            let children: Vec<(f32, Node)> = children
                .into_iter()
                .map(|(share, child)| (share, collapse(child)))
                .collect();
            // **A pane emptied by `close` is dropped, not kept as a hole.** The reference
            // reparents and deletes it; a placeholder is a *UI* state for a pane holding no
            // window, and this tree only ever holds ids that exist.
            let children: Vec<(f32, Node)> = children
                .into_iter()
                .filter(|(_, child)| !is_hole(child))
                .collect();
            match children.len() {
                0 => Node::Pane(WindowId(String::new())),
                1 => children.into_iter().next().expect("one child").1,
                _ => Node::Split { orientation, children },
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
        Node::Pane(_) => return Ok(()),
        Node::Split { orientation, children } => (orientation, children),
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
            Node::Pane(_) => {}
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
            Node::Split { orientation, children } => {
                assert_eq!(*orientation, Orientation::Horizontal);
                assert_eq!(children.len(), 2);
                assert!(holds(&children[0].1, &id("alpha")), "the old pane keeps its side");
                assert!(holds(&children[1].1, &id("beta")), "the new pane arrives after");
            }
            other => panic!("the root did not become a splitter: {other:?}"),
        }
    }

    /// **§3: "inserts into an existing splitter when the orientation agrees."** The reference
    /// re-divides the whole splitter on insert, and the two children get equal shares.
    #[test]
    fn a_matching_orientation_inserts_and_re_divides() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement.split(None, Orientation::Horizontal, Side::After, id("beta")).expect("splits");
        arrangement.split(None, Orientation::Horizontal, Side::After, id("gamma")).expect("splits");
        assert_eq!(arrangement.panes(), 3);
        match arrangement.root() {
            Node::Split { children, .. } => {
                assert_eq!(children.len(), 3, "inserted, not wrapped");
                for (share, _) in children {
                    assert!((share - 1.0 / 3.0).abs() < 1e-6, "re-divided equally, got {share}");
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
        arrangement.split(None, Orientation::Horizontal, Side::After, id("beta")).expect("splits");
        arrangement.split(None, Orientation::Horizontal, Side::After, id("gamma")).expect("splits");
        let outer_before: Vec<f32> = match arrangement.root() {
            Node::Split { children, .. } => children.iter().map(|(s, _)| *s).collect(),
            other => panic!("not a splitter: {other:?}"),
        };

        // **Split `gamma` vertically** — opposing the horizontal root, so this is a wrap.
        arrangement
            .split(Some(&id("gamma")), Orientation::Vertical, Side::After, id("delta"))
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
        arrangement.split(None, Orientation::Horizontal, Side::After, id("beta")).expect("splits");
        arrangement
            .split(Some(&id("beta")), Orientation::Vertical, Side::After, id("gamma"))
            .expect("splits");

        let after = arrangement.close(&id("gamma")).expect("two panes remain");
        assert_eq!(after.panes(), 2, "alpha and beta both survive closing gamma");
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
            arrangement.close(&id("alpha")),
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
        arrangement.split(None, Orientation::Horizontal, Side::After, id("beta")).expect("splits");
        arrangement
            .split(Some(&id("beta")), Orientation::Vertical, Side::After, id("gamma"))
            .expect("splits");
        arrangement
            .split(Some(&id("gamma")), Orientation::Horizontal, Side::After, id("delta"))
            .expect("splits");

        assert!(arrangement.check_floor(1000).is_ok(), "1000 px clears three levels");
        assert!(
            arrangement.check_floor(200).is_err(),
            "**200 px cannot give four panes 40 each**, and the refusal names the arithmetic \
             rather than letting a layout pass discover it after the window locks"
        );
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

    #[test]
    fn undock_leaves_the_window_free_floating() {
        let mut arrangement = Arrangement::of(id("alpha"));
        arrangement.split(None, Orientation::Horizontal, Side::After, id("beta")).expect("splits");
        let floating = arrangement.undock(&id("beta")).expect("beta is in the arrangement");
        assert_eq!(floating.panes(), 1, "H-7.24: a window may ignore the grid entirely");
        assert!(floating.windows().contains(&id("beta")));
    }

    fn contains_split(node: &Node) -> bool {
        match node {
            Node::Pane(_) => false,
            Node::Split { children, .. } => children.iter().any(|(_, c)| contains_split(c)),
        }
    }
}
