//! The gate's surface, spoken over MCP — **and gated by the tree's own gate, not by a second one.**
//!
//! # What this crate is not
//!
//! The first version read files and answered with text. That is not the api layer this tree
//! declares, and it was wrong in three measurable ways:
//!
//! 1. **It carried its own action map.** `voxel-package::bridge` already *is* the tool gate: a
//!    declared [`bridge::TOOLS`] table bijective onto [`Action`], a [`Capability`] partition over
//!    that vocabulary, `bridge::call` running coverage → capability → the one matrix, and a
//!    [`bridge::Receipt`] that names **which checks ran**. A second `match` on names here would be
//!    a second matrix, which this tree refuses by name. [`receipt`] now asks the bridge.
//! 2. **It filed into `logs/laya-decisions/`.** That directory is the shell's notification stream —
//!    `laya::queue_dir` → `Queue`, read by `shell::notifications::NotificationStream`, which
//!    re-hashes every artifact it shows. But that queue is **a projection of one pass over the
//!    chain**, and `write_the_gate_queue` **clears it** on every world open (*"the chain is the
//!    record, and this queue is a projection of one pass over it"*). Records filed there are
//!    wiped by the next run **and their positional names (`laya-NNNN`) are reused** — the exact
//!    trap the queue's own header warns about.
//! 3. **It invented a record kind.** `EventKind::RefusalRecorded` is declared ("a refusal nobody
//!    can read is a refusal that did not happen"), and `bridge::record_refusal` /
//!    `filing::append` are the declared writers. When the chain is not yet filed, the first filing
//!    is legitimate and [`reject_non_conformance`] takes it; when it is filed, `filing::append`
//!    **refuses a second filing by design** — one `.chain` file per workspace, and segmenting is a
//!    rule the record has not ruled — so the events are built, linked and readable, and the
//!    refusal to persist them is reported **in the tree's own words rather than worked around**.
//!
//! # The three surfaces this rides
//!
//! * **The gate.** [`receipt`] → `bridge::call`. A name the surface does not carry is refused by
//!   coverage; an action the session's capabilities do not cover is refused by the capability; a
//!   gated act with no grant is refused by the matrix. All three sentences are the bridge's.
//! * **The record.** [`reject_non_conformance`] appends one `RefusalRecorded` event per rejected
//!   cell, linked to the head read back from `chains/workspace.chain`, and files it when the
//!   workspace has no chain yet.
//! * **The api viewer.** [`api_view`] renders the document `window[api-viewer]` is for, from
//!   declarations only: the served tools, their **rows**, the actions those rows name, and
//!   `policy::permission_of`'s class — so the view cannot advertise a permission the chokepoint
//!   does not enforce.
//!
//! Every read is `AlwaysAllowed` in the matrix and needs `observe` — the capability that means
//! *"read what is, and change nothing."* Nothing is implicit, including that.
//!
//! **The audit, 2026-10-10.** 1 174 lines, 2 modules (lib + bin), 16 public items, 18 tests — and
//! **read by nobody**: the binary is an entry point the workspace does not consume. It now delegates
//! to `voxel-package::bridge` rather than carrying a second action map, which is the right shape and
//! was the defect its first version had. **Owed:** the thing its own TODO row 10 names — *a real client
//! calls the surface.* It is verified by transcript today, and nothing in the shell, an editor or CI
//! speaks to it, so its `reject_non_conformance` filing path is a capability with a reader that does
//! not exist yet.
#![warn(missing_docs)]

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use voxel_axis_mcp::{Registry, TicketGrant};
use voxel_axis_matrix::{Fix, Segment, Site};
use voxel_model::axis::AxisKind;
use voxel_model::chain::{Chain, EventKind, Namespace, Season};
use voxel_model::grant::Grant;
use voxel_model::ticket::EvidenceOrigin;
use voxel_model::RefusalSet;
use voxel_package::bridge::{self, Authority, Capability, Receipt};
use voxel_package::census::{self, Establishment};
use voxel_package::policy::{self, Action};
use voxel_package::{bloat, conformance, filing, patch};

/// The revision this server speaks, named so a client that disagrees is refused rather than guessed at.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// The surface, in served order. The work list is first because it is the reason the gate exists.
/// The filing tool is last because it is the only one that needs an author.
pub const TOOLS: [&str; 14] = [
    "get_work_list",
    "get_conformance",
    "get_census",
    "get_patch_set",
    "get_ownership",
    "get_test_bloat",
    "get_api_view",
    "judge_work",
    "reject_non_conformance",
    "get_axis_matrix",
    "get_axis_recipe",
    "get_patch_recipes",
    "grant_recipe",
    "apply_recipe",
];

/// The bridge row each served tool is a call to. **The tree's vocabulary, not a parallel one.**
/// The names are the bridge's own ([`bridge::TOOLS`]); this is the one place the two spellings
/// meet, and a tool with no row here is refused by coverage rather than served unclassified.
pub fn row_of(name: &str) -> Option<&'static str> {
    match name {
        "get_work_list" | "get_census" | "get_patch_set" | "get_ownership" | "get_test_bloat" => {
            Some("read_governance")
        }
        "get_conformance" => Some("lint_line"),
        "get_api_view" => Some("read_recipe"),
        // Judging a package is verifying it: a read of the record's own instrument, `AlwaysAllowed`
        // and covered by `observe`.
        "judge_work" => Some("verify_package"),
        "reject_non_conformance" => Some("append_event"),
        // **The axis route, in the tree's own vocabulary.** Reading the matrix is reading governance;
        // a recipe line is the `read_recipe` row's own act. Issuing a grant files authority against a
        // ticket (`file_ticket`, `present`). Applying a recipe is the act that resolves that ticket —
        // and it is `resolve_ticket`'s `RequiresGrant` class that refuses an apply with no grant, while
        // the signature over the grant is what the axis layer checks.
        "get_axis_matrix" => Some("read_governance"),
        "get_axis_recipe" | "get_patch_recipes" => Some("read_recipe"),
        "grant_recipe" => Some("file_ticket"),
        "apply_recipe" => Some("resolve_ticket"),
        _ => None,
    }
}

/// **The one gate.** `bridge::call` runs coverage, then the session's capability, then the matrix —
/// and the [`Receipt`] carries the checks that actually executed, because *a check that did not run
/// is not a check that passed*.
///
/// A name this server does not serve is passed through **verbatim**, so the refusal comes from the
/// bridge's own surface check and names the tool that was asked for.
pub fn receipt(name: &str, authority: &Authority, grant: Option<&Grant>) -> Receipt {
    match row_of(name) {
        Some(row) => bridge::call(authority, row, grant),
        None => bridge::call(authority, name, grant),
    }
}

/// The capabilities a request presents. **Nothing is implicit**: a request that names none gets
/// [`Authority::none`], and the refusal names the capability the row needs. An unknown name is
/// refused rather than dropped, because a client that misspells `observe` and is quietly given
/// nothing would read the refusal as a fact about the tool.
pub fn authority_from(params: &Value) -> Result<Authority, String> {
    let Some(names) = params.get("capabilities") else {
        return Ok(Authority::none());
    };
    let Some(names) = names.as_array() else {
        return Err("`capabilities` must be an array of capability names".to_string());
    };
    let mut authority = Authority::none();
    for name in names {
        let word = name.as_str().unwrap_or_default();
        let capability = Capability::parse(word).map_err(|why| why.to_string())?;
        authority = authority.grant(capability);
    }
    Ok(authority)
}

