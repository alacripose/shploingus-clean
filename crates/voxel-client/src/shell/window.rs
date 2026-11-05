//! **The window: its state, its tree, its drawing, and the machine that moves it.**
//!
//! Reference: `docs/portal_simulator.py` — `PortalWindow` 2778–3473. Two halves, both here:
//!
//! * **the lifecycle** — `split_pane` 2919–3012, `dock_window` 3013–3039, `undock_pane`
//!   3040–3085, `close_pane` 3086–3110, `toggle_maximize` 3540–3565's window-side arithmetic,
//!   `minimize`/`restore` 3566–3586, `get_all_panes` 2890–2904;
//! * **the composite and the mouse machine** — the composite construction 2817–2872 (the
//!   titlebar slot per `title_policy`, the content container), the pane chrome and seams, the
//!   drop preview on screen, `start_window_drag`/`continue_window_drag`/`end_window_drag`
//!   3165–3269, `_position_edge_triggers`/`show_edge_triggers`/`hide_edge_triggers` 3270–3296,
//!   `set_focus_visual` 3312–3320, `mousePressEvent` 3321–3353, `mouseMoveEvent` 3354–3390,
//!   `mouseReleaseEvent` 3391–3402, `leaveEvent` 3403–3405, `_edges_at` 3420–3430,
//!   `_cursor_for_edges` 3432–3442, `_resize_from` 3444–3473.
//!
//! # What this module is and is not
//!
//! It is the **state and the tree** of one top-level window, drawn from its own declaration
//! (`WindowSpec`), with the tree from `arrangement.rs` and the layout from `splitter.rs` —
//! one arithmetic, again. `State` is the shell's own `Status` (`mount.rs`), which is already
//! the reference's `WindowState` (76–82) value for value; **step 3 deletes the derived core
//! and this module takes over**, so no second state enum is created.
//!
//! The **pane ids are windows** in this tree's model (H-9.7: one kind of window): a pane holds
//! a [`WindowId`], and a docked window's id appears in its host's arrangement while its own
//! `host` field names the host — the reference reaches the same place through Qt's widget
//! parenting.
//!
//! **The window owns no registry.** Which panes hold which windows, what each pane's title is,
//! and which pane is empty are facts of the caller's window set; the drawing takes them as
//! values ([`WindowView`], `titles`) so this module stays a value type and the caller stays the
//! one owner of the set.
//!
//! # What changes, and why it is named
//!
//! * **The composite's border is 1 dp, the reference's `border_width`** (2819) — the tree's
//!   divider step, not the 8 dp grid step: the reference's own value is 1, and a window border
//!   is a line. A **maximized** window's margins are the grid step, where the reference writes
//!   `8` literally (3547–3553) — the same number, from the declared grid.
//! * **The minimum size is the declaration's** (`WindowSpec::min_size`), not a policy struct's
//!   (`WindowLayoutPolicy.min_width/min_height`, 90–102) — the package declares it.
//! * **`dock` takes the zone** the drop preview showed ([`DropZone`](crate::shell::drop::DropZone)),
//!   and performs the reference's own reversal for the `Before` side (3025–3031), so a
//!   multi-pane dock keeps its input order on both sides.
//! * **The resize margin is the grid step** (8), the reference's `resize_margin` (92) read from
//!   the declared grid — and it is *not* the drop rule's 24 dp band
//!   ([`EDGE_TRIGGER_DP`](crate::shell::drop::EDGE_TRIGGER_DP)), which is a different rule.
//! * **The edge triggers are pointer targets** (40 dp lanes, D-1) at the reference's own
//!   positions (3270–3283: the side pair at the content's mid-height, the top/bottom pair
//!   centred), and the hover band is `inset + lane` so **the band is exactly the lanes' own
//!   extent** — the reference's 28 = its 4 + its 24, restated with the ruled numbers.
//! * **One event, one verdict.** The reference answers a press three times (its own handler,
//!   the titlebar's, the pane's) and a double-click a fourth; here [`PortalWindow::press`]
//!   answers once, and **a double-click asks the title bar first** so a double-click on the top
//!   strip maximizes rather than resizing (the reference would do both).
//! * **`set_focus_visual` is not a stylesheet pass.** The reference re-polishes Qt's property
//!   engine (3312–3320); here focus is a value the drawing is handed — [`WindowView::focused`] —
//!   so an unfocused window draws its title one ink step down (`titlebar.rs`) and its outline
//!   at the divider step, the tree's declared focus language.
//! * **The drop preview is measured from the content container**, not the reference's window top
//!   plus `tb_h` (3179): the preview shows the rect the panes were laid out in, which is the
//!   same rect a dock lands in.
//!
//! # What this module does not draw
//!
//! The **content a pane hosts** — the four HUD panels, the console's three regions — belongs to
//! the caller (`ContentViews`, H-7.17). [`PortalWindow::elements`] returns the [`Layout`] it
//! drew the chrome from, so the caller places content against **the same rects**, which is
//! `SHELL-PORT.md` §5's one-arithmetic rule made structural: content cannot be drawn without
//! the layout the chrome used.
//!
//! # What is owed, and named
//!
//! **The slot-identity rule.** The pane header's `◧`/`⬓` split controls and the placeholder's
//! offers both need an **empty pane**, and this model's pane holds a window id; both refuse by
//! name until the rule lands (LEDGER C-116 §116.4). The seam drag is no longer owed: a press on
//! a handle answers [`WindowPress::Seam`] carrying the seam's address ([`splitter::Seam`]), and
//! [`SeamDrag`] is the state a caller keeps while it moves (`arrangement.rs::drag_seam` is the
//! mutation, the 40 dp floor its clamp).

use std::collections::{BTreeMap, BTreeSet};

use crate::shell::arrangement::{Arrangement, Node, Orientation, PaneRef, Side, WindowId};
use crate::shell::drop::{self, DropTarget, DropZone};
use crate::shell::mount::Status;
use crate::shell::pane::{self, PanePress};
use crate::shell::placeholder;
use crate::shell::splitter::{self, Layout};
use crate::shell::titlebar::{self, TitlePress};
use voxel_package::recipe::{BehaviorPolicy, LayoutPolicy, WindowSpec};
use voxel_ui::{
    contrast_over_backdrop, drop_zone_mark, pointer_glyph_dp, pointer_target_dp, spacing_dp, Align,
    Element, GlyphAction, Ink, Paint, Palette, PaneMark, Rect, Role, UiResult, WHITE,
};

/// **The window's border, in dp.** The reference's `WindowLayoutPolicy.border_width = 1` (2819)
/// and the tree's declared 1 dp divider step — the same number from two directions, which is why
/// it is stated rather than derived from the 8 dp grid.
pub const BORDER_DP: f64 = 1.0;

/// One top-level window: its declaration, its rect, its run state, and its pane tree.
///
/// **Cloneable, because the desktop holds it.** The reference's windows are Qt widgets, where a
/// second reference to one is free; here a window is a value and the registry
/// ([`Desktop`](crate::shell::desktop::Desktop)) needs to own it while callers read it. The
/// clone is the whole window — no shared mutable interior — so two copies cannot disagree.
#[derive(Clone)]
pub struct PortalWindow {
    /// The declaration travels with the window (the same rule `mount.rs`'s `Window` states):
    /// the title bar's controls, the floor and the content all come from here.
    pub spec: WindowSpec,
    /// The window's rect in dp when it is normal. A maximized window keeps the previous rect
    /// so restoring goes back to where it was.
    pub rect: Rect,
    /// The run state. `Status` is the shell's own enum (`mount.rs`), which is already the
    /// reference's `WindowState` (76–82) value for value — no second enum exists.
    pub status: Status,
    /// Where `restore` goes back to (the reference's `geometry_state.previous`, 3543–3546).
    pub previous: Option<Rect>,
    /// The pane tree. One pane, holding this window's own id, is the unstyled case.
    pub arrangement: Arrangement,
    /// The window that holds this one as a pane, when it is docked.
    pub host: Option<WindowId>,
    /// **Whether this window is pinned**: drawn while the shell is hidden, above the floating
    /// windows, with the minimal title bar.
    ///
    /// **Runtime state, and it is the only copy.** The package's `pin` rows say what is
    /// pinned when it is mounted; from then on the player's pin button is the writer, and
    /// `Mount::draw_order`, the HUD's layer choice and the debug lanes all read this field.
    /// `spec.pinnable` stays where the declaration put it — the recipe's own statement that a
    /// window *may* live in the HUD, which is what its validator checks a `pin` row against.
    pub pinned: bool,
}

/// The rects the composite hands out: the title bar's slot, and the content container's.
///
/// **One arithmetic**, and every rect in this module resolves against this pair — the drawing,
/// the pane hit tests, the drop preview and the edge triggers. That is `SHELL-PORT.md` §5's
/// whole point: the derived shell's pane-rect mismatch came from a second arithmetic existing
/// at all.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Composite {
    /// The title bar's slot, or `None` when the declaration draws no bar.
    pub title: Option<Rect>,
    /// The content container: what the arrangement is laid out inside.
    pub content: Rect,
}

