//! **The policy chokepoint — one decision function, so the one call site nobody remembered cannot be
//! the one that fails.**
//!
//! Derived from `docs/HYPERGRILLING.md` **§L-5** (*"`src/security/policy.ts` — the single
//! chokepoint"*, read as a document) and the policy rows of `docs/LEDGER.md` (**§66.3**'s security-lane
//! row, **R-62.3**'s three named decisions, and **§67.4**'s `AppendEvent` correction). The legacy
//! `crates/voxel-package/src/policy.rs` is the **witness** that this chokepoint is served rather than
//! decorative; it was **not read**, and nothing here is transcribed from it (P7 — a witness defines the
//! contract, never the source).
//!
//! # The four properties, which are the whole module
//!
//! 1. **Deny by default.** An act this module does not know about cannot be *expressed*: [`Action`] is
//!    closed, and [`permission_of`] is an exhaustive `match` with **no wildcard arm**, so a missing
//!    matrix entry is a compile error rather than a runtime deny nobody notices. *Silence never means
//!    permission* — and the refusal vocabulary the surrounding world already carries is what an
//!    undeclared name meets (`tool_undeclared`, raised where the line is parsed, before any dispatch).
//! 2. **Permission is a property of the action.** [`permission_of`] takes an [`Action`] and **nothing
//!    else** — no tool, no caller, no world. The same act has one class whichever door it is reached
//!    through, so a permission cannot be widened by reaching it a different way.
//! 3. **Grants are scoped and attributed.** [`Grant::of`] refuses a grant with **no granter**
//!    (`grant_unattributed`: *"an authorization with no author is not an authorization"*) and one whose
//!    scope is absent, undeclared or unbounded (`grant_unscoped`: *"'everything' is not a scope"*). The
//!    refusal is raised **at construction**, so an unauthorised grant is not a value this build can
//!    hold.
//! 4. **A refusal quotes the list.** [`decide`]'s denial names the act, the scope it requires, and the
//!    scopes the caller actually holds beside who granted each — a refusal a reader can act on rather
//!    than a bare *denied*.
//!
//! # The decisions inside this vocabulary, each a reading rather than a transcription
//!
//! * **The reads are ungated.** `ReadWorld` and `ReadGovernance` are `AlwaysAllowed`, for the reason
//!   the record gives for its own checks (*"a permission that verification itself needs is a permission
//!   that can be withdrawn from verification"*): a world a caller may not read is a world it can
//!   already see, because the world is a pure function of its identity, and gating the read would only
//!   hide the refusal rather than prevent the act.
//! * **`RaiseToast` is `AlwaysAllowed`, not gated.** The shell's queue is how a refused act becomes
//!   *visible* (the never-silent rule); a matrix in which the only way to speak is gated would make
//!   every refusal silent.
//! * **`VerifyPackage` and `LintLine` are ungated** because they are the checks everything else depends
//!   on — the same reading as the reads, applied to the conformance seam.
//! * **`CompileRecipe` is `AlwaysAllowed`.** Compiling declared text is a pure function of that text.
//! * **`AppendEvent` is gated**, and it is the act a derivation's *only* record effect amounts to — the
//!   correction §67.4 records, where a stage→action map that sent a chain write to an
//!   `AlwaysAllowed` action was found to be **factually wrong** and corrected so that *"the chain
//!   write needs authority"*. In this tree the row that changes the record is `geometry`, and its act
//!   is this one.
//! * **`DeleteContent` is carried and denied unconditionally.** Nothing in this build deletes; the
//!   action is in the vocabulary anyway, because *an act that cannot be named cannot be shown to be
//!   forbidden* — and a policy that could not show the forbidding would be a policy a reader has to
//!   take on trust.
//! * **`WriteExternal` is denied unconditionally** because it is not implemented: sending, publishing,
//!   purchasing and account changes have **no code path** in this build, and a class is not a promise
//!   to add one.
//! * **`PersistMaterial` is a *rejection rather than a permission*** (`never_store`): the material may
//!   be used and never persisted. That is a different verdict from *denied* — it refuses the storage,
//!   not the use — so it is a class of its own rather than a second spelling of `AlwaysDenied`.
//!
//! # Who acts, and who says so
//!
//! A [`Caller`] is an actor and the grants it holds. The reference world's own caller is declared once
//! in [`crate::reference`], beside its identity and its ruleset, so the binary, its evidence and its
//! tests cannot drift into three similar sessions — and a `Gate` **cannot be built without one**, for
//! the reason it cannot be built without a ruleset: a gate with no declared actor would be a gate
//! answering as nobody. A grant's **granter** is the authority that authorises the scope, named as a
//! document or a role; in this tree authority is always traced to a document (P1), and the audit line a
//! denial leaves on the record carries the same names a reader would have to trace anyway.

