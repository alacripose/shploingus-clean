---
name: "rust"
description: "Rust library implementation for a deterministic voxel world, presentation layer, and spec-driven U3 Shapers. USE FOR: Rust crate changes, Cargo configuration, deterministic voxel generation, voxel-world rules, presentation-layer types, spec 001, spec 002, U3 Shapers, Rust #[test] conformance suite, refuse never default, spec-kit Markdown under specs. DO NOT USE FOR: adding external dependencies, non-Rust application code, unrelated infrastructure, replacing explicit refusals with fallback values."
argument-hint: "[voxel-world, presentation, or U3 Shapers task]"
user-invokable: true
disable-model-invocation: false
---

# Rust Clean-Room Library

## Overview
Use this skill for work on the clean-room Rust library: its deterministic voxel-world core (spec 002), presentation layer (spec 001), and the next feature, U3 Shapers. Treat the spec-kit documents under `specs/` as the source of behavioral requirements.

The governing rule is refuse, never default. Invalid or missing inputs must produce an explicit refusal or error rather than a substituted value. Preserve the crate's zero-external-dependency constraint and its executable 52-test conformance suite.

## Core Concepts
- Specification-driven behavior: trace changes to the relevant spec before implementing them, and keep implementation, tests, and documentation aligned.
- Determinism: identical valid inputs must produce reproducible voxel-world results without implicit variation.
- Explicit refusal: make invalid or missing inputs observable through the project's established error or refusal behavior.
- Presentation boundaries: follow spec 001's Rust types and conversion boundaries; keep conversions explicit and handle invalid data without masking it.
- Conformance: the built-in Rust `#[test]` harness is the executable conformance suite; maintain the 52-test expectation without external dependencies.
- U3 Shapers: treat this as a spec-driven feature; establish the intended behavior from `specs/` before choosing types, rules, or conversions.

## Common Patterns
### Implementing voxel-world behavior
Input: a requested change to voxel-world rules or generation.

Pattern: find the relevant requirement in spec 002, identify the valid inputs and refusal cases, then implement behavior so identical valid inputs remain reproducible. Add or update focused conformance tests for valid and invalid cases.

Output: spec-aligned Rust behavior with explicit handling of invalid or missing inputs and corresponding tests.

### Changing presentation types or conversions
Input: a presentation-layer change or a request to connect world data to presentation data.

Pattern: consult spec 001 and use its defined types and conversion boundaries. Keep conversion decisions visible; return the established explicit error or refusal when the input cannot be represented.

Output: a clear boundary between world and presentation data without silent fallback values.

### Implementing U3 Shapers
Input: a U3 Shapers feature request.

Pattern: trace each requested behavior to the spec-kit documents under `specs/`. Implement only behavior supported by the specs, preserve the refuse-never-default rule, and add conformance tests for the specified cases.

Output: spec-backed U3 Shapers behavior with tests and documentation updated as needed.

### Verifying a change
Input: a Rust, spec, or documentation change.

Pattern: run the crate's built-in tests through Cargo. Check that the conformance suite remains executable, that no external dependencies were introduced, and that spec-kit Markdown and crate documentation stay consistent with changed behavior.

Output: a verified change with any remaining test failures or specification ambiguities reported explicitly.

## Pitfalls
- Do not substitute defaults for invalid or missing inputs; refuse explicitly.
- Do not infer unspecified behavior; check the applicable spec and surface ambiguity instead of inventing semantics.
- Do not compromise determinism; keep results reproducible for identical valid inputs.
- Do not blur presentation conversions; respect spec 001's types and boundaries and handle failed conversions explicitly.
- Do not add external dependencies; use Rust and Cargo's capabilities and the built-in `#[test]` harness.
- Do not update implementation alone when expectations change; keep specs, conformance tests, and crate documentation aligned.
