//! **The work this session found, filed as tickets rather than written as prose.**
//!
//! # Why these are filed and not appended to the ledger
//!
//! C-75.5 deferred *"how a ledger entry becomes a ticket"* until the audit settled which
//! questions were genuinely open, and the ruling since has been to file them. **The work below
//! is not a hand-kept list** — every objective cites the document that declares the gap and
//! the file+line that shows it is still open, and each is filed through the same chokepoint
//! the record uses.
//!
//! **The grammar is the board's, not this file's.** `split_kind` reads a leading `[BLD]`,
//! `Board::from_chain` derives the id from the chain position, and `Board` is a projection —
//! so filing a ticket here and reading the board there is one record read twice, which is
//! H-8.1's *"two projections of one record"* rather than a list and a store.
//!
//! # Why the ids are not asserted
//!
//! `Board::from_chain` numbers tickets `{prefix}-{chain position}`, so an id is a **position
//! in an append-only record** — adding a ticket shifts nothing before it, but hard-coding
//! `BLD-0003` here would make inserting a ticket a test failure. **The count and the set of
//! kinds are asserted; the ids are printed**, because the ids are a property of the record's
//! order rather than of this file.

use voxel_model::chain::{EventKind, Namespace, Season};
use voxel_model::envelope::{Control, EventType, Kind, RecordState};
use voxel_model::ticket::{Board, TicketKind};
use voxel_package::compiler::UnlinkedRecord;
use voxel_package::features::workspace_root;
use voxel_package::filing;

/// The namespace every filed ticket belongs to.
fn namespace() -> Namespace {
    Namespace::new("world-2468", 1, Season::Winter, "c-2026-10-01").expect("a namespace")
}

/// One piece of work, **with the document that declares it.**
///
/// **The `because` is not a comment.** A ticket whose objective is a bare imperative cannot be
/// checked by a later reader — they cannot tell whether it is still needed or was quietly
/// resolved. Carrying the declaring document in the ticket means the board answers *"why does
/// this exist"* without the ledger, which is the whole point of a ticket being a record rather
/// than a note.
struct Work {
    kind: TicketKind,
    objective: &'static str,
    because: &'static str,
}

