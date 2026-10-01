//! Transition — where two domains meet.
//!
//! **Source.** Shepard, D. *A Two-Dimensional Interpolation Function for
//! Irregularly-Spaced Data.* Proceedings of the 1968 ACM National Conference —
//! **inverse-distance weighting**: a sample's influence falls off with distance
//! and the weights are normalised, so the result is a **partition of unity**.
//!
//! # The difference between a *continuous* field and a *class* field
//!
//! This is the module where that distinction has teeth, and it is the same
//! distinction [`crate::fields`] makes about interpolation:
//!
//! * a **continuous** quantity (soil depth, temperature, elevation) blends
//!   smoothly, and [`blend_scalar`] gives a weighted mean;
//! * a **class** (a rock, a biome) **does not blend**. There is no rock that is
//!   40% granite and 60% shale, so [`blend_class`] **refuses** at a tie rather
//!   than inventing a material. It picks a side when the balance is clear and
//!   refuses when it is not, which is the only honest reading of "halfway between
//!   two biomes".
//!
//! A boundary between two domains is therefore a **sharp** edge in the world, not
//! a blur, and that is a modelling decision this module states rather than one it
//! lets fall out of a float comparison.

use voxel_model::{Refusal, RefusalKind, Refused};

/// The Shepard power. `2` is the standard inverse-square falloff.
pub const SHEPARD_POWER: u32 = 2;

/// The band around a 50/50 split in which a class blend is **ambiguous**.
///
/// Declared, and small on purpose: it exists so that a floating-point tie is
/// refused rather than resolved by whichever comparison happens to come first.
pub const AMBIGUITY_BAND: f64 = 0.02;

/// Normalised inverse-distance weights, in input order. Always sums to exactly 1
/// up to floating-point rounding of the division.
///
/// Distances must be finite and non-negative. A **zero** distance means the
/// sample *is* the query point and takes all the weight — which is not a special
/// case in the arithmetic (the other weights are 0), but is worth naming because
/// a naive `1/d` produces an infinity there.
pub fn shepard_weights(distances: &[f64]) -> Refused<Vec<f64>> {
    if distances.is_empty() {
        return Err(Refusal::new(
            RefusalKind::OutOfDeclaredRange,
            "no samples to weight",
        ));
    }
    for (i, d) in distances.iter().enumerate() {
        if !d.is_finite() || *d < 0.0 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("sample {i} has distance {d}; distances must be finite and non-negative"),
            ));
        }
    }

    // An exact hit wins outright.
    if let Some(i) = distances.iter().position(|d| *d == 0.0) {
        let mut w = vec![0.0; distances.len()];
        w[i] = 1.0;
        return Ok(w);
    }

    let raw: Vec<f64> = distances
        .iter()
        .map(|d| 1.0 / d.powi(SHEPARD_POWER as i32))
        .collect();
    let total: f64 = raw.iter().sum();

    if total <= 0.0 || !total.is_finite() {
        return Err(Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            format!("inverse-distance weights do not normalise (sum {total})"),
        ));
    }

    Ok(raw.into_iter().map(|r| r / total).collect())
}

/// A weighted mean of a continuous quantity. `samples` are `(value, distance)`.
pub fn blend_scalar(samples: &[(f64, f64)]) -> Refused<f64> {
    if samples.is_empty() {
        return Err(Refusal::new(
            RefusalKind::OutOfDeclaredRange,
            "no samples to blend",
        ));
    }
    for (i, (v, _)) in samples.iter().enumerate() {
        if !v.is_finite() {
            return Err(Refusal::owned(
                RefusalKind::NonFiniteValue,
                format!("sample {i} has value {v}"),
            ));
        }
    }
    let distances: Vec<f64> = samples.iter().map(|(_, d)| *d).collect();
    let weights = shepard_weights(&distances)?;
    Ok(samples
        .iter()
        .zip(weights.iter())
        .map(|((v, _), w)| v * w)
        .sum())
}

/// A class blend: pick the dominant class, or refuse if the split is ambiguous.
///
/// `weights` are the classes' shares, in `[0, 1]`. Two classes within
/// [`AMBIGUITY_BAND`] of each other at the top are a refusal — because the answer
/// the caller wants does not exist.
pub fn blend_class<T: Copy + PartialEq + core::fmt::Debug>(
    classes: &[(T, f64)],
) -> Refused<T> {
    if classes.is_empty() {
        return Err(Refusal::new(
            RefusalKind::OutOfDeclaredRange,
            "no classes to blend",
        ));
    }
    for (i, (_, w)) in classes.iter().enumerate() {
        if !w.is_finite() || *w < 0.0 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("class {i} has weight {w}"),
            ));
        }
    }

    // Sort by weight descending, ties broken by the **input order**, so the
    // result is deterministic and does not depend on how the caller built the slice.
    let mut order: Vec<usize> = (0..classes.len()).collect();
    order.sort_by(|a, b| {
        classes[*b]
            .1
            .partial_cmp(&classes[*a].1)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.cmp(b))
    });

    let (best, best_w) = (order[0], classes[order[0]].1);
    if order.len() > 1 {
        let second = order[1];
        let second_w = classes[second].1;
        if classes[best].0 != classes[second].0 && (best_w - second_w).abs() < AMBIGUITY_BAND {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "class blend is ambiguous: {:?} ({best_w:.4}) vs {:?} ({second_w:.4})",
                    classes[best].0, classes[second].0
                ),
            ));
        }
    }
    Ok(classes[best].0)
}

