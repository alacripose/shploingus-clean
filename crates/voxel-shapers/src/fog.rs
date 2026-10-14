//! Fog, mist, spray and foam — the **near-surface** water systems.
//!
//! **Source.** `docs/fog_mist_foam_and_aerosol_dispersion.pdf`. This is the companion to
//! [`crate::clouds`]: the same parcel, the same saturation relation, and a different question —
//! not *what is in the column* but *what is in the lowest metres of one address, and why*.
//!
//! # Addressed in metres, not per column
//!
//! Fog is a **layer**, and the guide says so from its first page: §3's ground fog is *"fog
//! concentrated in a shallow layer close to the surface"*, its depth grows as cooling continues,
//! and a valley's pool deepens while the ridge above it stays clear. So the entry point is
//! [`fog_at`], which takes the world's own **cubic address** `(x, y, z)` — the same address the
//! field plane samples and the same one the mesher draws — and answers nothing above the layer's
//! top. The column's loading ([`FogColumn`]) is computed once per address and shaped vertically,
//! which is the structure the 4×4×4 subvoxels and the rain animation need from it.
//!
//! # The one rule everything here obeys
//!
//! §1.1 and §2.3 are emphatic that **a white plume does not identify its own mechanism**: *"you
//! cannot infer the formation mechanism from a pale, white appearance alone."* Three pathways
//! produce suspended droplets and they are not the same process —
//!
//! * **condensation**, which needs a thermodynamic path to saturation: cooling **or** moistening;
//! * **mechanical atomization** (spray), which produces droplets with the air **below** saturation,
//!   so it must not be gated on humidity at all;
//! * **bubble bursting**, which is the only way foam becomes aerosol, and only through an explicit
//!   burst — the foam itself is a *liquid-surface state*.
//!
//! So this module never reads an appearance. It computes each source separately, and the
//! [`FogMechanism`] it reports is the **largest single contributor**, named from the cause. The
//! guide's caution is a test: *"do not generate sea fog solely from wind speed … they should be
//! separate but coupled processes."*
//!
//! # The quantitative handle
//!
//! Droplets scatter, and how much they scatter is Koschmieder's: `V ≈ 3.912 / β` for a 2 % contrast
//! threshold, with `β` the extinction of the **droplet field** (§8.1). Visibility is therefore read
//! from the droplets and never from relative humidity — *"do not map visibility directly from RH
//! alone; high RH can occur without visible fog if liquid droplets are absent"* (§10.1).
//!
//! # The budget
//!
//! §10.2's sentence is the shape of this module: *change in airborne liquid water = condensation
//! growth + injected spray − evaporation − deposition − precipitation loss + net transport*. What a
//! **stepless field** can honestly report is the equilibrium loading and its two producers; the
//! removals that need a store and a step (evaporation of existing droplets over time, deposition,
//! precipitation loss, transport) are named here and are **not** faked as numbers nothing can
//! produce. [`FogColumn::lwc_g_m3`] is `condensation_g_m3 + spray_g_m3` exactly, and that identity
//! is asserted.
//!
//! # Fidelity, and the tier this is
//!
//! §10.3's **Tier 2**. Temperature, humidity, dew point, surface-water temperature, pooling and
//! mixing are read or taken as inputs; drop-size *bins*, aerosol activation and chemistry are
//! Tier 3 and are not here. Spray's own fate is carried by one declared effective radius and
//! Stokes settling, which is the whole of what a single-radius spray can say.
//!
//! # What is read, and what is not
//!
//! * **Read.** The parcel ([`clouds::surface_air`] — the same one the cloud model builds clouds
//!   from, so there is one surface truth and not two), the wind's own strength through
//!   [`weather::onset`] and its direction through [`weather::wind_bearing_rad`], the terrain's
//!   shelter ([`shelter`]), the upslope along the wind ([`weather::sample`]), and the sea's own
//!   surface temperature with its seasonal lag ([`water_surface_temperature_c`]).
//! * **Not read yet, and each with its door named.** Flow accumulation and *lakes* (the guide's
//!   lakeside cases) want [`crate::hydrology`] at a watershed radius and are the expensive read this
//!   field does not make per address; cloud cover, which weakens radiative cooling, is
//!   [`clouds::cloud_column`]'s; and the 256-lattice fluid surface, where foam is actually drawn, is
//!   [`crate::fluidics`]'s.

use crate::clouds::{self, SurfaceAir};
use crate::scale;
use crate::spec::WorldSpec;
use crate::{climate, terrain, weather};

/// Visibility at or below which the guide's §1.1 convention calls the suspension **fog**, metres.
pub const FOG_VISIBILITY_M: f64 = scale::voxels(1_000.0);

/// Visibility under which it is **mist** rather than haze, metres. The guide notes the operational
/// thresholds vary between services; this one is declared so a class boundary exists at all.
pub const MIST_VISIBILITY_M: f64 = scale::voxels(10_000.0);

/// The visibility reported when there are no droplets in the air at all, metres — a cap, not a
/// measurement, because `V = 3.912 / β` diverges as `β → 0` and a caller wants a number.
pub const CLEAR_VISIBILITY_M: f64 = scale::voxels(100_000.0);

/// Koschmieder's constant: the 2 % contrast threshold's `−ln(0.02)`.
pub const KOSCHMIEDER: f64 = 3.912;

/// The effective radius of a **condensed** droplet, metres — 8 µm, mid-range for fog.
pub const DROPLET_RADIUS_M: f64 = 8.0e-6;

/// The effective radius of a **wind-driven spray** drop, metres. Larger than a fog droplet and
/// declared at the size a whitecap throws: the guide's split (§8.2, §5) is that mechanically made
/// drops overlap fog sizes at the small end and settle out quickly at the large end.
pub const SPRAY_RADIUS_M: f64 = 40.0e-6;

/// Liquid water's density, kg/m³.
pub const WATER_DENSITY_KG_M3: f64 = 1_000.0;

/// The liquid water content at which Koschmieder's relation first puts visibility at the fog
/// threshold — 0.021 g/m³.
///
/// **Not picked: derived, and asserted.** `β = 3·LWC / (2·ρ_w·r)` at [`DROPLET_RADIUS_M`] reaches
/// `3.912 / FOG_VISIBILITY_M` at this loading, so it is the module's own statement of the
/// convention in mass units, and the test that pins the pair is
/// `the_fog_threshold_is_the_convention_in_mass_units`.
pub const FOG_LWC_G_M3: f64 = 0.021;

/// The loading at which the field stops thickening, g/m³ — dense fog, the guide's §8.1 regime where
/// a bank of many small droplets has high extinction on little mass.
pub const MAX_LWC_G_M3: f64 = 0.5;

