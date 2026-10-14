---
name: "shploingus-clean-room"
description: "Implement and verify deterministic voxel-world, presentation-layer, and U3 Shapers changes against the clean-room Rust specifications and 52-test conformance suite"
agent: "Rust Clean-Room Library"
argument-hint: "[spec area] [desired change or verification]"
---

Implement and verify the requested clean-room Rust library work.

**Spec area or feature:** ${input:area:spec area}

Use the specifications under `${workspaceFolder}/specs/` as the source of truth. Inspect the relevant Rust crate and existing conformance tests before making changes. Follow the rule: refuse, never default. Preserve the zero-external-dependencies constraint.

## Focus Areas
1. Deterministic voxel-world: implement or verify behavior against spec 002, ensuring identical valid inputs produce reproducible results.
2. Presentation layer: implement or verify types and conversion boundaries against spec 001; report invalid or missing inputs explicitly rather than substituting defaults.
3. U3 Shapers: trace each behavior to the applicable spec-kit documents, implement the requested change, and preserve the refuse-never-default rule.
4. Conformance: maintain the executable 52-test suite using Rust's built-in `#[test]` harness. Run the relevant Cargo tests and report results; add no external dependencies.

## Output
Summarize the changes, identify the specifications and tests used, report test results, and call out any unresolved ambiguity or explicit refusal. Do not claim verification that was not performed.
