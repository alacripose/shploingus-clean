//! Fake wrapping: a **shading normal** bent around a face's recess pattern.
//!
//! # The problem this solves
//!
//! A `pillar` is a material shaper (`voxel-shapers::voxel`). It cannot cut the
//! block, so a pillar-shaped block is a **cube**. Drawn with a flat normal it looks
//! like a cube, and no amount of material subtlety fixes that: a cube's faces are
//! flat because they *are* flat.
//!
//! The fix is 48 years old and is not a trick. **Blinn (1978), *Simulation of
//! Wrinkled Surfaces*, SIGGRAPH '78** — the paper that introduced bump mapping —
//! states the licence: *a surface's shading normal may be perturbed independently of
//! its geometry*. The geometry stays a cube, the mesh stays one quad per face, and
//! the *normal each fragment is shaded with* follows the shape the material pattern
//! describes. That is what "fake wrapping" means here, and it is why a material
//! shaper is allowed to affect normals at all.
//!
//! # Where the law lives, and why it is not here
//!
//! The round is **not** computed in this module. `voxel-model::profile` owns the
//! shape laws: [`Profile::cap_normal`] is the exact outward normal of a cap, and a
//! pillar's shaft *is* a [`Profile::RoundColumn`]'s cap — the surface of revolution
//! `h(r) = √(R² − r²)` with `R` the circle inscribed in the block's square. There is
//! exactly **one statement** of that law in the workspace and it is `Profile`'s;
//! this module used to carry a second copy, which is how two sources of truth start.
//!
//! What is left here is the *policy* around that law, and each piece is a decision
//! rather than arithmetic:
//!
//! * **when a face wraps at all** — never, over a uniform pattern, because bending a
//!   plane's normal would make flat rock look curved and that error would be
//!   everywhere ([`is_flat`], the gate);
//! * **the recess pattern's slot order** — `u` outside and `v` inside, so slot
//!   `4·u + v`, which is [`voxel_model::VoxelShape::face_cells`]'s order and must not
//!   disagree with it ([`recess_at`]);
//! * **how the 256-level lattice gradient tilts an already-wrapped normal**
//!   ([`wrapped_normal_with_detail`]) — the lattice *varies* a surface, it does not
//!   supply one, which is the same rule `crate::openpbr::with_detail` follows for
//!   colour and for the same reason: a lattice that could supply a height would be a
//!   displacement map, which is storage, which is the thing C-2 forbids.
//!
//! # No bend-strength constant
//!
//! There is none, and that is deliberate: a number like "0.35 roundness" would be a
//! value with no source, which is the one thing this repository refuses. The only
//! inputs are the face's own normal and the fragment's position within the face.

use voxel_model::profile::Profile;
use voxel_model::{Face, Refusal, RefusalKind, Refused, SUBVOXEL_COUNT, SUBVOXEL_SIDE};

/// The radius of the circle inscribed in the unit square, and therefore of a pillar's
/// shaft. Half the block, because that is what *inscribed* means — it is
/// [`voxel_model::MAX_RADIUS`] and not a second copy of the same number.
pub const INSCRIBED_RADIUS: f64 = voxel_model::MAX_RADIUS;

/// **The profile a pillar's shaft is**: a round column inscribed in the block.
///
/// Named here so the pattern's law can be asked for by name, rather than
/// reconstructed from a radius at each call site.
pub const PILLAR: Profile = Profile::RoundColumn { radius: INSCRIBED_RADIUS };

/// Cells in one face, which is also the length of a `face_recess`.
pub const FACE_CELLS: usize = SUBVOXEL_COUNT / 4;

/// A pillar's surface height above the face plane, at radius `r` — the profile's own
/// cap law, so this is the pillar and not an approximation of one.
pub fn pillar_height(r: f64) -> f64 {
    PILLAR.cap_height(r)
}

/// The outward normal of the shape the pattern describes, as a tangency:
/// `(axial, radial)` in the face's own basis.
///
/// The profile's law again: `1.0` axial is a normal lying straight along the face
/// normal, and a positive radial component is one that has turned towards the rim.
/// At `r >= R` the pillar's surface has become its **side** and the normal is purely
/// radial — the exact limit, not a clamp.
pub fn pillar_tangency(r: f64) -> (f64, f64) {
    PILLAR.cap_tangency(r)
}