/// The composite of a window: the border, the bar's slot per the declaration, the content.
///
/// The reference's `root_layout` with `border_width` 1 and the titlebar added with stretch 0
/// (2817–2846); the bar occupies [`titlebar::bar_height_dp`] when the declaration permits a bar
/// (`titlebar::has_bar`). **The reference's `BOTTOM` position is not ported**: the package's
/// `LayoutPolicy::TITLE_POSITION` is a permit, not a position, and a second position token is
/// not in its vocabulary — a bottom bar would be invented here.
pub fn composite(spec: &WindowSpec, rect: Rect, pinned: bool) -> Composite {
    let inner = Rect::new(
        rect.x + BORDER_DP,
        rect.y + BORDER_DP,
        (rect.width_dp - 2.0 * BORDER_DP).max(0.0),
        (rect.height_dp - 2.0 * BORDER_DP).max(0.0),
    );
    if !titlebar::has_bar(spec) {
        return Composite {
            title: None,
            content: inner,
        };
    }
    // **The bar's own form, from the pin state**: a pinned window draws the minimal strip and
    // its body starts that much higher.
    let bar = titlebar::BarStyle::of(pinned).rect(Rect::new(
        inner.x,
        inner.y,
        inner.width_dp,
        inner.height_dp,
    ));
    let content = Rect::new(
        inner.x,
        bar.bottom(),
        inner.width_dp,
        (inner.height_dp - bar.height_dp).max(0.0),
    );
    Composite {
        title: Some(bar),
        content,
    }
}

/// The title bar's height for a window that draws one, else `0` — the reference's `tb_h`
/// (3179: `other_win.titlebar.height() if … else 0`), which the drop rule and the edge triggers
/// both measure from.
pub fn title_height(spec: &WindowSpec, pinned: bool) -> f64 {
    if titlebar::has_bar(spec) {
        titlebar::BarStyle::of(pinned).height_dp
    } else {
        0.0
    }
}

/// **What a pane's split control means**, in the reference's own wiring (`PaneHeader.__init__`,
/// 2534–2552): `◧` emits `splitRequested(pane, Horizontal, "after")` and `⬓` emits `Vertical`.
/// The other two controls are not splits and answer `None`.
pub fn split_axis(mark: PaneMark) -> Option<(Orientation, Side)> {
    match mark {
        PaneMark::SplitHorizontal => Some((Orientation::Horizontal, Side::After)),
        PaneMark::SplitVertical => Some((Orientation::Vertical, Side::After)),
        PaneMark::Undock | PaneMark::Close => None,
    }
}

impl PortalWindow {
    /// A window with no split: one pane, holding the window's own id (UI-SPEC §3).
    pub fn new(spec: WindowSpec, rect: Rect) -> UiResult<Self> {
        let id = WindowId::new(spec.id.clone())?;
        Ok(PortalWindow {
            spec,
            rect,
            status: Status::Normal,
            previous: None,
            arrangement: Arrangement::of(id),
            host: None,
            pinned: false,
        })
    }

    /// Whether the title bar carries this control, from the **declared** layout policy.
    ///
    /// **One implementation, and it is `titlebar::controls`'.** This is the membership form of
    /// the same question, kept here so a caller holding a window can ask it without a second
    /// walk — the predicate and the list cannot disagree because the list is built from it.
    pub fn permits(&self, action: GlyphAction) -> bool {
        let policy = self.spec.layout_policy;
        match action {
            // **The pin is always permitted**: it is the window's runtime state, not a
            // declared chrome bit (`titlebar::controls` says the same in list form).
            GlyphAction::Pin => true,
            GlyphAction::Minimize => policy.permits(LayoutPolicy::MINIMIZE),
            GlyphAction::Maximize => policy.permits(LayoutPolicy::MAXIMIZE),
            GlyphAction::Close => policy.permits(LayoutPolicy::CLOSE),
        }
    }

    /// **The bar this window draws**, from its pin state. The composite, the drawing and the
    /// hit tests all ask this one question.
    pub fn bar_style(&self) -> titlebar::BarStyle {
        titlebar::BarStyle::of(self.pinned)
    }

    /// **Pin or unpin the window.** Returns the new state. The caller that owns the layer
    /// (`Mount`) decides what a pinned window draws over; this only records the state the
    /// chrome reads.
    pub fn set_pinned(&mut self, pinned: bool) -> bool {
        self.pinned = pinned;
        self.pinned
    }

    /// Flip the pin. The pin glyph's press is the caller's; this is the state it changes.
    pub fn toggle_pin(&mut self) -> bool {
        self.set_pinned(!self.pinned)
    }

    /// The controls the title bar draws, in the declared order. **An unpermitted control is
    /// absent**, which is the HIG rule the record keeps: a disabled control that looks like a
    /// grant is worse than no control.
    pub fn controls(&self) -> Vec<GlyphAction> {
        titlebar::controls(&self.spec)
    }

    /// Every pane the window holds, in tree order (the reference's `get_all_panes`, 2890–2904).
    pub fn pane_ids(&self) -> BTreeSet<WindowId> {
        self.arrangement.windows()
    }

    /// Whether this window is drawn on its own; a docked window is drawn by its host.
    pub fn is_standalone(&self) -> bool {
        self.host.is_none()
    }

    /// **Whether this window's panes draw headers.** One pane holding the window itself does
    /// not: the title bar already names it, and a header underneath named the same window a
    /// second time in a 40 dp row the package never reserved — which is what put every window's
    /// title on screen twice and pushed the body past its declared geometry. A window the
    /// operator has split, or one holding a foreign pane, draws a header per pane, because then
    /// the header is the only thing naming each one.
    pub fn draws_pane_headers(&self) -> bool {
        !matches!(self.arrangement.root(), Node::Pane(id) if id.as_str() == self.spec.id)
    }

    /// The pane tree laid out inside the window's content container — the pass the drawing and
    /// the hit tests share (`splitter.rs`).
    pub fn layout(&self) -> UiResult<Layout> {
        splitter::layout(&self.arrangement, self.content())
    }

    /// The same layout **at a rect the caller supplies** — the entry a maximized window needs,
    /// because its drawn rect is the viewport's arithmetic and this value still holds the rect
    /// it will restore to. One implementation: this is [`PortalWindow::layout`] with the rect
    /// stated.
    pub fn layout_at(&self, rect: Rect) -> UiResult<Layout> {
        splitter::layout(
            &self.arrangement,
            composite(&self.spec, rect, self.pinned).content,
        )
    }

    /// The composite's content container — the rect the arrangement lives in.
    pub fn content(&self) -> Rect {
        composite(&self.spec, self.rect, self.pinned).content
    }

    /// The content container **at a supplied rect** (see [`PortalWindow::layout_at`]).
    ///
    /// A method rather than a free function, because the container depends on the bar's form
    /// and the bar's form is this window's pin state — an arithmetic that took a bare spec
    /// could place a pinned window's body under a bar it does not draw.
    pub fn content_at(&self, rect: Rect) -> Rect {
        composite(&self.spec, rect, self.pinned).content
    }

    /// The title bar's slot, when the declaration permits one.
    pub fn title_slot(&self) -> Option<Rect> {
        composite(&self.spec, self.rect, self.pinned).title
    }

    /// The title bar's slot **at a supplied rect**.
    pub fn title_slot_at(&self, rect: Rect) -> Option<Rect> {
        composite(&self.spec, rect, self.pinned).title
    }

    /// The bar's height at the window's own form — `0` when it draws no bar.
    pub fn title_height(&self) -> f64 {
        title_height(&self.spec, self.pinned)
    }

    /// `split(pane, h|v)`: the arrangement's own three cases, with the window's state kept.
    pub fn split(
        &mut self,
        target: Option<&WindowId>,
        orientation: Orientation,
        side: Side,
        new_pane: WindowId,
    ) -> UiResult<WindowId> {
        self.arrangement.split(target, orientation, side, new_pane)
    }

    /// `split(pane, h|v)` **into an empty slot** — the reference's `split_pane` with
    /// `new_pane=None` (2920–2921), the form the pane header's `◧`/`⬓` and the outer edge
    /// triggers take; the returned [`PaneRef`] is the new slot's address.
    pub fn split_slot(
        &mut self,
        target: Option<&PaneRef>,
        orientation: Orientation,
        side: Side,
    ) -> UiResult<PaneRef> {
        self.arrangement.split_slot(target, orientation, side)
    }

    /// `dock_window(source, target_pane, zone)` (3013–3039): every incoming pane is split into
    /// the tree at the target (or the reference's root case), in the zone's orientation and
    /// side, **with the reference's reversal for `Before`** so the input order survives the
    /// chained inserts.
    ///
    /// **The target is a [`PaneRef`]** — the drop can land on a slot as well as a window pane
    /// — and **each new pane becomes the next insert's target**, which is the reference's own
    /// `curr_target = self.split_pane(curr_target, …)` (3036–3037): docking several panes into
    /// one window lays them out in a row, not all against the original target.
    pub fn dock(
        &mut self,
        panes: &[WindowId],
        target: Option<&PaneRef>,
        zone: DropZone,
    ) -> UiResult<()> {
        let orientation = match zone {
            DropZone::Left | DropZone::Right => Orientation::Horizontal,
            DropZone::Top | DropZone::Bottom => Orientation::Vertical,
        };
        let side = match zone {
            DropZone::Left | DropZone::Top => Side::Before,
            DropZone::Right | DropZone::Bottom => Side::After,
        };
        let ordered: Vec<WindowId> = if side == Side::Before {
            panes.iter().rev().cloned().collect()
        } else {
            panes.to_vec()
        };
        let mut current = target.cloned();
        for pane in ordered {
            current = Some(self.arrangement.split_pane(
                current.as_ref(),
                orientation,
                side,
                Some(&pane),
            )?);
        }
        Ok(())
    }