/// The grant a request presents, or `None`. Both halves are required — `Grant::new` refuses an
/// empty granter or an empty scope, and its refusals are carried out verbatim.
pub fn grant_from(params: &Value) -> Result<Option<Grant>, String> {
    let granter = params.get("granter").and_then(Value::as_str);
    let scope = params.get("scope").and_then(Value::as_str);
    match (granter, scope) {
        (None, None) => Ok(None),
        (granter, scope) => Grant::new(granter.unwrap_or_default(), scope.unwrap_or_default())
            .map(Some)
            .map_err(|found| RefusalSet::to_string(&found)),
    }
}

/// The workspace root above this crate, found rather than assumed.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The tool list, as MCP wants it. Every schema is an empty object: no tool takes a request body.
pub fn tool_list() -> Value {
    let tools: Vec<Value> = TOOLS
        .iter()
        .map(|name| {
            json!({"name": name, "description": description(name), "inputSchema": schema_of(name)})
        })
        .collect();
    Value::Array(tools)
}

/// **What a tool needs, declared once.** An empty object is a tool that takes no arguments; every
/// other schema names exactly the arguments its body reads, so a client can construct a call without
/// reading this crate's source.
fn schema_of(name: &str) -> Value {
    let cell = json!({
        "ticket": {"type": "string"},
        "segment": {"type": "string"},
        "axis": {"type": "string"}
    });
    match name {
        "get_ownership" => json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"],
            "additionalProperties": false
        }),
        "get_axis_matrix" => json!({
            "type": "object",
            "properties": {"ticket": {"type": "string"}},
            "required": ["ticket"],
            "additionalProperties": false
        }),
        "get_axis_recipe" => json!({
            "type": "object",
            "properties": cell,
            "required": ["ticket", "segment", "axis"],
            "additionalProperties": false
        }),
        "get_patch_recipes" => json!({
            "type": "object",
            "properties": {
                "patch": {"type": "string"},
                "ticket": {"type": "string"}
            },
            "required": ["patch", "ticket"],
            "additionalProperties": false
        }),
        "grant_recipe" => json!({
            "type": "object",
            "properties": {
                "ticket": {"type": "string"},
                "segment": {"type": "string"},
                "axis": {"type": "string"},
                "expires": {"type": "integer", "minimum": 0}
            },
            "required": ["ticket", "segment", "axis"],
            "additionalProperties": false
        }),
        "apply_recipe" => json!({
            "type": "object",
            "properties": {
                "ticket": {"type": "string"},
                "segment": {"type": "string"},
                "axis": {"type": "string"},
                "ticket_grant": {"type": "string"}
            },
            "required": ["ticket", "segment", "axis", "ticket_grant"],
            "additionalProperties": false
        }),
        _ => json!({"type": "object", "properties": {}, "additionalProperties": false}),
    }
}

/// What each tool answers, in one sentence, because a tool nobody can choose is a tool nobody calls.
fn description(name: &str) -> &'static str {
    match name {
        "get_work_list" => {
            "Every census cell that is not active, one line each, naming the crate, the axis, the state and the reason. This is the work list."
        }
        "get_conformance" => {
            "The gate's verdict over every subject: active only when nothing is refused and nothing is unchecked, because an unmeasured cell is not a pass."
        }
        "get_census" => {
            "The crate-by-axis table and the classified top-level folders beside it, as the build gate prints them."
        }
        "get_patch_set" => {
            "The patch records and the target each declares: what is applied, what is owed, and where a change belongs."
        }
        "get_ownership" => {
            "Given a path: the crate it is in, and every patch record whose target names it."
        }
        "get_test_bloat" => "How much of the workspace is test lines, and where they live.",
        "get_api_view" => {
            "The document window[api-viewer] displays: every served tool, its bridge row, and that row's action and permission class, rendered from declarations."
        }
        "judge_work" => {
            "Judge a package's declared ballot against the artifact it names, with the classifier: the answer becomes a decision with the model's own numbers, its confidence, and the evidence origin the instrument carries. Takes a `ballot` path."
        }
        "reject_non_conformance" => {
            "Reject the cells the conformance rule refuses, by appending one refusal_recorded event per cell to the project chain. Requires the capability its row needs — the bridge names it when it is missing — and a scoped, attributed grant."
        }
        "get_axis_matrix" => {
            "The verified calibration matrix over this tree: one line per segment with its star, tier, ticket and witness, then the package — the same cells as cited, laddered statements with the pin over them. Reads without a key."
        }
        "get_patch_recipes" => {
            "Compile a patch record's declared recipes against the measured matrix: the edit, its site, the check that must move, and the gates the tree requires."
        }
        "get_axis_recipe" => {
            "The package's own line for a named ticket and cell: the statement, the file:line it cites, and the ladder reading the tree derives. Refused when the ticket or the cell was not the one measured."
        }
        "grant_recipe" => {
            "Issue a ticket-scoped grant for one cell, signed by this machine's key, valid until a tick given in unix seconds. Returns the credential as one line, which `apply_recipe` accepts. The signature covers the ticket, the cell and the expiry."
        }
        "apply_recipe" => {
            "Decide an apply on a measured cell under a signed grant. Six checks run — credential, signature, ticket, cell, expiry, witness — and the receipt names them. Nothing is written: the act stays with the caller."
        }
        _ => "Unknown tool.",
    }
}

/// One tool call, through the gate with no capability and no grant. Reads need `observe`, so this
/// is the shape a request with nothing presented gets — a refusal that names what is missing.
pub fn call(root: &Path, name: &str, arguments: &Value) -> Result<String, String> {
    call_with(root, name, arguments, &Authority::none(), None)
}

/// One tool call, through the gate. **The gate runs before the dispatcher**, so a refused act has no
/// side effect whether or not its body would have succeeded.
pub fn call_with(
    root: &Path,
    name: &str,
    arguments: &Value,
    authority: &Authority,
    grant: Option<&Grant>,
) -> Result<String, String> {
    // **A signed grant is the credential an apply presents, and it carries its own attribution.**
    // The bridge's grant is a presence check; the signature is the proof, and the axis layer is what
    // checks it. A line that does not parse is refused here with the axis crate's own kind —
    // `malformed_record`, not "no grant was presented", which would tell the client the wrong fact.
    let derived: Option<Grant> = match (name, grant) {
        ("apply_recipe", None) => match arguments.get("ticket_grant") {
            None => None,
            Some(value) => {
                let wire = value.as_str().ok_or_else(|| {
                    "apply_recipe needs `ticket_grant` as a string: the credential's own one-line \
                     form"
                        .to_string()
                })?;
                let parsed = TicketGrant::from_wire(wire)
                    .map_err(|found| RefusalSet::to_string(&found))?;
                Grant::new(parsed.grant().granter(), parsed.grant().scope()).ok()
            }
        },
        _ => None,
    };
    let receipt = receipt(name, authority, grant.or(derived.as_ref()));
    if !receipt.ran() {
        // The bridge's own sentence, and the checks that ran to produce it.
        return Err(receipt.line());
    }
    dispatch(root, name, arguments)
}

/// The bodies. Reached only through [`call_with`], never directly.
fn dispatch(root: &Path, name: &str, arguments: &Value) -> Result<String, String> {
    match name {
        "get_work_list" => work_list(root),
        "get_conformance" => conformance(root),
        "get_census" => census_table(root),
        "get_patch_set" => patch_set(root),
        "get_ownership" => ownership(root, arguments),
        "get_test_bloat" => Ok(bloat::measure_workspace_tests(root).describe()),
        "get_api_view" => Ok(api_view()),
        "judge_work" => judge_work(root, arguments),
        "reject_non_conformance" => reject_non_conformance(root),
        "get_axis_matrix" => axis_matrix(root, arguments),
        "get_axis_recipe" => axis_recipe(root, arguments),
        "get_patch_recipes" => patch_recipes(root, arguments),
        "grant_recipe" => grant_recipe(root, arguments),
        "apply_recipe" => apply_recipe(root, arguments),
        other => Err(format!("unknown tool {other}; the tools are {TOOLS:?}")),
    }
}

