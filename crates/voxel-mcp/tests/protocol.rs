//! The protocol, driven the way a client drives it: lines in, replies out.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use serde_json::Value;
use voxel_mcp::{serve, workspace_root, PROTOCOL_VERSION};

/// The ids of the records the surface is asked to report for `path`, computed from the files **by
/// the tool's own rule**: a record that names the path in its body, **or** whose declared `target`
/// names the crate the path lives under. Both halves are searched because they answer different
/// questions — a `target` names the system a change belongs to, the body names the artefact.
///
/// **Derived rather than remembered, because this went stale once.** The test hardcoded
/// `ownership\t0 record(s)` for `crates/voxel-audio/src/lib.rs`, and PATCH-036's backwalk then named
/// that file — so the test failed on a surface that was answering correctly. A count a later
/// backwalk can falsify is a count that was never about the tool.
fn records_naming(path: &str) -> Vec<String> {
    let part = path.split('/').nth(1).unwrap_or("<no second segment>");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let mut ids: Vec<String> = std::fs::read_dir(root.join("patches"))
        .expect("the record directory")
        .flatten()
        .filter_map(|entry| {
            let text = std::fs::read_to_string(entry.path()).ok()?;
            let target_names_the_crate = text
                .lines()
                .find(|line| line.starts_with("target:"))
                .is_some_and(|line| line.contains(part));
            (text.contains(path) || target_names_the_crate).then(|| {
                let name = entry.file_name().to_string_lossy().to_string();
                name.split('-').take(2).collect::<Vec<_>>().join("-")
            })
        })
        .collect();
    ids.sort();
    ids
}

