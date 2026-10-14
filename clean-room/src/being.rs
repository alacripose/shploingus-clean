//! **U12's first mob — one being, its organisation, its need, and its governed acts.**
//!
//! Derived from **documents**, not from the witness's source (Constitution P1/P7):
//!
//! | Source | What it declares here |
//! |---|---|
//! | `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 **U12** | the interface: *an `Organisation` with declared membership and authority, and an actor that asks the one tool table what it may do and whose every act is a governed event on the one `Chain`*; and §5 row 12's check: *an organisation's member acts only through the one tool table, and an act it may not do is refused by name and recorded* |
//! | the same plan, §16 | the owed half this pass reads: *U2's owed civilian/government vocabulary (T016)* — the organisation's nouns — and U7, because *a mob's choices are axis/laya questions* |
//! | `docs/CIVILIAN-PORT.md` (the `gov/mod.rs` row) and `src/axis.rs` | the vocabulary this tree does **not** declare: the **office rank** and the **certification bands** stay **owed by name** (`AxisKind::OrgRank`, `AxisKind::Certification`), so a being's office is the organisation's own name for it and never an invented rung |
//! | `docs/DESIGN-SPEC.md` §2 **F11** (actors, governed work) and **F8** (a ceiling refuses) | an act is governed — decided at one call site, denied by name where the charter does not reach, and recorded |
//! | `docs/SHAPERS.md` §1 row 5 (`hydrology`) | the world the being walks and drinks from is the game's **own** cell-based water law — `d8_receivers` (O'Callaghan & Mark 1984), and `lake_levels` (Priority-Flood, Barnes, Lehman & Mulla 2014) which filled it — **read out of the cubic world's own cells** ([`WorldRead::over_cubic`]), never a second needs model or a second automaton. **[`WorldRead::of`] is the *field* path**: a single static evaluation at the tick the caller passes, kept for the comparisons that are *about* the generator's float surface (the client's instrument, §37's attribution), and no longer the plane a need is met through. The state that **carries from tick to tick** is [`crate::automaton`]'s (§34, restated on the cells §38), which steps the same routing graph over those same cells, so a rained-on or drained column does change the being's next step |
//! | `patches/PATCH-039-the-corpus-beyond-this-tree.md` and `src/von.rs` | the **stage question**: the five stages a decision passes through, and the one calibrated instrument in this tree that reads which of them a record is at |
//!
//! # The being, one concern with one owner
//!
//! A being is a **record**, and everything about it is **read** from records: its need is a fold over its
//! own acts (never a field, never a per-tick decrement — this build has no need tick anywhere), its cell is
//! the cell its last act names, and its authority is its organisation's charter. Its choice is a **laya
//! question** (`Question::Choice` over the five stages, the floor declared in the charter before the answer
//! exists) and the **von** classifier reads its drafted act — so an act whose own record cannot be read is
//! refused, and an act read below the charter's floor is refused **by name**
//! ([`RefusalKind::StageBelowFloor`]) and recorded. That is the mission's ensoulment clause read on this
//! tree's own substrate: the being is not piloted (no tool moves it), it carries no hidden interior, and
//! what it is, is what its records say — read by the reading layer rather than asserted beside it.
//!
//! # The floor, and the **one** declared fact it rests on
//!
//! Measured, not guessed: the stage question reads a record's **head** and scores only the facts that head
//! **declares**. A member's own act stands on nothing, and its head carries `dependencies: []` — that one
//! *declared* fact ([`von::Declared::DeclaresNoDependency`]) is what carries the reading to
//! `verified-internal`; a head declaring `approval: design-approved` reads `proposed` instead. The key is
//! written into the head as a declared empty list rather than left to be *inferred* from a missing key,
//! because `von::head_field` reads a key the head names, and a fact this build depends on should be declared
//! rather than assumed. (An indented key is not read at all — the head's keys start at column zero.)
//!
//! **That fact holds by absence, and this slice says so.** An empty list declares no dependency: every other
//! fact of this tree's *patch*-record vocabulary — `operation`, `approval`, `source`, and a dependency that
//! is *there* — is **absent** from the draft's head, so what the floor rests on is a head that declares
//! almost nothing, read through a vocabulary that is not the being's own. A being-act vocabulary whose facts
//! a floor could rest on **positively** is owed (named below), and the run prints the one fact's own name
//! (`declares_no_dependency`) rather than a count, so the absence is visible in the output and not only here.
//! A member's act is its **own**: it carries nobody's approval, so it is an *internal* act, and the charter's
//! floor says exactly that — an act below internal verification (a proposal, a design) is filed rather than
//! taken, and an act carrying somebody's approval is not the member's to take either.
//!
//! **Owed, and named**: the being's act is scored on this tree's *patch*-record vocabulary
//! (`operation`/`approval`/`source`/`dependencies`). A being-act-appropriate declared-fact vocabulary — so
//! the floor can rest on a fact about an *act* that is **declared present** rather than on one that holds by
//! absence — is owed; this slice declares its one fact explicitly in place of inventing that vocabulary.
//!
//! # The first mob's world, and why it is not a second automaton
//!
//! The being's world is the U3 evaluation **filled into the cubic world's own cells** ([`crate::chunk`]) —
//! the same `elevation` and the same `water_level` the roster's own `hydrology` stage computed, quantised to
//! the voxel that holds each column's ground and carried in whole micrometres. Water stands where the
//! cells above a column's ground hold any (`CubicColumn::water_stands`), and the being's step follows the
//! **game's own flow direction**: [`crate::shapers::water::d8_receivers`], called again rather than
//! re-derived. A being whose reach finds no such cell is refused by name
//! ([`RefusalKind::WaterUnreachable`]) and the refusal is **recorded**. The field plane is still a plane this
//! tree can evaluate — and is, for the instrument and the attribution — but it is **not** the plane a need is
//! met through: one world, one law, one state.
//!
//! **One world read, and the stepping law's own state.** The being reads a world twice in the run: the
//! **cubic world's own cells** at `Tick(0)` ([`WorldRead::over_cubic`] of the world the stage's fields
//! were filled into) and the same world **after the law stepped it** ([`crate::automaton::Automaton`],
//! which holds that world's cells as its state — since §38 the law steps the cubic cells themselves) —
//! so a rained-on or drained column genuinely changes the being's next step, and the run prints both
//! planes side by side. [`WorldRead::of`] remains the **field path**: the generator's own float surface,
//! which the client's instrument and §37's own comparisons read, and which is no longer the plane the
//! being's need is met through. What has **not** been added is a second needs model or a second scheduler:
//! the automaton holds no being, no clock and no schedule (its own tick count is advanced only by an
//! explicit call), the being's need is still the fold over its own records ([`Being::thirsty`]), and its
//! acts are still one governed event each on the one `Chain`.

