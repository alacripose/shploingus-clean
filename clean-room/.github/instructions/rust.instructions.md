---
name: "rust"
description: "Rust crate architecture, deterministic voxel-world and presentation behavior from specs 001 and 002, explicit refusal for invalid or missing inputs, zero external dependencies, and Cargo's built-in test harness"
applyTo: "**/*.{rs,md,toml,lock}"
---

# Rust Clean-Room Library Standards

## Rust Architecture
- Keep voxel-world logic and presentation-layer conversions in clearly separated modules, following the boundaries defined in specs 001 and 002. Separation makes each behavior traceable to specification and keeps presentation concerns from changing world rules.
- Use explicit types and fallible return values at module boundaries instead of encoding invalid states as sentinel values. Callers must be able to distinguish valid results from refusals.
- Trace new behavior, including U3 Shapers, to the spec-kit documents under `specs/` before implementing it. Specifications are the source of truth for behavior.

```rust
pub mod presentation;
pub mod voxel_world;

pub use presentation::PresentError;
pub use voxel_world::World;
```

## Deterministic Voxel-World and Presentation Behavior
- Ensure identical valid inputs produce identical voxel-world results. Reproducibility is required for conformance and reliable testing.
- Avoid implicit sources of variation such as ambient randomness, current time, or iteration over unordered data unless a specification explicitly defines them. Hidden variation makes results difficult to reproduce.
- Keep presentation conversions explicit and consistent with spec 001. Conversion boundaries must not silently change voxel-world meaning.

```rust
pub fn build_world(input: &WorldInput) -> Result<World, WorldError> {
    validate_world_input(input)?;
    Ok(World::from_valid_input(input))
}
```

## Explicit Refusal for Invalid or Missing Inputs
- Return an explicit error or refusal when required input is absent or invalid. Substituting a default would hide a violation of the refuse-never-default rule.
- Do not use fallback values, silent coercion, or "best effort" behavior to make invalid inputs appear valid. Callers and tests must observe the refusal.
- Preserve the reason for refusal where the API allows it. Useful error information supports conformance checks without weakening validation.

```rust
pub fn required_dimension(value: Option<u32>) -> Result<u32, InputError> {
    value.ok_or(InputError::MissingDimension)
}
```

## Zero External Dependencies
- Keep the Rust crate free of external dependencies. The clean-room implementation and conformance suite must rely on Rust and Cargo's built-in capabilities.
- Do not add a crate to `Cargo.toml` or a dependency entry to `Cargo.lock`. Dependency changes violate the zero-external-dependencies constraint.

```toml
[package]
name = "clean-room"
version = "0.1.0"
edition = "2021"

[dependencies]
```

## Cargo Tests and Conformance
- Use Rust's built-in `#[test]` harness and run the suite with Cargo. This keeps verification available without external test frameworks.
- Maintain the executable 52-test conformance suite as specifications and behavior evolve. Changes must preserve coverage of the defined contract.
- Add tests for refusals as well as valid behavior. Explicitly checking invalid and missing inputs prevents defaults from slipping into the implementation.

```rust
#[test]
fn missing_dimension_is_refused() {
    assert_eq!(
        required_dimension(None),
        Err(InputError::MissingDimension)
    );
}
```

## Specification and Documentation Updates
- Update relevant spec-kit Markdown and Rust crate documentation together when behavior or conformance expectations change. Synchronized documentation keeps implementation and contract aligned.
- Keep documentation examples consistent with actual fallible APIs and deterministic behavior. Examples that imply defaults or nondeterminism misrepresent the crate's guarantees.

```rust
/// Builds a world from valid input, refusing invalid or missing values.
pub fn build_world(input: &WorldInput) -> Result<World, WorldError> {
    validate_world_input(input)?;
    Ok(World::from_valid_input(input))
}
```
