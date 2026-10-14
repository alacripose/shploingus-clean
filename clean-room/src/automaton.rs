//! **The water law on the cubic world's own cells — one state, one law, one tick, and no schedule of its own.**
//!
//! Derived from the mission's own updated component for **U12** — *the game's cellular automata systems
//! should allow for mobs and autonomous players to meet their own needs through itself* — read against
//! `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 U12 and the dated sentences this law's two halves close: §32's
//! *"its needs are met through the game's own cellular automata"* (met by one **static** evaluation, which
//! §33 flagged rather than glossed), §33's own owed line — *"a stepping cellular automaton on the field
//! plane is **owed**: state that carries tick to tick, so a rained-on or drained column changes the being's
//! next step"* — which §34 closed with this law **on the field plane**, and §37's ruling 3, which named the
//! next seam in the law's own terms: the cubic chunk is the world, so the water law belongs **on the cells**
//! — *"(a) the water law restated **on the cells** (a graph read from the cells' own ground or faces)"* —
//! rather than on the generator-fed column field beside them.
//!
//! # One concern, one owner
//!
//! | concern | owner |
//! |---|---|
//! | the **cell state** the law steps | [`CubicWorld`] — the water stands in the cells' own `water_um`; this struct holds that world and the count of ticks it has taken, and **nothing else** |
//! | the **law** that evolves it | [`Automaton::step`] — rain, and one synchronous hop by the **cells' own** routing graph ([`d8_receivers`] called on the surfaces the cells return, never re-derived), plus two **declared** constants |
//! | the **tick** that advances it | [`Automaton::step`] returns the [`Tick`] it produced; [`Automaton::steps`] advances a declared count. There is **no clock, no thread and no schedule** here: the caller's own tick is the only time there is |
//! | the law's **read** of its state | [`CubicWorld::water_columns`] — the ground and the water of every column, in one sweep, and the same arithmetic the one-column reading uses (a guard measures the two against each other) |
//! | the law's **write** of its state | [`CubicWorld::pour_columns`] — the inverse, one voxel's worth per cell ([`CELL_HOLD_UM`]), refusing by name what the world cannot carry |
//! | the **difference the voxel makes**, attributed | [`attribute_routes`] — §37's open question, measured per column and decomposed by cause |
//!
//! It is **not** a second needs model and **not** a second scheduler: it holds nothing about any being —
//! a being's need is still the fold over its own records ([`crate::being::Being::thirsty`]) — and the state
//! is a pure function of the cells it was handed and the number of ticks it has taken. It is not a second
//! *water* law either: the routing function is the one `hydrology` stage's own `d8_receivers`, and the
//! water starts from the filled surface that stage's `lake_levels` produced, poured into the cells by the
//! world's own build.
//!
//! # The law, in full
//!
//! One tick is one **synchronous** pass — every column's transfer is computed from the state at the *start*
//! of the tick, so the answer cannot depend on the order the columns are visited in (the property a cellular
//! automaton is for, and the one the determinism bar measures):
//!
//! 1. **Rain falls on every column**, at the declared [`RAIN_UM_PER_TICK`].
//! 2. **Every column's own water moves one hop**, by the graph **read from the cells**: the surfaces
//!    [`CubicWorld::water_columns`] returns — each column's highest solid cell — are what [`d8_receivers`]
//!    compares, so the law routes by the world it steps and not by the float field the world was built
//!    from. What a column held at the start of the tick, plus its own rain, leaves for the neighbour its
//!    receiver names — in the same tick, and computed from the start-of-tick state — so a column that
//!    *carries* water holds at the end of the tick **only what arrived** (one tick of residence, in
//!    transit). That is what makes the filled surface the generator left be routed downhill instead of
//!    standing where it was put.
//! 3. **A pit** — a column no neighbour is lower than, so the cells' own graph names no receiver — **keeps
//!    the declared fraction** of the water it holds at the start of the tick, its own plus its rain, and
//!    loses the rest; what reaches it during the pass joins its holding and is thinned on the next tick. A
//!    pit is therefore the **only** place water stays between ticks, and its loss is what bounds the plane:
//!    the one source is `cells × RAIN_UM_PER_TICK` per tick and the one sink is the pits' thinning, so a
//!    pit at rest holds the fixed point of its own recurrence — `kept(holding + rain) + inflow` — at most
//!    `10 × cells × RAIN_UM_PER_TICK` when the whole plane drains into it, and far less when its catchment
//!    is small. A dry pit **wets up** on the way there. The write is bounded the same way the state is:
//!    the world's span is built in whole chunks above the highest surface, and water the world could not
//!    carry is refused **by name** by the pour rather than dropped (a guard pours past the span and reads
//!    the refusal).
//!
//! Three consequences are the slice's own demonstration, and the run prints all three: a **dry sink** is
//! dry at the base state and stands water after one tick (rained on); the columns that stood the
//! generator's filled surface hand their own water on and hold only what reached them in that same pass
//! (drained); and the pit's holding keeps rising while its **increment decelerates**, because the thinning
//! of what it already holds grows with it and the rain does not.
//!
//! # The measured start: the cells' own water, not a float conversion
//!
//! §34's state began from `WorldRead::depth_um` — the read's own float field converted to whole
//! micrometres, a conversion that truncates, so 5 of 2 304 columns started one micrometre low (§37
//! measured it). The cells carry whole micrometres already, so this law starts from them **exactly**, and
//! the settled figures moved with that: the movement is recorded beside the figures rather than hidden.
//!
//! # §37's open question, measured rather than answered by preference
//!
//! The last pass measured the routing difference — the graph read off the cells' quantised surface differs
//! from the graph read off the generator's float surface at **2 033 of 2 304** columns — and left it open
//! whether that was *"a faithful consequence of the cubic law or a defect in the reading"*. This module
//! carries the instrument that answers it ([`attribute_routes`]): every differing column is examined and
//! counted under one of three **voxel** causes — the whole downhill drop inside the column's own voxel (the
//! cells see a pit where the generator drained), the generator's winner inside its own voxel while the
//! cells route to the steepest candidate they can see, or a quantised tie at the steepest that the law's
//! own scan order decides — and a column that is none of them is counted **unexplained**, which is what a
//! defect in the reading would look like. The run prints the decomposition; the guard below measures it.
//! The measured answer is in §38: every difference is the voxel's, none is unexplained, and the reading's
//! graph is this law's graph.
//!
//! # The settled world, and why the run steps to it (§35)
//!
//! A short horizon flatters this law. The audit that followed §34 measured, out of tree, what eight ticks of
//! it look like beside the world they settle into, and §35 moved the printed figures to the settled ones:
//! the wet set is fixed early, the generator's lake — a transient slug — collapses as it drains into the
//! pits, and the plane's total stops moving well before the declared horizon. The run prints the numbers
//! the being actually lives in ([`SETTLE_TICKS`]) with the short-horizon transient beside them for
//! contrast, and the settled shape is pinned by the test below, because a law whose headline figure is a
//! passing one describes a world nobody lives in.