use std::path::{Path, PathBuf};

use crate::chain::{Chain, EventKind, Namespace};
use crate::coordinates::VoxelPos;
use crate::digest::Digest;
use crate::field::fields_at;
use crate::laya::{self, Ballot, Question, Verdict};
use crate::policy::{self, Action, Caller, Grant};
use crate::refusal::{Refusal, RefusalKind, RefusalSet};
use crate::shapers::water::d8_receivers;
use crate::stage::{Declared, FieldId, Patch, ProcessRequest};
use crate::von::{self, Classifier, STAGES};
use crate::world::{Tick, WorldIdentity};

/// **The first organisation's floor** — the stage a member's draft must read at before the act is taken. It is
/// declared **before any answer exists** (laya's own first rule) and it is the stage an *internal* act is at:
/// a proposal or a design is not yet verified, and an act carrying somebody's approval is not the member's
/// own. Each charter carries its own floor, so a stricter organisation is a declared value rather than a
/// second code path.
pub const ACT_FLOOR: &str = "verified-internal";

/// **The organisation's own vocabulary** — a name, the **charter** that authorises its acts (a document in
/// this tree, because in this tree authority is traced to a document), the **office** a member holds, and the
/// scopes that office reaches.
///
/// The **rank** a government would order offices by is `AxisKind::OrgRank`, which this tree does not declare,
/// and the certification bands are `AxisKind::Certification`: both stay **owed by name**. So an office here is
/// a *name the organisation uses* and a set of scopes — never a rung invented in code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Organisation {
    name: &'static str,
    charter: &'static str,
    office: &'static str,
    scopes: &'static [&'static str],
    floor: &'static str,
}

impl Organisation {
    /// **The first organisation** — its charter is the port document that names the civilians' own process,
    /// which is the document this pass may re-derive from (`docs/CIVILIAN-PORT.md`, U12's fresh boundary).
    pub const fn first() -> Organisation {
        Organisation {
            name: "the undertow congregation",
            charter: "docs/CIVILIAN-PORT.md",
            office: "tidewarden",
            scopes: &["record"],
            floor: ACT_FLOOR,
        }
    }

    /// **A stricter organisation**: the same charter, a higher floor. Its members' acts read at
    /// `verified-internal` and are therefore filed rather than taken — declared here so the floor's refusal is
    /// a value this build can hold and measure, rather than a branch a test could invent.
    pub const fn stricter() -> Organisation {
        Organisation {
            name: "the draught guild",
            charter: "docs/CIVILIAN-PORT.md",
            office: "gatekeeper",
            scopes: &["record"],
            floor: "applied-internal",
        }
    }

    /// The stage a member's draft must read at before the act is taken — the charter's own floor.
    pub const fn floor(self) -> &'static str {
        self.floor
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn charter(self) -> &'static str {
        self.charter
    }

    pub const fn office(self) -> &'static str {
        self.office
    }

    pub const fn scopes(self) -> &'static [&'static str] {
        self.scopes
    }

    /// The member's caller: one grant per scope the office reaches, each **attributed to the charter**. An
    /// empty granter or an undeclared scope is refused at construction by the policy module's own rules, so a
    /// member cannot quietly become a wildcard.
    pub fn caller(self, member: &'static str) -> Result<Caller, Refusal> {
        let mut grants = Vec::with_capacity(self.scopes.len());
        for scope in self.scopes {
            grants.push(Grant::of(self.charter, scope)?);
        }
        Caller::of(member, grants)
    }

    /// **The charter read once over the whole closed action vocabulary** — every action this build declares,
    /// with the member's own decision beside it. This is what an autonomous actor asks before it acts, and the
    /// answer is [`policy::decide`]'s rather than a second table's.
    pub fn charter_read(self, member: &'static str) -> Result<Vec<(&'static str, bool)>, Refusal> {
        let caller = self.caller(member)?;
        let mut rows = Vec::with_capacity(Action::ALL.len());
        for action in Action::ALL {
            rows.push((action.name(), policy::decide(&caller, action).is_ok()));
        }
        Ok(rows)
    }
}

/// What a being did on one tick — the act its record names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// One step along the game's own flow direction, to the cell the receivers name.
    Walk { from: usize, to: usize },
    /// The being stands where the world's own filled surface is above the ground.
    Drink { cell: usize, depth_um: i64 },
}

impl Act {
    pub const fn name(self) -> &'static str {
        match self {
            Act::Walk { .. } => "walk",
            Act::Drink { .. } => "drink",
        }
    }
}