    /// `undock_pane(pane)` (3040–3085): the pane leaves the tree and is returned as a window of
    /// its own; **the window closes when its last pane does** (3081–3084).
    ///
    /// **Only a pane holding a window can be undocked here**: an empty slot has no id for this
    /// model's registry to file the new window under, so the pane header does not offer undock
    /// on a slot (`window.rs::elements_at`).
    pub fn undock(&mut self, pane: &WindowId) -> Option<WindowId> {
        let freed = self.arrangement.undock(pane)?;
        let id = freed.windows().into_iter().next()?;
        match self.arrangement.close(&PaneRef::Window(pane.clone())) {
            Some(next) => self.arrangement = next,
            None => self.status = Status::Closed,
        }
        Some(id)
    }

    /// `close_pane(pane)` (3086–3110). Returns whether the window survives — **the last pane
    /// closing closes the window**, and a slot is a pane, so a window of one window-pane and one
    /// slot survives closing either.
    pub fn close_pane(&mut self, pane: &PaneRef) -> bool {
        match self.arrangement.close(pane) {
            Some(next) => {
                self.arrangement = next;
                true
            }
            None => {
                self.status = Status::Closed;
                false
            }
        }
    }

    /// `toggle_maximize` (3540–3565): the previous rect is saved, the maximized rect is the
    /// desktop less the grid step's margins and the taskbar's height, and the declaration's
    /// minimum size is the floor — the reference's own arithmetic, its `8` and its
    /// `min_width/min_height` read from the grid and the declaration.
    pub fn toggle_maximize(&mut self, desktop: Rect, taskbar: f64) -> UiResult<()> {
        if self.status == Status::Maximized {
            if let Some(previous) = self.previous.take() {
                self.rect = previous;
            }
            self.status = Status::Normal;
            return Ok(());
        }
        self.previous = Some(self.rect);
        let margin = spacing_dp();
        let min_width = self.spec.min_size[0] as f64;
        let min_height = self.spec.min_size[1] as f64;
        self.rect = Rect::new(
            desktop.x + margin,
            desktop.y + margin,
            (desktop.width_dp - 2.0 * margin).max(min_width),
            (desktop.height_dp - taskbar - 2.0 * margin).max(min_height),
        );
        self.status = Status::Maximized;
        Ok(())
    }

    /// `minimize` (3566–3574).
    pub fn minimize(&mut self) {
        self.status = Status::Minimized;
    }

    /// `restore` (3575–3581).
    pub fn restore(&mut self) {
        self.status = Status::Normal;
    }

    /// Whether the window's chrome is drawn at all this frame.
    pub fn is_drawn(&self) -> bool {
        !matches!(self.status, Status::Minimized | Status::Closed)
    }

    // -----------------------------------------------------------------------
    // The composite drawing
    // -----------------------------------------------------------------------

    /// The window's chrome: its surface and outline, the title bar, the pane headers and
    /// placeholders, the seams, the drop preview, and the edge triggers — and **the layout it
    /// drew them from**, which is what the caller places content against.
    ///
    /// `titles` is what each pane's header draws, keyed by pane id. **A pane with no entry is a
    /// slot with no window** — the reference's empty `WindowPane` (2677–2685) — and draws the
    /// placeholder with the caller's `offers` ([`WindowView::offers`]). That is the one reading
    /// of the map: every pane id in this tree's model names a window, so an id the caller cannot
    /// title is an id the caller has no window for.
    ///
    /// A pane whose `titles` entry is absent *and* whose id is not in the tree cannot happen; a
    /// drop preview naming a pane the layout does not hold draws **nothing** rather than a
    /// window-edge preview that would show where the drop would not land.
    pub fn elements(
        &self,
        view: &WindowView,
        palette: &Palette,
        titles: &BTreeMap<String, String>,
        out: &mut Vec<Element>,
    ) -> UiResult<Layout> {
        self.elements_at(self.rect, view, palette, titles, out)
    }

    /// The same drawing **at a rect the caller supplies** — the maximized window's entry (its
    /// drawn rect is the viewport's arithmetic; its own `rect` is where it restores to). One
    /// implementation: [`PortalWindow::elements`] is this with `self.rect`.
    pub fn elements_at(
        &self,
        rect: Rect,
        view: &WindowView,
        palette: &Palette,
        titles: &BTreeMap<String, String>,
        out: &mut Vec<Element>,
    ) -> UiResult<Layout> {
        if !self.is_drawn() {
            return Ok(Layout {
                panes: Vec::new(),
                seams: Vec::new(),
            });
        }
        let composite = composite(&self.spec, rect, self.pinned);
        let level = self.spec.elevation;
        let opacity = (self.spec.opacity as f64 / 1000.0).clamp(0.0, 1.0);

        // The window's own surface: the declared body role, tinted by its level (C-109).
        let body = palette
            .get(Role::from_theme_role(&self.spec.theme_role)?)?
            .elevated(level)?;
        out.push(Element::Surface {
            rect,
            paint: Paint {
                rgb: body,
                alpha: opacity,
            },
        });
        // The outline: one ink step brighter when focused. `set_focus_visual` (3312–3320) is
        // this, applied at draw time — the reference re-polishes a stylesheet property instead.
        let outline = if view.focused {
            Ink::Secondary
        } else {
            Ink::Divider
        };
        for edge in [
            Rect::new(rect.x, rect.y, rect.width_dp, BORDER_DP),
            Rect::new(rect.x, rect.bottom() - BORDER_DP, rect.width_dp, BORDER_DP),
            Rect::new(rect.x, rect.y, BORDER_DP, rect.height_dp),
            Rect::new(rect.right() - BORDER_DP, rect.y, BORDER_DP, rect.height_dp),
        ] {
            out.push(Element::Rule {
                rect: edge,
                paint: outline.paint(),
            });
        }

        // The title bar's slot, per the declaration.
        if let Some(bar) = composite.title {
            out.extend(titlebar::elements(
                &self.spec,
                bar,
                &view.title,
                view.focused,
                palette,
                self.bar_style(),
            )?);
        }

        // The pane chrome, then the layout's seams — **at the rect that was drawn**, so a
        // maximized window's hit tests and its chrome share this pass's arithmetic.
        let layout = splitter::layout(&self.arrangement, composite.content)?;
        let undock_shown = self.arrangement.panes() > 1;
        let headers = self.draws_pane_headers();
        for (pane_rect, pane) in &layout.panes {
            // **A single pane that is the window draws no header** (see
            // [`PortalWindow::draws_pane_headers`]): the title bar is its chrome, and the row a
            // header would take is a row of the body the package declared. Its placeholder — an
            // empty slot — still draws, because a slot has no title bar naming it.
            let (title, slot) = match pane {
                PaneRef::Window(id) => (
                    titles
                        .get(id.as_str())
                        .cloned()
                        .unwrap_or_else(|| id.as_str().to_string()),
                    false,
                ),
                PaneRef::Slot(_) => (pane::empty_pane_title(), true),
            };
            // **An empty slot is never the `headers == false` case** — that case is a root
            // pane holding this window — so a slot always has a header to draw here.
            if headers {
                out.extend(pane::elements(
                    *pane_rect,
                    &title,
                    // **A slot cannot be undocked here**: it has no window id, and this model's
                    // registry names windows by id (owed, and named in `arrangement.rs::undock`).
                    undock_shown && !slot,
                    view.hovered_pane_mark,
                    level,
                    opacity,
                    palette,
                )?);
            }
            if slot {
                out.extend(placeholder::elements(
                    pane::body(*pane_rect),
                    &view.offers,
                    level,
                    opacity,
                    palette,
                )?);
            }
        }
        layout.seam_elements(view.hovered_seam, opacity, out);

        // The drop preview, drawn by the window the drop would land in (the reference keeps one
        // overlay per window and shows it on the target, 3264–3269). The window-edge geometry
        // is measured from the content container, which is the rect a dock lands in.
        if let Some(preview) = &view.drop {
            let pane_rect = preview
                .pane
                .as_ref()
                .and_then(|pane| layout.pane_rect(pane));
            if preview.pane.is_none() || pane_rect.is_some() {
                out.extend(drop::elements(
                    composite.content,
                    0.0,
                    preview,
                    pane_rect,
                    opacity,
                    palette,
                )?);
            }
        }

        // The edge triggers, above everything (the reference `raise_`s them, 3280–3281).
        if let Some(triggers) = view.triggers {
            let chrome = palette.get(Role::Chrome)?.elevated(level)?;
            let backdrop = chrome.blend_over(WHITE, opacity);
            contrast_over_backdrop(Ink::Full.composited_over(backdrop), backdrop)?;
            for (zone, lane) in triggers.lanes() {
                let surface = if view.hovered_zone == Some(zone) {
                    chrome.elevated(1)?
                } else {
                    chrome
                };
                out.push(Element::Surface {
                    rect: lane,
                    paint: Paint {
                        rgb: surface,
                        alpha: opacity,
                    },
                });
                let glyph = drop_zone_mark(zone.name())?;
                out.push(Element::Text {
                    rect: Rect::new(
                        lane.x + (lane.width_dp - pointer_glyph_dp()) / 2.0,
                        lane.y + (lane.height_dp - pointer_glyph_dp()) / 2.0,
                        pointer_glyph_dp(),
                        pointer_glyph_dp(),
                    ),
                    text: glyph.to_string(),
                    paint: Ink::Full.paint(),
                    size_dp: pointer_glyph_dp(),
                    align: Align::Start,
                });
            }
        }
        Ok(layout)
    }

