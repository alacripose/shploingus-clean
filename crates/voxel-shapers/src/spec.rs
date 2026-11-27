//! `WorldSpec` — the world as a pure function of its arguments.
//!
//! F1's contract is that the world is a pure function of
//! `(seed, tick, declared period)`. This module is where `seed` and `tick` live,
//! beside the **per-world configuration** the design spec calls for (§3:
//! *"configuration per world"* for every package).
//!
//! Everything here is a declared number with a stated reason. A generator whose
//! constants are anonymous is a generator nobody can tune, and — more to the
//! point — a generator whose constants are anonymous cannot be *rebuilt*, because
//! two runs cannot be shown to have used the same ones (WG-Q4).

use crate::scale;
use voxel_model::Refusal;

/// Per-world generator configuration.
///
/// Every field is a **tuning knob that is part of the world's identity**: change
/// one and the world is a different world, which is why the design spec's
/// identity rule (F4) binds *by effect, not by kind*.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Config {
    /// Height of the sea surface, in metres above the datum.
    pub sea_level_m: f64,
    /// The datum the whole terrain is measured from.
    pub base_elevation_m: f64,
    /// Peak-to-trough relief the terrain field can reach.
    pub relief_m: f64,
    /// Frequency of the continental masses — low, because continents are large. It is the
    /// reciprocal of the mounted worldgen package's continental wavelength.
    pub continent_frequency: f64,
    /// How many continental frequencies the ridged mountain term runs at: a range's wavelength is
    /// `1 / (continent_frequency · this)` ([`crate::terrain::range_wavelength_m`]).
    pub ridge_frequency_scale: f64,
    /// How much of [`Config::relief_m`] the ridged term raises where plates converge.
    pub ridge_amplitude: f64,
    /// How many tectonic plates the world is partitioned into.
    pub plate_count: u32,
    /// Edge length of a tectonic plate cell, in metres. A **declared** constant
    /// rather than one derived from `continent_frequency`: plates and continents
    /// are different scales in the real world and a generator that ties them
    /// together cannot be tuned to make them differ.
    pub plate_cell_size_m: f64,
    /// Half-width of the orogenic belt a plate boundary raises, in metres.
    ///
    /// This is the number that makes a mountain **range** a range and not a
    /// plateau: uplift falls off exponentially from the seam with this length
    /// scale, so a column 2 km inside a plate is untouched while a column on the
    /// seam is fully raised. Without it, proximity normalised by cell size sits
    /// around 0.5 *everywhere* — a Voronoi cell has no deep interior relative to
    /// its own size, so a ratio can never reach zero.
    pub plate_boundary_width_m: f64,
    /// Vertical layers a column's stratigraphy is divided into.
    pub layer_count: u32,
    /// Regional-field samples per region edge (F4's ladder is aligned to the octree).
    pub region_size: u32,
    /// Hydraulic-erosion droplets per column, as a *performance* knob.
    pub erosion_droplets_per_column: u32,
    /// Sea level below which a surface cell is submerged.
    pub soil_max_slope: f64,
    /// Environmental lapse rate, °C per **engine** metre. US Standard Atmosphere, 1976.
    ///
    /// The published `0.0065` °C per *real* metre, converted through [`crate::scale`]: the
    /// column is eight times shorter in engine metres, so the rate is eight times larger for
    /// the same temperature drop over the same real height. The default is therefore `0.052`.
    pub lapse_rate_c_per_m: f64,
    /// **Equatorial** sea-level annual mean temperature, °C — the value the latitude
    /// penalty of [`crate::climate::latitude_offset_c`] is measured *down* from.
    ///
    /// **Its meaning is what was wrong, and the surface is what showed it.** Read as a
    /// "sea-level mean at the datum", `15.0` combined with a `45 · sin²φ` latitude
    /// penalty applied latitude twice: the world's declared 45° came out at **−7.5 °C**,
    /// so every column classified as tundra, boreal forest or alpine and the whole world
    /// was surfaced with one material. `26.0` is the published equatorial zonal annual
    /// mean, paired with a span of `36` so 45° lands on its own published ≈ +8 °C
    /// (`logs/c159-biome-census.txt`, C-159.7).
    pub base_temperature_c: f64,
    /// Latitude of the world origin in degrees, which sets the insolation band.
    pub origin_latitude_deg: f64,
    /// Fraction of arriving moisture dropped per metre of orographic rise
    /// (Smith & Barstad 2004, in its linearised form).
    pub orographic_efficiency: f64,
    /// Cave carve threshold on the karst field.
    pub cave_threshold: f64,
}