/// **The world as the being reads it** — a surface, and the receivers of the game's own flow law over the
/// same columns. Nothing here is a new law: [`WorldRead::of`] computes `depth` as `water_level −
/// elevation` of the fields the roster already produced, [`WorldRead::over_cubic`] reads the same two
/// values out of the cubic world's own cells (the world [`crate::automaton::Automaton`] steps), and the
/// receivers are [`d8_receivers`] over whichever surface the constructor read. One struct, two
/// constructors, and no third way to answer with a water.
///
/// **One water truth (2026-10-13, §35).** [`WorldRead::depth`] is the *authority*: the water this read
/// stands, whichever state it was built over. [`WorldRead::water_level`] is the surface that water makes with
/// the ground — the roster's own field at [`WorldRead::of`], and **derived** (`elevation + depth`) over any
/// other state, so the struct cannot answer the water question twice with different answers. Before §35 it
/// could: a stepped read carried the generator's `water_level` beside its own `depth`, and
/// [`surface_digest`] — which hashes `elevation` and `water_level` — returned **the same digest for the
/// stepped world as for the static one**. (It was named `cells_digest` then; §37 and `LEDGER.md` C-189
/// renamed it for what it hashes, because the world's identity is now the cubic cells' own digest.)
#[derive(Clone, Debug, PartialEq)]
pub struct WorldRead {
    pub patch: Patch,
    pub origin: VoxelPos,
    pub elevation: Vec<f64>,
    /// The **surface** — the roster's own `water_level` field at [`WorldRead::of`], and `elevation + depth`
    /// for a stepped state. Derived from [`WorldRead::depth`], never a second authority for it.
    pub water_level: Vec<f64>,
    /// **The water this read stands**, one depth per column: the filled surface's own at [`WorldRead::of`],
    /// and the cells' own once the world has been stepped ([`WorldRead::over_cubic`] of
    /// [`crate::automaton::Automaton::world`]). Nothing downstream re-derives it, so a being cannot read
    /// one water and drink from another.
    pub depth: Vec<f64>,
    pub receivers: Vec<Option<usize>>,
}

impl WorldRead {
    /// **Read the generator's own fields** — the field path, kept for the comparisons that are *about* it
    /// (§37's surface reading, and the client's instrument). Two declared fields in **one** evaluation (one
    /// plane, one run), so "the same patch" cannot mean two different runs — and the read is the being's
    /// own `ReadWorld` act, decided at the one call site like every other act it takes.
    pub fn of(
        organisation: Organisation,
        member: &'static str,
        roster: &Declared,
        identity: WorldIdentity,
        tick: Tick,
        request: ProcessRequest,
        workers: usize,
    ) -> Result<WorldRead, Refusal> {
        let caller = organisation.caller(member)?;
        policy::decide(&caller, Action::ReadWorld)?;
        let patch = request.patch()?;
        let values = fields_at(
            roster,
            identity,
            tick,
            &[FieldId::Elevation, FieldId::WaterLevel],
            request,
            workers,
        )?;
        let elevation = values.first().cloned().unwrap_or_default();
        let water_level = values.get(1).cloned().unwrap_or_default();
        let depth = elevation
            .iter()
            .zip(&water_level)
            .map(|(ground, surface)| (surface - ground).max(0.0))
            .collect();
        let receivers = d8_receivers(patch, &elevation);
        Ok(WorldRead { patch, origin: patch.origin(), elevation, water_level, depth, receivers })
    }

    /// **The same world, read out of a cubic world's own cells** (2026-10-13, §37 and `LEDGER.md` C-189; the
    /// world the law steps since §38).
    ///
    /// The whole of the column world's survival is in these four lines: each column's surface reading
    /// ([`crate::chunk::CubicWorld::column`]) gives the ground and the water, the surface follows §35's
    /// identity (`water_level = elevation + depth`), and the routing graph is the **same** law
    /// ([`d8_receivers`]) over the surface the cells return — so this read is not a second world model, it
    /// is the *one* read every consumer already takes, sourced from the cells instead of from
    /// [`crate::field::fields_at`].
    ///
    /// **This is the reading of the world the stepping law actually stepped**: the law's state *is* the
    /// world ([`crate::automaton::Automaton::world`]), so passing that world here is the one way to read
    /// the stepped plane, and there is no second constructor that could answer with a state the law did
    /// not produce.
    ///
    /// The read still goes through the being's own act: the permission decided is [`Action::ReadWorld`].
    pub fn over_cubic(
        organisation: Organisation,
        member: &'static str,
        cubic: &crate::chunk::CubicWorld,
    ) -> Result<WorldRead, Refusal> {
        let caller = organisation.caller(member)?;
        policy::decide(&caller, Action::ReadWorld)?;
        let patch = cubic.patch();
        let columns = cubic.columns()?;
        let elevation: Vec<f64> = columns
            .iter()
            .map(|column| column.surface_um as f64 / 1_000_000.0)
            .collect();
        let depth: Vec<f64> = columns
            .iter()
            .map(|column| column.water_um as f64 / 1_000_000.0)
            .collect();
        let water_level = elevation
            .iter()
            .zip(&depth)
            .map(|(ground, water)| ground + water)
            .collect();
        let receivers = d8_receivers(patch, &elevation);
        Ok(WorldRead { patch, origin: patch.origin(), elevation, water_level, depth, receivers })
    }

    /// The water this read stands above the ground at a cell — the world's own filled surface at
    /// [`WorldRead::of`], and the automaton's own state once the plane has been stepped. Never negative.
    pub fn depth(&self, cell: usize) -> f64 {
        self.depth.get(cell).copied().unwrap_or(0.0).max(0.0)
    }

    /// Whether the world's own law stands water at a cell.
    pub fn water_stands(&self, cell: usize) -> bool {
        self.depth(cell) > 0.0
    }

    /// The cell one step along the game's own flow direction, or nothing where it is a sink.
    pub fn downhill(&self, cell: usize) -> Option<usize> {
        self.receivers.get(cell).copied().flatten()
    }