    // -----------------------------------------------------------------------
    // The mouse machine
    // -----------------------------------------------------------------------

    /// `mousePressEvent` (3321–3353) as **one verdict**.
    ///
    /// The reference's order is kept: the resize strip first (3326–3338), then the title bar —
    /// a control, a drag, or a double-click's maximize (2479–2515) — then the pane under the
    /// point (2712–2718). A double-click asks the title bar **before** the strip, because the
    /// reference would answer both (its press handler starts a resize and its double-click
    /// handler then maximizes); one event here gets one verdict, and the bar is the one the
    /// user aimed at.
    ///
    /// **The verdict does not apply itself.** Whether a maximized window may be dragged or
    /// resized is the desktop's rule (`mount.rs` ignores a maximized drag today; the reference
    /// moves it) and belongs to `desktop.rs` in step 2 — this module answers, the caller acts.
    ///
    /// **Which panes are empty slots is read from the tree, not from a title map**: the
    /// reference answers `pane.app is None`, and this tree's [`Node::Slot`] is that state, so a
    /// press inside a slot's offer grid is [`WindowPress::Offer`] — the reference's
    /// `PanePlaceholder.appSelected` (2622) routed through one verdict.
    pub fn press(&self, view: &WindowView, at: (f64, f64), double: bool) -> WindowPress {
        self.press_at(self.rect, view, at, double)
    }

    /// The same verdict **at a rect the caller supplies** (see [`PortalWindow::elements_at`]).
    pub fn press_at(
        &self,
        rect: Rect,
        view: &WindowView,
        at: (f64, f64),
        double: bool,
    ) -> WindowPress {
        if !self.is_drawn() || !rect.contains(at) {
            return WindowPress::None;
        }
        let composite = composite(&self.spec, rect, self.pinned);
        let style = self.bar_style();
        if double {
            if let Some(bar) = composite.title {
                if bar.contains(at) {
                    return match titlebar::press(&self.spec, bar, at, true, style) {
                        TitlePress::Maximize => WindowPress::Maximize,
                        TitlePress::Control(action) => WindowPress::Control(action),
                        TitlePress::Drag | TitlePress::None => WindowPress::None,
                    };
                }
            }
        }
        if self.spec.behavior_policy.permits(BehaviorPolicy::RESIZE) {
            let edges = ResizeEdges::at(rect, at);
            if edges.any() {
                return WindowPress::Resize(edges);
            }
        }
        if let Some(bar) = composite.title {
            if bar.contains(at) {
                return match titlebar::press(&self.spec, bar, at, false, style) {
                    TitlePress::Control(action) => WindowPress::Control(action),
                    TitlePress::Drag => WindowPress::Drag,
                    // A single press never maximizes; the double-click path is above.
                    TitlePress::Maximize | TitlePress::None => WindowPress::None,
                };
            }
        }
        // **The edge triggers are the topmost children of the window** — the reference
        // `raise_`s them above the panes (3280–3281), so a press inside a trigger lane is the
        // trigger's and never the pane header or body it overlaps. The lane test is the same
        // [`EdgeTriggers`] the drawing used. A press can only land in a lane while the lanes
        // are shown, because [`PortalWindow::edge_triggers_at`] gates them on the pointer being
        // inside the band the lanes occupy.
        if let Some(zone) = self
            .edge_triggers_at(rect, at)
            .and_then(|triggers| triggers.zone_at(at))
        {
            return WindowPress::Trigger(zone);
        }
        let Ok(layout) = splitter::layout(&self.arrangement, composite.content) else {
            return WindowPress::None;
        };
        // A seam is the narrower test and comes before the pane (a seam overlays two panes).
        if let Some(index) = layout.seam_at(at) {
            return WindowPress::Seam(layout.seams[index]);
        }
        match layout.pane_at(at) {
            Some(pane) => {
                let pane_rect = layout
                    .pane_rect(pane)
                    .expect("a pane in the layout has a rect");
                let undock_shown = self.arrangement.panes() > 1;
                // **A header that is not drawn is not hit-tested.** A single self-hosted pane
                // draws no header, so its top row is body — the reference's own leaf case, with
                // the split and close controls belonging to the window's title bar instead.
                let verdict = if self.draws_pane_headers() {
                    pane::press(
                        pane_rect,
                        undock_shown && matches!(pane, PaneRef::Window(_)),
                        at,
                    )
                } else {
                    PanePress::Body
                };
                match verdict {
                    PanePress::Action(mark) => WindowPress::PaneAction(pane.clone(), mark),
                    PanePress::Body => {
                        // **An empty slot's body is its offer grid**: the placeholder's rows
                        // answer first, then the body itself is the pane (the reference's
                        // `PanePlaceholder.appSelected`, 2622, routed through one verdict).
                        if matches!(pane, PaneRef::Slot(_)) {
                            if let Some(index) =
                                placeholder::press(pane::body(pane_rect), view.offers.len(), at)
                            {
                                return WindowPress::Offer(pane.clone(), index);
                            }
                        }
                        WindowPress::Pane(pane.clone())
                    }
                    PanePress::None => WindowPress::None,
                }
            }
            None => WindowPress::None,
        }
    }

    /// `mouseMoveEvent`'s edge-hover pass (3374–3390): the four trigger lanes when the pointer
    /// is near an edge, `None` otherwise.
    ///
    /// The band is **the lanes' own extent** — `inset + lane` (8 + 40) — where the reference's
    /// is 28 = its 4 + its 24. So the strip that shows a trigger is exactly the strip the lanes
    /// occupy, and the top band is measured from below the title bar as the reference measures
    /// it (`pos.y() >= th`, 3381). The reference hides them on `leaveEvent` (3403–3405) and
    /// after an outer split (2909); both are pointer and desktop facts the caller owns.
    pub fn edge_triggers(&self, at: (f64, f64)) -> Option<EdgeTriggers> {
        self.edge_triggers_at(self.rect, at)
    }

    /// The same hover test **at a rect the caller supplies** (see [`PortalWindow::press_at`]),
    /// so a maximized window's trigger lanes are measured against the rect it is drawn at
    /// rather than the rect it would restore to.
    pub fn edge_triggers_at(&self, rect: Rect, at: (f64, f64)) -> Option<EdgeTriggers> {
        if !self.is_drawn() || !rect.contains(at) {
            return None;
        }
        let band = EdgeTriggers::band_dp();
        let title_bottom = self
            .title_slot_at(rect)
            .map(|bar| bar.bottom())
            .unwrap_or(rect.y);
        let near = at.0 <= rect.x + band
            || at.0 >= rect.right() - band
            || (at.1 >= title_bottom && at.1 <= title_bottom + band)
            || at.1 >= rect.bottom() - band;
        near.then(|| EdgeTriggers::at(rect, title_bottom))
    }

    /// `_resize_from` (3444–3473): the rect a resize drag has reached, from the rect the drag
    /// started at and the pointer's total travel. The declaration's `min_size` is the floor, and
    /// the far edge stays put — the reference's own arithmetic, including that it reads the
    /// start rect for every branch (so a move never compounds).
    pub fn resize_from(&self, start: Rect, edges: ResizeEdges, dx: f64, dy: f64) -> Rect {
        let min_width = self.spec.min_size[0] as f64;
        let min_height = self.spec.min_size[1] as f64;
        let mut rect = start;
        if edges.west {
            let width = (start.width_dp - dx).max(min_width);
            rect.x = start.x + (start.width_dp - width);
            rect.width_dp = width;
        }
        if edges.east {
            rect.width_dp = (start.width_dp + dx).max(min_width);
        }
        if edges.north {
            let height = (start.height_dp - dy).max(min_height);
            rect.y = start.y + (start.height_dp - height);
            rect.height_dp = height;
        }
        if edges.south {
            rect.height_dp = (start.height_dp + dy).max(min_height);
        }
        rect
    }
}

/// The edges a resize can start from: the reference's `_edges_at` (3420–3430), whose `elif`
/// chain gives at most one edge per axis — a window narrower than twice the margin is on its
/// **west** edge and never on both.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ResizeEdges {
    pub west: bool,
    pub east: bool,
    pub north: bool,
    pub south: bool,
}

impl ResizeEdges {
    pub const NONE: Self = Self {
        west: false,
        east: false,
        north: false,
        south: false,
    };

    /// The strip's width: the grid step, the reference's `resize_margin` (92) read from the
    /// declared grid.
    pub fn margin_dp() -> f64 {
        spacing_dp()
    }

    /// Which edges a point is on, if any — the reference's `_edges_at` order (`x <= m` first,
    /// then `x >= width - m`), in the window's own coordinates.
    pub fn at(rect: Rect, at: (f64, f64)) -> Self {
        let margin = Self::margin_dp();
        let west = at.0 <= rect.x + margin;
        let east = !west && at.0 >= rect.right() - margin;
        let north = at.1 <= rect.y + margin;
        let south = !north && at.1 >= rect.bottom() - margin;
        ResizeEdges {
            west,
            east,
            north,
            south,
        }
    }

    pub fn any(self) -> bool {
        self != Self::NONE
    }

