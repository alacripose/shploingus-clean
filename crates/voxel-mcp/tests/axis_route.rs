//! **The axis route, driven the way a client drives it: lines in, replies out.**
//!
//! Nothing here reaches into the registry or calls a tool body directly. Every assertion is about a
//! reply the server produced from a JSON-RPC line, because the integration is the thing under test:
//! a tool that is listed but unreachable, a credential that parses but is not checked, or a refusal
//! that comes back as a success would all pass a unit test beside the dispatch and fail here.

use std::io::Cursor;

use serde_json::{json, Value};
use voxel_mcp::{serve, workspace_root, TOOLS};

const TICKET: &str = "T-0954";

/// Run a transcript and return one parsed reply per line, exactly as a client would read them.
fn transcript(requests: &[Value]) -> Vec<Value> {
    let root = workspace_root();
    let mut input = String::new();
    for request in requests {
        input.push_str(&request.to_string());
        input.push('\n');
    }
    let mut out = Vec::new();
    let mut sink = Cursor::new(&mut out);
    serve(&root, Cursor::new(input.as_bytes()), &mut sink).expect("the server reads its stdin");
    String::from_utf8(out)
        .expect("the protocol is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("every reply is one JSON object"))
        .collect()
}

fn call(name: &str, arguments: Value, capabilities: &[&str]) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": name, "arguments": arguments, "capabilities": capabilities}
    })
}

fn text_of(reply: &Value) -> String {
    reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn refused(reply: &Value) -> bool {
    reply["result"]["isError"].as_bool().unwrap_or(false)
}

/// The credential a `grant_recipe` reply carries: the one line that starts with the wire tag.
fn credential(reply: &Value) -> String {
    let text = text_of(reply);
    text.lines()
        .find(|line| line.starts_with("axis-grant/1\t"))
        .unwrap_or_else(|| panic!("no credential in {text}"))
        .to_string()
}

fn cell() -> Value {
    json!({"ticket": TICKET, "segment": "shell", "axis": "evidence_grade"})
}

#[test]
fn the_axis_route_is_listed_and_each_tool_declares_what_it_needs() {
    let replies = transcript(&[json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})]);
    let tools = replies[0]["result"]["tools"]
        .as_array()
        .expect("the tool list");
    for name in [
        "get_axis_matrix",
        "get_axis_recipe",
        "grant_recipe",
        "apply_recipe",
    ] {
        let tool = tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not listed"));
        assert!(
            !tool["description"].as_str().unwrap_or_default().is_empty(),
            "{name} has no description"
        );
        assert_eq!(tool["inputSchema"]["type"], "object", "{name} has no schema");
    }
    let apply = tools
        .iter()
        .find(|tool| tool["name"] == "apply_recipe")
        .expect("listed");
    assert_eq!(
        apply["inputSchema"]["required"],
        json!(["ticket", "segment", "axis", "ticket_grant"]),
        "the credential is a required argument, not an optional one"
    );
    assert_eq!(tools.len(), TOOLS.len());
}

#[test]
fn the_matrix_and_its_package_are_served_from_the_walk() {
    let replies = transcript(&[call("get_axis_matrix", json!({"ticket": TICKET}), &["observe"])]);
    assert!(!refused(&replies[0]), "{}", text_of(&replies[0]));
    let text = text_of(&replies[0]);
    // The format cell is the one cell outside `Segment::ALL` (PATCH-076); the count is derived, so
    // a new segment or a removed format cannot leave this test asserting yesterday's table.
    let cells = voxel_axis_matrix::Segment::ALL.len() + 1;
    assert!(text.contains(&format!("axis-matrix\tticket\t{TICKET}\tcell(s)\t{cells}")), "{text}");
    assert!(text.contains("shell"), "every segment is in the table: {text}");
    assert!(text.contains('★'), "the star is the matrix's own glyph: {text}");
    assert!(text.contains("axis-package\tpin\tsha256:"), "{text}");
    assert!(
        text.contains("# voxel package") && text.contains("## pin"),
        "the package is the format's own document: {text}"
    );
}

#[test]
fn reading_the_matrix_without_the_capability_is_refused_by_the_bridge() {
    let replies = transcript(&[call("get_axis_matrix", json!({"ticket": TICKET}), &[])]);
    assert!(refused(&replies[0]));
    let text = text_of(&replies[0]);
    assert!(text.contains("observe"), "the refusal names what is missing: {text}");
    assert!(text.contains("refused"), "{text}");
}

