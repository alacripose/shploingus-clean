//! **Shapers — two kinds × two levels, and a rule that is a type.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §2 F3 (matter is layered; the **stored** level is the 4×4×4
//! lattice and the **addressed** level is 256×) and §2 F2 (a material is a named set of components,
//! and a shape is what determines which cells hold matter).
//!
//! # The two kinds are a rule, not a label
//!
//! A **physical** shaper changes occupancy: it may fill or empty cells. A **material** shaper may
//! not — it may only recolour cells that already hold matter. That is the difference between carving
//! a shape out of a block and painting one, and it is enforced by [`apply`], which refuses a
//! material change of occupancy **by name**.
//!
//! # `acts_at` is required, and a claim with nothing behind it is refused
//!
//! Every shaper declares the levels it acts at. A shaper that claims the **addressed** level and
//! declares **no field** there is refused — otherwise "acts at 256×" is decorative, and the level
//! stops meaning anything. That is the rule [`ShaperSet::admit`] enforces.

use crate::coordinates::SubvoxelIndex;
use crate::material::MaterialId;
use crate::refusal::Refusal;

/// Which rule a shaper obeys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShaperKind {
    /// May change occupancy: fill empty cells and empty filled ones.
    Physical,
    /// May **not** change occupancy: it may only recolour cells that already hold matter.
    Material,
}

/// The two levels a shaper may act at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// The **stored** 4×4×4 lattice — shape and composition, 64 cells.
    Stored,
    /// The **addressed** 256× detail lattice — appearance, evaluated not stored.
    Addressed,
}

impl Level {
    pub const ALL: [Level; 2] = [Level::Stored, Level::Addressed];

    pub const fn name(self) -> &'static str {
        match self {
            Level::Stored => "stored",
            Level::Addressed => "addressed",
        }
    }

    const fn index(self) -> usize {
        match self {
            Level::Stored => 0,
            Level::Addressed => 1,
        }
    }
}

/// A change a shaper asks to make to one stored cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellChange {
    /// Put matter in a cell (occupancy may change).
    Fill(MaterialId),
    /// Remove matter from a cell (occupancy may change).
    Empty,
    /// Change the material of a cell that already holds matter (occupancy may not change).
    Recolour(MaterialId),
}

/// The **stored** 4×4×4 lattice of one voxel: which cells hold matter, and of what material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredLattice {
    cells: [Option<MaterialId>; 64],
}

impl StoredLattice {
    pub const fn empty() -> Self {
        StoredLattice { cells: [None; 64] }
    }

    pub fn get(&self, cell: SubvoxelIndex) -> Option<MaterialId> {
        self.cells[cell.flat() as usize]
    }

    fn set(&mut self, cell: SubvoxelIndex, value: Option<MaterialId>) {
        self.cells[cell.flat() as usize] = value;
    }

    /// How many cells hold matter. The `Tally` the plan says must be a printed number.
    pub fn occupancy_count(&self) -> usize {
        self.cells.iter().filter(|cell| cell.is_some()).count()
    }
}

/// **Apply one shaper change under the rule its kind obeys.** A material shaper cannot change
/// occupancy in either direction, and a recolour of an empty cell is refused because there is
/// nothing there to recolour.
pub fn apply(
    kind: ShaperKind,
    lattice: &mut StoredLattice,
    cell: SubvoxelIndex,
    change: CellChange,
) -> Result<(), Refusal> {
    let occupied = lattice.get(cell).is_some();
    match change {
        CellChange::Fill(material) => {
            if kind == ShaperKind::Material {
                return Err(Refusal::new(
                    crate::refusal::RefusalKind::MaterialShaperCannotChangeOccupancy,
                    format!("a material shaper cannot fill cell {}", cell.flat()),
                ));
            }
            lattice.set(cell, Some(material));
        }
        CellChange::Empty => {
            if kind == ShaperKind::Material {
                return Err(Refusal::new(
                    crate::refusal::RefusalKind::MaterialShaperCannotChangeOccupancy,
                    format!("a material shaper cannot empty cell {}", cell.flat()),
                ));
            }
            lattice.set(cell, None);
        }
        CellChange::Recolour(material) => {
            if !occupied {
                return Err(Refusal::new(
                    crate::refusal::RefusalKind::NoCellToRecolour,
                    format!("cell {} is empty", cell.flat()),
                ));
            }
            lattice.set(cell, Some(material));
        }
    }
    Ok(())
}