/// Run a scripted transcript and return the replies, one parsed object per line.
fn transcript(request: &str) -> Vec<Value> {
    let root = workspace_root();
    let mut out = Vec::new();
    let mut sink = Cursor::new(&mut out);
    serve(&root, Cursor::new(request.as_bytes()), &mut sink).expect("the server reads its stdin");
    String::from_utf8(out)
        .expect("the protocol is UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("every reply is one JSON object"))
        .collect()
}

fn text_of(reply: &Value) -> String {
    reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn initialize_names_the_protocol_and_the_server() {
    let replies = transcript(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
    assert_eq!(replies.len(), 1, "one request, one reply");
    assert_eq!(replies[0]["result"]["protocolVersion"], PROTOCOL_VERSION);
    assert_eq!(replies[0]["result"]["serverInfo"]["name"], "voxel-mcp");
    assert!(replies[0]["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn the_tool_list_carries_every_tool_with_a_description_and_a_schema() {
    let replies = transcript(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    let tools = replies[0]["result"]["tools"].as_array().expect("tools").clone();
    assert_eq!(tools.len(), voxel_mcp::TOOLS.len());
    for tool in &tools {
        let name = tool["name"].as_str().expect("a name");
        assert!(!tool["description"].as_str().unwrap_or_default().is_empty(), "{name} has no description");
        assert_eq!(tool["inputSchema"]["type"], "object", "{name} has no schema");
    }
    let ownership = tools
        .iter()
        .find(|tool| tool["name"] == "get_ownership")
        .expect("the ownership tool is served");
    assert_eq!(ownership["inputSchema"]["required"][0], "path");
}

#[test]
fn the_patch_set_is_served_with_the_target_each_record_declares() {
    // This is the read the agent needs: not just what is applied, but where a change belongs.
    let replies = transcript(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_patch_set","arguments":{},"capabilities":["observe"]}}"#,
    );
    let text = text_of(&replies[0]);
    assert!(!replies[0]["result"]["isError"].as_bool().unwrap_or(true), "{text}");
    assert!(text.contains("patch-set-records\t"), "{text}");
    assert!(text.contains("patch-set-dangling\t0"), "{text}");
    assert!(text.contains("patch\tPATCH-018\tapplied\t"), "{text}");
}

#[test]
fn ownership_names_the_crate_and_the_records_that_own_a_path() {
    let replies = transcript(
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_ownership","arguments":{"path":"crates/voxel-package/src/bloat.rs"},"capabilities":["observe"]}}"#,
    );
    let text = text_of(&replies[0]);
    assert!(text.contains("crate\tvoxel-package"), "{text}");
    assert!(text.contains("patch\tPATCH-017\t"), "{text}");
    // And the other half of the join: a file **no record names** is still answered, with a zero
    // rather than an error. Both counts are taken from the record set on disk, so a backwalk landing
    // later moves the number instead of breaking the test.
    // A path under a crate **no record names** is every crate now, so the zero case is a crate that
    // does not exist: the precondition is asserted from the record set rather than assumed.
    let unnamed = "crates/no-record-names-this/src/lib.rs";
    assert!(
        records_naming(unnamed).is_empty(),
        "the path chosen for the zero case is now named by a record: {}",
        records_naming(unnamed).join(" ")
    );
    for (path, crate_) in [
        ("crates/voxel-audio/src/lib.rs", "voxel-audio"),
        (unnamed, "no-record-names-this"),
    ] {
        let request = format!(
            r#"{{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{{"name":"get_ownership","arguments":{{"path":"{path}"}},"capabilities":["observe"]}}}}"#
        );
        let replies = transcript(&request);
        let text = text_of(&replies[0]);
        assert!(text.contains(&format!("crate\t{crate_}")), "{text}");
        let naming = records_naming(path);
        assert!(
            text.contains(&format!("ownership\t{} record(s)", naming.len())),
            "{path}: the surface disagrees with the record set {naming:?}:\n{text}"
        );
        for id in &naming {
            assert!(text.contains(&format!("patch\t{id}\t")), "{path}: {id} missing:\n{text}");
        }
    }
}

#[test]
fn the_test_measurement_is_served_without_a_census() {
    let replies = transcript(
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"get_test_bloat","arguments":{},"capabilities":["observe"]}}"#,
    );
    let text = text_of(&replies[0]);
    assert!(text.contains("test-bloat-functions\t"), "{text}");
    assert!(text.contains("test-bloat-inline-percent\t"), "{text}");
}

#[test]
fn a_tool_nobody_serves_is_refused_by_name_and_the_tools_are_listed() {
    let replies = transcript(
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"get_vibes","arguments":{}}}"#,
    );
    assert!(replies[0]["result"]["isError"].as_bool().unwrap_or(false));
    let text = text_of(&replies[0]);
    // Refused by **coverage** — the bridge's own first check — which is a stronger statement than
    // "unknown tool": a name with no row is a refusal, never an allow.
    assert!(text.contains("no tool named \"get_vibes\""), "{text}");
    assert!(text.contains("never an allow"), "{text}");
    assert!(text.contains("checks ran: none"), "coverage cannot have passed if it did not run: {text}");
}

#[test]
fn an_unknown_method_is_an_error_and_a_notification_is_not_answered() {
    let replies = transcript(
        "{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"resources/list\"}\n\
         {\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n\
         {\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\"}\n",
    );
    assert_eq!(replies.len(), 1, "two notifications produce no reply");
    assert_eq!(replies[0]["error"]["code"], -32601);
}

#[test]
fn a_line_that_is_not_json_is_a_parse_error_and_the_stream_continues() {
    let replies = transcript(
        "not json at all\n{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"tools/list\"}\n",
    );
    assert_eq!(replies.len(), 2, "the bad line is answered and the good one still is");
    assert_eq!(replies[0]["error"]["code"], -32700);
    assert!(replies[1]["result"]["tools"].is_array(), "the stream survived the bad line");
}

#[test]
fn a_missing_path_argument_is_refused_with_the_shape_it_wants() {
    let replies = transcript(
        r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"get_ownership","arguments":{},"capabilities":["observe"]}}"#,
    );
    let text = text_of(&replies[0]);
    assert!(replies[0]["result"]["isError"].as_bool().unwrap_or(false));
    assert!(text.contains("needs a `path` string"), "{text}");
}

#[test]
fn the_root_is_found_above_the_crate_and_holds_the_gate_it_serves() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    assert_eq!(root, workspace_root());
    assert!(root.join("patches").is_dir(), "the patch records are what this server serves");
    assert!(root.join("chains/workspace.chain").is_file(), "and the record is beside them");
}