/// What each degree of saturation excess condenses into, g/m³ per °C.
///
/// **Declared**, and it is §10.1's *"relax supersaturation by condensing vapor into droplets"*
/// with the timescale folded into one coefficient: a simple model may use a timescale rather than
/// droplet growth, and this is that, stated in the units the field reports.
pub const CONDENSATION_YIELD_G_M3_PER_C: f64 = 0.12;

/// How strongly a sheltered address concentrates the cooling the parcel already carries, `[0, 1]`.
///
/// The parcel's own night cooling is [`clouds`]' and is not repeated here: what a valley adds is
/// **drainage** — *"cold air drains downslope and collects in low terrain"* (§3.2) — so this
/// re-weights its effect rather than inventing a second cooling term.
pub const POOLING_WEIGHT: f64 = 0.75;

/// The horizontal reach the shelter is measured over, metres — the guide's *"shallow near-surface
/// process"* and the scale a hollow is: about a hundred metres, not a watershed.
pub const SHELTER_REACH_M: f64 = 64.0;

/// The depth below its surroundings at which an address reads as **fully sheltered**, metres.
pub const SHELTER_DEPTH_M: f64 = scale::voxels(8.0);

/// How deep the layer is where nothing pools, metres — the guide's *"shallow saturated layer"* over
/// open, unsheltered ground.
pub const LAYER_SKIN_DEPTH_M: f64 = scale::voxels(4.0);

/// How deep the layer is where cold air pools fully, metres. Declared, and it is the guide's §3.1
/// sequence's *"continued cooling can increase liquid water content and deepen fog"* carried to the
/// bottom of a basin: a pool, not a stratus deck.
pub const LAYER_POOL_DEPTH_M: f64 = scale::voxels(40.0);

/// The temperature contrast between water and air at which the guide's §4 **steam fog** begins, °C
/// — *"an autumn lake, rivers, reservoirs, or open water during a cold-air outbreak"*.
pub const STEAM_CONTRAST_C: f64 = 4.0;

/// The saturation deficit within which falling rain moistens the air enough for the guide's §4
/// **precipitation / frontal fog**, °C.
pub const FRONTAL_SPREAD_C: f64 = 2.0;

/// Wind at or below which whitecaps do not form, m/s — the guide's §5 *"breaking waves"* onset.
/// Beaufort 4 is when whitecaps become numerous, and this is the declared value of it.
pub const WHITECAP_WIND_M_S: f64 = 7.0;

/// The spray injected by fully exposed, fully breaking water, g/m³ at one metre per second above
/// [`WHITECAP_WIND_M_S`]. Declared: the guide gives spray a source flux and no constant.
pub const SPRAY_YIELD_G_M3_PER_M_S: f64 = 0.010;

/// The fraction of a foam field's bubbles that burst into airborne droplets.
///
/// §5 and §10.3: foam is a *surface state* and aerosol leaves it **only** through an explicit
/// burst, so this is the coupling, and it is small — most of a foam's gas returns to the air
/// directly and most of its film drains back into the liquid.
pub const FOAM_BURST_FRACTION: f64 = 0.05;

/// Wind at which the sea's surface reads as completely white, m/s — the top of the whitecap range,
/// declared at the point the guide's *"fully developed sea"* would sit.
pub const FOAM_FULL_WIND_M_S: f64 = 20.0;

/// The minimum wind of a calm day, m/s — a still address is not a windless one.
pub const WIND_CALM_M_S: f64 = 1.5;

/// The wind a fully developed storm's onset adds, m/s. Read through [`weather::onset`], which is
/// the share of the column's annual rain falling **now**, because in this tree a storm *is* wind
/// and rain: the guide gives no wind field, and a second one would be the second unrelated field
/// this crate's rules forbid (WG-Q5). Declared so the top of the world's onset range reaches a
/// breaking sea.
pub const WIND_STORM_M_S: f64 = 18.0;

/// How far the water surface's own temperature lags the season, ticks — an eighth of a year.
///
/// §4.2 and the reason water bodies are not "simply a source of more humidity": *"large lakes and
/// oceans have substantial heat capacity, so their temperatures change more slowly than land
/// surface temperatures."* One declared fraction, no second climate.
pub const WATER_LAG_TICKS: u64 = climate::YEAR_TICKS / 8;

/// The temperature below which the field declares **ice fog** rather than supercooled liquid, °C.
///
/// §9.3's caution is that *"do not treat every subzero fog as an ice-crystal field: supercooled
/// liquid droplets can persist below 0 C"*, so freezing is **not** the boundary — this much colder
/// threshold is, and it is declared rather than emergent for the same reason the guide declines to
/// give one.
pub const ICE_FOG_C: f64 = -25.0;

/// The dynamic viscosity of air at 0 °C, Pa·s — for Stokes settling of spray drops.
pub const AIR_VISCOSITY_PA_S: f64 = 1.81e-5;

/// The lift per unit of upslope a windward slope gives the air, metres — the same declared gain
/// [`crate::clouds`] uses to make its orographic cloud, so the fog standing on a slope and the
/// cloud above it are one lift and not two.
const OROGRAPHIC_LIFT_M: f64 = scale::voxels(8_000.0);

/// What produced an address's droplets. **The cause, never the appearance** (guide §1.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum FogMechanism {
    /// No droplets: the air is subsaturated and nothing mechanical is injecting any.
    None,
    /// Condensation reached by the surface's own cooling, concentrated where cold air pools (§3).
    Ground,
    /// Warm, moist air cooled to saturation by a **colder water surface** (§4).
    Advection,
    /// Colder air moistened by a **warmer water surface** — the guide's steam fog (§4).
    Steam,
    /// Moist air lifted along terrain and cooled by its own ascent (§4, upslope).
    Upslope,
    /// Rain evaporating into cooler air (§4, precipitation / frontal fog).
    Frontal,
    /// **Mechanical** droplets — a breaking sea, a fall, a jet (§2.3, §5). This is a cause of its
    /// own and not a kind of fog: it needs no saturation, and naming it `Ground` because the air
    /// looks white is exactly the inference §1.1 forbids.
    Spray,
}

impl FogMechanism {
    /// Every mechanism, hand-written for the same reason [`crate::stages::ALL`] is: a cause added
    /// without a name should not compile silently.
    pub const ALL: [FogMechanism; 7] = [
        FogMechanism::None,
        FogMechanism::Ground,
        FogMechanism::Advection,
        FogMechanism::Steam,
        FogMechanism::Upslope,
        FogMechanism::Frontal,
        FogMechanism::Spray,
    ];