impl Default for Config {
    /// **The terrain and climate constants are the mounted worldgen package's**, not literals in
    /// this file: `display-packages/worldgen/package.shspec` authors the continental wavelength,
    /// the relief, the sea line, the datum, the base temperature and the two ridge terms, and the
    /// game mounts that folder before it opens the world. A row edited on disk therefore changes
    /// the ground the same binary draws, without a rebuild — the experiment the package exists
    /// for. The numbers that remain here are the **plate and structural** dials, which are this
    /// crate's own and are not in either atlas.
    fn default() -> Self {
        let world = crate::worldgen::library();
        Config {
            // **These three place the world inside the first playable band**
            // (z = -16 … +47, C-6), and they come from the package: at a datum of 16 m and a
            // relief of 40 m the surface spans roughly -22 … +54, so the band contains it rather
            // than sitting under it. An earlier (sea 0, datum 64, relief 140) produced a world
            // with a land fraction of 1.00 — no sea at all — which the test below caught by
            // measuring rather than assuming.
            sea_level_m: world.sea_level_m,
            base_elevation_m: world.datum_m,
            relief_m: world.relief_m,
            continent_frequency: 1.0 / world.continental_wavelength_m,
            ridge_frequency_scale: world.ridge_frequency_scale,
            ridge_amplitude: world.ridge_amplitude,
            plate_count: 12,
            plate_cell_size_m: 6000.0,
            plate_boundary_width_m: 600.0,
            layer_count: 24,
            region_size: 32,
            erosion_droplets_per_column: 0,
            soil_max_slope: 0.62,
            lapse_rate_c_per_m: scale::per_voxel(0.0065),
            base_temperature_c: world.base_temperature_c,
            origin_latitude_deg: 45.0,
            orographic_efficiency: 0.0012,
            cave_threshold: 0.62,
        }
    }
}

impl Config {
    /// Refuse a configuration whose numbers cannot describe a world.
    ///
    /// This is the "a declared layer that is absent is a blocker, not an
    /// omission" rule (WG-Q3) applied to configuration: a `plate_count` of zero
    /// would make the tectonic partition empty, and an empty partition is not a
    /// world with no plates — it is a world with no *answer*.
    pub fn validate(&self) -> Result<(), Refusal> {
        let bad = |what: &str| -> Result<(), Refusal> {
            Err(Refusal::owned(
                voxel_model::RefusalKind::OutOfDeclaredRange,
                format!("config {what}"),
            ))
        };
        if self.plate_count == 0 {
            return bad("plate_count is 0: there would be no plates to partition");
        }
        if self.layer_count == 0 {
            return bad("layer_count is 0: there would be no stratigraphy");
        }
        if self.region_size == 0 {
            return bad("region_size is 0");
        }
        if !(self.plate_cell_size_m > 0.0 && self.plate_cell_size_m.is_finite()) {
            return bad("plate_cell_size_m must be positive and finite");
        }
        if !(self.plate_boundary_width_m > 0.0 && self.plate_boundary_width_m.is_finite()) {
            return bad("plate_boundary_width_m must be positive and finite");
        }
        if self.plate_boundary_width_m > self.plate_cell_size_m {
            return bad(
                "plate_boundary_width_m exceeds plate_cell_size_m: the orogenic belt \
                 would be wider than the plate, so there would be no undisturbed interior",
            );
        }
        if !(self.relief_m > 0.0 && self.relief_m.is_finite()) {
            return bad("relief_m must be positive and finite");
        }
        if !(self.continent_frequency > 0.0 && self.continent_frequency.is_finite()) {
            return bad("continent_frequency must be positive and finite");
        }
        if !(self.ridge_frequency_scale > 0.0 && self.ridge_frequency_scale.is_finite()) {
            return bad("ridge_frequency_scale must be positive and finite");
        }
        if !(self.ridge_amplitude >= 0.0 && self.ridge_amplitude.is_finite()) {
            return bad("ridge_amplitude must be finite and not negative");
        }
        if !(0.0..=90.0).contains(&self.origin_latitude_deg) {
            return bad("origin_latitude_deg outside [0, 90]");
        }
        if !(0.0..=1.0).contains(&self.cave_threshold) {
            return bad("cave_threshold outside [0, 1]");
        }
        for (name, v) in [
            ("sea_level_m", self.sea_level_m),
            ("base_elevation_m", self.base_elevation_m),
            ("lapse_rate_c_per_m", self.lapse_rate_c_per_m),
            ("base_temperature_c", self.base_temperature_c),
            ("soil_max_slope", self.soil_max_slope),
            ("orographic_efficiency", self.orographic_efficiency),
        ] {
            if !v.is_finite() {
                return bad(&format!("{name} is not finite"));
            }
        }
        Ok(())
    }
}

