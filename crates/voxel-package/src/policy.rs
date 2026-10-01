//! The policy chokepoint: **every action passes through one decision function.**
//!
//! # Where this comes from
//!
//! `docs/HYPERGRILLING.md` L-5 records the join with ala-system and states four
//! properties, quoted in spirit:
//!
//! * **Deny by default.** An action the module does not know about is **denied**.
//!   *"Silence never means permission."*
//! * **A missing matrix entry is a compile error** — `satisfies
//!   Record<Action, PermissionClass>` — *"not a runtime deny nobody notices."*
//! * **Permission class is a property of the action**, not of the tool or the
//!   caller: `run_connector(github, public)` and `run_connector(github, private)`
//!   are different classes through the same tool.
//! * **Grants are scoped and attributed.** A grant with no granter or no scope is
//!   **refused construction**: *"An authorization with no author is not an
//!   authorization."*
//!
//! D6 (`docs/HYPERGRILLING.md:1429`) names this the **permission/policy chokepoint**
//! and lists it among the six security items that are *"load-bearing for this tree"*. H-12.1
//! gives it its subject: the action vocabulary is **the full integration API**, and *"the
//! recipe system and its package proof are inside it, not beside it."*
//!
//! # Why this is the same pattern as the feature table
//!
//! `crate::features` makes **a declared feature with no witness a compile error**. This
//! module makes **an action with no permission entry a compile error**, by the same
//! mechanism: an exhaustive `match` with **no wildcard arm** over [`Action`].
//!
//! That is deliberate, and it is the answer to *"a lot of the missing functions are
//! supposed to be evaluators for the building."* A permission matrix that is merely
//! *complete at runtime* can be incomplete by accident; one that is complete **in the
//! type** cannot be merged incomplete.
//!
//! # Deny by default, and what "default" is allowed to mean here
//!
//! [`Action`] is a **closed** enum, so "an action the module does not know about" is
//! unrepresentable in Rust — the compiler refuses it before any policy runs. That is
//! the stronger half of the rule and it is structural.
//!
//! The half that needs care is the *grant* half: an action may be `PermissionClass::
//! Denied` outright, or `Conditional` — permitted only under a grant that is both
//! **scoped** and **attributed**. A missing, empty or unattributed grant is a
//! refusal, never a pass, and never a default.
//!
//! # What this module does not do
//!
//! It does not decide *whether a particular granter may grant this*. It decides
//! whether **this** call has an authorization attached to it, and it refuses
//! construction of an authorization with no author. Who is allowed to grant is a
//! further question, and this tree records it as **unruled** rather than guessing —
//! the same treatment D6 gives the package capability sandbox, which it names *"the
//! largest unruled thing in the project."*

use std::fmt;

use voxel_model::{Refusal, RefusalKind, RefusalSet};

/// What the record's action vocabulary names.
///
/// One variant per act. **Closed**: a caller cannot name an act this module has not
/// heard of, so "an action the module does not know about" is a compile error rather
/// than a runtime denial — which is L-5's stronger requirement met structurally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    // ---- the package and recipe route (H-12.1: the recipe is inside the API) ----
    /// Compile a recipe into a package.
    CompileRecipe,
    /// Read a recipe's declaration.
    ReadRecipe,
    /// Verify a package before it is used.
    VerifyPackage,
    /// Read this build's provenance.
    ReadProvenance,
    /// Run the conformance lint gate over a line.
    LintLine,

    // ---- the shell surface (H-9.7, H-9.8, H-12.2) ----
    /// Mount a declared window.
    MountWindow,
    /// Close a window.
    CloseWindow,
    /// Pin a window into the HUD.
    PinWindow,
    /// Unpin a window from the HUD.
    UnpinWindow,
    /// Set a window's opacity, elevation or theme.
    RestyleWindow,
    /// Enqueue a toast on the shell's own queue.
    RaiseToast,

    // ---- the record (H-8.1, and CLM: never delete) ----
    /// Read the append-only chain.
    ReadChain,
    /// Append an event to the chain.
    AppendEvent,
    /// Version, promote, supersede or retire a piece of content.
    TransitionContent,
    /// **Delete** content. Refused unconditionally, and the enum carries it anyway.
    DeleteContent,

    // ---- the world (H-8.3 cadences; the world's own determinism) ----
    /// Read the world.
    ReadWorld,
    /// Write to the world.
    WriteWorld,
    /// Save the world to disk.
    SaveWorld,
    /// Load the world from disk.
    LoadWorld,

    // ---- governance and security (the composition's other parts) ----
    /// File a ticket.
    FileTicket,
    /// Resolve or close a ticket.
    ResolveTicket,
    /// Read governance state.
    ReadGovernance,
    /// Speak through the streamer surface.
    Speak,
}