    /// `_cursor_for_edges` (3432–3442): the pointer shape a resize strip means, in Qt's own
    /// vocabulary (`SizeFDiagCursor` is the top-left / bottom-right axis) so the bin can
    /// translate it without a second set of names.
    pub fn cursor(self) -> Option<ResizeCursor> {
        let vertical = self.north || self.south;
        let horizontal = self.west || self.east;
        match (vertical, horizontal) {
            (true, true) => {
                let forward = (self.north && self.west) || (self.south && self.east);
                Some(if forward {
                    ResizeCursor::ForwardDiagonal
                } else {
                    ResizeCursor::BackwardDiagonal
                })
            }
            (true, false) => Some(ResizeCursor::NorthSouth),
            (false, true) => Some(ResizeCursor::EastWest),
            (false, false) => None,
        }
    }
}

/// The pointer shapes a resize strip means (the reference's `_cursor_for_edges`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResizeCursor {
    /// Qt's `SizeFDiagCursor` — the top-left / bottom-right axis.
    ForwardDiagonal,
    /// Qt's `SizeBDiagCursor` — the top-right / bottom-left axis.
    BackwardDiagonal,
    NorthSouth,
    EastWest,
}

/// The four edge-trigger lanes, positioned as the reference positions its buttons
/// (`_position_edge_triggers`, 3270–3283): the side pair at the content's mid-height against the
/// left and right edges, the top/bottom pair centred on the window's inline axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeTriggers {
    pub left: Rect,
    pub right: Rect,
    pub top: Rect,
    pub bottom: Rect,
}

impl EdgeTriggers {
    /// The hover band: **the lanes' own extent**, `inset + lane` — the reference's 28 = its 4
    /// inset + its 24 button, restated with the grid step and the dense-pointer target.
    pub fn band_dp() -> f64 {
        spacing_dp() + pointer_target_dp()
    }

    /// The four lanes for a window, given the title bar's bottom edge (the window's top when
    /// the declaration draws no bar).
    pub fn at(rect: Rect, title_bottom: f64) -> Self {
        let lane = pointer_target_dp();
        let inset = spacing_dp();
        let content_height = (rect.bottom() - title_bottom).max(0.0);
        let mid = title_bottom + (content_height - lane) / 2.0;
        EdgeTriggers {
            left: Rect::new(rect.x + inset, mid, lane, lane),
            right: Rect::new(rect.right() - inset - lane, mid, lane, lane),
            top: Rect::new(
                rect.x + (rect.width_dp - lane) / 2.0,
                title_bottom + inset,
                lane,
                lane,
            ),
            bottom: Rect::new(
                rect.x + (rect.width_dp - lane) / 2.0,
                rect.bottom() - inset - lane,
                lane,
                lane,
            ),
        }
    }

    /// The four lanes as `(zone, rect)`, in the reference's declaration order.
    pub fn lanes(self) -> [(DropZone, Rect); 4] {
        [
            (DropZone::Left, self.left),
            (DropZone::Right, self.right),
            (DropZone::Top, self.top),
            (DropZone::Bottom, self.bottom),
        ]
    }

    /// The zone a press on a trigger names (the reference's `clicked` lambdas, 2874–2883), or
    /// `None` when the point is between the lanes.
    pub fn zone_at(self, at: (f64, f64)) -> Option<DropZone> {
        self.lanes()
            .into_iter()
            .find(|(_, lane)| lane.contains(at))
            .map(|(zone, _)| zone)
    }
}

/// **A seam drag in flight** — which splitter's boundary, the shares it started from, and where
/// the pointer was when it grabbed. The reference gets this from Qt's `QSplitter` handle
/// (`setChildrenCollapsible(False)` and `minimumSizeHint` do the clamping); here the drag owns
/// the start pair and each move reads it, the same rule [`PortalWindow::resize_from`] states for
/// a resize.
#[derive(Clone, Debug, PartialEq)]
pub struct SeamDrag {
    /// The window whose arrangement holds the splitter.
    pub id: String,
    /// Which splitter, in the layout's tree order (the same order `Arrangement::shares` walks).
    pub splitter: usize,
    /// Which boundary of that splitter: between child `boundary` and child `boundary + 1`.
    pub boundary: usize,
    /// The splitter's orientation: a horizontal split's seam moves along x, a vertical one's
    /// along y.
    pub orientation: Orientation,
    /// The splitter's extent along its own axis, in dp.
    pub span: f64,
    /// The two shares the boundary separated when the drag started.
    pub from: (f32, f32),
    /// Where the pointer grabbed, in the window's coordinates — the delta's origin.
    pub at: (f64, f64),
}

impl SeamDrag {
    /// A drag that grabbed the seam at `at` — the start pair read from the tree's own shares.
    pub fn start(
        id: impl Into<String>,
        seam: &splitter::Seam,
        from: (f32, f32),
        at: (f64, f64),
    ) -> Self {
        SeamDrag {
            id: id.into(),
            splitter: seam.splitter,
            boundary: seam.boundary,
            orientation: seam.orientation,
            span: seam.span,
            from,
            at,
        }
    }

    /// The pointer's total travel along the seam's axis, from where it grabbed.
    pub fn delta(&self, at: (f64, f64)) -> f64 {
        match self.orientation {
            Orientation::Horizontal => at.0 - self.at.0,
            Orientation::Vertical => at.1 - self.at.1,
        }
    }
}

/// What a press on a window means — the reference's three handlers folded into one verdict.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowPress {
    /// A title-bar control.
    Control(GlyphAction),
    /// Start a window drag (`start_window_drag`, 3165–3168).
    Drag,
    /// Start a resize from these edges (3326–3338).
    Resize(ResizeEdges),
    /// A double-click on the bar where the behaviour policy permits maximize (2506–2515).
    Maximize,
    /// A pane's own header control. **The pane is a [`PaneRef`]**, so an empty slot's controls
    /// answer their own split and close exactly as a window pane's do.
    PaneAction(PaneRef, PaneMark),
    /// A seam: start a share drag. **The seam carries its address in the tree** ([`Seam`]), so
    /// the caller resolves the splitter and boundary without a second walk.
    Seam(splitter::Seam),
    /// A pane's body: focus the window that holds it (2712–2718).
    Pane(PaneRef),
    /// A press on an **empty slot's** offer row: place the offered window there (the
    /// reference's `PanePlaceholder.appSelected`, 2622).
    Offer(PaneRef, usize),
    /// **An edge trigger**: split the window at the outer edge this zone names (the reference's
    /// `clicked` lambdas, 2874–2883, which call `self.split(orientation, position)`).
    Trigger(DropZone),
    /// Nothing this window answers.
    None,
}

/// **The value the drawing needs to draw, as a value.** `PortalWindow` owns no focus, no hover,
/// no drag and no registry, so the caller passes the frame's state: the reference's
/// `set_focus_visual` (3312–3320) is this struct's `focused` field, applied at draw time
/// instead of through a Qt property re-polish.
#[derive(Clone, Debug, Default)]
pub struct WindowView {
    /// The title the bar draws (the reference's `set_title`, 2476; the derived title is the
    /// caller's — `Mount::title_of` — because the titles live in the window registry).
    pub title: String,
    /// Whether this window holds focus (the outline brightens one ink step, the title dims one
    /// step when it does not).
    pub focused: bool,
    /// The pane mark under the pointer, if any — one elevation step of hover.
    pub hovered_pane_mark: Option<PaneMark>,
    /// The seam index under the pointer, if any.
    pub hovered_seam: Option<usize>,
    /// The drop preview this window is showing, when a drag is over it (the reference's
    /// `_last_drop_*` on the *dragged* window names the target; here the target holds it).
    pub drop: Option<DropTarget>,
    /// The trigger lanes to draw, from [`PortalWindow::edge_triggers`] — the caller resolves
    /// them from the pointer so the drawing needs no second source.
    pub triggers: Option<EdgeTriggers>,
    /// The trigger under the pointer, if any: one elevation step of hover.
    pub hovered_zone: Option<DropZone>,
    /// What an **empty slot's** placeholder offers — the package's windows
    /// ([`placeholder.rs`](crate::shell::placeholder): *"the grid lists this tree's windows"*).
    pub offers: Vec<(String, String)>,
}

/// The drag in flight, as the reference keeps it on the window it is moving (`dragging`,
/// `drag_offset`, `_last_drop_win` / `_last_drop_pane` / `_last_drop_zone`, 2801–2813).
///
/// **The offset is the whole state a move needs**: `origin(at)` is where the window goes for a
/// pointer at `at`, and the caller clamps it. `moved` is the reference's own distinction — a
/// click that never moved is a focus, not a dock attempt (`pointer_released`).
#[derive(Clone, Debug, PartialEq)]
pub struct DragState {
    /// The window being moved.
    pub id: String,
    /// Where in the window the pointer grabbed it, in dp from the window's origin.
    pub offset: (f64, f64),
    /// Whether the pointer has actually moved it.
    pub moved: bool,
    /// The window and target the preview is currently shown on, if any.
    pub preview: Option<(String, DropTarget)>,
}

impl DragState {
    /// A drag that started at `at`, grabbing the window whose origin is `origin`.
    pub fn start(id: impl Into<String>, at: (f64, f64), origin: (f64, f64)) -> Self {
        DragState {
            id: id.into(),
            offset: (at.0 - origin.0, at.1 - origin.1),
            moved: false,
            preview: None,
        }
    }

