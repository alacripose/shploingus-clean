//! Fluidics — **the one flow model, as a cellular automaton**.
//!
//! The design is the user's `docs/fluidics_pressure_and_water_ingress.pdf`, adopted whole
//! (the user's ruling, 2026-10-04 — `docs/LEDGER.md` C-153). The mechanism it describes is a
//! **cellular automaton over a material lattice**: every cell carries its own water state,
//! and the timestep is the PDF's §9 recipe, in the PDF's own order:
//!
//! 1. boundary conditions — an outside water level, an inlet pressure, a drainage pressure;
//! 2. `Pc = retention_curve(S)` — capillary pressure from the cell's saturation;
//! 3. `H = p/(rho g) + z` — pressure head plus elevation head, one driving potential;
//! 4. `flux = -K_eff * grad(H)` — Darcy, with `K_eff = K(S)` from the conductivity curve;
//! 5. `S = update_by_mass_balance(S, flux, dt)` — exact;
//! 6. recompute pore accessibility, conductivity and capillary pressure;
//! 7. repeat.
//!
//! # The lattice, and why not a column
//!
//! The world is **cubic chunks** (`coord::CHUNK_SIDE` = 32, one voxel = one metre) and the
//! fluid lattice is the world's own six-neighbour grid — `±x`, `±y`, `±z`, with **z up**
//! (C-138). A vertical column was the first slice and is superseded: flow is not a one-axis
//! quantity, and a model that cannot carry water sideways cannot carry an ocean, a pipe, or
//! the atmosphere's water. Each edge is the face between two adjacent voxels; a pair is
//! processed **once**, from the lower coordinate, so the two sides of a chunk seam sample the
//! same global neighbour by construction — the city-builder's seam rule, and the reason the
//! edge's conductance is a *face* property rather than two half-readings.
//!
//! # What is exact, and what is declared
//!
//! * **Mass and volume are exact** ([`Ratio`], and [`voxel_model::conservation::Mass`] for a
//!   cell's fluid): a transfer is `K_face * |dH| * dt` in exact rationals, bounded by the
//!   giver's volume and the receiver's free pore volume, and the books close exactly.
//! * **Pressure, head and flux are carried exactly too** ([`Rational`], signed — suction is a
//!   negative gauge pressure, so the unsigned [`Ratio`] cannot hold them). The reference's
//!   fixed-point-at-scale-1000 pressure is the **declared scale at the boundary**:
//!   [`Cell::pressure_fixed`] converts at a caller's scale and refuses a value the scale
//!   cannot represent, so a rounded pressure is never silently substituted for the real one.
//! * **The driving tolerance is the declared family's** — `pressure and flow` gates at
//!   **one millimetre of head**, read from [`voxel_model::conservation`] at compile time
//!   (`HEAD_TOLERANCE`), so withdrawing the declaration fails the build.
//! * **Permeability is sourced or refused.** The PDF's own sentence — *porosity alone does
//!   not tell you how fast water moves* — is why every physical property here is a
//!   [`Sourced`] value: it cannot be constructed without a named source, and a cell whose
//!   material has no declared permeability is not a conductor.
//!
//! # The physics the PDF names, and where it lives here
//!
//! | the PDF | here |
//! |---|---|
//! | hydrostatic pressure `p = rho g h` | [`hydrostatic_pressure_pa`] |
//! | hydraulic head `H = p/(rho g) + z` | [`Cell::head_m`] |
//! | Young–Laplace `Pc = 2 gamma cos(theta) / r` | [`capillary_pressure_pa`] |
//! | capillary rise `h = 2 gamma cos(theta)/(rho g r)` | [`capillary_head_m`] |
//! | Darcy conductivity `K = k rho g / mu` | [`Cell::saturated_conductivity_m_s`] |
//! | slab estimate `Q = (k A / mu)(dp/L)` | [`slab_flow_m3_s`] |
//! | Lucas–Washburn `L^2 = gamma r cos(theta) t /(2 mu)` | [`imbibition_front_squared_m2`] |
//! | retention `S = S(Pc)` and `K(S)` | [`Curve`], [`Retention`] |
//! | hysteresis: drying is not wetting | [`Branch`] |
//!
//! # Named limits, refused rather than approximated
//!
//! One fluid per cell: two cells of different fluids do not mix or displace — the PDF's
//! mixture and gas work (`H-24.6`, *gas is in scope*) is the next landing, and the model
//! refuses an edge between two fluids rather than guessing. A face's conductance between two
//! materials is the **series** combination of the two half-cells (the PDF's *resistance*
//! picture: two resistors in series), declared here because a discretisation is a choice and
//! an undeclared choice is the thing this tree does not ship. Pipes are **carriers of the
//! same model** (H-24.2) and arrive as their own conductor law; today a non-adjacent carrier
//! has no face and is not expressible.
//!
//! # The rung
//!
//! [`is_due_at`] is `Stage::Hydrology`'s own ×4 cadence, read from the stage and never
//! restated; at the world clock's 20 Hz the step's `dt` is **1/5 s**, and the caller passes
//! it because the clock lives in the client and the model must not read one.

use crate::field_sampler::FixedValue;
use crate::stages::Stage;
use voxel_model::conservation::{ApproximateFamily, ToleranceClass};
use voxel_model::coord::{VoxelPos, FILL_SIDE};
use voxel_model::matter::Rational;
use voxel_model::material::Ratio;
use voxel_model::refusal::{Refusal, RefusalKind, Refused};
use voxel_model::world::FluidAddress;
use std::collections::BTreeMap;

/// The pressure-and-flow family's declared gating tolerance, in **metres of head**.
///
/// Read from the family table — one statement, so a change there cannot silently retune the
/// solver here. The match is evaluated at compile time: a class change that withdrew the
/// declaration stops the build instead of defaulting to "always flows".
pub const HEAD_TOLERANCE: Ratio = match ApproximateFamily::PressureAndFlow.tolerance_class() {
    ToleranceClass::Gating(tolerance) => tolerance,
    _ => panic!("pressure and flow must declare a gating tolerance for fluidics to gate on"),
};

/// How many moving passes per cell [`Lattice::settle`] may spend before refusing.
///
/// Declared, not derived: a CA pass moves fluid one face at a time, and a lattice still
/// moving at the cap is refused rather than silently stopped.
pub const SETTLE_CAP_PER_CELL: usize = 8;

/// One voxel's edge, in metres — the scale contract's own number (`coord`: *voxel = 1 m*).
pub const CELL_EDGE_M: Ratio = Ratio::from_parts(1, 1);

/// One voxel face's area, in square metres.
pub const CELL_FACE_M2: Ratio = Ratio::from_parts(1, 1);

/// The number of levels a cell's fill is resolved to: **32 per block axis, one level
/// per 31.25 mm** (`coord::FILL_SIDE`) — not 256, and not the material detail base's
/// own side (16 per subvoxel edge). The user ruled the scale, 2026-10-05: *"we
/// definitley don't need fluidics to be the 256 actual base"*, then *"it's the level
/// that the water and fluids should run at, so it can animate down to as small as
/// individual 32x pixels (not 256x because it'd waste performance ESPECIALLY because
/// of the transparency issues that will certainly come up)"*, and finally fixed the
/// number itself: *"I like the 32x per side of block, so if there's stuff happening
/// below that, then I don't want it"* (C-164).
///
/// This is the interpolation interface the user named: the stored 4×4×4 subvoxel grid is
/// the material resolution, and the appearance layer animates a fluid **within** a block
/// at these levels — rain, flowing water, molten metal, all from the same exact state.
/// Nothing here stores a dense grid: the only thing kept is one level per fluid cell.
pub const FILL_LEVELS: u32 = FILL_SIDE;

/// A cell's fill as a count of 1/`FILL_LEVELS`ths of the block's pore volume.
///
/// The level is **floored**, never rounded up: it counts the complete levels that are
/// filled, so a surface is never drawn above where the books say the fluid is. It is a
/// display value derived from the state by pure integer arithmetic — it decides nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FillLevel(u32);

impl FillLevel {
    /// An empty cell.
    pub const EMPTY: FillLevel = FillLevel(0);

    /// A cell full to its pore volume.
    pub const FULL: FillLevel = FillLevel(FILL_LEVELS);

    /// A level, refused outside the declared scale rather than clamped.
    pub fn new(level: u32) -> Refused<Self> {
        if level <= FILL_LEVELS {
            Ok(FillLevel(level))
        } else {
            Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a fill level above {FILL_LEVELS} is not a level of anything"),
            ))
        }
    }

    /// The exact **floor** of `saturation × FILL_LEVELS`, in integer arithmetic.
    pub fn of(saturation: Ratio) -> Refused<Self> {
        if saturation.is_greater_than(Ratio::ONE) {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a saturation of {}/{} is above one and has no fill level",
                    saturation.numerator(),
                    saturation.denominator()
                ),
            ));
        }
        let scaled = (saturation.numerator() as u128) * (FILL_LEVELS as u128)
            / (saturation.denominator() as u128);
        Ok(FillLevel(scaled as u32))
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    /// The level as an exact fraction of the block.
    pub fn fraction(self) -> Refused<Ratio> {
        Ratio::new(self.0 as u64, FILL_LEVELS as u64)
    }

    /// The fluid surface's height above the cell's floor, in metres — exact, because a
    /// cell's edge is one metre ([`CELL_EDGE_M`]).
    ///
    /// This is the interpolation height the appearance layer draws the surface at; it is
    /// **not** a claim about how the fluid sits in the pore space, which the retention
    /// curve owns.
    pub fn surface_height_m(self) -> Refused<Rational> {
        Rational::new(self.0 as i128, FILL_LEVELS as i128)
    }
}

