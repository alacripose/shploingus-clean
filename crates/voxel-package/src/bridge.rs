//! **The bridge: the world and the record behind four named capabilities.**
//!
//! # What this is, and where it comes from
//!
//! `planet-shploingus` (the sibling tree) carries an MCP surface: a session holds a set of
//! **capabilities** (`observe`, `step`, `snapshot`, `present`) and a table of tools, and a tool with
//! no row, no capability and no predicate is *"a refusal, never an allow."* Its `POLICY-CIVILIANS.md`
//! states the rule this module exists to honour:
//!
//! > *"**Refusals are named and recorded**: no row → `coverage`; no capability and no grant →
//! > `unauthorized`; a predicate that could not run → `unmeasured`; and each of them is a **ledger
//! > record**, which is where the in-memory `Denial` list dies."*
//!
//! > *"predicates are rows, never a score … a star lit by a predicate that did not run is the defect
//! > the five-star rule forbids."*
//!
//! # Why there is no second matrix here
//!
//! This tree already owns the permission matrix: [`crate::policy`] — one [`Action`] per act, a
//! [`PermissionClass`] per action, and [`crate::policy::decide`] as *the* chokepoint (D6's
//! **permission/policy chokepoint**, *"load-bearing for this tree"*). It exists because
//! `civilian/gov/city_reference.rs` holds a `Governor::authorize` that would be a **second decision
//! matrix**, and this tree refuses second matrices by name.
//!
//! So a [`Capability`] here is **not** a second permission model. It is a **partition label over the
//! one action vocabulary**: every [`Action`] belongs to exactly one capability, and exactly one
//! action belongs to none because the record forbids it outright. The four names are the port's;
//! the authority is [`crate::policy`]'s, and it decides every call *after* the capability check, not
//! instead of it.
//!
//! # Both checks run, and the receipt says which
//!
//! A capability check that passed is not a permission: [`crate::policy::Action::DeleteContent`] is
//! `AlwaysDenied`, so a session holding every capability and a valid grant still cannot delete. A
//! [`Receipt`] therefore carries [`Receipt::checks_ran`] — the checks that actually executed, in
//! order — because a check that did not run is not a check that passed.
//!
//! # And a denial goes on the record
//!
//! [`record_refusal`] files the refusal as a [`voxel_model::chain::EventKind::RefusalRecorded`]
//! event. That is §39's rule — *"a refusal nobody can read is a refusal that did not happen"* — and
//! it is why nothing here keeps a denial in memory only.

use std::fmt;

use voxel_model::chain::{Chain, Event, EventKind, Namespace};
use voxel_model::grant::Grant;
use voxel_model::refusal::{Refusal, RefusalKind, RefusalSet};

use crate::policy::{self, Action, Decision, PermissionClass};

/// A capability a session holds. **Nothing is granted implicitly.**
///
/// The four are the sibling tree's `planet-mcp/src/authority.rs` set, ported by name so the two
/// trees can be read against each other. What each one *covers* here is this tree's own partition of
/// [`Action`], declared in [`capability_of`] and pinned by a test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    /// Read what is, and change nothing.
    Observe,
    /// Advance a state that is already recorded — the world, or the record itself.
    Step,
    /// Arrange what is being looked at: the shell's own surface, and speech.
    Snapshot,
    /// Put something before the record for judgement.
    Present,
}

impl Capability {
    pub const ALL: [Capability; 4] = [
        Capability::Observe,
        Capability::Step,
        Capability::Snapshot,
        Capability::Present,
    ];

    /// A literal, for the same reason `EventKind::KIND_COUNT` is: the pin's value is meaningless and
    /// its **type** is the assertion.
    pub const COUNT: usize = 4;

