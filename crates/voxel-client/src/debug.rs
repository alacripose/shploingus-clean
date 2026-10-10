//! Debug lanes: the client's own test surface, drawn on the thing it tests.
//!
//! **There is no Tests tab and no separate test surface** (event-0013, H-7.25): *a test is visible
//! by looking at the thing it tests*, so the lanes on every window **are** the test surface. A lane
//! is a **declared, validated region** that says what is being exercised there, and it draws as a
//! labelled outline at **its own position** — a legend separates the name from the thing it names,
//! which is the defect the ruling rules out.
//!
//! # What the record rules, and where each rule lives here
//!
//! * **H-7.5 — a view is a data record.** What it selects, which stages it reads, its labels and its
//!   lane class. One generic renderer draws any declared view; adding a view is data, never Rust.
//! * **H-7.7 — the lane system is ported per the new requirements.** The classes, the **frozen
//!   vocabulary and its digest**, and the labels at the lane's own position. The *colours* are
//!   adapted here rather than lifted.
//! * **H-7.3 — the cuboid is one instrument with three uses**: player interaction, the scope a
//!   debug-lane view reads, and the selection the shapers apply to. One type, three callers.
//! * **H-7.1 / H-7.9 — the vector change view** is transient, outlines what actually changed, works
//!   **with the shell closed**, and carries the terrain shaper views.
//!
//! # It refuses rather than clamps
//!
//! Borrowed from the ancestor rule: *a debug overlay that silently drew off-screen, or a selection
//! that silently shrank, would both be lies about what was selected.* A lane that clamped its
//! bounds would draw a box somewhere other than where the thing under test is, and would then be
//! reporting a green light for a region it is not covering. **A player's** dragged rectangle is a
//! rectangle they meant to put there; **a lane's** declared rectangle is a claim about the client,
//! and a claim that no longer matches the client is not a claim.
//!
//! # The class set is open, and the hue is the class's
//!
//! The class vocabulary here is **not six entries frozen forever** — *more views do not mean more
//! hues*, and a class is a kind of claim, not a colour. Classes are added as views arrive, and the
//! count is **measured by a test** rather than asserted as a number.
//!
//! **Every class declares a hue, and a lane draws in its class's hue.** The user's ruling,
//! 2026-10-03: *"the hues per lane was nice."* The five values are **adapted from the ancestor's
//! six-hue table** (the `shploingus-game` tree's `client/debug.rs` — Frame amber, Control violet,
//! Tab blue, World green, Beacon warm white, Motion red) onto this tree's own classes: Structure ←
//! Frame, Affordance ← Control, World ← World, Change ← Motion, State ← Beacon. **The ancestor's
//! class semantics are not carried** — its Beacon (*"the one voxel the simulation moves each
//! tick"*) and Motion were invented there and are not this tree's (the shapers are not one voxel
//! per tick); only the colour values anchor to that table. The ancestor's Tab blue is not carried
//! either: this tree folds the strip cells into `Affordance` (`shell::debug`'s `bar-cell`), so a
//! sixth hue would be a hue for a class nothing is in — *more views do not mean more hues*.
//!
//! A lane is still measured **where it lands**: the class hue is contrast-measured over the
//! surface the lane covers, because ink is measured where it lands and a lane that cannot be read
//! is not a test. That is the one place the adapted values are checked; the hue is the class's
//! declaration, not a per-call-site choice.

use voxel_model::Digest;
use voxel_ui::{Align, Element, Paint, Rect};

/// The ink step a lane is drawn in over `backdrop`, or `None` when **no** step clears the floor.
///
/// The unit a lane's volume is measured in.
///
/// **Two overflows in different units are different defects**, so the unit is part of the lane's
/// identity and part of every refusal about it: a window is measured in square device-independent
/// pixels, the world in cubic world units.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum LaneUnit {
    /// A region on the screen, in device-independent pixels.
    Pixels,
    /// A region in the world, in world units (metres).
    WorldUnits,
}

impl LaneUnit {
    pub const ALL: [LaneUnit; 2] = [LaneUnit::Pixels, LaneUnit::WorldUnits];

    /// The unit's own name, for a refusal and for a label.
    pub const fn name(self) -> &'static str {
        match self {
            LaneUnit::Pixels => "square dp",
            LaneUnit::WorldUnits => "cubic world units",
        }
    }
}