use crate::chunk::CubicWorld;
use crate::digest::Digest;
use crate::refusal::Refusal;
use crate::shapers::water::d8_receivers;
use crate::stage::Patch;
use crate::world::Tick;

/// **The declared rain** — micrometres of water the law adds to every column on every tick. A declared
/// constant rather than an invented rate: the run prints it, and the automaton's own tests measure it.
pub const RAIN_UM_PER_TICK: i64 = 200;

/// **The declared fraction a pit keeps** — per cent of what it holds, the rest draining out of the pit on
/// that tick. One hundred would be a pit that never loses anything (and never reaches an equilibrium);
/// zero would be a plane whose pits never hold water (and a being that could never drink from one).
pub const PIT_KEEP_PERCENT: i64 = 90;

/// **The declared count the run steps to rest by** — the plane's own horizon rather than the being's, and a
/// measured one: the wet set is fixed early, the generator's lake — a transient slug — has collapsed by
/// tick 50, and the plane's total water stops moving by tick 200. The run prints the settled numbers rather
/// than a passing figure, so what a reader compares its bar against is the world the being actually lives
/// in.
pub const SETTLE_TICKS: usize = 200;

/// **The stepped world**: the cubic world whose cells are the state, and the count of ticks the law has
/// taken. Nothing else — no per-column depths of its own, no height of its own, no weather, no being.
///
/// **The cells are the state.** The law holds the world whole rather than a derived vector beside it, so
/// there is no second copy that could drift from the cells the next consumer reads: [`Automaton::world`]
/// hands out the state itself, and [`crate::being::WorldRead::over_cubic`] is the reading of it. The
/// routing graph is not held either: every tick reads it from the surfaces the cells return, so it cannot
/// describe a world the law is not stepping.
#[derive(Clone, Debug, PartialEq)]
pub struct Automaton {
    world: CubicWorld,
    ticks: u64,
}

impl Automaton {
    /// **The state the cells leave** — the world's own cells, held whole, at tick zero of the law's own
    /// count. No stage is evaluated here and nothing is sampled: the law takes the world it is handed and
    /// keeps it, so there is no second argument that could name a different patch, a different graph or a
    /// different water.
    pub fn over(world: &CubicWorld) -> Automaton {
        Automaton { world: world.clone(), ticks: 0 }
    }

    /// **The world the law is stepping** — the state itself, for a reader that wants the cells
    /// ([`crate::being::WorldRead::over_cubic`] is the one reading of them every consumer shares).
    pub fn world(&self) -> &CubicWorld {
        &self.world
    }