    pub const fn name(self) -> &'static str {
        match self {
            Capability::Observe => "observe",
            Capability::Step => "step",
            Capability::Snapshot => "snapshot",
            Capability::Present => "present",
        }
    }

    pub const fn describes(self) -> &'static str {
        match self {
            Capability::Observe => "read what is, and change nothing",
            Capability::Step => "advance a state that is already recorded",
            Capability::Snapshot => "arrange what is being looked at",
            Capability::Present => "put something before the record for judgement",
        }
    }

    /// Parse, **refusing** anything else — a capability nobody can spell is a capability nobody
    /// meant.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "unknown capability {word:?}; the set is {}",
                    Self::ALL.map(|c| c.name()).join(", ")
                )
            })
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// **The partition.** Which capability an action needs, or `None` when no capability can carry it.
///
/// The `match` has **no wildcard arm**, on the same argument [`crate::policy::permission_of`] makes:
/// a new [`Action`] must be classified by whoever adds it, and the compiler is what makes that
/// happen rather than a reader noticing.
///
/// `None` is returned **exactly** for the actions the record refuses outright, so the capability map
/// cannot become a way around the matrix. The correspondence is pinned by
/// `the_capability_map_and_the_always_denied_set_are_the_same_actions`.
pub const fn capability_of(action: Action) -> Option<Capability> {
    match action {
        // Read what is, and change nothing.
        Action::ReadRecipe
        | Action::ReadProvenance
        | Action::LintLine
        | Action::ReadChain
        | Action::ReadWorld
        | Action::ReadGovernance => Some(Capability::Observe),

        // Advance a state that is already recorded.
        Action::AppendEvent
        | Action::TransitionContent
        | Action::WriteWorld
        | Action::SaveWorld
        | Action::LoadWorld => Some(Capability::Step),

        // Arrange what is being looked at: changes what is *seen*, never what is.
        Action::MountWindow
        | Action::CloseWindow
        | Action::PinWindow
        | Action::UnpinWindow
        | Action::RestyleWindow
        | Action::RaiseToast
        | Action::Speak => Some(Capability::Snapshot),

        // Put something before the record for judgement.
        Action::CompileRecipe
        | Action::VerifyPackage
        | Action::FileTicket
        | Action::ResolveTicket => Some(Capability::Present),

        // **No capability carries this one.** `DeleteContent` is `AlwaysDenied` — CLM says never
        // delete — and an authorization that could permit it would make that ruling decorative.
        // Returning a capability here would put a second answer beside the matrix's.
        Action::DeleteContent => None,
    }
}

/// The capability set a bridge session holds. **Empty until something is granted**, so an
/// unconfigured session can do nothing — deny by default, at the session level.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Authority {
    capabilities: Vec<Capability>,
}

impl Authority {
    /// A session that may do nothing. **The default, and it is not a formality**: silence is not
    /// permission.
    pub fn none() -> Self {
        Authority {
            capabilities: Vec::new(),
        }
    }

    /// A local session, holding all four. The sibling tree's `Authority::full()`, and the same
    /// caveat applies: this is a statement about a *local single-user* socket, not authentication.
    pub fn full() -> Self {
        Authority {
            capabilities: Capability::ALL.to_vec(),
        }
    }

    pub fn grant(mut self, capability: Capability) -> Self {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
        self
    }

    pub fn allows(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn granted(&self) -> &[Capability] {
        &self.capabilities
    }
}

/// One row of the bridge's surface: a name a caller can use, and the action it performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tool {
    pub name: &'static str,
    pub action: Action,
}

impl Tool {
    /// The capability this tool needs, or `None` for an act the record forbids.
    pub const fn capability(&self) -> Option<Capability> {
        capability_of(self.action)
    }

    /// The class the matrix places this tool's action in. **Asked of the matrix, not restated.**
    pub const fn class(&self) -> PermissionClass {
        policy::permission_of(self.action)
    }
}

