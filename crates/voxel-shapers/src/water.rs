//! The water ledger: **three reservoirs, closed**.
//!
//! Ruled (C-94.0.4): the ledger carries **what this world actually has** — the
//! ocean, surface water and groundwater — and declares snowpack, soil moisture and
//! atmospheric vapour **out of scope**, because the world's atmosphere is a sky
//! shader and none of the water cycle is drawn: no cloud, no rain, no hydrology.
//! There is no seam to refuse through, because nothing is being withheld.
//!
//! # Why three is the closed set
//!
//! Six reservoirs were ruled in the abstract and three of them had **no source in
//! this tree at all** (C-94.6). A ledger that carried them anyway would have had to
//! invent a snowpack, and a ledger that reported them as zero would be the exact
//! failure this codebase refuses — a missing row is a visible blocker, never an
//! implicit zero. Three reservoirs with no invented storage is the honest set.
//!
//! # And why it closes *exactly*
//!
//! With no atmosphere and no soil, **every flux except one is internal**. Rain
//! enters; surface water runs off into the ocean; surface water infiltrates to
//! groundwater and comes back as baseflow. Runoff, infiltration and baseflow move
//! water between reservoirs and cancel. So:
//!
//! ```text
//! Σ storage_after − Σ storage_before == precipitation_in
//! ```
//!
//! and that is checkable rather than asserted. It is also the reason there is no
//! evaporation term here: **there is nowhere for evaporated water to go**, so a
//! ledger that subtracted it would be destroying mass, and refusing to destroy mass
//! is the whole of this crate's discipline.
//!
//! # Salinity is tracked as a class, not as a label
//!
//! The ocean is **saline**; surface water and groundwater are **fresh**. That is a
//! label on a reservoir and it is not enough, because runoff carries *fresh* water
//! into a *saline* reservoir, and a ledger that tracked one number per reservoir
//! could not say whether the salt or the fresh water was conserved. So each
//! reservoir holds a [`Contents`] split by class, and closure is checked **per
//! class** as well as in total:
//!
//! ```text
//! saline_after − saline_before == runoff
//! fresh_after  − fresh_before  == precipitation − runoff
//! ```
//!
//! # Storage is derived, not accumulated — and the seam is named
//!
//! All three reservoirs are **derived from terrain on every tick**: the ocean is
//! the depth below sea level, surface water is the priority-flood depth
//! `hydrology` already computes, groundwater is the saturated thickness below the
//! water table. Nothing is stored between ticks, so a ledger is a pure function of
//! `(tick, column)` — the same purity the tick ladder claims, and the same reason a
//! skipped tick gives the same world.
//!
//! The cost of that choice is named rather than hidden: **a lake appears and
//! vanishes with the elevation that made it** rather than filling over time. Making
//! water stateful means adding hysteresis, which is physically right and is a
//! behavioural change to what a voxel column *is*. That is the seam, and it is
//! where a later accumulator would attach.
//!
//! # Sources
//!
//! * O'Callaghan & Mark 1984 — D8 routing; Barnes, Lehman & Mulla 2014 —
//!   priority-flood. Both are already cited and used by [`crate::hydrology`], and
//!   this module borrows nothing from them but the *shape* of their algorithms.
//! * SUTRA 2.0 (Zheng & Wu) — the phase densities used by [`Phase::density_kg_m3`].
//!   **Reference, never rules** (C-48/C-49): the numbers are declared here and
//!   cited, and nothing in this tree is obliged to agree with them.
//!
//! The ratios in [`Ratios`] are **declared choices**, each with its reason, in the
//! same spirit as `hydrology::LAKE_EPSILON_M` and `DEFAULT_CHANNEL_THRESHOLD`. None
//! of them is measured, and none of them pretends to be.

use crate::hydrology::Region;
use crate::spec::WorldSpec;
use crate::terrain;
use crate::weather;
use voxel_model::{Refusal, RefusalKind, Refused};

/// The edge of one ledger cell, in metres. A square column of this edge.
pub const CELL_M: f64 = 1.0;

/// How deep below the water table the ledger counts as saturated, in metres.
///
/// **A declared depth, and it has to be one.** "Everything below the water table
/// is saturated" is true of a half-space and useless of a ledger: it would make
/// groundwater volume unbounded and the column's share unknowable. Thirty metres is
/// a declared reading depth, chosen so the number is *about* the saturated zone
/// rather than the whole crust, and it is named so a later edit has to say it
/// changed it.
pub const GROUNDWATER_CAP_M: f64 = 30.0;

/// Days in a year, for turning the clock into a rate.
///
/// Declared rather than taken from `f64::days_in_year`, which does not exist, and
/// **365.25** rather than 365 because a world whose seasons come from a seasonal
/// term is already standing in for the Julian year.
pub const DAYS_PER_YEAR: f64 = 365.25;