    /// The mechanism's name, for a HUD, a log or a census.
    pub const fn name(self) -> &'static str {
        match self {
            FogMechanism::None => "none",
            FogMechanism::Ground => "ground",
            FogMechanism::Advection => "advection",
            FogMechanism::Steam => "steam",
            FogMechanism::Upslope => "upslope",
            FogMechanism::Frontal => "frontal",
            FogMechanism::Spray => "spray",
        }
    }
}

/// A body of water's effect on the air just above it — the guide's §4, as two numbers.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WaterSurface {
    /// The water's **own** temperature, °C. Compared against the air's dew point for advection fog
    /// and against the air's temperature for steam fog; those are different comparisons and the
    /// guide's §4.1 is explicit that they are.
    pub temperature_c: f64,
    /// How exposed and broken the surface is, `[0, 1]`: a sheltered pond at `0`, an open sea at
    /// `1`. Scales spray, and nothing else — spray is the mechanical pathway and foam is its
    /// surface state.
    pub exposed: f64,
}

/// Everything the model needs that is not the parcel, so the physics can be driven from a test.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Conditions {
    /// The ground under the address, metres above the datum — where the layer starts and how deep
    /// it is.
    pub ground_m: f64,
    /// Wind speed, m/s. Drives **spray and mixing only** — never condensation.
    pub wind_m_s: f64,
    /// How strongly the address is sheltered, `[0, 1]`, from [`shelter`].
    pub pooling: f64,
    /// The wind's own component up the terrain's gradient, metres per metre, from
    /// [`weather::WeatherSample::upslope`]. Negative is leeward.
    pub upslope: f64,
    /// The synoptic ascent's weight, `[0, 1]` of [`weather`]'s own range — rain in transit.
    pub onset: f64,
    /// True while the sun is below the horizon and the surface is radiating rather than heated.
    pub night: bool,
    /// The water the address stands over or beside, if any.
    pub water: Option<WaterSurface>,
}

/// One **column's** near-surface loading, before it is given a height — the expensive half of
/// [`fog_at`], and the shape the guide's §3.1 sequence describes: how much water is in the air, and
/// how deep the layer that holds it is.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FogColumn {
    /// The parcel's temperature after its own night cooling, °C.
    pub temperature_c: f64,
    /// The dew point its vapour content implies, °C.
    pub dew_point_c: f64,
    /// `temperature_c − dew_point_c`, °C. At or below zero the air is saturated.
    pub saturation_deficit_c: f64,
    /// Droplets born of condensation, g/m³.
    pub condensation_g_m3: f64,
    /// Droplets born of mechanical breakup, g/m³ — **independent of saturation by construction**.
    pub spray_g_m3: f64,
    /// The two producers together, g/m³. The guide's budget without the terms a step would own.
    pub lwc_g_m3: f64,
    /// What made the droplets — the **largest** single contributor (§1.1, §2.3).
    pub mechanism: FogMechanism,
    /// The bottom of the layer, metres above the datum: the ground.
    pub layer_base_m: f64,
    /// The top of the layer, metres above the datum. Above it this column holds no fog.
    pub layer_top_m: f64,
    /// Whitecap coverage of the water surface, `[0, 1]`. A **surface state**: it is not droplets.
    pub foam_coverage: f64,
    /// The droplets a foam's bubbles eject, g/m³ — the only path from foam to air.
    pub burst_g_m3: f64,
    /// True when the droplets are liquid below 0 °C: supercooled fog, which §9.3 says is not ice.
    pub is_supercooled: bool,
    /// True only at or below [`ICE_FOG_C`], where the field declares ice particles instead.
    pub is_ice: bool,
    /// The settling speed of the spray drops, m/s — Stokes, at [`SPRAY_RADIUS_M`].
    pub spray_settling_m_s: f64,
}

/// What the near-surface field says at one **address** — a column's loading shaped by height.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FogSample {
    /// The address's own liquid water content, g/m³: the column's loading times its vertical
    /// profile. Zero above the layer's top — the guide's *"a thin fog layer that leaves treetops
    /// clear"* is this number being zero where the trees are.
    pub lwc_g_m3: f64,
    /// The address's own **mechanical** loading, g/m³ — spray and burst droplets, shaped by the
    /// same near-surface layer because that is where the drops are and where they settle out of.
    pub spray_g_m3: f64,
    /// Whether the address lies inside the layer at all.
    pub in_layer: bool,
    /// Its height within the layer, `[0, 1]`, or `0` outside it.
    pub through: f64,
    /// The parcel's temperature after its own night cooling, °C.
    pub temperature_c: f64,
    /// The dew point its vapour content implies, °C.
    pub dew_point_c: f64,
    /// `temperature_c − dew_point_c`, °C. At or below zero the air is saturated.
    pub saturation_deficit_c: f64,
    /// Extinction of the address's droplet field, per metre.
    pub extinction_per_m: f64,
    /// Koschmieder visibility of that extinction, metres.
    pub visibility_m: f64,
    /// What made the droplets at this address — the column's cause where it has loading, `None`
    /// above the layer.
    pub mechanism: FogMechanism,
    /// The bottom and top of the layer this address belongs to, metres above the datum.
    pub layer_base_m: f64,
    /// The top of that layer, metres above the datum.
    pub layer_top_m: f64,
}

impl FogSample {
    /// The guide's class of the suspension where the address is, from visibility alone: fog, mist,
    /// or nothing.
    pub fn class(&self) -> &'static str {
        if self.lwc_g_m3 <= 0.0 {
            "clear"
        } else if self.visibility_m <= FOG_VISIBILITY_M {
            "fog"
        } else if self.visibility_m <= MIST_VISIBILITY_M {
            "mist"
        } else {
            "clear"
        }
    }

    /// True when the address' air is saturated — the thermodynamic gate on **condensation only**.
    pub fn is_saturated(&self) -> bool {
        self.saturation_deficit_c <= 0.0
    }
}

/// The water surface's own temperature at a tick, °C — the annual mean, lagged by
/// [`WATER_LAG_TICKS`].
///
/// The sea holds its heat and gives it back late: this is [`climate::seasonal_offset_c`] read at a
/// tick an eighth of a year earlier, at sea level, with no lapse and no belt — because a water body
/// is one temperature across the whole of its surface, which is exactly what makes the air-water
/// **contrast** in §4 possible while land beside it swings freely.
pub fn water_surface_temperature_c(spec: &WorldSpec, tick: u64) -> f64 {
    let cfg = &spec.config;
    let lagged = tick.wrapping_sub(WATER_LAG_TICKS);
    cfg.base_temperature_c
        + climate::latitude_offset_c(cfg.origin_latitude_deg)
        + climate::seasonal_offset_c(spec, lagged)
}