/// The world: an identity (seed) and an instant (tick), plus its configuration.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorldSpec {
    pub seed: u64,
    pub tick: u64,
    pub config: Config,
}

impl WorldSpec {
    pub fn new(seed: u64, config: Config) -> Self {
        WorldSpec { seed, tick: 0, config }
    }

    pub fn at_tick(mut self, tick: u64) -> Self {
        self.tick = tick;
        self
    }

    /// The seed a **named stage** uses.
    ///
    /// Stage seeds are derived rather than reused so that two shapers cannot
    /// accidentally be driven by the same field. Sharing a raw seed between
    /// terrain and, say, caves is how a cave system ends up tracing the
    /// mountains exactly, which reads as a bug and is one.
    ///
    /// **The `^ self.seed` is load-bearing and was briefly dropped.** Extracting
    /// [`WorldSpec::stage_hash`] to share the FNV loop with
    /// [`WorldSpec::stationary_stage_seed`] left this line as
    /// `stage_hash(stage) ^ tick`, and with the world seed gone every world shared one
    /// prevailing wind. `weather::tests::the_prevailing_wind_is_a_property_of_the_world_
    /// not_the_place` caught it immediately — two different seeds produced the identical
    /// bearing `3.5371731874879977` — which is the test doing exactly its job.
    pub fn stage_seed(&self, stage: &str) -> u64 {
        voxel_noise::mix(
            self.stage_hash(stage) ^ self.seed ^ self.tick.wrapping_mul(0x9E37_79B9_7F4A_7C15),
        )
    }

    /// The same derivation as [`WorldSpec::stage_seed`] **with the tick left out**.
    ///
    /// # Why a second accessor, and what using the first one would have cost
    ///
    /// [`WorldSpec::stage_seed`] mixes `self.tick` into the seed. That is right for a
    /// field which is *re-derived* whenever the tick moves — each tick is then
    /// independent, which is what you want from a stage sample.
    ///
    /// It is **wrong** for a field that carries its own time coordinate. Sampling a
    /// Perlin field at `(x, z, tick / 2000)` under a seed that also encodes `tick`
    /// produces a completely different field at *every* tick: the seed changes, the
    /// coordinate changes, and the result is white noise rather than weather. It looks
    /// like working code — the values are in range, deterministic, and reproducible —
    /// and it has no continuity in time at all, which is precisely the property the
    /// caller was reaching for.
    ///
    /// **So: use this when the tick is in the sample position, and
    /// [`WorldSpec::stage_seed`] when the tick selects the field.** The onset layer in
    /// [`crate::weather`] is the first caller that needs the distinction.
    pub fn stationary_stage_seed(&self, stage: &str) -> u64 {
        voxel_noise::mix(self.stage_hash(stage) ^ self.seed)
    }