    /// How many ticks this state has taken, and which tick that is.
    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    /// The tick this state stands at: the law starts at the base world's own tick and counts from there,
    /// so a stepped state after `n` ticks is the plane at `Tick(base + n)`.
    pub const fn tick(&self, base: Tick) -> Tick {
        Tick(base.0 + self.ticks)
    }

    /// The water the **whole plane** holds, in micrometres — read from the cells, and the aggregate the
    /// settled-shape measurement reads: the one number that says whether the plane has come to rest.
    pub fn total_um(&self) -> i64 {
        self.world.water_columns().iter().map(|column| column.water_um).sum()
    }

    /// The water a column of the patch holds, in micrometres — read from the cells above its ground, and
    /// never negative. `0` for an index the patch does not have, exactly as the reading's own accessors
    /// answer for one.
    pub fn depth_um(&self, cell: usize) -> i64 {
        self.world
            .patch()
            .position(cell)
            .and_then(|at| self.world.column(at).ok())
            .map(|column| column.water_um)
            .unwrap_or(0)
    }

    /// Whether the cells hold any water above this column's ground.
    pub fn water_stands(&self, cell: usize) -> bool {
        self.depth_um(cell) > 0
    }

    /// How many columns hold water, and the deepest one (first in the patch's own order on a tie) — both
    /// read from the cells.
    pub fn standing(&self) -> usize {
        self.world
            .water_columns()
            .iter()
            .filter(|column| column.water_um > 0)
            .count()
    }

    pub fn deepest(&self) -> Option<(usize, i64)> {
        self.world
            .water_columns()
            .iter()
            .enumerate()
            .filter(|(_, column)| column.water_um > 0)
            .max_by_key(|(cell, column)| (column.water_um, std::cmp::Reverse(*cell)))
            .map(|(cell, column)| (cell, column.water_um))
    }

    /// **The state as its one identity** — the cells' own digest ([`CubicWorld::digest`], §37's ruling 2).
    /// The law keeps no second number of its own: two states with the same cells are the same world, and a
    /// state that moved a micrometre anywhere is a different one. (The tick count is the law's, not the
    /// world's: a plane at rest at tick 200 and the same plane at tick 700 are one world, and one digest.)
    pub fn digest(&self) -> Digest {
        self.world.digest()
    }

    /// **One tick of the law** — the synchronous pass the module header describes, with the graph read from
    /// the cells it steps — and the [`Tick`] it produced. The write replaces every column's water at once,
    /// so no column reads a value its neighbour has already changed in the same tick. The one way a tick
    /// can refuse is the pour's own named refusal: water the world's span cannot carry is never dropped.
    pub fn step(&mut self) -> Result<Tick, Refusal> {
        // The state, read from the cells: every column's ground and the water above it.
        let columns = self.world.water_columns();
        // The graph, read from the same cells: the surfaces the world itself returns. One law's own
        // comparison, on the state it steps.
        let elevation: Vec<f64> = columns.iter().map(|column| column.surface_z as f64).collect();
        let receivers = d8_receivers(self.world.patch(), &elevation);
        // One **change** per column, computed from the state this tick started with: what leaves a column
        // and what arrives at it are decided by the same snapshot, so no column can see a value its
        // neighbour has already changed in this pass, and the answer cannot depend on the visit order.
        let mut poured = vec![0_i64; columns.len()];
        for (cell, column) in columns.iter().enumerate() {
            // What this column holds at the start of the tick, plus its own rain.
            let own = column.water_um + RAIN_UM_PER_TICK;
            match receivers[cell] {
                // One hop, in this tick: the column's own water leaves for its receiver, whole.
                Some(to) => poured[to] += own,
                // A pit keeps the declared share of its own water and rain, and drains the rest.
                None => {
                    poured[cell] += own - own * (100 - PIT_KEEP_PERCENT) / 100;
                }
            }
        }
        self.world.pour_columns(&poured)?;
        self.ticks += 1;
        Ok(Tick(self.ticks))
    }

    /// **A declared count of ticks**, in order — the whole of what "N ticks later" means here.
    pub fn steps(&mut self, count: usize) -> Result<Tick, Refusal> {
        for _ in 0..count {
            self.step()?;
        }
        Ok(Tick(self.ticks))
    }
}