/// The synoptic wind at an address, m/s — calm air plus the storm the world is having.
///
/// Read from [`weather::onset`] rather than a second wind field, and the direction that goes with
/// it is [`weather::wind_bearing_rad`]: a storm is wind and rain together, and this tree models
/// storms once.
pub fn synoptic_wind_m_s(spec: &WorldSpec, x: f64, z: f64, tick: u64) -> f64 {
    WIND_CALM_M_S + WIND_STORM_M_S * weather::onset(spec, x, z, tick).clamp(0.0, 1.0)
}

/// How sheltered an address is, `[0, 1]`: how far its ground sits **below** the ground around it.
///
/// The guide's §3.2, measured rather than assumed: *"cold air is denser than warmer air … and tends
/// to flow downslope under gravity. Valleys can therefore accumulate a humid, cool layer while
/// ridges remain clear."* The reach is [`SHELTER_REACH_M`] on four bearings, so a hollow a hundred
/// metres across reads sheltered and a ridge never does. Cost: four terrain samples, which is why a
/// caller reads this once per address-plane column and not once per voxel.
pub fn shelter(spec: &WorldSpec, x: f64, z: f64) -> f64 {
    let here = terrain::elevation_m(spec, x, z);
    let mut sum = 0.0;
    for (dx, dz) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        sum += terrain::elevation_m(spec, x + dx * SHELTER_REACH_M, z + dz * SHELTER_REACH_M);
    }
    let around = sum / 4.0;
    ((around - here) / SHELTER_DEPTH_M).clamp(0.0, 1.0)
}

/// Extinction of a droplet field, per **engine metre**: `β = 3·LWC / (2·ρ_w·r)`.
///
/// The geometric-optics form for a monodisperse population, which is the honest statement of a
/// single-radius model: §8.1 warns that fog droplets are Mie scatterers and that a size
/// *distribution* is what a fine model carries, so this is the Tier-2 approximation and the radius
/// is a declared effective one.
///
/// **The result is a per-length quantity and so passes through [`crate::scale::per_voxel`].** The
/// loading (g/m³) and the radius are intensive/particle quantities and are unchanged, but the
/// engine's metre is eight real metres, so the same loading extinguishes eight times more per
/// *engine* metre — which is what keeps the derived fog threshold (0.021 g/m³) landing on the
/// visibility the guide's convention names.
pub fn extinction_per_m(lwc_g_m3: f64, radius_m: f64) -> f64 {
    if lwc_g_m3 <= 0.0 || radius_m <= 0.0 {
        return 0.0;
    }
    let lwc_kg_m3 = lwc_g_m3 * 1.0e-3;
    scale::per_voxel(3.0 * lwc_kg_m3 / (2.0 * WATER_DENSITY_KG_M3 * radius_m))
}

/// Koschmieder visibility for an extinction, metres — `V ≈ 3.912 / β`, capped at
/// [`CLEAR_VISIBILITY_M`] where there is nothing to see through.
pub fn visibility_m(extinction_per_m: f64) -> f64 {
    if extinction_per_m <= 0.0 {
        return CLEAR_VISIBILITY_M;
    }
    (KOSCHMIEDER / extinction_per_m).min(CLEAR_VISIBILITY_M)
}

/// Stokes settling speed of a droplet, m/s — how fast a spray drop falls out of the air.
///
/// §8.2's transport consequence, in one line: *"large spray drops … higher inertia and settling
/// speed; more likely to fall or strike nearby surfaces."* It is `2ρ_w g r² / (9μ)` and it is the
/// reason a spray cloud is a near-field phenomenon while fog is not.
pub fn settling_velocity_m_s(radius_m: f64) -> f64 {
    2.0 * WATER_DENSITY_KG_M3 * crate::atmosphere::GRAVITY_M_S2 * radius_m * radius_m
        / (9.0 * AIR_VISCOSITY_PA_S)
}

/// Whitecap coverage of a water surface at a wind speed, `[0, 1]` — the guide's §5 foam state.
pub fn foam_coverage(wind_m_s: f64) -> f64 {
    ((wind_m_s - WHITECAP_WIND_M_S) / (FOAM_FULL_WIND_M_S - WHITECAP_WIND_M_S)).clamp(0.0, 1.0)
}

/// The droplets a **foam burst** ejects, g/m³.
///
/// The whole of the coupling the guide allows: foam is a surface state, and aerosol leaves it only
/// here. It is a fraction of what the same wind's spray would carry, scaled by the coverage — so a
/// flat calm with foam on it (an inlet, a weir) ejects nothing, which is the rule and the test.
pub fn burst_g_m3(foam_coverage: f64, wind_m_s: f64) -> f64 {
    if wind_m_s <= 0.0 {
        return 0.0;
    }
    FOAM_BURST_FRACTION * SPRAY_YIELD_G_M3_PER_M_S * foam_coverage.clamp(0.0, 1.0) * wind_m_s
}

/// The droplets **mechanical breakup** puts in the air, g/m³ — spray plus what a foam's bubbles
/// burst. Gated on the wind and the water's exposure, and on nothing else: §2.3 is explicit that
/// atomization *"can be present even when RH is below 100 %"*.
pub fn spray_g_m3(wind_m_s: f64, water: Option<WaterSurface>) -> f64 {
    let Some(water) = water else {
        return 0.0;
    };
    let exposed = water.exposed.clamp(0.0, 1.0);
    let spray = SPRAY_YIELD_G_M3_PER_M_S * (wind_m_s - WHITECAP_WIND_M_S).max(0.0) * exposed;
    let burst = burst_g_m3(foam_coverage(wind_m_s), wind_m_s) * exposed;
    spray + burst
}

/// The layer's own depth above the ground, metres — a skin where nothing pools, the full pool where
/// cold air collects.
///
/// §3.1's *"continued cooling can increase liquid water content and deepen fog"* and §3.2's
/// drainage, as one declared pair of depths scaled by [`Conditions::pooling`]. The water-driven
/// mechanisms (advection, steam) do **not** deepen it: a marine layer is thin whatever the air over
/// it is doing, which is why this reads the terrain and not the loading.
pub fn layer_depth_m(pooling: f64) -> f64 {
    LAYER_SKIN_DEPTH_M + (LAYER_POOL_DEPTH_M - LAYER_SKIN_DEPTH_M) * pooling.clamp(0.0, 1.0)
}