#[test]
fn a_recipe_is_the_packages_own_line_for_the_measured_cell() {
    // **The reading is the cell's, asked for rather than hard-coded.** This test used to assert the
    // letter `B`; the walk's grade for `shell` moved to `C` when the census's pins moved, and the
    // test then failed while the server was answering correctly. A grade written into the test is
    // an assertion about yesterday's tree, so the matrix is asked what the cell says and the
    // recipe is required to say the same.
    let replies = transcript(&[
        call("get_axis_matrix", json!({"ticket": TICKET}), &["observe"]),
        call("get_axis_recipe", cell(), &["observe"]),
    ]);
    assert!(!refused(&replies[0]), "{}", text_of(&replies[0]));
    assert!(!refused(&replies[1]), "{}", text_of(&replies[1]));
    let matrix = text_of(&replies[0]);
    let shell = matrix
        .lines()
        .find(|line| line.starts_with("shell "))
        .unwrap_or_else(|| panic!("shell is in the matrix: {matrix}"));
    let grade = shell
        .split("evidence_grade ")
        .nth(1)
        .and_then(|rest| rest.chars().next())
        .unwrap_or_else(|| panic!("the cell carries a grade: {shell}"));
    let text = text_of(&replies[1]);
    assert!(text.contains("axis-recipe\tT-0954\tshell\tevidence_grade"), "{text}");
    assert!(
        text.contains("crates/voxel-client/src/shell/mod.rs"),
        "the citation is the package's: {text}"
    );
    assert!(
        text.contains(&format!("evidence_grade {grade}")),
        "the ladder reading is the cell's ({grade}): {text}"
    );
}

#[test]
fn a_recipe_for_a_cell_that_was_not_measured_is_no_reading_not_a_success() {
    let replies = transcript(&[call(
        "get_axis_recipe",
        json!({"ticket": TICKET, "segment": "shell", "axis": "support_level"}),
        &["observe"],
    )]);
    assert!(refused(&replies[0]));
    let text = text_of(&replies[0]);
    assert!(text.contains("no_reading"), "the axis crate's own kind: {text}");
}

#[test]
fn an_apply_with_no_credential_is_refused_and_an_issued_one_permits_it() {
    // 1. No credential: the bridge's own class refuses before the layer is reached.
    let bare = transcript(&[call("apply_recipe", cell(), &["present"])]);
    assert!(refused(&bare[0]), "a credential appeared from nowhere");
    let refused_text = text_of(&bare[0]);
    assert!(refused_text.contains("resolve_ticket"), "{refused_text}");
    assert!(
        refused_text.contains("silence is not permission"),
        "the gate's own sentence: {refused_text}"
    );

    // 2. The authority issues one, signed by this machine's key.
    let issued = transcript(&[call("grant_recipe", cell(), &["present"])]);
    assert!(!refused(&issued[0]), "{}", text_of(&issued[0]));
    assert!(
        text_of(&issued[0]).contains("grant-issued\tuntil\t"),
        "{}",
        text_of(&issued[0])
    );
    let wire = credential(&issued[0]);
    assert_eq!(wire.split('\t').count(), 10, "{wire}");

    // 3. The same call, with the credential, is permitted and names every check.
    let permitted = transcript(&[call(
        "apply_recipe",
        json!({"ticket": TICKET, "segment": "shell", "axis": "evidence_grade", "ticket_grant": wire}),
        &["present"],
    )]);
    assert!(!refused(&permitted[0]), "{}", text_of(&permitted[0]));
    let text = text_of(&permitted[0]);
    assert!(text.contains("apply permitted on T-0954"), "{text}");
    assert!(
        text.contains("checks: credential · signature · ticket · cell · expiry · witness"),
        "{text}"
    );
}

#[test]
fn a_credential_moved_after_signing_is_refused_by_the_signature() {
    let issued = transcript(&[call("grant_recipe", cell(), &["present"])]);
    let wire = credential(&issued[0]);
    let tampered = wire.replace(TICKET, "T-9999");
    assert_ne!(tampered, wire);
    let replies = transcript(&[
        call(
            "apply_recipe",
            json!({"ticket": TICKET, "segment": "shell", "axis": "evidence_grade", "ticket_grant": tampered}),
            &["present"],
        ),
    ]);
    assert!(refused(&replies[0]));
    let text = text_of(&replies[0]);
    assert!(text.contains("signature_invalid"), "{text}");
}

