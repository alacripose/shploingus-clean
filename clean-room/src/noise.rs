//! **Noise primitives — each one a paper, not a vibe.**
//!
//! Derived from `docs/SHAPERS.md` §0 (the primitive table: every shaper is built from these, and
//! each is a paper) and §2 (the determinism contract every shaper obeys: a pure function of
//! `(seed, tick, declared period)`, no wall clock, no thread order, no RNG state).
//!
//! | Primitive | Paper |
//! |---|---|
//! | gradient noise | Perlin, *An Image Synthesizer*, SIGGRAPH 1985; *Improving Noise*, SIGGRAPH 2002 |
//! | coherent hash | the splitmix64 finaliser (Steele et al. 2014) — **the same mixer** `world::mix` uses |
//! | cellular noise | Worley, *A Cellular Texture Basis Function*, SIGGRAPH 1996 |
//! | fBm | Mandelbrot & Van Ness, *Fractional Brownian Motions, Fractional Noises and Applications*, SIAM Review 1968 |
//! | ridged multifractal | Musgrave, Kolb & Mace, *The Synthesis and Rendering of Eroded Fractal Terrains*, SIGGRAPH 1989 |
//! | domain warping | Perlin 1985 §; Quilez, *Domain warping* (iquilezles.org, 2002) |
//!
//! # Why the gradients are eight declared directions and not `cos`/`sin`
//!
//! A gradient noise that derives its gradient angle from `f64::cos` is deterministic on *this*
//! machine and not guaranteed to be on another: libm's trig is not required to be correctly rounded.
//! Eight declared direction vectors are exact constants, so two machines agree **byte for byte** —
//! which is what P3 asks of the whole world, not just of this file.
//!
//! A **stream** is the sub-stream a primitive is drawn from. Two stages sampling different physics
//! take different streams and therefore never share one noise field by accident; the closed set of
//! streams is U2's [`crate::world::Domain`], and the tick enters through the caller's seed.

use crate::world::mix;

/// `1 / sqrt(2)`, exactly as declared (the diagonal directions' component).
const SQRT_HALF: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// The eight declared gradient directions. Exact constants; no trigonometry at sample time.
const DIRECTIONS: [(f64, f64); 8] = [
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (SQRT_HALF, SQRT_HALF),
    (-SQRT_HALF, SQRT_HALF),
    (SQRT_HALF, -SQRT_HALF),
    (-SQRT_HALF, -SQRT_HALF),
];

/// The nearest feature point a [`Noise::cellular`] sample found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cellular {
    /// Distance to the nearest feature point (Worley's `F1`).
    pub f1: f64,
    /// Distance to the second nearest (Worley's `F2`); `f1 <= f2`.
    pub f2: f64,
    /// The lattice cell that owns `f1` — the **plate**, the **biome seed**, the named region.
    pub cell: (i64, i64),
}

/// A seeded noise field. Pure: two calls with the same arguments are the same number.
#[derive(Clone, Copy, Debug)]
pub struct Noise {
    seed: u64,
    stream: u64,
}

impl Noise {
    /// A field on one sub-stream. `stream` is a declared tag (a [`crate::world::Domain`]'s, or a
    /// stage's), so two systems never share a field by accident.
    pub const fn new(seed: u64, stream: u64) -> Self {
        Noise { seed, stream }
    }

    /// The raw coherent hash of a lattice point. The whole file is built on this one function.
    pub fn hash_at(&self, x: i64, y: i64) -> u64 {
        let a = mix(self.seed ^ mix(self.stream ^ (x as u64).wrapping_mul(0xA24B_AED4_963E_E407)));
        mix(a ^ mix((y as u64).wrapping_mul(0x9FB2_1C65_1E98_DF25)))
    }