/// How many base ticks one world year is.
///
/// **Derived from the clock, not declared**, so the ledger's rain rate cannot drift
/// away from [`crate::climate::TICKS_PER_DAY`]: change the day and the rain
/// follows. This was originally a bare `0.0001` years per tick, which was wrong by
/// about **876×** — it delivered 876 years of rainfall per world year, and the
/// ratio validator refused it, which is the only reason it was caught.
pub fn year_in_ticks() -> f64 {
    crate::climate::TICKS_PER_DAY as f64 * DAYS_PER_YEAR
}

/// How far below sea level the water table sits, in metres.
///
/// Read from [`crate::caves::CaveParams::water_table_above_sea_m`] rather than
/// restated, so the ledger and the cave carver cannot disagree about where the
/// water is.
pub fn water_table_m(spec: &WorldSpec) -> f64 {
    spec.config.sea_level_m + crate::caves::CaveParams::default().water_table_above_sea_m
}

/// The three reservoirs this world has.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Reservoir {
    /// Below sea level. **Saline.**
    Ocean,
    /// Standing and flowing fresh water: lakes, flood fill, channels.
    Surface,
    /// Saturated rock below the water table. **Fresh.**
    Groundwater,
}

impl Reservoir {
    /// Every reservoir, hand-written for the same reason `stages::ALL` is.
    pub const ALL: [Reservoir; 3] = [Reservoir::Ocean, Reservoir::Surface, Reservoir::Groundwater];

    pub const fn name(self) -> &'static str {
        match self {
            Reservoir::Ocean => "ocean",
            Reservoir::Surface => "surface",
            Reservoir::Groundwater => "groundwater",
        }
    }

    pub const fn index(self) -> usize {
        self as usize
    }

    /// Which class this reservoir's own water is in.
    ///
    /// A statement about where the reservoir *is*, not about what has flowed into
    /// it: runoff makes the ocean's fresh fraction non-zero, and that is measured
    /// rather than assumed away.
    pub const fn native_salinity(self) -> Salinity {
        match self {
            Reservoir::Ocean => Salinity::Saline,
            Reservoir::Surface | Reservoir::Groundwater => Salinity::Fresh,
        }
    }
}

/// Which class a volume of water is in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Salinity {
    Fresh,
    Saline,
}

/// The physical phase a reservoir's water is in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Phase {
    Liquid,
    /// Ice. This world has no snowpack, so solid water can only appear as lake or
    /// sea ice — and nothing in the tree computes it yet, which is why
    /// [`Storage::phase`] is **declared on the storage rather than derived**, and
    /// why the ledger's closure is over volume and not over mass.
    Solid,
}

impl Phase {
    /// Bulk density, in kg/m³.
    ///
    /// Liquid **1000** at 4 °C; solid **917** at 0 °C — the SUTRA pair. Declared
    /// here and cited, and *reference rather than rules* (C-48/C-49).
    pub const fn density_kg_m3(self) -> f64 {
        match self {
            Phase::Liquid => 1_000.0,
            Phase::Solid => 917.0,
        }
    }
}

/// A reservoir's contents, **split by class**.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Contents {
    pub fresh_m3: f64,
    pub saline_m3: f64,
}

impl Contents {
    pub const ZERO: Contents = Contents { fresh_m3: 0.0, saline_m3: 0.0 };

    pub fn total_m3(self) -> f64 {
        self.fresh_m3 + self.saline_m3
    }

    pub fn of(self, class: Salinity) -> f64 {
        match class {
            Salinity::Fresh => self.fresh_m3,
            Salinity::Saline => self.saline_m3,
        }
    }

    /// Move `volume_m3` out of `class`, refusing rather than going negative.
    ///
    /// A negative reservoir is not a small number, it is a wrong one, so this
    /// refuses instead of clamping: a clamped withdrawal is water invented from
    /// nothing, and an invented reservoir is exactly how a "conserving" ledger
    /// starts conserving the wrong thing.
    fn withdraw(&mut self, class: Salinity, volume_m3: f64, at: &str) -> Refused<()> {
        let available = self.of(class);
        if volume_m3 > available {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "{at}: withdrawing {volume_m3} m3 of {} water from a reservoir holding \
                     {available} m3 — a negative reservoir is a wrong reservoir",
                    match class {
                        Salinity::Fresh => "fresh",
                        Salinity::Saline => "saline",
                    }
                ),
            ));
        }
        match class {
            Salinity::Fresh => self.fresh_m3 -= volume_m3,
            Salinity::Saline => self.saline_m3 -= volume_m3,
        }
        Ok(())
    }

    fn deposit(&mut self, class: Salinity, volume_m3: f64) {
        match class {
            Salinity::Fresh => self.fresh_m3 += volume_m3,
            Salinity::Saline => self.saline_m3 += volume_m3,
        }
    }
}

