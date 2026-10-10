use std::fmt;
use std::ops::RangeInclusive;

use crate::matter::Rational;
// **The governed book, not a world.** `DeltaStore` is the tree's one store a change is
// committed into (`world/book.rs`); the world itself is the game's, composed with the
// earthworks record. `coord_text` is the record's own spelling of a coordinate, used here
// because this tree's `VoxelPos` deliberately carries no `Display`.
use crate::world::{
    coord_text, ActorId, ComponentId, Coord, DeltaStore, PhysicalVerb, ReservationId, TaskId,
};

use super::{ActorError, ToolId, VerbPolicy};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CuboidSelection {
    pub min: Coord,
    pub max: Coord,
}

impl CuboidSelection {
    pub fn new(min: Coord, max: Coord) -> Result<Self, SelectionError> {
        if min.x > max.x || min.y > max.y || min.z > max.z {
            return Err(SelectionError::InvalidBounds);
        }
        Ok(Self { min, max })
    }

    pub fn contains(self, coord: Coord) -> bool {
        (self.min.x..=self.max.x).contains(&coord.x)
            && (self.min.y..=self.max.y).contains(&coord.y)
            && (self.min.z..=self.max.z).contains(&coord.z)
    }

    pub fn cells(self) -> Result<u64, SelectionError> {
        let x = checked_volume(self.min.x, self.max.x)?;
        let y = checked_volume(self.min.y, self.max.y)?;
        let z = checked_volume(self.min.z, self.max.z)?;
        x.checked_mul(y)
            .and_then(|value| value.checked_mul(z))
            .ok_or(SelectionError::VolumeOverflow)
    }

    pub(crate) fn coords(self) -> impl Iterator<Item = Coord> {
        let ys = inclusive(self.min.y, self.max.y);
        let zs = inclusive(self.min.z, self.max.z);
        inclusive(self.min.x, self.max.x).flat_map(move |x| {
            let ys = ys.clone();
            let zs = zs.clone();
            ys.flat_map(move |y| {
                let zs = zs.clone();
                zs.map(move |z| Coord::new(x, y, z))
            })
        })
    }
}

fn inclusive(min: i32, max: i32) -> RangeInclusive<i32> {
    min..=max
}