/// The declared fixed-point scale for pressure at a boundary that wants one.
///
/// The reference's `HYDROLOGY_SCALE = 1000` mechanism, cited: pressure crosses to a
/// fixed-point representation at a declared scale, and a value the scale cannot represent
/// **refuses** ([`Cell::pressure_fixed`]) rather than rounding.
pub const PRESSURE_SCALE: i128 = 1000;

/// Standard gravity as the PDF's own worked examples use it: **9.81 m/s²**.
pub const fn gravity_m_s2() -> Ratio {
    Ratio::from_parts(981, 100)
}

/// A value and **where it came from**. The PDF's parameter checklist ends in measured or
/// fitted values; a value without a source is exactly the invented number this model may not
/// carry, so the source is a required argument and an empty one is refused by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sourced<T> {
    value: T,
    source: &'static str,
}

impl<T> Sourced<T> {
    pub fn new(value: T, source: &'static str) -> Refused<Self> {
        if source.trim().is_empty() {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                "a physical value with no source is an invented value; name where it came from",
            ));
        }
        Ok(Sourced { value, source })
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn source(&self) -> &'static str {
        self.source
    }
}

/// A nonnegative physical quantity with a source. Used for the properties the PDF's
/// checklist names; a negative one is refused at construction.
macro_rules! sourced_quantity {
    ($(#[$doc:meta])* $name:ident, $what:literal) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name(Sourced<Rational>);

        impl $name {
            pub fn new(value: Rational, source: &'static str) -> Refused<Self> {
                value.require_nonnegative($what)?;
                Ok($name(Sourced::new(value, source)?))
            }

            pub fn value(&self) -> Rational {
                *self.0.value()
            }

            pub fn source(&self) -> &'static str {
                self.0.source()
            }
        }
    };
}

sourced_quantity!(
    /// Intrinsic permeability, in square metres. **Not porosity** — the PDF's own words.
    Permeability,
    "permeability"
);
sourced_quantity!(
    /// Dynamic viscosity, in pascal-seconds.
    Viscosity,
    "viscosity"
);
sourced_quantity!(
    /// Surface tension, in newtons per metre.
    SurfaceTension,
    "surface tension"
);
sourced_quantity!(
    /// Effective pore or throat radius, in metres.
    PoreRadius,
    "pore radius"
);
sourced_quantity!(
    /// Temperature, in kelvin. Carried as state; the `mu(T)`/`gamma(T)` relations are
    /// `[NS]` and are not invented here.
    Temperature,
    "temperature"
);

/// The wetting/contact behaviour, as the **cosine of the contact angle** — the quantity
/// Young–Laplace actually uses. Negative is a non-wetting liquid (the PDF's `theta > 90`);
/// a magnitude above one is not a cosine and is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactCosine(Sourced<Rational>);

impl ContactCosine {
    pub fn new(cos_theta: Rational, source: &'static str) -> Refused<Self> {
        let one = Rational::from_integer(1);
        if cos_theta.try_cmp(&one)? == core::cmp::Ordering::Greater {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "cos(theta) is {}/{}, and a cosine is at most 1",
                    cos_theta.numerator(),
                    cos_theta.denominator()
                ),
            ));
        }
        let minus_one = Rational::from_integer(-1);
        if cos_theta.try_cmp(&minus_one)? == core::cmp::Ordering::Less {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "cos(theta) is {}/{}, and a cosine is at least -1",
                    cos_theta.numerator(),
                    cos_theta.denominator()
                ),
            ));
        }
        Ok(ContactCosine(Sourced::new(cos_theta, source)?))
    }

    pub fn value(&self) -> Rational {
        *self.0.value()
    }

    pub fn source(&self) -> &'static str {
        self.0.source()
    }
}

/// Which branch of a hysteretic relation a cell is on.
///
/// The PDF: *the path by which a material becomes wet can differ from the path by which it
/// dries*, so "the same saturation" is not one pressure. A cell switches branch with the
/// sign of its volume change and holds it while the saturation is unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Branch {
    Wetting,
    Drying,
}

/// A fitted relation: exact breakpoints with a source, evaluated by exact interpolation.
///
/// The PDF names van Genuchten and Mualem as the closed forms; a fractional power cannot be
/// an exact rational, and this tree's rule is *no floats, ever* (H-18.7). So the relation is
/// carried as its **fitted table** — the PDF's own allowance, *real materials require
/// measured or fitted parameters* — and the table is exact. Both ends must be declared
/// (`S = 0` and `S = 1`), because a curve whose ends are guessed is a default wearing a
/// table's clothes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Curve {
    points: Vec<(Ratio, Rational)>,
    source: &'static str,
}

impl Curve {
    /// A retention curve: capillary pressure in pascals, nonnegative (suction), and **zero at
    /// saturation** — the free surface where `p_water = p_air`, which is the PDF's own
    /// boundary.
    pub fn retention(points: Vec<(Ratio, Rational)>, source: &'static str) -> Refused<Self> {
        let curve = Self::new(points, source)?;
        for (s, value) in &curve.points {
            value.require_nonnegative("capillary pressure")?;
            if s.is_one() && !value.is_zero() {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "a retention curve must reach zero capillary pressure at saturation, \
                         and this one says {}/{} Pa at S = 1",
                        value.numerator(),
                        value.denominator()
                    ),
                ));
            }
        }
        Ok(curve)
    }

    /// A conductivity curve: the relative conductivity `K(S)/K_sat`, in `[0, 1]`.
    pub fn conductivity(points: Vec<(Ratio, Rational)>, source: &'static str) -> Refused<Self> {
        let curve = Self::new(points, source)?;
        let one = Rational::from_integer(1);
        for (_, value) in &curve.points {
            value.require_nonnegative("relative conductivity")?;
            if value.try_cmp(&one)? == core::cmp::Ordering::Greater {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "a relative conductivity of {}/{} exceeds 1",
                        value.numerator(),
                        value.denominator()
                    ),
                ));
            }
        }
        Ok(curve)
    }

    fn new(points: Vec<(Ratio, Rational)>, source: &'static str) -> Refused<Self> {
        if points.len() < 2 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                "a curve needs at least two declared points; one point is a constant pretending \
                 to be a relation",
            ));
        }
        if !points[0].0.is_zero() {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a curve must be declared from S = 0, and it starts at {}", points[0].0),
            ));
        }
        if !points[points.len() - 1].0.is_one() {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a curve must be declared to S = 1, and it ends at {}",
                    points[points.len() - 1].0
                ),
            ));
        }
        for pair in points.windows(2) {
            if !pair[1].0.is_greater_than(pair[0].0) {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    format!(
                        "curve saturations must strictly ascend, and {} does not follow {}",
                        pair[1].0, pair[0].0
                    ),
                ));
            }
        }
        let _ = Sourced::new((), source)?;
        Ok(Curve { points, source })
    }

    /// The exact value at a saturation, interpolated between the two declared points that
    /// bracket it. A saturation outside `[0, 1]` is refused.
    pub fn at(&self, saturation: Ratio) -> Refused<Rational> {
        let s = ratio_to_rational(saturation, "saturation")?;
        let zero = Rational::ZERO;
        if s.try_cmp(&zero)? == core::cmp::Ordering::Less {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("saturation {saturation} is negative"),
            ));
        }
        let one = Rational::from_integer(1);
        if s.try_cmp(&one)? == core::cmp::Ordering::Greater {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("saturation {saturation} is above one"),
            ));
        }
        for pair in self.points.windows(2) {
            let (s0, v0) = pair[0];
            let (s1, v1) = pair[1];
            if !saturation.is_greater_than(s1) {
                let s0 = ratio_to_rational(s0, "curve point")?;
                let s1 = ratio_to_rational(s1, "curve point")?;
                let span = sub(&s1, &s0, "curve span")?;
                let t = div(&sub(&s, &s0, "curve offset")?, &span, "curve fraction")?;
                let rise = sub(&v1, &v0, "curve rise")?;
                return add(&v0, &mul(&rise, &t, "curve rise")?, "curve value");
            }
        }
        // The last point is S = 1 by construction, so the loop always returns.
        Err(Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            "a curve with no bracket for the asked saturation is not a curve",
        ))
    }

    pub fn source(&self) -> &'static str {
        self.source
    }

    pub fn points(&self) -> &[(Ratio, Rational)] {
        &self.points
    }
}

/// A hysteretic retention relation: the wetting path and the drying path, both declared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Retention {
    wetting: Curve,
    drying: Curve,
}

impl Retention {
    pub fn new(wetting: Curve, drying: Curve) -> Refused<Self> {
        Ok(Retention { wetting, drying })
    }

    pub fn curve(&self, branch: Branch) -> &Curve {
        match branch {
            Branch::Wetting => &self.wetting,
            Branch::Drying => &self.drying,
        }
    }
}

/// A fluid, by its row's properties: what it is and what its movement costs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fluid {
    address: FluidAddress,
    density_kg_m3: Ratio,
    viscosity: Viscosity,
    surface_tension: SurfaceTension,
}