/// What a shaper declares: its name, its kind, and the levels it acts at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShaperDeclaration {
    pub name: &'static str,
    pub kind: ShaperKind,
    /// `[stored, addressed]` — the levels it acts at. Empty is refused.
    pub levels: [bool; 2],
    /// Whether a **field** is declared at the addressed level. Required when `levels[1]` is set.
    pub declares_addressed_field: bool,
}

impl ShaperDeclaration {
    fn acts_at(&self, level: Level) -> bool {
        self.levels[level.index()]
    }
}

/// **The set.** Every admitted shaper declares the levels it acts at, and the set's coverage is a
/// printed grid rather than an assertion.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShaperSet {
    declarations: Vec<ShaperDeclaration>,
}

impl ShaperSet {
    pub const fn new() -> Self {
        ShaperSet { declarations: Vec::new() }
    }

    /// Admit a shaper, refusing by name: a name already admitted, a declaration that names no level,
    /// or a claim on the addressed level with no field declared behind it.
    pub fn admit(&mut self, declaration: ShaperDeclaration) -> Result<(), Refusal> {
        if self.declarations.iter().any(|d| d.name == declaration.name) {
            return Err(Refusal::new(
                crate::refusal::RefusalKind::DuplicateShaper,
                format!("shaper {:?} is already admitted", declaration.name),
            ));
        }
        if !declaration.levels[0] && !declaration.levels[1] {
            return Err(Refusal::new(
                crate::refusal::RefusalKind::ShaperDeclaresNothing,
                format!("shaper {:?} acts at no level", declaration.name),
            ));
        }
        if declaration.levels[Level::Addressed.index()] && !declaration.declares_addressed_field {
            return Err(Refusal::new(
                crate::refusal::RefusalKind::ShaperDeclaresNothing,
                format!(
                    "shaper {:?} claims the addressed level and declares no field",
                    declaration.name
                ),
            ));
        }
        self.declarations.push(declaration);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.declarations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }

    /// The 2×2 coverage grid `[kind][level]` — how many admitted shapers occupy each cell. A cell at
    /// zero is a taxonomy this build does not exercise, and it is visible rather than implied.
    pub fn coverage(&self) -> [[usize; 2]; 2] {
        let mut grid = [[0usize; 2]; 2];
        for d in &self.declarations {
            let kind = match d.kind {
                ShaperKind::Physical => 0,
                ShaperKind::Material => 1,
            };
            for level in Level::ALL {
                if d.acts_at(level) {
                    grid[kind][level.index()] += 1;
                }
            }
        }
        grid
    }