/// The kind of claim a lane makes.
///
/// **This is an open vocabulary, not a frozen list of hues.** A class says *what kind of thing is
/// being claimed here*; the hue that carries it is the class's own ([`LaneClass::hue`]), adapted
/// rather than open — *more views do not mean more hues*.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum LaneClass {
    /// The shell's own structure: the bar, a window's title bar, its body, a pane.
    Structure,
    /// An affordance: a control a player can press, a bar entry, a menu row.
    Affordance,
    /// The world's own structure: the extents, a chunk, a selection.
    World,
    /// What a shaper or an edit changed, and where.
    Change,
    /// Where the world's state is being read from: streaming, persistence, a clock.
    State,
}

impl LaneClass {
    pub const ALL: [LaneClass; 5] = [
        LaneClass::Structure,
        LaneClass::Affordance,
        LaneClass::World,
        LaneClass::Change,
        LaneClass::State,
    ];

    /// The class's stable slug, so a report and a digest can name it.
    pub const fn slug(self) -> &'static str {
        match self {
            LaneClass::Structure => "structure",
            LaneClass::Affordance => "affordance",
            LaneClass::World => "world",
            LaneClass::Change => "change",
            LaneClass::State => "state",
        }
    }

    /// The class's short label, drawn **at the lane's own position**.
    pub const fn label(self) -> &'static str {
        match self {
            LaneClass::Structure => "structure",
            LaneClass::Affordance => "affordance",
            LaneClass::World => "world",
            LaneClass::Change => "change",
            LaneClass::State => "state",
        }
    }

    /// **The hue this class is drawn in.**
    ///
    /// The user's ruling, 2026-10-03 — *"the hues per lane was nice"* — and the values are adapted
    /// from the ancestor's six-hue table (see the module docs). Brightness is the anchor: the
    /// ancestor's rationale is that its six read *"against the shell's own dark panels and against
    /// a voxel of any colour in the terrain palette"*, which is exactly what a mark over the world
    /// needs. `Change` is the reddest of the five because a change is the mark a reader most needs
    /// to pick out of the picture — the vector change view draws in it.
    pub const fn hue(self) -> voxel_ui::Rgb {
        match self {
            // Frame amber: [1.00, 0.74, 0.16].
            LaneClass::Structure => voxel_ui::Rgb::new(255, 189, 41),
            // Control violet: [0.72, 0.44, 1.00].
            LaneClass::Affordance => voxel_ui::Rgb::new(184, 112, 255),
            // World green: [0.30, 0.95, 0.52].
            LaneClass::World => voxel_ui::Rgb::new(77, 242, 133),
            // Motion red: [1.00, 0.32, 0.36].
            LaneClass::Change => voxel_ui::Rgb::new(255, 82, 92),
            // Beacon warm white: [1.00, 0.97, 0.82].
            LaneClass::State => voxel_ui::Rgb::new(255, 247, 209),
        }
    }
}

/// Why a lane was refused.
///
/// **Every variant names both numbers where both exist**, because the whole point of the §5.7 rule
/// is that a refusal carries the two numbers rather than a verdict.
#[derive(Clone, PartialEq, Debug)]
pub enum LaneRefusal {
    /// The lane's region leaves the surface it was declared against.
    ///
    /// **Not clamped.** An overlay that drew inside the screen anyway would be a box somewhere other
    /// than the thing under test.
    OffSurface { lane: String, region: String, surface: String },
    /// The region is not a region: inverted bounds, or a non-finite number.
    NotARegion { lane: String, region: String },
    /// The set of lanes is larger than the budget declared for its unit.
    OverBudget { unit: LaneUnit, lanes: usize, budget: usize },
    /// A lane with no label. A lane that says nothing is not a test surface.
    Unlabelled { lane: String },
}

impl std::fmt::Display for LaneRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaneRefusal::OffSurface { lane, region, surface } => write!(
                f,
                "lane {lane} declares {region}, which is not inside {surface}; a lane that clamped \
                 its bounds would draw a box somewhere other than the thing it claims to cover"
            ),
            LaneRefusal::NotARegion { lane, region } => {
                write!(f, "lane {lane} declares {region}, which is not a region")
            }
            LaneRefusal::OverBudget { unit, lanes, budget } => write!(
                f,
                "{} lanes is over the {} budget of {budget}; the budget is declared, not discovered",
                lanes,
                unit.name()
            ),
            LaneRefusal::Unlabelled { lane } => {
                write!(f, "lane {lane} has no label, and an unlabelled lane says nothing")
            }
        }
    }
}