/// **The surface, and it is total over the action vocabulary**: one row per [`Action`], named after
/// it, in [`Action::ALL`]'s order.
///
/// Totality is the point rather than a nicety. A tool that is *absent* is a capability the bridge
/// silently does not have, and the sibling tree's rule is that *"a missing row … is a refusal, never
/// an allow"* — so the row for the act this tree forbids ([`Action::DeleteContent`]) is **shipped and
/// refused by name** rather than left out. `delete_content` exists so that calling it is a recorded
/// denial instead of an absent verb nobody can read.
pub const TOOLS: [Tool; Action::COUNT] = [
    Tool { name: "compile_recipe", action: Action::CompileRecipe },
    Tool { name: "read_recipe", action: Action::ReadRecipe },
    Tool { name: "verify_package", action: Action::VerifyPackage },
    Tool { name: "read_provenance", action: Action::ReadProvenance },
    Tool { name: "lint_line", action: Action::LintLine },
    Tool { name: "mount_window", action: Action::MountWindow },
    Tool { name: "close_window", action: Action::CloseWindow },
    Tool { name: "pin_window", action: Action::PinWindow },
    Tool { name: "unpin_window", action: Action::UnpinWindow },
    Tool { name: "restyle_window", action: Action::RestyleWindow },
    Tool { name: "raise_toast", action: Action::RaiseToast },
    Tool { name: "read_chain", action: Action::ReadChain },
    Tool { name: "append_event", action: Action::AppendEvent },
    Tool { name: "transition_content", action: Action::TransitionContent },
    Tool { name: "delete_content", action: Action::DeleteContent },
    Tool { name: "read_world", action: Action::ReadWorld },
    Tool { name: "write_world", action: Action::WriteWorld },
    Tool { name: "save_world", action: Action::SaveWorld },
    Tool { name: "load_world", action: Action::LoadWorld },
    Tool { name: "file_ticket", action: Action::FileTicket },
    Tool { name: "resolve_ticket", action: Action::ResolveTicket },
    Tool { name: "read_governance", action: Action::ReadGovernance },
    Tool { name: "speak", action: Action::Speak },
];

/// **The pin.** Every action has a row, and no row is spare — checked by the compiler as well as by
/// a test, the way `KINDS_ARE_TOTAL` is.
pub const TOOLS_ARE_TOTAL: [(); Action::COUNT] = [(); TOOLS.len()];

/// The predicates a call may run, **declared** rather than named at the call site, so a receipt
/// cannot report a predicated that does not exist.
pub const PREDICATES: [&str; 3] = [PREDICATE_COVERAGE, PREDICATE_CAPABILITY, PREDICATE_MATRIX];

/// The tool name resolved to a row.
pub const PREDICATE_COVERAGE: &str = "tool_has_a_row";
/// The session's own authority covers what the row needs.
pub const PREDICATE_CAPABILITY: &str = "capability_held";
/// The one matrix permits the act, under the grant presented.
pub const PREDICATE_MATRIX: &str = "matrix_permits";

/// The row for a tool name, or `None` when the surface does not carry it.
///
/// A lookup rather than a `match`, so the table above is the single place a verb is spelled.
pub fn tool(name: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|row| row.name == name.trim())
}

/// What a call did, and **which checks actually ran**.
#[derive(Clone, Debug)]
pub struct Receipt {
    /// The name the caller used, verbatim — including one the surface does not carry, because a
    /// refusal has to name what was asked for.
    pub tool: String,
    /// The action, when the name resolved to a row.
    pub action: Option<Action>,
    /// The capability the row needs, when there was a row.
    pub capability: Option<Capability>,
    /// The matrix's decision, when the matrix was reached. **`None` means it never ran**, which is a
    /// different fact from a denial and must not be rendered as one.
    pub decision: Option<Decision>,
    /// Every refusal this call produced.
    pub refusals: RefusalSet,
    /// The checks that executed, in order. **A check that did not run is not a check that passed.**
    pub checks_ran: Vec<&'static str>,
}

impl Receipt {
    /// Whether the act may proceed. **Only true when every check ran and all of them passed**, so a
    /// caller cannot read a partial receipt as permission.
    pub fn ran(&self) -> bool {
        self.checks_ran.len() == PREDICATES.len()
            && self.refusals.is_empty()
            && self.decision.as_ref().is_some_and(Decision::is_permitted)
    }

    /// Whether the refusal was a capability the session does not hold.
    pub fn is_unauthorized(&self) -> bool {
        self.action.is_some() && !self.checks_ran.contains(&PREDICATE_CAPABILITY)
    }

    /// The line a caller prints or files for this receipt.
    pub fn line(&self) -> String {
        let verdict = if self.ran() { "ran" } else { "refused" };
        let mut out = format!(
            "{} {} — checks ran: {}",
            self.tool,
            verdict,
            if self.checks_ran.is_empty() {
                "none".to_string()
            } else {
                self.checks_ran.join(", ")
            }
        );
        if let Some(refusal) = self.refusals.first_named() {
            out.push_str(&format!(" — {}", refusal.detail()));
        }
        out
    }
}