impl Fluid {
    pub fn new(
        address: FluidAddress,
        density_kg_m3: Ratio,
        viscosity: Viscosity,
        surface_tension: SurfaceTension,
    ) -> Refused<Self> {
        if density_kg_m3.is_zero() {
            return Err(Refusal::owned(
                RefusalKind::ZeroRatio,
                "a fluid with zero density has no hydrostatic pressure and no head",
            ));
        }
        Ok(Fluid { address, density_kg_m3, viscosity, surface_tension })
    }

    pub fn address(&self) -> &FluidAddress {
        &self.address
    }

    pub fn density_kg_m3(&self) -> Ratio {
        self.density_kg_m3
    }

    pub fn viscosity(&self) -> &Viscosity {
        &self.viscosity
    }

    pub fn surface_tension(&self) -> &SurfaceTension {
        &self.surface_tension
    }

    /// The fluid's weight density `rho g`, in `N/m³` — the divisor that turns pressure into
    /// head and the factor that turns a pore radius into a capillary head.
    pub fn weight_density(&self) -> Refused<Ratio> {
        Ok(self.density_kg_m3 * gravity_m_s2())
    }
}

/// A porous material: geometry and wetting, the PDF's checklist's material half.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PorousMaterial {
    permeability: Permeability,
    pore_radius: PoreRadius,
    contact: ContactCosine,
    retention: Retention,
    conductivity: Curve,
}

impl PorousMaterial {
    pub fn new(
        permeability: Permeability,
        pore_radius: PoreRadius,
        contact: ContactCosine,
        retention: Retention,
        conductivity: Curve,
    ) -> Refused<Self> {
        Ok(PorousMaterial { permeability, pore_radius, contact, retention, conductivity })
    }

    pub fn permeability(&self) -> &Permeability {
        &self.permeability
    }

    pub fn pore_radius(&self) -> &PoreRadius {
        &self.pore_radius
    }

    pub fn contact(&self) -> &ContactCosine {
        &self.contact
    }

    pub fn retention(&self) -> &Retention {
        &self.retention
    }

    pub fn conductivity(&self) -> &Curve {
        &self.conductivity
    }

    /// The saturated hydraulic conductivity `K = k rho g / mu`, in m/s — the PDF's §5.
    pub fn saturated_conductivity_m_s(&self, fluid: &Fluid) -> Refused<Ratio> {
        let k = rational_to_ratio(&self.permeability.value(), "permeability")?;
        let mu = rational_to_ratio(&fluid.viscosity.value(), "viscosity")?;
        if mu.is_zero() {
            return Err(Refusal::owned(
                RefusalKind::ZeroRatio,
                "a zero viscosity gives an infinite conductivity",
            ));
        }
        let weight = k * fluid.density_kg_m3() * gravity_m_s2();
        weight.checked_div(mu)
    }
}

/// A cell's state — the PDF's §9 *minimum state*: `p_w`, `z`, `S`, `k`, `K(S)`, `Pc(S)`,
/// `r_eff`, `theta`, `x_f`, `T`.
///
/// The volume is the state the books close on; the saturation is derived from it; the
/// pressure is derived from the saturation through the retention curve (branch included)
/// unless the cell is a boundary, whose pressure the caller imposes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    fluid: Fluid,
    material: PorousMaterial,
    volume_m3: Ratio,
    capacity_m3: Ratio,
    pressure_pa: Rational,
    branch: Branch,
    front_m2: Rational,
    temperature: Temperature,
    imposed: bool,
}

impl Cell {
    /// A cell of one fluid in one material, holding `volume_m3` of it.
    ///
    /// Refuses a cell with no pore volume and a volume above the pore volume; refuses a
    /// negative volume. The initial branch is the caller's, because the PDF's state carries
    /// *whether the material is drying, wetting, or cycling* — the history is not inferable
    /// from the saturation.
    pub fn new(
        fluid: Fluid,
        material: PorousMaterial,
        capacity_m3: Ratio,
        volume_m3: Ratio,
        branch: Branch,
        temperature: Temperature,
    ) -> Refused<Self> {
        if capacity_m3.is_zero() {
            return Err(Refusal::owned(
                RefusalKind::ZeroRatio,
                "a cell with no pore volume cannot hold a fluid",
            ));
        }
        if volume_m3.is_greater_than(capacity_m3) {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "a cell may not hold more than its pore volume: {volume_m3} m³ in \
                     {capacity_m3} m³ of pore space (H-24.1)"
                ),
            ));
        }
        let mut cell = Cell {
            fluid,
            material,
            volume_m3,
            capacity_m3,
            pressure_pa: Rational::ZERO,
            branch,
            front_m2: Rational::ZERO,
            temperature,
            imposed: false,
        };
        cell.sync_pressure()?;
        Ok(cell)
    }

    pub fn volume_m3(&self) -> Ratio {
        self.volume_m3
    }

    pub fn capacity_m3(&self) -> Ratio {
        self.capacity_m3
    }

    pub fn free_m3(&self) -> Refused<Ratio> {
        self.capacity_m3.checked_sub(self.volume_m3)
    }

    pub fn fluid(&self) -> &Fluid {
        &self.fluid
    }

    pub fn material(&self) -> &PorousMaterial {
        &self.material
    }

    pub fn branch(&self) -> Branch {
        self.branch
    }

    pub fn temperature(&self) -> &Temperature {
        &self.temperature
    }

    /// The fraction of the pore volume that is filled, exactly.
    pub fn saturation(&self) -> Refused<Ratio> {
        self.volume_m3.checked_div(self.capacity_m3)
    }

    /// The gauge water pressure. Negative means suction — the PDF's *hidden pump*.
    pub fn pressure_pa(&self) -> Rational {
        self.pressure_pa
    }

    /// The pressure as a fixed-point value at a caller's declared scale.
    ///
    /// The reference's mechanism (`HYDROLOGY_SCALE = 1000`); a value the scale cannot
    /// represent is **refused** rather than rounded, because a rounded pressure is a
    /// different world wearing the right units.
    pub fn pressure_fixed(&self, scale: i128) -> Refused<FixedValue> {
        if scale <= 0 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("a pressure scale of {scale} is not positive"),
            ));
        }
        let scaled = mul(&self.pressure_pa, &Rational::from_integer(scale), "pressure scale")?;
        if scaled.denominator() != 1 {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "the pressure {}/{} Pa is not representable at scale {scale}; it is not \
                     rounded to fit",
                    scaled.numerator(),
                    scaled.denominator()
                ),
            ));
        }
        FixedValue::new(scaled.numerator(), scale).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("the pressure scale {scale} is not usable"),
            )
        })
    }

    /// The cell's fill at the world's own interpolation scale, from its exact
    /// saturation — the value the appearance layer animates the subvoxels from (rain,
    /// flowing water, molten metal; `docs/FLUIDICS.md` §3).
    pub fn fill_level(&self) -> Refused<FillLevel> {
        FillLevel::of(self.saturation()?)
    }

    /// The wetting-front estimate `x_f`: the squared position of the sharp front, exact. It
    /// advances by the PDF's Lucas–Washburn rate while the cell is on its wetting path and
    /// unsaturated, and it is deliberately not the flux — the PDF calls the sharp front an
    /// approximation and this model's flow is the continuum one.
    pub fn front_m2(&self) -> Rational {
        self.front_m2
    }

    /// The hydraulic head `H = p/(rho g) + z`, in metres, at an elevation in metres.
    pub fn head_m(&self, elevation_m: i32) -> Refused<Rational> {
        let weight = self.fluid.weight_density()?;
        let weight = ratio_to_rational(weight, "weight density")?;
        let pressure_head = div(&self.pressure_pa, &weight, "pressure head")?;
        add(&pressure_head, &Rational::from_integer(elevation_m as i128), "head")
    }

    /// The saturated conductivity `K = k rho g / mu`, in m/s.
    pub fn saturated_conductivity_m_s(&self) -> Refused<Ratio> {
        self.material.saturated_conductivity_m_s(&self.fluid)
    }

    /// The effective conductivity `K(S)`, in m/s.
    pub fn effective_conductivity_m_s(&self) -> Refused<Ratio> {
        let saturation = self.saturation()?;
        let relative = self.material.conductivity().at(saturation)?;
        let relative = rational_to_ratio(&relative, "relative conductivity")?;
        Ok(self.saturated_conductivity_m_s()? * relative)
    }

    /// Add fluid from a boundary or a source, refusing an overflow by name.
    pub fn deposit(&mut self, volume_m3: Ratio) -> Refused<()> {
        let total = self.volume_m3 + volume_m3;
        if total.is_greater_than(self.capacity_m3) {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "deposit refused: {} would hold {total} m³ in a pore volume of {} m³ (H-24.1)",
                    self.fluid.address.as_str(),
                    self.capacity_m3
                ),
            ));
        }
        self.volume_m3 = total;
        self.branch = Branch::Wetting;
        self.sync_pressure()?;
        Ok(())
    }

    /// Remove fluid to a sink or a drainage boundary, refusing to remove what is not there.
    pub fn drain(&mut self, volume_m3: Ratio) -> Refused<()> {
        self.volume_m3 = self.volume_m3.checked_sub(volume_m3)?;
        self.branch = Branch::Drying;
        self.sync_pressure()?;
        Ok(())
    }

    /// Impose a boundary pressure: the cell's pressure stops being derived from its own
    /// retention curve, which is what the PDF's *inlet pressure* and *drainage pressure*
    /// boundary conditions are.
    pub fn impose_pressure(&mut self, pressure_pa: Rational) {
        self.pressure_pa = pressure_pa;
        self.imposed = true;
    }

    /// Whether the pressure is the caller's boundary condition rather than derived.
    pub fn is_imposed(&self) -> bool {
        self.imposed
    }

    /// Recompute the pressure from the retention curve at the current saturation. A boundary
    /// cell keeps its imposed pressure.
    fn sync_pressure(&mut self) -> Refused<()> {
        if self.imposed {
            return Ok(());
        }
        let saturation = self.saturation()?;
        let pc = self.material.retention().curve(self.branch).at(saturation)?;
        // Pc = p_air - p_water, and gauge pressure takes p_air = 0, so p_w = -Pc.
        self.pressure_pa = sub(&Rational::ZERO, &pc, "gauge pressure")?;
        Ok(())
    }
}