impl Action {
    /// How many actions the vocabulary holds: **a literal, on purpose**.
    ///
    /// Declared as 24 and written as 23 during authoring, and the mismatch was a
    /// **compile error** rather than a silent drift — which is the property the
    /// literal exists to provide, and the same trade `crate::features` makes.
    pub const COUNT: usize = 23;

    /// Every action, in declaration order.
    pub const ALL: [Action; Self::COUNT] = [
        Action::CompileRecipe,
        Action::ReadRecipe,
        Action::VerifyPackage,
        Action::ReadProvenance,
        Action::LintLine,
        Action::MountWindow,
        Action::CloseWindow,
        Action::PinWindow,
        Action::UnpinWindow,
        Action::RestyleWindow,
        Action::RaiseToast,
        Action::ReadChain,
        Action::AppendEvent,
        Action::TransitionContent,
        Action::DeleteContent,
        Action::ReadWorld,
        Action::WriteWorld,
        Action::SaveWorld,
        Action::LoadWorld,
        Action::FileTicket,
        Action::ResolveTicket,
        Action::ReadGovernance,
        Action::Speak,
    ];

    /// A stable, machine-readable name. Never prose.
    pub const fn name(self) -> &'static str {
        match self {
            Action::CompileRecipe => "compile_recipe",
            Action::ReadRecipe => "read_recipe",
            Action::VerifyPackage => "verify_package",
            Action::ReadProvenance => "read_provenance",
            Action::LintLine => "lint_line",
            Action::MountWindow => "mount_window",
            Action::CloseWindow => "close_window",
            Action::PinWindow => "pin_window",
            Action::UnpinWindow => "unpin_window",
            Action::RestyleWindow => "restyle_window",
            Action::RaiseToast => "raise_toast",
            Action::ReadChain => "read_chain",
            Action::AppendEvent => "append_event",
            Action::TransitionContent => "transition_content",
            Action::DeleteContent => "delete_content",
            Action::ReadWorld => "read_world",
            Action::WriteWorld => "write_world",
            Action::SaveWorld => "save_world",
            Action::LoadWorld => "load_world",
            Action::FileTicket => "file_ticket",
            Action::ResolveTicket => "resolve_ticket",
            Action::ReadGovernance => "read_governance",
            Action::Speak => "speak",
        }
    }

    /// A one-line statement of what this action is, for the API view.
    ///
    /// Readable on purpose: H-12.1's subject is rendered to a person in the right
    /// pane of the Mods window, so an action with no description would be a row a
    /// reader cannot understand.
    pub const fn describes(self) -> &'static str {
        match self {
            Action::CompileRecipe => "compile a recipe declaration into a package",
            Action::ReadRecipe => "read a recipe's declaration",
            Action::VerifyPackage => "check a package's signature before it is used",
            Action::ReadProvenance => "read the provenance of this build",
            Action::LintLine => "put a line to the conformance oracle",
            Action::MountWindow => "mount a declared window into the shell",
            Action::CloseWindow => "close a mounted window",
            Action::PinWindow => "pin a window into the HUD",
            Action::UnpinWindow => "unpin a window from the HUD",
            Action::RestyleWindow => "set a window's opacity, elevation or theme",
            Action::RaiseToast => "enqueue a toast on the shell's own queue",
            Action::ReadChain => "read the append-only event chain",
            Action::AppendEvent => "append an event to the chain",
            Action::TransitionContent => "version, promote, supersede or retire content",
            Action::DeleteContent => "delete content (refused: the record says never delete)",
            Action::ReadWorld => "read the world",
            Action::WriteWorld => "write to the world",
            Action::SaveWorld => "save the world to disk",
            Action::LoadWorld => "load the world from disk",
            Action::FileTicket => "file a ticket",
            Action::ResolveTicket => "resolve or close a ticket",
            Action::ReadGovernance => "read governance state",
            Action::Speak => "speak through the streamer surface",
        }
    }

    /// Look an action up by name. Unknown is refused rather than defaulted.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|action| action.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "unknown action {word:?}; the vocabulary is closed and holds {}",
                    Self::COUNT
                )
            })
    }
}