    /// A set that covers all four cells of the taxonomy, so the grid can be shown to be reachable.
    pub fn full() -> Self {
        let mut set = ShaperSet::new();
        let rows = [
            ("carve", ShaperKind::Physical, [true, false], false),
            ("cut", ShaperKind::Physical, [true, true], true),
            ("seam", ShaperKind::Material, [true, false], false),
            ("ripple", ShaperKind::Material, [false, true], true),
        ];
        for (name, kind, levels, declares_addressed_field) in rows {
            set.admit(ShaperDeclaration { name, kind, levels, declares_addressed_field })
                .expect("the exemplar set is well formed");
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(i: u8, j: u8, k: u8) -> SubvoxelIndex {
        SubvoxelIndex::new(i, j, k).unwrap()
    }

    #[test]
    fn a_physical_shaper_changes_occupancy_and_a_material_shaper_cannot() {
        let mut lattice = StoredLattice::empty();
        assert_eq!(lattice.occupancy_count(), 0);

        // A physical shaper fills; occupancy changes.
        apply(ShaperKind::Physical, &mut lattice, cell(0, 0, 0), CellChange::Fill(MaterialId(4)))
            .unwrap();
        assert_eq!(lattice.occupancy_count(), 1);
        assert_eq!(lattice.get(cell(0, 0, 0)), Some(MaterialId(4)));

        // A physical shaper empties; occupancy changes back.
        apply(ShaperKind::Physical, &mut lattice, cell(0, 0, 0), CellChange::Empty).unwrap();
        assert_eq!(lattice.occupancy_count(), 0);

        // **A material shaper cannot fill an empty cell.**
        let err = apply(ShaperKind::Material, &mut lattice, cell(1, 1, 1), CellChange::Fill(MaterialId(4)))
            .unwrap_err();
        assert_eq!(
            err.kind(),
            crate::refusal::RefusalKind::MaterialShaperCannotChangeOccupancy
        );
        assert_eq!(lattice.occupancy_count(), 0, "the refusal changed nothing");

        // **Nor empty a filled one.**
        apply(ShaperKind::Physical, &mut lattice, cell(1, 1, 1), CellChange::Fill(MaterialId(2)))
            .unwrap();
        let err = apply(ShaperKind::Material, &mut lattice, cell(1, 1, 1), CellChange::Empty)
            .unwrap_err();
        assert_eq!(
            err.kind(),
            crate::refusal::RefusalKind::MaterialShaperCannotChangeOccupancy
        );
        assert_eq!(lattice.occupancy_count(), 1, "the cell still holds matter");

        // But it **may** recolour a filled cell, and occupancy is unchanged.
        apply(ShaperKind::Material, &mut lattice, cell(1, 1, 1), CellChange::Recolour(MaterialId(6)))
            .unwrap();
        assert_eq!(lattice.get(cell(1, 1, 1)), Some(MaterialId(6)));
        assert_eq!(lattice.occupancy_count(), 1);

        // And a recolour of an empty cell is refused by name.
        let err = apply(ShaperKind::Material, &mut lattice, cell(2, 2, 2), CellChange::Recolour(MaterialId(6)))
            .unwrap_err();
        assert_eq!(err.kind(), crate::refusal::RefusalKind::NoCellToRecolour);
    }

    #[test]
    fn a_shaper_that_claims_the_addressed_level_and_declares_nothing_is_refused_by_name() {
        let mut set = ShaperSet::new();
        let err = set
            .admit(ShaperDeclaration {
                name: "ripple",
                kind: ShaperKind::Material,
                levels: [false, true],
                declares_addressed_field: false,
            })
            .unwrap_err();
        assert_eq!(err.kind(), crate::refusal::RefusalKind::ShaperDeclaresNothing);
        assert!(err.detail().contains("addressed"));
        assert_eq!(set.len(), 0, "nothing is admitted on a refusal");
    }

    #[test]
    fn a_shaper_that_acts_at_no_level_is_refused_and_a_duplicate_name_is_refused() {
        let mut set = ShaperSet::new();
        let err = set
            .admit(ShaperDeclaration {
                name: "nothing",
                kind: ShaperKind::Physical,
                levels: [false, false],
                declares_addressed_field: false,
            })
            .unwrap_err();
        assert_eq!(err.kind(), crate::refusal::RefusalKind::ShaperDeclaresNothing);

        set.admit(ShaperDeclaration {
            name: "carve",
            kind: ShaperKind::Physical,
            levels: [true, false],
            declares_addressed_field: false,
        })
        .unwrap();
        let err = set
            .admit(ShaperDeclaration {
                name: "carve",
                kind: ShaperKind::Material,
                levels: [true, false],
                declares_addressed_field: false,
            })
            .unwrap_err();
        assert_eq!(err.kind(), crate::refusal::RefusalKind::DuplicateShaper);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn the_taxonomy_is_two_kinds_by_two_levels_and_every_cell_is_reachable() {
        let set = ShaperSet::full();
        assert_eq!(set.len(), 4);
        let grid = set.coverage();
        for (kind, row) in grid.iter().enumerate() {
            for (level, count) in row.iter().enumerate() {
                assert!(
                    *count >= 1,
                    "kind {kind} × level {level} is unexercised: {grid:?}"
                );
            }
        }
        // The stored level is the 4×4×4 lattice; a cell index addresses it.
        let mut lattice = StoredLattice::empty();
        apply(ShaperKind::Physical, &mut lattice, cell(3, 3, 3), CellChange::Fill(MaterialId(1)))
            .unwrap();
        assert_eq!(lattice.occupancy_count(), 1);
        assert_eq!(cell(3, 3, 3).flat(), 63);
    }
}