/// The whole near-surface field for one **column**, from the parcel and the conditions — **the
/// physics, with no world read in it**, so a test can drive any case the guide names.
///
/// The condensation terms are summed, because §10.2's budget is a sum; the mechanism reported is
/// the largest single one, because §1.1 forbids naming a mechanism from an appearance and the only
/// honest name is the cause that did the most work. A supersaturated case relaxes by condensing
/// (§10.1) and a subsaturated one condenses nothing at all — including over water, which cools or
/// moistens the air but cannot saturate air that is too dry for it (§4.3).
pub fn column_of(parcel: &SurfaceAir, conditions: &Conditions) -> FogColumn {
    let deficit = parcel.temperature_c - parcel.dew_point_c;
    let saturated = deficit <= 0.0;

    let mut ground = 0.0;
    let mut advection = 0.0;
    let mut steam = 0.0;
    let mut upslope = 0.0;
    let mut frontal = 0.0;

    if saturated {
        let excess = -deficit;
        // The address's own cooling is the parcel's; what a hollow adds is that the cold air stays
        // there, so the pooling weight scales it rather than adding a second temperature.
        ground = CONDENSATION_YIELD_G_M3_PER_C
            * excess
            * (1.0 + POOLING_WEIGHT * conditions.pooling.clamp(0.0, 1.0))
            * parcel.relative_humidity.clamp(0.0, 1.0);
    }

    if let Some(water) = conditions.water {
        let exposed = water.exposed.clamp(0.0, 1.0);
        // §4.1's first comparison: air cooled **to saturation** by water colder than its dew point.
        if water.temperature_c < parcel.dew_point_c {
            advection = CONDENSATION_YIELD_G_M3_PER_C
                * (parcel.dew_point_c - water.temperature_c)
                * exposed
                * parcel.relative_humidity.clamp(0.0, 1.0);
        }
        // And the second, which is a different pair of variables entirely: water **warmer than the
        // air** moistens it. The guide's §4.3 case — air too dry, or water and air alike in
        // temperature — gives neither, which is why this is not simply a humidity bonus.
        let contrast = water.temperature_c - parcel.temperature_c;
        if contrast > STEAM_CONTRAST_C {
            steam = CONDENSATION_YIELD_G_M3_PER_C * (contrast - STEAM_CONTRAST_C) * exposed;
        }
    }

    // §4's upslope case, read through the same lift the cloud model already uses: moist air rising
    // along the terrain cools at the dry rate until its own dew point is reached, and what is left
    // over is the excess it condenses.
    if conditions.upslope > 0.0 {
        let lift_m = conditions.upslope * OROGRAPHIC_LIFT_M;
        let after = deficit - clouds::DRY_LAPSE_C_PER_M * lift_m;
        if after < 0.0 {
            upslope =
                CONDENSATION_YIELD_G_M3_PER_C * (-after) * parcel.relative_humidity.clamp(0.0, 1.0);
        }
    }

    // §4's precipitation case: rain evaporating into air that is already close to saturation.
    if conditions.onset > weather::SILENT_ONSET && deficit < FRONTAL_SPREAD_C {
        frontal = CONDENSATION_YIELD_G_M3_PER_C
            * (FRONTAL_SPREAD_C - deficit)
            * conditions.onset.clamp(0.0, 1.0);
    }

    let condensation = (ground + advection + steam + upslope + frontal).min(MAX_LWC_G_M3);
    let spray = spray_g_m3(conditions.wind_m_s, conditions.water);
    let lwc = (condensation + spray).min(MAX_LWC_G_M3);

    let mut largest = 0.0f64;
    let mut mechanism = FogMechanism::None;
    for (name, value) in [
        (FogMechanism::Ground, ground),
        (FogMechanism::Advection, advection),
        (FogMechanism::Steam, steam),
        (FogMechanism::Upslope, upslope),
        (FogMechanism::Frontal, frontal),
    ] {
        if value > largest {
            largest = value;
            mechanism = name;
        }
    }
    if spray > largest {
        mechanism = FogMechanism::Spray;
    }

    FogColumn {
        temperature_c: parcel.temperature_c,
        dew_point_c: parcel.dew_point_c,
        saturation_deficit_c: deficit,
        condensation_g_m3: condensation,
        spray_g_m3: spray,
        lwc_g_m3: lwc,
        mechanism,
        layer_base_m: conditions.ground_m,
        layer_top_m: conditions.ground_m + layer_depth_m(conditions.pooling),
        foam_coverage: foam_coverage(conditions.wind_m_s),
        burst_g_m3: burst_g_m3(foam_coverage(conditions.wind_m_s), conditions.wind_m_s),
        is_supercooled: lwc > 0.0
            && parcel.temperature_c < 0.0
            && parcel.temperature_c > ICE_FOG_C,
        is_ice: lwc > 0.0 && parcel.temperature_c <= ICE_FOG_C,
        spray_settling_m_s: settling_velocity_m_s(SPRAY_RADIUS_M),
    }
}

/// The near-surface field at a **cubic address**, from the world's own state.
///
/// `y_m` is the address's own height above the datum, which is what makes this the cubic read and
/// not a column one: the layer has a bottom and a top ([`layer_depth_m`]), its loading is shaped
/// between them, and above the top the answer is *no droplets* — §3's *"thin fog layer that leaves
/// treetops clear"*, and the reason a valley's fog can be walked out of by climbing.
///
/// Cost: one parcel (which is the cloud model's own), four terrain samples for the shelter, one
/// upslope from [`weather`], and one onset — a per-address read, and the field plane's own cadence
/// is what decides how often it is asked.
///
/// Water is read as **the sea**, from the terrain's own line against sea level, and lakes are not:
/// that is the named limitation in this module's header, with [`crate::hydrology`] as its door.
pub fn fog_at(spec: &WorldSpec, x: f64, y_m: f64, z: f64, tick: u64) -> FogSample {
    fog_at_with(spec, x, y_m, z, tick, None)
}