/// One reservoir at one tick.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Storage {
    pub reservoir: Reservoir,
    /// **Declared**, not derived — see [`Phase`]. Volume is what the ledger
    /// closes over, and the phase is what would make a volume a mass.
    pub phase: Phase,
    pub contents: Contents,
}

impl Storage {
    pub fn total_m3(&self) -> f64 {
        self.contents.total_m3()
    }

    pub fn of(&self, class: Salinity) -> f64 {
        self.contents.of(class)
    }
}

/// Which process moved the water.
///
/// Named because **each process carries its own declared ratio**: the reference's
/// open decision was whether one world-time clock serves every process, and the
/// ruling was that it does not — snow answers on a seasonal clock, groundwater on
/// an annual one, weather per tick. With three reservoirs and no snow, that becomes
/// four independent declared fractions rather than one, and they are listed here so
/// the count cannot quietly change.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Process {
    /// Rain falling. The ledger's **only** source.
    Precipitation,
    /// Surface water reaching the sea.
    Runoff,
    /// Surface water soaking down.
    Infiltration,
    /// Groundwater coming back out.
    Baseflow,
}

impl Process {
    pub const ALL: [Process; 4] =
        [Process::Precipitation, Process::Runoff, Process::Infiltration, Process::Baseflow];

    pub const fn name(self) -> &'static str {
        match self {
            Process::Precipitation => "precipitation",
            Process::Runoff => "runoff",
            Process::Infiltration => "infiltration",
            Process::Baseflow => "baseflow",
        }
    }
}

/// One transfer between two reservoirs, with the process that caused it.
///
/// A **source** transfer — [`Process::Precipitation`] — names its destination in
/// both `from` and `to`. There is no reservoir outside the ledger to name as the
/// origin, and inventing an `Option` here would let a caller pass `None` and get a
/// transfer that appears from nowhere. Repeating the destination says plainly that
/// the water came from outside the set.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Transfer {
    pub from: Reservoir,
    pub to: Reservoir,
    pub class: Salinity,
    pub volume_m3: f64,
    pub process: Process,
}

/// The four declared ratios. **Every one is a declared choice.**
///
/// * `precipitation_years_per_tick` — how many years of rainfall one tick delivers.
///   **Not a free number**: it is `1 / year_in_ticks()` by default, and a test holds
///   it to the clock, because a literal here is a rate that silently disagrees with
///   `TICKS_PER_DAY`.
/// * `runoff_per_tick` — the share of standing surface water that reaches the sea
///   in one tick. Low, because a lake does not empty between frames.
/// * `infiltration_per_tick` — the share of surface water that soaks down.
/// * `baseflow_per_tick` — the share of groundwater that returns to the surface.
///
/// Each is a fraction of **its own** reservoir per tick, not of a shared pool, and
/// each is refused outside `0.0..=1.0` — a ratio above 1 is not a fast process, it
/// is water appearing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ratios {
    pub precipitation_years_per_tick: f64,
    pub runoff_per_tick: f64,
    pub infiltration_per_tick: f64,
    pub baseflow_per_tick: f64,
}

impl Default for Ratios {
    /// The **declared** set. Named here as the `Default` so that a caller who
    /// writes `Ratios::default()` gets the declared numbers and not a zeroed
    /// struct — a zeroed ratio set would be a ledger where nothing ever moves, and
    /// it would look closed.
    fn default() -> Self {
        Ratios {
            precipitation_years_per_tick: 1.0 / year_in_ticks(),
            runoff_per_tick: 0.002,
            infiltration_per_tick: 0.001,
            baseflow_per_tick: 0.0005,
        }
    }
}

impl Ratios {
    /// The declared set, validated.
    pub fn declared() -> Refused<Self> {
        Ratios::default().validated()
    }

    /// Refuse a ratio outside its declared range, **by name**.
    ///
    /// Precipitation is bounded only by being non-negative and finite: whether it
    /// is the *right* rate is a different question from whether it is a rate at
    /// all, and mixing the two would let a validator vouch for a number chosen by
    /// hand. [`tests::the_declared_rain_rate_is_the_clock_s_own`] is what holds it
    /// to the clock.
    pub fn validated(self) -> Refused<Self> {
        let checks: [(Process, f64, f64); 4] = [
            (Process::Precipitation, self.precipitation_years_per_tick, f64::MAX),
            (Process::Runoff, self.runoff_per_tick, 1.0),
            (Process::Infiltration, self.infiltration_per_tick, 1.0),
            (Process::Baseflow, self.baseflow_per_tick, 1.0),
        ];
        for (process, value, max) in checks {
            if !value.is_finite() || value < 0.0 || value > max {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "{}'s ratio is {value}, outside the declared 0.0..={max}",
                        process.name()
                    ),
                ));
            }
        }
        Ok(self)
    }
}