use crate::refusal::{Refusal, RefusalKind};

/// **The declared scopes** — what a grant can be *about*, closed. A scope is deliberately coarse: it
/// names the owner whose state the act touches (the record, the world's documents, the content the
/// gate admits), because a scope fine enough to be a permission per act would be a second copy of the
/// action vocabulary and two vocabularies can disagree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    /// Appending a governed event to the one chain, and anything whose only record effect is that.
    Record,
    /// Admitting a declared row through the content gate (a document read in place by reference).
    Content,
    /// Writing a world down at a declared reference, and reading one back from it.
    World,
}

impl Scope {
    /// Every scope this build declares, in one list, so the count and the vocabulary cannot drift.
    pub const ALL: [Scope; 3] = [Scope::Record, Scope::Content, Scope::World];

    /// How many scopes there are. Written down, and pinned to [`Scope::ALL`] by a test.
    pub const COUNT: usize = 3;

    /// The scope's one spelling on a line, in a grant and in a refusal's quote.
    pub const fn name(self) -> &'static str {
        match self {
            Scope::Record => "record",
            Scope::Content => "content",
            Scope::World => "world",
        }
    }

    /// Parse a declared scope, **refusing** anything else. `"everything"` is not a scope
    /// (`docs/HYPERGRILLING.md` §L-5), and it is not one here for the same reason it is not one there:
    /// a scope that means *all* is a scope that cannot be quoted back in a refusal.
    pub fn parse(text: &str) -> Option<Scope> {
        let text = text.trim();
        Scope::ALL.into_iter().find(|scope| scope.name() == text)
    }
}

/// **A grant**: one scope, and the authority that granted it. Both are required — a grant with no
/// granter and a grant with no (or unbounded) scope are not weak grants this build tolerates, they are
/// **refused construction** (`docs/HYPERGRILLING.md` §L-5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grant {
    granter: &'static str,
    scope: Scope,
}

impl Grant {
    /// **The one constructor**, from the declared text — the granter's name and the scope's name —
    /// because both faults L-5 names are *absent values*, and only a construction from text can refuse
    /// an absence. A blank granter refuses [`RefusalKind::GrantUnattributed`]; a scope naming nothing
    /// this build declares (including `"everything"`) refuses [`RefusalKind::GrantUnscoped`] with the
    /// declared list in the message.
    pub fn of(granter: &'static str, scope: &str) -> Result<Grant, Refusal> {
        if granter.trim().is_empty() {
            return Err(Refusal::new(
                RefusalKind::GrantUnattributed,
                format!(
                    "a grant for the `{scope}` scope names no granter — an authorization with no \
                     author is not an authorization"
                ),
            ));
        }
        match Scope::parse(scope) {
            Some(scope) => Ok(Grant { granter, scope }),
            None => Err(Refusal::new(
                RefusalKind::GrantUnscoped,
                format!(
                    "`{scope}` is not a declared scope — the declared scopes are {}; a grant with no \
                     scope is refused construction, and 'everything' is not a scope",
                    declared_scopes()
                ),
            )),
        }
    }

    /// The authority that granted this scope, by name.
    pub const fn granter(self) -> &'static str {
        self.granter
    }

    /// What the grant is about.
    pub const fn scope(self) -> Scope {
        self.scope
    }
}