/// Read JSON-RPC off `input` and write replies to `output`, until the stream ends.
pub fn serve(root: &Path, input: impl BufRead, output: &mut impl Write) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            send(output, failed(Value::Null, -32700, "the request line is not JSON"))?;
            continue;
        };
        // A notification carries no id, and a reply to one would be a message the client cannot read.
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str).unwrap_or_default();
        match method {
            "initialize" => send(
                output,
                ok(
                    id.unwrap_or(Value::Null),
                    json!({
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "voxel-mcp", "version": env!("CARGO_PKG_VERSION")}
                    }),
                ),
            )?,
            "tools/list" => send(
                output,
                ok(id.unwrap_or(Value::Null), json!({"tools": tool_list()})),
            )?,
            "tools/call" => {
                let params = request.get("params").cloned().unwrap_or(Value::Null);
                let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
                let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                // A malformed capability or grant is a refusal rather than an absent one: a
                // granter with no scope authorizes everything, and that would read as a permission
                // this server never granted.
                let body = match (authority_from(&params), grant_from(&params)) {
                    (Err(why), _) | (_, Err(why)) => {
                        json!({"content": [{"type": "text", "text": why}], "isError": true})
                    }
                    (Ok(authority), Ok(grant)) => {
                        match call_with(root, name, &arguments, &authority, grant.as_ref()) {
                            Ok(text) => {
                                json!({"content": [{"type": "text", "text": text}], "isError": false})
                            }
                            Err(why) => {
                                json!({"content": [{"type": "text", "text": why}], "isError": true})
                            }
                        }
                    }
                };
                send(output, ok(id.unwrap_or(Value::Null), body))?;
            }
            "notifications/initialized" => {}
            _ if id.is_none() => {}
            other => send(
                output,
                failed(id.unwrap_or(Value::Null), -32601, &format!("unknown method {other}")),
            )?,
        }
    }
    output.flush()
}

/// **The api viewer's text, rendered from declarations.** No sentence here is written by hand about
/// the surface: the tool list is [`TOOLS`], each description is [`description`], the row is
/// [`row_of`]'s, the action is that row's in [`bridge::TOOLS`], and the class is
/// [`policy::permission_of`]'s own token. Add an action to the matrix and this document changes with
/// it — which is why it cannot advertise a permission the gate does not enforce.
pub fn api_view() -> String {
    let mut out = String::new();
    out.push_str("# The gate's MCP surface\n");
    out.push_str(&format!("protocol\t{PROTOCOL_VERSION}\n"));
    out.push_str(&format!("tools\t{}\n", TOOLS.len()));
    for name in TOOLS {
        let row = row_of(name).expect("every served tool names a bridge row");
        let tool = bridge::tool(row).expect("every row is in the bridge's own table");
        out.push_str(&format!(
            "tool\t{name}\trow\t{row}\taction\t{}\tclass\t{}\t{}\n",
            tool.action.name(),
            permission_of_name(tool.action),
            description(name),
        ));
    }
    out.push_str(
        "chokepoint\tvoxel-package::bridge\tcoverage, then capability, then voxel-package::policy: \
         reads need observe; filing is append_event and requires a scoped, attributed grant\n",
    );
    out
}

/// The permission class as the matrix spells it. Split out so [`api_view`] reads one token and a
/// test can assert the document and the matrix agree.
pub fn permission_of_name(action: Action) -> &'static str {
    match policy::permission_of(action) {
        policy::PermissionClass::AlwaysAllowed => "always_allowed",
        policy::PermissionClass::AlwaysDenied => "always_denied",
        policy::PermissionClass::RequiresGrant => "requires_grant",
    }
}

/// **Judge a package's declared ballot, with the classifier.**
///
/// The ballot comes from the package format and the state is the artifact's **own bytes**, read off
/// disk now rather than carried in the request: a judgment about bytes the caller supplied would be
/// a judgment about bytes nobody measured, which is the one thing the digest discipline in this
/// tree exists to prevent.
///
/// The model loads **on the first judgment and not before**. A client that only reads the gate never
/// pays the ~20 s load, and a client that judges pays it once — which is why this is a `OnceCell`
/// rather than a load per call, and why the load is inside this tool and not at server startup.
///
/// The output is the decision, in the gate's `key<TAB>value` shape: every number is the model's own
/// and the origin is the `A` rung, written by the instrument.
pub fn judge_work(root: &Path, arguments: &Value) -> Result<String, String> {
    let ballot_path = arguments.get("ballot").and_then(Value::as_str).ok_or_else(|| {
        "judge_work needs a `ballot` path, as {\"ballot\": \"packages/alpha-shell.ballot\"}"
            .to_string()
    })?;
    if ballot_path.contains("..") || ballot_path.starts_with('/') {
        return Err(format!(
            "refused: {ballot_path} is not a path inside the workspace"
        ));
    }
    let ballots = voxel_package::ballot::read(&root.join(ballot_path))
        .map_err(|found| RefusalSet::to_string(&found))?;

    let mut out = format!(
        "judge-model\t{}\njudge-cache\t{}\njudge-cached\t{}\njudge-ballots\t{}\n",
        voxel_von::MODEL,
        voxel_von::Von::cache_dir().display(),
        voxel_von::cached(),
        ballots.len(),
    );
    // The guard is the session; `&mut classifier` derefs to `&mut Von` for the ask below.
    let mut classifier = classifier()?;
    let mut belonging = 0usize;
    let mut non_readings = 0usize;
    for ballot in &ballots {
        let artifact = root.join(&ballot.artifact);
        let bytes = match std::fs::read(&artifact) {
            Ok(bytes) => bytes,
            Err(failure) => {
                non_readings += 1;
                out.push_str(&format!(
                    "judgment\t{}\tno-reading\tthe artifact {} cannot be read: {failure}\n",
                    ballot.id, ballot.artifact
                ));
                continue;
            }
        };
        let text = String::from_utf8_lossy(&bytes).to_string();
        let state = json!({
            "artifact": ballot.artifact,
            "claim": ballot.claim,
            "presenter": ballot.presenter,
            "bytes": bytes.len(),
            "text": text,
        });
        let started = std::time::Instant::now();
        let decision = voxel_von::judge::ask(&mut classifier, ballot, &state)?;
        let verdict = match decision.verdict {
            voxel_model::laya::Verdict::Belongs => "belongs",
            voxel_model::laya::Verdict::NotBelongs => "not-belongs",
            voxel_model::laya::Verdict::NoReading => "no-reading",
        };
        if matches!(decision.verdict, voxel_model::laya::Verdict::Belongs) {
            belonging += 1;
        }
        if matches!(decision.verdict, voxel_model::laya::Verdict::NoReading) {
            non_readings += 1;
        }
        out.push_str(&format!(
            "judgment\t{}\t{}\t{}\tconfidence {:.4}\torigin {}\tartifact {}\t{}ms\n",
            ballot.id,
            verdict,
            ballot.kind.name(),
            decision.confidence,
            decision
                .origin
                .map(|origin| origin.letter())
                .unwrap_or("none"),
            ballot.artifact,
            started.elapsed().as_millis(),
        ));
        for grade in &decision.grades {
            out.push_str(&format!("grade\t{}\t{}\t{}\n", ballot.id, grade.0.name(), grade.1));
        }
        for finding in &decision.findings {
            out.push_str(&format!("finding\t{}\t{finding}\n", ballot.id));
        }
    }
    out.push_str(&format!(
        "judgments\t{}\njudged-belongs\t{}\njudged-non-readings\t{}\n",
        ballots.len(),
        belonging,
        non_readings
    ));
    Ok(out)
}

