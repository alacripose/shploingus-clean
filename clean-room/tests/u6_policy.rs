//! **U6's policy chokepoint, driven through the public surface** — `specs/005-u6-record-api`, and
//! `docs/HYPERGRILLING.md` §L-5 / `docs/LEDGER.md` R-62.3 as the documents it was derived from.
//!
//! The unit tests inside `src/policy.rs` check the matrix and the construction rules. This file checks
//! the two things only a *running gate* can show: that every row of the one tool table names an act
//! whose class is declared, and that the one call site actually bites — an actor holding nothing is
//! refused **by name**, **recorded**, and the record still verifies.

use shploingus_clean_room::bridge;
use shploingus_clean_room::policy;
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::refusal::RefusalKind;
use shploingus_clean_room::save::Ruleset;
use shploingus_clean_room::stage::Declared;

fn opened_gate(roster: &Declared) -> bridge::Gate<'_> {
    let mut gate = reference::gate(roster, 1).expect("the reference declaration is admitted");
    let root = std::env::temp_dir().join("shploingus-u6-policy-tests");
    let _ = std::fs::create_dir_all(&root);
    gate.set_intake_root(&root);
    gate
}

/// **Every declared door names one act, and every act is a row a reader can understand.** The mapping
/// lives on the row (`Tool::action`) rather than in a second table, so this test can only fail if a row
/// was added without its act — which is the property `docs/HYPERGRILLING.md` §L-5 calls the injection
/// boundary: *"the declared tool list is exhaustive, and every declared tool maps to exactly one
/// action."*
#[test]
fn every_declared_door_names_one_act_and_every_act_is_readable() {
    assert_eq!(bridge::TOOLS.len(), bridge::TOOL_COUNT);
    for tool in bridge::TOOLS.iter() {
        let action = tool.action;
        assert!(!action.describes().trim().is_empty(), "{}: no description", tool.name);
        assert_eq!(
            policy::Action::parse(action.name()),
            Some(action),
            "{} names an act that does not parse back",
            tool.name
        );
    }
    // The five acts the table's eight rows reach, named one by one so a row's act cannot drift silently.
    let reached: Vec<policy::Action> = bridge::TOOLS.iter().map(|tool| tool.action).collect();
    for (verb, action) in [
        ("truth", policy::Action::ReadWorld),
        ("fields", policy::Action::ReadWorld),
        ("geometry", policy::Action::AppendEvent),
        ("held", policy::Action::ReadWorld),
        ("record", policy::Action::ReadWorld),
        ("admit", policy::Action::AdmitContent),
        ("save", policy::Action::SaveWorld),
        ("load", policy::Action::LoadWorld),
    ] {
        let row = bridge::tool(verb).expect("a declared verb");
        assert_eq!(row.action, action, "`{verb}`");
        assert!(reached.contains(&action));
    }
    assert_eq!(policy::check_matrix(), Ok(()));
    // And the four gated rows are exactly the rows whose act is gated: a row cannot be gated by
    // accident, and a gated act cannot be reached by a row that declares another.
    let gated_tools: Vec<&str> = bridge::TOOLS
        .iter()
        .filter(|tool| matches!(policy::permission_of(tool.action), policy::PermissionClass::Gated(_)))
        .map(|tool| tool.name)
        .collect();
    assert_eq!(gated_tools, vec!["geometry", "admit", "save", "load"]);
}

/// **A class is a property of the act.** The gate asks `permission_of` about the act its row declares and
/// nothing else — so the same act has one class whichever door it is reached through, and the door's
/// identity never widens it. This test states that from the outside: the two reads are
/// `AlwaysAllowed` whether they are reached through `truth` or through `held`, and `geometry`'s act is
/// gated even though a derivation looks like work rather than a write.
#[test]
fn a_class_is_a_property_of_the_act_and_not_of_the_door() {
    let truth = bridge::tool("truth").expect("declared");
    let held = bridge::tool("held").expect("declared");
    assert_eq!(truth.action, held.action);
    assert_eq!(policy::permission_of(truth.action), policy::PermissionClass::AlwaysAllowed);
    assert_eq!(
        policy::permission_of(bridge::tool("geometry").expect("declared").action),
        policy::PermissionClass::Gated(policy::Scope::Record)
    );
    // Every act in the vocabulary has a class — there is no act without one, and no wildcard arm.
    for action in policy::Action::ALL {
        let class = policy::permission_of(action);
        assert!(!class.describe().is_empty(), "{}", action.name());
    }
}

/// **Deny by default, at the one call site.** The reference operator serves the gated acts; an actor
/// holding nothing is refused **by name** — and the refusal is **recorded**, because a refusal nobody
/// can read is a refusal that did not happen. The reads still answer: a read is not an act anyone can
/// be kept out of.
#[test]
fn a_caller_with_no_grants_is_refused_by_name_and_the_refusal_is_recorded() {
    let roster = process::roster().expect("the reference roster");
    let mut gate = opened_gate(&roster);

    // As the reference operator: the gated acts answer.
    let saved = gate
        .call("save path=tests-policy.save")
        .expect("the operator holds the `world` scope");
    let digest = match &saved.answer {
        bridge::Answer::Saved(facts) => facts.digest.to_hex(),
        other => panic!("`save` answered {other:?}"),
    };
    gate.call("geometry chunk=0,0,0 level=0")
        .expect("the operator holds the `record` scope");
    gate.call(&format!("load path=tests-policy.save digest={digest}"))
        .expect("the operator holds the `world` scope");

    // As a reader holding nothing.
    let before = gate.chain().len();
    gate.set_caller(policy::Caller::of("reader", Vec::new()).expect("a named actor"));
    for line in ["truth", "held", "record"] {
        assert!(gate.call(line).is_ok(), "a read is not gated: `{line}`");
    }
    let mut refusals = Vec::new();
    for line in [
        "geometry chunk=1,0,0 level=0",
        "save path=reader.save",
        &format!("load path=tests-policy.save digest={digest}"),
    ] {
        match gate.call(line) {
            Ok(_) => panic!("the reader was served `{line}`"),
            Err(refusal) => refusals.push(refusal),
        }
    }
    assert!(
        refusals.iter().all(|refusal| refusal.kind() == RefusalKind::PolicyDenied),
        "{refusals:?}"
    );
    // The refusal **quotes the list**: the act, the scope it requires, what the caller holds, and the
    // declared scopes.
    let quoted = refusals[0].detail();
    for fragment in ["`reader`", "append_event", "`record`", "no scopes", "the declared scopes are"] {
        assert!(quoted.contains(fragment), "`{fragment}` is not in: {quoted}");
    }
    // And each denial is on the record, which still verifies: a refusal nobody can read is a refusal
    // that did not happen.
    assert_eq!(gate.chain().len(), before + refusals.len(), "every denial is recorded");
    assert!(gate.chain().verify().is_ok());
}