/// The declared scopes as one readable list — the form a refusal quotes.
pub fn declared_scopes() -> String {
    Scope::ALL
        .iter()
        .map(|scope| format!("`{}`", scope.name()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// **The class of an act.** Permission is a property of the *action*: [`permission_of`] returns one of
/// these from an [`Action`] alone, so no caller and no tool can widen it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionClass {
    /// Nothing is needed to do this; the act is a read of the world's own state or a check everything
    /// depends on. Declared deliberately rather than left out of the matrix.
    AlwaysAllowed,
    /// **Denied unconditionally**, and the reason is part of the class: the act is carried so the
    /// forbidding can be seen, and the denial quotes this sentence. No grant in any vocabulary reaches
    /// it, because the class is not a predicate over grants.
    AlwaysDenied(&'static str),
    /// Reachable only with a grant of this scope.
    Gated(Scope),
    /// **A rejection rather than a permission**: the material may be used and **never persisted**. Not
    /// `AlwaysDenied`, because the act it refuses is the *storage* rather than the use.
    NeverStore,
}

impl PermissionClass {
    /// What this class means, in words, for a reader of the matrix.
    pub fn describe(self) -> String {
        match self {
            PermissionClass::AlwaysAllowed => {
                "allowed: a read of the world's own state, or a check everything depends on".to_string()
            }
            PermissionClass::AlwaysDenied(reason) => {
                format!("denied unconditionally: {reason}")
            }
            PermissionClass::Gated(scope) => {
                format!("gated: requires a grant of the `{}` scope", scope.name())
            }
            PermissionClass::NeverStore => {
                "a rejection rather than a permission: this material may be used and never \
                 persisted"
                    .to_string()
            }
        }
    }
}

/// **The closed action vocabulary.** An act this build does not know about cannot be expressed, which
/// is how *deny by default* is made structural rather than a runtime hope (`docs/HYPERGRILLING.md`
/// §L-5: *"an action the module does not know about is denied"*; `docs/LEDGER.md` R-62.3).
///
/// Five of the thirteen are the acts this tree's own gate serves and are reached through one of
/// [`crate::bridge::TOOLS`]' rows; the other eight are named by the documents' own rulings and carry
/// the class that ruling gives them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    /// Reading the world's own state through the gate — the `truth`, `fields`, `held` and `record`
    /// rows. A read.
    ReadWorld,
    /// Appending a governed event to the one chain — reached by the `geometry` row, whose only effect
    /// on the record is the event it appends (`docs/LEDGER.md` §67.4).
    AppendEvent,
    /// Admitting a declared row through the content gate — the `admit` row.
    AdmitContent,
    /// Writing the world down at a declared reference — the `save` row.
    SaveWorld,
    /// Reading a world back from a declared reference — the `load` row.
    LoadWorld,
    /// Compiling declared text into a recipe. A pure function of that text.
    CompileRecipe,
    /// Verifying a package's own declared claims. A check everything else depends on.
    VerifyPackage,
    /// Linting a line of declared text. The same.
    LintLine,
    /// Showing a refusal to a player. The surface a refusal must be able to reach, so it is never
    /// gated.
    RaiseToast,
    /// Reading the governance on the record. A read.
    ReadGovernance,
    /// Deleting content. **Carried and denied**: nothing in this build deletes, and the act is named so
    /// that the forbidding can be shown rather than assumed.
    DeleteContent,
    /// Writing outside this world — sending, publishing, purchasing, account changes. **Denied because
    /// it is not implemented**: there is no code path, and a class is not a promise to add one.
    WriteExternal,
    /// Persisting material. **A rejection rather than a permission** (`never_store`).
    PersistMaterial,
}

/// How many actions this build declares. Written down, and pinned to [`Action::ALL`] by a test.
pub const ACTION_COUNT: usize = 13;

impl Action {
    /// Every action this build declares, in one list.
    pub const ALL: [Action; ACTION_COUNT] = [
        Action::ReadWorld,
        Action::AppendEvent,
        Action::AdmitContent,
        Action::SaveWorld,
        Action::LoadWorld,
        Action::CompileRecipe,
        Action::VerifyPackage,
        Action::LintLine,
        Action::RaiseToast,
        Action::ReadGovernance,
        Action::DeleteContent,
        Action::WriteExternal,
        Action::PersistMaterial,
    ];

    /// How many actions there are — the same number as [`ACTION_COUNT`], so a reader of the matrix has
    /// one figure rather than two.
    pub const COUNT: usize = ACTION_COUNT;

    /// The act's one spelling, in a refusal, on the record's audit line, and in the matrix's printout.
    pub const fn name(self) -> &'static str {
        match self {
            Action::ReadWorld => "read_world",
            Action::AppendEvent => "append_event",
            Action::AdmitContent => "admit_content",
            Action::SaveWorld => "save_world",
            Action::LoadWorld => "load_world",
            Action::CompileRecipe => "compile_recipe",
            Action::VerifyPackage => "verify_package",
            Action::LintLine => "lint_line",
            Action::RaiseToast => "raise_toast",
            Action::ReadGovernance => "read_governance",
            Action::DeleteContent => "delete_content",
            Action::WriteExternal => "write_external",
            Action::PersistMaterial => "persist_material",
        }
    }

    /// Parse, **refusing** anything else. An act outside this list is not a nearly-right act.
    pub fn parse(word: &str) -> Option<Action> {
        let word = word.trim();
        Action::ALL.into_iter().find(|action| action.name() == word)
    }

    /// **What this act is**, in words, for a reader of the API view. A matrix entry with nothing to say
    /// is a row a reader cannot understand, and [`check_matrix`] refuses one
    /// (`docs/LEDGER.md` §66.3's readable-API requirement).
    pub const fn describes(self) -> &'static str {
        match self {
            Action::ReadWorld => "reading the world's own state through the gate",
            Action::AppendEvent => "appending one governed event to the record",
            Action::AdmitContent => "admitting a declared row through the content gate",
            Action::SaveWorld => "writing the world down at a declared reference",
            Action::LoadWorld => "reading a world back from a declared reference",
            Action::CompileRecipe => "compiling declared text into a recipe",
            Action::VerifyPackage => "verifying a package's own declared claims",
            Action::LintLine => "linting a line of declared text",
            Action::RaiseToast => "showing a refusal to a player",
            Action::ReadGovernance => "reading the governance on the record",
            Action::DeleteContent => "deleting content",
            Action::WriteExternal => "writing outside this world",
            Action::PersistMaterial => "persisting material",
        }
    }
}