/// [`fog_at`] with a **caller-known water body** — a lake, a river, a reservoir — added.
///
/// The guide's lakeside cases (§4) are the reason this entry point exists: the field cannot see
/// still water on its own yet, so a caller that already knows where the water is (the 256-level
/// lattice's surface state, say) says so and gets the advection and steam fog it implies.
pub fn fog_at_with(
    spec: &WorldSpec,
    x: f64,
    y_m: f64,
    z: f64,
    tick: u64,
    body: Option<WaterSurface>,
) -> FogSample {
    let ground_m = terrain::elevation_m(spec, x, z);
    let water = body.or_else(|| {
        let sea = ground_m <= spec.config.sea_level_m;
        sea.then_some(WaterSurface {
            temperature_c: water_surface_temperature_c(spec, tick),
            exposed: 1.0,
        })
    });
    let conditions = Conditions {
        ground_m,
        wind_m_s: synoptic_wind_m_s(spec, x, z, tick),
        pooling: shelter(spec, x, z),
        upslope: weather::sample(spec, x, z, tick).upslope,
        onset: weather::precipitation_now(spec, x, z, tick).onset,
        night: clouds::heating(spec, tick) <= 0.0,
        water,
    };
    let column = column_of(&clouds::surface_air(spec, x, z, tick), &conditions);

    // The vertical profile: zero at the layer's own base and top, deepest in the middle, which is
    // [`clouds::cloud_at`]'s shape because a fog layer and a cloud layer are the same object with
    // different feet.
    let depth = column.layer_top_m - column.layer_base_m;
    if depth <= 0.0 || y_m < column.layer_base_m || y_m >= column.layer_top_m {
        return FogSample {
            lwc_g_m3: 0.0,
            spray_g_m3: 0.0,
            in_layer: false,
            through: 0.0,
            temperature_c: column.temperature_c,
            dew_point_c: column.dew_point_c,
            saturation_deficit_c: column.saturation_deficit_c,
            extinction_per_m: 0.0,
            visibility_m: CLEAR_VISIBILITY_M,
            mechanism: FogMechanism::None,
            layer_base_m: column.layer_base_m,
            layer_top_m: column.layer_top_m,
        };
    }
    let through = (y_m - column.layer_base_m) / depth;
    let shape = (std::f64::consts::PI * through).sin();
    let lwc = column.lwc_g_m3 * shape;
    let extinction = extinction_per_m(lwc, DROPLET_RADIUS_M);
    FogSample {
        lwc_g_m3: lwc,
        spray_g_m3: column.spray_g_m3 * shape,
        in_layer: true,
        through,
        temperature_c: column.temperature_c,
        dew_point_c: column.dew_point_c,
        saturation_deficit_c: column.saturation_deficit_c,
        extinction_per_m: extinction,
        visibility_m: visibility_m(extinction),
        mechanism: if lwc > 0.0 {
            column.mechanism
        } else {
            FogMechanism::None
        },
        layer_base_m: column.layer_base_m,
        layer_top_m: column.layer_top_m,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2468, Config::default())
    }

    /// A parcel built by hand, so a test can state its own temperature and dew point.
    fn parcel_at(temperature_c: f64, dew_point_c: f64, relative_humidity: f64) -> SurfaceAir {
        SurfaceAir {
            elevation_m: 0.0,
            temperature_c,
            relative_humidity,
            dew_point_c,
            lcl_m: ((temperature_c - dew_point_c) / clouds::DRY_LAPSE_C_PER_M).max(0.0),
        }
    }

    fn land(wind_m_s: f64) -> Conditions {
        Conditions {
            ground_m: 0.0,
            wind_m_s,
            pooling: 0.0,
            upslope: 0.0,
            onset: 0.0,
            night: true,
            water: None,
        }
    }

    #[test]
    fn the_fog_threshold_is_the_convention_in_mass_units() {
        // The declared loading is *derived* from the convention, so this is the derivation checked:
        // at `FOG_LWC_G_M3` the visibility is at the fog line, and half of it is not fog at all.
        let at = visibility_m(extinction_per_m(FOG_LWC_G_M3, DROPLET_RADIUS_M));
        assert!(
            at <= FOG_VISIBILITY_M,
            "the declared fog loading gives {at:.0} m of visibility, which is not fog"
        );
        assert!(
            at > FOG_VISIBILITY_M * 0.8,
            "the declared fog loading is far past the threshold ({at:.0} m), so it is not the \
             threshold in mass units"
        );
        let half = visibility_m(extinction_per_m(FOG_LWC_G_M3 / 2.0, DROPLET_RADIUS_M));
        assert!(
            half > FOG_VISIBILITY_M,
            "half the fog loading still reads as fog ({half:.0} m)"
        );
    }

    #[test]
    fn visibility_comes_from_the_droplets_and_not_from_the_humidity() {
        // §10.1's rule as a measurement: humid air with no droplets is clear, and two addresses with
        // the same humidity and different droplet loadings do not agree. Without this the model
        // would be a humidity-to-fog slider, which is the mechanism the guide says not to skip.
        assert_eq!(
            visibility_m(extinction_per_m(0.0, DROPLET_RADIUS_M)),
            CLEAR_VISIBILITY_M
        );
        let clean = extinction_per_m(0.0, DROPLET_RADIUS_M);
        let loaded = extinction_per_m(FOG_LWC_G_M3, DROPLET_RADIUS_M);
        assert!(loaded > clean);
        // A saturated parcel over calm, unbroken water is moisture without droplets, and reads clear.
        let humid = parcel_at(10.0, 10.0, 1.0);
        let mut conditions = land(0.0);
        conditions.water = Some(WaterSurface {
            temperature_c: 10.0,
            exposed: 1.0,
        });
        let calm = column_of(&humid, &conditions);
        assert_eq!(calm.lwc_g_m3, 0.0, "{calm:?}");
    }

    #[test]
    fn a_wind_alone_makes_no_fog() {
        // The guide's own caution, as a test: *"do not generate sea fog solely from wind speed."*
        // A dry parcel on land in a gale condenses nothing and injects nothing.
        let dry = parcel_at(20.0, 4.0, 0.25);
        let mut conditions = land(WHITECAP_WIND_M_S * 2.0);
        conditions.pooling = 1.0;
        let gale = column_of(&dry, &conditions);
        assert_eq!(gale.lwc_g_m3, 0.0, "{gale:?}");
        assert_eq!(gale.mechanism, FogMechanism::None);
        // And even over water: wind on a dry parcel makes spray, never condensed fog.
        conditions.water = Some(WaterSurface {
            temperature_c: 20.0,
            exposed: 1.0,
        });
        let over_water = column_of(&dry, &conditions);
        assert_eq!(over_water.condensation_g_m3, 0.0);
        assert!(over_water.spray_g_m3 > 0.0, "{over_water:?}");
        assert_eq!(over_water.mechanism, FogMechanism::Spray);
    }

    #[test]
    fn spray_needs_no_saturation_and_fog_needs_a_water_that_cools_or_moistens() {
        // §2.3's separation, in one pair of cases. Spray is mechanical: it exists with the air dry
        // (asserted above) and it is the only droplet source that does not consult the deficit.
        let wet = parcel_at(12.0, 11.0, 0.95);
        let mut calm = land(0.0);
        calm.water = Some(WaterSurface {
            temperature_c: 12.0,
            exposed: 1.0,
        });
        assert_eq!(spray_g_m3(0.0, calm.water), 0.0, "a calm sea threw no spray");
        assert!(spray_g_m3(12.0, calm.water) > 0.0);
        assert!(spray_g_m3(12.0, None) == 0.0, "dry land threw spray");
        // Condensation, meanwhile, needs the water to be *colder* than the dew point: the same
        // saturated parcel over the same sea, one degree warmer, makes advection fog instead.
        let mut cold = calm;
        cold.water = Some(WaterSurface {
            temperature_c: 6.0,
            exposed: 1.0,
        });
        let advected = column_of(&wet, &cold);
        assert!(advected.condensation_g_m3 > 0.0, "{advected:?}");
        assert_eq!(advected.mechanism, FogMechanism::Advection);
        let warm = column_of(&wet, &calm);
        assert_eq!(
            warm.condensation_g_m3, 0.0,
            "a sea as warm as the air's dew point condensed anyway, so the comparison is not the \
             water's own temperature"
        );
    }

    #[test]
    fn a_cold_air_outbreak_over_warm_water_is_steam_and_not_more_humidity() {
        // §4's second comparison, which is a different pair of variables from the first: water
        // warmer than the *air* moistens it, and the guide's §4.3 case — water and air alike in
        // temperature — gives nothing at all.
        let cold_air = parcel_at(2.0, 1.0, 0.8);
        let mut conditions = land(2.0);
        conditions.water = Some(WaterSurface {
            temperature_c: 18.0,
            exposed: 1.0,
        });
        let steam = column_of(&cold_air, &conditions);
        assert!(steam.condensation_g_m3 > 0.0, "{steam:?}");
        assert_eq!(steam.mechanism, FogMechanism::Steam);
        // Below the declared contrast there is no steam, and the guide says why: neither cooling
        // nor moistening is strong enough.
        let mut tepid = conditions;
        tepid.water = Some(WaterSurface {
            temperature_c: 4.0,
            exposed: 1.0,
        });
        let nothing = column_of(&cold_air, &tepid);
        assert_eq!(nothing.condensation_g_m3, 0.0, "{nothing:?}");
    }

    #[test]
    fn foam_is_a_surface_state_and_only_a_burst_makes_aerosol_of_it() {
        // §5 and §10.3: bubble bursting *"is gas-in-liquid that emits aerosol only through an
        // explicit burst"*. The coverage is a state the water wears; the air only sees the
        // fraction that bursts, and a surface below the whitecap wind has neither.
        assert_eq!(foam_coverage(WHITECAP_WIND_M_S), 0.0);
        assert_eq!(foam_coverage(0.0), 0.0);
        assert_eq!(foam_coverage(FOAM_FULL_WIND_M_S), 1.0);
        let coverage = foam_coverage(14.0);
        assert!(coverage > 0.0 && coverage < 1.0, "coverage was {coverage}");
        assert_eq!(burst_g_m3(coverage, 0.0), 0.0, "foam threw droplets with no wind");
        let burst = burst_g_m3(coverage, 14.0);
        let spray = spray_g_m3(
            14.0,
            Some(WaterSurface {
                temperature_c: 10.0,
                exposed: 1.0,
            }),
        );
        assert!(
            burst < spray,
            "the burst is meant to be the smaller part of the spray"
        );
        assert!(burst > 0.0);
    }

    #[test]
    fn ground_fog_pools_at_night_and_the_same_parcel_evaporates_by_day() {
        let damp = parcel_at(4.0, 5.0, 0.9);
        let mut night = land(2.0);
        night.pooling = 1.0;
        let pooled = column_of(&damp, &night);
        assert!(pooled.condensation_g_m3 > 0.0, "{pooled:?}");
        assert_eq!(pooled.mechanism, FogMechanism::Ground);
        // The same column on a ridge pools less, which is §3.2's "ridges remain clear".
        let mut ridge = night;
        ridge.pooling = 0.0;
        let clear_ridge = column_of(&damp, &ridge);
        assert!(
            clear_ridge.condensation_g_m3 < pooled.condensation_g_m3,
            "a ridge held as much fog as the hollow below it"
        );
        // And by day the parcel is subsaturated, so the guide's §10.1 relaxation condenses nothing.
        let warm = parcel_at(12.0, 4.0, 0.6);
        let day = column_of(&warm, &night);
        assert_eq!(day.condensation_g_m3, 0.0, "{day:?}");
    }

    #[test]
    fn rain_in_cool_air_makes_fog_without_a_clear_night_sky() {
        // §4's precipitation row: *"rain evaporates into cooler air, raising vapor content toward
        // saturation … fog can develop even without clear nighttime skies."*
        let cool = parcel_at(9.0, 7.5, 0.85);
        let mut raining = land(3.0);
        raining.onset = weather::WET_ONSET + 0.05;
        let frontal = column_of(&cool, &raining);
        assert!(frontal.condensation_g_m3 > 0.0, "{frontal:?}");
        assert_eq!(frontal.mechanism, FogMechanism::Frontal);
        // The gate is the rain, not the night: the same parcel with no rain condenses nothing.
        raining.onset = 0.0;
        let dry_spell = column_of(&cool, &raining);
        assert_eq!(dry_spell.condensation_g_m3, 0.0, "{dry_spell:?}");
    }

    #[test]
    fn the_budget_the_field_reports_closes_over_its_two_producers() {
        // §10.2's sentence, as far as a stepless field can honestly carry it: what is in the air is
        // what condensation and mechanical breakup put there, and nothing else.
        let parcel = parcel_at(3.0, 4.0, 0.95);
        let mut conditions = land(15.0);
        conditions.pooling = 0.5;
        conditions.water = Some(WaterSurface {
            temperature_c: 2.0,
            exposed: 1.0,
        });
        conditions.onset = weather::WET_ONSET + 0.02;
        conditions.upslope = 0.05;
        let s = column_of(&parcel, &conditions);
        assert!(
            (s.lwc_g_m3 - (s.condensation_g_m3 + s.spray_g_m3)).abs() < 1e-12
                || s.lwc_g_m3 == MAX_LWC_G_M3,
            "{s:?}"
        );
        assert!(s.lwc_g_m3 > 0.0);
        assert!(s.lwc_g_m3 <= MAX_LWC_G_M3);
    }

    #[test]
    fn subzero_fog_is_supercooled_liquid_until_the_declared_ice_line() {
        // §9.3: *"do not treat every subzero fog as an ice-crystal field: supercooled liquid
        // droplets can persist below 0 C."*
        let freezing = parcel_at(-4.0, -3.0, 0.9);
        let s = column_of(&freezing, &land(1.0));
        assert!(s.lwc_g_m3 > 0.0, "{s:?}");
        assert!(s.is_supercooled && !s.is_ice, "{s:?}");
        let deep_cold = parcel_at(ICE_FOG_C - 1.0, ICE_FOG_C, 0.9);
        let ice = column_of(&deep_cold, &land(1.0));
        assert!(ice.is_ice && !ice.is_supercooled, "{ice:?}");
    }

    #[test]
    fn a_spray_drop_falls_out_far_faster_than_a_fog_droplet() {
        // §8.2's transport consequence: big drops settle, small ones stay.
        let fog_drop = settling_velocity_m_s(DROPLET_RADIUS_M);
        let spray_drop = settling_velocity_m_s(SPRAY_RADIUS_M);
        assert!(spray_drop > fog_drop * 10.0, "{spray_drop} vs {fog_drop}");
        assert!(
            fog_drop < 0.01,
            "a fog droplet settling at {fog_drop} m/s is a raindrop"
        );
    }

    #[test]
    fn the_layer_has_a_bottom_a_middle_and_a_top() {
        // The vertical structure itself, which is what makes this a cubic read: a pooling column's
        // layer is deeper than a ridge's, the loading is zero at the layer's own base and top,
        // deepest in the middle, and **nothing above the top** — §3's treetops standing clear.
        let s = spec();
        let mut at = (0.0f64, 0.0f64);
        let mut best = 0.0;
        for x in (-1200..1200).step_by(61) {
            for z in (-1200..1200).step_by(67) {
                let (xf, zf) = (f64::from(x), f64::from(z));
                let p = shelter(&s, xf, zf);
                if p > best {
                    best = p;
                    at = (xf, zf);
                }
            }
        }
        assert!(best > 0.0, "the sweep found no sheltered address at all");
        let ground = terrain::elevation_m(&s, at.0, at.1);
        let mut mid = 0.0f64;
        let mut samples = 0;
        let mut top = ground;
        for step in 0..=20 {
            let y = ground + step as f64;
            let sample = fog_at(&s, at.0, y, at.1, 0);
            top = sample.layer_top_m;
            if sample.in_layer {
                samples += 1;
                mid = mid.max(sample.lwc_g_m3);
                assert!(
                    (0.0..=MAX_LWC_G_M3).contains(&sample.lwc_g_m3),
                    "{sample:?}"
                );
                assert!(sample.through >= 0.0 && sample.through < 1.0);
                assert_eq!(
                    sample.extinction_per_m,
                    extinction_per_m(sample.lwc_g_m3, DROPLET_RADIUS_M)
                );
            }
        }
        assert!(samples > 0, "the pool at ({}, {}) had no layer", at.0, at.1);
        assert!(mid > 0.0, "the layer held no droplets anywhere in it");
        // Above the top, the answer is no droplets at all.
        let above = fog_at(&s, at.0, top + 1.0, at.1, 0);
        assert!(!above.in_layer && above.lwc_g_m3 == 0.0, "{above:?}");
        assert_eq!(above.visibility_m, CLEAR_VISIBILITY_M);
        assert_eq!(above.mechanism, FogMechanism::None);
        // And the base of the layer is the ground it stands on.
        let base = fog_at(&s, at.0, ground, at.1, 0);
        assert!(base.layer_base_m <= ground + 1e-9);
        assert!(base.layer_top_m > base.layer_base_m);
    }

    #[test]
    fn the_shelter_is_a_hollow_measure_and_the_ridge_is_not() {
        // §3.2 over the real world: shelter is a field, in range, deterministic, and it reads
        // hollows rather than hills — measured as an ordering between the low and high ground it
        // finds, not asserted as a shape.
        let s = spec();
        let mut columns = Vec::new();
        for x in (-1500..1500).step_by(53) {
            for z in (-1500..1500).step_by(59) {
                let (xf, zf) = (f64::from(x), f64::from(z));
                let p = shelter(&s, xf, zf);
                assert!((0.0..=1.0).contains(&p), "shelter was {p}");
                assert_eq!(p, shelter(&s, xf, zf));
                let above_sea = terrain::elevation_m(&s, xf, zf) - s.config.sea_level_m;
                columns.push((above_sea, p));
            }
        }
        // The split is by the sweep's **own** quartiles rather than by an absolute height: this
        // world's relief is 40 m, so "100 m above the sea" is a height it never reaches. The claim
        // the guide makes is comparative — hollows hold the cold air and ridges do not — so the
        // comparison is between its lowest and highest ground.
        columns.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("finite elevations"));
        let n = columns.len();
        let mean = |slice: &[(f64, f64)]| {
            slice.iter().map(|(_, p)| p).sum::<f64>() / slice.len() as f64
        };
        let low = mean(&columns[..n / 4]);
        let high = mean(&columns[3 * n / 4..]);
        assert!(n > 100, "the sweep found {n} columns");
        assert!(
            low > high,
            "the lowest quarter sheltered {low:.3} and the highest quarter {high:.3}"
        );
    }

    #[test]
    fn the_world_read_is_in_range_and_names_a_mechanism_for_what_it_finds() {
        let s = spec();
        let mut seen = std::collections::BTreeSet::new();
        for tick in [0u64, climate::TICKS_PER_DAY / 2, climate::YEAR_TICKS / 4] {
            for x in (-2000..2000).step_by(211) {
                for z in (-2000..2000).step_by(223) {
                    let ground = terrain::elevation_m(&s, f64::from(x), f64::from(z));
                    // Just above the ground, in **real** metres put through the world's scale:
                    // the layer this samples is a few engine metres deep, so a bare `2.0` here
                    // would now stand above it and the sweep would see nothing at all.
                    let sample = fog_at(&s, f64::from(x), ground + scale::voxels(2.0), f64::from(z), tick);
                    assert!((0.0..=MAX_LWC_G_M3).contains(&sample.lwc_g_m3), "{sample:?}");
                    assert!(sample.visibility_m >= 1.0 && sample.visibility_m <= CLEAR_VISIBILITY_M);
                    assert_eq!(sample.mechanism == FogMechanism::None, sample.lwc_g_m3 == 0.0);
                    if sample.in_layer && sample.lwc_g_m3 > 0.0 {
                        seen.insert(sample.mechanism);
                    }
                }
            }
        }
        assert!(
            seen.contains(&FogMechanism::Ground) || seen.contains(&FogMechanism::Advection),
            "a day in this world never made a near-surface droplet: {seen:?}"
        );
    }
}