    /// A coherent hash in `[0, 1)` — the value noise a lattice point holds.
    pub fn unit_at(&self, x: i64, y: i64) -> f64 {
        // 53 bits of mantissa: the standard `[0,1)` conversion, exact in binary.
        (self.hash_at(x, y) >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }

    /// The same hash on a third integer axis, so a lattice point can carry more than one number
    /// (a feature point needs two, and an octave needs its own offset).
    fn hash_at3(&self, x: i64, y: i64, salt: u64) -> u64 {
        mix(self.hash_at(x, y) ^ mix(salt.wrapping_mul(0xD6E8_FEB8_6659_FD93)))
    }

    /// Gradient noise in `[-1, 1]` (Perlin 1985; the fade is Perlin 2002's `6t⁵−15t⁴+10t³`).
    pub fn gradient(&self, x: f64, y: f64) -> f64 {
        let (x0, y0) = (x.floor(), y.floor());
        let (xi, yi) = (x0 as i64, y0 as i64);
        let (fx, fy) = (x - x0, y - y0);

        let dot = |cx: i64, cy: i64| {
            let direction = DIRECTIONS[(self.hash_at(cx, cy) & 7) as usize];
            direction.0 * (x - cx as f64) + direction.1 * (y - cy as f64)
        };

        let (n00, n10) = (dot(xi, yi), dot(xi + 1, yi));
        let (n01, n11) = (dot(xi, yi + 1), dot(xi + 1, yi + 1));
        let (sx, sy) = (fade(fx), fade(fy));
        let a = lerp(n00, n10, sx);
        let b = lerp(n01, n11, sx);
        lerp(a, b, sy)
    }

    /// Fractal Brownian motion (Mandelbrot & Van Ness 1968) over `octaves`, normalised so the value
    /// stays in `[-1, 1]` whatever the octave count. Each octave takes its own declared offset, so
    /// the octaves do not line up on the lattice.
    pub fn fbm(&self, x: f64, y: f64, octaves: u32, lacunarity: f64, gain: f64) -> f64 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut total = 0.0;
        let mut frequency = 1.0;
        for octave in 0..octaves {
            let ox = self.unit_at(octave as i64, 0x51_17);
            let oy = self.unit_at(0x9F_31, octave as i64);
            value += amplitude * self.gradient(x * frequency + ox, y * frequency + oy);
            total += amplitude;
            frequency *= lacunarity;
            amplitude *= gain;
        }
        if total == 0.0 {
            return 0.0;
        }
        value / total
    }

    /// Ridged multifractal (Musgrave, Kolb & Mace 1989) in `[0, 1]`: sharp ridges where the noise
    /// crosses zero, which is what the terrain stage's relief needs.
    pub fn ridged(&self, x: f64, y: f64, octaves: u32, lacunarity: f64, gain: f64) -> f64 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut total = 0.0;
        let mut frequency = 1.0;
        for octave in 0..octaves {
            let ox = self.unit_at(octave as i64, 0x2C_09);
            let oy = self.unit_at(0x77_53, octave as i64);
            let ridge = 1.0 - self.gradient(x * frequency + ox, y * frequency + oy).abs();
            value += amplitude * ridge * ridge;
            total += amplitude;
            frequency *= lacunarity;
            amplitude *= gain;
        }
        if total == 0.0 {
            return 0.0;
        }
        value / total
    }

    /// Domain warping (Perlin 1985 §; Quilez 2002): sample a position that has itself been moved by
    /// the field, so features bend instead of sitting on the lattice.
    pub fn warp(&self, x: f64, y: f64, strength: f64) -> (f64, f64) {
        let dx = self.gradient(x + 5.2, y + 1.3);
        let dy = self.gradient(x + 9.7, y + 2.8);
        (x + strength * dx, y + strength * dy)
    }

    /// Cellular noise (Worley 1996): the two nearest feature points and the cell owning the nearest.
    /// Feature points live inside their own cell, so the search is the nine cells around the sample.
    pub fn cellular(&self, x: f64, y: f64) -> Cellular {
        let (cx, cy) = (x.floor() as i64, y.floor() as i64);
        let mut out = Cellular { f1: f64::INFINITY, f2: f64::INFINITY, cell: (cx, cy) };
        for i in -1..=1 {
            for j in -1..=1 {
                let (gx, gy) = (cx + i, cy + j);
                let px = gx as f64 + self.unit_at(gx, gy);
                let py = gy as f64 + (self.hash_at3(gx, gy, 0xF00D) >> 11) as f64
                    * (1.0 / 9_007_199_254_740_992.0);
                let distance = ((px - x) * (px - x) + (py - y) * (py - y)).sqrt();
                if distance < out.f1 {
                    out.f2 = out.f1;
                    out.f1 = distance;
                    out.cell = (gx, gy);
                } else if distance < out.f2 {
                    out.f2 = distance;
                }
            }
        }
        out
    }
}