    pub fn cells(&self) -> usize {
        self.patch.len()
    }

    /// The depth in micrometres, as a whole number a payload can carry without a float format.
    pub fn depth_um(&self, cell: usize) -> i64 {
        (self.depth(cell) * 1_000_000.0) as i64
    }
}

/// **The walk's own one answer: where a being standing at `cell` first stands in water, and how many steps
/// that takes** — one hop at a time along the game's own flow direction ([`WorldRead::downhill`]) while no
/// water stands, in whole steps, or `None` where the walk never reaches water within `reach`.
///
/// This is the walk [`Being::tick`] takes one act at a time, answered as a pair — the step count and the
/// column — so an instrument can compare two planes' walks column by column without carrying a second walk of
/// its own, and so [`start_cell`] and [`dry_sink`] read the same law rather than two descriptions of it. A
/// column the world's own routing carries nowhere ends the walk, exactly as the being's own tick refuses there.
pub fn walk_to_water(world: &WorldRead, cell: usize, reach: usize) -> Option<(usize, usize)> {
    let mut at = cell;
    for step in 1..=reach {
        at = world.downhill(at)?;
        if world.water_stands(at) {
            return Some((step, at));
        }
    }
    None
}

/// **Where the world's own law puts the first mob** — the first dry cell, in the patch's own order, whose
/// flow path reaches standing water within `reach` steps ([`walk_to_water`]). A being placed there has a walk
/// a reader can watch; a being placed on water has no walk at all, and a being placed elsewhere may have
/// nowhere to go. Both ends of that are the world's own law's answer rather than a coordinate anybody chose.
pub fn start_cell(world: &WorldRead, reach: usize) -> Option<usize> {
    (0..world.cells())
        .find(|cell| !world.water_stands(*cell) && walk_to_water(world, *cell, reach).is_some())
}

/// **The first dry sink** — a cell the world's own routing carries nowhere and where no water stands: the place
/// a being's reach cannot help, kept here so the refusal is a value the evidence and the tests share.
pub fn dry_sink(world: &WorldRead) -> Option<usize> {
    (0..world.cells()).find(|cell| !world.water_stands(*cell) && world.downhill(*cell).is_none())
}

/// **One being**: its name, its organisation, the chain its acts go on, the directory its drafts are read in
/// place from, how far it will walk before it refuses, and the cell its own record declares as its birthplace.
/// It holds **no interior**: its need, its cell and its acts are read off the chain.
#[derive(Debug)]
pub struct Being {
    name: &'static str,
    organisation: Organisation,
    chain: Chain,
    dir: PathBuf,
    reach: usize,
    born_at: Option<usize>,
}

