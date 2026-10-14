# Tasks 001 — U1 Presentation

- [x] T001 Setup: create the clean-room Cargo project as its **own** workspace so the legacy
      workspace cannot claim it — `Cargo.toml`, `.gitignore` · *check:* `cargo test` runs in
      `clean-room/` and the repository-root `cargo test` still ignores it.
- [x] T002 Define the unified-systems registry U1–U8 from the work plan — `src/systems.rs` ·
      *check:* `the_registry_is_closed_and_ordered` asserts eight systems, unique ids, ascending slice.
- [x] T003 Declare the appearance types and refusals — `src/presentation.rs` · *check:*
      `a_surface_without_reflectance_is_refused_by_name`, `a_non_unit_normal_is_refused_by_name`.
- [x] T004 [US1] Implement the four Source 2 terms over the reduced OpenPBR set —
      `src/presentation.rs` · *check:* `the_fma_bar_measures_at_least_three_bands` prints its count.
- [x] T005 [US3] Pin purity and run-order independence — `src/presentation.rs` · *check:*
      `shading_is_a_pure_function_of_its_inputs`.
- [x] T006 Entry point: print the registry and the measurement; exit non-zero on a failed bar —
      `src/main.rs` · *check:* `cargo run` prints the count and exits 0.
- [x] T007 Point agent context at the constitution and this spec (agent-forge shape) —
      `.github/copilot-instructions.md`, `.github/instructions/clean-room.instructions.md` ·
      *check:* both files name the constitution and this spec.
- [ ] T008 Complete the reduced OpenPBR parameter set and name the remainder absent — still in scope
      for U1 · *check:* a test enumerates the absent parameters, as the legacy `openpbr::tests` does.
- [x] T009a [US2] **The visible frame** — the half of T009 this pass took: a **real** window, the world's
      own cells drawn through the contract's own one `shade` call, one tick of the plane's own law per frame,
      the window's own pixels read back and compared, and the frame written as a P6 screenshot — never
      `nodraw` (C-40) · *check:* `the_presented_frame_reaches_the_window_s_own_pixels` (every pixel of the
      client area, every frame) plus the band count on the frame's own pixels, and the screenshot at
      `docs/baselines/u1-visible-client/frame.ppm` — plan §36.
- [ ] T009b [US2] **The GPU pass** — T009's other half, owed **by name**: a shader checked against
      `presentation::shade` on the same surface · *check:* the GPU comparison. **Held behind `PATCH-040`'s
      audit**: the frame `src/surface.rs` draws is a **column** picture while the record's own coordinates are
      cubic (`32³` chunks, three axes), so *which* picture the shader is checked against is the audit's ruling
      rather than this task's assumption.
- [ ] T010 [P] Sky: radiance from the atmospheric state, so the surface is lit by a modelled sky and
      not a constant — *check:* a daylight test at a declared sun altitude. *(2026-10-13, §36: the client's
      light **is** the solver's own Sun at a declared altitude (`surface::SUN_ALTITUDE_DEG = 35.0`, its own
      transmitted colour printed as the tint `(1.000, 0.925, 0.733)`), so nothing in the frame is a constant —
      but the **sky's own radiance** is not yet a *term* in it: the contract's `Light` carries a direction and
      a tint, so what lights a surface is the Sun and not the sky. Owed by name.)*
- [ ] T011 **Constitution task:** re-run the checks behind every P1–P7 row of `plan.md` §3 and record
      the output. A task is not complete unless its check was run.