/// The class an action belongs to.
///
/// **A property of the action, not of the tool or the caller** — L-5's rule, and the
/// reason this is a function of [`Action`] alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionClass {
    /// Refused outright. No grant can permit it, and this is the class
    /// [`Action::DeleteContent`] holds: CLM says **never delete**, and an
    /// authorization that could permit deletion would make that ruling decorative.
    AlwaysDenied,
    /// Permitted without any grant. Read-only acts the shell itself needs.
    AlwaysAllowed,
    /// Permitted **only** under a grant that is both scoped and attributed.
    ///
    /// The interesting class, and the one that makes "deny by default" mean something:
    /// silence is not permission, so the absence of a grant is a refusal.
    RequiresGrant,
}

impl PermissionClass {
    pub const fn slug(self) -> &'static str {
        match self {
            PermissionClass::AlwaysDenied => "always_denied",
            PermissionClass::AlwaysAllowed => "always_allowed",
            PermissionClass::RequiresGrant => "requires_grant",
        }
    }

    /// The rendering, because a class with no rendering is a blank row.
    pub const fn describe(self) -> &'static str {
        match self {
            PermissionClass::AlwaysDenied => {
                "always denied — no grant can permit this, because the record forbids it"
            }
            PermissionClass::AlwaysAllowed => "always allowed — a read-only act, no grant needed",
            PermissionClass::RequiresGrant => {
                "requires a grant — permitted only by a grant that is scoped and attributed"
            }
        }
    }
}

/// **The matrix.** An action's permission class.
///
/// **Exhaustive, with no wildcard arm** — L-5's *"a missing matrix entry is a compile
/// error, not a runtime deny nobody notices."* Add a variant to [`Action`] and this
/// function stops compiling until someone classifies it.
///
/// This is the same mechanism as `crate::features::witness_of`, and for the same
/// reason: a non-exhaustive match cannot be merged, and a test can be deleted.
pub const fn permission_of(action: Action) -> PermissionClass {
    match action {
        // The package and recipe route. Reading and compiling a recipe are how the
        // system is built, so they need no grant; **verifying** a package is the
        // check everything else depends on, so it is never gated behind a grant that
        // verification itself might need.
        Action::CompileRecipe => PermissionClass::AlwaysAllowed,
        Action::ReadRecipe => PermissionClass::AlwaysAllowed,
        Action::VerifyPackage => PermissionClass::AlwaysAllowed,
        Action::ReadProvenance => PermissionClass::AlwaysAllowed,
        Action::LintLine => PermissionClass::AlwaysAllowed,

        // The shell surface. Mounting is gated because a package that may mount
        // anything may mount anything; the rest follow the same reasoning, except
        // raising a toast, which is the shell's own queue and is how a refused
        // action becomes *visible* rather than silent.
        Action::MountWindow => PermissionClass::RequiresGrant,
        Action::CloseWindow => PermissionClass::RequiresGrant,
        Action::PinWindow => PermissionClass::RequiresGrant,
        Action::UnpinWindow => PermissionClass::RequiresGrant,
        Action::RestyleWindow => PermissionClass::RequiresGrant,
        Action::RaiseToast => PermissionClass::AlwaysAllowed,

        // The record. Appending is the whole point of an append-only chain, but it
        // is still an act with consequences, so it needs an author.
        Action::ReadChain => PermissionClass::AlwaysAllowed,
        Action::AppendEvent => PermissionClass::RequiresGrant,
        Action::TransitionContent => PermissionClass::RequiresGrant,
        Action::DeleteContent => PermissionClass::AlwaysDenied,

        // The world.
        Action::ReadWorld => PermissionClass::AlwaysAllowed,
        Action::WriteWorld => PermissionClass::RequiresGrant,
        Action::SaveWorld => PermissionClass::RequiresGrant,
        Action::LoadWorld => PermissionClass::RequiresGrant,

        // Governance, tickets and speech.
        Action::FileTicket => PermissionClass::AlwaysAllowed,
        Action::ResolveTicket => PermissionClass::RequiresGrant,
        Action::ReadGovernance => PermissionClass::AlwaysAllowed,
        Action::Speak => PermissionClass::RequiresGrant,
    }
}