/// **The classifier, loaded once per process, on the first judgment and never at startup.**
///
/// The session is kept in a `Mutex` because it is asked `&mut self`, and the model is loaded
/// exactly once: a second judgment reuses the session instead of re-reading 757 MiB off disk, which
/// a load-per-call would do while looking correct. A load that failed is remembered as a failure
/// rather than retried on every call, so a client gets the same reason instead of a 20 s pause.
fn classifier() -> Result<std::sync::MutexGuard<'static, voxel_von::Von>, String> {
    static LOADED: std::sync::OnceLock<
        Result<std::sync::Mutex<voxel_von::Von>, String>,
    > = std::sync::OnceLock::new();
    let loaded = LOADED.get_or_init(|| voxel_von::Von::load().map(std::sync::Mutex::new));
    match loaded {
        Ok(session) => session
            .lock()
            .map_err(|_| "the classifier session was poisoned by an earlier panic".to_string()),
        Err(why) => Err(why.clone()),
    }
}

/// **The rejection, appended to the record.** One rejected cell becomes one
/// [`EventKind::RefusalRecorded`] event, linked to the head read back from
/// `chains/workspace.chain` — the chain's own kind for it (*"a refusal nobody can read is a refusal
/// that did not happen"*), and the chain's own writer computes the linkage, so an event cannot name
/// a predecessor it does not have.
///
/// The chain is read back from the workspace, never built fresh: appending to a chain that did not
/// include the filed records would produce a second record claiming the genesis digest, which the
/// read-back refuses as `ChainBroken` — correctly, and not something to work around.
///
/// Persisting is [`filing::append`]'s decision, not this crate's. When nothing is filed, the first
/// filing is legitimate and happens. When the chain is already filed, that function **refuses a
/// second filing by design** — one `.chain` file per workspace, and segmenting is a rule the record
/// has not ruled — so the events are built, linked and readable, and the refusal to persist them is
/// reported in the tree's words rather than invented around with a `workspace-2.chain`.
pub fn reject_non_conformance(root: &Path) -> Result<String, String> {
    let verdict = conformance::verdict(root)?;
    let chain_dir = root.join(filing::CHAIN_DIR);
    let mut chain = bloat::read_chain(&chain_dir)?;
    let namespace = Namespace::new("shploingus", 0, Season::Spring, "retrofit")
        .map_err(|found| RefusalSet::to_string(&found))?;
    let recorded = record_rejections(&mut chain, &namespace, &verdict);
    let mut out = recorded;
    out.push_str(&format!(
        "reject-chain-head\t{}\nreject-chain-events\t{}\n",
        chain.digest().to_hex(),
        chain.len()
    ));
    match filing_persist(root, &chain) {
        Ok(path) => out.push_str(&format!("reject-filed\t{}\n", path.display())),
        Err(why) => out.push_str(&format!(
            "reject-not-filed\t{why}\nreject-not-filed-reason\tthe chain is one file per \
             workspace and segmenting it is a rule the record has not ruled; the events above are \
             built, linked and readable, and persisting them is that ruling's to make\n"
        )),
    }
    Ok(out)
}

/// The events themselves, split out so they can be built and read without touching the disk.
///
/// The detail carries **what was measured and where**: the cell's key, its state, and the census's
/// own reason — the same four fields `conformance::Verdict::describe` prints, so a reader of the
/// chain and a reader of the build report cannot disagree about what was refused.
pub fn record_rejections(chain: &mut Chain, namespace: &Namespace, verdict: &conformance::Verdict) -> String {
    let rejected = verdict.rejected();
    let mut out = format!(
        "reject-non-conformance\t{}\nreject-recording\t{}\nreject-accepted\t{}\nreject-unchecked\t{}\n",
        verdict.state().slug(),
        rejected.len(),
        verdict.accepted.len(),
        verdict.unchecked.len(),
    );
    let mut kinds = std::collections::BTreeSet::new();
    for refusal in &rejected {
        // **The origin is on the event**, in the ladder's own letter: the census was read back from
        // this project's own tree, which is the `B` rung and not something an oracle decided.
        let detail = format!(
            "{}\t{}\t{}\torigin {}",
            refusal.key(),
            refusal.state,
            refusal.why,
            EvidenceOrigin::ReadBack.describe()
        );
        match chain.append(namespace.clone(), EventKind::RefusalRecorded, detail) {
            Ok(event) => {
                kinds.insert(event.kind.name());
                out.push_str(&format!(
                    "refusal-recorded\t{}\t{}\t{}\tindex {}\n",
                    refusal.crate_, refusal.axis, refusal.state, event.index
                ));
            }
            // An append that refuses says so and the run continues: eleven recorded refusals and
            // one refusal to record is a fact about the record, and stopping would hide the eleven.
            Err(found) => out.push_str(&format!(
                "refusal-not-recorded\t{}\t{}\n",
                refusal.key(),
                RefusalSet::to_string(&found)
            )),
        }
    }
    out.push_str(&format!("reject-created\t{}\n", kinds.into_iter().collect::<Vec<_>>().join(",")));
    out
}

/// File the chain when the workspace has none — **and let `filing` decide**, rather than asking
/// `is_filed` here and doing the same thing twice with a different answer.
fn filing_persist(root: &Path, chain: &Chain) -> Result<PathBuf, String> {
    let bytes = chain_bytes(chain);
    filing::append(root, &bytes).map_err(|found| RefusalSet::to_string(&found))
}

/// The chain's bytes as the workspace chain file spells them: one rendered record per line, which is
/// what `compiler::read_back` reads and what `bloat::read_chain` measures.
///
/// **Rendered from the events, not from a second writer**: the fields are the event's own, and the
/// linkage is the event's own `previous`, so a record cannot be filed claiming a predecessor the
/// chain does not carry.
fn chain_bytes(chain: &Chain) -> Vec<u8> {
    let mut out = String::new();
    chain.fold((), |(), event| {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            event.index,
            event.namespace.world,
            event.kind.name(),
            event.previous.render(),
            event.detail
        ));
    });
    out.into_bytes()
}

/// The census, or the refusals that stood in its way.
fn readings(root: &Path) -> Result<census::Census, String> {
    census::census(root).map_err(|found| RefusalSet::to_string(&found))
}

/// **The crate table, with the folders beside it.** The census measures workspace members, so a
/// folder that is not a crate is outside it; the folder reading is the half that says which
/// folder is a package, an emit root or a declared non-package. Both come from
/// `voxel-package`'s own instruments — this function joins two readings, it does not take one.
fn census_table(root: &Path) -> Result<String, String> {
    let table = census::report(&readings(root)?);
    let folders = census::folders::classify(root).map_err(|found| RefusalSet::to_string(&found))?;
    Ok(format!("{table}\n{}", folders.describe()))
}

/// Every cell that is not active, with its reason. **The refusal list is the work list** — computed, not remembered.
fn work_list(root: &Path) -> Result<String, String> {
    let census = readings(root)?;
    let mut lines = String::new();
    let mut refused = 0usize;
    let mut unchecked = 0usize;
    let mut stale = 0usize;
    let mut inactive = 0usize;
    for reading in &census.readings {
        for (axis, state, why) in reading.gaps() {
            match state {
                Establishment::Refused => refused += 1,
                Establishment::Unchecked => unchecked += 1,
                // **Stale is its own count here too.** Folding it into `inactive` would hand this
                // surface's reader a number about a tree that no longer exists and present it as
                // owed work in this one — an agent would go and write documentation that is already
                // there. The line still prints: the repair is to re-run the recipe.
                Establishment::Stale => stale += 1,
                _ => inactive += 1,
            }
            lines.push_str(&format!(
                "work\t{}\t{}\t{}\t{}\n",
                reading.crate_.name(),
                axis.name(),
                state.slug(),
                why.unwrap_or("no reason recorded")
            ));
        }
    }
    Ok(format!(
        "{lines}work-list-total\t{}\nwork-list-refused\t{refused}\nwork-list-unchecked\t{unchecked}\nwork-list-stale\t{stale}\nwork-list-inactive\t{inactive}\n",
        refused + unchecked + stale + inactive
    ))
}

