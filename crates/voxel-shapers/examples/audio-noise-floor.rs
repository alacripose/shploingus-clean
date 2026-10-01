//! **PROTOTYPE — THROWAWAY.** Nothing depends on this file; delete it freely.
//!
//! Question: *what do this world's numbers actually give a procedural rain/wind
//! voice, and where does its noise floor land?*
//!
//! R33 ("rodio dynamic audio for all audio") is scoped nowhere, and nobody had
//! asked the shapers what they can actually drive. **No rodio and no audio
//! device**: the noise floor is a property of the numbers and the gain map, not
//! of the sink, and this repo can measure that headless.
//!
//! Run: `cargo run -p voxel-shapers --example audio-noise-floor`

use voxel_shapers::climate::{self, DAYS_PER_YEAR, TICKS_PER_DAY, YEAR_TICKS};
use voxel_shapers::spec::{Config, WorldSpec};
use voxel_shapers::weather;

/// Decibels relative to full scale, from an RMS. Floors at a nominal -240 dB so
/// true digital silence prints as silence rather than as `-inf`.
fn dbfs(rms: f64) -> f64 {
    20.0 * rms.max(1e-12).log10()
}

/// splitmix64 — same generator `voxel-noise` publishes, inlined so the
/// prototype depends on nothing it is not measuring.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    /// Uniform in `[-1, 1)`.
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 40) as f32 / (1u32 << 24) as f32) * 2.0 - 1.0
    }
}

/// A noise voice: white noise, one shaping pole, one gain. **The filter shape is
/// not the finding** — the level and the dynamic range are, so this stays crude
/// on purpose and says so rather than implying a rain model it does not have.
struct Voice {
    rng: Rng,
    state: f32,
    /// One-pole coefficient in `[0, 1)`; larger is darker.
    damp: f32,
}

impl Voice {
    fn new(seed: u64, damp: f32) -> Self {
        Voice { rng: Rng::new(seed), state: 0.0, damp }
    }

    /// Render `frames` frames at a linear gain, returning `(rms, peak)`.
    fn render(&mut self, frames: usize, gain: f32) -> (f64, f64) {
        let mut sum = 0.0f64;
        let mut peak = 0.0f64;
        for _ in 0..frames {
            let x = self.rng.next();
            // y[n] = (1-d) * x[n] + d * y[n-1]
            self.state = (1.0 - self.damp) * x + self.damp * self.state;
            let s = (self.state * gain) as f64;
            sum += s * s;
            peak = peak.max(s.abs());
        }
        ((sum / frames as f64).sqrt(), peak)
    }
}

/// Equal to printed precision but not bit-equal — the case a rounded readout
/// hides and an exact comparison catches.
fn i_is_close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6 && a != b
}

