//! Deterministic hashing.
//!
//! Noise needs randomness that is a **pure function of position and seed**, not
//! a stream. A stream cannot be sampled out of order, which is fatal for a world
//! generator that evaluates a chunk in any thread in any order (F12: worker count
//! is a performance knob and never changes output).
//!
//! So there is no RNG state here at all — only a hash. `splitmix64`'s finaliser
//! (Steele, Lea & Flood, 2014) is used because it is small, well-mixed and
//! stateless, and because every platform agrees on integer multiplication.

/// The splitmix64 finaliser: a bijection on `u64` with good avalanche.
#[inline]
pub const fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hash a seed and two coordinates.
#[inline]
pub fn hash2(seed: u64, x: i64, y: i64) -> u64 {
    mix(mix(seed ^ 0x1234_5678_9ABC_DEF0) ^ (x as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93) ^ (y as u64).wrapping_mul(0xA076_1D64_78BD_642F))
}

/// Hash a seed and three coordinates.
#[inline]
pub fn hash3(seed: u64, x: i64, y: i64, z: i64) -> u64 {
    mix(
        mix(mix(seed) ^ (x as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93))
            ^ (y as u64).wrapping_mul(0xA076_1D64_78BD_642F)
            ^ (z as u64).wrapping_mul(0xE703_7ED1_A0B4_28DB),
    )
}

/// A hashed coordinate as a value in `[0, 1)`.
#[inline]
pub fn unit2(seed: u64, x: i64, y: i64) -> f64 {
    // 53 bits is exactly the f64 mantissa: the mapping is uniform enough that no
    // two adjacent hashes collapse, which a 32-bit scale would allow.
    (hash2(seed, x, y) >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}

/// A hashed coordinate as a value in `[0, 1)`.
#[inline]
pub fn unit3(seed: u64, x: i64, y: i64, z: i64) -> f64 {
    (hash3(seed, x, y, z) >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}

/// A hashed coordinate as a value in `[-1, 1)`.
#[inline]
pub fn signed3(seed: u64, x: i64, y: i64, z: i64) -> f64 {
    unit3(seed, x, y, z) * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashing_is_a_pure_function_of_its_inputs() {
        assert_eq!(hash3(7, 1, 2, 3), hash3(7, 1, 2, 3));
        assert_eq!(unit3(7, 1, 2, 3), unit3(7, 1, 2, 3));
    }

    #[test]
    fn changing_any_component_changes_the_hash() {
        let base = hash3(7, 1, 2, 3);
        assert_ne!(base, hash3(8, 1, 2, 3));
        assert_ne!(base, hash3(7, 9, 2, 3));
        assert_ne!(base, hash3(7, 1, 9, 3));
        assert_ne!(base, hash3(7, 1, 2, 9));
    }

    #[test]
    fn hashes_are_well_distributed_over_a_small_box() {
        // A hash that clusters would show up as a biased noise field. Sample a
        // 64³ box and check the mean is near 0.5 and both halves are populated.
        let n = 64i64;
        let mut sum = 0.0;
        let mut low = 0usize;
        let mut count = 0usize;
        for x in 0..n {
            for y in 0..n {
                for z in 0..n {
                    let v = unit3(42, x, y, z);
                    assert!((0.0..1.0).contains(&v));
                    sum += v;
                    if v < 0.5 {
                        low += 1;
                    }
                    count += 1;
                }
            }
        }
        let mean = sum / count as f64;
        assert!((mean - 0.5).abs() < 0.01, "mean was {mean}");
        let frac_low = low as f64 / count as f64;
        assert!((0.45..0.55).contains(&frac_low), "low fraction was {frac_low}");
    }

    #[test]
    fn negative_coordinates_hash_as_their_own_values() {
        // A generator that folds -1 onto 0 mirrors the world about the origin.
        assert_ne!(hash3(1, -1, 0, 0), hash3(1, 0, 0, 0));
    }

    #[test]
    fn signed_values_are_in_range() {
        for i in -40i64..40 {
            let v = signed3(3, i, i * 3, i * 7);
            assert!((-1.0..1.0).contains(&v));
        }
    }
}