/// **The matrix.** One class per action, as an exhaustive `match` with **no wildcard arm** — so a
/// missing entry is a compile error (`E0004`) rather than a runtime deny nobody notices. This
/// function takes an [`Action`] and nothing else: permission is a property of the action, and there is
/// no parameter through which a caller or a tool could widen it.
pub const fn permission_of(action: Action) -> PermissionClass {
    match action {
        // Reads and the checks everything depends on.
        Action::ReadWorld => PermissionClass::AlwaysAllowed,
        Action::ReadGovernance => PermissionClass::AlwaysAllowed,
        Action::VerifyPackage => PermissionClass::AlwaysAllowed,
        Action::LintLine => PermissionClass::AlwaysAllowed,
        Action::RaiseToast => PermissionClass::AlwaysAllowed,
        Action::CompileRecipe => PermissionClass::AlwaysAllowed,
        // The acts that change what the world holds or where it is written.
        Action::AppendEvent => PermissionClass::Gated(Scope::Record),
        Action::AdmitContent => PermissionClass::Gated(Scope::Content),
        Action::SaveWorld => PermissionClass::Gated(Scope::World),
        Action::LoadWorld => PermissionClass::Gated(Scope::World),
        // Carried so the forbidding can be shown, and denied because the act does not exist here.
        Action::DeleteContent => {
            PermissionClass::AlwaysDenied("nothing in this build deletes: content is archived")
        }
        Action::WriteExternal => PermissionClass::AlwaysDenied(
            "nothing outside this world is written: the act has no code path in this build",
        ),
        // A rejection rather than a permission.
        Action::PersistMaterial => PermissionClass::NeverStore,
    }
}