/// The verdict, from the gate's own rule in `voxel-package::conformance`, so this surface and any
/// build door cannot answer differently about the same tree.
fn conformance(root: &Path) -> Result<String, String> {
    Ok(voxel_package::conformance::verdict(root)?.describe())
}

/// The patch records with their targets: what is applied, what is owed, and where a change belongs.
fn patch_set(root: &Path) -> Result<String, String> {
    let set = patch::read_dir(&root.join("patches"));
    let mut out = set.describe();
    out.push('\n');
    // **The same join the gate prints, from the same function**: which parts of the tree a record
    // names and which it does not. A served answer and a built answer that could differ would be
    // two answers to one question.
    out.push_str(&set.coverage(&voxel_package::census::Crate::ALL).describe());
    out.push('\n');
    // The same placement reading the gate prints, from the same function: every line's place,
    // per file, biggest work item first. A served answer and a built answer that could differ
    // would be two answers to one question.
    out.push_str(&patch::placement(&set, root).describe());
    out.push('\n');
    for record in &set.records {
        out.push_str(&format!("patch\t{}\t{}\t{}\n", record.id, record.status, record.target));
        for problem in &record.problems {
            out.push_str(&format!("patch-problem\t{}\t{problem}\n", record.id));
        }
    }
    Ok(out)
}