/// A step's report: what moved, and how much.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Step {
    /// Faces that issued a transfer.
    pub transfers: usize,
    /// The exact volume the step moved.
    pub volume_m3: Ratio,
}

impl Step {
    pub fn moved(&self) -> bool {
        self.transfers > 0
    }
}

/// The fluid lattice: cells on the world's own six-neighbour grid.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lattice {
    cells: BTreeMap<VoxelPos, Cell>,
}

/// The three positive axis steps; every edge is found once, from its lower coordinate.
const POSITIVE_NEIGHBOURS: [[i32; 3]; 3] = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];

impl Lattice {
    pub fn new() -> Self {
        Lattice::default()
    }

    /// Place a cell. A position already holding a fluid is refused rather than replaced —
    /// two fluids in one voxel is the mixing this model does not do.
    pub fn insert(&mut self, at: VoxelPos, cell: Cell) -> Refused<()> {
        if self.cells.contains_key(&at) {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!("({}, {}, {}) already holds a fluid", at.x, at.y, at.z),
            ));
        }
        self.cells.insert(at, cell);
        Ok(())
    }

    pub fn get(&self, at: VoxelPos) -> Option<&Cell> {
        self.cells.get(&at)
    }

    pub fn get_mut(&mut self, at: VoxelPos) -> Option<&mut Cell> {
        self.cells.get_mut(&at)
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn positions(&self) -> impl Iterator<Item = VoxelPos> + '_ {
        self.cells.keys().copied()
    }

    /// The exact total fluid volume — the number that must not move except by deposit/drain.
    pub fn total_volume_m3(&self) -> Ratio {
        self.cells.values().fold(Ratio::ZERO, |total, cell| total + cell.volume_m3)
    }

    /// One cellular-automaton timestep, in the PDF's recipe order.
    ///
    /// Every transfer is computed from **one snapshot**; a cell that would give more than it
    /// holds has all its outgoing transfers scaled by one exact factor, and a receiver that
    /// would be overfilled has all its incoming transfers scaled by another — so scaling can
    /// neither create nor destroy volume, and the result is a function of the input alone,
    /// not of insertion order. If any cell's new pressure cannot be represented, the step
    /// refuses **before mutating anything**.
    pub fn step(&mut self, dt_seconds: Ratio) -> Refused<Step> {
        let dt = ratio_to_rational(dt_seconds, "timestep")?;
        // 1. The snapshot the whole step is computed from.
        let positions: Vec<VoxelPos> = self.cells.keys().copied().collect();
        let mut heads: Vec<Rational> = Vec::with_capacity(positions.len());
        let mut conductivities: Vec<Ratio> = Vec::with_capacity(positions.len());
        for at in &positions {
            let cell = &self.cells[at];
            heads.push(cell.head_m(at.z)?);
            conductivities.push(cell.effective_conductivity_m_s()?);
        }

        // 2. The faces: one edge per adjacent pair, from the lower coordinate, so a chunk
        //    seam is one edge and both sides read the same neighbour.
        let mut planned: Vec<(usize, usize, Rational)> = Vec::new();
        for (index, at) in positions.iter().enumerate() {
            for [dx, dy, dz] in POSITIVE_NEIGHBOURS {
                let Some(neighbour) = at.checked_add(dx, dy, dz).ok() else {
                    continue;
                };
                let Some(other) = positions.binary_search(&neighbour).ok() else {
                    continue;
                };
                let difference = sub(&heads[other], &heads[index], "head difference")?;
                let magnitude = abs(&difference)?;
                if magnitude.try_cmp(&ratio_to_rational(HEAD_TOLERANCE, "tolerance")?)?
                    != core::cmp::Ordering::Greater
                {
                    // At or below the declared tolerance, no flow is authorised.
                    continue;
                }
                let face = face_conductivity(&conductivities[index], &conductivities[other])?;
                if face.is_zero() {
                    continue;
                }
                let face = ratio_to_rational(face, "face conductivity")?;
                let amount = mul(&mul(&face, &magnitude, "transfer")?, &dt, "transfer")?;
                if amount.is_zero() {
                    continue;
                }
                // Flow runs down the potential: "from" carries the higher head.
                let (from, to) = if difference.is_positive() { (other, index) } else { (index, other) };
                planned.push((from, to, amount));
            }
        }

        // 3. Bound the plan exactly: no giver gives more than it holds, no receiver is
        //    overfilled, and every face is scaled **once**, by the smaller of its giver's and
        //    its receiver's bounds.
        //
        //    The arithmetic is `Rational`, not `Ratio`: `Ratio` is unsigned and *saturates*,
        //    and a saturated scale is a wrong scale that still looks like a ratio. `Rational`
        //    refuses on overflow, so a plan that cannot be represented refuses the step.
        let mut outgoing: Vec<Rational> = vec![Rational::ZERO; positions.len()];
        let mut incoming: Vec<Rational> = vec![Rational::ZERO; positions.len()];
        for (from, to, amount) in &planned {
            outgoing[*from] = add(&outgoing[*from], amount, "outgoing total")?;
            incoming[*to] = add(&incoming[*to], amount, "incoming total")?;
        }
        let mut factor: Vec<Rational> = vec![Rational::from_integer(1); positions.len()];
        for (index, at) in positions.iter().enumerate() {
            let cell = &self.cells[at];
            let available = ratio_to_rational(cell.volume_m3, "volume")?;
            if greater(&outgoing[index], &available)? {
                factor[index] = div(&available, &outgoing[index], "giver scale")?;
            }
            // A cell that both gives and receives is bounded by the free space it has now;
            // its own outflow is not counted as room for its inflow. The bound is
            // conservative, and a conservative bound is declared, never silently exceeded.
            let free = ratio_to_rational(cell.free_m3()?, "free pore volume")?;
            if greater(&incoming[index], &free)? {
                let receiver = div(&free, &incoming[index], "receiver scale")?;
                if less(&receiver, &factor[index])? {
                    factor[index] = receiver;
                }
            }
        }
        for (from, to, amount) in planned.iter_mut() {
            let scale = if less(&factor[*from], &factor[*to])? { factor[*from] } else { factor[*to] };
            *amount = mul(amount, &scale, "transfer")?;
        }
        planned.retain(|(_, _, amount)| !amount.is_zero());

        if planned.is_empty() {
            return Ok(Step::default());
        }

        // 4. The new state of every cell, computed before anything is written: a refusal is
        //    a refusal of the step, not half a step.
        let mut next: Vec<(Branch, Rational, Rational)> = Vec::with_capacity(positions.len());
        for (index, at) in positions.iter().enumerate() {
            let cell = &self.cells[at];
            let mut gained = Rational::ZERO;
            let mut lost = Rational::ZERO;
            for (from, to, amount) in &planned {
                if *from == index {
                    lost = add(&lost, amount, "outgoing total")?;
                }
                if *to == index {
                    gained = add(&gained, amount, "incoming total")?;
                }
            }
            let volume = ratio_to_rational(cell.volume_m3, "volume")?;
            let volume = sub(&add(&volume, &gained, "volume")?, &lost, "volume")?;
            let capacity = ratio_to_rational(cell.capacity_m3, "capacity")?;
            if greater(&volume, &capacity)? {
                return Err(Refusal::owned(
                    RefusalKind::OutOfDeclaredRange,
                    "the step's balance would overfill a cell, so the plan is refused whole",
                ));
            }
            let saturation = rational_to_ratio(&div(&volume, &capacity, "saturation")?, "saturation")?;
            let branch = if greater(&gained, &lost)? {
                Branch::Wetting
            } else if greater(&lost, &gained)? {
                Branch::Drying
            } else {
                cell.branch
            };
            let pc = cell.material.retention().curve(branch).at(saturation)?;
            let pressure = if cell.imposed {
                cell.pressure_pa
            } else {
                sub(&Rational::ZERO, &pc, "gauge pressure")?
            };
            // The front's imbibition rate, when the cell is wetting and unsaturated.
            let mut front = cell.front_m2;
            if branch == Branch::Wetting && !saturation.is_one() {
                let rate = imbibition_rate(&cell.fluid, &cell.material)?;
                front = add(&front, &mul(&rate, &dt, "front advance")?, "front")?;
            }
            next.push((branch, pressure, front));
        }

        // 5. Apply, in the lattice's canonical order.
        let mut moved = Rational::ZERO;
        for (from, to, amount) in &planned {
            let amount = rational_to_ratio(amount, "transfer")?;
            let from_cell = self.cells.get_mut(&positions[*from]).expect("a planned giver exists");
            from_cell.volume_m3 = from_cell.volume_m3.checked_sub(amount)?;
            let to_cell = self.cells.get_mut(&positions[*to]).expect("a planned receiver exists");
            to_cell.volume_m3 += amount;
            moved = add(&moved, &ratio_to_rational(amount, "transfer")?, "moved volume")?;
        }
        for (index, (branch, pressure, front)) in next.into_iter().enumerate() {
            let cell = self.cells.get_mut(&positions[index]).expect("a planned cell exists");
            cell.branch = branch;
            cell.pressure_pa = pressure;
            cell.front_m2 = front;
        }

        Ok(Step {
            transfers: planned.len(),
            volume_m3: rational_to_ratio(&moved, "moved volume")?,
        })
    }

    /// Run to stillness in at most [`SETTLE_CAP_PER_CELL`] × cells moving passes. A lattice
    /// still moving at the cap is refused, never silently stopped.
    pub fn settle(&mut self, dt_seconds: Ratio) -> Refused<Step> {
        let cap = SETTLE_CAP_PER_CELL * self.cells.len();
        let mut total = Step::default();
        for _ in 0..cap {
            let step = self.step(dt_seconds)?;
            if !step.moved() {
                return Ok(total);
            }
            total.transfers += step.transfers;
            total.volume_m3 += step.volume_m3;
        }
        Err(Refusal::owned(
            RefusalKind::Unimplemented,
            format!(
                "the lattice did not settle within {cap} moving passes; the cap is a declared \
                 boundary, not a silent stop"
            ),
        ))
    }
}