    /// Where the window's origin goes for a pointer at `at`.
    pub fn origin(&self, at: (f64, f64)) -> (f64, f64) {
        (at.0 - self.offset.0, at.1 - self.offset.1)
    }

    /// Record that the pointer moved, and answer where the window goes.
    pub fn moved_to(&mut self, at: (f64, f64)) -> (f64, f64) {
        self.moved = true;
        self.origin(at)
    }

    /// What the preview is showing now — the reference's `_last_drop_*` update, including its
    /// clearing when the pointer leaves every window (3240–3246).
    pub fn set_preview(&mut self, found: Option<(String, DropTarget)>) {
        self.preview = found;
    }
}

/// **Where a dragged window would land**, resolved against the windows under the cursor — the
/// reference's `continue_window_drag` body (3175–3246) as one pure function.
///
/// `candidates` is `(id, rect, title bar height, layout)` in draw order, **back to front**: the
/// reference iterates its desktop list and stops at the first window whose geometry holds the
/// cursor, and its list is back-to-front, so the last match is the topmost. This keeps the last
/// match, which is that rule stated once.
pub fn drop_target_at<'a>(
    candidates: impl IntoIterator<Item = (&'a str, Rect, f64, &'a Layout)>,
    at: (f64, f64),
) -> Option<(String, DropTarget)> {
    let mut found = None;
    for (id, rect, title_height, layout) in candidates {
        if let Some(target) = drop::zone_at(rect, title_height, layout, at) {
            found = Some((id.to_string(), target));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_package::recipe::{Content, ExtentUnit, Lane, LayoutPolicy};

    fn spec_with(layout: u16, behavior: u16) -> WindowSpec {
        WindowSpec {
            id: "host".to_string(),
            title: "window host".to_string(),
            content: Content::ALL[0],
            lane: Lane::ALL[0],
            extent_unit: ExtentUnit::ALL[0],
            geometry: [100, 80, 640, 406],
            min_size: [260, 160],
            opacity: 1000,
            elevation: 0,
            theme_role: "surface".to_string(),
            pinnable: false,
            layout_policy: LayoutPolicy(layout),
            behavior_policy: BehaviorPolicy(behavior),
            record_id: "C-1".to_string(),
        }
    }

    /// A declaration with every control and every behaviour, so a test reads only the bits it
    /// is about.
    fn spec(id: &str) -> WindowSpec {
        let mut spec = spec_with(
            LayoutPolicy::MINIMIZE.0
                | LayoutPolicy::MAXIMIZE.0
                | LayoutPolicy::CLOSE.0
                | LayoutPolicy::TITLE_POSITION.0,
            BehaviorPolicy::DRAG.0 | BehaviorPolicy::RESIZE.0 | BehaviorPolicy::MAXIMIZE.0,
        );
        spec.id = id.to_string();
        spec.title = format!("window {id}");
        spec
    }

    fn window(id: &str) -> PortalWindow {
        PortalWindow::new(spec(id), Rect::new(100.0, 80.0, 640.0, 406.0)).expect("a window")
    }

    fn id(name: &str) -> WindowId {
        WindowId::new(name).expect("a window id")
    }

    fn view(title: &str) -> WindowView {
        WindowView {
            title: title.to_string(),
            ..Default::default()
        }
    }

    /// A view offering one window, for the empty-slot tests.
    fn offering_view(title: &str, offers: usize) -> WindowView {
        let mut view = view(title);
        view.offers = (0..offers)
            .map(|index| (format!("w{index}"), format!("Window {index}")))
            .collect();
        view
    }

    fn titles(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(id, title)| (id.to_string(), title.to_string()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Lifecycle (the first landing)
    // -----------------------------------------------------------------------

    #[test]
    fn a_new_window_is_one_pane_holding_itself() {
        let w = window("host");
        assert_eq!(w.pane_ids().len(), 1);
        assert!(w.pane_ids().contains(&id("host")));
        assert!(w.is_standalone());
        assert_eq!(w.status, Status::Normal);
    }

    #[test]
    fn a_window_that_is_one_pane_draws_its_title_once() {
        // **The regression guard for the doubled title.** The title bar names the window; a
        // pane header on top of it named the same window again, 40 dp lower.
        let w = window("host");
        assert!(!w.draws_pane_headers());
        let mut out = Vec::new();
        w.elements(
            &view("Inventory"),
            &Palette::declared(),
            &titles(&[("host", "Inventory")]),
            &mut out,
        )
        .expect("draws");
        let titles_drawn = out
            .iter()
            .filter(|element| matches!(element, Element::Text { text, .. } if text == "Inventory"))
            .count();
        assert_eq!(titles_drawn, 1, "the window's name is drawn once");
    }

    #[test]
    fn a_split_window_draws_a_header_per_pane() {
        let mut w = window("host");
        w.split(
            Some(&id("host")),
            Orientation::Horizontal,
            Side::After,
            id("other"),
        )
        .expect("splits");
        assert!(w.draws_pane_headers(), "two panes need two headers");
        let mut out = Vec::new();
        w.elements(
            &view("host and other"),
            &Palette::declared(),
            &titles(&[("host", "Host"), ("other", "Other")]),
            &mut out,
        )
        .expect("draws");
        for pane in ["Host", "Other"] {
            assert!(
                out.iter()
                    .any(|element| matches!(element, Element::Text { text, .. } if text == pane)),
                "each pane is named by its own header"
            );
        }
    }

    #[test]
    fn the_row_a_header_would_take_is_body_in_a_single_pane_window() {
        // The pane's top row drew split and close controls; it is not drawn, so it must not be
        // hit-tested either.
        let w = window("host");
        let top_of_content = w.content().y + 1.0;
        assert_eq!(
            w.press(
                &view("Inventory"),
                (w.content().x + 20.0, top_of_content),
                false
            ),
            WindowPress::Pane(PaneRef::Window(id("host")))
        );
    }

    #[test]
    fn minimize_and_restore_move_the_state_and_not_the_rect() {
        let mut w = window("host");
        let before = w.rect;
        w.minimize();
        assert_eq!(w.status, Status::Minimized);
        assert!(!w.is_drawn());
        w.restore();
        assert_eq!(w.status, Status::Normal);
        assert_eq!(w.rect, before);
        assert!(w.is_drawn());
    }

    #[test]
    fn maximize_saves_the_previous_rect_and_restore_returns_to_it() {
        let mut w = window("host");
        let before = w.rect;
        let desktop = Rect::new(0.0, 0.0, 1920.0, 1080.0);
        w.toggle_maximize(desktop, 48.0).expect("maximizes");
        assert_eq!(w.status, Status::Maximized);
        assert_eq!(w.previous, Some(before));
        assert_eq!(w.rect.x, spacing_dp());
        assert_eq!(w.rect.y, spacing_dp());
        assert_eq!(w.rect.width_dp, 1920.0 - 2.0 * spacing_dp());
        assert_eq!(w.rect.height_dp, 1080.0 - 48.0 - 2.0 * spacing_dp());
        w.toggle_maximize(desktop, 48.0).expect("restores");
        assert_eq!(w.status, Status::Normal);
        assert_eq!(w.rect, before, "restore goes back where the window was");
        assert_eq!(w.previous, None);
    }

    #[test]
    fn a_maximized_rect_never_goes_below_the_declared_minimum_size() {
        let mut w =
            PortalWindow::new(spec("tiny"), Rect::new(0.0, 0.0, 300.0, 200.0)).expect("a window");
        let desktop = Rect::new(0.0, 0.0, 100.0, 100.0);
        w.toggle_maximize(desktop, 48.0).expect("maximizes");
        assert_eq!(w.rect.width_dp, 260.0);
        assert_eq!(w.rect.height_dp, 160.0);
    }

    #[test]
    fn closing_a_guest_keeps_the_window_and_closing_the_last_pane_does_not() {
        let mut w = window("host");
        w.split(None, Orientation::Horizontal, Side::After, id("guest"))
            .expect("splits");
        assert!(w.close_pane(&PaneRef::Window(id("guest"))));
        assert_eq!(w.status, Status::Normal);
        assert_eq!(w.pane_ids().len(), 1);
        assert!(!w.close_pane(&PaneRef::Window(id("host"))));
        assert_eq!(w.status, Status::Closed);
        assert!(!w.is_drawn());
    }

    #[test]
    fn undocking_a_pane_frees_it_and_closes_an_emptied_window() {
        let mut w = window("host");
        w.split(None, Orientation::Horizontal, Side::After, id("guest"))
            .expect("splits");
        assert_eq!(w.undock(&id("guest")), Some(id("guest")));
        assert_eq!(w.status, Status::Normal);
        assert_eq!(w.pane_ids().len(), 1);
        // Undocking the host's own pane leaves nothing.
        assert_eq!(w.undock(&id("host")), Some(id("host")));
        assert_eq!(w.status, Status::Closed);
    }

    #[test]
    fn docking_keeps_the_input_order_on_both_sides() {
        let mut w = window("host");
        w.dock(&[id("a"), id("b")], None, DropZone::Right)
            .expect("docks");
        let layout = w.layout().expect("lays out");
        let order: Vec<&str> = layout
            .panes
            .iter()
            .map(|(_, pane)| pane.window().expect("a window pane").as_str())
            .collect();
        assert_eq!(order, vec!["host", "a", "b"], "after keeps the input order");

        let mut w = window("host");
        w.dock(&[id("a"), id("b")], None, DropZone::Left)
            .expect("docks");
        let layout = w.layout().expect("lays out");
        let order: Vec<&str> = layout
            .panes
            .iter()
            .map(|(_, pane)| pane.window().expect("a window pane").as_str())
            .collect();
        assert_eq!(
            order,
            vec!["a", "b", "host"],
            "before keeps the input order too"
        );
    }

    #[test]
    fn docking_into_a_pane_target_splits_that_pane_and_its_zone_decides_the_axis() {
        let mut w = window("host");
        w.split(None, Orientation::Horizontal, Side::After, id("guest"))
            .expect("splits");
        w.dock(
            &[id("extra")],
            Some(&PaneRef::Window(id("guest"))),
            DropZone::Bottom,
        )
        .expect("docks");
        let layout = w.layout().expect("lays out");
        let rect_of = |name: &str| {
            layout
                .window_rect(&id(name))
                .unwrap_or_else(|| panic!("{name} is in the tree"))
        };
        // `guest` and `extra` stack vertically, in guest's old column.
        assert_eq!(rect_of("extra").x, rect_of("guest").x);
        assert_eq!(
            rect_of("extra").y,
            rect_of("guest").y + rect_of("guest").height_dp
        );
    }

    // -----------------------------------------------------------------------
    // The composite
    // -----------------------------------------------------------------------

    #[test]
    fn the_composite_is_the_border_then_the_bar_then_the_content() {
        let w = window("host");
        let composite = composite(&w.spec, w.rect, w.pinned);
        let bar = composite.title.expect("the declaration permits a bar");
        assert_eq!(bar.x, w.rect.x + BORDER_DP);
        assert_eq!(bar.y, w.rect.y + BORDER_DP);
        assert_eq!(bar.width_dp, w.rect.width_dp - 2.0 * BORDER_DP);
        assert_eq!(bar.height_dp, titlebar::bar_height_dp());
        assert_eq!(composite.content.y, bar.bottom());
        assert_eq!(composite.content.x, bar.x);
        assert_eq!(
            composite.content.height_dp,
            w.rect.height_dp - 2.0 * BORDER_DP - bar.height_dp
        );
    }

    #[test]
    fn a_window_without_title_position_has_no_bar_and_the_content_is_the_inner_rect() {
        let w = PortalWindow::new(
            spec_with(LayoutPolicy::CLOSE.0, BehaviorPolicy::DRAG.0),
            Rect::new(100.0, 80.0, 640.0, 406.0),
        )
        .expect("a window");
        let composite = composite(&w.spec, w.rect, w.pinned);
        assert!(composite.title.is_none());
        assert_eq!(composite.content.y, w.rect.y + BORDER_DP);
        assert_eq!(
            composite.content.height_dp,
            w.rect.height_dp - 2.0 * BORDER_DP
        );
        assert_eq!(w.title_height(), 0.0);
    }

    #[test]
    fn the_drawing_and_the_hit_tests_resolve_the_same_pane_rects() {
        let mut w = window("host");
        w.split(None, Orientation::Horizontal, Side::After, id("guest"))
            .expect("splits");
        let palette = Palette::declared();
        let mut out = Vec::new();
        let layout = w
            .elements(
                &view("host"),
                &palette,
                &titles(&[("host", "host"), ("guest", "guest")]),
                &mut out,
            )
            .expect("draws");
        // The close control of `guest`, at the rect the drawing used — the press must answer
        // the same pane the layout drew.
        let guest = layout.window_rect(&id("guest")).expect("guest is laid out");
        let (mark, lane) = pane::action_lanes(guest, true)[3];
        assert_eq!(mark, PaneMark::Close);
        let at = (lane.x + lane.width_dp / 2.0, lane.y + lane.height_dp / 2.0);
        assert_eq!(
            w.press(&view("host"), at, false),
            WindowPress::PaneAction(PaneRef::Window(id("guest")), mark)
        );
    }

    #[test]
    fn the_focused_windows_outline_is_one_ink_step_brighter() {
        let w = window("host");
        let palette = Palette::declared();
        let draw = |focused: bool| {
            let mut out = Vec::new();
            w.elements(
                &WindowView {
                    title: "host".to_string(),
                    focused,
                    ..Default::default()
                },
                &palette,
                &titles(&[("host", "host")]),
                &mut out,
            )
            .expect("draws");
            out
        };
        let outline = |out: &[Element]| match &out[1] {
            Element::Rule { paint, .. } => paint.alpha,
            other => panic!("the outline is the second element, found {other:?}"),
        };
        assert_eq!(outline(&draw(true)), Ink::Secondary.opacity());
        assert_eq!(outline(&draw(false)), Ink::Divider.opacity());
    }

    #[test]
    fn an_empty_slot_draws_the_placeholder_and_says_so() {
        let mut w = window("host");
        // **The split is what makes a slot**, and the tree — not the title map — says so: the
        // host's own pane is titled and draws its title, the new pane is empty and draws the
        // reference's `◫  New Split Slot` header and placeholder (2653–2718).
        w.split_slot(None, Orientation::Horizontal, Side::After)
            .expect("splits");
        let palette = Palette::declared();
        let mut out = Vec::new();
        w.elements(
            &view("host"),
            &palette,
            &titles(&[("host", "host")]),
            &mut out,
        )
        .expect("draws");
        let empty_title = pane::empty_pane_title();
        let texts: Vec<&str> = out.iter().filter_map(|element| element.text()).collect();
        assert!(texts.contains(&empty_title.as_str()), "found {texts:?}");
        assert!(texts.iter().any(|text| text.contains("place here")));
    }

    #[test]
    fn the_drop_preview_is_drawn_by_the_window_the_drop_lands_in() {
        let mut w = window("host");
        w.split(None, Orientation::Horizontal, Side::After, id("guest"))
            .expect("splits");
        let palette = Palette::declared();
        let known = titles(&[("host", "host"), ("guest", "guest")]);
        let mut plain = Vec::new();
        w.elements(&view("host"), &palette, &known, &mut plain)
            .expect("draws");
        let mut previewed = Vec::new();
        let shown = WindowView {
            drop: Some(DropTarget {
                zone: DropZone::Left,
                pane: Some(PaneRef::Window(id("guest"))),
            }),
            ..view("host")
        };
        w.elements(&shown, &palette, &known, &mut previewed)
            .expect("draws");
        // The wash, its dashed border, and the label on its pill.
        assert!(previewed.len() > plain.len() + 12, "the preview is drawn");
        // A preview naming a pane this layout does not hold draws nothing rather than a
        // window-edge preview that would show where the drop would not land.
        let mut stale = Vec::new();
        let shown = WindowView {
            drop: Some(DropTarget {
                zone: DropZone::Left,
                pane: Some(PaneRef::Window(id("zz"))),
            }),
            ..view("host")
        };
        w.elements(&shown, &palette, &known, &mut stale)
            .expect("draws");
        assert_eq!(stale.len(), plain.len());
    }

    // -----------------------------------------------------------------------
    // The mouse machine
    // -----------------------------------------------------------------------

    #[test]
    fn a_press_on_an_empty_slot_s_offer_answers_the_offer_and_the_body_focuses_it() {
        let mut w = window("host");
        // **A slot is the pane a split made**: the offer grid belongs to it, addressed by the
        // path the layout stamped — not to the host's own pane.
        let slot = w
            .split_slot(None, Orientation::Horizontal, Side::After)
            .expect("splits");
        let shown = offering_view("host", 2);
        let layout = w.layout().expect("lays out");
        let pane_rect = layout.pane_rect(&slot).expect("the slot is laid out");
        let body = pane::body(pane_rect);
        let lane = placeholder::offer_lanes(placeholder::frame(body), 2)[1];
        let at = (lane.x + 2.0, lane.y + 2.0);
        assert_eq!(
            w.press(&shown, at, false),
            WindowPress::Offer(slot.clone(), 1)
        );
        // The note, between the mark and the offers: the body, which focuses.
        let note = placeholder::note_rect(placeholder::frame(body));
        let at = (note.x + 2.0, note.y + 2.0);
        assert_eq!(w.press(&shown, at, false), WindowPress::Pane(slot));
    }

    #[test]
    fn an_edge_trigger_press_is_the_zone_the_lane_shows() {
        let w = window("host");
        let content = composite(&w.spec, w.rect, w.pinned).content;
        // The left lane, at the content's mid-height: a press inside it is the trigger's, and
        // it wins over the pane body underneath it (the reference `raise_`s the buttons,
        // 3280–3281).
        let triggers = w.edge_triggers((w.rect.x + 20.0, content.y + content.height_dp / 2.0));
        let lane = triggers.expect("the lane shows").left;
        let at = (lane.x + lane.width_dp / 2.0, lane.y + lane.height_dp / 2.0);
        assert_eq!(
            w.press(&view("host"), at, false),
            WindowPress::Trigger(DropZone::Left)
        );
        // Away from the lanes the same pane answers its body.
        let at = (w.rect.x + w.rect.width_dp / 2.0, content.y + 200.0);
        assert_eq!(
            w.press(&view("host"), at, false),
            WindowPress::Pane(PaneRef::Window(id("host")))
        );
    }

    #[test]
    fn a_press_on_the_resize_strip_resizes_and_a_press_on_the_bar_drags() {
        let w = window("host");
        let strip = (w.rect.x + 4.0, w.rect.y + 200.0);
        assert_eq!(
            w.press(&view("host"), strip, false),
            WindowPress::Resize(ResizeEdges {
                west: true,
                ..Default::default()
            })
        );
        let bar = (w.rect.x + 100.0, w.rect.y + 10.0);
        assert_eq!(w.press(&view("host"), bar, false), WindowPress::Drag);
        // Without the resize permit the same strip is not a resize.
        let still = PortalWindow::new(
            spec_with(
                LayoutPolicy::TITLE_POSITION.0 | LayoutPolicy::CLOSE.0,
                BehaviorPolicy::DRAG.0,
            ),
            w.rect,
        )
        .expect("a window");
        assert!(matches!(
            still.press(&view("host"), strip, false),
            WindowPress::Pane(_)
        ));
    }

    #[test]
    fn a_double_click_asks_the_bar_first_and_answers_maximize() {
        let w = window("host");
        let bar = (w.rect.x + 100.0, w.rect.y + 10.0);
        assert_eq!(w.press(&view("host"), bar, true), WindowPress::Maximize);
        // Even on the top strip, where a single press resizes: one event, one verdict.
        let strip = (w.rect.x + 100.0, w.rect.y + 4.0);
        assert_eq!(w.press(&view("host"), strip, true), WindowPress::Maximize);
        // Where the policy forbids maximizing, the double-click answers nothing.
        let may_not = PortalWindow::new(
            spec_with(
                LayoutPolicy::TITLE_POSITION.0 | LayoutPolicy::CLOSE.0,
                BehaviorPolicy::DRAG.0,
            ),
            w.rect,
        )
        .expect("a window");
        assert_eq!(may_not.press(&view("host"), bar, true), WindowPress::None);
    }

    #[test]
    fn the_resize_arithmetic_grows_from_the_far_edge_and_stops_at_the_floor() {
        let w = window("host");
        let start = w.rect;
        let west = ResizeEdges {
            west: true,
            ..Default::default()
        };
        let grown = w.resize_from(start, west, -50.0, 0.0);
        assert_eq!(grown.width_dp, start.width_dp + 50.0);
        assert_eq!(grown.x, start.x - 50.0, "the far edge stays put");
        let floored = w.resize_from(start, west, 500.0, 0.0);
        assert_eq!(floored.width_dp, 260.0);
        assert_eq!(floored.x, start.x + start.width_dp - 260.0);
        let north = ResizeEdges {
            north: true,
            ..Default::default()
        };
        let floored = w.resize_from(start, north, 0.0, 500.0);
        assert_eq!(floored.height_dp, 160.0);
        assert_eq!(floored.y, start.y + start.height_dp - 160.0);
        let east_south = ResizeEdges {
            east: true,
            south: true,
            ..Default::default()
        };
        let grown = w.resize_from(start, east_south, 30.0, 40.0);
        assert_eq!(grown.width_dp, start.width_dp + 30.0);
        assert_eq!(grown.height_dp, start.height_dp + 40.0);
        assert_eq!((grown.x, grown.y), (start.x, start.y));
    }

    #[test]
    fn the_resize_strip_answers_one_edge_per_axis_and_its_cursor() {
        let rect = Rect::new(100.0, 80.0, 640.0, 406.0);
        let west = ResizeEdges::at(rect, (102.0, 200.0));
        assert_eq!(
            west,
            ResizeEdges {
                west: true,
                ..Default::default()
            }
        );
        assert_eq!(west.cursor(), Some(ResizeCursor::EastWest));
        let north_west = ResizeEdges::at(rect, (102.0, 82.0));
        assert_eq!(
            north_west,
            ResizeEdges {
                west: true,
                north: true,
                ..Default::default()
            }
        );
        assert_eq!(north_west.cursor(), Some(ResizeCursor::ForwardDiagonal));
        let south_east = ResizeEdges::at(rect, (738.0, 484.0));
        assert_eq!(south_east.cursor(), Some(ResizeCursor::ForwardDiagonal));
        let north_east = ResizeEdges::at(rect, (738.0, 82.0));
        assert_eq!(north_east.cursor(), Some(ResizeCursor::BackwardDiagonal));
        let north = ResizeEdges::at(rect, (400.0, 82.0));
        assert_eq!(north.cursor(), Some(ResizeCursor::NorthSouth));
        let middle = ResizeEdges::at(rect, (400.0, 200.0));
        assert!(!middle.any());
        assert_eq!(middle.cursor(), None);
        // A window narrower than twice the margin is on its west edge, never both — the
        // reference's `elif`.
        let narrow = ResizeEdges::at(Rect::new(0.0, 0.0, 10.0, 400.0), (5.0, 200.0));
        assert!(narrow.west && !narrow.east);
    }

    #[test]
    fn the_edge_triggers_appear_inside_the_band_and_answer_their_zone() {
        let w = window("host");
        let at = (w.rect.x + 5.0, w.rect.y + 200.0);
        let triggers = w.edge_triggers(at).expect("near the west edge");
        assert_eq!(
            triggers.zone_at((triggers.left.x + 20.0, triggers.left.y + 20.0)),
            Some(DropZone::Left)
        );
        // Every lane is a pointer target inside the window.
        assert_eq!(EdgeTriggers::band_dp(), spacing_dp() + pointer_target_dp());
        for (_, lane) in triggers.lanes() {
            assert_eq!(lane.width_dp, pointer_target_dp());
            assert_eq!(lane.height_dp, pointer_target_dp());
            assert!(lane.x >= w.rect.x && lane.right() <= w.rect.right());
            assert!(lane.y >= w.rect.y && lane.bottom() <= w.rect.bottom());
        }
        // The top lane starts below the title bar, as the reference measures it.
        let bar = w.title_slot().expect("a bar");
        assert_eq!(triggers.top.y, bar.bottom() + spacing_dp());
        // Away from every edge, no triggers.
        assert!(w
            .edge_triggers((w.rect.x + 200.0, w.rect.y + 200.0))
            .is_none());
    }

    #[test]
    fn the_trigger_lanes_hold_the_rect_the_window_is_drawn_at() {
        // The maximized case: the drawn rect is the desktop's arithmetic and `self.rect` is
        // where the window will restore to, so the hover test has to take the drawn one.
        let w = window("host");
        let drawn = Rect::new(0.0, 40.0, 1280.0, 680.0);
        // Inside the left lane: the lane sits at the **content's** mid-height, and the content
        // starts below the 1 dp border and the 40 dp bar — the reference's own arithmetic.
        let near_left = (drawn.x + 20.0, drawn.y + 350.0);
        let triggers = w
            .edge_triggers_at(drawn, near_left)
            .expect("near the drawn window's left edge");
        assert!(
            triggers.left.contains(near_left),
            "the lane is at the drawn rect"
        );
        let bar = w.title_slot_at(drawn).expect("a bar");
        assert_eq!(triggers.top.y, bar.bottom() + spacing_dp());
        // Away from every edge of the drawn rect: none.
        assert!(w.edge_triggers_at(drawn, (400.0, 400.0)).is_none());
    }

    #[test]
    fn a_drag_over_two_windows_targets_the_topmost_and_the_pane_under_the_cursor() {
        let a_rect = Rect::new(0.0, 0.0, 600.0, 400.0);
        let b_rect = Rect::new(100.0, 50.0, 600.0, 400.0);
        let a_layout = splitter::layout(&Arrangement::of(id("a")), a_rect).expect("lays out");
        let b_layout = splitter::layout(&Arrangement::of(id("b")), b_rect).expect("lays out");
        let candidates = [
            ("a", a_rect, 40.0, &a_layout),
            ("b", b_rect, 40.0, &b_layout),
        ];
        // Inside both: the last candidate (the topmost) wins.
        let (found, target) = drop_target_at(candidates, (300.0, 250.0)).expect("a target");
        assert_eq!(found, "b");
        assert_eq!(target.pane, Some(PaneRef::Window(id("b"))));
        assert_eq!(target.zone, DropZone::Left);
        // A point only `a` covers answers `a`.
        let (found, target) = drop_target_at(candidates, (50.0, 350.0)).expect("a target");
        assert_eq!(found, "a");
        assert_eq!(target.pane, Some(PaneRef::Window(id("a"))));
    }

    #[test]
    fn a_drag_keeps_the_offset_and_the_preview_it_is_showing() {
        let mut drag = DragState::start("host", (120.0, 90.0), (100.0, 80.0));
        assert_eq!(drag.offset, (20.0, 10.0));
        assert!(!drag.moved);
        assert_eq!(drag.origin((300.0, 200.0)), (280.0, 190.0));
        assert_eq!(drag.moved_to((300.0, 200.0)), (280.0, 190.0));
        assert!(drag.moved);
        drag.set_preview(Some((
            "other".to_string(),
            DropTarget {
                zone: DropZone::Left,
                pane: Some(PaneRef::Window(id("other"))),
            },
        )));
        assert_eq!(
            drag.preview.as_ref().map(|(id, _)| id.as_str()),
            Some("other")
        );
        drag.set_preview(None);
        assert!(drag.preview.is_none());
    }

    #[test]
    fn a_pane_split_control_carries_the_reference_s_own_axis() {
        assert_eq!(
            split_axis(PaneMark::SplitHorizontal),
            Some((Orientation::Horizontal, Side::After))
        );
        assert_eq!(
            split_axis(PaneMark::SplitVertical),
            Some((Orientation::Vertical, Side::After))
        );
        assert_eq!(split_axis(PaneMark::Undock), None);
        assert_eq!(split_axis(PaneMark::Close), None);
    }
}