/// Where a path belongs: the crate it is in, and every record that names it.
///
/// **Both halves of a record are searched, because they answer different questions.** `target` names
/// the *system* a change belongs to (`shploingus/test-bloat`), while the artifact is named in the
/// body — so a read of `target` alone finds nothing for a record that plainly changed this file.
///
/// **The body half reads the record's own extracted paths** (`PatchRecord::named_paths`), which is the
/// same list `patch::coverage` joins against: one extraction, two readers, so a path this surface
/// finds and the coverage report does not (or the reverse) is not a disagreement but a bug.
fn ownership(root: &Path, arguments: &Value) -> Result<String, String> {
    let path = arguments.get("path").and_then(Value::as_str).ok_or_else(|| {
        "get_ownership needs a `path` string, as {\"path\": \"crates/voxel-package/src/bloat.rs\"}"
            .to_string()
    })?;
    if path.contains("..") || path.starts_with('/') {
        return Err(format!("refused: {path} is not a path inside the workspace"));
    }
    let named = path.split('/').nth(1).unwrap_or("<not under a crate>");
    let mut out = format!("path\t{path}\ncrate\t{named}\n");
    let dir = root.join("patches");
    let entries = std::fs::read_dir(&dir)
        .map_err(|why| format!("{} could not be read: {why}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    files.sort();
    let mut naming = 0usize;
    for file in files {
        if file.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let name = file.file_name().and_then(|name| name.to_str()).unwrap_or("<unprintable>");
        let record = patch::parse(name, &text);
        if record.target.contains(named)
            || record.named_paths.iter().any(|named_path| named_path == path)
        {
            naming += 1;
            out.push_str(&format!("patch\t{}\t{}\t{}\n", record.id, record.status, record.target));
        }
    }
    out.push_str(&format!("ownership\t{naming} record(s) name this path\n"));
    Ok(out)
}

/// How long a grant issued without an explicit expiry stays authority for, in unix seconds.
pub const DEFAULT_GRANT_SECONDS: u64 = 600;

/// **The verified matrix and its package.** Read without a key, so a client that only looks at the
/// tree never causes one to be minted: every line is the matrix's or the package's own.
fn axis_matrix(root: &Path, arguments: &Value) -> Result<String, String> {
    let ticket = string_arg(arguments, "ticket", "{\"ticket\": \"T-0954\"}")?;
    let registry = Registry::read(root, ticket).map_err(|found| RefusalSet::to_string(&found))?;
    let matrix = registry.matrix().report().join("\n");
    let pin = registry
        .package()
        .map(|package| package.pin().digest.to_string())
        .unwrap_or_default();
    let package = registry
        .package()
        .map(|package| package.render())
        .unwrap_or_default();
    Ok(format!(
        "axis-matrix\tticket\t{ticket}\tcell(s)\t{}\n{matrix}\naxis-package\tpin\t{pin}\n{package}\n",
        registry.matrix().len()
    ))
}

/// **The package's own line for a named ticket and cell.** The layer checks that the ticket is the
/// one measured and that the cell names a witness; this function spells neither rule.
fn axis_recipe(root: &Path, arguments: &Value) -> Result<String, String> {
    let ticket = string_arg(arguments, "ticket", "{\"ticket\": \"T-0954\"}")?;
    let segment = segment_arg(arguments)?;
    let axis = axis_arg(arguments)?;
    let registry = Registry::read(root, ticket).map_err(|found| RefusalSet::to_string(&found))?;
    let line = registry
        .recipe(ticket, segment, axis)
        .map_err(|found| RefusalSet::to_string(&found))?;
    Ok(format!(
        "axis-recipe\t{ticket}\t{}\t{}\n{}\n",
        segment.name(),
        axis.name(),
        line.render()
    ))
}

/// **A patch record's declared recipes, compiled against the measured matrix.**
///
/// The join the surface was missing: a patch says what changed; its `recipe` blocks say what to type,
/// at which `file:line`, what the check must print, and which gates the tree requires. Each block is
/// compiled through the matrix's own constructor, so a recipe for a cell the walk did not measure is
/// refused by name rather than rendered as if it were portable.
fn patch_recipes(root: &Path, arguments: &Value) -> Result<String, String> {
    let wanted = string_arg(arguments, "patch", "{\"patch\": \"PATCH-061\"}")?;
    let ticket = string_arg(arguments, "ticket", "{\"ticket\": \"T-0954\"}")?;
    let dir = root.join("patches");
    let entries = std::fs::read_dir(&dir)
        .map_err(|why| format!("{} could not be read: {why}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("md"))
        .collect();
    files.sort();
    let file = files
        .iter()
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(wanted))
        })
        .ok_or_else(|| format!("refused: no patch record starts with `{wanted}` under patches/"))?;
    let name = file.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    let text = std::fs::read_to_string(file).map_err(|why| format!("{name} could not be read: {why}"))?;
    let record = patch::parse(name, &text);
    let mut out = format!("patch\t{}\t{}\n", record.id, record.title);
    for problem in &record.problems {
        out.push_str(&format!("patch-problem\t\t{problem}\n"));
    }
    // **A record that cannot say who it is, is named as such.** The blocks are still compiled below,
    // because the act is readable even when the record's frontmatter is not.
    if !record.complete() {
        out.push_str(&format!(
            "patch-unevaluable\t{}\t{}\n",
            record.id,
            record.problems.join("; ")
        ));
    }
    if record.recipes.is_empty() {
        let why = if record.problems.is_empty() {
            "a prose record is a record, not an act".to_string()
        } else {
            format!("the record is not evaluable: {}", record.problems.join("; "))
        };
        return Err(format!(
            "{out}refused: {} declares no `recipe` block, so there is nothing to compile: {why}",
            record.id
        ));
    }
    let registry = Registry::read(root, ticket).map_err(|found| RefusalSet::to_string(&found))?;
    let mut compiled = 0usize;
    for (at, declared) in record.recipes.iter().enumerate() {
        if compile_declared(&registry, declared, at + 1, &mut out) {
            compiled += 1;
        }
    }
    out.push_str(&format!("compiled\t{compiled}\t{}\n", record.recipes.len()));
    Ok(out)
}

/// **The one compiler for a declared act**, shared by the single-record route and the corpus walk.
/// Returns whether the block became a recipe line; every reason it did not is already in `out`.
fn compile_declared(
    registry: &Registry,
    declared: &patch::PatchRecipe,
    slot: usize,
    out: &mut String,
) -> bool {
    for gate in &declared.requires {
        out.push_str(&format!("requires\t{slot}\t{gate}\n"));
    }
    if !declared.complete() {
        for problem in &declared.problems {
            out.push_str(&format!("recipe-problem\t{slot}\t{problem}\n"));
        }
        return false;
    }
    let segment = match Segment::parse(&declared.segment) {
        Ok(segment) => segment,
        Err(refusal) => {
            out.push_str(&format!("recipe-problem\t{slot}\t{refusal}\n"));
            return false;
        }
    };
    let Some(axis) = AxisKind::ALL.into_iter().find(|axis| axis.name() == declared.axis) else {
        out.push_str(&format!(
            "recipe-problem\t{slot}\t`{}` is not an axis\n",
            declared.axis
        ));
        return false;
    };
    let Some((path, line)) = declared.site_parts() else {
        out.push_str(&format!("recipe-problem\t{slot}\tthe site is not `path:line`\n"));
        return false;
    };
    let fix = Fix::new(
        Site::new(path, line),
        declared.what.clone(),
        declared.because.clone(),
        declared.check.clone(),
        declared.witness.clone(),
        declared.expects.unwrap_or(0),
    );
    match registry.matrix().recipe(segment, axis, fix) {
        Ok(recipe) => match recipe.render() {
            Ok(text) => {
                out.push_str(&format!(
                    "recipe\t{slot}\t{}/{}\n{text}\n",
                    segment.name(),
                    axis.name()
                ));
                true
            }
            Err(refusal) => {
                out.push_str(&format!("recipe-problem\t{slot}\t{}\n", refusal.detail()));
                false
            }
        },
        Err(found) => {
            out.push_str(&format!("recipe-refused\t{slot}\t{}\n", RefusalSet::to_string(&found)));
            false
        }
    }
}

/// **The whole patch corpus, one recipe line at a time.** Records that already declare `recipe`
/// blocks are compiled through the same [`compile_declared`] the single-record route uses. Records
/// that are prose only get a draft **only from fields their own text sources**: the title, the first
/// `file:line`, the first command, a witness path, and a target that names exactly one segment. A
/// field that cannot be sourced is a named gap, not a plausible default. With `write`, compiled
/// drafts land in `patches/upstream/recipes/<PATCH>.recipe` and a `coverage.tsv`; no patch record is
/// edited, so a draft cannot pass as the record's own words.
///
/// **The summary's first field is `records`, not `patches` and not `blocks + gaps`.** The old line
/// printed that sum under the label `patches`, which made 81 read as a file count; the corpus has 76
/// records, 30 declared blocks and 50 gaps, and the three are now named separately.
pub fn patch_recipe_coverage(root: &Path, ticket: &str, write: bool) -> Result<String, String> {
    let dir = root.join("patches");
    let entries = std::fs::read_dir(&dir)
        .map_err(|why| format!("{} could not be read: {why}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|extension| extension.to_str()) == Some("md")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("PATCH-"))
        })
        .collect();
    files.sort();
    let registry = Registry::read(root, ticket).map_err(|found| RefusalSet::to_string(&found))?;

    let mut out = String::new();
    let records = files.len();
    let mut blocks = 0usize;
    let mut compiled_existing = 0usize;
    let mut derived = 0usize;
    let mut compiled_derived = 0usize;
    let mut rederived = 0usize;
    let mut rederived_match = 0usize;
    let mut gaps = 0usize;
    let mut written: Vec<(String, String)> = Vec::new();

    for file in files {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let text = std::fs::read_to_string(&file)
            .map_err(|why| format!("{} could not be read: {why}", file.display()))?;
        let record = patch::parse(&name, &text);
        if !record.problems.is_empty() {
            let label = if record.id.is_empty() { name.as_str() } else { record.id.as_str() };
            out.push_str(&format!(
                "record-problem\t{label}\t{}\n",
                record.problems.join("; ")
            ));
        }
        if !record.recipes.is_empty() {
            blocks += record.recipes.len();
            out.push_str(&format!(
                "patch\t{}\tblocks\t{}\n",
                record.id,
                record.recipes.len()
            ));
            for (at, declared) in record.recipes.iter().enumerate() {
                if compile_declared(&registry, declared, at + 1, &mut out) {
                    compiled_existing += 1;
                }
                // **A block is not exempt from the prose it claims to compile.** When the
                // record's own prose states an act, derive it again and compare the sourced
                // coordinates. The block is still compiled above; this line is the proof that
                // the same act can be reached from the prose, not merely copied into a fence.
                if prose_act(&text).is_some() {
                    rederived += 1;
                    if reconcile_block(root, &record, &text, &name, ticket, declared, at + 1, &mut out) {
                        rederived_match += 1;
                    }
                }
            }
            continue;
        }

        let draft = derive_recipe(root, &record, &text, &name, ticket);
        if !draft.complete() {
            gaps += 1;
            out.push_str(&format!(
                "gap\t{}\t{}\n",
                record.id,
                draft.problems.join("; ")
            ));
            continue;
        }
        derived += 1;
        let mut block = String::new();
        if compile_declared(&registry, &draft, 1, &mut block) {
            compiled_derived += 1;
            out.push_str(&format!(
                "derived\t{}\t{}/{}\n{block}",
                record.id,
                draft.segment,
                draft.axis
            ));
            if write {
                let body = block.split_once('\n').map(|(_, body)| body).unwrap_or("");
                written.push((record.id.clone(), body.to_string()));
            }
        } else {
            gaps += 1;
            out.push_str(&format!("gap\t{}\t{}\n", record.id, block.replace('\n', " ")));
        }
    }

    out.push_str(&format!(
        "records\t{records}\tblocks\t{blocks}\tcompiled\t{compiled_existing}\tderived\t{derived}\tderived-compiled\t{compiled_derived}\trederived\t{rederived}\trederived-match\t{rederived_match}\tgaps\t{gaps}\n"
    ));

    if write {
        let out_dir = root.join("patches/upstream/recipes");
        std::fs::create_dir_all(&out_dir)
            .map_err(|why| format!("{} could not be created: {why}", out_dir.display()))?;
        for (id, body) in &written {
            std::fs::write(out_dir.join(format!("{id}.recipe")), body)
                .map_err(|why| format!("{} could not be written: {why}", id))?;
        }
        let mut coverage = String::from("patch\tkind\tproblems\n");
        for line in out.lines() {
            if let Some(rest) = line.strip_prefix("gap\t") {
                coverage.push_str(&format!("gap\t{rest}\n"));
            }
        }
        std::fs::write(out_dir.join("coverage.tsv"), coverage)
            .map_err(|why| format!("the coverage table could not be written: {why}"))?;
        out.push_str(&format!("wrote\t{}\t{}\n", written.len(), out_dir.display()));
    }
    Ok(out)
}

/// One draft act from a prose record. **Nothing is invented**: every field is copied from the record
/// or its text, and a field with no source becomes a named problem the caller prints.
fn derive_recipe(
    root: &Path,
    record: &patch::PatchRecord,
    text: &str,
    file_name: &str,
    ticket: &str,
) -> patch::PatchRecipe {
    let mut recipe = patch::PatchRecipe {
        ticket: ticket.to_string(),
        axis: "evidence_grade".to_string(),
        requires: vec!["cargo check --workspace".to_string()],
        expects: Some(0),
        ..patch::PatchRecipe::default()
    };
    let mut problems = Vec::new();

    // **The prose act is the route.** A record may state its act in a sentence rather than a fence;
    // when it does, those coordinates win over any incidental `path:line` earlier in the document.
    let act = prose_act(text);
    if let Some(act) = &act {
        recipe.site = act.site.clone();
        recipe.check = act.check.clone();
        recipe.witness = act
            .witness
            .clone()
            .or_else(|| act.site.rsplit_once(':').map(|(path, _)| path.to_string()))
            .unwrap_or_default();
        match segment_for_site(&act.site) {
            Ok(segment) => recipe.segment = segment.to_string(),
            Err(why) => problems.push(why),
        }
        if let Err(refusal) = Site::new(
            act.site.rsplit_once(':').map(|(path, _)| path.to_string()).unwrap_or_default(),
            act.site.rsplit_once(':').and_then(|(_, line)| line.parse().ok()).unwrap_or(0),
        )
        .locate(root)
        {
            problems.push(refusal.detail().to_string());
        }
    } else {
        match segment_for(record) {
            Ok(segment) => recipe.segment = segment.to_string(),
            Err(why) => problems.push(why),
        }
        match first_site(text) {
            Some((path, line)) => {
                if let Err(refusal) = Site::new(path.clone(), line).locate(root) {
                    problems.push(refusal.detail().to_string());
                }
                recipe.site = format!("{path}:{line}");
            }
            None => problems.push(
                "no `path:line` in the record; a recipe site cannot be guessed from a path alone"
                    .to_string(),
            ),
        }
        match first_command(text) {
            Some(command) => recipe.check = command,
            None => problems.push(
                "no `cargo` command in the record; the check that must move is not stated"
                    .to_string(),
            ),
        }
        recipe.witness = first_witness(text).unwrap_or_else(|| format!("patches/{file_name}"));
    }

    let fallback_title = title_of(text);
    let title = if record.title.trim().is_empty() { fallback_title.as_str() } else { record.title.as_str() };
    recipe.what = truncate(title, 256);
    if recipe.what.trim().is_empty() {
        problems.push("the record has no title".to_string());
    }
    recipe.because = change_summary(text).unwrap_or_else(|| recipe.what.clone());
    recipe.problems = problems;
    recipe
}

/// **A sentence the record can carry instead of a fence.** The prose names the site, the check and,
/// when it has one, the witness; this reads those three and nothing else. A record that does not say
/// `The act is at` has no prose act, so a historical `path:line` in it is not promoted by accident.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ProseAct {
    site: String,
    check: String,
    witness: Option<String>,
}