/// **The work, in the order the chain holds it.**
///
/// **Every row here was measured, not recalled.** The pane-tree rows come from
/// `F:/Documents/simulators/docs/implementation_plans/task_splitting_and_grid_engine.md`'s five
/// numbered requirements, of which two are implemented; the craft-floor rows come from
/// `F:/Documents/skills_directory/reference/craft-floor.md`, read this session for the first
/// time; the rest are gaps this session's own measurements opened.
const WORK: [Work; 42] = [
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] resize a pane: drag a divider, move the stretch factors, and refuse a \
                    resize that would break the 40x40 floor",
        because: "task_splitting_and_grid_engine.md requirement 3 — divider handles slide freely; \
                  arrangement.rs has stretch factors and no operation that changes them, so \
                  check_floor guards a resize that cannot happen",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] dock a window: hit-test a drag to a pane or an outer edge, preview the \
                    zone, and insert on drop",
        because: "requirement 4 — dock_window(source, target_pane, zone) plus the start/continue/\
                  end drag hit-test; arrangement.rs has undock, which is the inverse, and no way \
                  back in",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] build the shell-owned notification queue: one writer, one rendering per \
                    record, one at a time",
        because: "H-12.2 'The shell owns a queue'; notification_queue is unchecked with no \
                  witness at crates/voxel-client/src/shell/notifications.rs",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] render the gate's findings as a Panel from the same structure the gate \
                    prints, so a green build and a legible panel cannot disagree",
        because: "UI-SPEC §9 'two readers' (C-53.3); the filing half is possible now that \
                  filing::linked_records exists, and the rendering half is not started",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] enforce the unenforced halves of the craft floor in voxel-ui as \
                    refusals: depth offset, measure, motion easing, copy naming recovery",
        because: "skills_directory/reference/craft-floor.md — voxel-ui already holds the contrast \
                  and state halves by type, and nothing anywhere measures depth, measure, motion \
                  or copy",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] re-audit each feature row's means text against what its witness \
                    actually contains, and stop reporting active when the witness is thinner \
                    than the feature",
        because: "shell_arrangement reads active because arrangement.rs exists; the reference \
                  declares five requirements and two are implemented, and features.rs states its \
                  own means is 'checked against nothing'",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] make Openness::Settled constructible, so a reader of the record can see \
                    that a question is answered",
        because: "C-74.2 — the arm is declared and documented and constructed nowhere, so the \
                  reader still reports 27 open questions about a record with none",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule how the pre-chain ledger entries become chain history, or rule that \
                    the projection begins fresh from C-73",
        because: "39 entries are in docs/LEDGER.md and not on the chain; the projection emitter \
                  stays gated until this is answered, and it is the record's decision",
    },

    // ---- The ticking atmosphere scene (TODO §4 ask 2; C-88.7) ----
    //
    // Nine tickets rather than one, because a ticket whose objective cannot be
    // checked is a note. Each of these is separately verifiable, and the source
    // is named per row so the sky's numbers stay auditable one at a time.

    Work {
        kind: TicketKind::Build,
        objective: "[BLD] make solar position a pure function of (tick, declared latitude): \
                    declination and hour angle to altitude and azimuth, with no clock and no RNG",
        because: "TODO §4 row 1 — Meeus, Astronomical Algorithms 2nd ed. 1998, and the NOAA \
                  Solar Calculator; the boundary test is equinox noon altitude = 90 deg \
                  minus latitude",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] give SkyPhase the real twilight thresholds of -6, -12 and -18 degrees \
                    plus golden hour, blue hour and the Belt of Venus, and test each at its \
                    exact boundary angle",
        because: "TODO §4 'the events' row — 'a SkyPhase enum with the real thresholds'; a \
                  twilight that is a chosen number rather than the published one is a claim",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] carry every sky constant in a provenance inventory that names its \
                    source, and count the project extensions in a test so an unsourced number \
                    cannot be added silently",
        because: "TODO §4 'every constant carries a SourcedValue-style source' and C-27's \
                  rule; voxel-appearance::provenance already counts its own extensions and \
                  the sky must not be the layer that skips the discipline",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] evaluate sky radiance from the real scattering models so the sky's \
                    colour is the model's output: Rayleigh and Mie coefficients, the \
                    Henyey-Greenstein phase function, and Kasten-Young air mass",
        because: "TODO §4 rows 2 and 3 — Preetham, Shirley & Smits 1999; Bodhaine et al. \
                  1999; Henyey & Greenstein 1941; Kasten & Young 1989. Deriving the \
                  radiance is what keeps TODO §5's reserved area out of this round",
    },
    Work {
        kind: TicketKind::Material,
        objective: "[MAT] give the night a source that is not the sun dimmed: the Moon's real \
                    phase derived from tick at the published Bond albedo, so a night is not \
                    the day with a smaller number",
        because: "TODO §4 'Night' row — Bond albedo 0.12 (NASA); without it the night is a \
                  magnitude this project chose rather than one it derived",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] build the fixed 20 Hz clock with an accumulator that bounds how many \
                    missed ticks one slow frame may catch up on, and count what it dropped",
        because: "H-7.2 and H-8.3 — base 20 Hz with declared stage cadences; C-88.7 records \
                  that the tick does not advance, and an unbounded accumulator would let a \
                  stalled frame fast-forward the world",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] wire the tick into the client so the sun and the sky reach the frame: \
                    the clear colour and the light rig both read the tick, and the sun carries \
                    its own colour, intensity and direction so a shadow can be lit by it",
        because: "C-88.7 'the tick does not advance yet' and the ledger's own note that the \
                  sky is the clear colour, which is transparent and therefore black; the \
                  user's requirement is a fusion renderer whose shadows carry light properties",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule the fusion declaration's constraint: customization of how the \
                    world looks, bounded to the real-world models, with every override \
                    carrying a source the way SourcedReflectance does",
        because: "the user's own answer for the sky — 'a fusion renderer that allows for \
                  customization of how the world looks but keeping constrained to the real \
                  world models'; TODO §5 lists sky and sun colour as the reserved area, so \
                  the bound is a ruling and not a default this round may pick",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] measure the sky offscreen: read the pixels back and show that the \
                    clear colour and the rig both change with the tick, so 'the sky is lit' \
                    is a number rather than a claim",
        because: "C-40 'never test the client invisibly' and C-34's precedent — the band \
                  count was only real once a frame carried it; a sky nobody measured is \
                  C-38 waiting to happen again",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule how the sun and the moon are bodies: a sky pass drawing each \
                    disc from its angular radius, the moon as real voxels at a scaled \
                    distance, or both as voxels with a declared compression ratio",
        because: "the user's own answer — 'file the ticket'; sky.rs builds both bodies as \
                  DIRECTIONAL LIGHT ONLY (a SolarPosition and a W/m^2 figure), so neither \
                  is drawn. Three findings block a voxels answer and each must be read \
                  before one is chosen: (a) the sun at 1 AU is 1.5e11 m and the \
                  coordinate space is signed i32 voxels over +/-2e6 m (C-6), so a voxel \
                  sun overflows at ANY level; (b) no material can emit — openpbr.rs \
                  derives emission as 'nothing in the model can declare an emitter, so no \
                  material emits', so a voxel sun has no material to be; (c) C-2 reads the \
                  256 lattice as an addressed SURFACE displacement and refine subdivides a \
                  quad, so a sphere's silhouette is a different primitive, not a \
                  displacement",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule whether the gas and fluid model is in this branch, because \
                    machines and extra subvoxel shapers depend on what a fluid is and \
                    nothing has decided it",
        because: "the user's own answer, in the same breath as the sun — 'there is also a \
                  gas and fluidics model that hasn't been decided for this branch as well \
                  as machines which determines extra subvoxel shapers'; hydrology models \
                  water and substrate models soil, so the fluid substrate is what stands \
                  between the roster and any machine that moves a gas",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] derive the extra subvoxel shapers from the gas and fluid ruling \
                    rather than inventing them, so a machine's behaviour is a consequence \
                    of what a fluid is and not a list of shapes",
        because: "the user's answer — 'machines which determines extra subvoxel shapers'; \
                  §2.4's roster is the shaper list and a machine is not a member of it, \
                  so this is the gap that roster does not cover",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule air as a CONSERVED TYPED FLUID in the world's own ledger rather \
                    than as an analytic formula, and make the sky derive from that authority \
                    rather than beside it",
        because: "the reference record, read this round on the user's instruction ('check the \
                  target reference docs for the complete idea of the gaseous model'). GRILLING-C11.md:5234 \
                  Q678 / WG-Q17 'Split authority': atmospheric composition/pressure/weather may be \
                  authoritative WHEN IT AFFECTS SIMULATION, decorative skybox may remain \
                  presentation, and the exact split is CAP-001/008/009 work. GRILLING-C11.md:5312 \
                  Q686 / WG-Q25 'Atmospheric state authoritative': pressure, temperature, humidity, \
                  precipitation and wind 'are world facts; skybox/star presentation derives from \
                  them where applicable'. Q679/WG-Q18 'Typed fluid system with water, oils, lava, and \
                  gaseous contents'; Q685/WG-Q24 'Conserved typed fluid state' — water, oils, lava and \
                  GASES carry quantity/volume, pressure, phase, flow, sources/sinks and LEDGER \
                  PARTICIPATION, with explicit phase transitions. THIS TREE ALREADY HAS THE ACCOUNT: \
                  voxel_model::conservation::Source::Atmosphere is a first-class destination \
                  ('F3: damage adds mass from a...'), so the ledger is in place and the atmosphere is \
                  not in it. C-48 makes the old trees reference, never authority, so the shape is \
                  evidence and the user asking for it is what carries it in.",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] delete the second air column: one function answers what a light ray's \
                    path loses, and both the noon normaliser and the sun's own colour read it",
        because: "MEASURED THIS ROUND, and it is the defect LEDGER C-88 named in another form. \
                  voxel_shapers::sky::direct_irradiance_w_m2 is `1361 * exp(-0.1083 * air_mass)` — \
                  band-averaged Rayleigh, no ozone, no Mie, no per-channel — while \
                  voxel_appearance::sky::sun_light is per-channel Bodhaine beta x scale height x \
                  air mass x Mie at T=2 x ozone. voxel-client::skylight then normalises one by the \
                  other, and the noon intensity measured 0.9007 instead of 1.0. The right fix is \
                  one air column, not a tolerance: a test asserting 'within 0.85..1.0' would have \
                  hidden a 10% disagreement between two physical models.",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] make the moon a REFLECTANCE on a lit surface — solar constant at its \
                    orbit, Bond albedo as a SourcedReflectance, attenuated back through the same \
                    air column — instead of an independent emitter",
        because: "the user's correction, 'the moons light is actually the light reflected off the \
                  sun'. The current sky.rs models it as a second light source of its own, which is \
                  wrong in kind: the Moon is a lit surface and belongs on the same reflectance path \
                  every other material uses (C-39's Reflectance), so its colour is the sun's own \
                  colour reflected rather than a third constant. Ordered AFTER the air column, by \
                  the user's own ruling, because both depend on the shared path.",
    },

    // ---- Read this round out of the old projects' own ledgers ----
    //
    // Five, and each one is a gap the other trees' records *name* that this tree
    // has not measured. They carry the ledger id behind them, because the old
    // trees' practice is that a decision is bound to the record that holds it,
    // and a gap found by reading is worth as much as one found by running.

    Work {
        kind: TicketKind::Case,
        objective: "[CSE] give every decision in this record a status and a hazard beside it, and \
                    a withdrawal record that supersedes a wrong claim by id — so a settled \
                    decision carries what could still break it",
        because: "MEASURED in both old trees' ledgers and absent here. \
                  C:/Users/ala/Documents/.superpowers/hypergrilling/2026-09-25-modding-ledger-tech/ledger.jsonl \
                  uses settled | settled_with_hazard | settled_with_spec_tension | contested | \
                  withdrawn | blocking_finding, and 0110 carries rule AND hazard AND required \
                  constraint in one entry; 0194 is filed as 'Written but NOT YET VERIFIED' and \
                  0196 withdraws it by id with five probed inputs. docs/LEDGER.md holds ZERO of \
                  those status words across 6002 lines, so a reader here cannot tell a settled \
                  decision from one with a live hazard. Their METHOD is worth carrying; their \
                  VOLUME is not.",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule that a structure is a NAMED LEDGER-RESIDENT OBJECT with an owner \
                    rather than a gesture over voxels, because 'the machine moved' is false \
                    otherwise",
        because: "the old trees' ledger 0224 Q23: 'a structure is a NAMED, LEDGER-RESIDENT \
                  OBJECT with an owner, not a gesture over voxels', the reason being that without \
                  it the claim that a machine moved is a falsehood. Their 0216 Q6 makes fluid \
                  matter-in-containers for the same reason. This tree has no structure type at \
                  all: `impl Shaper for` is exhaustive across voxel-shapers as Slab, LoopCut, \
                  Column, Boulder, Flowstone, Coral, and world_source::shape_at still returns \
                  VoxelShape::solid for every voxel.",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] key the surface displacement by (structure, material, face) instead of \
                    (material, face) for the whole world, so two machines may pattern the same \
                    wall without one refusing the other",
        because: "MEASURED. voxel_model::shaper::DisplacementField holds `entries: \
                  Vec<((u16, usize), DisplacementEntry)>` keyed by material and face_slot(face) \
                  for the whole world, so a second displacement on one face REFUSES. The field \
                  is also amplitude-plus-profile only, or two gains per material, so a \
                  per-instance per-position pattern is not expressible at all. That is not a \
                  tuning gap: it is the whole of the machine-surface-pattern ask, and it is a \
                  change to the key rather than a new shaper.",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule that air is SEVERAL NAMED POOLS mixing only through declared \
                    processes, with a stored global account and per-basin accounts cross-checked \
                    at audit — not one global scalar and not one account",
        because: "the old trees' ledger 0224 Q42, 'C. CLOSED, WITH MULTIPLE POOLS - fresh, salt, \
                  groundwater - mixing only through named processes', forced by 0216 Q6 (fluid \
                  is conserved matter) plus 0123 (the baseline atmosphere is sourced FROM the \
                  water account, making that account the root), with the stated consequence that a \
                  SINGLE pool makes fresh water a finite resource that can be exhausted. Their \
                  0104 and 0106 then keep BOTH a stored global account and per-basin exact \
                  accounts and check each basin against the global at audit, treating divergence \
                  as a recorded conflict. This changes the shape INT-0020 assumed.",
    },
    Work {
        kind: TicketKind::Material,
        objective: "[MAT] derive soil porosity HERE from a named source, or refuse every porosity \
                    this tree states — because the only number available is another tree's \
                    provisional row with its own tolerance undefined",
        because: "the reference's soil is subvoxel and porosity-driven: composition.rs:140-200 \
                  `Layer { ..., porosity, fluid_capacity, ... }` refusing \
                  FluidCapacityExceedsPorosity when fluid_capacity > volume * porosity, and \
                  world/substrate.rs:575-578 computing pore volume as voxel_volume * \
                  source.porosity(). But its only porosity ROW is generated — EARTH-GEO-04 = \
                  [5.0, 8.0, 15.0] '% porosity', Lifecycle::Provisional, tolerance 'not defined; \
                  human review required'. That is a number this tree may not cite, and C-48 makes \
                  it reference on one subject only: why a choice was made.",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule the exact/approximate boundary for a fluid: mass and volume as \
                    exact rationals, pressure and temperature and flow as declared-approximate \
                    with a named scale — and forbid a tolerance-bearing quantity feeding a \
                    conserved one",
        because: "the old trees' ledger 0082 draws the line explicitly. EXACT, never approximate: \
                  mass, volume, component lineage, fluid volume and mass, lifecycle counters. \
                  DECLARED APPROXIMATE: temperature, pressure and flow, process rates and \
                  cadences, weather/atmosphere/climate, ecology populations, regional field \
                  interpolation. Its own risk field is the load-bearing part — 'if \
                  tolerance-bearing quantities ever feed mass or volume, the audited-to-the-gram \
                  claim silently weakens. Every tolerance must be proven not to propagate into \
                  conserved quantities.' CORRECTION, recorded against my own first filing of \
                  this ticket: the boundary IS enforced here, not absent. \
                  voxel_model::conservation::Mass::from_approximate refuses an Approximate with no \
                  declared tolerance, refuses a zero tolerance as 'a zero tolerance is an exact \
                  claim about an inexact value', and rounds at the declared one (1234.7 at Ratio \
                  1/100 becomes exactly 1235, with a comment that a second undeclared \
                  approximation must not creep in). What is missing is the DECLARED ROSTER — which \
                  named quantity sits on which side — and the scale for the approximate side. The \
                  ticket stands for those two, not for the enforcement.",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] carry pressure, temperature and flow as fixed-point integers at a \
                    DECLARED scale, so a threshold test is an exact integer comparison, and bind \
                    the scale into identity so changing it refuses an old save",
        because: "the old trees' ledger 0092, 'exact_test_approximate_field': a field is stored as a \
                  declared-scale fixed-point integer, so 'field >= threshold' is an exact integer \
                  comparison and body membership is exactly repeatable. The declared tolerance \
                  covers how the stored value approximates the continuum, NEVER whether the test \
                  is repeatable — that removes the replay hazard, because an approximate threshold \
                  could classify the same cell differently across runs. Their 0087 then binds every \
                  declared tolerance into identity, so changing a scale orphans old saves by \
                  refusing them, which is the intended behaviour. This tree's atmosphere is \
                  currently f64 in kPa (voxel_shapers::atmosphere) with no such line drawn.",
    },

    // ---- Read this round out of shploingus-slice1, the second ledger ----

    Work {
        kind: TicketKind::Build,
        objective: "[BLD] add RecordState::Contested for the case where the ACTION is settled and \
                    a CONSEQUENCE is owed — because Unresolved says no answer was given and \
                    Superseded says a newer one replaced it, and neither is what happened",
        because: "MEASURED, and it is the gap the other tree's grammar has and this one's does \
                  not. \
                  C:/Users/ala/Documents/.superpowers/hypergrilling/shploingus-slice1/ledger.jsonl \
                  event-0005 supersedes event-0002 and is itself state=contested, with its own \
                  reason: 'This event is CONTESTED rather than answered because it withdraws a \
                  decision recorded two events earlier without itself settling the replacement. \
                  The world re-grill is the settling event.' event-0014 is the same shape: \
                  state_is_contested, 'The revert is settled, but the replacement target is NOT', \
                  with an `owed` field naming what must be decided. This tree's \
                  voxel_model::envelope::RecordState is Answered | Unresolved | Superseded (ALL, \
                  3, count asserted), so this state has nowhere to live and a settled action with \
                  an owed consequence would be filed as Unresolved, which claims no answer \
                  exists. This also sharpens CSE-0023: the status must be a closed enum on the \
                  fixed record, not a free-text field.",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule that withdrawing a decision does NOT refute what it described, and \
                    carry a flag saying so — or a later reader will 'fix' a seam that was left \
                    open on purpose",
        because: "shploingus-slice1 event-0003, measured. The agent found C10's adaptability \
                  complaint still literally true — BuildingKind is a closed 4-variant enum at \
                  src/sim/mod.rs:280, name() and part_prefix() both match all four with no \
                  wildcard arm, and src/materials/effects.rs:296 iterates the literal \
                  [\"home\", \"shop\", \"factory\", \"power\"] — and the user answered OUTDATED \
                  rather than confirming. The event records it as 'a withdrawal, not a \
                  refutation — the enum IS closed, and the decision is that closing it is not a \
                  problem to solve. Any future lane that opens BuildingKind must cite this event \
                  and say why the withdrawal no longer applies', and carries is_flagged / \
                  why_flagged: 'A closed 4-variant enum that every structure must be added to by \
                  hand is exactly the seam a package system has to cross... a later agent reading \
                  the code will see the seam and assume it is a bug.'",
    },
    Work {
        kind: TicketKind::Case,
        objective: "[CSE] record a decision the tooling could not write as a NAMED GAP with its \
                    count left unreconciled, rather than leaving a silence a later session would \
                    be free to fill with an invention",
        because: "shploingus-slice1 event-0018's prior_recording_defect, measured. The agent lost \
                  shell and file access when the tool catalog changed; the decisions were MADE \
                  and could not be filed, surviving only in an untracked handoff note. The repair \
                  event says: 'A session reading only the ledger would have seen event-0017 stop \
                  at \"the six tabs are the windows\" with nothing about arrangement, and would \
                  have been free to invent one.' It also refuses to reconcile a count: the \
                  handoff says 'the six above' and lists four, so four are recorded and 'if two \
                  further rulings were made in that round they are not in the handoff and must \
                  be re-asked rather than inferred.' This tree's docs/LEDGER.md is 6002 lines of \
                  hand-written prose outside the hash-linked chain, so the same failure mode is \
                  structurally possible here.",
    },

    // ---- The three the air ruling needs before any of it can be written ----
    //
    // Filed from the research the user asked for ('research required'), after
    // ruling that an atmosphere which is not in the voxels is not a world fact.
    // That refusal eliminates every other home for conserved per-place state and
    // leaves the subvoxel cell array — which is missing two small things.

    Work {
        kind: TicketKind::Build,
        objective: "[BLD] declare the voxel-to-metre scale as one constant with a named source, \
                    and bind it into identity — because the world states a density in kg/m^3 and \
                    holds a lattice in voxels with no declared number between them",
        because: "MEASURED. crates/voxel-model/src/coord.rs:27,29,33 declare SUBVOXEL_SIDE = 4, \
                  SUBVOXEL_COUNT = 64 and DETAIL_SIDE = 256, and no metres-per-block constant \
                  exists in voxel-model at all. Meanwhile \
                  crates/voxel-shapers/src/atmosphere.rs states SEA_LEVEL_DENSITY_KG_M3 = 1.225 \
                  and SCALE_HEIGHT_M = 8500.0. A density with no length cannot yield a volume, so \
                  no conserved quantity can be computed from a cell. BLD-0029's ruling — derive \
                  each scale from its unit — cannot even be applied until this unit exists. \
                  Identity-bound per their 0087, because changing it changes every derived mass.",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] give a subvoxel cell a DECLARED EXACT VOLUME, so a cell's mass is \
                    volume times bulk density as an exact rational and a cell can hold matter",
        because: "MEASURED. crates/voxel-model/src/subvoxel.rs holds ZERO occurrences of 'volume' \
                  or 'Volume'; VoxelShape is `cells: [MaterialId; SUBVOXEL_COUNT]` and nothing \
                  else, and `VoxelShape::solid(m)` fills 64 ids with no size attached. \
                  Material::bulk_density() is already exactly the right arithmetic — \
                  '= Sum(ratio * density), exactly. kg/m^3 as an exact rational' — so the only \
                  missing term is the cell's own volume. Without it, air cannot be matter in a \
                  cell even after AIR is split from EMPTY, and INT-0026's cells have no mass to \
                  conserve.",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] split materials::AIR from MaterialId::EMPTY so an air mixture is an \
                    admitted Material with gas components and a density, rather than the absence \
                    of matter by another name",
        because: "THE USER'S RULING, and it makes this a blocker rather than a choice: 'an \
                  atmosphere that is not in the voxels is not a world fact.' MEASURED as the \
                  blocker — crates/voxel-shapers/src/column.rs:50-51 reads '/// Absence of matter. \
                  Never admitted; [MaterialId::EMPTY] by another name' and \
                  `pub const AIR: MaterialId = MaterialId(0);` while material.rs:212 makes \
                  `MaterialId::EMPTY = 0` 'The absence of matter. Never admitted, never valid as a \
                  component'. Two different meanings on one id, and the doc already calls it out. \
                  The mixture is representable today: Material takes a Vec<Component> with an \
                  exact Ratio and an exact kg/m^3 density, and MaterialId::EMPTY stays 0 as the \
                  genuine absence.",
    },

    // ---- From the 54 events of the longer persisted slice1 ledger ----
    //
    // Read at C:/Users/ala/Documents/shploingus/ledger/shploingus-slice1.jsonl,
    // 76 events, 364,611 bytes. Only 22 of those were in the hypergrilling copy.

    Work {
        kind: TicketKind::Build,
        objective: "[BLD] admit the baseline atmosphere as TICK-ZERO DATA with a source per number, \
                    instead of four f64 constants in a shaper — because that admission is the \
                    exact thing the other tree is blocked without",
        because: "shploingus/ledger/shploingus-slice1.jsonl event-0052 (F7, climate, atmosphere, \
                  weather, biomes) is state=unresolved and says so in its own words: 'No admitted \
                  baseline atmosphere exists as data, so the tick-zero starting state that the \
                  admitted-baseline decision requires is not present.' The admission it needs was \
                  ruled at their 0123: 'An admitted baseline atmosphere is declared as tick-zero \
                  data and sourced from the closed water account, so the first weather process has \
                  a real starting state rather than nothing to act on.' THIS TREE ALREADY HOLDS \
                  THE NUMBERS — crates/voxel-shapers/src/atmosphere.rs declares \
                  SEA_LEVEL_PRESSURE_KPA = 101.325, SEA_LEVEL_DENSITY_KG_M3 = 1.225, \
                  SCALE_HEIGHT_M = 8500.0 and GRAVITY_M_S2 = 9.806_65 — as f64 in a shaper, with \
                  no admission and no source. Admitting them is the whole of the difference \
                  between the two trees on F7.",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule that climate is NOT a regional field — temperature, moisture, \
                    precipitation, humidity, pressure, wind and storms are PROCESSES — and that \
                    the field ladder therefore carries lithology, elevation and soil depth only",
        because: "Their ledger 0118, indexed at shploingus-slice1 event-0052 with state=unresolved: \
                  'Climate is NOT a regional field. Temperature, moisture, precipitation, \
                  humidity, pressure, wind and storms are all processes, and climate emerges over \
                  time.' The recorded major consequence is that the field ladder SHRINKS, and \
                  their 0162 required a spec amendment to strike the climate field families — \
                  which event-0052 records as still not applied, so 'the project has been running \
                  against a document that contradicts a settled decision.' This tree has \
                  crates/voxel-shapers/src/fields.rs and the ladder, so the same question is open \
                  here and the same amendment is owed rather than inherited silently.",
    },

    // ---- From events 23-46 of the longer persisted slice1 ledger ----
    //
    // The three that govern work already in flight in this tree. Each is a
    // measured instance of a defect class this tree has a rule about and has
    // not yet tested for.

    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule that a world with no ADMITTED initial atmospheric state must REFUSE \
                    its first weather process and name the missing declaration — never begin at a \
                    standard pressure, however well sourced that constant is",
        because: "shploingus/ledger/shploingus-slice1.jsonl event-0038, which carries forward the \
                  ancestor's 0118 (climate is all processes, so a world at tick zero has no \
                  climate) and rules: 'A world at tick zero with no declared initial atmospheric \
                  state must REFUSE to run its first weather process and say which state is \
                  missing, rather than quietly beginning at some standard pressure. The refusal \
                  names the absent declaration the way a horizon out of bounds names the number it \
                  refused.' It records the rejected alternative and why it is wrong: 'a process \
                  with nothing to act on does not converge on its own, and nothing in the \
                  implementation would notice.' THIS TREE HAS THE CONSTANT ALREADY — \
                  atmosphere.rs declares SEA_LEVEL_PRESSURE_KPA = 101.325 — so the temptation is \
                  exactly the one the event refuses. BLD-0036 is the same ruling from the other \
                  side: the number is admissible as tick-zero DATA or it is a default.",
    },
    Work {
        kind: TicketKind::Case,
        objective: "[CSE] prove every gate reading can go RED — a mutation check on the values the \
                    gate measures, because a suite that cannot go red is not evidence and a \
                    number asserted in isolation can be a number that goes nowhere",
        because: "Two measured instances in the same ledger. event-0036: tests asserted \
                  `shell.blur_strength() > 0.0` and `== 0.0` when closed, and BOTH PASSED — \
                  because both assert the SHELL's number and no test observes that number arriving \
                  at the shader. 'BLUR_STRENGTH of 0.82 is a perfectly good number that goes \
                  nowhere.' The blur chain existed, compiled, bound, and was disabled at runtime by \
                  its own per-frame uniform write. It was found 'by a person playing the thing for \
                  seven minutes and saying there was no blur', and the event records that this is \
                  'the first instance in this project of the reverse also being true, where a \
                  measurement was not a sighting'. event-0030: a mutation deleted the refusal-colour \
                  path and all 192 tests still passed; the cause was that the mesher lived in the \
                  binary and so had no tests, and moving it out found a real bug — culling gated on \
                  definition.solid meant 'WATER WAS IN THE WORLD AND INVISIBLE'. BLD-0016's pixel \
                  readback is the right instrument and is not sufficient without this.",
    },
    Work {
        kind: TicketKind::Intervention,
        objective: "[INT] rule whether this tree adopts the octree-aligned field ladder — a rung \
                    INDEX (level n = 32 * 2^n, octree-aligned to the 32^3 chunk, level count the \
                    only free parameter) carrying lithology, elevation and soil depth only — or \
                    states its own shape and records the divergence",
        because: "shploingus/ledger/shploingus-slice1.jsonl event-0037 rejects a declared \
                  per-family cell size as the resolution model and rules the ladder shape is not a \
                  free parameter, then records the sibling tree DIVERGING: eight field families \
                  including Climate, Groundwater and Atmosphere, three of which event 0118 struck, \
                  and no ladder at all. ITS INSTRUCTION IS THE RULE WORTH CARRYING: record the \
                  divergence so a later session reading a green crate knows green was never \
                  evidence of conformance. MEASURED IN THIS TREE, AND THE ANSWER IS THAT IT \
                  ALREADY CONFORMS — so this ticket is no longer a question and is kept only as \
                  the standing instruction to check, not to assume. \
                  crates/voxel-shapers/src/fields.rs declares STANDARD_LEVELS = 20, \
                  blocks_at_level(level) as CHUNK_SIDE << level with the comment 'aligned to the \
                  octree', Level::lattice_of and Level::world_of, and FieldKind::ALL of exactly \
                  THREE — Lithology, Elevation, SoilDepth — not eight. FieldSpec carries a \
                  declared unit(), a provenance string, an exact Option<(u64,u64)> tolerance \
                  whose None is documented as 'makes no tolerance claim, which is different from a \
                  tolerance of zero', and identity() over 'everything that can change a value', \
                  with rebind_required refusing rather than re-deriving. What it does NOT do is \
                  declare a per-family resolution the way event-0064's FieldContract does: the \
                  level count is on the ladder, not on the family. That is the live remainder of \
                  this ticket, and it is                  the sibling divergence that event-0064 sharpens.",
    },
    Work {
        kind: TicketKind::Build,
        objective: "[BLD] add a behavioural census axis whose cell is a NAMED TEST PATH the coverage \
                    matrix forces every crate to fill — derived from this record format, not from \
                    the old tree's per-part claim gate",
        because: "DERIVED FROM THE v3 FORMAT, on the user's instruction that the old material is \
                  reference and the axis is re-derived. MEASURED: \
                  voxel_model::chain::Event is { index, namespace, kind, detail, previous } and \
                  compute_digest(index, &namespace, kind, &detail, head) puts the DETAIL INSIDE \
                  THE DIGEST; Event::append already refuses an empty detail ('a {} event with an \
                  empty detail cannot be read back, so it records nothing'); Envelope::check \
                  refuses an Unresolved record carrying an answer and a Superseded record \
                  superseding nothing; and bloat.rs measures Chain::Event::detail against the \
                  fixed frame. SO A WITNESS HERE IS NOT A FIELD — it is what the detail names, \
                  and because the detail is hashed, naming a different test yields a DIFFERENT \
                  RECORD. That is strictly stronger than the reference's per-part gate, which \
                  catches a part resolving to no declared material but does nothing about a claim \
                  being re-pointed at a different witness. C-70.1 already measured the guard this \
                  builds on: adding an axis alone is GREEN, adding it to AXES with the count \
                  bumped is REFUSED at build time naming every unfilled cell — 'the guard is the \
                  coverage cell, not the enum'. REFERENCE, NOT AUTHORITY, and specifically not \
                  copied: ala-cities gates per PART because a part is its unit of claim \
                  (materials::world::Part carries a note whose doc reads 'Why this part is this \
                  material, so the claim can be argued with'), and a part is not a thing this \
                  record format has.",
    },
];