impl Being {
    /// **The one constructor, from the being's own record** — and the **soul check** is here. A being's record
    /// must carry a claim: a document with no claim is a non-reading ([`crate::von::regions`]'s own fence has
    /// no body to read), so a being without von is refused **by the model** with the reading layer's own word.
    pub fn of(
        name: &'static str,
        organisation: Organisation,
        record: &Path,
        dir: PathBuf,
        reach: usize,
    ) -> Result<Being, Refusal> {
        let text = std::fs::read_to_string(record).map_err(|error| {
            Refusal::new(
                RefusalKind::SourceAbsent,
                format!("the being's record {} is not there to be read: {error}", record.display()),
            )
        })?;
        let Some((head, body)) = von::regions(&text) else {
            return Err(Refusal::new(
                RefusalKind::NoReading,
                format!(
                    "{} is not a record at all: it opens with no frontmatter fence, so this being carries no \
                     claim — and a being whose records cannot be read is not ensouled, which this model refuses \
                     rather than steps",
                    record.display()
                ),
            ));
        };
        if body.trim().is_empty() {
            return Err(Refusal::new(
                RefusalKind::NoReading,
                format!(
                    "{} carries a head and an empty claim: there is nothing of this being to read, and an \
                     unreadable being is refused rather than piloted",
                    record.display()
                ),
            ));
        }
        let born_at = von::head_field(head, "cell").and_then(|value| value.parse::<usize>().ok());
        std::fs::create_dir_all(&dir).map_err(|error| {
            Refusal::new(
                RefusalKind::DocumentUnwritable,
                format!("the being's draft directory {} cannot be made: {error}", dir.display()),
            )
        })?;
        Ok(Being { name, organisation, chain: Chain::new(), dir, reach, born_at })
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn organisation(&self) -> Organisation {
        self.organisation
    }

    pub fn chain(&self) -> &Chain {
        &self.chain
    }

    pub fn reach(&self) -> usize {
        self.reach
    }

    pub fn drafts_dir(&self) -> &Path {
        &self.dir
    }

    /// **The being's cell, read from its own records** — the cell its last act names, or the cell its record
    /// declares as its birthplace. Never a field this struct mutates for its own sake.
    pub fn cell(&self) -> Option<usize> {
        self.chain
            .events()
            .iter()
            .rev()
            .find(|event| event.kind() == EventKind::BeingActed)
            .and_then(|event| field_of(event.payload(), "cell"))
            .and_then(|value| value.parse::<usize>().ok())
            .or(self.born_at)
    }

    /// **The being's need, read from its own records**: it is thirsty until one of its own acts drew from the
    /// world's own water. A fold over the chain, so there is no meter to tick and no field to drift.
    pub fn thirsty(&self) -> bool {
        !self.chain.events().iter().any(|event| {
            event.kind() == EventKind::BeingActed && field_of(event.payload(), "act") == Some("drink")
        })
    }

    /// How many steps this being has taken, read from its own records the same way.
    pub fn steps(&self) -> usize {
        self.chain
            .events()
            .iter()
            .filter(|event| {
                event.kind() == EventKind::BeingActed && field_of(event.payload(), "act") == Some("walk")
            })
            .count()
    }

    /// How many refusals of its own this being has on the record — each denial and each unmet need is one.
    pub fn refusals(&self) -> usize {
        self.chain
            .events()
            .iter()
            .filter(|event| event.kind() == EventKind::RefusalRecorded)
            .count()
    }

    /// **One tick: the being's drive, from the world's own cells to its own record.** It reads the world,
    /// drafts the act it would take, puts the draft to the stage question, and — only if the reading is the
    /// charter's floor — asks the one call site for permission and appends the governed event.
    ///
    /// Refused by name at every step: no water within reach ([`RefusalKind::WaterUnreachable`]), an
    /// unreachable or uncalibrated instrument (`oracle_unreachable` / `not_established`, the laya layer's own),
    /// an unreadable draft (`no_reading`), a reading below the floor ([`RefusalKind::StageBelowFloor`]), and a
    /// denial ([`RefusalKind::PolicyDenied`]). **Every one of them is recorded** before it is returned.
    pub fn tick(
        &mut self,
        world: &WorldRead,
        classifier: &Classifier,
        root: &Path,
    ) -> Result<Act, Refusal> {
        let cell = self.cell().unwrap_or_else(|| (world.cells() / 2).min(world.cells().saturating_sub(1)));
        if !self.thirsty() {
            // The need is read, not ticked: a being whose records already say it drank has nothing to do, and
            // the act it names is the drink it took.
            return Ok(Act::Drink { cell, depth_um: world.depth_um(cell) });
        }
        // The act the being would take: drink where the world's own water stands, otherwise one step along the
        // game's own flow direction. A step is refused once its reach is spent.
        let act = if world.water_stands(cell) {
            Act::Drink { cell, depth_um: world.depth_um(cell) }
        } else if self.steps() >= self.reach {
            return Err(self.record_refusal(Refusal::new(
                RefusalKind::WaterUnreachable,
                format!(
                    "the being walked its whole reach ({} cell(s)) from cell {cell} and found no cell where the \
                     world's own filled surface stands above the ground: a need the world cannot meet where the \
                     being may reach is refused by name and recorded, rather than met by a meter that would \
                     hide the world's own answer",
                    self.reach
                ),
            )));
        } else {
            match world.downhill(cell) {
                Some(next) => Act::Walk { from: cell, to: next },
                None => {
                    return Err(self.record_refusal(Refusal::new(
                        RefusalKind::WaterUnreachable,
                        format!(
                            "cell {cell} is a sink of the game's own flow law — no receiver carries it — and no \
                             water stands there: the being has nowhere the world's own routing can take it"
                        ),
                    )))
                }
            }
        };

        // The draft: the act's own record, written in place because a ballot is about an **artifact** and the
        // reading is re-derivable from the bytes at that path.
        let draft = self.draft_path();
        let bytes = draft_record(self.name, self.organisation, &act, world);
        std::fs::write(&draft, &bytes).map_err(|error| {
            Refusal::new(
                RefusalKind::DocumentUnwritable,
                format!("the being's draft {} cannot be written: {error}", draft.display()),
            )
        })?;

        // **The question**, over the five declared stages, with the charter's floor declared before the answer
        // exists — and the **von** classifier as the instrument, because a mob's choices are axis/laya
        // questions and this is the one calibrated reading in the tree.
        let reading = match stage_of(self.organisation, self.name, &draft, classifier, root) {
            Ok((reading, _digest, _features)) => reading,
            Err(refusal) => return Err(self.record_refusal(refusal)),
        };
        if reading != self.organisation.floor() {
            return Err(self.record_refusal(Refusal::new(
                RefusalKind::StageBelowFloor,
                format!(
                    "the being's act reads `{reading}` where the charter's floor is `{}`: an act below the \
                     floor is not taken, and the being files it rather than acting on it",
                    self.organisation.floor()
                ),
            )));
        }

        // **The one call site.** The act is a governed event or it is nothing; a denial is recorded before it
        // is returned, so a refusal is a finding on the record rather than a sentence in a log.
        let caller = self.organisation.caller(self.name)?;
        if let Err(denial) = policy::decide(&caller, Action::AppendEvent) {
            return Err(self.record_refusal(denial));
        }
        let payload = match act {
            Act::Walk { from, to } => format!(
                "being={} org={} act=walk cell={to} from={from} stage={reading}",
                self.name,
                self.organisation.name()
            ),
            Act::Drink { cell, depth_um } => format!(
                "being={} org={} act=drink cell={cell} depth_um={depth_um} stage={reading}",
                self.name,
                self.organisation.name()
            ),
        };
        self.chain
            .append(Namespace::Record, EventKind::BeingActed, payload)
            .map_err(refusal_from_the_record)?;
        Ok(act)
    }

    /// **An act the charter does not reach**, attempted so the denial can be seen: the being asks for a world
    /// write it holds no scope for. The denial is returned **by name** and — because every denial is recorded —
    /// it is on the chain before the caller sees it.
    pub fn attempt_act_it_may_not_do(&mut self) -> Result<(), Refusal> {
        let caller = self.organisation.caller(self.name)?;
        match policy::decide(&caller, Action::SaveWorld) {
            Ok(()) => Ok(()),
            Err(denial) => Err(self.record_refusal(denial)),
        }
    }

    /// The being's own record of its acts, re-walked: every link and every digest. A chain the being acted on
    /// must verify, or nothing it says about itself stands.
    pub fn verify(&self) -> Result<(), RefusalSet> {
        self.chain.verify()
    }

    /// The draft this tick would write — one path per act, under the being's own directory.
    pub fn draft_path(&self) -> PathBuf {
        self.dir.join(format!("{}-act-{}.md", self.name.replace(' ', "-"), self.chain.len()))
    }

    /// **Record a refusal, then return it.** Every refusal a being meets is a finding on the one chain: a
    /// denial, a need the world cannot meet, an act below its own floor. A refusal that cannot be recorded is
    /// returned carrying that fact rather than swallowing it.
    fn record_refusal(&mut self, refusal: Refusal) -> Refusal {
        let payload = format!(
            "being={} org={} refusal={} detail={}",
            self.name,
            self.organisation.name(),
            refusal.kind().name(),
            refusal.detail()
        );
        match self.chain.append(Namespace::Record, EventKind::RefusalRecorded, payload) {
            Ok(_) => refusal,
            Err(chain_refusal) => refusal_from_the_record(chain_refusal),
        }
    }
}

/// **Put one drafted act to the stage question** — the being's drive's own seam, and the one place the laya
/// and von layers meet this module. Returns the stage the instrument read, the artifact's digest and the
/// declared features that held, so a report can show the reading rather than assert it.
pub fn stage_of(
    organisation: Organisation,
    being: &'static str,
    draft: &Path,
    classifier: &Classifier,
    root: &Path,
) -> Result<(String, Digest, Vec<&'static str>), Refusal> {
    let question = Question::choice(&STAGES, organisation.floor())
        .map_err(|refusals| refusal_from_a_set(RefusalKind::BallotIncomplete, &refusals))?;
    let artifact = draft.to_string_lossy().to_string();
    let ballot = Ballot::declare(
        &format!("u12-{being}-act"),
        "the being's own drive to meet its need",
        &artifact,
        "read the being's draft act and say which stage of its life the act is at",
        question,
        being,
        24,
    )
    .map_err(|refusals| refusal_from_a_set(RefusalKind::BallotIncomplete, &refusals))?;
    let answered = laya::answer(
        &ballot,
        classifier,
        root,
        Some("the being's own note: an act is taken only where its own record has been read at the charter's floor"),
    )
    .map_err(|refusals| refusal_from_a_set(RefusalKind::NoReading, &refusals))?;
    if answered.verdict() == Verdict::NoReading {
        return Err(Refusal::new(
            RefusalKind::NoReading,
            format!(
                "the stage question came back a non-reading for `{}`: a body with nothing in it to read is not \
                 a stage, and a non-reading is never a pass",
                draft.display()
            ),
        ));
    }
    // **The instrument's own option, read rather than re-derived** ([`laya::Answer::chosen`]). There is no
    // second argmax here, so nothing in this module can disagree with the instrument about a tie.
    let Some(reading) = answered.chosen() else {
        return Err(Refusal::new(
            RefusalKind::NoReading,
            format!(
                "the stage question answered for `{}` but named no option it chose: an answer with no option is \
                 a non-reading whatever its verdict word says",
                draft.display()
            ),
        ));
    };
    Ok((reading.to_string(), answered.evidence().digest, answered.evidence().features.clone()))
}

/// One `key=value` field out of a governed payload. The payload form is the chain's own (no tabs, no
/// newlines), and a field that is not there is `None` rather than a guess.
pub fn field_of<'a>(payload: &'a str, key: &str) -> Option<&'a str> {
    payload.split(' ').find_map(|pair| pair.strip_prefix(&format!("{key}=")))
}