/// **The bridge's one entry point.** Call a tool by name, under a session's authority, with an
/// optional grant.
///
/// The order is the argument, and each step is a refusal rather than a fallthrough:
///
/// 1. **Coverage** — the name resolves to a row, or the call is refused by name.
/// 2. **Capability** — the session holds what the row needs. An act with no capability
///    ([`Action::DeleteContent`]) is refused **here**, before the matrix is consulted, and the
///    receipt records that the matrix never ran.
/// 3. **The matrix** — [`crate::policy::decide_or_refuse`], the one chokepoint, with the grant.
///
/// Nothing is executed by this function: it decides. A caller that gets [`Receipt::ran`] `true` is
/// the caller that performs the act, which is why a receipt that says otherwise cannot be mistaken
/// for permission to proceed.
pub fn call(authority: &Authority, name: &str, grant: Option<&Grant>) -> Receipt {
    let mut checks_ran: Vec<&'static str> = Vec::new();

    let Some(row) = tool(name) else {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::Unimplemented,
            format!(
                "the surface carries no tool named {name:?}; a missing row is a refusal, never an \
                 allow ({} tools exist)",
                TOOLS.len()
            ),
        );
        return Receipt {
            tool: name.to_string(),
            action: None,
            capability: None,
            decision: None,
            refusals,
            checks_ran,
        };
    };
    checks_ran.push(PREDICATE_COVERAGE);

    let mut refusals = RefusalSet::new();
    let Some(capability) = row.capability() else {
        refusals.refuse(
            RefusalKind::UntrustedKey,
            format!(
                "{} is {}: no capability carries it, and the matrix refuses it outright — CLM says \
                 never delete, and an authorization that could permit deletion would make that \
                 ruling decorative",
                row.name,
                row.class().slug()
            ),
        );
        return Receipt {
            tool: row.name.to_string(),
            action: Some(row.action),
            capability: None,
            decision: None,
            refusals,
            checks_ran,
        };
    };

    if !authority.allows(capability) {
        refusals.refuse(
            RefusalKind::UntrustedKey,
            format!(
                "{} needs the {capability} capability and this session does not hold it; silence is \
                 not permission",
                row.name
            ),
        );
        return Receipt {
            tool: row.name.to_string(),
            action: Some(row.action),
            capability: Some(capability),
            decision: None,
            refusals,
            checks_ran,
        };
    }
    checks_ran.push(PREDICATE_CAPABILITY);

    let (decision, refusals) = policy::decide_or_refuse(row.action, grant);
    checks_ran.push(PREDICATE_MATRIX);

    Receipt {
        tool: row.name.to_string(),
        action: Some(row.action),
        capability: Some(capability),
        decision: Some(decision),
        refusals,
        checks_ran,
    }
}

/// **File a refusal on the record**, because an in-memory denial is a denial that did not happen.
///
/// Refuses to file anything for a receipt that permitted the call: recording a refusal for an act
/// that was allowed would make the record say a thing that did not happen, and the record is
/// append-only, so it could not be taken back.
pub fn record_refusal<'chain>(
    chain: &'chain mut Chain,
    namespace: Namespace,
    receipt: &Receipt,
) -> Result<&'chain Event, RefusalSet> {
    if receipt.refusals.is_empty() {
        let mut refusals = RefusalSet::new();
        refusals.refuse(
            RefusalKind::MalformedRecord,
            format!(
                "{} produced no refusal, so there is nothing to file; an append-only record cannot \
                 take a line back",
                receipt.tool
            ),
        );
        return Err(refusals);
    }
    chain.append(namespace, EventKind::RefusalRecorded, receipt.line())
}