/// One column's ledger at one tick.
#[derive(Clone, PartialEq, Debug)]
pub struct Ledger {
    pub tick: u64,
    pub column: (i32, i32),
    /// Storage **derived from terrain**: the "before".
    pub derived: [Storage; 3],
    /// Derived storage plus this tick's net fluxes: the "after".
    pub storage: [Storage; 3],
    pub transfers: Vec<Transfer>,
}

impl Ledger {
    /// Build a column's ledger at `tick` and advance it by one tick's fluxes.
    ///
    /// `region` must be a filled [`hydrology::Region`] centred on the column: the
    /// surface-water figure is the priority-flood depth, and that is the only
    /// standing-water measure this tree has.
    pub fn at(
        spec: &WorldSpec,
        x: f64,
        z: f64,
        tick: u64,
        region: &Region,
        ratios: Ratios,
    ) -> Refused<Self> {
        let ratios = ratios.validated()?;
        let column = (x as i32, z as i32);
        let derived = derived_storage(spec, x, z, region);

        let mut storage = derived;
        let mut transfers = Vec::new();

        // 1. Precipitation: the only source. mm/year -> m/year -> m this tick.
        let rate_mm = weather::precipitation_now(spec, x, z, tick).rate_mm_per_year;
        let rain_m = (rate_mm / 1_000.0) * ratios.precipitation_years_per_tick;
        let area = CELL_M * CELL_M;
        let rain_m3 = rain_m * area;
        if rain_m3 > 0.0 {
            storage[Reservoir::Surface.index()].contents.deposit(Salinity::Fresh, rain_m3);
            transfers.push(Transfer {
                from: Reservoir::Surface,
                to: Reservoir::Surface,
                class: Salinity::Fresh,
                volume_m3: rain_m3,
                process: Process::Precipitation,
            });
        }

        // 2. Runoff, 3. infiltration, 4. baseflow: **all internal**, which is why
        //    they cancel in the closure identity.
        internal(
            &mut storage,
            &mut transfers,
            Reservoir::Surface,
            Reservoir::Ocean,
            Salinity::Fresh,
            ratios.runoff_per_tick,
            Process::Runoff,
        )?;
        internal(
            &mut storage,
            &mut transfers,
            Reservoir::Surface,
            Reservoir::Groundwater,
            Salinity::Fresh,
            ratios.infiltration_per_tick,
            Process::Infiltration,
        )?;
        internal(
            &mut storage,
            &mut transfers,
            Reservoir::Groundwater,
            Reservoir::Surface,
            Salinity::Fresh,
            ratios.baseflow_per_tick,
            Process::Baseflow,
        )?;

        Ok(Ledger { tick, column, derived, storage, transfers })
    }

    /// Total volume in all three reservoirs.
    pub fn total_m3(storage: &[Storage; 3]) -> f64 {
        storage.iter().map(|s| s.total_m3()).sum()
    }

    pub fn derived_total_m3(&self) -> f64 {
        Self::total_m3(&self.derived)
    }

    pub fn storage_total_m3(&self) -> f64 {
        Self::total_m3(&self.storage)
    }

    /// One class's total across all three reservoirs.
    pub fn class_total_m3(storage: &[Storage; 3], class: Salinity) -> f64 {
        storage.iter().map(|s| s.of(class)).sum()
    }

    /// The volume that entered from outside: precipitation only.
    pub fn source_m3(&self) -> f64 {
        self.transfers
            .iter()
            .filter(|t| t.process == Process::Precipitation)
            .map(|t| t.volume_m3)
            .sum()
    }

    /// The volume that left the reservoirs: runoff into the ocean, counted by the
    /// **fresh** class, because that is the class it takes.
    ///
    /// This is a *report*, not a term in the closure identity: runoff does not leave
    /// the ledger, it changes class. It is exposed because a reader who cannot see
    /// where the fresh water went has no way to tell a conserving ledger from a
    /// diluting one.
    pub fn runoff_m3(&self) -> f64 {
        self.transfers
            .iter()
            .filter(|t| t.process == Process::Runoff)
            .map(|t| t.volume_m3)
            .sum()
    }

    /// **The closure error, in m³.** Zero is a closed ledger.
    pub fn closure_error_m3(&self) -> f64 {
        self.storage_total_m3() - self.derived_total_m3() - self.source_m3()
    }