/// The draft act record — the being's own decision document. Its **claim** carries the four parts every
/// decision in this tree carries (change, acceptance, evidence, rollback), because that is what a reader
/// checks it against; the **head** carries the being's own declarations, and the stage question reads those.
fn draft_record(name: &str, organisation: Organisation, act: &Act, world: &WorldRead) -> Vec<u8> {
    let (action, cell, metres, extra) = match act {
        Act::Walk { from, to } => ("walk", *to, world.depth(*to), format!("from {from}")),
        Act::Drink { cell, .. } => {
            ("drink", *cell, world.depth(*cell), "a draw from standing water".to_string())
        }
    };
    let text = format!(
        "---\n\
         record: being-act\n\
         being: {name}\n\
         organisation: {org}\n\
         office: {office}\n\
         need: water\n\
         dependencies: []\n\
         act: {action}\n\
         cell: {cell}\n\
         {extra}\n\
         ---\n\
         \n\
         # Change\n\
         \n\
         {name} takes one act toward its own need: it {action}s at cell {cell} of the world's own grid \
         ({extra}). The world's cells are not written: the act is a record of a being, and the ground it stands \
         on is the game's own law's to change, not the being's.\n\
         \n\
         # Acceptance\n\
         \n\
         The act is accepted when the world's own cells stand `{metres:.6}` m of water above the ground at the \
         cell the act names, read out of those cells rather than remembered — the ground is the voxel that holds \
         it and the water is the cells above it, one reading (§38).\n\
         \n\
         # Evidence\n\
         \n\
         | what | where |\n\
         |---|---|\n\
         | the world's own flow law | `d8_receivers` (O'Callaghan & Mark 1984) |\n\
         | the world's own water | `lake_levels` (Priority-Flood, Barnes, Lehman & Mulla 2014) |\n\
         | the stage question | `patches/PATCH-039-the-corpus-beyond-this-tree.md` |\n\
         | the vocabulary | `docs/CIVILIAN-PORT.md` (the `gov/mod.rs` row) |\n\
         \n\
         Re-derive with `cargo test --offline -p shploingus-clean-room being` and read the same cells; the \
         being's chain carries the act's own governed event and its digest.\n\
         \n\
         # Rollback\n\
         \n\
         Nothing in the world is mutated by this act: the world is a pure function of its seed, its declared \
         period and its tick, so undoing the act is dropping this record rather than repairing a cell. The \
         being's chain is append-only, so the rollback a reader needs is the refusal path: an act below the \
         charter's floor is filed and never taken.\n\
         \n\
         ## How this draft is read\n\
         \n\
         The record is put in place and read in place: the head for its declared facts, the claim for its \
         features, and the stage question answered over the five stages by the classifier calibrated on this \
         tree's own sixteen readings.\n",
        name = name,
        org = organisation.name(),
        office = organisation.office(),
        action = action,
        cell = cell,
        extra = extra,
        metres = metres,
    );
    text.into_bytes()
}

