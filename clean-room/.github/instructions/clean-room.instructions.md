---
name: Clean Room
description: The clean room's P1/P2/P3/P4/P6/P7 rules for every Rust source file — auto-applied
applyTo: "src/**/*.rs"
---
<!-- Generated as frontmatter by microsoft/agent-forge (`forge generate`, 2026-10-11); the body is
     this project's, authored before the generator ran and kept on reconciliation. The generated
     `applyTo: "**/*"` was narrowed to this project's sources, because the rules below are about
     `.rs` files and a `**/*` glob would claim the specs too. See
     docs/CLEAN-ROOM-PLAN-2026-10-11.md §4.2 and §8. -->

# Rust rules for the clean room

Every `.rs` file under `clean-room/`:

- starts with a `//!` module header that **names the document it was derived from** (P1), and never
  mentions a legacy file as a source;
- declares refusals as **named variants**, not booleans and not defaults (P2);
- keeps every function it exposes **pure** unless it is an explicit seam, and documents the seam (P3);
- carries a `#[cfg(test)]` module whose tests **print the measurement** the spec's success criteria
  name (P4);
- adds **no dependency** without a line beside it saying what it does that we need and that it is
  actually used;
- ports parent code deliberately where a seam is being reconciled (P7, amended 2026-10-13): a port
  declares its `path` dependency, pins the parent bytes it read by `path + digest`, and lands under
  the convention — measured figures, refusals by name, guards proven to bite, bytes digested.

Before marking a task complete, run:

```bash
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
```