/// **How the cells' own routing differs from the generator's float surface**, decomposed by cause — the
/// measured answer to §37's open question (*"a faithful consequence of the cubic law or a defect in the
/// reading"*), counted per column rather than asserted as a total.
///
/// A voxel is one metre and the cells' ground is the voxel that *holds* the generator's surface, so the
/// cells' height is the generator's rounded **down** to its voxel — monotone, so the quantisation can only
/// create ties, never invert an order (that is why a column the generator pits cannot become a routing
/// column in the cells, and the count is measured rather than argued). Every differing column is therefore
/// one of exactly three effects, and each is checked against the values rather than assumed:
///
/// 1. **The voxel swallowed the drop** ([`RouteAttribution::swallowed`]) — the generator drains, and every
///    neighbour its own law considered strictly lower is in the column's **own** voxel: the whole downhill
///    drop is sub-voxel, and the cells' law honestly sees a pit.
/// 2. **The voxel swallowed the winner** ([`RouteAttribution::inside_the_voxel`]) — both route, but the
///    generator's winner's drop is entirely inside the column's own voxel, so it is no candidate for the
///    cells, which route to the steepest of the candidates they can see.
/// 3. **The voxel tied the steepest** ([`RouteAttribution::tied_at_the_top`]) — both route, the generator's
///    winner is a candidate, and the quantised drop **ties** with the cells' winner at the maximum, so the
///    law's own scan order decides between equals.
///
/// Anything else is counted rather than rounded into one of the three: [`RouteAttribution::wrong_way`] for
/// a column the generator pits and the cells route (impossible while the quantisation is monotone, and
/// measured rather than assumed) and [`RouteAttribution::unexplained`] for a difference the voxel cannot
/// account for — which is exactly what a **defect in the reading** would look like. The run prints all of
/// it, so the answer to §37 is a measurement a reader can check, not a position.
///
/// # The check that comes before the decomposition
///
/// A decomposition of a difference cannot say whether the *reading* is honest — a reading that quantised
/// wrongly would produce differences this same three-way split would happily file. So the cause is measured
/// **first and at every column**: [`RouteAttribution::quantised`] counts the columns whose cells' ground is
/// the declared voxel of the generator's own elevation (`floor`), and [`RouteAttribution::not_the_voxel`]
/// counts the ones where it is not. The second count is the only one that could name a defect, and it is
/// printed beside the decomposition rather than left to the reader to infer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct RouteAttribution {
    pub columns: usize,
    /// The columns whose cells' ground **is** the declared voxel of the generator's own elevation — the
    /// reading's own fidelity, measured at every column rather than at the ones that route differently.
    pub quantised: usize,
    /// A column whose cells' ground is **not** the voxel that contains the generator's elevation: a defect
    /// in the reading itself, and the one count that would make every other number here meaningless.
    pub not_the_voxel: usize,
    pub agrees: usize,
    pub swallowed: usize,
    pub inside_the_voxel: usize,
    pub tied_at_the_top: usize,
    pub wrong_way: usize,
    pub unexplained: usize,
}

impl RouteAttribution {
    /// Every column that routes differently, by whatever cause.
    pub const fn differs(&self) -> usize {
        self.swallowed + self.inside_the_voxel + self.tied_at_the_top + self.wrong_way + self.unexplained
    }

    /// Whether every difference is one of the three voxel effects and the reading is the declared
    /// quantisation at every column — the answer §37 asked for, as a value.
    pub const fn is_the_voxel(&self) -> bool {
        self.not_the_voxel == 0 && self.wrong_way == 0 && self.unexplained == 0
    }
}