#[test]
fn the_work_is_filed_as_tickets_and_read_back_off_the_board() {
    let mut workspace = std::env::temp_dir();
    workspace.push(format!("shploingus-file-work-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&workspace);

    let namespace = namespace();
    let mut bytes: Vec<u8> = Vec::new();
    for entry in &WORK {
        // **Through the chokepoint**, so the tickets form a hash-linked chain rather than a
        // run of records that each deny knowing the one before.
        bytes = filing::linked_records(
            &workspace,
            &bytes,
            [UnlinkedRecord {
                namespace: &namespace,
                event_kind: EventKind::TicketTransition,
                question: entry.objective,
                answer: None,
                kind: Kind::Authority,
                event_type: EventType::UserDecision,
                state: RecordState::Answered,
                control: Control::Continue,
                actor: "user",
                round_id: "c-2026-10-01",
                supersedes: None,
            }],
        )
        .expect("links");
    }
    filing::append(&workspace, &bytes).expect("files");

    let chain = filing::read(&workspace);
    let board = Board::from_chain(&chain);

    // **The two views agree, which is the point of filing rather than writing a list.**
    assert_eq!(chain.len(), WORK.len(), "one event per piece of work");
    assert_eq!(board.len(), WORK.len(), "and one ticket per event on the board");
    assert!(
        chain.tamper_evident_description().starts_with("hash-linked"),
        "**the tickets are a tamper-evident record**, not a list that can be edited: {}",
        chain.tamper_evident_description()
    );

    println!("\n# the board, read off the chain\n");
    let mut by_kind: Vec<(TicketKind, usize)> = Vec::new();
    for ticket in board.tickets() {
        if !by_kind.iter().any(|(kind, _)| *kind == ticket.kind) {
            by_kind.push((ticket.kind, 0));
        }
        by_kind.iter_mut().find(|(kind, _)| *kind == ticket.kind).expect("just added").1 += 1;
    }
    for (kind, count) in &by_kind {
        println!("{} filed\t{}", kind.name(), count);
    }
    println!();
    for (index, ticket) in board.tickets().iter().enumerate() {
        let work = &WORK[index];
        println!("{}\t{}", ticket.id.as_str(), ticket.objective);
        println!("\tbecause: {}", work.because);
    }

    // **Every kind used is a real kind**, so a typo in `[BLD]` cannot invent a prefix — the
    // board would fall back to `Build` and the objective would silently lose its marker.
    for entry in &WORK {
        // **`[BLD]` is five characters, and the first version sliced four** — so the check
        // compared `"BLD"` against `"[BLD]"` and failed on every row while proving nothing. The
        // marker is built from the kind and compared whole, which is the only way the check
        // can distinguish a correct marker from a wrong one.
        let marker = format!("[{}]", entry.kind.prefix());
        assert!(
            entry.objective.starts_with(&marker),
            "{} does not open with {marker:?}, so `split_kind` would not read the kind and the \
             objective would be filed as a Build with the marker still in its text",
            entry.objective
        );
    }

    // **And the objectives are not empty, because a ticket with no objective is a ticket
    // nobody can act on** — `split_kind` defaults to `Build` with the whole detail as the
    // objective, so a blank one still yields a ticket.
    for entry in &WORK {
        assert!(
            entry.objective.trim_start_matches(&format!("[{}] ", entry.kind.prefix())).len() > 20,
            "{} has no substance after its marker",
            entry.objective
        );
    }

    let _ = std::fs::remove_dir_all(&workspace);
}

/// How many work items this file declares. **The board's count, not a literal** — the same
/// lesson the migration's tests learned the hard way when C-89 made a pinned 54 stale.
fn count_of_work() -> usize {
    WORK.len()
}

/// **The user's ticket, 2026-09-30. Three parts, because they are three defects and one
/// of them is not this tree's to fix alone.**
///
/// # 1. Window text overlaps and does not respect hierarchy
///
/// **Not diagnosed here, deliberately.** C-40 is standing and it says *never test the client
/// invisibly*, so a cause named from reading layout code would be exactly the "a plausible
/// substitute for a missing reading" the record refuses. This is filed as an observation with
/// a reproduction, not as a diagnosis with a fix attached.
///
/// # 2. Systems still missing, and the reason the user gives
///
/// *"...because of refusal to study the entire ledger through the system and the laya judge
/// oracle and decision model."* Recorded as the user's stated reason — **carried as a claim,
/// not paraphrased into agreement.** It is a claim about conduct and it stays one.
///
/// The measurable part, which is not a matter of opinion: **`oracle-is-laya no` is a string
/// literal.** `crates/shploingus/build.rs:140-141`:
///
/// ```text
/// report.push_str("oracle-kind\tstructural-deterministic\n");
/// report.push_str("oracle-is-laya\tno\n");
/// ```
///
/// So the gate does not measure whether the oracle is Laya — **it prints that it is not.** A
/// row that could read either way and cannot is a report about nothing, which is the same
/// defect D107 names when a count of zero stands in for a measurement that did not run. This
/// is the one item on the ticket this repository can close without a ruling, and the other
/// two depend on it.
///
/// # 3. `axis.rs` for package and crate grading in the mods API view
///
/// The user's own context, recorded: they hypergrilled making Laya also the oracle **because
/// `axis.rs` is needed to display package and crate grading in the mods API view**, in the
/// half-complete re-derivation of `portal_simulator.py`.
///
/// What this tree holds, measured: **not `axis.rs`, its pattern.** `census.rs` cites the
/// reference path and carries the shape — `Axis` (five variants), `COVERAGE` transposed as
/// `[Axis; AXIS_COUNT]` per crate, and `check_coverage` as *"the runtime anti-fabrication
/// guard"*. C-70.1 measured the guard's real edge: adding an axis to `AXES` with `AXIS_COUNT`
/// bumped is **refused at build time** naming every unfilled cell, so the guard is the
/// coverage cell and not the enum. `apidoc.rs` renders the API view from declarations and has
/// no grades to show.
#[test]
#[ignore = "files the user's three-part ticket onto the canonical record; run it deliberately"]
fn file_the_window_and_oracle_ticket_onto_the_record() {
    let root = workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root");
    assert!(filing::is_filed(&root), "the record must be filed first");

    if filing::read(&root)
        .fold(Vec::new(), |mut acc: Vec<String>, event| {
            if event.detail.contains("window text overlaps") {
                acc.push(event.detail.clone());
            }
            acc
        })
        .is_empty()
    {
        let before = std::fs::read(filing::chain_path(&root)).expect("the record reads");
        let namespace = namespace();
        filing::grow(
            &root,
            &before,
            [UnlinkedRecord {
                namespace: &namespace,
                event_kind: EventKind::TicketTransition,
                question: "[BLD] window text overlaps without respecting hierarchy; oracle-is-laya \
                            is printed rather than measured; axis.rs grading is needed for the \
                            mods API view",
                answer: Some(
                    "USER'S TICKET, three parts. (1) Window text overlaps and ignores hierarchy — \
                     filed as an observation, not a diagnosis: C-40 forbids testing the client \
                     invisibly, so naming a cause from layout code would be a plausible \
                     substitute for a missing reading. (2) The user's stated reason for the \
                     missing systems is a refusal to study the entire ledger through the system \
                     and the laya judge oracle and decision model — carried as the user's claim, \
                     not paraphrased into agreement. The measurable part: oracle-is-laya no is a \
                     STRING LITERAL at build.rs:141, alongside oracle-kind \
                     structural-deterministic at :140. The gate prints that the oracle is not \
                     Laya rather than measuring it, so a row that could read either way cannot \
                     — D107's absence rendered as absence. (3) axis.rs is needed to display \
                     package and crate grading in the mods API view, per the user's hypergrilled \
                     decision to make Laya also the oracle, in the half-complete re-derivation \
                     of portal_simulator.py. This tree holds axis.rs's PATTERN, not the file: \
                     census.rs carries Axis (5), COVERAGE as [Axis; AXIS_COUNT] per crate, and \
                     check_coverage as the anti-fabrication guard — and C-70.1 measured that the \
                     guard is the coverage cell, not the enum. apidoc.rs renders the API view \
                     and has no grades to show.",
                ),
                kind: Kind::Evidence,
                event_type: EventType::BoundaryRuling,
                state: RecordState::Unresolved,
                control: Control::OneMoreRound,
                actor: "user",
                round_id: "c-2026-10-01",
                supersedes: None,
            }],
        )
        .expect("the window and oracle ticket is filed");
    }

    let board = Board::from_chain(&filing::read(&root));
    assert!(
        board.tickets().iter().any(|t| t.objective.contains("window text overlaps")),
        "**the ticket is on the record**, not only in a conversation"
    );
}

/// **Filed because it was built first and filed second, which is backwards.**
///
/// # The defect, measured
///
/// `voxel_shapers::stages::Declared` is the **only authority-bearing object in the world
/// model** — it decides which of the sixteen stages run, and the stages are the only things
/// in this repository that change the world. It was `Declared { stages: BTreeSet<Stage> }`
/// with `full()` and `closure_of(..)` as public constructors, so any caller including a test
/// could declare a world with no actor, no scope, no state and nothing recorded. **A
/// read-only action needs an attributed grant; writing the terrain needs none** — because
/// `voxel_package::policy::permission_of` governs every `Action`, and a stage is not an
/// `Action`.
///
/// # Why the obvious fix was wrong, and this ticket exists because of it
///
/// The first attempt added an `Authority` enum to `Declared` with a `Granted { granter,
/// scope, anchor }` variant. **That is a second answer to a question `policy::Grant` already
/// answers** — `Grant { granter, scope }`, `Grant::new` refusing both on construction with
/// `RefusalKind::FabricatedFeature`. So the attempt was D-07 twice over: two names for one
/// thing, and two different refusal kinds for the same failure, so a caller filtering by
/// kind gets different answers depending on which type it asked. It also added an `anchor`
/// field with no source. **Reverted byte-for-byte; `voxel-shapers --lib` is back to 177.**
///
/// # The real blocker, which is a dependency direction
///
/// `Cargo.toml`: `voxel-package` depends on `voxel-shapers`. So **`voxel-shapers` cannot see
/// `policy::Grant` at all.** The duplication was not carelessness alone — the grant type is
/// in the wrong crate for the thing that needs granting, and that is a design decision about
/// where a shared governance type lives. It was made unilaterally, which is the part that was
/// actually wrong.
///
/// # What the reference says, and it was already in the tree
///
/// `voxel-model/src/ticket.rs` cites `ala-cities::gov` four times by name: `TicketStatus`
/// (nine), `TicketKind` (four), `RetirementReason` (fourteen), `Evidence` (three). **That is
/// the governance model, already imported by citation, and the `Authority` enum was a fourth
/// invented shape beside it.** Per C-48/C-49 the reference rules on one subject only — *why a
/// choice was made* — so the shape is re-derived here, but it is not invented beside a
/// citation that already names where it came from.
///
/// # Open, and deliberately not decided here
///
/// 1. Does a stage belong in `policy`'s `Record<Action, PermissionClass>` matrix, or does the
///    `Declared` set carry the authority and the stages stay ungated? F12 says the world is
///    pure in `(tick, address)`, so authority must not reach the computation; the two are
///    separable but the seam has to be ruled, not chosen.
/// 2. If authority travels on `Declared`, the grant type must move to `voxel-model` so both
///    crates can see one type. **That is a change to where a governance type lives.**
/// 3. Whether an unattributed world is *refused* or *reported*. Refusing breaks twenty-five
///    call sites; reporting is D107's absence-rendered-as-absence. The record has not said.
#[test]
#[ignore = "files one governance ticket onto the canonical record; run it deliberately, once"]
fn file_the_governance_ticket_onto_the_record() {
    let root = workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root");
    assert!(filing::is_filed(&root), "the record must be filed first");

    let objective = "[INT] give the shaper stage set a declared authority, reusing the existing \
                    governance vocabulary rather than a second one";
    if filing::read(&root)
        .fold(Vec::new(), |mut acc: Vec<String>, event| {
            if event.detail.contains("give the shaper stage set a declared authority") {
                acc.push(event.detail.clone());
            }
            acc
        })
        .is_empty()
    {
        let before = std::fs::read(filing::chain_path(&root)).expect("the record reads");
        let namespace = namespace();
        filing::grow(
            &root,
            &before,
            [UnlinkedRecord {
                namespace: &namespace,
                event_kind: EventKind::TicketTransition,
                question: objective,
                answer: Some(
                    "Declared is the only authority-bearing object in the world model and the \
                     only thing constructible with no author: every Action passes policy's \
                     permission_of, and a stage is not an Action. The first fix added an \
                     Authority{granter,scope,anchor} beside policy::Grant{granter,scope} — \
                     D-07 twice, two names and two refusal kinds for one failure — and was \
                     reverted (voxel-shapers --lib back to 177). The blocker is the dependency \
                     direction: voxel-package depends on voxel-shapers, so voxel-shapers cannot \
                     see Grant. Reference already in the tree: ticket.rs cites ala-cities::gov \
                     four times (TicketStatus, TicketKind, RetirementReason, Evidence). Open: \
                     does a stage enter the permission matrix or does Declared carry the \
                     authority; if the latter the grant type moves to voxel-model; and is an \
                     unattributed world refused or reported.",
                ),
                kind: Kind::UnresolvedAmbiguity,
                event_type: EventType::ChildEscalation,
                state: RecordState::Unresolved,
                control: Control::Stop,
                actor: "agent-migration",
                round_id: "c-2026-10-01",
                supersedes: None,
            }],
        )
        .expect("the governance ticket is filed");
    }

    let board = Board::from_chain(&filing::read(&root));
    assert!(
        board.tickets().iter().any(|t| t.objective.contains("declared authority")),
        "**the governance question is on the record**, not only in a conversation"
    );
}

/// **File the work tickets onto the real record, not a scratch directory.**
///
/// # Why this exists, and it is the fourth time
///
/// Every other filing test in this file writes to `std::env::temp_dir()` and removes it. That
/// is the defect C-73 opened the whole chain question with — *"Every chain write in this
/// repository went to a temporary directory … so it proved the round trip and then threw the
/// record away"* — and after `first_ticket.rs`, `round_trip.rs` and `filed_record.rs` it was
/// done a fourth time by the migration, whose entry claimed a chain that existed only inside
/// a test's scratch directory.
///
/// **The measurement that made it visible:** with the migration filed, the chain held **56
/// events and 0 tickets**. The 42 real work tickets existed only as `const WORK` in this
/// test file, so the record the system is built on carried history and no work — and
/// `compiler::emit`, which consumes tickets, had nothing to consume.
///
/// # Why it is `#[ignore]`d
///
/// It writes the canonical record. D97 rules Python out and D98 calls a terminal-only finding
/// incomplete, so a person chooses to run it rather than it happening on every build. And it
/// cannot fire twice silently: [`voxel_package::filing::grow`] refuses a `prior` that is not
/// the bytes on disk, and a second run would append 42 duplicates under fresh positions.
/// Run it with: `cargo test --offline -p voxel-package --test file_work -- --ignored`
#[test]
#[ignore = "files the 42 work tickets onto the canonical record; run it deliberately, once"]
fn file_the_work_tickets_onto_the_record() {
    let root = workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root");
    assert!(
        filing::is_filed(&root),
        "the migration must have been filed first — `grow` extends a chain and refuses to \
         create one, and filing the work before the history would put the work at positions 1..42"
    );

    let before = std::fs::read(filing::chain_path(&root)).expect("the record reads");
    let board_before = voxel_model::ticket::Board::from_chain(&filing::read(&root));

    // **Already filed is a state, not a failure — and it is the state this one-shot reaches
    // after it has run once.** The first version asserted the board was empty and failed on
    // the second run with `left: 42`, which read like a defect and was the opposite: the
    // one-shot had done its job, and the refusal to file it twice is
    // `filing::grow`'s prior-bytes check working.
    //
    // The second version compared the board's **length** to 42, and that broke the moment a
    // second ticket was filed onto the same record — the governance ticket took the board to
    // 43 and then the window/oracle ticket to 44, and a one-shot that fails because *other*
    // work was filed is a test measuring the wrong thing. **The question is whether these
    // work items are on the record, not how many tickets there are.**
    //
    // The third version asked that question with `objective[..40].trim()`, and **it could
    // never match** — a guard that cannot succeed, which is worse than no guard because it
    // looks like one. `split_kind` strips the `[BLD] ` marker off the detail before the board
    // ever sees it, so the board holds `" resize a pane: drag a divider…"` while the constant
    // holds `"[BLD] resize a pane: drag a…"`; the marker is in one and not the other, so the
    // `contains` was false for every row, for every run. Three `--ignored` runs therefore filed
    // the 42 work items three times: `128 = 42 × 3 + 2` tickets, and the chain grew from
    // 812 KB to 13.2 MB.
    //
    // **The lesson is not the off-by-marker. It is that an append-only record plus a
    // non-idempotent writer is unrecoverable** — `grow` never truncates, which is its whole
    // safety property, and that is exactly why the duplicates cannot be deleted. The careful
    // property and the careless check composed into a permanent record.
    //
    // So the marker is stripped here **by the board's own rule**, not by a second copy of it:
    // if `split_kind` ever changes, this comparison changes with it instead of drifting.
    let marker = |entry: &Work| format!("[{}] ", entry.kind.prefix());
    let already = WORK
        .iter()
        .filter(|entry| {
            let objective = entry.objective.trim_start_matches(marker(entry).as_str());
            let head: String = objective.chars().take(48).collect();
            board_before.tickets().iter().any(|t| t.objective.contains(head.as_str()))
        })
        .count();
    if already == count_of_work() {
        println!(
            "all {} work tickets are already on the record at {} (board holds {} tickets in \
             total) — nothing to do",
            count_of_work(),
            filing::chain_path(&root).display(),
            board_before.len()
        );
        return;
    }
    assert_eq!(
        already, 0,
        "measured before: {} of the {} work tickets are on the record, which is neither none nor \
         all. The record is in a state this one-shot does not know how to extend, so it refuses \
         rather than appending a partial set.",
        already,
        count_of_work()
    );

    let namespace = namespace();
    let mut records: Vec<voxel_package::compiler::UnlinkedRecord<'_>> = WORK
        .iter()
        .map(|entry| voxel_package::compiler::UnlinkedRecord {
            namespace: &namespace,
            event_kind: EventKind::TicketTransition,
            question: entry.objective,
            answer: Some(entry.because),
            kind: Kind::Authority,
            event_type: EventType::UserDecision,
            state: RecordState::Answered,
            control: Control::Continue,
            actor: "agent-migration",
            round_id: "c-2026-10-01",
            supersedes: None,
        })
        .collect();
    let count = records.len();
    filing::grow(&root, &before, records.drain(..)).expect("the work is filed");

    // **The record grew and kept everything.** `grow` opens the file for append and never for
    // write, so this is a property of the code rather than a check — and the assertion is here
    // because a property nobody checks is a property that can change.
    let after = std::fs::read(filing::chain_path(&root)).expect("the record reads");
    assert!(after.len() > before.len(), "the record grew");
    assert!(after.starts_with(&before), "and nothing already filed was shortened");

    let board = voxel_model::ticket::Board::from_chain(&filing::read(&root));
    assert_eq!(board.len(), count, "**the work is on the record now**, not in a test fixture");
    assert_eq!(
        filing::read(&root).len(),
        56 + count,
        "56 migrated events plus the filed work, all in one chain — a second .chain file would \
         make bloat::read_chain refuse the whole directory"
    );

    // **And the `because` survived.** A ticket whose objective is a bare imperative cannot be
    // checked by a later reader, which is why every row carries the document that declares it.
    // `Board::from_chain` puts the whole detail in the gate statement, so the reason has to be
    // findable there.
    let first = &board.tickets()[0];
    assert!(
        first.objective.contains("requirement") || first.gate.statement.contains("because")
            || first.gate.statement.len() > first.objective.len(),
        "the declaring document must travel with the ticket, or the board answers 'why does this \
         exist' without the ledger: objective={:?} gate={:?}",
        first.objective,
        first.gate.statement
    );
}
