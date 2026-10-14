//! The surface as a client of the api layer: **the bridge's gate, the chain, the api viewer.**
//!
//! Nothing here tests a text reply on its own. Every assertion is about one of the three things the
//! game declares — coverage/capability/matrix, the append-only record, the rendered document — or
//! about the fact that a refused call changes nothing.

use std::path::PathBuf;

use serde_json::json;
use voxel_mcp::{
    api_view, call_with, permission_of_name, receipt, record_rejections, row_of, TOOLS,
};
use voxel_model::chain::{Chain, EventKind, Linkage, Namespace, Season};
use voxel_model::grant::Grant;
use voxel_package::bridge::{self, Authority, Capability};
use voxel_package::census;
use voxel_package::conformance;
use voxel_package::policy::{self, PermissionClass};
use voxel_package::{bloat, filing};

/// The tools that read, and the filing tool that needs an author.
fn reads() -> impl Iterator<Item = &'static str> {
    TOOLS.into_iter().filter(|name| *name != "reject_non_conformance")
}

/// The capability the bridge declares a served tool's row needs — read from the partition, never
/// assumed here.
fn capability_of(name: &str) -> Capability {
    bridge::tool(row_of(name).expect("a row"))
        .expect("the row is in the table")
        .capability()
        .expect("every served row names a capability")
}

fn namespace() -> Namespace {
    Namespace::new("shploingus", 0, Season::Spring, "retrofit").expect("a named namespace")
}

#[test]
fn nothing_is_implicit_including_reading() {
    // A request that presents nothing gets nothing: the refusal is the bridge's, and it names the
    // capability the row needs rather than reporting a fact about the tool.
    for name in TOOLS {
        let refused = receipt(name, &Authority::none(), None);
        assert!(!refused.ran(), "{name} ran with no capability presented");
        assert!(refused.is_unauthorized(), "{name} was not refused for want of a capability");
        assert!(
            refused.line().contains("refused"),
            "a refusal reads as a refusal: {}",
            refused.line()
        );
    }
    // And **each read runs under the capability the bridge says its row needs** — asserted against
    // the partition rather than against a guess. The first version of this test hardcoded `observe`
    // for every read and failed the moment `judge_work` was added, because verifying a package needs
    // `present`: the bridge is right and the test was asserting a belief.
    for name in reads() {
        let needed = capability_of(name);
        let permitted = receipt(name, &Authority::none().grant(needed), None);
        assert!(
            permitted.ran(),
            "{name} does not run under its own row's capability {}: {}",
            needed.name(),
            permitted.line()
        );
    }
    // The one read that is not `observe`, named so a later change to either side fails here.
    let judged = bridge::tool("verify_package").expect("verify_package is a row");
    assert_eq!(judged.capability(), Some(Capability::Present));
    assert_eq!(row_of("judge_work"), Some("verify_package"));
}

#[test]
fn the_filing_tool_is_the_bridge_row_append_event_and_the_matrix_gates_it() {
    assert_eq!(row_of("reject_non_conformance"), Some("append_event"));
    let row = bridge::tool("append_event").expect("append_event is a row");
    assert_eq!(row.action, policy::Action::AppendEvent);
    assert_eq!(
        policy::permission_of(row.action),
        PermissionClass::RequiresGrant,
        "filing onto the chain must need an author"
    );

    // No grant: refused by the matrix, in the matrix's own words.
    let refused = receipt("reject_non_conformance", &Authority::full(), None);
    assert!(!refused.ran());
    let line = refused.line();
    assert!(line.contains("append_event"), "{line}");
    assert!(
        line.contains("silence is not permission"),
        "the refusal is the matrix's own sentence: {line}"
    );
    // The capability check is the one that ran before it — a check that did not run is not a check
    // that passed, so the receipt names what it actually did.
    assert!(line.contains("capability_held"), "{line}");

    let grant = Grant::new("ala", "conformance/reject").expect("a scoped, attributed grant");
    let permitted = receipt("reject_non_conformance", &Authority::full(), Some(&grant));
    assert!(permitted.ran(), "{}", permitted.line());
}