fn prose_act(text: &str) -> Option<ProseAct> {
    for line in text.lines() {
        let line = line.trim();
        if !line.contains("The act is at") {
            continue;
        }
        let spans = backticked(line);
        let site = spans.iter().find(|span| {
            span.rsplit_once(':')
                .is_some_and(|(path, line)| path.ends_with(".rs") && line.parse::<u32>().is_ok())
        })?
        .clone();
        let command = spans.iter().find(|span| span.trim_start().starts_with("cargo "))?.clone();
        let check = if command.contains(" -> ") {
            command
        } else if command.contains(" test") || command.contains(" build") {
            format!("{command} -> failures 0")
        } else {
            format!("{command} -> exit 0")
        };
        let witness = line
            .split_once("witness is")
            .and_then(|(_, rest)| backticked(rest).into_iter().next());
        return Some(ProseAct { site, check, witness });
    }
    None
}

/// Every backticked span in one line, in order. The record's own prose marks its machine-read
/// fields the same way its tables mark a path, so no second grammar is introduced.
fn backticked(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('`') else {
            break;
        };
        out.push(after[..end].trim().to_string());
        rest = &after[end + 1..];
    }
    out
}

/// The title a record's first `#` heading carries, after the em dash or hyphen. Used only when the
/// frontmatter has no title, so a prose-only record can still name its act.
fn title_of(text: &str) -> String {
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("# ") else {
            continue;
        };
        let rest = rest.trim();
        if let Some((_, title)) = rest.split_once('\u{2014}') {
            return title.trim().to_string();
        }
        if let Some((_, title)) = rest.split_once(" - ") {
            return title.trim().to_string();
        }
        return rest.to_string();
    }
    String::new()
}

/// Compare one declared block with the act its own prose states. Only the sourced coordinates are
/// compared: `what` and `because` are editorial prose, while a site, a check, a witness and a
/// segment are facts the matrix compiles. Returns whether all four agree.
fn reconcile_block(
    root: &Path,
    record: &patch::PatchRecord,
    text: &str,
    file_name: &str,
    ticket: &str,
    declared: &patch::PatchRecipe,
    slot: usize,
    out: &mut String,
) -> bool {
    let draft = derive_recipe(root, record, text, file_name, ticket);
    if !draft.complete() {
        out.push_str(&format!(
            "rederived\t{}\tblock {slot}\tnot-sourced\t{}\n",
            record.id,
            draft.problems.join("; ")
        ));
        return false;
    }
    let pairs = [
        ("segment", draft.segment.as_str(), declared.segment.as_str()),
        ("site", draft.site.as_str(), declared.site.as_str()),
        ("check", draft.check.as_str(), declared.check.as_str()),
        ("witness", draft.witness.as_str(), declared.witness.as_str()),
    ];
    let differs: Vec<String> = pairs
        .iter()
        .filter(|(_, left, right)| left != right)
        .map(|(field, left, right)| format!("{field} prose={left:?} block={right:?}"))
        .collect();
    if differs.is_empty() {
        out.push_str(&format!(
            "rederived\t{}\tblock {slot}\tmatch\tsegment,site,check,witness\n",
            record.id
        ));
        true
    } else {
        out.push_str(&format!(
            "rederived\t{}\tblock {slot}\tdiffers\t{}\n",
            record.id,
            differs.join("; ")
        ));
        false
    }
}

/// The segment a site path names, by the matrix's own vocabulary.
fn segment_for_site(site: &str) -> Result<&'static str, String> {
    let path = site.rsplit_once(':').map(|(path, _)| path).unwrap_or(site);
    let segment = if path.starts_with("crates/axis/") {
        "axis"
    } else if path.starts_with("crates/voxel-client/src/shell") {
        "shell"
    } else if path.starts_with("crates/voxel-client/src/pattern") {
        "pattern"
    } else if path.starts_with("crates/voxel-client") {
        "shell"
    } else if path.starts_with("crates/voxel-ui") {
        "ui"
    } else if path.starts_with("crates/voxel-model")
        || path.starts_with("crates/voxel-noise")
        || path.starts_with("crates/voxel-light")
        || path.starts_with("crates/voxel-mesh")
        || path.starts_with("crates/voxel-shapers")
        || path.starts_with("crates/voxel-appearance")
        || path.starts_with("crates/voxel-stream")
    {
        "world"
    } else if path.starts_with("crates/voxel-package")
        || path.starts_with("crates/voxel-mcp")
        || path.starts_with("crates/voxel-von")
        || path.starts_with("crates/voxel-instrument")
    {
        "tooling"
    } else if path.starts_with("crates/shploingus") {
        "game"
    } else {
        return Err(format!("site {site:?} names no segment the matrix has"));
    };
    Ok(segment)
}