/// How many lanes the surface will hold, **per unit**.
///
/// **Declared, not discovered** — the same rule the agent-side selection budget follows. A budget
/// that is measured from whatever the frame happened to produce is a budget that moves when the
/// world does.
pub const LANE_BUDGET_PIXELS: usize = 256;
/// World lanes are bounded far lower than screen lanes: the world is unbounded and the screen is
/// not, so an unbounded world lane set is a performance claim nobody made.
pub const LANE_BUDGET_WORLD_UNITS: usize = 64;

/// The budget declared for one unit.
pub const fn lane_budget(unit: LaneUnit) -> usize {
    match unit {
        LaneUnit::Pixels => LANE_BUDGET_PIXELS,
        LaneUnit::WorldUnits => LANE_BUDGET_WORLD_UNITS,
    }
}

/// One declared region, in one unit.
///
/// **A lane is data.** H-7.5: what it selects, which stages it reads, its label and its class —
/// and *adding a view is data, never Rust*.
#[derive(Clone, PartialEq, Debug)]
pub struct Lane {
    /// The lane's stable identifier, unique within a set.
    pub id: String,
    pub class: LaneClass,
    /// Drawn at the lane's own position. **Never in a legend.**
    pub label: String,
    pub unit: LaneUnit,
    /// The region, in `unit`.
    pub region: Region,
}

/// Where a lane is. **One type for both units**, so "a lane is a declared region" is one statement.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Region {
    /// A rectangle in device-independent pixels.
    Rect(Rect),
    /// An axis-aligned box in world units, as `(minimum, maximum)`.
    Cuboid([f64; 3], [f64; 3]),
}

impl Region {
    /// Whether the region lies entirely inside the surface it was declared against.
    ///
    /// **No `clamp_inside` here**, and the difference is the module's whole argument: a rectangle a
    /// *player* dragged to the edge of a screen is one they meant to put there, while a rectangle a
    /// *lane* declared and the overlay then quietly shrank is a claim about the client that is no
    /// longer true.
    pub fn inside(&self, surface: &Region) -> bool {
        match (self, surface) {
            (Region::Rect(a), Region::Rect(b)) => {
                a.x >= b.x && a.y >= b.y && a.right() <= b.right() && a.bottom() <= b.bottom()
            }
            (Region::Cuboid(lo, hi), Region::Cuboid(slo, shi)) => {
                (0..3).all(|i| lo[i] >= slo[i] && hi[i] <= shi[i])
            }
            // **Different units are not comparable, and saying so beats clamping one into the
            // other.** A pixel lane is never inside a world lane.
            _ => false,
        }
    }

    /// A finite, non-empty description of the region, for a refusal.
    pub fn describe(&self) -> String {
        match self {
            Region::Rect(r) => {
                format!("{}×{} dp at ({}, {})", r.width_dp, r.height_dp, r.x, r.y)
            }
            Region::Cuboid(lo, hi) => {
                let span: Vec<String> = (0..3).map(|i| format!("{}", hi[i] - lo[i])).collect();
                format!("{} cubic world units", span.join("×"))
            }
        }
    }
}

impl Lane {
    /// A lane over a rectangle in dp.
    pub fn pixels(
        id: impl Into<String>,
        class: LaneClass,
        label: impl Into<String>,
        region: Rect,
    ) -> Lane {
        Lane {
            id: id.into(),
            class,
            label: label.into(),
            unit: LaneUnit::Pixels,
            region: Region::Rect(region),
        }
    }

    /// A lane over a box in world units.
    pub fn world(
        id: impl Into<String>,
        class: LaneClass,
        label: impl Into<String>,
        minimum: [f64; 3],
        maximum: [f64; 3],
    ) -> Lane {
        Lane {
            id: id.into(),
            class,
            label: label.into(),
            unit: LaneUnit::WorldUnits,
            region: Region::Cuboid(minimum, maximum),
        }
    }