/// **Check the surface itself**: totality, a class per tool, and a capability for every act the
/// matrix does not refuse outright.
///
/// This is the port of the sibling tree's *"totality is pinned … a new tool cannot ship ungated"*,
/// asked of this tree's matrix rather than of a table beside it.
pub fn check_surface() -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    for row in TOOLS.iter() {
        for refusal in row_refusals(row).all() {
            refusals.push(Refusal::owned(refusal.kind(), refusal.detail()));
        }
    }

    // Every action exactly once: the surface is a bijection onto the vocabulary, not a sample.
    let mut actions: Vec<Action> = TOOLS.iter().map(|row| row.action).collect();
    let total = actions.len();
    actions.sort();
    actions.dedup();
    if actions.len() != total {
        refusals.refuse(
            RefusalKind::RecordExists,
            format!("{total} rows cover {} distinct actions; a repeated row is a second spelling", actions.len()),
        );
    }
    for action in Action::ALL {
        if !TOOLS.iter().any(|row| row.action == action) {
            refusals.refuse(
                RefusalKind::UncoveredAxis,
                format!("{} has no row, so no caller can reach it and no refusal can name it", action.name()),
            );
        }
    }

    refusals.into_result(())
}

/// **One row's coherence, reported and not ruled on.**
///
/// The reader half of the pair [`crate::decision::rule_for_tool`] completes: this says *what is
/// wrong with the row* and the rule says *what that is worth*. The split is C-84.2's — *"the readers
/// report; the layer decides"* — and it is what lets a second instrument handed the same row reach
/// its own verdict instead of inheriting this module's.
///
/// Two checks, both properties of the row alone:
///
/// * a name that is not a single token, which no caller can spell and no log can name;
/// * a capability that disagrees with the matrix's class for the act — **the rule that keeps a
///   second matrix from growing here.** [`capability_of`] could return `None` for a permitted act
///   and still compile; this is what refuses it, and
///   `the_capability_map_and_the_always_denied_set_are_the_same_actions` is the witness.
///
/// **A row that reports nothing is not thereby permitted.** The act the record forbids outright is
/// a coherent row — class `always_denied`, no capability — and it belongs here rather than being
/// left out, because the whole reason it ships is so that calling it is a recorded denial instead
/// of an absent verb nobody can read.
pub fn row_refusals(row: &Tool) -> RefusalSet {
    let mut refusals = RefusalSet::new();

    if row.name.trim().is_empty() {
        refusals.refuse(
            RefusalKind::FabricatedFeature,
            format!(
                "{}: the row has no name, so no caller can reach it",
                row.action.name()
            ),
        );
    }
    if row.name != row.name.trim() || row.name.contains(' ') {
        refusals.refuse(
            RefusalKind::MalformedIdentity,
            format!(
                "{:?}: a tool name is one token, so it can be spelled at a call site",
                row.name
            ),
        );
    }

    // The rule that keeps a second matrix from growing here: a row may not carry a capability for
    // an act the matrix refuses outright, and it may not omit one for an act it permits.
    let denied = row.class() == PermissionClass::AlwaysDenied;
    if denied && row.capability().is_some() {
        refusals.refuse(
            RefusalKind::UncoveredAxis,
            format!(
                "{} is {} but carries the {} capability; a capability must not sit beside a \
                 class the matrix has already refused",
                row.name,
                row.class().slug(),
                row.capability().expect("just checked").name()
            ),
        );
    }
    if !denied && row.capability().is_none() {
        refusals.refuse(
            RefusalKind::UncoveredAxis,
            format!(
                "{} is {} and no capability carries it, so the surface would ship an act with no \
                 gate in front of it",
                row.name,
                row.class().slug()
            ),
        );
    }

    refusals
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::chain::{Chain, EventKind, Namespace, Season};

    fn grant() -> Grant {
        Grant::new("operator", "the shell").expect("a scoped, attributed grant")
    }

    fn namespace() -> Namespace {
        Namespace::new("shploingus", 0, Season::Winter, "C-1").expect("a well-formed namespace")
    }

    /// The act the matrix refuses outright, found by **class** rather than by name, so this helper
    /// cannot go stale if the verb is ever renamed.
    fn the_forbidden_row() -> &'static Tool {
        TOOLS
            .iter()
            .find(|row| row.class() == PermissionClass::AlwaysDenied)
            .expect("the vocabulary holds at least one act the record forbids")
    }

    #[test]
    fn the_surface_is_total_over_the_action_vocabulary_and_named_after_it() {
        // The crate's own pin is the compile-time half; this is the half a pin cannot state — that
        // there is one row per action, that a row's name *is* its action's name, and that the name
        // is how the surface is reached.
        let _pin: [(); Action::COUNT] = TOOLS_ARE_TOTAL;
        assert!(check_surface().is_ok(), "{}", check_surface().unwrap_err());
        for action in Action::ALL {
            let mut rows = 0usize;
            for row in TOOLS {
                if row.action == action {
                    rows += 1;
                    assert_eq!(row.name, action.name(), "a row is named after its action");
                    assert_eq!(
                        tool(row.name).map(|found| found.name),
                        Some(row.name),
                        "and is reached by that name"
                    );
                }
            }
            assert_eq!(rows, 1, "{} has {rows} rows, not one", action.name());
        }
    }

    #[test]
    fn a_tool_name_the_surface_does_not_carry_is_refused_by_name() {
        // The sibling tree's rule: a missing row is a refusal, never an allow. Called with every
        // capability held and a valid grant, so the refusal cannot be the capability gate's.
        let receipt = call(&Authority::full(), "unmount_everything", Some(&grant()));
        assert!(!receipt.ran());
        assert!(receipt.action.is_none(), "no row was reached");
        assert!(
            receipt.checks_ran.is_empty(),
            "not one check ran: {:?}",
            receipt.checks_ran
        );
        assert_eq!(
            receipt.tool, "unmount_everything",
            "the refusal names what was asked for"
        );
        assert!(
            receipt
                .refusals
                .first_named()
                .expect("an absent row is a refusal")
                .detail()
                .contains("no tool named"),
            "{}",
            receipt.line()
        );
    }

    #[test]
    fn a_capability_the_session_does_not_hold_refuses_before_the_matrix_runs() {
        // `read_chain` is `always_allowed`, so the matrix *would* permit it — and it still does not
        // run. That ordering is the assertion: the session is refused by the absence of a
        // capability, and the receipt records that the matrix was never reached rather than
        // rendering a decision it did not make.
        let receipt = call(&Authority::none(), "read_chain", None);
        assert!(!receipt.ran());
        assert!(receipt.is_unauthorized());
        assert_eq!(receipt.checks_ran, vec![PREDICATE_COVERAGE]);
        assert!(
            receipt.decision.is_none(),
            "a decision that never ran is not a denial, and must not render as one"
        );
        assert!(receipt
            .refusals
            .first_named()
            .expect("a refusal")
            .detail()
            .contains("silence is not permission"));
    }

    #[test]
    fn the_act_the_record_forbids_is_refused_at_the_gate_and_not_by_the_matrix() {
        // Every capability held *and* a valid grant, and it still cannot run: that is the
        // difference between holding a capability and being permitted.
        let row = the_forbidden_row();
        let receipt = call(&Authority::full(), row.name, Some(&grant()));
        assert!(!receipt.ran());
        assert_eq!(receipt.action, Some(row.action));
        assert_eq!(
            receipt.capability, None,
            "no capability carries the act the record forbids"
        );
        assert_eq!(receipt.decision, None, "the matrix was not reached");
        assert!(receipt
            .refusals
            .first_named()
            .expect("a refusal")
            .detail()
            .contains("never delete"));
    }

    #[test]
    fn holding_every_capability_is_still_not_permission() {
        // A row that needs a grant, called with no grant, with all four held. This is the only
        // test here that asserts every check ran *and* the act was still refused — the property
        // `Receipt::ran` exists to state.
        let receipt = call(&Authority::full(), "mount_window", None);
        assert!(!receipt.ran());
        assert_eq!(receipt.checks_ran, PREDICATES.to_vec());
        assert!(
            !receipt.is_unauthorized(),
            "the capability was held; the grant was not"
        );
        assert_eq!(
            receipt.decision.as_ref().expect("the matrix ran").slug(),
            "denied"
        );
    }

    #[test]
    fn a_call_runs_only_when_every_check_ran_and_all_of_them_passed() {
        let receipt = call(&Authority::full(), "mount_window", Some(&grant()));
        assert!(receipt.ran());
        assert_eq!(receipt.checks_ran, PREDICATES.to_vec());
        assert_eq!(receipt.capability, Some(Capability::Snapshot));
        assert!(receipt.line().contains("ran"), "{}", receipt.line());
        match receipt.decision.as_ref().expect("the matrix ran") {
            Decision::Granted { granter, scope } => {
                assert_eq!(granter, "operator");
                assert_eq!(scope, "the shell");
            }
            other => panic!("expected the named grant, got {other}"),
        }
    }

    #[test]
    fn the_capability_map_and_the_always_denied_set_are_the_same_actions() {
        // **The witness `capability_of` cannot be.** A `const fn` returning `None` for a permitted
        // act compiles; only this correspondence refuses it — so the capability gate can never be a
        // way round the matrix, and no permitted act can ship with no gate in front of it.
        for action in Action::ALL {
            let denied = policy::permission_of(action) == PermissionClass::AlwaysDenied;
            assert_eq!(
                capability_of(action).is_none(),
                denied,
                "{} is {} and capability_of says {}",
                action.name(),
                policy::permission_of(action).slug(),
                if denied { "no capability" } else { "a capability" }
            );
        }
        assert_eq!(
            Action::ALL
                .iter()
                .filter(|action| capability_of(**action).is_none())
                .count(),
            1,
            "the record forbids exactly one act"
        );
    }

    #[test]
    fn every_capability_carries_at_least_one_act_so_none_is_decorative() {
        // Four names ported from the sibling tree. A name covering nothing would be a capability a
        // session could hold and never use, which reads as a permission and is not one.
        for capability in Capability::ALL {
            let covered = TOOLS
                .iter()
                .filter(|row| row.capability() == Some(capability))
                .count();
            assert!(covered > 0, "{} carries no tool", capability.name());
        }
        assert_eq!(
            TOOLS.iter().filter(|row| row.capability().is_some()).count(),
            Action::COUNT - 1,
            "every act but the one the record forbids is gated"
        );
        assert_eq!(Capability::COUNT, Capability::ALL.len());
    }

    #[test]
    fn a_capability_nobody_can_spell_is_refused_rather_than_guessed() {
        assert_eq!(Capability::parse(" observe ").unwrap(), Capability::Observe);
        let refused = Capability::parse("observe_all").expect_err("a word nobody meant");
        assert!(refused.contains("unknown capability"), "{refused}");
        assert!(
            refused.contains("observe"),
            "and it names the set it does have: {refused}"
        );
    }

    #[test]
    fn a_row_no_caller_can_spell_is_reported_by_the_row_check() {
        // The one defect a row can carry alone and that no type can rule out: a name that is not a
        // single token. Constructible, so the check has a witness rather than being a branch nobody
        // has ever taken — `habitat.rs`'s rule about dead arms, applied to a checker.
        let unspellable = Tool {
            name: "mount window",
            action: Action::MountWindow,
        };
        let refusals = row_refusals(&unspellable);
        assert_eq!(refusals.len(), 1, "{refusals}");
        assert!(refusals
            .first_named()
            .expect("a refusal")
            .detail()
            .contains("one token"));
        // And the shipped row for the same action is clean, so the difference is the name alone.
        assert!(row_refusals(tool("mount_window").expect("a row")).is_empty());
    }

    #[test]
    fn a_refusal_goes_on_the_record_and_a_permitted_call_files_nothing() {
        // §39: a refusal nobody can read is a refusal that did not happen. And the converse,
        // because the chain is append-only — a refusal recorded for an act that was *allowed*
        // could not be taken back.
        let refused = call(&Authority::none(), "read_chain", None);
        let mut chain = Chain::linked();
        let event = record_refusal(&mut chain, namespace(), &refused).expect("a refusal is filed");
        assert_eq!(event.kind, EventKind::RefusalRecorded);
        assert!(
            event.detail.contains("read_chain"),
            "the record names the verb: {}",
            event.detail
        );

        let permitted = call(&Authority::full(), "read_chain", None);
        assert!(permitted.ran());
        let error =
            record_refusal(&mut chain, namespace(), &permitted).expect_err("nothing to file");
        assert!(error
            .first_named()
            .expect("a refusal")
            .detail()
            .contains("nothing to file"));
        assert_eq!(chain.len(), 1, "and the record did not grow");
    }
}