/// True when a face's pattern has nothing to wrap around.
pub fn is_flat(recess: &[bool; FACE_CELLS]) -> bool {
    !recess.iter().any(|r| *r)
}

/// **The fake wrap.** Bend `face`'s normal for a fragment at `(u, v)` within the face,
/// given the face's recess pattern.
///
/// `u` and `v` are in `[0, 1]` across the face, in the face's own basis. They are
/// **refused** rather than clamped when they fall outside, for the same reason every
/// other range in this repository is: a clamped coordinate silently answers a
/// question nobody asked, and here it would answer it with a normal.
///
/// A face whose pattern is uniform — the overwhelming majority of a world — comes
/// back with its own normal, unchanged. That gate is this module's, and the bend
/// inside it is [`Profile::cap_normal`]'s.
pub fn wrapped_normal(
    face: Face,
    recess: &[bool; FACE_CELLS],
    u: f64,
    v: f64,
) -> Refused<[f64; 3]> {
    if is_flat(recess) {
        // Still validated: a flat face refuses a bad coordinate exactly like a
        // recessed one, so the gate cannot be used to smuggle a bad query through.
        if !(0.0..=1.0).contains(&u) || !u.is_finite() || !(0.0..=1.0).contains(&v) || !v.is_finite()
        {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("face coordinate ({u}, {v}) outside [0, 1]"),
            ));
        }
        return Ok(voxel_model::face_vector(face));
    }
    PILLAR.cap_normal(face, u, v)
}

/// The recess pattern's `(u, v)` slot, matching
/// [`voxel_model::VoxelShape::face_recess`]'s order: `u` on the outside, `v` inside,
/// so the slot of a cell centre is `4·u + v`.
pub fn recess_at(recess: &[bool; FACE_CELLS], u: usize, v: usize) -> bool {
    recess[SUBVOXEL_SIDE as usize * u + v]
}