    /// Whether the region is a region at all: finite numbers, and non-empty.
    ///
    /// **An inverted cuboid is refused rather than normalised.** Sorting the corners to make it
    /// valid would make the lane cover a region nobody declared.
    pub fn is_a_region(&self) -> bool {
        match self.region {
            Region::Rect(r) => {
                r.width_dp.is_finite()
                    && r.height_dp.is_finite()
                    && r.x.is_finite()
                    && r.y.is_finite()
                    && r.width_dp > 0.0
                    && r.height_dp > 0.0
            }
            Region::Cuboid(lo, hi) => {
                (0..3).all(|i| lo[i].is_finite() && hi[i].is_finite() && hi[i] > lo[i])
            }
        }
    }

    /// The lane's outline, drawn as four rules in its own unit's plane.
    ///
    /// **Screen lanes only**: a world lane is a box in three dimensions and the renderer that draws
    /// one projects its corners first (the vector change view, H-7.9). Asking this for a world lane
    /// is a refusal rather than a projection guessed here.
    pub fn outline(&self) -> Result<[Rect; 4], LaneRefusal> {
        let Region::Rect(r) = self.region else {
            return Err(LaneRefusal::NotARegion {
                lane: self.id.clone(),
                region: self.region.describe(),
            });
        };
        let (x, y, w, h) = (r.x, r.y, r.width_dp, r.height_dp);
        Ok([
            Rect::new(x, y, w, 1.0),
            Rect::new(x, y + h - 1.0, w, 1.0),
            Rect::new(x, y, 1.0, h),
            Rect::new(x + w - 1.0, y, 1.0, h),
        ])
    }
}

/// One declared set of lanes, checked against the surface it is drawn on.
///
/// **The budget is per unit and the surface is one region**, so a screen lane set and a world lane
/// set are checked by the same code against their own declared surfaces.
#[derive(Clone, Debug, Default)]
pub struct LaneSet {
    lanes: Vec<Lane>,
}

impl LaneSet {
    pub fn new() -> LaneSet {
        LaneSet { lanes: Vec::new() }
    }

    /// Add a lane. **Unvalidated**: [`LaneSet::validate`] is where the refusals are, so that a set
    /// can be assembled from several sources and checked once.
    pub fn push(&mut self, lane: Lane) -> &mut LaneSet {
        self.lanes.push(lane);
        self
    }

    pub fn lanes(&self) -> &[Lane] {
        &self.lanes
    }

    pub fn len(&self) -> usize {
        self.lanes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lanes.is_empty()
    }

    /// Every lane whose class is `class`, in declaration order.
    pub fn of_class(&self, class: LaneClass) -> Vec<&Lane> {
        self.lanes.iter().filter(|lane| lane.class == class).collect()
    }

    /// **Collects every refusal rather than returning the first**, the same rule
    /// `voxel_model::RefusalSet` follows: one broken lane in a set of two hundred is one finding,
    /// and a caller that stops at the first has to re-run to find the rest.
    pub fn validate(&self, surface: &Region) -> Vec<LaneRefusal> {
        let mut out = Vec::new();
        let mut by_unit: Vec<(LaneUnit, usize)> = Vec::new();
        for lane in &self.lanes {
            if lane.label.trim().is_empty() {
                out.push(LaneRefusal::Unlabelled { lane: lane.id.clone() });
            }
            if !lane.is_a_region() {
                out.push(LaneRefusal::NotARegion {
                    lane: lane.id.clone(),
                    region: lane.region.describe(),
                });
                continue;
            }
            if !lane.region.inside(surface) {
                out.push(LaneRefusal::OffSurface {
                    lane: lane.id.clone(),
                    region: lane.region.describe(),
                    surface: surface.describe(),
                });
            }
            match by_unit.iter_mut().find(|(unit, _)| *unit == lane.unit) {
                Some((_, count)) => *count += 1,
                None => by_unit.push((lane.unit, 1)),
            }
        }
        for (unit, count) in by_unit {
            let budget = lane_budget(unit);
            if count > budget {
                out.push(LaneRefusal::OverBudget { unit, lanes: count, budget });
            }
        }
        out
    }