/// A two-way class transition by a fraction `t`, where `t = 0` is all `a`.
///
/// A convenience over [`blend_class`] that makes the tie explicit: `t = 0.5`
/// between two *different* classes is a refusal, always.
pub fn cross_class<T: Copy + PartialEq + core::fmt::Debug>(
    a: T,
    b: T,
    t: f64,
) -> Refused<T> {
    if !t.is_finite() || !(0.0..=1.0).contains(&t) {
        return Err(Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            format!("transition fraction {t} outside [0, 1]"),
        ));
    }
    blend_class(&[(a, 1.0 - t), (b, t)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geology::Lithology;

    #[test]
    fn weights_are_a_partition_of_unity() {
        let w = shepard_weights(&[1.0, 2.0, 3.0, 10.0]).unwrap();
        assert_eq!(w.len(), 4);
        let sum: f64 = w.iter().sum();
        assert!((sum - 1.0).abs() < 1e-12, "weights summed to {sum}");
        // Closer samples weigh more.
        assert!(w[0] > w[1] && w[1] > w[2] && w[2] > w[3]);
    }

    #[test]
    fn an_exact_hit_takes_all_the_weight_without_an_infinity() {
        let w = shepard_weights(&[3.0, 0.0, 5.0]).unwrap();
        assert_eq!(w, vec![0.0, 1.0, 0.0]);
    }

    #[test]
    fn bad_distances_are_refused_rather_than_absolute_valued() {
        assert_eq!(
            shepard_weights(&[-1.0, 2.0]).unwrap_err().kind(),
            RefusalKind::OutOfDeclaredRange
        );
        assert_eq!(
            shepard_weights(&[f64::NAN]).unwrap_err().kind(),
            RefusalKind::OutOfDeclaredRange
        );
        assert_eq!(
            shepard_weights(&[f64::INFINITY]).unwrap_err().kind(),
            RefusalKind::OutOfDeclaredRange
        );
        assert!(shepard_weights(&[]).is_err());
    }

    #[test]
    fn a_continuous_blend_is_a_weighted_mean() {
        // Two samples equidistant: the mean, exactly.
        let v = blend_scalar(&[(10.0, 1.0), (20.0, 1.0)]).unwrap();
        assert!((v - 15.0).abs() < 1e-12);
        // The nearer one dominates.
        let v = blend_scalar(&[(10.0, 1.0), (20.0, 10.0)]).unwrap();
        assert!(v < 11.0, "the far sample dominated: {v}");
    }

    #[test]
    fn a_class_blend_refuses_a_tie_instead_of_inventing_a_rock() {
        // The whole point of this module. There is no 50/50 granite-shale.
        let r = cross_class(Lithology::Igneous, Lithology::Sedimentary, 0.5).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(r.detail().contains("ambiguous"), "reason was {}", r.detail());
    }

    #[test]
    fn a_class_blend_picks_a_side_when_the_balance_is_clear() {
        assert_eq!(
            cross_class(Lithology::Igneous, Lithology::Sedimentary, 0.1).unwrap(),
            Lithology::Igneous
        );
        assert_eq!(
            cross_class(Lithology::Igneous, Lithology::Sedimentary, 0.9).unwrap(),
            Lithology::Sedimentary
        );
        // Equal classes are never ambiguous, whatever the fraction.
        assert_eq!(
            cross_class(Lithology::Igneous, Lithology::Igneous, 0.5).unwrap(),
            Lithology::Igneous
        );
    }

    #[test]
    fn an_out_of_range_fraction_is_refused() {
        assert!(cross_class(Lithology::Igneous, Lithology::Sedimentary, 1.5).is_err());
        assert!(cross_class(Lithology::Igneous, Lithology::Sedimentary, -0.1).is_err());
        assert!(cross_class(Lithology::Igneous, Lithology::Sedimentary, f64::NAN).is_err());
        // The boundaries themselves are valid.
        assert!(cross_class(Lithology::Igneous, Lithology::Sedimentary, 0.0).is_ok());
        assert!(cross_class(Lithology::Igneous, Lithology::Sedimentary, 1.0).is_ok());
    }

    #[test]
    fn blending_is_deterministic_regardless_of_input_order() {
        let a = blend_class(&[(Lithology::Igneous, 0.7), (Lithology::Bedrock, 0.3)]).unwrap();
        let b = blend_class(&[(Lithology::Bedrock, 0.3), (Lithology::Igneous, 0.7)]).unwrap();
        assert_eq!(a, b);
    }
}