/// **The measured difference, per column** — the generator's float surfaces and the cells' own surfaces,
/// both routed by the one drawing law ([`d8_receivers`]), with every disagreement examined and counted
/// under its cause. See [`RouteAttribution`] for what each count means and why the three voxel causes are
/// the only ones a faithful reading can produce.
pub fn attribute_routes(patch: Patch, generator: &[f64], cells: &[f64]) -> RouteAttribution {
    let generated = d8_receivers(patch, generator);
    let quantised = d8_receivers(patch, cells);
    let mut counted = RouteAttribution { columns: patch.len(), ..RouteAttribution::default() };
    for cell in 0..patch.len() {
        // **The reading's own fidelity, measured at every column**: the cells' ground is the voxel that
        // contains the generator's elevation, or it is not, and this count is what says which.
        if cells[cell] == generator[cell].floor() {
            counted.quantised += 1;
        } else {
            counted.not_the_voxel += 1;
        }
        let (generator_receiver, cell_receiver) = (generated[cell], quantised[cell]);
        if generator_receiver == cell_receiver {
            counted.agrees += 1;
            continue;
        }
        // The steepest drop the cells can see among their **own** candidates — the value the law's own
        // scan compares, so "the cells route to the best they can see" is measured rather than asserted.
        let steepest = patch
            .neighbour_steps(cell)
            .into_iter()
            .map(|(neighbour, _, _)| cells[cell] - cells[neighbour])
            .filter(|drop| *drop > 0.0)
            .fold(f64::NEG_INFINITY, f64::max);
        match (generator_receiver, cell_receiver) {
            (Some(winner), None) => {
                // The generator drained; the cells see no lower neighbour at all. The voxel's cause, stated
                // concretely: every neighbour the generator's own law considered strictly lower sits in the
                // column's **own** voxel (`cells[n] == cells[cell]`), so the whole drop is sub-voxel.
                let all_inside = patch.neighbour_steps(cell).into_iter().all(|(neighbour, _, _)| {
                    generator[neighbour] >= generator[cell] || cells[neighbour] == cells[cell]
                });
                if all_inside && cells[winner] == cells[cell] {
                    counted.swallowed += 1;
                } else {
                    counted.unexplained += 1;
                }
            }
            (Some(winner), Some(other)) => {
                let winner_drop = cells[cell] - cells[winner];
                if winner_drop == 0.0 {
                    // The generator's winner is no candidate for the cells — its whole drop is inside the
                    // column's own voxel — and the cells route to the steepest candidate they can see.
                    if cells[cell] - cells[other] == steepest {
                        counted.inside_the_voxel += 1;
                    } else {
                        counted.unexplained += 1;
                    }
                } else if winner_drop == steepest && cells[cell] - cells[other] == steepest {
                    // Both are candidates and the quantised drop ties at the steepest: the law's own scan
                    // order decides between equals, exactly as it does on any tie.
                    counted.tied_at_the_top += 1;
                } else {
                    counted.unexplained += 1;
                }
            }
            // Monotone quantisation cannot make a pit drain: this is never expected, and it is counted
            // rather than assumed away.
            (None, Some(_)) => counted.wrong_way += 1,
            // Two sinks are an agreement, counted above; the arm exists so the match is total rather than
            // wild, which is how a fourth cause would have to be named rather than folded into a default.
            (None, None) => counted.agrees += 1,
        }
    }
    counted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::being::{self, Organisation};
    use crate::chunk::CubicWorld;
    use crate::field::fields_at;
    use crate::process::{declared_request, roster};
    use crate::reference;
    use crate::stage::FieldId;
    use crate::world::Tick;

    /// The three declared fields and the reference ladder — the same fields the mesher, the client and the
    /// world's own build read, so a test and the binary describe one world rather than two similar ones.
    fn fields() -> (Patch, Vec<f64>, Vec<f64>, Vec<f64>, crate::mesh::Ladder) {
        let roster = roster().expect("the declared roster must admit");
        let identity = reference::identity().expect("the reference identity");
        let request = declared_request();
        let values = fields_at(
            &roster,
            identity,
            Tick(0),
            &[FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology],
            request,
            1,
        )
        .expect("the three declared fields answer in one evaluation");
        (
            request.patch().expect("the declared patch"),
            values[0].clone(),
            values[1].clone(),
            values[2].clone(),
            reference::ladder().expect("the reference ladder"),
        )
    }

    /// The one world every U12 measurement steps: the declared surface, filled into its cubic world — the
    /// same build the binary and the acceptance tests use.
    fn cubic() -> CubicWorld {
        let (patch, elevation, water_level, classes, ladder) = fields();
        CubicWorld::from_surface(patch, &elevation, &water_level, &classes, &ladder)
            .expect("the declared surface fills a cubic world")
    }

    /// The being's own read of a world at the tick the world is in — the one reading every consumer
    /// shares.
    fn read(world: &CubicWorld) -> being::WorldRead {
        being::WorldRead::over_cubic(Organisation::first(), "the first mob", world)
            .expect("the world's own cells are readable")
    }

    /// The surfaces the cells return, in the patch's own order — what `attribute_routes` compares.
    fn cell_surfaces(world: &CubicWorld) -> Vec<f64> {
        world
            .water_columns()
            .iter()
            .map(|column| column.surface_z as f64)
            .collect()
    }

    /// The water one column contributes to its receiver in one tick: what it holds, plus its own rain.
    fn own(column: &crate::chunk::ColumnWater) -> i64 {
        column.water_um + RAIN_UM_PER_TICK
    }

    #[test]
    fn one_tick_is_one_synchronous_hop_by_the_declared_arithmetic_over_the_cells() {
        let world = cubic();
        let columns = world.water_columns();
        let receivers = d8_receivers(world.patch(), &cell_surfaces(&world));
        let mut stepped = Automaton::over(&world);
        stepped.step().expect("the declared world holds one tick of the law's own rain");
        // Every column's new water, stated as the law states it: what carried into it, plus — for a pit —
        // the declared fraction of what it held at the start of the pass.
        for cell in 0..columns.len() {
            let arrived: i64 = (0..columns.len())
                .filter(|upstream| receivers[*upstream] == Some(cell))
                .map(|upstream| own(&columns[upstream]))
                .sum();
            let expected = match receivers[cell] {
                // Its own water left in this tick, whole: what stands here is only what arrived.
                Some(_) => arrived,
                // A pit keeps the declared share of its own water and rain, and holds the arrival entire.
                None => own(&columns[cell]) - own(&columns[cell]) * (100 - PIT_KEEP_PERCENT) / 100 + arrived,
            };
            assert_eq!(stepped.depth_um(cell), expected, "column {cell}");
            // And the write put it back in the cells: the same number, read through the one-column
            // reading rather than the law's own accessor.
            let at = world.patch().position(cell).expect("every patch column has a position");
            assert_eq!(
                stepped.world().column(at).expect("the stepped world still owns its columns").water_um,
                expected,
                "column {cell}: the cells must hold what the law's arithmetic says they should"
            );
        }
        // A column that carried water at the base state keeps none of its own: the highest column the base
        // state stands water on holds, after one tick, only what its neighbours rained into it.
        let read = read(&world);
        let carrying = (0..columns.len())
            .filter(|cell| read.water_stands(*cell) && receivers[*cell].is_some())
            .max_by(|a, b| read.elevation[*a].partial_cmp(&read.elevation[*b]).expect("terrain"))
            .expect("the declared patch has a column that carries water downhill");
        assert!(
            stepped.depth_um(carrying) < own(&columns[carrying]),
            "a carrying column cannot keep its own water: {} µm stands where {} µm left it",
            stepped.depth_um(carrying),
            own(&columns[carrying])
        );
        // And the plane conserves water apart from the pits, which thin the declared fraction of what they
        // hold: nothing else here creates or destroys it.
        let before: i64 = columns.iter().map(|column| column.water_um).sum();
        let input = columns.len() as i64 * RAIN_UM_PER_TICK;
        let drained: i64 = (0..columns.len())
            .filter(|cell| receivers[*cell].is_none())
            .map(|cell| own(&columns[cell]) * (100 - PIT_KEEP_PERCENT) / 100)
            .sum();
        assert_eq!(
            stepped.total_um(),
            before + input - drained,
            "one tick in, one tick out"
        );
    }

    #[test]
    fn rain_wets_a_dry_pit_and_the_plane_comes_to_rest_at_the_laws_own_fixed_point() {
        let world = cubic();
        let base = read(&world);
        let sink = being::dry_sink(&base).expect("the declared patch has a dry sink");
        let mut stepped = Automaton::over(&world);
        assert!(!stepped.water_stands(sink), "a dry sink is dry at the base state");
        stepped.step().expect("the declared world holds the rain");
        // A pit that held nothing cannot stand nothing after a tick of rain: water stands where it falls.
        // (The exact per-column arithmetic of that tick is the law's own test's subject; this test's is
        // what happens over many of them.)
        let one = stepped.depth_um(sink);
        assert!(one > 0, "the declared rain wets a dry pit: {one} µm after one tick");
        stepped.steps(99).expect("the declared world holds the rain");
        let hundred = stepped.depth_um(sink);
        stepped.steps(2_000).expect("the declared world holds the rain");
        let two_thousand = stepped.depth_um(sink);
        stepped.steps(100).expect("the declared world holds the rain");
        let later = stepped.depth_um(sink);
        assert!(hundred > one, "the declared rain wets a dry pit");
        // **Then it comes to rest.** The loss is a share of what the pit holds and the rain is constant, so
        // the increments shrink to nothing and the holding settles — here it is already flat from tick 100
        // to tick 2100, which is what makes this a bounded law rather than one that grows without bound.
        assert_eq!(
            two_thousand, hundred,
            "the holding settles within the first hundred ticks and stays put: {hundred} µm"
        );
        assert_eq!(
            later, two_thousand,
            "and the plane has come to rest: {two_thousand} µm from tick 100 to tick 2100"
        );
        // And the rest is **the law's own fixed point**, checked as this law's arithmetic rather than as a
        // formula invented here: what the sink keeps of its holding plus the water its upstream neighbours
        // send it, next tick, is what it is holding now.
        let receivers = d8_receivers(world.patch(), &cell_surfaces(stepped.world()));
        let inflow: i64 = (0..world.patch().len())
            .filter(|upstream| receivers[*upstream] == Some(sink))
            .map(|upstream| stepped.depth_um(upstream) + RAIN_UM_PER_TICK)
            .sum();
        let held = two_thousand + RAIN_UM_PER_TICK;
        assert_eq!(
            two_thousand,
            held - held * (100 - PIT_KEEP_PERCENT) / 100 + inflow,
            "at rest the sink holds the fixed point of its own recurrence"
        );
        let ceiling = 10 * world.patch().len() as i64 * RAIN_UM_PER_TICK;
        assert!(
            two_thousand <= ceiling,
            "the pit holds {} µm, under the whole-plane bound {ceiling} µm",
            two_thousand
        );
    }

    #[test]
    fn the_plane_comes_to_rest_and_the_settled_world_is_not_the_transient() {
        // What the run prints is the world the being lives in, and §35 measured that a short horizon is not
        // that world. Pinned here as the law's own shape: the wet set is the same from the short horizon
        // onwards, the transient slug collapses, and the plane's total stops moving.
        let world = cubic();
        let mut early = Automaton::over(&world);
        early.steps(8).expect("the declared world holds the rain");
        let mut settled = Automaton::over(&world);
        settled.steps(SETTLE_TICKS).expect("the declared world holds the rain");

        assert_eq!(
            settled.standing(),
            early.standing(),
            "the wet set is fixed early — {} column(s) stand water at tick 8 and at tick {SETTLE_TICKS}",
            early.standing()
        );
        assert!(
            settled.total_um() < early.total_um(),
            "the plane's total falls as the generator's lake drains into the pits: {} µm at tick 8, {} µm at rest",
            early.total_um(),
            settled.total_um()
        );
        assert!(
            settled.deepest().map(|(_, depth)| depth) < early.deepest().map(|(_, depth)| depth),
            "and the deepest column is no longer the transient peak: {:?} at rest against {:?} at tick 8",
            settled.deepest(),
            early.deepest()
        );
        // **At rest means at rest**: five hundred further ticks move nothing — not the total, not the
        // deepest column, and not the world's own identity.
        let mut later = settled.clone();
        later.steps(500).expect("the declared world holds the rain");
        assert_eq!(later.total_um(), settled.total_um(), "the total water stops moving at rest");
        assert_eq!(later.deepest(), settled.deepest(), "and so does the deepest column");
        assert_eq!(later.digest(), settled.digest(), "and the world's own identity stops moving with it");
        // **The settled world is a marsh, and that is a declared consequence rather than a surprise**: more
        // than half the plane stands water — measured, and named here so the figure cannot drift unnoticed.
        assert!(
            settled.standing() * 2 > world.patch().len(),
            "the settled plane is more than half wet: {} of {} column(s)",
            settled.standing(),
            world.patch().len()
        );
    }

    #[test]
    fn the_law_is_a_pure_function_of_the_cells_and_the_tick_count() {
        // Two runs of one world produce one state and one identity, and the state is untouched by the order
        // the law was asked for it: a step is one pass, and every write lands on the next state.
        let world = cubic();
        let mut a = Automaton::over(&world);
        let mut b = Automaton::over(&world);
        a.steps(6).expect("the declared world holds the rain");
        b.step().expect("the declared world holds the rain");
        b.steps(5).expect("the declared world holds the rain");
        assert_eq!(a, b);
        assert_eq!(a.digest(), b.digest());
        assert_eq!(a.ticks(), 6);
        assert_eq!(a.tick(Tick(0)), Tick(6));
        // And a fresh state is the cells' own water, not a plane of zeros — read from the cells, and equal
        // to the one-column reading of the same world.
        let fresh = Automaton::over(&world);
        let at = world.patch().position(0).expect("a patch column");
        assert_eq!(
            fresh.depth_um(0),
            world.column(at).expect("the base world owns its columns").water_um
        );
        assert_eq!(fresh.ticks(), 0);
    }

    #[test]
    fn the_states_identity_is_the_worlds_own_identity() {
        // §37's ruling 2 read into the law: the law keeps no digest of its own, so the state's identity is
        // the cells' digest, and a tick that moves any cell moves it.
        let world = cubic();
        let before = world.digest();
        let mut stepped = Automaton::over(&world);
        assert_eq!(stepped.digest(), before, "an unstepped law is the world it holds");
        stepped.step().expect("the declared world holds the rain");
        assert_eq!(
            stepped.digest(),
            stepped.world().digest(),
            "and a stepped one is still the cells' own digest, not a second number"
        );
        assert_ne!(stepped.digest(), before, "one tick of rain moved the world's identity");
    }

    #[test]
    fn the_read_a_being_gets_is_the_stepped_worlds_own_cells() {
        let world = cubic();
        let base = read(&world);
        let mut stepped = Automaton::over(&world);
        stepped.steps(4).expect("the declared world holds the rain");
        let stepped_read = read(stepped.world());
        let carrying = (0..base.cells())
            .find(|cell| base.water_stands(*cell))
            .expect("water stands somewhere");
        assert_eq!(stepped_read.patch, base.patch, "the patch is the world's, not the law's");
        assert_eq!(stepped_read.origin, base.origin);
        assert_eq!(stepped_read.elevation, base.elevation, "the ground did not move with the water");
        assert_eq!(
            stepped_read.receivers,
            d8_receivers(world.patch(), &cell_surfaces(stepped.world())),
            "one routing graph, read from the stepped cells"
        );
        assert_eq!(
            stepped_read.depth_um(carrying),
            stepped.depth_um(carrying),
            "the read's water is the law's state"
        );
        assert_ne!(
            stepped_read.depth_um(carrying),
            base.depth_um(carrying),
            "and it is not the base world's any more"
        );
    }

    #[test]
    fn every_route_difference_is_the_voxel_and_none_is_unexplained() {
        // §37's open question, measured: the generator's float surface against the cells' own, every
        // disagreement counted under its cause. The guards are the two counts that must be zero.
        let (patch, elevation, _, _, _) = fields();
        let world = cubic();
        let attribution = attribute_routes(patch, &elevation, &cell_surfaces(&world));
        assert_eq!(attribution.columns, patch.len());
        assert_eq!(
            attribution.quantised + attribution.not_the_voxel,
            attribution.columns,
            "every column's reading is checked against the declared quantisation: {} of {}, {} not",
            attribution.quantised,
            attribution.columns,
            attribution.not_the_voxel
        );
        assert_eq!(
            attribution.not_the_voxel, 0,
            "the cells' ground must be the voxel that holds the generator's elevation at every column — \
             this is what a defect in the reading would look like: {attribution:?}"
        );
        assert_eq!(
            attribution.agrees + attribution.differs(),
            attribution.columns,
            "every column is counted once"
        );
        assert_eq!(
            attribution.wrong_way, 0,
            "the cells cannot route where the generator's own law pitted: the voxel's quantisation is monotone"
        );
        assert_eq!(
            attribution.unexplained, 0,
            "every difference must be one of the three voxel effects: {attribution:?}"
        );
        assert!(attribution.is_the_voxel(), "the decomposition answers §37: {attribution:?}");
        // And the total is the same difference the last pass printed, restated as a decomposition rather
        // than a count: the direct comparison is measured beside the attributed one.
        let generated = d8_receivers(patch, &elevation);
        let quantised = d8_receivers(patch, &cell_surfaces(&world));
        let direct = (0..patch.len())
            .filter(|cell| generated[*cell] != quantised[*cell])
            .count();
        assert_eq!(attribution.differs(), direct, "the attributed total is the routing difference");
        println!(
            "route attribution: {} of {} column(s) route differently — {} swallowed whole in their own \
             voxel, {} with the generator's winner inside its own voxel, {} tied at the steepest quantised \
             drop; {} wrong way, {} unexplained; the reading is the declared voxel at {} column(s), not at {}",
            attribution.differs(),
            attribution.columns,
            attribution.swallowed,
            attribution.inside_the_voxel,
            attribution.tied_at_the_top,
            attribution.wrong_way,
            attribution.unexplained,
            attribution.quantised,
            attribution.not_the_voxel
        );
    }

    #[test]
    fn the_law_steps_the_cells_and_not_the_generator() {
        // The law's graph must be the *cells'* graph, which this patch measures to differ from the
        // generator's. One tick's arithmetic follows the cells' receivers where the two graphs disagree,
        // and one ground cell written (privately, where the writer lives) moves the graph the next tick
        // reads.
        let world = cubic();
        let columns = world.water_columns();
        let cell_graph = d8_receivers(world.patch(), &cell_surfaces(&world));
        let (patch, elevation, _, _, _) = fields();
        let generator_graph = d8_receivers(patch, &elevation);
        let different = (0..columns.len()).filter(|cell| cell_graph[*cell] != generator_graph[*cell]).count();
        assert!(different > 0, "this patch is where the two graphs differ, and the law must follow the cells");
        let mut stepped = Automaton::over(&world);
        stepped.step().expect("the declared world holds the rain");
        // Every column's water after one tick is the cells'-graph arithmetic, at a column where the two
        // graphs name different receivers — so a law still reading the generator's field would answer
        // something else there. The prediction is the law's own arithmetic under a graph, so it is stated
        // once and evaluated under both, rather than the test carrying a second law of its own.
        let predict = |graph: &[Option<usize>], cell: usize| -> i64 {
            let own = columns[cell].water_um + RAIN_UM_PER_TICK;
            let arrived: i64 = (0..columns.len())
                .filter(|upstream| graph[*upstream] == Some(cell))
                .map(|upstream| columns[upstream].water_um + RAIN_UM_PER_TICK)
                .sum();
            match graph[cell] {
                // Its own water left in this tick: only what arrived stands here.
                Some(_) => arrived,
                // A pit keeps the declared share and holds the arrival entire.
                None => own - own * (100 - PIT_KEEP_PERCENT) / 100 + arrived,
            }
        };
        // **Every column, not one**: the tick is the cells' own graph's arithmetic everywhere, and the count
        // of columns the two graphs answer differently on is measured beside it — a law that read the
        // generator's own field would have to disagree here wherever the two graphs do.
        let mut moved = 0_usize;
        for cell in 0..columns.len() {
            assert_eq!(
                stepped.depth_um(cell),
                predict(&cell_graph, cell),
                "column {cell}: the tick must be the cells' own graph's arithmetic"
            );
            if predict(&cell_graph, cell) != predict(&generator_graph, cell) {
                moved += 1;
            }
        }
        assert!(
            moved > 0,
            "and the generator's own graph answers differently at {moved} column(s) — this is where a law \
             still reading it would be caught"
        );
    }
}