    /// The lanes as elements: a labelled outline per screen lane, drawn in the design system's own
    /// ink ladder.
    ///
    /// **The class's hue, measured where the lane lands** (see the module docs). The hue is
    /// contrast-measured over **the backdrop the lane actually covers**, because ink is measured
    /// where it lands rather than where it was authored — and a lane that cannot be read over the
    /// surface it is drawn on is **skipped rather than drawn unreadable**.
    ///
    /// **No palette role is asked for**: a lane is an annotation over the surface, not a surface,
    /// and its colour is its class's declaration rather than a role the palette happens to admit.
    ///
    /// **World lanes are refused here rather than projected by guesswork**: the vector change view
    /// owns that projection (H-7.9) and this is the surface it draws into.
    pub fn elements(&self, backdrop_of: impl Fn(&Lane) -> voxel_ui::Rgb) -> Vec<Element> {
        let mut out = Vec::new();
        for lane in &self.lanes {
            if lane.unit != LaneUnit::Pixels {
                continue;
            }
            // **Measured where this lane lands, not where another one lands.** A lane over a window
            // body and a lane over a title bar sit on different surfaces, and one backdrop for the
            // whole set would be a number about neither — so a hue that fails over one surface is
            // still drawn where it reads.
            let hue = lane.class.hue();
            if voxel_ui::contrast_over_backdrop(hue, backdrop_of(lane)).is_err() {
                continue;
            }
            let paint = Paint { rgb: hue, alpha: 1.0 };
            let Ok(edges) = lane.outline() else { continue };
            for edge in edges {
                out.push(Element::Rule { rect: edge, paint });
            }
            let Region::Rect(r) = lane.region else { continue };
            // The label sits **at the lane's own position**, not in a legend.
            out.push(Element::Text {
                rect: Rect::new(r.x, r.y, r.width_dp, voxel_ui::text_size_dp()),
                text: format!("{} {}", lane.class.label(), lane.label),
                paint,
                size_dp: voxel_ui::text_size_dp(),
                align: Align::Start,
            });
        }
        out
    }
}

/// The **frozen vocabulary**: every lane identifier the client may declare, with its class.
///
/// **Adding a lane changes this table and therefore [`vocabulary_digest`]**, so a build that
/// declares a lane nobody reviewed stops matching the digest the tests pin. This is the same rule
/// as the evidence vocabulary: a growing vocabulary is not a silent growth.
pub const LANE_VOCABULARY: &[(&str, LaneClass)] = &[
    ("bar", LaneClass::Structure),
    ("bar-cell", LaneClass::Affordance),
    ("window", LaneClass::Structure),
    ("title-bar", LaneClass::Structure),
    ("window-body", LaneClass::Structure),
    ("pane", LaneClass::Structure),
    ("control", LaneClass::Affordance),
    ("menu-row", LaneClass::Affordance),
    ("world-extents", LaneClass::World),
    ("chunk", LaneClass::World),
    ("selection", LaneClass::World),
    ("change", LaneClass::Change),
    ("edit", LaneClass::Change),
    ("stream", LaneClass::State),
    ("budget", LaneClass::State),
];

/// The class a vocabulary identifier declares, or `None` for one the table does not hold.
///
/// **`None` for an undeclared identifier, never a guess.** A lane whose identifier is not in the
/// frozen vocabulary is a lane nobody reviewed, and the honest answer to "which class is it" is that
/// there isn't one yet.
pub fn vocabulary_class(id: &str) -> Option<LaneClass> {
    LANE_VOCABULARY
        .iter()
        .find(|(declared, _)| *declared == id)
        .map(|(_, class)| *class)
}