/// The face conductivity between two cells: the **series** combination of the two
/// half-cells, `2 K1 K2 / (K1 + K2)`.
///
/// The PDF's resistance picture — *a porous material can be modeled as a hydraulic
/// resistor* — says the face that separates two materials is two resistors in series, and
/// the series combination is the exact form that conserves the flux across the face. This is
/// the model's declared discretisation of `-K grad(H)`, not a measured value; a zero
/// conductivity on either side makes the face impermeable.
fn face_conductivity(left: &Ratio, right: &Ratio) -> Refused<Ratio> {
    if left.is_zero() || right.is_zero() {
        return Ok(Ratio::ZERO);
    }
    let sum = *left + *right;
    if sum.is_zero() {
        return Ok(Ratio::ZERO);
    }
    (*left * *right * Ratio::from_parts(2, 1)).checked_div(sum)
}

/// The PDF's hydrostatic pressure, `p = rho g h`, in pascals.
pub fn hydrostatic_pressure_pa(density_kg_m3: Ratio, depth_m: Ratio) -> Refused<Rational> {
    let weight = density_kg_m3 * gravity_m_s2();
    ratio_to_rational(weight * depth_m, "hydrostatic pressure")
}

/// The PDF's Young–Laplace capillary pressure, `Pc = 2 gamma cos(theta) / r`, in pascals.
pub fn capillary_pressure_pa(
    surface_tension: &SurfaceTension,
    contact: &ContactCosine,
    pore_radius: &PoreRadius,
) -> Refused<Rational> {
    let r = rational_to_ratio(&pore_radius.value(), "pore radius")?;
    if r.is_zero() {
        return Err(Refusal::owned(
            RefusalKind::ZeroRatio,
            "a zero pore radius has no capillary pressure",
        ));
    }
    let two_gamma = Ratio::from_parts(2, 1) * rational_to_ratio(&surface_tension.value(), "surface tension")?;
    let numerator = mul(
        &ratio_to_rational(two_gamma, "surface tension")?,
        &contact.value(),
        "capillary pressure",
    )?;
    div(&numerator, &ratio_to_rational(r, "pore radius")?, "capillary pressure")
}

/// The PDF's capillary head, `h = 2 gamma cos(theta) / (rho g r)`, in metres.
pub fn capillary_head_m(
    surface_tension: &SurfaceTension,
    contact: &ContactCosine,
    pore_radius: &PoreRadius,
    fluid: &Fluid,
) -> Refused<Rational> {
    let pc = capillary_pressure_pa(surface_tension, contact, pore_radius)?;
    let weight = ratio_to_rational(fluid.weight_density()?, "weight density")?;
    div(&pc, &weight, "capillary head")
}

/// The PDF's slab estimate, `Q = (k A / mu)(dp / L)`, in m³/s.
pub fn slab_flow_m3_s(
    permeability: &Permeability,
    area_m2: Ratio,
    pressure_drop_pa: Ratio,
    thickness_m: Ratio,
    viscosity: &Viscosity,
) -> Refused<Rational> {
    if thickness_m.is_zero() {
        return Err(Refusal::owned(
            RefusalKind::ZeroRatio,
            "a slab with no thickness has no flow",
        ));
    }
    let mu = rational_to_ratio(&viscosity.value(), "viscosity")?;
    if mu.is_zero() {
        return Err(Refusal::owned(
            RefusalKind::ZeroRatio,
            "a zero viscosity gives an infinite flow",
        ));
    }
    let k = rational_to_ratio(&permeability.value(), "permeability")?;
    let left = (k * area_m2).checked_div(mu)?;
    let right = pressure_drop_pa.checked_div(thickness_m)?;
    ratio_to_rational(left * right, "slab flow")
}

/// The PDF's Lucas–Washburn front, `L^2 = gamma r cos(theta) t / (2 mu)`, in m².
///
/// The squared front is exact; the front itself is a square root and is left as the squared
/// value rather than approximated.
pub fn imbibition_front_squared_m2(
    surface_tension: &SurfaceTension,
    pore_radius: &PoreRadius,
    contact: &ContactCosine,
    viscosity: &Viscosity,
    seconds: Ratio,
) -> Refused<Rational> {
    let two_mu = Ratio::from_parts(2, 1) * rational_to_ratio(&viscosity.value(), "viscosity")?;
    if two_mu.is_zero() {
        return Err(Refusal::owned(
            RefusalKind::ZeroRatio,
            "a zero viscosity gives an infinite imbibition rate",
        ));
    }
    let gamma_r = rational_to_ratio(&surface_tension.value(), "surface tension")?
        * rational_to_ratio(&pore_radius.value(), "pore radius")?;
    let rate = gamma_r.checked_div(two_mu)?;
    let placed = ratio_to_rational(rate * seconds, "front")?;
    mul(&placed, &contact.value(), "front")
}

/// The imbibition **rate** `d(L²)/dt = gamma r cos(theta) / (2 mu)`, in m²/s — the per-cell
/// front advance the step applies while a cell is on its wetting path.
fn imbibition_rate(fluid: &Fluid, material: &PorousMaterial) -> Refused<Rational> {
    imbibition_front_squared_m2(
        &fluid.surface_tension,
        material.pore_radius(),
        material.contact(),
        &fluid.viscosity,
        Ratio::ONE,
    )
}

/// Whether the fluidics rung is due at `tick`: `Stage::Hydrology`'s own declared cadence
/// (×4, H-8.3; C-88.4). Read from the stage, never restated.
pub fn is_due_at(tick: u64) -> bool {
    Stage::Hydrology.is_due_at(tick)
}

fn ratio_to_rational(value: Ratio, what: &str) -> Refused<Rational> {
    Rational::new(value.numerator() as i128, value.denominator() as i128).map_err(|_| {
        Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            format!("{what} {value} does not fit the exact rational number"),
        )
    })
}

fn rational_to_ratio(value: &Rational, what: &str) -> Refused<Ratio> {
    value.require_nonnegative(what)?;
    let numerator = u64::try_from(value.numerator()).map_err(|_| {
        Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            format!("{what} {} leaves the declared range", value.numerator()),
        )
    })?;
    let denominator = u64::try_from(value.denominator()).map_err(|_| {
        Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            format!("{what} {} leaves the declared range", value.denominator()),
        )
    })?;
    Ratio::new(numerator, denominator).map_err(|_| {
        Refusal::owned(
            RefusalKind::OutOfDeclaredRange,
            format!("{what} is not a ratio"),
        )
    })
}

fn add(left: &Rational, right: &Rational, what: &str) -> Refused<Rational> {
    left.checked_add(right).ok_or_else(|| overflow(what))
}

fn sub(left: &Rational, right: &Rational, what: &str) -> Refused<Rational> {
    left.checked_sub(right).ok_or_else(|| overflow(what))
}

fn mul(left: &Rational, right: &Rational, what: &str) -> Refused<Rational> {
    left.checked_mul(right).ok_or_else(|| overflow(what))
}

fn div(left: &Rational, right: &Rational, what: &str) -> Refused<Rational> {
    left.checked_div(right).ok_or_else(|| overflow(what))
}

fn abs(value: &Rational) -> Refused<Rational> {
    if value.is_negative() {
        sub(&Rational::ZERO, value, "magnitude")
    } else {
        Ok(*value)
    }
}

fn greater(left: &Rational, right: &Rational) -> Refused<bool> {
    Ok(left.try_cmp(right)? == core::cmp::Ordering::Greater)
}

fn less(left: &Rational, right: &Rational) -> Refused<bool> {
    Ok(left.try_cmp(right)? == core::cmp::Ordering::Less)
}