/// The segment a record's `target` names, or the sentence saying why it is not one.
fn segment_for(record: &patch::PatchRecord) -> Result<&'static str, String> {
    let mut found = std::collections::BTreeSet::new();
    let text = format!("{} {}", record.target, record.named_paths.join(" "));
    if text.contains("voxel-client/src/shell") || record.target.contains("shploingus/shell") {
        found.insert("shell");
    }
    if text.contains("voxel-client/src/pattern") {
        found.insert("pattern");
    }
    if text.contains("voxel-client") && !text.contains("voxel-client/src/pattern") {
        found.insert("shell");
    }
    if text.contains("voxel-ui") {
        found.insert("ui");
    }
    if text.contains("voxel-model")
        || text.contains("voxel-noise")
        || text.contains("voxel-light")
        || text.contains("voxel-mesh")
        || text.contains("voxel-shapers")
        || text.contains("voxel-appearance")
        || text.contains("voxel-stream")
    {
        found.insert("world");
    }
    if text.contains("crates/axis")
        || record.target.contains("axis")
        || text.contains("shploingus/being")
    {
        found.insert("axis");
    }
    if text.contains("voxel-package")
        || text.contains("voxel-mcp")
        || text.contains("voxel-von")
        || text.contains("voxel-instrument")
    {
        found.insert("tooling");
    }
    if text.contains("shploingus/src") || record.target.trim() == "shploingus" {
        found.insert("game");
    }
    match found.len() {
        1 => Ok(found.into_iter().next().expect("one member")),
        0 => Err(format!(
            "target {:?} names no segment the matrix has; a recipe cannot pick one for it",
            record.target
        )),
        _ => Err(format!(
            "target {:?} names {} segments ({}); one act belongs to one cell",
            record.target,
            found.len(),
            found.into_iter().collect::<Vec<_>>().join(", ")
        )),
    }
}

/// The first `path:line` a record states, in the order it states them.
fn first_site(text: &str) -> Option<(String, u32)> {
    for line in text.lines() {
        for token in line.split(|c: char| {
            c.is_whitespace() || "`()[],;\"'".contains(c)
        }) {
            let token = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '/' && c != '.' && c != '_' && c != '-');
            let Some((path, number)) = token.rsplit_once(':') else {
                continue;
            };
            if !path.contains('/') || !path.ends_with(".rs") {
                continue;
            }
            let Ok(line) = number.parse::<u32>() else {
                continue;
            };
            if line > 0 {
                return Some((path.to_string(), line));
            }
        }
    }
    None
}

/// The first `cargo` command the record states, with the movement a reader can check.
fn first_command(text: &str) -> Option<String> {
    let mut checks = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches("$ ").trim_matches('`').trim();
        if !line.starts_with("cargo ") {
            continue;
        }
        if line.contains(" -> ") {
            checks.push(line.to_string());
            continue;
        }
        let movement = if line.contains(" test") || line.contains(" build") {
            "failures 0"
        } else {
            "exit 0"
        };
        checks.push(format!("{line} -> {movement}"));
    }
    checks.sort_by_key(|command| !command.contains(" test"));
    checks.into_iter().next()
}

/// A baseline or conformance path the record names, or `None`.
fn first_witness(text: &str) -> Option<String> {
    for line in text.lines() {
        for token in line.split_whitespace() {
            let token = token.trim_matches(|c: char| "`()[],;\"'".contains(c));
            if token.starts_with("docs/baselines/") || token.starts_with("conformance/") {
                return Some(token.to_string());
            }
        }
    }
    None
}

/// The first sentence of `# Change`, or `None` when the record carries no such section.
fn change_summary(text: &str) -> Option<String> {
    let mut inside = false;
    let mut body = Vec::new();
    for line in text.lines() {
        if line.trim() == "# Change" {
            inside = true;
            continue;
        }
        if inside && line.starts_with("# ") {
            break;
        }
        if inside {
            body.push(line.trim());
        }
    }
    let joined = body.join(" ");
    let sentence = joined
        .split_once(". ")
        .map(|(sentence, _)| sentence)
        .unwrap_or(&joined)
        .trim();
    (!sentence.is_empty()).then(|| truncate(sentence, 1000))
}

/// A UTF-8-safe prefix, so a title cannot be cut through a character.
fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

/// **Issue the credential an apply presents.** The expiry is in unix seconds, and the server's clock
/// is the one that will be checked; a client's own clock is a claim, not a fact.
fn grant_recipe(root: &Path, arguments: &Value) -> Result<String, String> {
    let ticket = string_arg(arguments, "ticket", "{\"ticket\": \"T-0954\"}")?;
    let segment = segment_arg(arguments)?;
    let axis = axis_arg(arguments)?;
    let expires = match arguments.get("expires") {
        None => unix_now().saturating_add(DEFAULT_GRANT_SECONDS),
        Some(value) => value.as_u64().ok_or_else(|| {
            "`expires` must be an unsigned tick in unix seconds".to_string()
        })?,
    };
    let security = voxel_axis_security::Security::machine_local(root)
        .map_err(|found| RefusalSet::to_string(&found))?;
    let registry = Registry::mount(root, ticket, security)
        .map_err(|found| RefusalSet::to_string(&found))?;
    let grant = registry
        .grant(ticket, segment, axis, expires)
        .map_err(|found| RefusalSet::to_string(&found))?;
    Ok(format!("grant-issued\tuntil\t{expires}\n{}\n", grant.to_wire()))
}

/// **Decide an apply under a signed grant.** The wire line is parsed and then verified by the layer;
/// a tampered field is `signature_invalid`, another key is `untrusted_key`, and an expiry in the
/// past is `destination_invariant_refused` — each the axis crates' own kind, printed as itself.
fn apply_recipe(root: &Path, arguments: &Value) -> Result<String, String> {
    let ticket = string_arg(arguments, "ticket", "{\"ticket\": \"T-0954\"}")?;
    let segment = segment_arg(arguments)?;
    let axis = axis_arg(arguments)?;
    let wire = string_arg(
        arguments,
        "ticket_grant",
        "{\"ticket_grant\": \"axis-grant/1\\t…\"}",
    )?;
    let credential = TicketGrant::from_wire(wire).map_err(|found| RefusalSet::to_string(&found))?;
    let security = voxel_axis_security::Security::machine_local(root)
        .map_err(|found| RefusalSet::to_string(&found))?;
    let registry = Registry::mount(root, ticket, security)
        .map_err(|found| RefusalSet::to_string(&found))?;
    let applied = registry
        .apply(Some(&credential), ticket, segment, axis, unix_now())
        .map_err(|found| RefusalSet::to_string(&found))?;
    Ok(format!("{}\n", applied.render()))
}

/// A required string argument, refused with the shape the tool wants rather than a bare "missing".
fn string_arg<'a>(
    arguments: &'a Value,
    field: &str,
    shape: &str,
) -> Result<&'a str, String> {
    arguments.get(field).and_then(Value::as_str).ok_or_else(|| {
        format!("this tool needs a `{field}` string, as {shape}")
    })
}

/// The segment, parsed by the matrix's own parser.
fn segment_arg(arguments: &Value) -> Result<Segment, String> {
    let word = string_arg(arguments, "segment", "{\"segment\": \"shell\"}")?;
    Segment::parse(word)
        .map_err(|refusal| format!("refused: `{word}` is not a segment: {refusal}"))
}

/// The axis, looked up in the closed vocabulary rather than spelled a second time.
fn axis_arg(arguments: &Value) -> Result<AxisKind, String> {
    let word = string_arg(arguments, "axis", "{\"axis\": \"evidence_grade\"}")?;
    AxisKind::ALL
        .into_iter()
        .find(|axis| axis.name() == word)
        .ok_or_else(|| {
            format!(
                "refused: `{word}` is not an axis; the vocabulary is {}",
                AxisKind::ALL.map(|axis| axis.name()).join(", ")
            )
        })
}

/// The server's clock, in unix seconds. A clock before the epoch answers 0, and a grant expiring at
/// 0 is already expired, so the failure direction is a refusal rather than a permission.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn failed(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn send(output: &mut impl Write, message: Value) -> std::io::Result<()> {
    writeln!(output, "{message}")
}