/// **The one check of the matrix itself**: every action must have something to say. A row a reader
/// cannot understand is refused by name rather than printed empty
/// (`docs/LEDGER.md` §66.3 — *"an action with no description is a row a reader cannot understand, and
/// `check_matrix` refuses one"*).
pub fn check_matrix() -> Result<(), Refusal> {
    for action in Action::ALL {
        if action.describes().trim().is_empty() {
            return Err(Refusal::new(
                RefusalKind::ActionUndescribed,
                format!(
                    "`{}` is in the matrix and describes nothing — a row a reader cannot understand \
                     is refused rather than printed",
                    action.name()
                ),
            ));
        }
    }
    Ok(())
}

/// **Who is acting, and with what authority.** A caller is an actor's name and the grants it holds; a
/// caller with no grants is a legal value (a reader), and every act it is not granted will refuse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caller {
    actor: &'static str,
    grants: Vec<Grant>,
}

impl Caller {
    /// **The one constructor.** An unnamed actor refuses [`RefusalKind::GrantUnattributed`] for the
    /// same reason a grant with no granter does — an authorization with no author is not an
    /// authorization, and an act with no actor is not attributable on the record either.
    pub fn of(actor: &'static str, grants: Vec<Grant>) -> Result<Caller, Refusal> {
        if actor.trim().is_empty() {
            return Err(Refusal::new(
                RefusalKind::GrantUnattributed,
                "a caller names no actor — an authorization with no author is not an authorization, \
                 and an act with no actor cannot be recorded as anyone's",
            ));
        }
        Ok(Caller { actor, grants })
    }

    /// The actor's name, as the record's audit line spells it.
    pub const fn actor(&self) -> &'static str {
        self.actor
    }

    /// The grants this caller holds, in declaration order.
    pub fn grants(&self) -> &[Grant] {
        &self.grants
    }

    /// Whether this caller holds **any** grant of the given scope. A scope held twice is still one
    /// scope: holding more grants cannot be less authority.
    pub fn holds(&self, scope: Scope) -> bool {
        self.grants.iter().any(|grant| grant.scope() == scope)
    }
}