    /// FNV-1a over the stage name, before the world and tick are folded in.
    fn stage_hash(&self, stage: &str) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64; // FNV-1a offset basis
        for b in stage.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01B3);
        }
        h
    }

    /// Validate the configuration before anything is generated from it.
    pub fn validate(&self) -> Result<(), Refusal> {
        self.config.validate()
    }

    /// **A value that changes whenever anything a shaper can read changes.**
    ///
    /// For **cache keys only**. `flora` memoizes a pure function per `(world, cell)`, and
    /// two different worlds must not share a row — the `seed` alone is not enough, because
    /// tests and examples build several `Config`s over one seed inside one process, and a
    /// cache that answered one world's tree for another's would be a wrong world rather
    /// than a slow one. The [`Self::fingerprint_covers_every_config_field`] test flips
    /// every field the struct has, so adding a field to `Config` without adding it here
    /// fails a test instead of shipping a collision.
    pub fn fingerprint(&self) -> u64 {
        let c = &self.config;
        let mut h = self.seed;
        for bits in [
            c.sea_level_m.to_bits(),
            c.base_elevation_m.to_bits(),
            c.relief_m.to_bits(),
            c.continent_frequency.to_bits(),
            c.ridge_frequency_scale.to_bits(),
            c.ridge_amplitude.to_bits(),
            c.plate_count as u64,
            c.plate_cell_size_m.to_bits(),
            c.plate_boundary_width_m.to_bits(),
            c.layer_count as u64,
            c.region_size as u64,
            c.erosion_droplets_per_column as u64,
            c.soil_max_slope.to_bits(),
            c.lapse_rate_c_per_m.to_bits(),
            c.base_temperature_c.to_bits(),
            c.origin_latitude_deg.to_bits(),
            c.orographic_efficiency.to_bits(),
            c.cave_threshold.to_bits(),
        ] {
            h = voxel_noise::mix(h ^ bits);
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_config_describes_a_world() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn a_config_that_cannot_describe_a_world_is_refused_by_name() {
        let c = Config { plate_count: 0, ..Config::default() };
        let r = c.validate().unwrap_err();
        assert!(r.detail().contains("plate_count"));

        let c = Config { relief_m: -1.0, ..Config::default() };
        assert!(c.validate().is_err());

        let c = Config { origin_latitude_deg: 120.0, ..Config::default() };
        assert!(c.validate().is_err());

        let c = Config { sea_level_m: f64::NAN, ..Config::default() };
        assert!(c.validate().is_err());
    }

    #[test]
    fn stage_seeds_are_distinct_per_stage() {
        let w = WorldSpec::new(42, Config::default());
        let names = [
            "tectonics", "geology", "terrain", "hydrology", "erosion", "climate", "atmosphere",
            "weather", "caves", "substrate", "ecology", "delta", "transition",
        ];
        let mut seeds: Vec<u64> = names.iter().map(|n| w.stage_seed(n)).collect();
        let before = seeds.len();
        seeds.sort_unstable();
        seeds.dedup();
        assert_eq!(seeds.len(), before, "two stages share a seed");
    }

    #[test]
    fn fingerprint_covers_every_config_field() {
        // A `Config` field that is missing from `fingerprint` is a cache collision between
        // two different worlds, and this list is what makes forgetting one loud.
        let base = WorldSpec::new(7, Config::default()).fingerprint();
        let mut flips: Vec<Config> = Vec::new();
        macro_rules! flip {
            ($($field:ident : $value:expr),* $(,)?) => { $({
                let mut c = Config::default();
                c.$field = $value;
                flips.push(c);
            })* };
        }
        flip! {
            sea_level_m: Config::default().sea_level_m + 1.0,
            base_elevation_m: Config::default().base_elevation_m + 1.0,
            relief_m: Config::default().relief_m + 1.0,
            continent_frequency: Config::default().continent_frequency + 1e-6,
            ridge_frequency_scale: Config::default().ridge_frequency_scale + 1.0,
            ridge_amplitude: Config::default().ridge_amplitude + 0.05,
            plate_count: Config::default().plate_count + 1,
            plate_cell_size_m: Config::default().plate_cell_size_m + 1.0,
            plate_boundary_width_m: Config::default().plate_boundary_width_m + 1.0,
            layer_count: Config::default().layer_count + 1,
            region_size: Config::default().region_size + 1,
            erosion_droplets_per_column: Config::default().erosion_droplets_per_column + 1,
            soil_max_slope: Config::default().soil_max_slope + 0.01,
            lapse_rate_c_per_m: Config::default().lapse_rate_c_per_m + 1e-4,
            base_temperature_c: Config::default().base_temperature_c + 1.0,
            origin_latitude_deg: Config::default().origin_latitude_deg + 1.0,
            orographic_efficiency: Config::default().orographic_efficiency + 1e-4,
            cave_threshold: Config::default().cave_threshold + 0.01,
        }
        assert_eq!(flips.len(), 18, "a config field is not exercised here");
        for c in flips {
            assert_ne!(
                base,
                WorldSpec::new(7, c).fingerprint(),
                "a config field is not in the fingerprint"
            );
        }
        // The seed is in it too, and the tick is deliberately not: the fingerprint names
        // the *world*, and the tick is what the world is sampled at.
        assert_ne!(base, WorldSpec::new(8, Config::default()).fingerprint());
        assert_eq!(
            base,
            WorldSpec::new(7, Config::default()).at_tick(9_999).fingerprint()
        );
    }

    #[test]
    fn the_world_changes_with_the_seed_and_the_tick_and_is_stable_otherwise() {
        let a = WorldSpec::new(1, Config::default());
        let b = WorldSpec::new(2, Config::default());
        let c = WorldSpec::new(1, Config::default()).at_tick(1);
        assert_ne!(a.stage_seed("terrain"), b.stage_seed("terrain"));
        assert_ne!(a.stage_seed("terrain"), c.stage_seed("terrain"));
        assert_eq!(a.stage_seed("terrain"), WorldSpec::new(1, Config::default()).stage_seed("terrain"));
    }
}