    /// The closure error of one class. Zero is a closed ledger **per class**, which
    /// is the stronger claim and the one that catches salt being created.
    ///
    /// **A class is a global property, not a per-reservoir label**, and both halves
    /// of that sentence were got wrong here before the measurement caught them.
    /// Runoff carries *fresh* water to the ocean and the salt does not move, so the
    /// saline class is **invariant**. And because the fresh water arriving at the
    /// ocean joins the ocean's *fresh* bucket, it is still fresh — so runoff does
    /// not leave the fresh class either. The only thing that changes a class is
    /// **precipitation**, which enters from outside the whole set.
    ///
    /// So: `fresh_after − fresh_before == source`, `saline_after == saline_before`,
    /// and their sum is the total identity above. An earlier version of this
    /// subtracted the runoff from the fresh class, and the sweep reported a fresh
    /// error of `8.9e-4` while the total was exact at `2.2e-16` — which is only
    /// possible if one class identity is wrong, and it was.
    pub fn class_closure_error_m3(&self, class: Salinity) -> f64 {
        let before = Self::class_total_m3(&self.derived, class);
        let after = Self::class_total_m3(&self.storage, class);
        match class {
            // Fresh: rain in. Runoff is fresh water moving between two places that
            // are both fresh, so it is not an exit from the class.
            Salinity::Fresh => (after - before) - self.source_m3(),
            // Saline: unchanged. Nothing here makes or moves salt.
            Salinity::Saline => after - before,
        }
    }

    /// **The mixing error, in m³**: how much the ocean's *fresh* content grew by,
    /// minus the runoff that arrived.
    ///
    /// This is the claim that is actually worth making about salinity, and it is a
    /// different one from the class closures: it says the fresh water that reached
    /// the sea is where it went, rather than that the salt did not change.
    /// Storage on one side, the recorded transfers on the other, so it is a check
    /// rather than a restatement.
    pub fn mixing_error_m3(&self) -> f64 {
        let ocean = Reservoir::Ocean.index();
        (self.storage[ocean].of(Salinity::Fresh) - self.derived[ocean].of(Salinity::Fresh))
            - self.runoff_m3()
    }
}

/// Move `ratio` of one reservoir's fresh water into another, refusing if it would
/// go negative rather than clamping.
fn internal(
    storage: &mut [Storage; 3],
    transfers: &mut Vec<Transfer>,
    from: Reservoir,
    to: Reservoir,
    class: Salinity,
    ratio: f64,
    process: Process,
) -> Refused<()> {
    let available = storage[from.index()].contents.of(class);
    let volume = available * ratio;
    if volume <= 0.0 {
        return Ok(());
    }
    storage[from.index()].contents.withdraw(class, volume, process.name())?;
    storage[to.index()].contents.deposit(class, volume);
    transfers.push(Transfer { from, to, class, volume_m3: volume, process });
    Ok(())
}

/// Storage derived from terrain, before any flux.
///
/// **Nothing here is stored between ticks.** That is what makes a ledger a pure
/// function of `(tick, column)` and a skipped tick harmless — and it is also what
/// makes a lake vanish with its terrain, which is the named cost.
pub fn derived_storage(spec: &WorldSpec, x: f64, z: f64, region: &Region) -> [Storage; 3] {
    let elevation = terrain::elevation_m(spec, x, z);
    let sea = spec.config.sea_level_m;
    let area = CELL_M * CELL_M;

    // Ocean: the depth below sea level. A column at or above sea level has none.
    let ocean_m3 = ((sea - elevation).max(0.0)) * area;

    // Surface: the priority-flood depth at the region's centre.
    let (ci, cj) = region.centre();
    let surface_m3 = region.water_depth_m(ci, cj) * area;

    // Groundwater: saturated thickness below the water table, capped.
    let saturated = (water_table_m(spec) - elevation).clamp(0.0, GROUNDWATER_CAP_M);

    [
        Storage {
            reservoir: Reservoir::Ocean,
            phase: Phase::Liquid,
            contents: Contents {
                fresh_m3: 0.0,
                saline_m3: if ocean_m3 > 0.0 { ocean_m3 } else { 0.0 },
            },
        },
        Storage {
            reservoir: Reservoir::Surface,
            phase: Phase::Liquid,
            contents: Contents {
                fresh_m3: if surface_m3 > 0.0 { surface_m3 } else { 0.0 },
                saline_m3: 0.0,
            },
        },
        Storage {
            reservoir: Reservoir::Groundwater,
            phase: Phase::Liquid,
            contents: Contents {
                fresh_m3: saturated * area,
                saline_m3: 0.0,
            },
        },
    ]
}