#[test]
fn a_tool_the_surface_does_not_carry_is_refused_by_coverage_never_by_a_fallthrough() {
    let refused = receipt("get_vibes", &Authority::full(), None);
    assert!(!refused.ran());
    let line = refused.line();
    assert!(line.contains("no tool named \"get_vibes\""), "{line}");
    assert!(line.contains("never an allow"), "{line}");
    assert!(
        line.contains("checks ran: none"),
        "coverage did not run, so it cannot have passed: {line}"
    );
}

#[test]
fn a_grant_must_name_both_an_author_and_a_scope() {
    // `Grant::new` is the tree's constructor and its refusals are carried out, not defaulted.
    let why = Grant::new("", "conformance/reject")
        .err()
        .map(|found| format!("{found}"))
        .expect("a blank granter is refused");
    assert!(why.contains("granter") || why.contains("author"), "{why}");
    let why = Grant::new("ala", "   ")
        .err()
        .map(|found| format!("{found}"))
        .expect("a blank scope is refused");
    assert!(why.contains("scope"), "{why}");
}

#[test]
fn a_refused_filing_changes_nothing_on_disk() {
    let root = voxel_mcp::workspace_root();
    let chain_file = filing::chain_path(&root);
    let before = std::fs::read(&chain_file).ok();

    let why = call_with(
        &root,
        "reject_non_conformance",
        &json!({}),
        &Authority::full(),
        None,
    )
    .expect_err("a filing with no grant is refused");
    assert!(why.contains("append_event") && why.contains("silence is not permission"), "{why}");

    let after = std::fs::read(&chain_file).ok();
    assert_eq!(
        before.is_some(),
        after.is_some(),
        "the refused call created or removed the chain file"
    );
    if let (Some(before), Some(after)) = (before, after) {
        assert_eq!(before, after, "the refused call wrote to the record");
    }
}

#[test]
fn the_rejections_become_refusal_recorded_events_chained_to_the_chain_they_join() {
    let root = voxel_mcp::workspace_root();
    // **The mechanism is exercised against a door with nothing accepted**, which is the state it
    // exists for — but the refused cells are now **declared by this test instead of read off the
    // tree**, because reading them off the tree made this test a statement about the retrofit rather
    // than about the mechanism. It broke twice for that one reason: when
    // `conformance/accepted-refusals.txt` landed (2026-10-09) every refusal was accepted by name, so
    // `rejected()` was empty and this failed on a tree whose door was working exactly as declared;
    // then the sixteen `documented` sections landed (2026-10-10), the last refused cell in the tree
    // became `inactive`, and it failed again with nothing left in the tree to reject. The rule's
    // *input* is the rule's business; what this file is for is that a refused cell becomes a
    // chained event.
    let verdict = conformance::Verdict {
        refusals: vec![
            conformance::Refusal {
                crate_: "voxel-model".to_string(),
                axis: "documented".to_string(),
                state: "refused".to_string(),
                why: "a refused cell, declared here rather than read from the tree".to_string(),
            },
            conformance::Refusal {
                crate_: "voxel-client".to_string(),
                axis: "tested".to_string(),
                state: "refused".to_string(),
                why: "a second one, so the events have an order to keep".to_string(),
            },
        ],
        accepted: Vec::new(),
        unchecked: Vec::new(),
        stale: Vec::new(),
        unmatched: Vec::new(),
        accepted_declared: false,
        tested: census::CRATE_COUNT,
    };
    let rejected = verdict.rejected();
    assert_eq!(rejected.len(), 2, "the fixture's own two refusals");
    // **And one live reading, kept so this test cannot pass against a census that stopped walking.**
    // How many subjects the door reads does not move with the retrofit; *what it refuses* only ever
    // did. Sixteen members, sixteen readings.
    let live = census::census(&root).expect("the census walks the tree");
    assert_eq!(
        conformance::of(&live, &[], false).tested,
        census::CRATE_COUNT,
        "the door measured a different number of subjects than the workspace has members"
    );

    // Read the chain back from the workspace **before** appending, which is what the writer does: an
    // event linked to a genesis that is not its predecessor is refused on read-back.
    let mut chain = bloat::read_chain(&root.join(filing::CHAIN_DIR)).expect("the chain reads back");
    let before_len = chain.len();
    let text = record_rejections(&mut chain, &namespace(), &verdict);

    assert!(
        text.contains(&format!("reject-recording\t{}", rejected.len())),
        "every rejected cell is recorded: {text}"
    );
    assert!(text.contains("reject-created\trefusal_recorded"), "{text}");
    assert_eq!(chain.len(), before_len + rejected.len(), "one event per rejected cell");

    let events: Vec<_> = {
        let mut collected = Vec::new();
        chain.fold((), |(), event| collected.push(event.clone()));
        collected
    };
    let mut digests = std::collections::BTreeSet::new();
    for (offset, event) in events[before_len..].iter().enumerate() {
        assert_eq!(event.kind, EventKind::RefusalRecorded, "the chain's own kind for it");
        assert_eq!(event.index as usize, before_len + offset);
        // **Chained, not all claiming the genesis**: each new event names the one before it, and the
        // first names the chain's real head.
        let predecessor = &events[event.index as usize - 1];
        assert_eq!(
            event.previous,
            Linkage::Linked(predecessor.digest),
            "event {} does not name its predecessor",
            event.index
        );
        assert!(digests.insert(event.digest), "two events share a digest");
        assert!(!event.detail.trim().is_empty(), "a blank line records nothing");
        // The detail carries the cell and how the reading was obtained, in the ladder's own words.
        assert!(event.detail.contains('\t'), "{}", event.detail);
        assert!(event.detail.contains("origin B"), "{}", event.detail);
    }
    // And the set of cells recorded is the set the rule refused.
    let recorded: Vec<String> = events[before_len..]
        .iter()
        .map(|event| event.detail.split('\t').next().unwrap_or_default().to_string())
        .collect();
    let mut expected: Vec<String> = rejected.iter().map(|refusal| refusal.key()).collect();
    expected.sort();
    let mut recorded_sorted = recorded.clone();
    recorded_sorted.sort();
    assert_eq!(recorded_sorted, expected, "the chain records a different set than the rule refused");
}