/// The digest over the frozen vocabulary, so a lane added without review stops matching.
pub fn vocabulary_digest() -> Digest {
    let mut bytes = Vec::new();
    for (id, class) in LANE_VOCABULARY {
        bytes.extend_from_slice(id.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(class.slug().as_bytes());
        bytes.push(b'\n');
    }
    Digest::of(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The surface the pixel lanes in these tests are declared against.
    fn screen() -> Region {
        Region::Rect(Rect::new(0.0, 0.0, 1280.0, 720.0))
    }

    #[test]
    fn a_lane_that_leaves_the_surface_is_refused_rather_than_clamped() {
        // The module's whole argument, as a test.
        let mut set = LaneSet::new();
        set.push(Lane::pixels(
            "window",
            LaneClass::Structure,
            "World",
            Rect::new(1200.0, 48.0, 400.0, 360.0),
        ));
        let refusals = set.validate(&screen());
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        match &refusals[0] {
            LaneRefusal::OffSurface { lane, region, surface } => {
                assert_eq!(lane, "window");
                assert!(region.contains("400"), "{region}");
                assert!(surface.contains("1280"), "{surface}");
            }
            other => panic!("wrong refusal: {other}"),
        }
    }

    #[test]
    fn a_lane_exactly_on_the_surface_edge_is_inside_it() {
        // Half-open, like `Rect::contains`: the far edge is the surface's, and a lane that reaches
        // it has not overflowed.
        let mut set = LaneSet::new();
        set.push(Lane::pixels(
            "window",
            LaneClass::Structure,
            "World",
            Rect::new(0.0, 0.0, 1280.0, 720.0),
        ));
        assert!(set.validate(&screen()).is_empty(), "{:?}", set.validate(&screen()));
    }

    #[test]
    fn an_inverted_cuboid_is_refused_rather_than_sorted() {
        // Sorting the corners would make the lane cover a region nobody declared.
        let lane = Lane::world("selection", LaneClass::World, "pick", [4.0, 8.0, 0.0], [0.0, 0.0, 1.0]);
        assert!(!lane.is_a_region());
        let mut set = LaneSet::new();
        set.push(lane);
        let refusals = set.validate(&Region::Cuboid([-64.0; 3], [64.0; 3]));
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert!(matches!(refusals[0], LaneRefusal::NotARegion { .. }));
    }

    #[test]
    fn a_pixel_lane_is_never_inside_a_world_lane() {
        // Two units, two overflows, two different defects. Comparing them would be a category
        // error that happens to compile.
        let pixel = Region::Rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        let world = Region::Cuboid([0.0; 3], [10.0; 3]);
        assert!(!pixel.inside(&world));
        assert!(!world.inside(&pixel));
    }

    #[test]
    fn the_budget_is_per_unit_and_declared() {
        let mut set = LaneSet::new();
        for i in 0..=LANE_BUDGET_WORLD_UNITS {
            let lo = [i as f64, 0.0, 0.0];
            let hi = [i as f64 + 0.5, 1.0, 1.0];
            set.push(Lane::world("chunk", LaneClass::World, format!("c{i}"), lo, hi));
        }
        let refusals = set.validate(&Region::Cuboid([-64.0; 3], [64.0; 3]));
        let over = refusals
            .iter()
            .find(|r| matches!(r, LaneRefusal::OverBudget { unit: LaneUnit::WorldUnits, .. }))
            .expect("the world budget is enforced");
        match over {
            LaneRefusal::OverBudget { unit, lanes, budget } => {
                assert_eq!((*unit, *lanes, *budget), (LaneUnit::WorldUnits, LANE_BUDGET_WORLD_UNITS + 1, LANE_BUDGET_WORLD_UNITS));
            }
            other => panic!("wrong refusal: {other}"),
        }
        // And the screen lanes in the same set are untouched by the world's overflow.
        assert!(!refusals.iter().any(|r| matches!(r, LaneRefusal::OverBudget { unit: LaneUnit::Pixels, .. })));
    }

    #[test]
    fn validate_collects_every_refusal_rather_than_the_first() {
        let mut set = LaneSet::new();
        set.push(Lane::pixels("a", LaneClass::Structure, "", Rect::new(0.0, 0.0, 10.0, 10.0)));
        set.push(Lane::pixels(
            "b",
            LaneClass::Structure,
            "off",
            Rect::new(2000.0, 0.0, 10.0, 10.0),
        ));
        set.push(Lane::world("c", LaneClass::World, "inverted", [1.0; 3], [0.0; 3]));
        let refusals = set.validate(&screen());
        assert_eq!(refusals.len(), 3, "{refusals:?}");
    }

    #[test]
    fn a_lane_with_no_label_is_refused() {
        let mut set = LaneSet::new();
        set.push(Lane::pixels("a", LaneClass::Structure, "   ", Rect::new(0.0, 0.0, 10.0, 10.0)));
        assert!(matches!(set.validate(&screen())[0], LaneRefusal::Unlabelled { .. }));
    }

    #[test]
    fn every_class_declares_a_distinct_hue_that_reads_over_the_surfaces_lanes_land_on() {
        // The user's ruling, 2026-10-03: *"the hues per lane was nice."* The count is still
        // measured here rather than asserted elsewhere, and a class added without a hue does not
        // compile — the match in `hue` has no wildcard arm.
        let mut seen: Vec<voxel_ui::Rgb> = Vec::new();
        for class in LaneClass::ALL {
            assert!(!class.slug().is_empty());
            assert!(!class.label().is_empty());
            let hue = class.hue();
            assert!(!seen.contains(&hue), "{} shares a hue with another class", class.slug());
            seen.push(hue);
        }
        assert_eq!(LaneClass::ALL.len(), 5, "the class set is measured here, not asserted elsewhere");
        // **Readable where lanes are drawn**, over the two backdrops `shell::debug` hands them:
        // the window body and the title bar. A hue that failed here would be a lane the overlay
        // silently skipped on every window.
        let palette = voxel_ui::Palette::declared();
        for role in [voxel_ui::Role::Surface, voxel_ui::Role::Chrome] {
            let backdrop = palette.get(role).expect("the declared palette names both roles");
            for class in LaneClass::ALL {
                assert!(
                    voxel_ui::contrast_over_backdrop(class.hue(), backdrop).is_ok(),
                    "{} is unreadable over the surface its lanes land on",
                    class.slug()
                );
            }
        }
    }

    #[test]
    fn the_vocabulary_is_frozen_and_a_lane_outside_it_has_no_class() {
        // Adding a row changes the digest; that is the point of the digest.
        assert_eq!(vocabulary_class("window"), Some(LaneClass::Structure));
        assert_eq!(vocabulary_class("control"), Some(LaneClass::Affordance));
        assert_eq!(vocabulary_class("not-a-lane"), None, "never a guess");
        // Every declared identifier has a distinct slug pair, so two rows cannot collide silently.
        let mut seen = std::collections::BTreeSet::new();
        for (id, class) in LANE_VOCABULARY {
            assert!(seen.insert((*id, class.slug())), "{id} is declared twice");
        }
        assert_eq!(vocabulary_digest(), vocabulary_digest(), "a digest is a function of the table");
    }

    #[test]
    fn the_vocabulary_covers_every_class_or_says_which_one_is_empty() {
        // A class nothing is in is a colour nobody should learn — so if a class has no lanes, the
        // test says so rather than letting the vocabulary drift away from the classes.
        let mut declared: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for (_, class) in LANE_VOCABULARY {
            declared.insert(class.slug());
        }
        for class in LaneClass::ALL {
            assert!(declared.contains(class.slug()), "{} has no lane in the vocabulary", class.slug());
        }
    }

    #[test]
    fn a_lane_draws_its_outline_and_its_label_at_its_own_position() {
        let mut set = LaneSet::new();
        set.push(Lane::pixels(
            "window",
            LaneClass::Structure,
            "World",
            Rect::new(16.0, 48.0, 336.0, 360.0),
        ));
        let elements = set.elements(|_| voxel_ui::Rgb::new(0, 0, 0));
        let labels: Vec<&str> = elements.iter().filter_map(|e| e.text()).collect();
        assert_eq!(labels, vec!["structure World"], "the label is drawn, not put in a legend");
        let text = elements.iter().find(|e| e.text().is_some()).expect("the label");
        assert_eq!(text.rect().x, 16.0, "at the lane's own x");
        assert_eq!(text.rect().y, 48.0, "and its own y");
        // Four edges, so the outline is a box and not a line.
        let rules = elements.iter().filter(|e| matches!(e, Element::Rule { .. })).count();
        assert_eq!(rules, 4);
    }

    #[test]
    fn a_world_lane_is_not_projected_by_guesswork() {
        // The vector change view owns the projection (H-7.9); this surface does not invent one.
        let mut set = LaneSet::new();
        set.push(Lane::world("chunk", LaneClass::World, "0,0,0", [0.0; 3], [16.0; 3]));
        let elements = set.elements(|_| voxel_ui::Rgb::new(0, 0, 0));
        assert!(elements.is_empty(), "{elements:?}");
        assert!(matches!(
            Lane::world("c", LaneClass::World, "c", [0.0; 3], [1.0; 3]).outline(),
            Err(LaneRefusal::NotARegion { .. })
        ));
    }
}