/// The draft a being writes when it declares its act a *design* rather than an act of its own — the voyage at
/// the head of this module's floor: a head carrying somebody's approval reads `proposed`, and the being files
/// it. Kept in the module because the refusal is part of the being's own drive, and the evidence and the tests
/// drive the same bytes.
pub fn draft_of_a_declared_design(name: &str, organisation: Organisation, cell: usize) -> Vec<u8> {
    format!(
        "---\n\
         record: being-act\n\
         being: {name}\n\
         organisation: {org}\n\
         approval: design-approved\n\
         source: user-directed\n\
         dependencies: []\n\
         act: walk\n\
         cell: {cell}\n\
         ---\n\
         \n\
         # Change\n\
         \n\
         {name} would walk to cell {cell}, declared as a design carrying somebody's approval rather than as an \
         act of its own.\n\
         \n\
         # Rollback\n\
         \n\
         Nothing is taken: the draft is filed.\n",
        org = organisation.name(),
    )
    .into_bytes()
}

/// A refusal whose set came from the laya layer: the first refusal is the one the caller sees, with the
/// caller's own context beside it so the seam is named rather than implied.
fn refusal_from_a_set(kind: RefusalKind, refusals: &RefusalSet) -> Refusal {
    match refusals.joined() {
        Some(first) => Refusal::new(first.kind(), format!("{first} ({kind:?} at the being's seam)")),
        None => Refusal::new(kind, "the set carried no refusal, which is itself a fault".to_string()),
    }
}

/// A chain refusal while recording: the record's own refusal is the one reported, with the fact that the
/// being's act could not be recorded beside it rather than blanking it out.
fn refusal_from_the_record(chain_refusal: Refusal) -> Refusal {
    Refusal::new(
        chain_refusal.kind(),
        format!(
            "the being's act could not be recorded on the one chain ({}): {}",
            chain_refusal.detail(),
            chain_refusal.kind().name()
        ),
    )
}

/// **The *surface read's* own digest** — `elevation` and `water_level`, as one number (2026-10-13, §37;
/// named `cells_digest` until the shape ruling, C-189).
///
/// It is **not** the world's identity, and the name no longer says it is: a `WorldRead` is a *reading of*
/// a cubic world's surface (ruling 1), and the world's own cells have their own identity —
/// [`crate::chunk::CubicWorld::digest`], which hashes every cell of every chunk. This digest hashes the two
/// per-column arrays of one reading, which is what it always did. A stepped world has a **different**
/// surface read from the world it was stepped from — its water moved — and the world's own identity
/// ([`crate::automaton::Automaton::digest`]) moves with it, so the two kinds of identity are both visible
/// and neither is mistaken for the other.
pub fn surface_digest(world: &WorldRead) -> Digest {
    let mut parts: Vec<Vec<u8>> = Vec::with_capacity(3);
    parts.push(format!("{:?}", world.origin).into_bytes());
    for values in [&world.elevation, &world.water_level] {
        parts.push(
            values.iter().map(|value| format!("{value:.6}")).collect::<Vec<_>>().join(",").into_bytes(),
        );
    }
    let borrowed: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
    Digest::of_parts(&borrowed)
}