fn overflow(what: &str) -> Refusal {
    Refusal::owned(
        RefusalKind::OutOfDeclaredRange,
        format!("{what} left the exact rational range"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::matter::{FluidSpecies, Phase};

    fn water() -> FluidAddress {
        FluidAddress::new("water", FluidSpecies::Water, Phase::Liquid)
            .expect("water/liquid is a valid address")
    }

    /// The PDF's own worked-example values: rho = 1000 kg/m³, mu = 1e-3 Pa·s,
    /// gamma = 0.072 N/m, all declared with the PDF as their source.
    fn water_fluid() -> Fluid {
        Fluid::new(
            water(),
            Ratio::from_u64(1000),
            Viscosity::new(Rational::new(1, 1000).unwrap(), "the PDF's Example 3 value").unwrap(),
            SurfaceTension::new(Rational::new(18, 250).unwrap(), "the PDF's 0.072 N/m").unwrap(),
        )
        .unwrap()
    }

    fn temperature() -> Temperature {
        Temperature::new(Rational::from_integer(293), "a declared fixture, 20 C").unwrap()
    }

    /// A coarse declared fixture with **no suction** — retention is flat zero, so gravity
    /// alone decides. Chosen so one step can move a whole cell's volume.
    fn no_suction_material() -> PorousMaterial {
        flat_material(Rational::new(1, 1_000_000).unwrap(), Rational::from_integer(1))
    }

    /// A declared fixture whose conductivity is small enough that one step moves an exact
    /// fraction of a cell — for asserting the flux formula itself.
    fn slow_material() -> PorousMaterial {
        flat_material(Rational::new(1, 1_000_000_000).unwrap(), Rational::from_integer(1))
    }

    /// A declared fixture: no suction, a stated permeability, and a stated relative
    /// conductivity at every saturation.
    fn flat_material(permeability: Rational, relative_conductivity: Rational) -> PorousMaterial {
        let flat = Curve::retention(
            vec![(Ratio::ZERO, Rational::ZERO), (Ratio::ONE, Rational::ZERO)],
            "declared fixture: a coarse medium with no suction",
        )
        .unwrap();
        let full = Curve::conductivity(
            vec![
                (Ratio::ZERO, relative_conductivity),
                (Ratio::ONE, relative_conductivity),
            ],
            "declared fixture: a stated relative conductivity",
        )
        .unwrap();
        PorousMaterial::new(
            Permeability::new(permeability, "declared fixture").unwrap(),
            PoreRadius::new(Rational::new(1, 100_000).unwrap(), "declared fixture, 10 um").unwrap(),
            ContactCosine::new(Rational::from_integer(1), "declared fixture, a wetting pore").unwrap(),
            Retention::new(flat.clone(), flat).unwrap(),
            full,
        )
        .unwrap()
    }

    /// A declared fixture with capillary suction: Pc falls from 14 700 Pa dry to zero at
    /// saturation, in declared steps.
    fn sucking_material() -> PorousMaterial {
        let curve = Curve::retention(
            vec![
                (Ratio::ZERO, Rational::from_integer(14_700)),
                (Ratio::new(1, 2).unwrap(), Rational::from_integer(1000)),
                (Ratio::ONE, Rational::ZERO),
            ],
            "declared fixture: a fine medium",
        )
        .unwrap();
        let conductivity = Curve::conductivity(
            vec![
                (Ratio::ZERO, Rational::new(1, 100).unwrap()),
                (Ratio::ONE, Rational::from_integer(1)),
            ],
            "declared fixture: a drying-conductivity curve",
        )
        .unwrap();
        PorousMaterial::new(
            Permeability::new(Rational::new(1, 10_000_000_000).unwrap(), "declared fixture").unwrap(),
            PoreRadius::new(Rational::new(1, 100_000).unwrap(), "declared fixture, 10 um").unwrap(),
            ContactCosine::new(Rational::from_integer(1), "declared fixture, a wetting pore").unwrap(),
            Retention::new(curve.clone(), curve).unwrap(),
            conductivity,
        )
        .unwrap()
    }

    fn cell(material: PorousMaterial, capacity: Ratio, volume: Ratio) -> Cell {
        Cell::new(
            water_fluid(),
            material,
            capacity,
            volume,
            Branch::Drying,
            temperature(),
        )
        .unwrap()
    }

    fn two_cell_lattice() -> Lattice {
        let mut lattice = Lattice::new();
        let material = no_suction_material();
        lattice
            .insert(VoxelPos::new(0, 0, 0), cell(material.clone(), Ratio::from_parts(1, 2), Ratio::ZERO))
            .unwrap();
        lattice
            .insert(
                VoxelPos::new(0, 0, 1),
                cell(material, Ratio::from_parts(1, 2), Ratio::from_parts(1, 4)),
            )
            .unwrap();
        lattice
    }

    /// The PDF's Example 1: `p_g = rho g h`; a one-metre column is about 9.8 kPa.
    #[test]
    fn the_pdfs_one_metre_column_is_9_81_kpa() {
        let p = hydrostatic_pressure_pa(Ratio::from_u64(1000), Ratio::ONE).unwrap();
        assert_eq!(p, Rational::from_integer(9810));
    }

    /// The PDF's Example 2: a 10 µm wetting pore is 14.4 kPa and about 1.47 m of head.
    #[test]
    fn the_pdfs_ten_micrometre_pore_is_14_4_kpa_and_about_1_47_m_of_head() {
        let gamma = SurfaceTension::new(Rational::new(18, 250).unwrap(), "the PDF").unwrap();
        let contact = ContactCosine::new(Rational::from_integer(1), "theta = 0").unwrap();
        let radius = PoreRadius::new(Rational::new(1, 100_000).unwrap(), "10 um").unwrap();
        let pc = capillary_pressure_pa(&gamma, &contact, &radius).unwrap();
        assert_eq!(pc, Rational::from_integer(14_400));

        let head = capillary_head_m(&gamma, &contact, &radius, &water_fluid()).unwrap();
        // 160/109 m — between the PDF's 1.46 and its rounded 1.47, exactly.
        assert_eq!(head, Rational::new(160, 109).unwrap());
        assert!(head.try_cmp(&Rational::new(146, 100).unwrap()).unwrap() == core::cmp::Ordering::Greater);
        assert!(head.try_cmp(&Rational::new(147, 100).unwrap()).unwrap() == core::cmp::Ordering::Less);
    }

    /// The PDF's Example 3: the slab estimate is `1.0e-10 m³/s` — 0.36 mL an hour.
    #[test]
    fn the_pdfs_slab_is_1e_10_cubic_metres_a_second_and_0_36_ml_an_hour() {
        let k = Permeability::new(Rational::new(1, 10_000_000_000_000_000).unwrap(), "the PDF, 1e-16 m2").unwrap();
        let mu = Viscosity::new(Rational::new(1, 1000).unwrap(), "the PDF").unwrap();
        let q = slab_flow_m3_s(
            &k,
            Ratio::new(1, 100).unwrap(),
            Ratio::from_u64(10_000),
            Ratio::new(1, 10).unwrap(),
            &mu,
        )
        .unwrap();
        assert_eq!(q, Rational::new(1, 10_000_000_000).unwrap());
        // An hour of it is 3.6e-7 m³, and 0.36 of the millilitre (1e-6 m³).
        let hour = q.checked_mul(&Rational::from_integer(3600)).unwrap();
        assert_eq!(hour, Rational::new(9, 25_000_000).unwrap());
        assert_eq!(hour, Rational::new(9, 25).unwrap().checked_mul(&Rational::new(1, 1_000_000).unwrap()).unwrap());
    }

    /// The PDF's Example 4: after one second the front's **square** is 3.6e-4 m², so the
    /// front is between 18 and 19 mm (the PDF: about 19 mm) — and no square root is taken.
    #[test]
    fn the_pdfs_lucas_washburn_front_is_between_18_and_19_mm_in_one_second() {
        let gamma = SurfaceTension::new(Rational::new(18, 250).unwrap(), "the PDF").unwrap();
        let radius = PoreRadius::new(Rational::new(1, 100_000).unwrap(), "10 um").unwrap();
        let contact = ContactCosine::new(Rational::from_integer(1), "theta = 0").unwrap();
        let mu = Viscosity::new(Rational::new(1, 1000).unwrap(), "the PDF").unwrap();
        let l2 = imbibition_front_squared_m2(&gamma, &radius, &contact, &mu, Ratio::ONE).unwrap();
        assert_eq!(l2, Rational::new(9, 25_000).unwrap());
        // L < 19 mm exactly, because 0.019² > 9/25000; and L > 18 mm, because 0.018² < it.
        assert!(l2.try_cmp(&Rational::new(361, 1_000_000).unwrap()).unwrap() == core::cmp::Ordering::Less);
        assert!(l2.try_cmp(&Rational::new(324, 1_000_000).unwrap()).unwrap() == core::cmp::Ordering::Greater);
    }

    /// Gravity alone: the upper cell's water drains down and the books close exactly.
    #[test]
    fn a_two_cell_lattice_drains_from_the_top_down_and_conserves_exactly() {
        let mut lattice = two_cell_lattice();
        let before = lattice.total_volume_m3();
        assert_eq!(before, Ratio::from_parts(1, 4));

        lattice.settle(Ratio::new(1, 5).unwrap()).unwrap();

        let bottom = lattice.get(VoxelPos::new(0, 0, 0)).unwrap();
        let top = lattice.get(VoxelPos::new(0, 0, 1)).unwrap();
        assert_eq!(bottom.volume_m3(), Ratio::from_parts(1, 4), "the water did not reach the bottom");
        assert_eq!(top.volume_m3(), Ratio::ZERO, "the top cell kept water against gravity");
        assert_eq!(lattice.total_volume_m3(), before, "the books did not close");
    }

    /// A head difference at or below the declared millimetre authorises nothing.
    #[test]
    fn a_head_difference_at_or_below_one_millimetre_moves_nothing() {
        let material = no_suction_material();
        let mut lattice = Lattice::new();
        let mut left = cell(material.clone(), Ratio::ONE, Ratio::new(1, 2).unwrap());
        let right = cell(material, Ratio::ONE, Ratio::new(1, 2).unwrap());
        // Exactly the declared millimetre of head: p = rho g h = 9810/1000 Pa at z = 0.
        left.impose_pressure(Rational::new(981, 100).unwrap());
        lattice.insert(VoxelPos::new(0, 0, 0), left).unwrap();
        lattice.insert(VoxelPos::new(1, 0, 0), right).unwrap();
        let step = lattice.step(Ratio::new(1, 5).unwrap()).unwrap();
        assert_eq!(step, Step::default(), "a sub-tolerance head moved fluid");
    }

    /// The amount is the PDF's Darcy flux exactly, and it follows the potential gradient.
    #[test]
    fn water_moves_from_high_head_to_low_head_and_the_amount_is_the_declared_flow() {
        let material = slow_material();
        let mut lattice = Lattice::new();
        let mut high = cell(material.clone(), Ratio::ONE, Ratio::new(1, 2).unwrap());
        let mut low = cell(material, Ratio::ONE, Ratio::new(1, 2).unwrap());
        // One metre of head, imposed at both ends: p/(rho g) = 1 m.
        high.impose_pressure(Rational::from_integer(9810));
        low.impose_pressure(Rational::ZERO);
        lattice.insert(VoxelPos::new(0, 0, 0), high).unwrap();
        lattice.insert(VoxelPos::new(1, 0, 0), low).unwrap();

        let step = lattice.step(Ratio::new(1, 5).unwrap()).unwrap();
        // K = k rho g / mu = 981/100000 m/s; over one metre of head and 1/5 s that is
        // 981/500000 m³, exactly.
        assert_eq!(step.volume_m3, Ratio::new(981, 500_000).unwrap());
        assert_eq!(
            lattice.get(VoxelPos::new(1, 0, 0)).unwrap().volume_m3(),
            Ratio::from_parts(1, 2) + Ratio::new(981, 500_000).unwrap()
        );
        // And with the gradient reversed the flow reverses.
        let mut backwards = Lattice::new();
        let mut high = cell(slow_material(), Ratio::ONE, Ratio::new(1, 2).unwrap());
        let mut low = cell(slow_material(), Ratio::ONE, Ratio::new(1, 2).unwrap());
        high.impose_pressure(Rational::ZERO);
        low.impose_pressure(Rational::from_integer(9810));
        backwards.insert(VoxelPos::new(0, 0, 0), high).unwrap();
        backwards.insert(VoxelPos::new(1, 0, 0), low).unwrap();
        let step = backwards.step(Ratio::new(1, 5).unwrap()).unwrap();
        assert_eq!(step.volume_m3, Ratio::new(981, 500_000).unwrap());
        assert_eq!(
            backwards.get(VoxelPos::new(0, 0, 0)).unwrap().volume_m3(),
            Ratio::from_parts(1, 2) + Ratio::new(981, 500_000).unwrap()
        );
    }

    /// A dry cell's retention-derived pressure is **negative**, and that suction is what
    /// pulls water into it — the PDF's hidden pump.
    #[test]
    fn a_dry_cell_sucks_water_in_because_its_pressure_is_negative() {
        let material = sucking_material();
        let mut lattice = Lattice::new();
        // A wet cell at z = 1 above a dry cell at z = 0: gravity alone would take the water
        // down, and suction is what makes it enter the dry medium at all.
        lattice
            .insert(VoxelPos::new(0, 0, 0), cell(material.clone(), Ratio::ONE, Ratio::ZERO))
            .unwrap();
        lattice
            .insert(VoxelPos::new(0, 0, 1), cell(material, Ratio::ONE, Ratio::ONE))
            .unwrap();
        let dry = lattice.get(VoxelPos::new(0, 0, 0)).unwrap();
        assert!(dry.pressure_pa().is_negative(), "a dry cell's gauge pressure was not negative");
        assert_eq!(dry.pressure_pa(), Rational::from_integer(-14_700));

        let step = lattice.step(Ratio::new(1, 5).unwrap()).unwrap();
        assert!(!step.volume_m3.is_zero(), "suction moved nothing");
        assert!(lattice.get(VoxelPos::new(0, 0, 0)).unwrap().volume_m3() > Ratio::ZERO);
    }

    /// Hysteresis: the same saturation has two pressures, one per branch.
    #[test]
    fn wetting_and_drying_are_two_curves_and_the_same_saturation_gives_two_pressures() {
        let wetting = Curve::retention(
            vec![(Ratio::ZERO, Rational::from_integer(14_700)), (Ratio::ONE, Rational::ZERO)],
            "declared fixture: the wetting path",
        )
        .unwrap();
        let drying = Curve::retention(
            vec![(Ratio::ZERO, Rational::from_integer(29_400)), (Ratio::ONE, Rational::ZERO)],
            "declared fixture: the drying path",
        )
        .unwrap();
        let retention = Retention::new(wetting.clone(), drying.clone()).unwrap();
        let half = Ratio::new(1, 2).unwrap();
        assert_eq!(wetting.at(half).unwrap(), Rational::new(14_700, 2).unwrap());
        assert_eq!(drying.at(half).unwrap(), Rational::new(29_400, 2).unwrap());
        assert_ne!(retention.curve(Branch::Wetting).at(half), retention.curve(Branch::Drying).at(half));
    }

    /// A lattice is a function of its cells, never of the order they were inserted in.
    #[test]
    fn the_same_lattice_is_the_same_world_whatever_the_insertion_order() {
        let forward = |reverse: bool| {
            let material = no_suction_material();
            let mut lattice = Lattice::new();
            let cells = vec![
                (VoxelPos::new(0, 0, 0), cell(material.clone(), Ratio::ONE, Ratio::ZERO)),
                (VoxelPos::new(1, 0, 0), cell(material.clone(), Ratio::ONE, Ratio::new(1, 4).unwrap())),
                (VoxelPos::new(1, 0, 1), cell(material, Ratio::ONE, Ratio::new(1, 4).unwrap())),
            ];
            let order: Vec<_> = if reverse { cells.into_iter().rev().collect() } else { cells };
            for (at, cell) in order {
                lattice.insert(at, cell).unwrap();
            }
            lattice.step(Ratio::new(1, 5).unwrap()).unwrap();
            lattice
        };
        let a = forward(false);
        let b = forward(true);
        assert_eq!(a, b, "insertion order changed the world");
    }

    /// A chunk seam is one edge: the cells either side of x = 0 are in different chunks, and
    /// the flow crosses the seam with both sides reading the same neighbour.
    #[test]
    fn a_chunk_seam_is_one_edge_and_the_flow_crosses_it() {
        let before = VoxelPos::new(-1, 0, 0);
        let after = VoxelPos::new(0, 0, 0);
        assert_ne!(before.chunk(), after.chunk(), "the chosen cells are not across a chunk seam");
        assert_eq!(before.chunk().x, -1);

        let material = slow_material();
        let mut lattice = Lattice::new();
        let mut high = cell(material.clone(), Ratio::ONE, Ratio::new(1, 2).unwrap());
        let mut low = cell(material, Ratio::ONE, Ratio::new(1, 2).unwrap());
        high.impose_pressure(Rational::from_integer(9810));
        low.impose_pressure(Rational::ZERO);
        lattice.insert(before, high).unwrap();
        lattice.insert(after, low).unwrap();
        let step = lattice.step(Ratio::new(1, 5).unwrap()).unwrap();
        assert_eq!(step.transfers, 1, "the seam was not one edge");
        assert_eq!(step.volume_m3, Ratio::new(981, 500_000).unwrap());
    }

    /// A property with no source is refused by name, and no physical value can be built
    /// without one.
    #[test]
    fn a_property_without_a_source_is_refused_by_name() {
        let refusal = Permeability::new(Rational::from_integer(1), "   ").unwrap_err();
        assert!(refusal.detail().contains("invented value"), "got {refusal}");
        assert!(Sourced::new(1u8, "").is_err());
    }

    /// A curve must declare both ends and strictly ascend; a constant is not a relation.
    #[test]
    fn a_curve_must_declare_both_ends_and_rise() {
        assert!(Curve::retention(vec![(Ratio::ZERO, Rational::ZERO)], "x").is_err());
        assert!(Curve::retention(
            vec![(Ratio::new(1, 2).unwrap(), Rational::ZERO), (Ratio::ONE, Rational::ZERO)],
            "x"
        )
        .is_err());
        assert!(Curve::retention(
            vec![(Ratio::ZERO, Rational::ZERO), (Ratio::new(1, 2).unwrap(), Rational::ZERO)],
            "x"
        )
        .is_err());
        assert!(Curve::retention(
            vec![
                (Ratio::ZERO, Rational::ZERO),
                (Ratio::new(1, 2).unwrap(), Rational::from_integer(5)),
                (Ratio::ONE, Rational::from_integer(1)),
            ],
            "x"
        )
        .is_err(), "a retention curve that does not reach zero at saturation was accepted");
        // A conductivity above one is not a relative conductivity.
        assert!(Curve::conductivity(
            vec![(Ratio::ZERO, Rational::from_integer(2)), (Ratio::ONE, Rational::from_integer(1))],
            "x"
        )
        .is_err());
    }

    /// A cell refuses to hold more than its pore volume, and a drain refuses to remove what
    /// is not there.
    #[test]
    fn a_cell_refuses_to_hold_more_than_its_pore_volume() {
        let material = no_suction_material();
        let refusal = Cell::new(
            water_fluid(),
            material.clone(),
            Ratio::new(1, 2).unwrap(),
            Ratio::new(3, 5).unwrap(),
            Branch::Drying,
            temperature(),
        )
        .unwrap_err();
        assert!(refusal.detail().contains("pore volume"), "got {refusal}");

        let mut cell = cell(material, Ratio::new(1, 2).unwrap(), Ratio::new(1, 4).unwrap());
        let refusal = cell.drain(Ratio::new(1, 3).unwrap()).unwrap_err();
        assert!(refusal.detail().contains("withdrawn") || refusal.detail().contains("negative"));
        assert_eq!(cell.volume_m3(), Ratio::new(1, 4).unwrap(), "a refused drain changed the cell");
    }

    /// A curve interpolates exactly between its declared points, both sides of a breakpoint.
    #[test]
    fn a_curve_interpolates_exactly_between_its_declared_points() {
        let curve = sucking_material();
        let retention = curve.retention().curve(Branch::Drying);
        assert_eq!(retention.at(Ratio::ZERO).unwrap(), Rational::from_integer(14_700));
        assert_eq!(retention.at(Ratio::new(1, 4).unwrap()).unwrap(), Rational::from_integer(7_850));
        assert_eq!(retention.at(Ratio::new(1, 2).unwrap()).unwrap(), Rational::from_integer(1000));
        assert_eq!(retention.at(Ratio::new(3, 4).unwrap()).unwrap(), Rational::from_integer(500));
        assert_eq!(retention.at(Ratio::ONE).unwrap(), Rational::ZERO);
        assert!(retention.at(Ratio::new(3, 2).unwrap()).is_err(), "S > 1 was evaluated");
    }

    /// The fixed-point boundary refuses a pressure its declared scale cannot represent.
    #[test]
    fn a_pressure_the_declared_scale_cannot_represent_refuses() {
        let material = no_suction_material();
        let mut cell = cell(material, Ratio::ONE, Ratio::ZERO);
        cell.impose_pressure(Rational::new(1, 3).unwrap());
        assert!(cell.pressure_fixed(1000).is_err(), "a third of a pascal was rounded to fit");
        cell.impose_pressure(Rational::from_integer(9810));
        let fixed = cell.pressure_fixed(1000).unwrap();
        assert_eq!(fixed.raw(), 9_810_000);
        assert_eq!(fixed.scale(), 1000);
    }

    /// A cell giving to two neighbours at once never gives more than it holds: the step's
    /// outgoing transfers are scaled by one exact factor.
    #[test]
    fn a_cell_never_gives_more_than_it_holds_when_two_neighbours_ask() {
        let material = no_suction_material();
        let mut lattice = Lattice::new();
        let mut centre = cell(material.clone(), Ratio::new(1, 100).unwrap(), Ratio::new(1, 1000).unwrap());
        centre.impose_pressure(Rational::from_integer(9810));
        lattice.insert(VoxelPos::new(0, 0, 0), centre).unwrap();
        for at in [VoxelPos::new(1, 0, 0), VoxelPos::new(0, 1, 0)] {
            let mut sink = cell(material.clone(), Ratio::ONE, Ratio::ZERO);
            sink.impose_pressure(Rational::ZERO);
            lattice.insert(at, sink).unwrap();
        }
        let before = lattice.total_volume_m3();
        lattice.step(Ratio::new(1, 5).unwrap()).unwrap();
        assert_eq!(lattice.total_volume_m3(), before, "scaling did not conserve volume");
        assert!(lattice.get(VoxelPos::new(0, 0, 0)).unwrap().volume_m3() <= Ratio::new(1, 1000).unwrap());
    }

    /// The conductivity curve scales the flow exactly: the same geometry and volumes, one
    /// material at `K_rel = 1` and one at `K_rel = 1/2`, move volumes in the ratio two to one.
    #[test]
    fn the_conductivity_curve_scales_the_flow() {
        let moved = |relative: Rational| {
            let material = flat_material(Rational::new(1, 1_000_000_000).unwrap(), relative);
            let mut lattice = Lattice::new();
            let mut giver = cell(material.clone(), Ratio::ONE, Ratio::new(1, 2).unwrap());
            giver.impose_pressure(Rational::from_integer(9810));
            let mut taker = cell(material, Ratio::ONE, Ratio::new(1, 2).unwrap());
            taker.impose_pressure(Rational::ZERO);
            lattice.insert(VoxelPos::new(0, 0, 0), giver).unwrap();
            lattice.insert(VoxelPos::new(1, 0, 0), taker).unwrap();
            lattice.step(Ratio::new(1, 5).unwrap()).unwrap().volume_m3
        };
        let whole = moved(Rational::from_integer(1));
        let halved = moved(Rational::new(1, 2).unwrap());
        assert_eq!(whole, Ratio::new(981, 500_000).unwrap());
        assert_eq!(halved, Ratio::new(981, 1_000_000).unwrap());
        assert_eq!(whole, halved * Ratio::from_parts(2, 1));
    }

    /// An imposed pressure is a boundary condition: it survives the step, while an interior
    /// cell's pressure follows its own retention curve.
    #[test]
    fn an_imposed_pressure_is_a_boundary_condition() {
        let material = sucking_material();
        let mut boundary = cell(material.clone(), Ratio::ONE, Ratio::new(1, 2).unwrap());
        boundary.impose_pressure(Rational::from_integer(9810));
        let interior = cell(material, Ratio::ONE, Ratio::new(1, 2).unwrap());
        let interior_before = interior.pressure_pa();
        assert!(interior_before != Rational::from_integer(9810));
        let mut lattice = Lattice::new();
        lattice.insert(VoxelPos::new(0, 0, 0), boundary).unwrap();
        lattice.insert(VoxelPos::new(1, 0, 0), interior).unwrap();
        lattice.step(Ratio::new(1, 5).unwrap()).unwrap();
        assert_eq!(
            lattice.get(VoxelPos::new(0, 0, 0)).unwrap().pressure_pa(),
            Rational::from_integer(9810)
        );
        assert_ne!(
            lattice.get(VoxelPos::new(1, 0, 0)).unwrap().pressure_pa(),
            Rational::from_integer(9810),
            "the interior cell kept the boundary's pressure"
        );
    }

    /// The six-neighbour rule: the three positive axis steps reach three neighbours, and a
    /// diagonal is not adjacent.
    #[test]
    fn the_lattice_is_six_neighbours_and_z_is_up() {
        let material = no_suction_material();
        let mut lattice = Lattice::new();
        let centre = VoxelPos::new(0, 0, 0);
        lattice.insert(centre, cell(material.clone(), Ratio::ONE, Ratio::ZERO)).unwrap();
        for [dx, dy, dz] in [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]] {
            let at = centre.checked_add(dx, dy, dz).unwrap();
            assert_ne!(at, centre);
            assert_eq!(at.manhattan(centre), 1);
        }
        // Two diagonals are not neighbours: a face is never a corner.
        let diagonal = centre.checked_add(1, 1, 0).unwrap();
        assert_eq!(diagonal.manhattan(centre), 2);
        // z is up: the vertical pair differs in z and shares x and y.
        let up = centre.checked_add(0, 0, 1).unwrap();
        assert_eq!((up.x, up.y), (centre.x, centre.y));
        assert_eq!(up.z, centre.z + 1);
    }

    /// The fill level is the world's own fill scale, floored from the exact saturation,
    /// and it is the interpolation value the user named (rain, water, molten metal).
    #[test]
    fn the_fill_level_is_fill_side_floored_and_never_drawn_above_the_books() {
        assert_eq!(FILL_LEVELS, FILL_SIDE);
        let material = no_suction_material();
        assert_eq!(cell(material.clone(), Ratio::ONE, Ratio::ZERO).fill_level().unwrap(), FillLevel::EMPTY);
        assert_eq!(
            cell(material.clone(), Ratio::ONE, Ratio::new(1, 2).unwrap()).fill_level().unwrap(),
            FillLevel::new(FILL_LEVELS / 2).unwrap()
        );
        assert_eq!(cell(material.clone(), Ratio::ONE, Ratio::ONE).fill_level().unwrap(), FillLevel::FULL);
        // A third is floored to 10 of 32, never rounded up to 11: the surface is never
        // drawn above where the fluid is.
        let third = cell(material, Ratio::from_u64(3), Ratio::ONE).fill_level().unwrap();
        assert_eq!(third.raw(), FILL_LEVELS / 3);
        let expected = Ratio::new((FILL_LEVELS / 3) as u64, FILL_LEVELS as u64).unwrap();
        assert_eq!(third.fraction().unwrap(), expected);
        assert_eq!(third.surface_height_m().unwrap(), Rational::new(5, 16).unwrap());
        // A level outside the declared scale refuses by name rather than clamping.
        assert!(FillLevel::new(FILL_LEVELS + 1).is_err());
        assert_eq!(FillLevel::FULL.surface_height_m().unwrap(), Rational::from_integer(1));
    }

    /// The fluidics rung is the hydrology stage's own ×4 cadence, read from the stage.
    #[test]
    fn the_fluidics_rung_is_the_hydrology_stages_own_rung() {
        for tick in 0..=8u64 {
            assert_eq!(is_due_at(tick), Stage::Hydrology.is_due_at(tick), "tick {tick}");
        }
        assert!(is_due_at(0) && is_due_at(4) && is_due_at(8));
        assert!(!is_due_at(1));
    }
}