/// The scopes this caller holds, quoted for a refusal — *"a refusal quotes the list"*. `None` reads as
/// `no scopes`, so the quote is never a blank.
pub fn held_scopes(caller: &Caller) -> String {
    if caller.grants().is_empty() {
        return "no scopes".to_string();
    }
    caller
        .grants()
        .iter()
        .map(|grant| format!("`{}` (granted by `{}`)", grant.scope().name(), grant.granter()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// **The one decision function.** `Ok(())` when the act may be performed; a named refusal otherwise,
/// with the list quoted. Everything that acts goes through this — the gate consults it at exactly one
/// place before it dispatches, so *"the one call site nobody remembered"* has one place to be.
///
/// The three ways an act can be refused are three different facts, so they read as three: a gated act
/// with no grant names the scope it needed and what the caller holds; an unconditional denial quotes
/// the class's own reason; a `never_store` act says the material may be used and not persisted.
pub fn decide(caller: &Caller, action: Action) -> Result<(), Refusal> {
    match permission_of(action) {
        PermissionClass::AlwaysAllowed => Ok(()),
        PermissionClass::Gated(scope) => {
            if caller.holds(scope) {
                Ok(())
            } else {
                Err(Refusal::new(
                    RefusalKind::PolicyDenied,
                    format!(
                        "`{}` may not `{}`: it requires a grant of the `{}` scope and `{}` holds {} \
                         — the declared scopes are {}",
                        caller.actor(),
                        action.name(),
                        scope.name(),
                        caller.actor(),
                        held_scopes(caller),
                        declared_scopes()
                    ),
                ))
            }
        }
        PermissionClass::AlwaysDenied(reason) => Err(Refusal::new(
            RefusalKind::PolicyDenied,
            format!(
                "`{}` may not `{}`: the act is denied unconditionally to every caller — {reason}",
                caller.actor(),
                action.name()
            ),
        )),
        PermissionClass::NeverStore => Err(Refusal::new(
            RefusalKind::NeverStore,
            format!(
                "`{}` may not `{}`: this material may be used and never persisted, so the storage is \
                 rejected rather than permitted",
                caller.actor(),
                action.name()
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller(actor: &'static str, grants: &[(&'static str, &str)]) -> Caller {
        let grants = grants
            .iter()
            .map(|(granter, scope)| Grant::of(granter, scope).expect("a declared grant"))
            .collect();
        Caller::of(actor, grants).expect("a named actor")
    }

    #[test]
    fn the_vocabularies_are_closed_and_counted() {
        assert_eq!(Action::ALL.len(), ACTION_COUNT);
        assert_eq!(Action::COUNT, ACTION_COUNT);
        assert_eq!(Scope::ALL.len(), Scope::COUNT);
        // Every name round-trips, and no name is declared twice.
        for (i, action) in Action::ALL.iter().enumerate() {
            assert_eq!(Action::parse(action.name()), Some(*action), "{action:?}");
            assert!(
                Action::ALL[..i].iter().all(|other| other.name() != action.name()),
                "duplicate name {}",
                action.name()
            );
            // Every action describes itself, and the matrix check agrees.
            assert!(!action.describes().is_empty(), "{action:?}");
        }
        for (i, scope) in Scope::ALL.iter().enumerate() {
            assert_eq!(Scope::parse(scope.name()), Some(*scope));
            assert!(Scope::ALL[..i].iter().all(|other| other.name() != scope.name()));
        }
        assert_eq!(check_matrix(), Ok(()));
    }

    #[test]
    fn a_name_outside_the_vocabulary_is_not_an_action() {
        for word in ["", " ", "execute", "*", "all", "everything", "READ_WORLD"] {
            assert_eq!(Action::parse(word), None, "`{word}` is not an action");
            assert_eq!(Scope::parse(word), None, "`{word}` is not a scope");
        }
    }

    #[test]
    fn a_grant_with_no_granter_or_no_scope_is_refused_at_construction() {
        let unattributed = Grant::of("", "record").expect_err("no granter");
        assert_eq!(unattributed.kind(), RefusalKind::GrantUnattributed);
        let blank = Grant::of("   ", "record").expect_err("a blank granter");
        assert_eq!(blank.kind(), RefusalKind::GrantUnattributed);

        for scope in ["", "everything", "all", "*", "Record"] {
            let unscoped = Grant::of("operator", scope).expect_err("no declared scope");
            assert_eq!(unscoped.kind(), RefusalKind::GrantUnscoped, "`{scope}`");
            assert!(
                unscoped.detail().contains("everything"),
                "the refusal says why unbounded is not a scope: {}",
                unscoped.detail()
            );
        }

        // And a named actor holding an undeclared grant is not a caller this build can build either.
        let unnamed = Caller::of("", vec![]).expect_err("no actor");
        assert_eq!(unnamed.kind(), RefusalKind::GrantUnattributed);
    }

    #[test]
    fn permission_is_a_property_of_the_action() {
        // The same action, decided for two callers with different grants: the *class* is the same
        // value, and only the decision differs. Nothing about the tool or the caller reaches
        // `permission_of`.
        let gated: Action = Action::AdmitContent;
        assert_eq!(permission_of(gated), PermissionClass::Gated(Scope::Content));
        assert_eq!(permission_of(gated), permission_of(Action::parse("admit_content").unwrap()));

        let operator = caller("operator", &[("the operator's declaration", "content")]);
        let reader = caller("reader", &[]);
        assert_eq!(decide(&operator, gated), Ok(()));
        assert!(decide(&reader, gated).is_err());
        // The reads need nothing from either, so a reader is a reader.
        assert_eq!(decide(&reader, Action::ReadWorld), Ok(()));
    }

    #[test]
    fn deny_by_default_and_every_denial_quotes_the_list() {
        let reader = caller("reader", &[]);
        // A gated act with no grant: the quote names the act, the scope and what is held.
        let denial = decide(&reader, Action::SaveWorld).expect_err("no grant");
        assert_eq!(denial.kind(), RefusalKind::PolicyDenied);
        assert!(denial.detail().contains("save_world"), "{}", denial.detail());
        assert!(denial.detail().contains("`world`"), "{}", denial.detail());
        assert!(denial.detail().contains("no scopes"), "{}", denial.detail());
        assert!(denial.detail().contains("`record`"), "the declared list", );

        // A partial grant quotes what the caller *does* hold, with the granter named.
        let partial = caller("clerk", &[("docs/DESIGN-SPEC.md §2 F1", "record")]);
        let denial = decide(&partial, Action::LoadWorld).expect_err("no world grant");
        assert!(denial.detail().contains("granted by `"), "{}", denial.detail());
        assert!(denial.detail().contains("docs/DESIGN-SPEC.md §2 F1"), "{}", denial.detail());

        // The unconditional denials quote the class's own reason, and no grant changes that.
        let everything = caller(
            "root",
            &[("the operator", "record"), ("the operator", "content"), ("the operator", "world")],
        );
        let deletion = decide(&everything, Action::DeleteContent).expect_err("denied");
        assert_eq!(deletion.kind(), RefusalKind::PolicyDenied);
        assert!(deletion.detail().contains("unconditionally"), "{}", deletion.detail());
        assert!(deletion.detail().contains("archived"), "{}", deletion.detail());
        let external = decide(&everything, Action::WriteExternal).expect_err("denied");
        assert!(external.detail().contains("no code path"), "{}", external.detail());

        // And a rejection is not a denial: `never_store` refuses the storage, not the use.
        let storage = decide(&everything, Action::PersistMaterial).expect_err("rejected");
        assert_eq!(storage.kind(), RefusalKind::NeverStore);
        assert!(storage.detail().contains("never persisted"), "{}", storage.detail());
    }

    #[test]
    fn the_ungated_acts_are_the_ones_the_documents_say_are_ungated() {
        // Named one by one rather than counted, so a class change has to be made here as well.
        for action in [
            Action::ReadWorld,
            Action::ReadGovernance,
            Action::VerifyPackage,
            Action::LintLine,
            Action::RaiseToast,
            Action::CompileRecipe,
        ] {
            assert_eq!(
                permission_of(action),
                PermissionClass::AlwaysAllowed,
                "{} is a read or a check everything depends on",
                action.name()
            );
        }
        assert_eq!(permission_of(Action::AppendEvent), PermissionClass::Gated(Scope::Record));
        assert_eq!(permission_of(Action::AdmitContent), PermissionClass::Gated(Scope::Content));
        assert_eq!(permission_of(Action::SaveWorld), PermissionClass::Gated(Scope::World));
        assert_eq!(permission_of(Action::LoadWorld), PermissionClass::Gated(Scope::World));
        assert_eq!(permission_of(Action::PersistMaterial), PermissionClass::NeverStore);
        assert!(matches!(permission_of(Action::DeleteContent), PermissionClass::AlwaysDenied(_)));
        assert!(matches!(permission_of(Action::WriteExternal), PermissionClass::AlwaysDenied(_)));
        // Six allowed, four gated, one rejection, two denied: the matrix is counted, not surveyed.
        let counts = [
            (PermissionClass::AlwaysAllowed, 6usize),
            (PermissionClass::Gated(Scope::Record), 1usize),
            (PermissionClass::Gated(Scope::Content), 1usize),
            (PermissionClass::Gated(Scope::World), 2usize),
            (PermissionClass::NeverStore, 1usize),
        ];
        let denied = Action::ALL
            .iter()
            .filter(|action| matches!(permission_of(**action), PermissionClass::AlwaysDenied(_)))
            .count();
        assert_eq!(denied, 2);
        for (class, expected) in counts {
            let found = Action::ALL.iter().filter(|a| permission_of(**a) == class).count();
            assert_eq!(found, expected, "{class:?}");
        }
    }

    #[test]
    fn a_scope_held_twice_is_still_one_scope() {
        let doubled = caller(
            "operator",
            &[("the operator's declaration", "world"), ("a second declaration", "world")],
        );
        assert!(doubled.holds(Scope::World));
        assert_eq!(doubled.grants().len(), 2);
        assert!(decide(&doubled, Action::SaveWorld).is_ok());
        // Holding one scope is not holding another.
        assert!(!doubled.holds(Scope::Content));
        assert!(decide(&doubled, Action::AdmitContent).is_err());
    }
}