/// Perlin 2002's fade curve: `6t⁵ − 15t⁴ + 10t³`.
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_primitives_are_pure_functions_of_seed_and_position() {
        // A stage's effect is deterministic for a given world and seed — the property U3 claims.
        let field = Noise::new(0x5EED_2468, 1);
        let same = Noise::new(0x5EED_2468, 1);
        let other_seed = Noise::new(0x5EED_2469, 1);
        let other_stream = Noise::new(0x5EED_2468, 2);

        let samples = [(0.5, 0.5), (12.25, -3.75), (-100.125, 64.5), (1e6 + 0.5, 7.0)];
        for (x, y) in samples {
            assert_eq!(field.gradient(x, y), same.gradient(x, y), "gradient must be pure");
            assert_eq!(
                field.fbm(x, y, 6, 2.0, 0.5),
                same.fbm(x, y, 6, 2.0, 0.5),
                "fBm must be pure"
            );
            assert_eq!(field.ridged(x, y, 5, 2.0, 0.5), same.ridged(x, y, 5, 2.0, 0.5));
            assert_eq!(field.cellular(x, y), same.cellular(x, y));
            assert_eq!(field.warp(x, y, 1.5), same.warp(x, y, 1.5));

            // A different seed and a different stream are different fields, not the same one.
            assert_ne!(field.gradient(x, y), other_seed.gradient(x, y));
            assert_ne!(field.gradient(x, y), other_stream.gradient(x, y));
        }
    }

    #[test]
    fn every_primitive_stays_inside_its_declared_bound() {
        let field = Noise::new(7, 3);
        for i in 0..200 {
            let x = i as f64 * 0.37 - 20.0;
            let y = i as f64 * -0.11 + 3.0;
            let gradient = field.gradient(x, y);
            assert!(gradient.abs() <= 1.0, "gradient out of [-1, 1]: {gradient}");
            let fbm = field.fbm(x, y, 8, 2.0, 0.5);
            assert!(fbm.abs() <= 1.0, "fBm out of [-1, 1]: {fbm}");
            let ridged = field.ridged(x, y, 8, 2.0, 0.5);
            assert!((0.0..=1.0).contains(&ridged), "ridged out of [0, 1]: {ridged}");
            let unit = field.unit_at(i, -i);
            assert!((0.0..1.0).contains(&unit), "unit hash out of [0, 1): {unit}");
        }
    }

    #[test]
    fn cellular_orders_its_two_distances_and_names_a_nearest_cell() {
        let field = Noise::new(11, 5);
        for i in 0..50 {
            let x = i as f64 * 0.53;
            let y = 4.0 - i as f64 * 0.29;
            let cellular = field.cellular(x, y);
            assert!(cellular.f1 <= cellular.f2, "f1 must be the nearer of the two");
            assert!(cellular.f1 >= 0.0);
            // The reported cell really is a candidate: its own point is inside the cell's unit square.
            let (gx, gy) = cellular.cell;
            let px = gx as f64 + field.unit_at(gx, gy);
            let py = gy as f64
                + (field.hash_at3(gx, gy, 0xF00D) >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0);
            assert!((px - x).abs() <= 2.0 && (py - y).abs() <= 2.0, "the cell must be a neighbour");
        }
        // Two neighbouring samples on a coarse field often share a plate; the map is not constant.
        let cells: std::collections::BTreeSet<(i64, i64)> =
            (0..40).map(|i| field.cellular(i as f64 * 9.0, 0.5).cell).collect();
        assert!(cells.len() > 3, "a cellular field must have more than a few plates: {cells:?}");
    }

    #[test]
    fn domain_warping_moves_a_point_and_keeps_it_a_function() {
        let field = Noise::new(99, 1);
        let (x, y) = (3.5, -2.5);
        let (wx, wy) = field.warp(x, y, 2.0);
        assert!(
            (wx - x).abs() <= 2.0 && (wy - y).abs() <= 2.0,
            "the warp must stay inside its declared strength"
        );
        assert!((wx, wy) != (x, y), "a zero-strength warp is not a warp");
        assert_eq!(field.warp(x, y, 2.0), (wx, wy));
    }
}