#[test]
fn an_expired_grant_is_refused_at_the_servers_own_clock() {
    let replies = transcript(&[call(
        "grant_recipe",
        json!({"ticket": TICKET, "segment": "shell", "axis": "evidence_grade", "expires": 0}),
        &["present"],
    )]);
    assert!(!refused(&replies[0]), "{}", text_of(&replies[0]));
    let wire = credential(&replies[0]);
    let applied = transcript(&[call(
        "apply_recipe",
        json!({"ticket": TICKET, "segment": "shell", "axis": "evidence_grade", "ticket_grant": wire}),
        &["present"],
    )]);
    assert!(refused(&applied[0]));
    let text = text_of(&applied[0]);
    assert!(text.contains("destination_invariant_refused"), "{text}");
    assert!(text.contains("expired"), "{text}");
}

#[test]
fn a_grant_for_another_ticket_is_refused_even_though_its_signature_holds() {
    let other = "T-0000";
    let issued = transcript(&[call(
        "grant_recipe",
        json!({"ticket": other, "segment": "shell", "axis": "evidence_grade"}),
        &["present"],
    )]);
    assert!(!refused(&issued[0]), "{}", text_of(&issued[0]));
    let wire = credential(&issued[0]);
    let replies = transcript(&[call(
        "apply_recipe",
        json!({"ticket": TICKET, "segment": "shell", "axis": "evidence_grade", "ticket_grant": wire}),
        &["present"],
    )]);
    assert!(refused(&replies[0]));
    let text = text_of(&replies[0]);
    assert!(text.contains("untrusted_key"), "{text}");
    assert!(
        text.contains(other) && text.contains(TICKET),
        "both tickets are named: {text}"
    );
}

#[test]
fn a_line_that_is_not_a_credential_is_refused_as_a_malformed_record() {
    let replies = transcript(&[call(
        "apply_recipe",
        json!({"ticket": TICKET, "segment": "shell", "axis": "evidence_grade", "ticket_grant": "not a credential"}),
        &["present"],
    )]);
    assert!(refused(&replies[0]));
    let text = text_of(&replies[0]);
    assert!(text.contains("malformed_record"), "{text}");
    assert!(
        !text.contains("silence is not permission"),
        "a malformed credential is not an absent one: {text}"
    );
}

#[test]
fn a_patch_record_compiles_into_recipes_and_serves_its_gates() {
    let reply = transcript(&[call(
        "get_patch_recipes",
        json!({"patch": "PATCH-061", "ticket": TICKET}),
        &["observe"],
    )])[0]
    .clone();
    assert!(!refused(&reply), "{}", text_of(&reply));
    let text = text_of(&reply);
    assert!(text.contains("patch\tPATCH-061\t"), "{text}");
    assert_eq!(text.matches("requires\t").count(), 3, "{text}");
    assert_eq!(text.matches("segment=axis").count(), 1, "{text}");
    assert_eq!(text.matches("segment=shell").count(), 1, "{text}");
    assert!(text.contains("compiled\t2\t2"), "{text}");
    assert!(
        text.contains("fix      crates/voxel-mcp/src/lib.rs:861"),
        "the compile block names the function that compiles: {text}"
    );
    assert!(
        text.contains("fix      crates/voxel-client/src/shell/mount.rs:2378"),
        "the loader block names `load_package`, not a line in `read_the_board`: {text}"
    );
    assert!(
        text.contains("shell::mount::tests::the_loader_reads"),
        "the loader block's check does not exercise the loader: {text}"
    );
    assert!(
        !text.contains("lib.rs:849") && !text.contains("mount.rs:2308"),
        "a block still points at a function that is not its act: {text}"
    );
    assert!(
        !text.contains("crates/voxel-von/src/lib.rs:91"),
        "the unlanded classifier block is gone: {text}"
    );
}

#[test]
fn a_record_without_frontmatter_still_compiles_its_act_and_is_named_unevaluable() {
    let reply = transcript(&[call(
        "get_patch_recipes",
        json!({"patch": "PATCH-046", "ticket": TICKET}),
        &["observe"],
    )])[0]
    .clone();
    assert!(!refused(&reply), "{}", text_of(&reply));
    let text = text_of(&reply);
    assert!(text.contains("patch-unevaluable\t"), "{text}");
    assert!(text.contains("no frontmatter"), "{text}");
    assert!(text.contains("shell/evidence_grade"), "{text}");
    assert!(text.contains("compiled\t1\t1"), "{text}");
}

#[test]
fn a_patch_without_recipes_is_refused_as_prose() {
    let reply = transcript(&[call(
        "get_patch_recipes",
        json!({"patch": "PATCH-056", "ticket": TICKET}),
        &["observe"],
    )])[0]
    .clone();
    assert!(refused(&reply), "{}", text_of(&reply));
    assert!(text_of(&reply).contains("declares no `recipe` block"), "{}", text_of(&reply));
}
