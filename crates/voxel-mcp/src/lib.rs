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

use serde_json::{json, Value};
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
pub const TOOLS: [&str; 9] = [
    "get_work_list",
    "get_conformance",
    "get_census",
    "get_patch_set",
    "get_ownership",
    "get_test_bloat",
    "get_api_view",
    "judge_work",
    "reject_non_conformance",
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
            let schema = if *name == "get_ownership" {
                json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"],
                    "additionalProperties": false
                })
            } else {
                json!({"type": "object", "properties": {}, "additionalProperties": false})
            };
            json!({"name": name, "description": description(name), "inputSchema": schema})
        })
        .collect();
    Value::Array(tools)
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
        "get_census" => "The whole crate-by-axis table, as the build gate prints it.",
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
    let receipt = receipt(name, authority, grant);
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
        "get_census" => Ok(census::report(&readings(root)?)),
        "get_patch_set" => patch_set(root),
        "get_ownership" => ownership(root, arguments),
        "get_test_bloat" => Ok(bloat::measure_workspace_tests(root).describe()),
        "get_api_view" => Ok(api_view()),
        "judge_work" => judge_work(root, arguments),
        "reject_non_conformance" => reject_non_conformance(root),
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

fn ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn failed(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn send(output: &mut impl Write, message: Value) -> std::io::Result<()> {
    writeln!(output, "{message}")
}