/// **Where the two resolutions meet.** The coarse bend of [`wrapped_normal`], tilted a
/// little further by a **gradient** sampled from the addressed 256× lattice.
///
/// `gradient` is the lattice's own height gradient in the face's `(u, v)` basis, in
/// units of a subvoxel per unit of face. It is added to the profile's tangency rather
/// than replacing it: the lattice *varies* a surface, it does not supply one. A
/// lattice that could supply a height would be a displacement map, which is storage,
/// which is the thing C-2 forbids.
pub fn wrapped_normal_with_detail(
    face: Face,
    recess: &[bool; FACE_CELLS],
    u: f64,
    v: f64,
    gradient: [f64; 2],
) -> Refused<[f64; 3]> {
    let base = wrapped_normal(face, recess, u, v)?;
    if is_flat(recess) || (gradient[0] == 0.0 && gradient[1] == 0.0) {
        return Ok(base);
    }
    if !gradient[0].is_finite() || !gradient[1].is_finite() {
        return Err(Refusal::owned(
            RefusalKind::NonFiniteValue,
            format!("a non-finite detail gradient ({}, {})", gradient[0], gradient[1]),
        ));
    }
    let (u_axis, v_axis) = voxel_model::in_plane_axes(face);
    let out = [
        base[0] - gradient[0] * u_axis[0] - gradient[1] * v_axis[0],
        base[1] - gradient[0] * u_axis[1] - gradient[1] * v_axis[1],
        base[2] - gradient[0] * u_axis[2] - gradient[1] * v_axis[2],
    ];
    let length = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
    if length == 0.0 || !length.is_finite() {
        return Err(Refusal::owned(
            RefusalKind::NonFiniteValue,
            format!("the detailed normal degenerated at ({u}, {v})"),
        ));
    }
    Ok([out[0] / length, out[1] / length, out[2] / length])
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::{MaterialId, SubvoxelIndex, VoxelShape};

    const SHEATH: MaterialId = MaterialId(6);
    const INSET: MaterialId = MaterialId(5);

    /// The pattern a `pillar` shaper writes: the block's **corner columns** — the
    /// sixteen cells whose centres lie outside the inscribed circle.
    fn pillar_shape() -> VoxelShape {
        let mut shape = VoxelShape::solid(SHEATH);
        for at in SubvoxelIndex::all() {
            let extreme = |c: u8| c == 0 || c == 3;
            if extreme(at.x) && extreme(at.y) {
                shape.set(at, INSET);
            }
        }
        shape
    }

    fn pillar_recess(face: Face) -> [bool; FACE_CELLS] {
        pillar_shape().face_recess(face)
    }

    #[test]
    fn the_inscribed_radius_is_half_a_block_because_that_is_what_inscribed_means() {
        assert_eq!(INSCRIBED_RADIUS, 0.5);
        assert_eq!(PILLAR, Profile::RoundColumn { radius: 0.5 });
        assert_eq!(PILLAR.max_radius(), INSCRIBED_RADIUS);
        // The corner of the square is at √2/2 from the centre, so it is
        // `√2/2 − 1/2` outside the circle. Nothing here chooses that number; it is the
        // geometry of a circle in a square, and it is the whole of the "roundness" this
        // module provides.
        let corner = (0.5f64 * 0.5 + 0.5 * 0.5).sqrt();
        let sagitta = corner - INSCRIBED_RADIUS;
        assert!((sagitta - (2.0f64.sqrt() - 1.0) / 2.0).abs() < 1e-12);
        assert!(sagitta > 0.2 && sagitta < 0.21);
    }

    #[test]
    fn the_law_here_is_the_profiles_law_and_not_a_second_copy() {
        // Pinned against the profile directly: if either statement of the round-column
        // law changes, this fails, which is the point of there being one statement.
        for step in 0..=16 {
            let r = step as f64 / 32.0;
            assert_eq!(pillar_height(r), PILLAR.cap_height(r));
            assert_eq!(pillar_tangency(r), PILLAR.cap_tangency(r));
        }
    }

    #[test]
    fn a_flat_face_comes_back_unchanged_bit_for_bit() {
        // The gate that keeps flat rock flat. A uniform face has no recess, and a
        // normal bent over a plane would make every flat surface in the world look
        // curved — which is a worse error than a flat pillar, because it is everywhere.
        let flat = [false; FACE_CELLS];
        for face in Face::ALL {
            let n = voxel_model::face_vector(face);
            for (u, v) in [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (0.13, 0.87)] {
                assert_eq!(wrapped_normal(face, &flat, u, v).unwrap(), n, "{face:?}");
            }
            // And a bad coordinate is refused even when there is nothing to wrap.
            assert!(wrapped_normal(face, &flat, 1.5, 0.5).is_err(), "{face:?}");
        }
        assert!(is_flat(&flat));
        assert!(!is_flat(&pillar_recess(Face::PosZ)));
    }

    #[test]
    fn the_centre_of_a_pillar_face_looks_straight_out() {
        // At r = 0 the cylinder's normal is its axis, so the face's centre is shaded
        // exactly as a flat face would be. A pillar is round at the edges and flat in
        // the middle, which is what a cylinder is.
        for face in Face::ALL {
            let n = wrapped_normal(face, &pillar_recess(face), 0.5, 0.5).unwrap();
            assert_eq!(n, voxel_model::face_vector(face), "{face:?}");
        }
    }

    #[test]
    fn the_rim_of_a_pillar_face_turns_to_look_sideways() {
        // At r >= R the pillar's surface *is* its side, so the normal is radial: no
        // axial component at all. This is the limit of the real cylinder and not a
        // clamp, so it is asserted as an equality rather than a bound.
        for face in Face::ALL {
            let n = wrapped_normal(face, &pillar_recess(face), 1.0, 0.5).unwrap();
            let axis = face.axis() as usize;
            assert!(n[axis].abs() < 1e-12, "{face:?} still points along its axis");
            // And it points outward, away from the face's centre — towards the rim.
            let (u_axis, _) = voxel_model::in_plane_axes(face);
            let along = n[0] * u_axis[0] + n[1] * u_axis[1] + n[2] * u_axis[2];
            assert!(along > 0.0, "{face:?} bent the wrong way: {n:?}");
        }
    }

    #[test]
    fn the_bend_grows_monotonically_from_the_centre_to_the_rim() {
        // "Round" is not a vibe: it is a normal whose tilt rises with radius. A module
        // that returned a constant tilt would pass "the rim turns" and be quite wrong.
        let recess = pillar_recess(Face::PosZ);
        let mut last_axial = f64::INFINITY;
        let mut last_radial = -1.0;
        for step in 0..=10 {
            let r = 0.05 * step as f64;
            let n = wrapped_normal(Face::PosZ, &recess, 0.5 + r, 0.5).unwrap();
            let axial = n[2].abs();
            let radial = (n[0] * n[0] + n[1] * n[1]).sqrt();
            assert!(axial <= last_axial + 1e-12, "axial rose at r = {r}");
            assert!(radial >= last_radial - 1e-12, "radial fell at r = {r}");
            last_axial = axial;
            last_radial = radial;
        }
        // And at the far end the tilt is large — the surface has turned most of the way
        // to sideways, which is what makes it read as a pillar.
        let edge = wrapped_normal(Face::PosZ, &recess, 0.5 + INSCRIBED_RADIUS, 0.5).unwrap();
        assert!(edge[2].abs() < 1e-12);
        assert!((edge[0] * edge[0] + edge[1] * edge[1]).sqrt() > 0.999);
    }

    #[test]
    fn every_normal_is_a_unit_vector_and_finite_everywhere_on_the_face() {
        // The normal is normalised, so the unbounded `h'` as r → R cannot leak a
        // non-finite value into a frame. Checked on a grid that walks straight through
        // the rim.
        for face in Face::ALL {
            let recess = pillar_recess(face);
            for i in 0..=20 {
                for j in 0..=20 {
                    let (u, v) = (i as f64 / 20.0, j as f64 / 20.0);
                    let n = wrapped_normal(face, &recess, u, v).unwrap();
                    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                    assert!((length - 1.0).abs() < 1e-12, "{face:?} ({u}, {v}) length {length}");
                    assert!(n.iter().all(|c| c.is_finite()), "{face:?} ({u}, {v}) {n:?}");
                }
            }
        }
    }

    #[test]
    fn the_bend_is_symmetric_in_u_and_v_which_a_cylinder_has_to_be() {
        // A pillar does not know about x and y. If the bend were not symmetric the
        // pattern would be face-consistent and the *shading* would not be, which is the
        // kind of error that looks like a lighting bug for a week.
        let recess = pillar_recess(Face::PosZ);
        for (u, v) in [(0.1, 0.7), (0.7, 0.1), (0.25, 0.9), (0.9, 0.25), (0.4, 0.55)] {
            let a = wrapped_normal(Face::PosZ, &recess, u, v).unwrap();
            let b = wrapped_normal(Face::PosZ, &recess, v, u).unwrap();
            assert!((a[2] - b[2]).abs() < 1e-12, "({u}, {v})");
            let ra = (a[0] * a[0] + a[1] * a[1]).sqrt();
            let rb = (b[0] * b[0] + b[1] * b[1]).sqrt();
            assert!((ra - rb).abs() < 1e-12, "({u}, {v})");
        }
    }

    #[test]
    fn a_face_coordinate_outside_the_face_is_refused_rather_than_clamped() {
        let recess = pillar_recess(Face::PosZ);
        assert!(wrapped_normal(Face::PosZ, &recess, 1.5, 0.5).is_err());
        assert!(wrapped_normal(Face::PosZ, &recess, 0.5, -0.001).is_err());
        assert!(wrapped_normal(Face::PosZ, &recess, f64::NAN, 0.5).is_err());
        // The exact edges are inside, because a face coordinate of 1.0 is the rim.
        assert!(wrapped_normal(Face::PosZ, &recess, 1.0, 1.0).is_ok());
    }

    #[test]
    fn the_recess_slot_order_matches_the_models_face_cells_order() {
        // `face_recess` walks u on the outside and v inside, so slot = 4u + v. Pinned
        // against a shape whose recessed set is not symmetric, so a transposed index
        // fails instead of passing by luck.
        let mut shape = VoxelShape::solid(SHEATH);
        for at in SubvoxelIndex::all() {
            if at.z == 3 && at.x == 0 {
                shape.set(at, INSET);
            }
        }
        let recess = shape.face_recess(Face::PosZ);
        // The minority is the u = 0 column, so slots 0, 4, 8, 12.
        for u in 0..4usize {
            for v in 0..4usize {
                assert_eq!(recess_at(&recess, u, v), u == 0, "slot ({u}, {v}) read wrongly");
            }
        }
    }

    #[test]
    fn the_lattice_gradient_tilts_an_already_wrapped_face_further() {
        let recess = pillar_recess(Face::PosZ);
        let base = wrapped_normal(Face::PosZ, &recess, 0.5, 0.5).unwrap();
        // A gradient with no magnitude is a no-op — the lattice varies a surface, it
        // does not supply one.
        assert_eq!(
            wrapped_normal_with_detail(Face::PosZ, &recess, 0.5, 0.5, [0.0, 0.0]).unwrap(),
            base
        );
        let tilted =
            wrapped_normal_with_detail(Face::PosZ, &recess, 0.5, 0.5, [0.25, 0.0]).unwrap();
        assert_ne!(tilted, base);
        let length = (tilted[0] * tilted[0] + tilted[1] * tilted[1] + tilted[2] * tilted[2]).sqrt();
        assert!((length - 1.0).abs() < 1e-12);
        // It tilted in −u, because the normal is `base − ∇h` and ∇h is +u.
        assert!(tilted[0] < base[0]);

        // And a flat face stays flat even with a gradient: the gate is the pattern, and
        // a physical displacement on flat rock is not this module's business.
        let flat = [false; FACE_CELLS];
        assert_eq!(
            wrapped_normal_with_detail(Face::PosZ, &flat, 0.5, 0.5, [0.9, 0.9]).unwrap(),
            voxel_model::face_vector(Face::PosZ)
        );
        // A non-finite gradient is refused, not absorbed.
        assert!(
            wrapped_normal_with_detail(Face::PosZ, &recess, 0.5, 0.5, [f64::NAN, 0.0]).is_err()
        );
    }

    #[test]
    fn a_cap_s_recess_is_the_profiles_own_outward_pattern() {
        // The two halves live in different crates, so the pattern is the interface —
        // and a **cap** is where the interface is exact: the cells a pillar re-slots
        // on a cap are precisely the cells `Profile::outward_cells` calls outside, in
        // the same slot order. Asserted against the profile's own sampler rather than
        // a hand-written bitmap, so the two statements cannot drift apart.
        for face in [Face::PosZ, Face::NegZ] {
            let recess = pillar_recess(face);
            assert_eq!(recess.to_vec(), PILLAR.outward_cells(4, INSCRIBED_RADIUS), "{face:?}");
            assert_eq!(recess.iter().filter(|r| **r).count(), 4, "{face:?}");
            // And those four are the corners of the square, which is the law's shape
            // rather than a stored pattern.
            for (u, v) in [(0usize, 0usize), (3, 0), (0, 3), (3, 3)] {
                assert!(recess_at(&recess, u, v), "{face:?} corner ({u}, {v})");
            }
        }
    }

    #[test]
    fn a_side_s_recess_is_half_the_face_and_the_tie_is_named() {
        // The finding. A cylinder's matter outside the body at a **side** face is
        // outside it for every `z` once `y` is at an end, so the pattern at a side is
        // two whole rows — exactly **half** the face. `face_recess` asks which material
        // covers "most" of the face, and half is not most: `face_blend` breaks the tie
        // by material id (`voxel_model`), the lower id becomes the majority, and the
        // recess is the *other* half. The result is deterministic and it is named here
        // rather than discovered later.
        //
        // Which half of a tied face should bend is a question for `docs/APPEARANCE.md`,
        // which is the user's; this module's gate only needs to know that the face is
        // not uniform.
        let insetted_is_lower = INSET.0 < SHEATH.0;
        for face in [Face::NegX, Face::PosX, Face::NegY, Face::PosY] {
            let recess = pillar_recess(face);
            assert_eq!(recess.iter().filter(|r| **r).count(), 8, "{face:?}");
            assert!(!is_flat(&recess), "{face:?} would not wrap at all");
            for u in 0..4usize {
                for v in 0..4usize {
                    // The shaper insets the columns `u = 0` and `u = 3`; the recess is
                    // whatever material is *not* the majority, so on a tied face it is
                    // the complement of the insets when the inset id is the lower one.
                    let insetted = u == 0 || u == 3;
                    let recessed = insetted != insetted_is_lower;
                    assert_eq!(recess_at(&recess, u, v), recessed, "{face:?} ({u}, {v})");
                }
            }
        }
    }
}