fn main() {
    let spec = WorldSpec::new(20_260_930, Config::default());

    // ---------------------------------------------------------------------
    // 1. What the world actually spans.
    // ---------------------------------------------------------------------
    println!("== the world's own numbers ==\n");

    let (lo, hi, step) = (-8000.0_f64, 8000.0_f64, 500.0_f64);
    let ticks = [
        0,
        YEAR_TICKS / 4,
        YEAR_TICKS / 2,
        (YEAR_TICKS * 3) / 4,
        YEAR_TICKS - 1,
    ];

    let mut p_min = f64::INFINITY;
    let mut p_max = f64::NEG_INFINITY;
    let mut m_min = f64::INFINITY;
    let mut m_max = f64::NEG_INFINITY;
    let mut columns = 0u32;
    let mut leeward = 0u32;
    let mut windward = 0u32;
    // Columns where precipitation never moves across the whole sampled year.
    let mut frozen = 0u32;
    let mut frozen_max_mm = 0.0f64;

    let mut x = lo;
    while x <= hi {
        let mut z = lo;
        while z <= hi {
            columns += 1;
            let m = climate::moisture(&spec, x, z);
            m_min = m_min.min(m);
            m_max = m_max.max(m);

            let bearing = weather::wind_bearing_rad(&spec, x, z);
            let (dx, dz) = {
                const H: f64 = 2.0;
                use voxel_shapers::terrain;
                (
                    (terrain::elevation_m(&spec, x + H, z) - terrain::elevation_m(&spec, x - H, z))
                        / (2.0 * H),
                    (terrain::elevation_m(&spec, x, z + H) - terrain::elevation_m(&spec, x, z - H))
                        / (2.0 * H),
                )
            };
            let upslope = dx * bearing.sin() + dz * bearing.cos();
            if upslope <= 0.0 {
                leeward += 1;
            } else {
                windward += 1;
            }

            let mut c_min = f64::INFINITY;
            let mut c_max = f64::NEG_INFINITY;
            for &t in &ticks {
                let p = weather::precipitation_mm(&spec, x, z, t);
                p_min = p_min.min(p);
                p_max = p_max.max(p);
                c_min = c_min.min(p);
                c_max = c_max.max(p);
            }
            if c_max - c_min == 0.0 {
                frozen += 1;
                frozen_max_mm = frozen_max_mm.max(c_max);
            }
            z += step;
        }
        x += step;
    }

    println!("columns sampled          {columns}  ({lo} .. {hi} m, step {step} m)");
    println!("windward / leeward       {windward} / {leeward}");
    println!("moisture                 {m_min:.4} .. {m_max:.4}");
    println!(
        "precipitation mm/year    {p_min:.1} .. {p_max:.1}   (span {:.1} dB)",
        20.0 * (p_max / p_min.max(1e-9)).log10()
    );

    // ---------------------------------------------------------------------
    // 2. The finding about the rain shadow.
    // ---------------------------------------------------------------------
    println!("\n== columns whose precipitation never moves across a year ==");
    println!("{} of {columns} ({:.1}%)", frozen, 100.0 * frozen as f64 / columns as f64);
    println!("loudest of them          {frozen_max_mm:.1} mm/year — constant, not zero");

    // Prove it is the branch, not the sampling. **Printed to full precision on
    // purpose**: the first version of this printed two decimals and showed a
    // windward column as `651.19 / 651.19`, which reads as a counterexample but
    // is not one — `sin(PI)` is `1.2246e-16`, not zero, so the seasonal term is
    // `0.7500000000000001` at mid-year and `0.75` at tick 0. The two columns
    // differ in the seventeenth digit and are equal to any printed precision.
    let mut shown = 0;
    let mut x = lo;
    while x <= hi && shown < 3 {
        let mut z = lo;
        while z <= hi {
            let a = weather::precipitation_mm(&spec, x, z, 0);
            let b = weather::precipitation_mm(&spec, x, z, YEAR_TICKS / 2);
            let season = {
                let lo_p = weather::precipitation_mm(&spec, x, z, 0);
                let hi_p = (0..64)
                    .map(|i| weather::precipitation_mm(&spec, x, z, i * YEAR_TICKS / 64))
                    .fold(f64::NEG_INFINITY, f64::max);
                20.0 * (hi_p / lo_p.max(1e-9)).log10()
            };
            if a == b && shown < 3 {
                println!(
                    "  ({x:>6.0}, {z:>6.0})  tick 0 = {a:.6}  mid-year = {b:.6}  delta = {:.1e}",
                    (b - a).abs()
                );
                shown += 1;
            } else if shown < 3 && i_is_close(a, b) {
                println!(
                    "  ({x:>6.0}, {z:>6.0})  tick 0 = {a:.6}  mid-year = {b:.6}  \
                     seasonal swing {season:5.1} dB  <- NOT frozen, merely rounded alike"
                );
                shown += 1;
            }
            z += step;
        }
        x += step;
    }

    // ---------------------------------------------------------------------
    // 3. Wind: what does the world even have?
    // ---------------------------------------------------------------------
    println!("\n== wind ==");
    let w = weather::sample(&spec, 0.0, 0.0, 0);
    println!("WeatherSample at origin: precipitation_mm {:.1}, wind_bearing_rad {:.4}, upslope {:.5}",
        w.precipitation_mm, w.wind_bearing_rad, w.upslope);
    println!("there is no wind SPEED field anywhere in the shapers — bearing is a direction,");
    println!("upslope is a slope. Wind loudness has to be invented, or declared.");

    // ---------------------------------------------------------------------
    // 4. The noise floor: what the numbers do to a gain map.
    // ---------------------------------------------------------------------
    println!("\n== rain voice: level against the world's own range ==\n");

    const FRAMES: usize = 48_000;
    let ref_p = 1000.0f64; // the world's mid-range, mm/year

    let cases: [(&str, f64); 4] = [
        ("world minimum", p_min),
        ("10% of reference", ref_p * 0.1),
        ("reference", ref_p),
        ("world maximum", p_max),
    ];

    println!(
        "{:<18} {:>12} {:>10} {:>10} {:>9}",
        "case", "mm/year", "linear dBFS", "declared dBFS", "peak"
    );

    // **A unity-gain noise voice is not 0 dBFS.** Measured: filtered noise at
    // gain 1.0 reads about -8 dBFS RMS, because a one-pole lowpass throws away
    // most of the spectrum's power. The first version of this map anchored the
    // wettest column to gain 1.0 and so lost ~8 dB of headroom for free.
    // So the map is normalised against the voice's **measured** unity RMS.
    let mut probe = Voice::new(1, 0.35);
    let (unity_rms, _) = probe.render(FRAMES, 1.0);
    let target_rms = 10f64.powf(-6.0 / 20.0); // -6 dBFS: headroom for a mix
    let normalise = (target_rms / unity_rms) as f32;
    println!(
        "unity-gain voice measures {:.1} dBFS; to land the wettest column on -6 dBFS it \
         needs gain {normalise:.3}, not 1.0\n",
        dbfs(unity_rms)
    );

    // Declared map: the world's own span laid onto 48 dB of range, ending at the
    // target rather than at unity. Logarithmic in mm — which is the finding, so
    // it is written out longhand rather than hidden in a helper.
    const MAP_RANGE_DB: f64 = 48.0;
    let log_span = (p_max / p_min).ln();
    for (name, mm) in cases {
        let linear = (mm / p_max) as f32;

        let t = ((mm / p_min).ln() / log_span).clamp(0.0, 1.0);
        let declared_db = -MAP_RANGE_DB + MAP_RANGE_DB * t;
        let declared_gain = (10f64.powf(declared_db / 20.0) * normalise as f64) as f32;

        let mut a = Voice::new(1, 0.35);
        let (rms_a, _) = a.render(FRAMES, linear);
        let mut b = Voice::new(1, 0.35);
        let (rms_b, peak_b) = b.render(FRAMES, declared_gain);
        println!(
            "{name:<18} {mm:>12.1} {:>10.1} {:>10.1} {:>9.3}",
            dbfs(rms_a as f64),
            dbfs(rms_b as f64),
            peak_b
        );
    }

    println!("\nreference floors: 16-bit full scale is about -96 dBFS, f32 about -144 dBFS.");

    // ---------------------------------------------------------------------
    // 5. Does the quiet end survive being pushed down?
    // ---------------------------------------------------------------------
    println!("\n== the quiet end, amplified to listen to ==");
    let mut quiet = Voice::new(2, 0.35);
    let (rms, _) = quiet.render(FRAMES, 1.0);
    println!(
        "a unity-gain voice reads {:.1} dBFS, so the world's driest column at a linear map is",
        dbfs(rms)
    );
    let quietest_linear = (p_min / p_max) as f32 * rms as f32;
    println!(
        "{:.1} dBFS — {} the 16-bit floor.",
        dbfs(quietest_linear as f64),
        if dbfs(quietest_linear as f64) < -96.0 { "BELOW" } else { "above" }
    );

    println!("\n== day length, for the record ==");
    println!(
        "a day is {TICKS_PER_DAY} ticks over {DAYS_PER_YEAR} days; a year is {YEAR_TICKS} ticks"
    );
}