/// A grant: who authorised it, and over what.
///
/// **Re-exported from `voxel_model::grant`, not defined here, and that is the fix rather than
/// a tidy-up.** `voxel-package` depends on `voxel-shapers`, so the stage set — the only
/// authority-bearing object in the world model — could not see this type, and the first
/// attempt at closing that gap added a *second* grant-shaped enum to the stage set. Two names
/// for one meaning is D-07, so the type moved down to the crate both can see and this is
/// now the same type rather than a copy of it.
///
/// **The refusals are unchanged, including the kind.** `RefusalKind::FabricatedFeature` for
/// an empty granter and for an empty scope, because a caller filtering by kind must get the
/// same answer whichever crate it asked.
pub use voxel_model::grant::Grant;

/// The decision the chokepoint reached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Permitted, with no grant required.
    Allowed,
    /// Permitted by this grant, which is named.
    Granted { granter: String, scope: String },
    /// Denied, and **by name why** — never a bare `false`.
    Denied { reason: String },
}

impl Decision {
    pub const fn slug(&self) -> &'static str {
        match self {
            Decision::Allowed => "allowed",
            Decision::Granted { .. } => "granted",
            Decision::Denied { .. } => "denied",
        }
    }

    pub fn is_permitted(&self) -> bool {
        !matches!(self, Decision::Denied { .. })
    }
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Decision::Allowed => f.write_str("allowed — no grant required"),
            Decision::Granted { granter, scope } => {
                write!(f, "granted by {granter} over {scope}")
            }
            Decision::Denied { reason } => write!(f, "denied — {reason}"),
        }
    }
}

/// **The chokepoint.** One decision function, so the permission matrix is testable
/// *as a matrix* rather than per call site — L-5's stated reason: *"and so the one
/// call site nobody remembered cannot be the one that fails."*
///
/// **Deny by default.** Every branch here is an explicit decision about a named
/// action. There is no fallthrough and no catch-all, because a catch-all is how
/// silence becomes permission.
pub fn decide(action: Action, grant: Option<&Grant>) -> Decision {
    match permission_of(action) {
        PermissionClass::AlwaysAllowed => Decision::Allowed,
        PermissionClass::AlwaysDenied => Decision::Denied {
            reason: format!(
                "{} is always denied: the record forbids it, and no grant can permit it",
                action.name()
            ),
        },
        PermissionClass::RequiresGrant => match grant {
            // A missing grant is a denial, not a default permission.
            None => Decision::Denied {
                reason: format!(
                    "{} is {} and no grant was presented; silence is not permission",
                    action.name(),
                    permission_of(action).slug()
                ),
            },
            Some(grant) => Decision::Granted {
                granter: grant.granter().to_owned(),
                scope: grant.scope().to_owned(),
            },
        },
    }
}

