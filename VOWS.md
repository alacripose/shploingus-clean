### YOU MUST ADHERE TO THE FOLLOWING PRACTICES FOR DEVELOPMENT
- Never rationalize for more than a quick moment without asking user.
- Never attempt shortcuts.
- Never hallucinate, assume, or decide without asking user for consent.
- Never create scaffolds/mock/boilerplates/AI Slop/or underbuild.
- Think rarely and on a budget: mechanical work (searches, lints, single-file edits, test runs) acts immediately with zero deliberation; anything warranting planning gets exactly ONE deliberate pass, and that pass must be visible as a written plan — never silent reasoning loops, repeated re-reads, or same-model re-derivations.
- Gather ALL context first, then plan with the best reasoning path available: start file-picker + code-searcher in parallel, read every file the change touches (symbols, current behavior, conventions, tests), and produce a solid build plan in the standard format (goal → files to touch with why → change list → risks → validation) — via a Thinker agent when one is available, otherwise by planning directly with an adversarial review before the plan reaches the user.
- Must ask the user if they approve of the build plan before implementing.
- Docstrings can be NO LONGER than 1 or 2 sentences.

### BUILD PROTOCOL
Mechanical work acts immediately. Anything warranting a plan gets exactly one deliberate, context-complete planning pass (Vows 5–6); that plan is shown for approval before the first edit (Vow 7); implementation proceeds only on approval, then is validated and reviewed.

# Project Roadmap: Rust Autonomous TODO Engine (RATE)

> **Agent Protocol:** You have access to the `write_todo` tool. Use it autonomously whenever you discover open-ended architecture needs, unmapped edge cases, or sub-routines that require isolated validation.
> Format your tool executions against the schema defined below.

---

## 🛠️ Tool Definition: `write_todo`
When discovering an open-ended task requirement, execute the tool call using this exact schema structure:
```json
{
  "name": "write_todo",
  "arguments": {
    "phase": "Target Phase Header Name",
    "task_title": "Concise statement of the open-ended task",
    "dependencies": ["Task X.X"],
    "criteria": "Programmatic definition of success (e.g., specific cargo tests or compiler checks)"
  }
}
```

---

## 🟩 Phase 1: Core Parsing & Architecture
- [ ] Task 1.1: Initialize project workspace and foundational CLI definitions
  <!-- STATUS: Pending -->
  <!-- DEPENDENCIES: None -->
  <!-- CRITERIA: Create binary crate with `clap` for CLI argument parsing (`--todo-file`). -->
- [ ] Task 1.2: Implement Markdown AST parser for TODO checklist extraction
  <!-- STATUS: Pending -->
  <!-- DEPENDENCIES: Task 1.1 -->
  <!-- CRITERIA: Use `pulldown-cmark` to parse checklist items, metadata blocks, and agent signatures. -->
- [ ] Task 1.3: Design the State Machine and Git Branch isolation manager
  <!-- STATUS: Pending -->
  <!-- DEPENDENCIES: Task 1.2 -->
  <!-- CRITERIA: Module using `git2-rs` to automatically checkout task-specific branches (e.g., `agent/task-1.3`). -->

## 🟨 Phase 2: Agent Execution Loop & Context Windows
- [ ] Task 2.1: Build async file-watcher for dynamic TODO updates
  <!-- STATUS: Pending -->
  <!-- DEPENDENCIES: Task 1.3 -->
  <!-- CRITERIA: Integrate `notify` crate to detect structural manual changes to `TODO.md`. -->
- [ ] Task 2.2: Implement LLM API Connector and Token Context Optimizer
  <!-- STATUS: Pending -->
  <!-- DEPENDENCIES: Task 2.1 -->
  <!-- CRITERIA: Use `reqwest` to call the LLM backend. Inject structural `src/` hierarchy layout maps into the context window. -->
- [ ] Task 2.3: Engineering runtime sandbox safety boundary
  <!-- STATUS: Pending -->
  <!-- DEPENDENCIES: Task 2.2 -->
  <!-- CRITERIA: Create execution wrapper that executes `cargo check` and handles compile error loops gracefully. -->

## 🔍 Discovered Context & Open-Ended Tasks
<!-- TARGET ZONE: The `write_todo` tool will autonomously inject discovered edge cases and unmapped requirements below this line -->

- [ ] Task O.1: Handle multi-agent race conditions on file-write operations
  <!-- STATUS: Pending -->
  <!-- DEPENDENCIES: Task 2.1 -->
  <!-- CRITERIA: Create an integration test where 3 threads call `write_todo` simultaneously; ensure a thread-safe file lock prevents corruption. -->

---

## 📈 Active Execution Ledger

| Agent ID | Task Reference | Target Branch | Start Time (UTC) | Status / Heartbeat |
| :--- | :--- | :--- | :--- | :--- |
| `None` | `None` | `None` | `None` | Waiting for agent initialization... |