/// **Grants are scoped and attributed, refused at construction.** A grant with no author and a grant
/// whose scope is unbounded or undeclared are not weak grants this build tolerates — they are values it
/// cannot hold.
#[test]
fn a_grant_with_no_granter_or_no_scope_is_refused_construction() {
    assert_eq!(
        policy::Grant::of("", "record").expect_err("no granter").kind(),
        RefusalKind::GrantUnattributed
    );
    assert_eq!(
        policy::Grant::of("operator", "everything").expect_err("no scope").kind(),
        RefusalKind::GrantUnscoped
    );
    assert_eq!(
        policy::Grant::of("operator", "worlds").expect_err("undeclared").kind(),
        RefusalKind::GrantUnscoped
    );
    assert_eq!(
        policy::Caller::of("", Vec::new()).expect_err("no actor").kind(),
        RefusalKind::GrantUnattributed
    );
    // A declared grant, held beside a second scope, is still only what it says it is.
    let operator = policy::Caller::of(
        "operator",
        vec![policy::Grant::of("specs/006-world-saves-and-seeds/spec.md", "world").expect("declared")],
    )
    .expect("a named actor");
    assert!(operator.holds(policy::Scope::World));
    assert!(!operator.holds(policy::Scope::Content));
    assert_eq!(policy::decide(&operator, policy::Action::SaveWorld), Ok(()));
    assert_eq!(
        policy::decide(&operator, policy::Action::AdmitContent)
            .expect_err("no content grant")
            .kind(),
        RefusalKind::PolicyDenied
    );
}

/// **The two unconditional denials hold for an actor that holds everything**, and a rejection is not a
/// denial, and the ungated acts are an **allowlist rather than an omission** — each one is named here,
/// so adding an act to the vocabulary has to be decided here as well as in the matrix.
#[test]
fn the_unconditional_denials_and_the_ungated_acts_are_declared_rather_than_omitted() {
    let mut grants = Vec::new();
    for scope in ["record", "content", "world"] {
        grants.push(policy::Grant::of("the operator's own declaration", scope).expect("declared"));
    }
    let everything = policy::Caller::of("root", grants).expect("a named actor");
    assert_eq!(
        policy::decide(&everything, policy::Action::DeleteContent)
            .expect_err("denied")
            .kind(),
        RefusalKind::PolicyDenied
    );
    assert_eq!(
        policy::decide(&everything, policy::Action::WriteExternal)
            .expect_err("denied")
            .kind(),
        RefusalKind::PolicyDenied
    );
    assert_eq!(
        policy::decide(&everything, policy::Action::PersistMaterial)
            .expect_err("rejected")
            .kind(),
        RefusalKind::NeverStore
    );
    for action in [
        policy::Action::ReadWorld,
        policy::Action::ReadGovernance,
        policy::Action::VerifyPackage,
        policy::Action::LintLine,
        policy::Action::RaiseToast,
        policy::Action::CompileRecipe,
    ] {
        assert_eq!(
            policy::permission_of(action),
            policy::PermissionClass::AlwaysAllowed,
            "{}",
            action.name()
        );
        assert_eq!(policy::decide(&everything, action), Ok(()));
    }
    // The vocabulary is closed: an act outside it is not a nearly-right act.
    for word in ["everything", "execute", "*", "all", "read_worlds"] {
        assert_eq!(policy::Action::parse(word), None, "`{word}`");
    }
}

/// **The reference session's own actor is declared once**, and a gate cannot be built without one — so
/// the binary, its evidence and its tests answer as the **same** actor rather than as three similar
/// ones. The reference caller's grants are attributed to the documents that authorise them, and the
/// session's ruleset identity is unchanged by any of this.
#[test]
fn the_reference_session_declares_who_acts_and_with_what_authority() {
    let caller = reference::caller().expect("the reference caller is declared");
    assert_eq!(caller.actor(), "operator");
    assert_eq!(caller.grants().len(), policy::Scope::COUNT);
    for scope in policy::Scope::ALL {
        assert!(caller.holds(scope), "`{}`", scope.name());
    }
    for grant in caller.grants() {
        assert!(!grant.granter().trim().is_empty(), "an unattributed grant");
        assert!(grant.granter().contains("specs/"), "{}", grant.granter());
    }
    // The gate answers as that caller, and its own declarations are untouched by the policy.
    let roster = process::roster().expect("the reference roster");
    let gate = reference::gate(&roster, 1).expect("the reference declaration");
    assert_eq!(gate.caller().actor(), "operator");
    assert_eq!(gate.ruleset(), Ruleset::new(7, 3));
    assert_eq!(gate.chain().len(), 0, "a session's record opens empty");
}