/// Decide, and collect a refusal when the answer is a denial.
///
/// The two callers of this module need different things: the **client** needs a
/// decision it can render, and the **gate** needs a reason it can refuse with. Both
/// are produced here so neither can exist without the other.
pub fn decide_or_refuse(
    action: Action,
    grant: Option<&Grant>,
) -> (Decision, RefusalSet) {
    let decision = decide(action, grant);
    let mut refusals = RefusalSet::new();
    if let Decision::Denied { reason } = &decision {
        refusals.push(Refusal::owned(RefusalKind::UntrustedKey, format!("{}: {reason}", action.name())));
    }
    (decision, refusals)
}

/// Check the matrix itself: every action classified, every class rendered.
pub fn check_matrix() -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();

    for action in Action::ALL {
        if action.describes().trim().is_empty() {
            refusals.push(Refusal::owned(
                RefusalKind::FabricatedFeature,
                format!(
                    "{}: the action has no description, so the API view would render a row \
                     nobody can read",
                    action.name()
                ),
            ));
        }
        if permission_of(action).describe().trim().is_empty() {
            refusals.push(Refusal::owned(
                RefusalKind::FabricatedFeature,
                format!("{}: its permission class has no rendering", action.name()),
            ));
        }
    }

    // Totality: an action with no name is unreachable from the API view.
    let mut named: Vec<&str> = Action::ALL.iter().map(|action| action.name()).collect();
    let total = named.len();
    named.sort_unstable();
    named.dedup();
    if named.len() != total {
        refusals.push(Refusal::owned(
            RefusalKind::FabricatedFeature,
            "two actions share a name, so one of them cannot be named".to_owned(),
        ));
    }

    refusals.into_result(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant() -> Grant {
        Grant::new("the user", "the mods window").expect("a named, scoped grant constructs")
    }

    #[test]
    fn the_matrix_is_total_and_every_action_is_described() {
        // The compile-time half of L-5's rule is `permission_of`'s exhaustive match;
        // this is the runtime half, and it checks the parts a match cannot.
        check_matrix().expect("a complete matrix");
        assert_eq!(Action::ALL.len(), Action::COUNT);
    }

    #[test]
    fn every_action_name_round_trips_and_an_unknown_one_is_refused() {
        for action in Action::ALL {
            assert_eq!(Action::parse(action.name()).unwrap(), action);
        }
        // The vocabulary is closed: a caller cannot name an act nobody declared.
        assert!(Action::parse("exec_shell").is_err());
        assert!(Action::parse("").is_err());
    }

    #[test]
    fn an_action_with_no_permission_entry_would_not_compile() {
        // Stated as a property of the shape rather than tested by mutation, because
        // the mechanism is the absence of a wildcard arm in `permission_of`. This
        // test pins the half that *is* reachable at runtime: every declared action
        // resolves to a class, and `permission_of` is total over `Action::ALL`.
        for action in Action::ALL {
            let _ = permission_of(action);
        }
    }

    #[test]
    fn a_read_only_act_needs_no_grant() {
        let decision = decide(Action::ReadRecipe, None);
        assert_eq!(decision, Decision::Allowed);
        assert!(decision.is_permitted());
    }

    #[test]
    fn silence_is_not_permission() {
        // The load-bearing rule. A gated action with no grant is **denied**, and the
        // denial says why in words rather than returning a bare false.
        let decision = decide(Action::MountWindow, None);
        assert!(!decision.is_permitted());
        match &decision {
            Decision::Denied { reason } => {
                assert!(reason.contains("silence is not permission"), "{reason}");
            }
            other => panic!("expected a denial, got {other}"),
        }
    }

    #[test]
    fn a_scoped_and_attributed_grant_permits_a_gated_action_and_names_who() {
        let decision = decide(Action::MountWindow, Some(&grant()));
        assert!(decision.is_permitted());
        assert_eq!(decision.slug(), "granted");
        // The grant is *named* in the decision, so a rendered row says who allowed it.
        assert!(decision.to_string().contains("the user"), "{decision}");
        assert!(decision.to_string().contains("the mods window"), "{decision}");
    }

    #[test]
    fn an_authorization_with_no_author_is_not_an_authorization() {
        // L-5 verbatim, and refused at **construction** rather than at use, so an
        // unattributed grant cannot be carried around and later found to be empty.
        for bad in ["", "   "] {
            let refusals = Grant::new(bad, "some scope").expect_err("no granter must refuse");
            assert!(!refusals.is_empty());
            assert!(
                refusals.all()[0].detail().contains("no author"),
                "{}",
                refusals.all()[0].detail()
            );
        }
    }

    #[test]
    fn an_authorization_with_no_scope_is_refused_too() {
        // A grant with no scope authorizes everything, which is the same failure as
        // authorizing nothing.
        let refusals = Grant::new("the user", "  ").expect_err("no scope must refuse");
        assert!(!refusals.is_empty());
        assert!(refusals.all()[0].detail().contains("no scope"));
    }

    #[test]
    fn both_defects_are_collected_rather_than_the_first_one() {
        let refusals = Grant::new("", "").expect_err("both must refuse");
        assert_eq!(refusals.len(), 2, "{refusals}");
    }

    #[test]
    fn a_forbidden_act_stays_forbidden_under_a_perfect_grant() {
        // CLM: **never delete**. The enum carries the action so the API view can
        // *show* that it is refused — an action that cannot be named cannot be shown
        // to be forbidden — but no grant reaches it.
        let decision = decide(Action::DeleteContent, Some(&grant()));
        assert!(!decision.is_permitted());
        assert!(decision.to_string().contains("always denied"), "{decision}");
    }

    #[test]
    fn a_denial_comes_with_a_refusal_that_names_the_action() {
        // The client renders the decision; the gate refuses with the reason. Both
        // are produced, so neither can exist without the other.
        let (decision, refusals) = decide_or_refuse(Action::WriteWorld, None);
        assert!(!decision.is_permitted());
        assert_eq!(refusals.len(), 1);
        assert!(refusals.all()[0].detail().contains("write_world"));

        let (allowed, no_refusals) = decide_or_refuse(Action::ReadWorld, None);
        assert!(allowed.is_permitted());
        assert!(no_refusals.is_empty(), "a permitted action is not a refusal");
    }

    #[test]
    fn every_permission_class_has_a_distinct_slug_and_a_rendering() {
        let all = [
            PermissionClass::AlwaysDenied,
            PermissionClass::AlwaysAllowed,
            PermissionClass::RequiresGrant,
        ];
        let mut slugs: Vec<&str> = all.iter().map(|class| class.slug()).collect();
        let total = slugs.len();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), total, "two classes share a slug");
        for class in all {
            assert!(!class.describe().is_empty(), "{} has no rendering", class.slug());
        }
    }

    #[test]
    fn the_three_classes_are_all_represented_in_the_matrix() {
        // A matrix holding only two of the three classes would mean one rule was
        // never applied to anything.
        let denied = Action::ALL
            .iter()
            .filter(|a| permission_of(**a) == PermissionClass::AlwaysDenied)
            .count();
        let allowed = Action::ALL
            .iter()
            .filter(|a| permission_of(**a) == PermissionClass::AlwaysAllowed)
            .count();
        let granted = Action::ALL
            .iter()
            .filter(|a| permission_of(**a) == PermissionClass::RequiresGrant)
            .count();
        assert!(denied > 0 && allowed > 0 && granted > 0);
        assert_eq!(denied + allowed + granted, Action::COUNT);
    }

    #[test]
    fn raising_a_toast_is_allowed_because_a_refusal_must_be_visible() {
        // AWA3D's never-silent rule (C-60) reaching the policy: if a package's
        // action is refused, the shell's own queue is how the player learns. A
        // permission matrix in which the *only* way to speak is gated would make
        // every refusal invisible.
        assert_eq!(permission_of(Action::RaiseToast), PermissionClass::AlwaysAllowed);
    }
}