/// The chain's own digest, for the report: what the being did, as one number.
pub fn chain_digest(chain: &Chain) -> Digest {
    let mut parts: Vec<Vec<u8>> = Vec::with_capacity(chain.len() * 2);
    for event in chain.events() {
        parts.push(event.payload().as_bytes().to_vec());
        parts.push(event.digest().as_bytes().to_vec());
    }
    let borrowed: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
    Digest::of_parts(&borrowed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// **The one world** — the declared request the whole binary reads, so a test and the report are answers
    /// about the same patch rather than about two similar ones.
    fn world() -> WorldRead {
        let roster = crate::process::roster().expect("the declared roster");
        let identity = crate::reference::identity().expect("the reference identity");
        WorldRead::of(
            Organisation::first(),
            "the first mob",
            &roster,
            identity,
            Tick(0),
            crate::process::declared_request(),
            1,
        )
        .expect("the world's own cells are readable")
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("shploingus-clean-u12-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    fn soul(dir: &Path, body: &str, cell: Option<usize>) -> PathBuf {
        let record = dir.join("being.md");
        let born = cell.map(|cell| format!("cell: {cell}\n")).unwrap_or_default();
        fs::write(
            &record,
            format!("---\nrecord: being\nbeing: the first mob\n{born}---\n\n{body}\n"),
        )
        .expect("the being's own record is written");
        record
    }

    #[test]
    fn an_unsouled_being_is_refused_with_the_reading_layer_s_own_word() {
        let dir = scratch("soul");
        let headless = dir.join("no-claim.md");
        fs::write(&headless, "record: being\nno fence at all\n").expect("written");
        let refused = Being::of("the first mob", Organisation::first(), &headless, dir.join("drafts"), 4)
            .expect_err("a being with no claim is refused");
        assert_eq!(refused.kind(), RefusalKind::NoReading);
        assert!(refused.detail().contains("ensouled"));

        let empty = soul(&dir, "", None);
        let refused = Being::of("the first mob", Organisation::first(), &empty, dir.join("drafts"), 4)
            .expect_err("a being with an empty claim is refused");
        assert_eq!(refused.kind(), RefusalKind::NoReading);
    }

    #[test]
    fn the_charter_reaches_the_record_and_never_the_world() {
        let organisation = Organisation::first();
        let rows = organisation.charter_read("the first mob").expect("the charter reads");
        assert_eq!(rows.len(), Action::ALL.len());
        let allowed: Vec<&str> = rows.iter().filter(|(_, ok)| *ok).map(|(name, _)| *name).collect();
        assert!(allowed.contains(&"read_world"));
        assert!(allowed.contains(&"append_event"));
        assert!(!allowed.contains(&"save_world"));
        assert!(!allowed.contains(&"write_external"));
        assert_eq!(organisation.charter(), "docs/CIVILIAN-PORT.md");
        assert_eq!(organisation.office(), "tidewarden");
    }

    #[test]
    fn the_world_s_own_law_stands_water_and_names_the_being_s_step() {
        let world = world();
        let cells = world.cells();
        let standing = (0..cells).filter(|cell| world.water_stands(*cell)).count();
        let sink = dry_sink(&world).expect("the reference patch has a dry sink");
        let start = start_cell(&world, 8).expect("a dry cell whose own flow path reaches water");
        assert_eq!(cells, 48 * 48, "the declared patch's own side");
        assert!(standing > 0, "the world's own hydrology must stand water on its own patch");
        assert_eq!(world.downhill(sink), None);
        assert!(!world.water_stands(sink));
        for cell in 0..cells {
            assert!(world.depth(cell) >= 0.0, "a depth the world's own law never reports is refused");
        }
        // The walk the run prints: the flow path from the being's own start cell to the water it drinks.
        let mut at = start;
        let mut steps = 0;
        while !world.water_stands(at) {
            at = world.downhill(at).expect("a cell on the chosen flow path leads somewhere");
            steps += 1;
            assert!(steps <= 8, "the start cell was chosen for a path within the being's reach");
        }
        println!("world: {cells} cells, {standing} hold water, sink {sink}, start {start} → {at} in {steps}");
        assert!(world.depth(at) > 0.0);
    }

    #[test]
    fn the_first_mob_walks_the_world_s_own_flow_law_to_water_and_drinks() {
        let dir = scratch("drive");
        let world = world();
        let classifier = Classifier::classified().expect("the sixteen readings calibrate");
        let root = von::root();
        let start = start_cell(&world, 8).expect("the world's own law names a start cell");
        let record = soul(&dir, "this being's need is water and its acts are its own.", Some(start));
        let mut mob =
            Being::of("the first mob", Organisation::first(), &record, dir.join("drafts"), 8).expect("the being");
        assert_eq!(mob.cell(), Some(start), "the birthplace its record declares");
        let mut acts = Vec::new();
        for _ in 0..8 {
            match mob.tick(&world, &classifier, &root).expect("the drive is not refused") {
                act @ Act::Drink { .. } => {
                    acts.push(act);
                    break;
                }
                act => acts.push(act),
            }
        }
        println!("drive: start {start} → {acts:?}");
        let walked = acts.iter().filter(|act| matches!(act, Act::Walk { .. })).count();
        assert!(walked >= 1, "the being's own drive walks the world's own flow path");
        let Some(Act::Drink { cell, depth_um }) = acts.last().copied() else {
            panic!("the drive must end in a drink: {acts:?}");
        };
        assert!(world.water_stands(cell), "the being drinks where the world's own law stands water");
        assert_eq!(depth_um, world.depth_um(cell));
        assert!(!mob.thirsty(), "the need is a fold over the being's own acts");
        assert!(mob.verify().is_ok(), "the being's chain verifies");
        assert_eq!(mob.steps(), walked);
        assert_eq!(mob.refusals(), 0, "a met need meets no refusal");
    }

    #[test]
    fn the_stage_question_reads_the_being_s_own_act_at_the_charter_s_floor() {
        let dir = scratch("stage");
        let world = world();
        let classifier = Classifier::classified().expect("the sixteen readings calibrate");
        let root = von::root();
        let start = start_cell(&world, 8).expect("a start cell");
        let act = Act::Walk { from: start, to: world.downhill(start).expect("a step") };
        let own = dir.join("own.md");
        fs::write(&own, draft_record("the first mob", Organisation::first(), &act, &world)).expect("written");
        // The fact the floor rests on is **declared in the head**, not left to be inferred from a missing
        // key — and the reading names it. Both are pinned here because §33's whole repair is this: a fact
        // that holds by absence is written down, so a reader can see what the floor stands on.
        let head = String::from_utf8(fs::read(&own).expect("the draft is readable")).expect("the draft is text");
        assert!(
            head.contains("dependencies: []"),
            "the draft's head declares its (empty) dependency list rather than omitting the key"
        );
        let (reading, digest, features) =
            stage_of(Organisation::first(), "the first mob", &own, &classifier, &root)
                .expect("the being's own draft is read");
        assert_eq!(
            reading,
            Organisation::first().floor(),
            "the measured reading of the being's own act: {features:?}"
        );
        assert_eq!(
            features,
            vec!["declares_no_dependency"],
            "one declared fact carries the floor, and it is the fact that holds by absence: {features:?}"
        );
        assert_eq!(digest.to_hex().len(), 64);

        // A draft that declares itself a design carrying somebody's approval reads below the floor — measured,
        // not assumed — so the being files it rather than acting on it.
        let design = dir.join("design.md");
        fs::write(&design, draft_of_a_declared_design("the first mob", Organisation::first(), start))
            .expect("written");
        let (reading, _, _) =
            stage_of(Organisation::first(), "the first mob", &design, &classifier, &root)
                .expect("the design's draft is read");
        assert_eq!(reading, "proposed");
        assert_ne!(reading, Organisation::first().floor());
        // And the stricter organisation's floor is above the reading the same act earns, so its members file
        // the act — the charter's own value, not a second branch.
        assert_ne!(Organisation::stricter().floor(), reading);
    }
}