fn checked_volume(min: i32, max: i32) -> Result<u64, SelectionError> {
    let count = i64::from(max) - i64::from(min) + 1;
    u64::try_from(count).map_err(|_| SelectionError::VolumeOverflow)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstimateValue {
    Known(Rational),
    Unavailable(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialRequirement {
    Component {
        component: ComponentId,
        amount: Rational,
    },
    NoMaterial(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionEstimate {
    pub verb: PhysicalVerb,
    pub cells: u64,
    pub intersected_cells: u64,
    pub work_ticks: u64,
    pub mass_before: Rational,
    pub mass_after: Rational,
    pub required_material: Rational,
    pub material_requirement: MaterialRequirement,
    pub target_amounts: Vec<(Coord, Rational)>,
    pub fluid: EstimateValue,
    pub risk: EstimateValue,
    pub targets: Vec<Coord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionBudget {
    pub max_cells: u64,
    pub max_work: u64,
    pub work_per_cell: u64,
}

impl Default for SelectionBudget {
    fn default() -> Self {
        Self {
            max_cells: 4_096,
            max_work: 8_192,
            work_per_cell: 2,
        }
    }
}

pub fn estimate_selection(
    store: &DeltaStore,
    selection: CuboidSelection,
    verb: PhysicalVerb,
) -> Result<SelectionEstimate, SelectionError> {
    estimate_selection_bounded(store, selection, verb, SelectionBudget::default())
}

pub fn estimate_selection_bounded(
    store: &DeltaStore,
    selection: CuboidSelection,
    verb: PhysicalVerb,
    budget: SelectionBudget,
) -> Result<SelectionEstimate, SelectionError> {
    let cells = selection.cells()?;
    if cells > budget.max_cells {
        return Err(SelectionError::BudgetExceeded {
            requested: cells,
            maximum: budget.max_cells,
        });
    }
    let prospective_work = cells
        .checked_mul(budget.work_per_cell)
        .ok_or(SelectionError::ArithmeticOverflow)?;
    if prospective_work > budget.max_work {
        return Err(SelectionError::BudgetExceeded {
            requested: prospective_work,
            maximum: budget.max_work,
        });
    }
    let snapshot = store
        .conservation_snapshot()
        .map_err(|error| SelectionError::World(error.to_string()))?;
    let mut intersected_cells = 0_u64;
    let mut mass_before = Rational::ZERO;
    let mut component = None;
    let mut target_amounts = Vec::new();
    // **A reading, not a sum.** The governed book keeps no fluid mass, so this stays zero
    // unless every cell's provenance is silent about fluids — see the fluid rule below.
    let fluid = Rational::ZERO;
    let mut fluid_available = true;
    let mut targets = Vec::new();
    for coord in selection.coords() {
        let occupancy = snapshot
            .occupancy
            .get(&coord)
            .copied()
            .ok_or(SelectionError::UnsupportedCell(coord))?;
        let provenance = snapshot
            .occupancy_provenance
            .get(&coord)
            .ok_or(SelectionError::UnsupportedCell(coord))?;
        let _ = occupancy;
        intersected_cells = intersected_cells
            .checked_add(1)
            .ok_or(SelectionError::VolumeOverflow)?;
        targets.push(coord);
        if !matches!(verb, PhysicalVerb::Use | PhysicalVerb::Craft) {
            let amounts = snapshot
                .component_accounts
                .iter()
                .filter_map(|((candidate, candidate_id), amount)| {
                    (candidate == &provenance.account).then_some((candidate_id, amount))
                })
                .collect::<Vec<_>>();
            let [(candidate_id, amount)] = amounts.as_slice() else {
                return Err(SelectionError::MultipleComponents(coord));
            };
            match &component {
                None => component = Some((*candidate_id).clone()),
                Some(selected) if selected == *candidate_id => {}
                Some(_) => return Err(SelectionError::MixedComponents(coord)),
            }
            mass_before = mass_before
                .checked_add(amount)
                .ok_or(SelectionError::ArithmeticOverflow)?;
            // The amount is owned here: `Rational` is `Copy`, and a row that borrowed from
            // the snapshot would make the estimate outlive the book it was read from.
            target_amounts.push((coord, **amount));
        }
        // **A fluid address is an unreadable quantity in the governed book, and an absence
        // is not.** The book keeps no fluid mass (the hydrology is `voxel-shapers`', `event-0133`),
        // so a cell whose provenance names a fluid has no reading — `Unavailable`, never a
        // zero dressed as one. A cell with no fluid address has nothing to read and reads
        // `Known(ZERO)`: the quantity is genuinely zero, not missing.
        if provenance.fluid_address.is_some() {
            fluid_available = false;
        }
    }
    if intersected_cells == 0 {
        return Err(SelectionError::NoSupportedCells);
    }
    let work_ticks = prospective_work;
    let required_material = match verb {
        PhysicalVerb::Fill | PhysicalVerb::Place => {
            Rational::from_integer(intersected_cells as i128)
        }
        PhysicalVerb::Break | PhysicalVerb::Gather | PhysicalVerb::Dig => mass_before,
        PhysicalVerb::Use | PhysicalVerb::Craft => Rational::ZERO,
    };
    let mass_after = match verb {
        PhysicalVerb::Break | PhysicalVerb::Gather | PhysicalVerb::Dig => Rational::ZERO,
        PhysicalVerb::Fill | PhysicalVerb::Place => mass_before
            .checked_add(&required_material)
            .ok_or(SelectionError::ArithmeticOverflow)?,
        PhysicalVerb::Use | PhysicalVerb::Craft => mass_before,
    };
    let material_requirement = match verb {
        PhysicalVerb::Use | PhysicalVerb::Craft => {
            MaterialRequirement::NoMaterial("use/craft consume no material")
        }
        PhysicalVerb::Break
        | PhysicalVerb::Gather
        | PhysicalVerb::Dig
        | PhysicalVerb::Fill
        | PhysicalVerb::Place => MaterialRequirement::Component {
            component: component.ok_or(SelectionError::MixedComponents(selection.min))?,
            amount: required_material,
        },
    };
    Ok(SelectionEstimate {
        verb,
        cells,
        intersected_cells,
        work_ticks,
        mass_before,
        mass_after,
        required_material,
        material_requirement,
        target_amounts,
        fluid: if fluid_available {
            EstimateValue::Known(fluid)
        } else {
            EstimateValue::Unavailable("the governed book keeps no fluid mass")
        },
        risk: EstimateValue::Unavailable("draft risk source"),
        targets,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanId(u64);

impl PlanId {
    pub fn new(value: impl Into<String>) -> Result<Self, SelectionError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SelectionError::InvalidPlanId);
        }
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in value.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Ok(Self(hash))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for PlanId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "selection:{:016x}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanState {
    Reserved,
    Committed,
    Cancelled,
    Refused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionPreview {
    pub verb: PhysicalVerb,
    pub bounds: CuboidSelection,
    pub targets: Vec<Coord>,
    pub estimate: SelectionEstimate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionPlan {
    pub id: PlanId,
    pub actor: ActorId,
    pub task: Option<TaskId>,
    pub tool: Option<ToolId>,
    pub capability: PhysicalVerb,
    pub policy: VerbPolicy,
    pub selection: CuboidSelection,
    pub targets: Vec<Coord>,
    pub reservations: Vec<ReservationId>,
    pub preview: SelectionPreview,
    pub state: PlanState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionError {
    InvalidBounds,
    InvalidPlanId,
    VolumeOverflow,
    NoSupportedCells,
    ArithmeticOverflow,
    BudgetExceeded { requested: u64, maximum: u64 },
    UnsupportedCell(Coord),
    MultipleComponents(Coord),
    MixedComponents(Coord),
    OutsideDomain(Coord),
    EstimateUnavailable(&'static str),
    Actor(ActorError),
    World(String),
}

impl fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBounds => {
                formatter.write_str("cuboid minimum bounds must not exceed maximum bounds")
            }
            Self::InvalidPlanId => formatter.write_str("selection plan id must not be empty"),
            Self::VolumeOverflow => formatter.write_str("selection volume overflow"),
            Self::NoSupportedCells => formatter.write_str("selection has no cells"),
            Self::ArithmeticOverflow => {
                formatter.write_str("selection estimate arithmetic overflow")
            }
            Self::BudgetExceeded { requested, maximum } => write!(
                formatter,
                "selection work {requested} exceeds configured maximum {maximum}"
            ),
            Self::UnsupportedCell(coord) => write!(
                formatter,
                "selection cell {} has no supported occupancy",
                coord_text(*coord)
            ),
            Self::MultipleComponents(coord) => write!(
                formatter,
                "selection cell {} has multiple component amounts",
                coord_text(*coord)
            ),
            Self::MixedComponents(coord) => write!(
                formatter,
                "selection cell {} does not use the selected component",
                coord_text(*coord)
            ),
            Self::OutsideDomain(coord) => write!(
                formatter,
                "selection cell {} is outside the world domain",
                coord_text(*coord)
            ),
            Self::EstimateUnavailable(source) => {
                write!(formatter, "selection estimate source unavailable: {source}")
            }
            Self::Actor(error) => write!(formatter, "{error}"),
            Self::World(reason) => write!(formatter, "selection world read failed: {reason}"),
        }
    }
}

impl std::error::Error for SelectionError {}

impl From<ActorError> for SelectionError {
    fn from(error: ActorError) -> Self {
        Self::Actor(error)
    }
}