/// A column's ledger with a default region, for callers that do not build one.
///
/// The region is the honest cost of the honest model (see
/// [`crate::hydrology::sample`]): flow is not local, so a standing-water depth
/// needs a patch of terrain, and a caller that shrinks the radius is choosing a
/// smaller watershed rather than a cheaper approximation of the same one.
pub fn ledger_at(spec: &WorldSpec, x: f64, z: f64, tick: u64, radius: usize) -> Refused<Ledger> {
    let mut region = Region::around(spec, (x, z), radius, CELL_M);
    region.fill_depressions();
    Ledger::at(spec, x, z, tick, &region, Ratios::declared()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn spec() -> WorldSpec {
        WorldSpec::new(2024, Config::default())
    }

    fn ledger(x: f64, z: f64, tick: u64) -> Ledger {
        ledger_at(&spec(), x, z, tick, 10).expect("the declared ratios are valid")
    }

    /// Closure is exact up to floating-point, so the tolerance is relative and
    /// stated rather than absolute.
    const TOL: f64 = 1e-9;

    fn assert_closed(l: &Ledger) {
        let scale = l.storage_total_m3().max(1.0);
        let err = l.closure_error_m3().abs();
        assert!(err <= TOL * scale, "ledger at {:?} closed with error {err} m3", l.column);
        for class in [Salinity::Fresh, Salinity::Saline] {
            let e = l.class_closure_error_m3(class).abs();
            assert!(
                e <= TOL * scale,
                "ledger at {:?} closed its {:?} class with error {e} m3",
                l.column,
                class
            );
        }
    }

    #[test]
    fn the_ledger_closes_on_every_column_of_a_real_transect() {
        // The claim the whole module exists for, over the world rather than over
        // a hand-built case: whatever the terrain does, inflow equals the change.
        let mut checked = 0;
        let mut with_water = 0;
        for i in -40..40 {
            let x = i as f64 * 250.0;
            for z in [-1000.0, 0.0, 1000.0] {
                let l = ledger(x, z, 4);
                assert_closed(&l);
                checked += 1;
                if l.derived_total_m3() > 0.0 {
                    with_water += 1;
                }
            }
        }
        assert_eq!(checked, 240);
        assert!(with_water > checked / 2, "only {with_water} of {checked} columns held water");
    }

    #[test]
    fn a_ledger_closes_at_every_tick_the_ladder_reaches() {
        // Closure is not a property of one lucky tick. Tick 0 is due for every
        // cadence, so it is the fullest the ledger ever gets.
        for tick in [0u64, 1, 4, 20, 24_000, 2_304_000] {
            assert_closed(&ledger(500.0, -300.0, tick));
        }
    }

    #[test]
    fn no_reservoir_is_ever_negative() {
        for i in -20..20 {
            let l = ledger(i as f64 * 400.0, 700.0, 0);
            for s in &l.storage {
                assert!(s.total_m3() >= 0.0, "{:?} went negative at {:?}", s.reservoir, l.column);
                assert!(s.contents.fresh_m3.is_finite() && s.contents.saline_m3.is_finite());
            }
        }
    }

    #[test]
    fn the_ocean_is_saline_and_the_fresh_water_is_fresh() {
        // Measured on a column that is genuinely below sea level, because a column
        // above it has no ocean and would make this vacuous.
        let s = spec();
        let mut found = None;
        'search: for x in (-6000..6000).step_by(137) {
            for z in (-6000..6000).step_by(149) {
                if terrain::elevation_m(&s, x as f64 + 0.5, z as f64 + 0.5)
                    < s.config.sea_level_m - 2.0
                {
                    found = Some((x as f64, z as f64));
                    break 'search;
                }
            }
        }
        let (x, z) = found.expect("this world has an ocean");
        let l = ledger_at(&s, x, z, 0, 10).unwrap();

        let ocean = l.storage[Reservoir::Ocean.index()];
        assert!(ocean.of(Salinity::Saline) > 0.0, "the ocean holds no salt water");
        // **The ocean is not purely saline, and an earlier version of this test
        // asserted it was.** Runoff carries fresh water into it — measured at
        // 8.4e-12 m3 on the column this search found — so the honest claim is that
        // the fresh fraction is negligible and that it *equals the runoff that
        // arrived*, not that it is zero. A test demanding exactly zero would have
        // been demanding that runoff stop working at the coast.
        assert!(
            l.mixing_error_m3().abs() < 1e-15,
            "the ocean's fresh water and the runoff that reached it disagree by {}",
            l.mixing_error_m3()
        );
        assert!(
            ocean.of(Salinity::Fresh) / ocean.total_m3() < 1e-6,
            "the ocean column is {} fresh, which is not negligible",
            ocean.of(Salinity::Fresh) / ocean.total_m3()
        );
        // And the salt itself did not move: the saline volume is exactly what the
        // terrain put there.
        assert_eq!(
            ocean.of(Salinity::Saline),
            l.derived[Reservoir::Ocean.index()].of(Salinity::Saline),
            "the saline volume changed in a column with no salt transport"
        );
        for r in [Reservoir::Surface, Reservoir::Groundwater] {
            let s = l.storage[r.index()];
            assert_eq!(s.of(Salinity::Saline), 0.0, "{} is not a fresh reservoir", r.name());
        }
    }

    #[test]
    fn runoff_moves_fresh_water_into_the_ocean_without_creating_or_destroying_any() {
        // The claim that makes salinity worth tracking separately, stated as the two
        // per-class identities rather than as a difference between them.
        //
        // An earlier version compared the saline delta against the fresh delta and
        // demanded they were opposite, and then separately claimed the saline class
        // grew by the runoff. Both are wrong: infiltration and baseflow move fresh
        // water between two **fresh** reservoirs, so neither class sees them; and
        // fresh water arriving at the ocean does not arrive with salt, so the saline
        // class is invariant. Measuring it is what showed it — the run reported a
        // per-class error of 8.9e-4 while the total was exact, and the only way
        // those can both be true is if the class identities were wrong.
        //
        // So: saline unchanged; fresh gained exactly the rain; and the fresh water that
        // reached the sea is where it went. Runoff moves fresh water between the
        // surface reservoir and the ocean's fresh bucket, and both are fresh — so
        // **runoff is not an exit from the fresh class**, which is the second
        // version of this mistake and the one the `8.9e-4` measured error named.
        let mut moved = 0.0;
        for i in -12..12 {
            let l = ledger(i as f64 * 500.0, 0.0, 4);
            let fresh_before = Ledger::class_total_m3(&l.derived, Salinity::Fresh);
            let fresh_after = Ledger::class_total_m3(&l.storage, Salinity::Fresh);
            let saline_after = Ledger::class_total_m3(&l.storage, Salinity::Saline);
            let saline_before = Ledger::class_total_m3(&l.derived, Salinity::Saline);
            let scale = saline_after.abs().max(fresh_before.abs()).max(1.0);

            let saline_drift = (saline_after - saline_before).abs();
            assert!(
                saline_drift <= 1e-9 * scale,
                "at {:?} the saline class moved {saline_drift} m3, and nothing in this tree \
                 transports salt",
                l.column
            );

            // The mixing claim: the ocean's fresh content grew by exactly the runoff.
            let mixing = l.mixing_error_m3().abs();
            assert!(
                mixing <= 1e-9 * scale,
                "at {:?} the ocean's fresh water and the runoff disagree by {mixing} m3",
                l.column
            );

            let fresh_drift = (fresh_after - fresh_before - l.source_m3()).abs();
            assert!(
                fresh_drift <= 1e-9 * scale,
                "at {:?} the fresh class drifted {fresh_drift} m3 against a source of {} m3",
                l.column,
                l.source_m3()
            );

            moved += l.runoff_m3();
        }
        assert!(moved > 0.0, "no runoff happened anywhere on the transect, so this is vacuous");
    }

    #[test]
    fn precipitation_is_the_only_source_and_the_internal_fluxes_cancel() {
        // The closure identity, checked by *counting*: every transfer is either the
        // one source or one of the three internal pairs, and there is no fourth
        // kind of movement to have gone unnoticed.
        let l = ledger(500.0, -300.0, 4);
        for t in &l.transfers {
            match t.process {
                Process::Precipitation => {
                    assert_eq!(t.to, Reservoir::Surface, "rain landed somewhere else");
                    assert_eq!(t.class, Salinity::Fresh, "rain is fresh");
                }
                Process::Runoff => assert_eq!((t.from, t.to), (Reservoir::Surface, Reservoir::Ocean)),
                Process::Infiltration => {
                    assert_eq!((t.from, t.to), (Reservoir::Surface, Reservoir::Groundwater))
                }
                Process::Baseflow => {
                    assert_eq!((t.from, t.to), (Reservoir::Groundwater, Reservoir::Surface))
                }
            }
        }
        // And the sums really are what the identity says they are, computed
        // independently of `closure_error_m3`.
        let total_delta = l.storage_total_m3() - l.derived_total_m3();
        assert!((total_delta - l.source_m3()).abs() < 1e-9);
    }

    #[test]
    fn a_ratio_outside_its_declared_range_is_refused_by_name() {
        // Teeth: a ratio of 1.5 is water appearing, not a fast process.
        let r = Ratios { runoff_per_tick: 1.5, ..Ratios::default() };
        let e = r.validated().unwrap_err();
        assert_eq!(e.kind(), RefusalKind::OutOfDeclaredRange);
        assert!(e.detail().contains("runoff"), "got {e}");

        let r = Ratios { baseflow_per_tick: -0.1, ..Ratios::default() };
        assert!(r.validated().unwrap_err().detail().contains("baseflow"));

        // Precipitation's own bound is "is it a rate at all".
        let r = Ratios { precipitation_years_per_tick: f64::NAN, ..Ratios::default() };
        assert!(r.validated().unwrap_err().detail().contains("precipitation"));
    }

    #[test]
    fn a_withdrawal_larger_than_a_reservoir_is_refused_and_not_clamped() {
        // The clamp is the failure: a clamped withdrawal invents water, and an
        // invented reservoir is how a conserving ledger starts conserving the wrong
        // thing.
        let mut c = Contents { fresh_m3: 1.0, saline_m3: 0.0 };
        let e = c.withdraw(Salinity::Fresh, 2.0, "runoff").unwrap_err();
        assert_eq!(e.kind(), RefusalKind::OutOfDeclaredRange);
        assert_eq!(c.fresh_m3, 1.0, "a refused withdrawal still changed the reservoir");
        assert!(c.withdraw(Salinity::Fresh, 1.0, "runoff").is_ok());
        assert_eq!(c.fresh_m3, 0.0);
        assert!(c.withdraw(Salinity::Saline, 0.0, "runoff").is_ok());
    }

    #[test]
    fn storage_is_derived_from_terrain_so_a_skipped_tick_changes_nothing_but_the_flux() {
        // The purity claim, stated as a measurement: two ticks' worth of
        // accumulation, or none, must not change the *derived* figure — because
        // nothing is stored between ticks.
        let a = ledger(0.0, 0.0, 4);
        let b = ledger(0.0, 0.0, 8);
        assert_eq!(a.derived, b.derived, "derived storage changed with the tick");
        // And the only thing the tick changes is the flux, so the change is the
        // rain and nothing else.
        assert!(a.storage_total_m3() > a.derived_total_m3() || a.source_m3() == 0.0);
    }

    #[test]
    fn the_reservoir_set_is_three_and_the_out_of_scope_three_are_named() {
        // The ruling, held as an assertion. Snowpack, soil moisture and vapour are
        // out of scope because the world's atmosphere is a sky shader and none of
        // the water cycle is drawn; a sixth reservoir would mean inventing storage
        // this tree has no source for.
        assert_eq!(Reservoir::ALL.len(), 3);
        let names: Vec<&str> = Reservoir::ALL.iter().map(|r| r.name()).collect();
        assert_eq!(names, ["ocean", "surface", "groundwater"]);
        // One source and three internal transfers. Stated as a count because that
        // count is the closure argument: every process other than the first is a
        // movement between two of the three, so they cancel.
        assert_eq!(Process::ALL.len(), 4);
        assert_eq!(Process::Precipitation, Process::ALL[0], "the source is listed first");
    }

    #[test]
    fn the_phase_densities_are_the_sutra_pair_and_are_declared_here() {
        // Reference, never rules (C-48/C-49): the numbers are cited and declared,
        // and nothing in the tree is obliged to agree with them.
        assert_eq!(Phase::Liquid.density_kg_m3(), 1_000.0);
        assert_eq!(Phase::Solid.density_kg_m3(), 917.0);
        // And the ledger closes over **volume**, not mass — which is why the phase
        // is declared rather than derived, and why nothing above depends on it.
        let l = ledger(500.0, -300.0, 4);
        for s in &l.storage {
            assert_eq!(s.phase, Phase::Liquid, "{} has no derived phase", s.reservoir.name());
        }
        assert_closed(&l);
    }

    #[test]
    fn the_declared_rain_rate_is_the_clock_s_own() {
        // The literal that was wrong by 876x is now the clock's own arithmetic, and
        // this is what stops it becoming a literal again.
        let r = Ratios::declared().unwrap();
        assert_eq!(r.precipitation_years_per_tick, 1.0 / year_in_ticks());
        // And the numbers are checkable against the calendar rather than trusted.
        assert!(year_in_ticks() > 8_000_000.0, "a year is {} ticks", year_in_ticks());
        assert!(
            (year_in_ticks() - crate::climate::TICKS_PER_DAY as f64 * 365.25).abs() < 1e-6,
            "the year is not the day times the year"
        );
        // A tick is a small slice of a year: a thousand ticks must be well under
        // one percent of it, which is what makes the mm/year -> m/tick conversion
        // sane rather than a rounding error in disguise.
        assert!(1000.0 * r.precipitation_years_per_tick < 0.001);
    }

    #[test]
    fn groundwater_is_capped_and_the_cap_is_the_declared_one() {
        // Without a cap, "below the water table" is unbounded and the column's
        // share unknowable.
        let s = spec();
        let mut region = Region::around(&s, (0.0, 0.0), 4, CELL_M);
        region.fill_depressions();
        let storage = derived_storage(&s, 0.0, 0.0, &region);
        let groundwater = storage[Reservoir::Groundwater.index()];
        assert!(groundwater.total_m3() <= GROUNDWATER_CAP_M * CELL_M * CELL_M + 1e-12);
        // And a column below the cap counts only up to its own surface.
        let deep = (water_table_m(&s) - s.config.sea_level_m).clamp(0.0, GROUNDWATER_CAP_M);
        assert!(deep > 0.0, "the water table is not above sea level, so nothing saturates");
    }
}