#[test]
fn the_api_view_is_rendered_from_declarations_and_cannot_advertise_a_permission_it_lacks() {
    let document = api_view();
    assert!(document.contains(&format!("tools\t{}", TOOLS.len())), "{document}");
    for name in TOOLS {
        let row = row_of(name).expect("every served tool names a bridge row");
        let tool = bridge::tool(row).expect("the row is in the bridge's own table");
        let line = document
            .lines()
            .find(|line| line.starts_with(&format!("tool\t{name}\t")))
            .unwrap_or_else(|| panic!("{name} is not in the view:\n{document}"));
        assert!(line.contains(&format!("row\t{row}")), "{line}");
        assert!(line.contains(&format!("action\t{}", tool.action.name())), "{line}");
        assert!(
            line.contains(&format!("class\t{}", permission_of_name(tool.action))),
            "the view advertises a class the matrix does not hold: {line}"
        );
    }
    let filing_line = document
        .lines()
        .find(|line| line.starts_with("tool\treject_non_conformance\t"))
        .expect("the filing tool is documented");
    assert!(filing_line.contains("requires_grant"), "{filing_line}");
    assert!(document.contains("coverage, then capability, then voxel-package::policy"), "{document}");
}

#[test]
fn the_game_s_per_run_queue_is_out_of_reach_of_this_crate() {
    // The regression this rewrite fixes, as a fact about what the crate can even name:
    // `logs/laya-decisions/` is the shell's stream, the game's pass clears it, and its records are
    // named by position. So the crate must not link the module that writes it — a doc comment about
    // the mistake is fine, a call to `laya::write_one` is not.
    let source = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs"),
    )
    .expect("this crate's own source reads");
    assert!(
        !source.contains("use voxel_model::laya"),
        "the surface links laya again, which is how it filed into the queue the game clears"
    );
    for call in ["laya::write_one", "laya::write_notification", "laya::queue_dir("] {
        assert!(!source.contains(call), "the surface calls {call}");
    }
    // And it reads no queue either: the chain is the record, and the projection is the game's to
    // make. (The header *mentions* the shell's reader — that is the mistake being documented.)
    assert!(
        !source.contains("NotificationStream::"),
        "the surface reads the game's projection instead of the record"
    );
    let _ = Chain::new();
}
