//! **The classifier port: this project's questions, answered by a model it did not write.**
//!
//! # Why a model is here at all, and what it may decide
//!
//! `laya` (the decision layer) judges an artifact by asking one of three questions — `choice`,
//! `score` or `noul` — and this crate is the other end of that: [`von`] loads the ONNX export the
//! sibling tree measured (`Zatsepin/von-onnx-fp16`, CPU), and [`answers`] asks it the question the
//! decision layer declares.
//!
//! **A model's answer is evidence, not authority.** That is the ticket-evidence ladder's own rule
//! (`docs/LEDGER.md`, L-5: *agent self-report is evidence, not authority*), and it is why the
//! answer carries [`Origin::Oracle`] — the `A` rung, something outside this project decided it —
//! rather than being promoted into a verdict here. The decision layer decides what an `A`-rung
//! answer is worth; this crate only gets the answer and says who gave it.
//!
//! # Cost, measured rather than assumed
//!
//! Upstream's own measurement (`planet-shploingus/docs/VON-SPIKE.md`) is the cost model this crate
//! is built around: loading from cache ~3 s, one ask p50 **17.2 s** for a three-question duty set,
//! **1.44 s** for a single `noul`, and *"cost follows the question structure, not the state"* —
//! growing the state 100× added 8 %, while a 6-option choice costs ~12× a yes/no question. So:
//! the backend is held for the life of a session rather than reloaded per ask, and a caller asking
//! n questions pays n ballots. **Determinism held across repeats**, and there is *no answer cache*
//! upstream (*"content-addressed caching is ours to add"*), which is why [`answers`] is a pure
//! function of the state, the question and the model id: the same inputs are the same answers.
//!
//! **The audit, 2026-10-10 — and this is the seam's real status.** 758 lines, 2 modules, 15 public
//! items, 6 tests, read by one crate: `voxel-mcp`. The chain is **`voxel-von` → `voxel-mcp` → nobody**:
//! a tool server that nothing in this workspace consumes. So the classifier is reachable only through
//! an entry point no caller uses, and none of its outputs — judgments, ballots — reach the game. Against
//! the user's own reading of it (*"our laya von musli format package loader system … still isn't
//! providing actual code and information for the system to use"*): measured, that is accurate, and this
//! crate and `voxel-mcp` sit outside the game's dependency path entirely. **Owed: one decision** — is
//! the seam a gate instrument, where it is now, or a developer feedback loop, where it was meant to be?
//! Those are different products, and this crate carries both jobs' code.
#![warn(missing_docs)]

use std::collections::HashMap;

use jigor::{Answer, Backend, Question, VonBackend};

/// The judgment: a ballot's floor against the model's answer.
pub mod judge;

/// The model this tree measured, named so a second model is a visible change rather than a
/// surprise. `VonBackend::new()` resolves exactly this id.
pub const MODEL: &str = "Zatsepin/von-onnx-fp16";

/// The classifier, loaded once and asked many times.
///
/// **A newtype rather than a bare backend**, because the two facts a caller needs about a loaded
/// model — which id it is, and that it is loaded — should not be inferable only from a type name
/// someone has to look up.
pub struct Von {
    backend: VonBackend,
    model: String,
}

impl Von {
    /// Load the measured model from the cache. **Not lazy, and deliberately.**
    ///
    /// A load that happens inside the first question would make the first ask 3 s slower than
    /// every other one and put the failure — a missing cache, a mismatched export — in the middle
    /// of a judgment instead of at its start.
    pub fn load() -> Result<Self, String> {
        let backend = VonBackend::new().map_err(|failure| {
            format!(
                "{MODEL} could not be loaded: {failure}. The ONNX export is cached under \
                 `jigor::cache_dir()` (`JIGOR_CACHE_DIR` overrides it); a missing cache is a \
                 download, not a missing feature"
            )
        })?;
        Ok(Self {
            backend,
            model: MODEL.to_string(),
        })
    }

    /// The model id this instance answers with.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Where the model cache is, as the library resolves it — printed rather than assumed, because
    /// a judgment that read a different model than the one it names is not a judgment.
    pub fn cache_dir() -> std::path::PathBuf {
        jigor::cache_dir()
    }

    /// Ask this model the declared questions about one state.
    pub fn answers(
        &mut self,
        state: &serde_json::Value,
        questions: &[Question],
    ) -> Result<HashMap<String, Answer>, String> {
        self.backend
            .answers(state, questions, None)
            .map_err(|failure| format!("{} could not answer: {failure}", self.model))
    }
}

/// Whether a classifier can be loaded at all, without loading one: the four files the export needs.
///
/// **A question asked of the cache, not of the network.** A caller deciding whether to judge
/// dynamically can ask this and get `false` in microseconds rather than discovering it after a
/// download attempt inside a judgment.
pub fn cached() -> bool {
    let dir = jigor::cache_dir();
    let model = dir.join("models--Zatsepin--von-onnx-fp16");
    model.join("refs").join("main").is_file()
        || std::fs::read_dir(&model)
            .map(|entries| {
                entries.flatten().any(|entry| {
                    entry
                        .path()
                        .join("snapshots")
                        .is_dir()
                })
            })
            .unwrap_or(false)
}

/// **A yes/no judgment** — the shape the sibling tree's `noul` names, and one ballot.
pub fn noul(id: &str, instructions: &str) -> Question {
    jigor::noul(id, instructions)
}

/// **One option from a fixed set.** Each option is both the answer key and its own description.
pub fn choice(id: &str, instructions: &str, options: &[&str]) -> Question {
    jigor::choice(id, instructions, options)
}

/// **A position on an ordered scale**, weakest to strongest — an ordinal, never a score to average.
pub fn score(id: &str, instructions: &str, levels: &[&str]) -> Question {
    jigor::score(id, instructions, levels)
}

/// The probability of `yes`, when the answer was a `noul`. `None` for the other shapes, so a caller
/// cannot read a choice's confidence as a probability of yes.
pub fn yes_probability(answer: &Answer) -> Option<f32> {
    match answer {
        Answer::Noul { probability } => Some(*probability),
        _ => None,
    }
}

/// The answer on the wire, in the library's own shape.
pub fn to_wire(answer: &Answer) -> serde_json::Value {
    jigor::answer_to_json(answer)
}
