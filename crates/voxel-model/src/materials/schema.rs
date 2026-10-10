//! The substance/process/tier schema and its gate — H-18.8 step 1.
//!
//! The campaign's claim is that **nothing is made from nothing**, and this is
//! where it becomes checkable rather than asserted: two data tables (substances,
//! processes), the tiers their work belongs to, the vocabulary of things that must
//! *stand* for work to be possible, and one gate that collects every defect at
//! once. The shape mirrors the reference (`tools/materials/SCHEMA.md`) — it is
//! read for the shape, never the authority: every number in the declaration is
//! this tree's own row with its own provenance.
//!
//! # What the schema makes true
//!
//! 1. Every substance has a **canonical unit**, and every quantity converts to
//!    grams **exactly** — by a declared rational, never by a float constant.
//! 2. Every process **balances**: what goes in equals what comes out, in grams.
//!    A gather is not an exception: its input side is the world, and the gate
//!    admits it only when every output is something the world already holds
//!    (`mined`, `gathered`, `salvaged`).
//! 3. Every process has a **mechanism with a number** — heat, time, labour or
//!    power, at least one of them stated. A process with no mechanism is a
//!    defect, not an abstraction.
//! 4. Every substance either has a **consumer** or its row says **why** it is
//!    an end product. The reason is printed as open, never silently swallowed.
//! 5. **The content is unbounded; the schema is not.** An age is rows.
//!
//! # Provenance and absence
//!
//! Every row carries an [`Availability`]: `[V]` read from a source, `[D]`
//! declared by this tree (a game rate, named as one), `[NS]` not yet sourced.
//! An `[NS]` row may not carry a number: an empty column is not a zero, and a
//! value with no source is exactly the fabrication the discipline refuses.
//! `porosity` and `fluid_capacity` (H-24.1) follow the same rule — a row has
//! them or it does not, and `fluid_capacity > porosity` is refused by name.

use crate::material::Ratio;
use crate::matter::rational::Rational;
use crate::refusal::{Refusal, RefusalKind, RefusalSet, Refused};
use std::collections::{BTreeMap, BTreeSet};

/// The ten appearance families (H-18.2's read view; the row itself is
/// substance-first). Closed because the appearance layer resolves families, and
/// an eleventh family would be a decision about how the world reads, not a row.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Family {
    Metal,
    Enamel,
    Glass,
    Ceramic,
    Polymer,
    Paper,
    Road,
    Water,
    Organic,
    Soil,
}

impl Family {
    pub const ALL: [Family; 10] = [
        Family::Metal,
        Family::Enamel,
        Family::Glass,
        Family::Ceramic,
        Family::Polymer,
        Family::Paper,
        Family::Road,
        Family::Water,
        Family::Organic,
        Family::Soil,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Family::Metal => "metal",
            Family::Enamel => "enamel",
            Family::Glass => "glass",
            Family::Ceramic => "ceramic",
            Family::Polymer => "polymer",
            Family::Paper => "paper",
            Family::Road => "road",
            Family::Water => "water",
            Family::Organic => "organic",
            Family::Soil => "soil",
        }
    }

    pub fn parse(text: &str) -> Refused<Self> {
        Self::ALL
            .into_iter()
            .find(|family| family.name() == text)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::UndeclaredName,
                    format!("{text:?} is not one of the ten families"),
                )
            })
    }
}

/// The hue anchors: seven sampled/declared slots plus `natural`.
///
/// `natural` is the anchor that says "this row presents with its own colour, not
/// a declared hue slot" — it is not a colour and it is not a gap.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Hue {
    Red,
    Amber,
    Yellow,
    Green,
    Cyan,
    Blue,
    Violet,
    Natural,
}

impl Hue {
    pub const ALL: [Hue; 8] = [
        Hue::Red,
        Hue::Amber,
        Hue::Yellow,
        Hue::Green,
        Hue::Cyan,
        Hue::Blue,
        Hue::Violet,
        Hue::Natural,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Hue::Red => "red",
            Hue::Amber => "amber",
            Hue::Yellow => "yellow",
            Hue::Green => "green",
            Hue::Cyan => "cyan",
            Hue::Blue => "blue",
            Hue::Violet => "violet",
            Hue::Natural => "natural",
        }
    }

    pub fn parse(text: &str) -> Refused<Self> {
        Self::ALL
            .into_iter()
            .find(|hue| hue.name() == text)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::UndeclaredName,
                    format!("{text:?} is not one of the eight hue anchors"),
                )
            })
    }
}

/// The only level vocabulary (H-18.3): body, edge, deep.
///
/// The lightness deltas are **exact signed rationals**, not floats, and they are
/// *deltas* — `deep` is negative, which is why this reads
/// [`Rational`](crate::matter::rational::Rational) rather than the unsigned
/// [`Ratio`] the quantities use.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Level {
    Body,
    Edge,
    Deep,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Body, Level::Edge, Level::Deep];

    pub const fn name(self) -> &'static str {
        match self {
            Level::Body => "body",
            Level::Edge => "edge",
            Level::Deep => "deep",
        }
    }

    pub fn parse(text: &str) -> Refused<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.name() == text)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::UndeclaredName,
                    format!("{text:?} is not one of body/edge/deep"),
                )
            })
    }

    /// The lightness delta the level reads with: `0`, `+1/10`, `-9/50`.
    pub fn lightness_delta(self) -> Refused<Rational> {
        match self {
            Level::Body => Ok(Rational::ZERO),
            Level::Edge => Rational::new(1, 10),
            Level::Deep => Rational::new(-9, 50),
        }
    }
}

/// A substance's canonical unit — what the ledger counts in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Unit {
    /// grams
    Mass,
    /// millilitres, converted with a declared density
    Volume,
    /// units, converted with a declared unit mass
    Count,
    /// millilitres at STP, converted with a declared density
    Gas,
}

impl Unit {
    pub const ALL: [Unit; 4] = [Unit::Mass, Unit::Volume, Unit::Count, Unit::Gas];

    pub const fn name(self) -> &'static str {
        match self {
            Unit::Mass => "Mass",
            Unit::Volume => "Volume",
            Unit::Count => "Count",
            Unit::Gas => "Gas",
        }
    }

    pub fn parse(text: &str) -> Refused<Self> {
        Self::ALL
            .into_iter()
            .find(|unit| unit.name() == text)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::UndeclaredName,
                    format!("{text:?} is not one of Mass/Volume/Count/Gas"),
                )
            })
    }
}

/// `[V]` read, `[D]` declared, `[NS]` not yet sourced (H-18.7, verbatim).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Availability {
    /// Read from a named source.
    Read,
    /// A game rate, declared as such — a value no source states.
    Declared,
    /// Not yet sourced. **A row in this state may not carry a number.**
    NotYetSourced,
}

impl Availability {
    pub const ALL: [Availability; 3] = [
        Availability::Read,
        Availability::Declared,
        Availability::NotYetSourced,
    ];

    pub const fn marker(self) -> &'static str {
        match self {
            Availability::Read => "V",
            Availability::Declared => "D",
            Availability::NotYetSourced => "NS",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Availability::Read => "read",
            Availability::Declared => "declared",
            Availability::NotYetSourced => "not_yet_sourced",
        }
    }

    pub fn parse(text: &str) -> Refused<Self> {
        Self::ALL
            .into_iter()
            .find(|state| state.marker() == text)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::UndeclaredName,
                    format!("{text:?} is not one of V/D/NS"),
                )
            })
    }
}

/// The closed tag set.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Tag {
    Fuel,
    Edible,
    Tool,
    Structure,
    Precious,
    Salvage,
}

impl Tag {
    pub const ALL: [Tag; 6] = [
        Tag::Fuel,
        Tag::Edible,
        Tag::Tool,
        Tag::Structure,
        Tag::Precious,
        Tag::Salvage,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Tag::Fuel => "fuel",
            Tag::Edible => "edible",
            Tag::Tool => "tool",
            Tag::Structure => "structure",
            Tag::Precious => "precious",
            Tag::Salvage => "salvage",
        }
    }

    pub fn parse(text: &str) -> Refused<Self> {
        Self::ALL
            .into_iter()
            .find(|tag| tag.name() == text)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::UndeclaredName,
                    format!("{text:?} is not one of the closed tag set"),
                )
            })
    }
}

/// How a substance enters the world.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Source {
    Mined,
    Grown,
    Gathered,
    Salvaged,
    Made,
}

impl Source {
    pub const ALL: [Source; 5] = [
        Source::Mined,
        Source::Grown,
        Source::Gathered,
        Source::Salvaged,
        Source::Made,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Source::Mined => "mined",
            Source::Grown => "grown",
            Source::Gathered => "gathered",
            Source::Salvaged => "salvaged",
            Source::Made => "made",
        }
    }

    /// Whether a process with **no inputs** may hand this out: the world already
    /// holds it.
    pub const fn is_held_by_world(self) -> bool {
        matches!(self, Source::Mined | Source::Grown | Source::Gathered | Source::Salvaged)
    }

    pub fn parse(text: &str) -> Refused<Self> {
        Self::ALL
            .into_iter()
            .find(|source| source.name() == text)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::UndeclaredName,
                    format!("{text:?} is not one of mined/grown/gathered/salvaged/made"),
                )
            })
    }
}

/// Where a substance goes when a process gives it up.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Sink {
    Holding,
    Atmosphere,
}

impl Sink {
    pub const fn name(self) -> &'static str {
        match self {
            Sink::Holding => "holding",
            Sink::Atmosphere => "atmosphere",
        }
    }

    pub fn parse(text: &str) -> Refused<Self> {
        match text {
            "holding" => Ok(Sink::Holding),
            "atmosphere" => Ok(Sink::Atmosphere),
            _ => Err(Refusal::owned(
                RefusalKind::UndeclaredName,
                format!("{text:?} is not holding or atmosphere"),
            )),
        }
    }
}

/// A declared exact rational: `n` or `n/d`. **A float is refused by name.**
pub fn parse_ratio(text: &str) -> Refused<Ratio> {
    let floats = |text: &str| {
        Refusal::owned(
            RefusalKind::NonRationalQuantity,
            format!("{text:?} is not an exact rational; declare it as n or n/d"),
        )
    };
    if let Some((num, den)) = text.split_once('/') {
        let num = num.parse::<u64>().map_err(|_| floats(text))?;
        let den = den.parse::<u64>().map_err(|_| floats(text))?;
        Ratio::new(num, den)
    } else {
        let num = text.parse::<u64>().map_err(|_| floats(text))?;
        Ok(Ratio::from_u64(num))
    }
}

/// A substance row. Substance-first: the row is the identity, and
/// `family × hue × level` is how it is *read* for appearance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Substance {
    pub name: String,
    pub family: Family,
    pub hue: Hue,
    pub unit: Unit,
    /// g/mL. Required for `Volume` and `Gas` — the units that convert by it.
    /// A `Mass` or `Count` may still state it: a density is a physical property
    /// (geology reads it), not a conversion that only some units need. What is
    /// refused is a **unit mass** anywhere but a `Count`, because that is a
    /// conversion for a unit that does not use it.
    pub density_g_ml: Option<Ratio>,
    /// grams per unit. Required for `Count`, refused for everything else.
    pub unit_mass_g: Option<Ratio>,
    /// Grams lost per gram held per sim-day. `0` is a declared value (nothing
    /// rots yet); it is never an omission.
    pub rot_per_day: Ratio,
    pub tags: Vec<Tag>,
    pub source: Source,
    pub sink: Sink,
    /// The natural unit a person reads it in, with exact grams per display unit
    /// when the unit converts (a count's display factor is the unit mass, so it
    /// is `None` here and never a second conversion).
    pub display: (String, Option<Ratio>),
    /// Fraction of the bulk that is void. `[NS]` until sourced.
    pub porosity: Option<Ratio>,
    /// Fraction of the bulk that can hold fluid. Never greater than `porosity`.
    pub fluid_capacity: Option<Ratio>,
    pub provenance: Availability,
    pub note: String,
    /// The reason a substance with no consumer still exists. Printed as open.
    pub no_consumer: Option<String>,
}

impl Substance {
    /// Convert an amount in this substance's canonical unit to grams, exactly.
    pub fn to_grams(&self, amount: Ratio) -> Refused<Ratio> {
        match self.unit {
            Unit::Mass => Ok(amount),
            Unit::Volume | Unit::Gas => self
                .density_g_ml
                .map(|density| amount * density)
                .ok_or_else(|| {
                    Refusal::owned(
                        RefusalKind::FieldDataMissing,
                        format!(
                            "{} is a {} and declares no density, so it cannot become grams",
                            self.name,
                            self.unit.name()
                        ),
                    )
                }),
            Unit::Count => self
                .unit_mass_g
                .map(|unit_mass| amount * unit_mass)
                .ok_or_else(|| {
                    Refusal::owned(
                        RefusalKind::FieldDataMissing,
                        format!(
                            "{} is a Count and declares no unit mass, so one {} is not a mass",
                            self.name, self.name
                        ),
                    )
                }),
        }
    }
}

/// One quantity of one substance, in that substance's canonical unit.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Qty {
    pub substance: String,
    pub amount: Ratio,
}

impl Qty {
    pub fn new(substance: impl Into<String>, amount: Ratio) -> Self {
        Qty { substance: substance.into(), amount }
    }
}

/// Which body the heat is measured in — the ambiguity `SOURCES.md` found is
/// worth about 600 °C, so a process declaring heat must say which it means.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeatKind {
    Material,
    Flame,
}

impl HeatKind {
    pub const fn name(self) -> &'static str {
        match self {
            HeatKind::Material => "material",
            HeatKind::Flame => "flame",
        }
    }

    pub fn parse(text: &str) -> Refused<Self> {
        match text {
            "material" => Ok(HeatKind::Material),
            "flame" => Ok(HeatKind::Flame),
            _ => Err(Refusal::owned(
                RefusalKind::UndeclaredName,
                format!("{text:?} is not material or flame"),
            )),
        }
    }
}

/// Q37's stopping rule: a process declares heat, time, labour or power, with a
/// number.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Mechanism {
    pub heat_c: Option<Ratio>,
    pub heat_kind: Option<HeatKind>,
    pub hours: Ratio,
    pub labour_hours: Ratio,
    pub power_kw: Ratio,
}

impl Mechanism {
    /// Whether anything about this mechanism is stated at all.
    pub fn is_stated(&self) -> bool {
        self.heat_c.is_some()
            || !self.hours.is_zero()
            || !self.labour_hours.is_zero()
            || !self.power_kw.is_zero()
    }
}

/// What must exist for the work to be possible. Every name is checked against
/// the declared vocabulary; a name nothing declares is refused, and a
/// vocabulary entry nothing requires is refused too.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Requires {
    pub structure: Option<String>,
    pub tool: Option<String>,
    pub skill: Option<String>,
}

impl Requires {
    pub fn is_empty(&self) -> bool {
        self.structure.is_none() && self.tool.is_none() && self.skill.is_none()
    }
}

/// A process row: inputs and outputs that balance in grams, a mechanism with a
/// number, and the tier the work belongs to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Process {
    pub name: String,
    pub tier: String,
    pub inputs: Vec<Qty>,
    pub outputs: Vec<Qty>,
    pub mechanism: Mechanism,
    pub requires: Requires,
    /// Declared losses are **outputs** and therefore balance.
    pub provenance: Availability,
    pub note: String,
}

impl Process {
    /// A gather: no inputs, and its input side is the world.
    pub fn is_gather(&self) -> bool {
        self.inputs.is_empty()
    }
}

/// An age, as a row. `ordinal` is what a progression compares against.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tier {
    pub name: String,
    pub ordinal: u32,
    pub note: String,
}

/// The kinds of thing that must stand for work to be possible.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum VocabKind {
    Structure,
    Tool,
    Skill,
}

impl VocabKind {
    pub const ALL: [VocabKind; 3] = [VocabKind::Structure, VocabKind::Tool, VocabKind::Skill];

    pub const fn name(self) -> &'static str {
        match self {
            VocabKind::Structure => "structure",
            VocabKind::Tool => "tool",
            VocabKind::Skill => "skill",
        }
    }

    pub fn parse(text: &str) -> Refused<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == text)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::UndeclaredName,
                    format!("{text:?} is not structure/tool/skill"),
                )
            })
    }
}

/// One entry of the vocabulary: a thing that must stand, and the note saying
/// what it is. An entry no process requires is refused — this is a vocabulary,
/// not a wish list.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Vocabulary {
    pub kind: VocabKind,
    pub name: String,
    pub note: String,
}

/// The two tables and the tiers, admitted together.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Table {
    pub tiers: Vec<Tier>,
    pub substances: BTreeMap<String, Substance>,
    pub processes: Vec<Process>,
    pub vocabulary: Vec<Vocabulary>,
}

impl Table {
    pub fn new(
        tiers: Vec<Tier>,
        substances: Vec<Substance>,
        processes: Vec<Process>,
        vocabulary: Vec<Vocabulary>,
    ) -> Self {
        Table {
            tiers,
            substances: substances
                .into_iter()
                .map(|substance| (substance.name.clone(), substance))
                .collect(),
            processes,
            vocabulary,
        }
    }

    pub fn substance(&self, name: &str) -> Refused<&Substance> {
        self.substances.get(name).ok_or_else(|| {
            Refusal::owned(
                RefusalKind::UndeclaredName,
                format!("{name:?} is not a declared substance"),
            )
        })
    }

    /// The gram total of one side of a process, exactly.
    pub fn grams_total(&self, side: &[Qty]) -> Refused<Ratio> {
        let mut total = Ratio::ZERO;
        for qty in side {
            total += self.substance(&qty.substance)?.to_grams(qty.amount)?;
        }
        Ok(total)
    }

    /// The reasons substance rows give for having no consumer — *open work*,
    /// printed rather than suppressed, and not defects.
    pub fn open_reasons(&self) -> Vec<String> {
        let mut reasons: Vec<String> = self
            .substances
            .values()
            .filter(|substance| {
                !self.is_consumed(&substance.name) && substance.no_consumer.is_some()
            })
            .filter_map(|substance| {
                substance
                    .no_consumer
                    .as_ref()
                    .map(|reason| format!("{}: {reason}", substance.name))
            })
            .collect();
        reasons.sort();
        reasons
    }

    /// Whether any process consumes this substance.
    pub fn is_consumed(&self, name: &str) -> bool {
        self.processes
            .iter()
            .any(|process| process.inputs.iter().any(|qty| qty.substance == name))
    }

    /// Every defect, collected — never the first only.
    pub fn gate(&self) -> RefusalSet {
        let mut defects = RefusalSet::new();
        self.gate_tiers(&mut defects);
        self.gate_substances(&mut defects);
        self.gate_processes(&mut defects);
        self.gate_vocabulary(&mut defects);
        defects
    }

    fn gate_tiers(&self, defects: &mut RefusalSet) {
        let mut names = BTreeSet::new();
        let mut ordinals = BTreeSet::new();
        for tier in &self.tiers {
            if tier.name.is_empty() {
                defects.refuse(RefusalKind::UndeclaredName, "a tier with an empty name");
            }
            if !names.insert(tier.name.clone()) {
                defects.refuse(
                    RefusalKind::UndeclaredName,
                    format!("tier {:?} is declared twice", tier.name),
                );
            }
            if !ordinals.insert(tier.ordinal) {
                defects.refuse(
                    RefusalKind::OutOfDeclaredRange,
                    format!("two tiers share ordinal {}", tier.ordinal),
                );
            }
        }
    }

    fn gate_substances(&self, defects: &mut RefusalSet) {
        for substance in self.substances.values() {
            let name = &substance.name;

            // Unit rules: every unit converts to grams exactly, or the row is
            // not a row. A **density is a property**, so it may stand on any
            // row; what is checked is whether the unit *needs* it to convert.
            // (`Volume` and `Gas` do; a `Mass` converts by definition and a
            // `Count` by its unit mass — which is why a unit mass on a
            // non-Count is the contradiction.)
            if matches!(substance.unit, Unit::Volume | Unit::Gas) && substance.density_g_ml.is_none() {
                defects.refuse(
                    RefusalKind::FieldDataMissing,
                    format!("{name}: a {} declares no density", substance.unit.name()),
                );
            }
            match substance.unit {
                Unit::Count => {
                    if substance.unit_mass_g.is_none() {
                        defects.refuse(
                            RefusalKind::FieldDataMissing,
                            format!("{name}: a Count declares no unit mass"),
                        );
                    }
                }
                _ => {
                    if substance.unit_mass_g.is_some() {
                        defects.refuse(
                            RefusalKind::OutOfDeclaredRange,
                            format!("{name}: unit mass is for Counts, not {}", substance.unit.name()),
                        );
                    }
                }
            }
            if let Some(density) = substance.density_g_ml {
                if density.is_zero() {
                    defects.refuse(
                        RefusalKind::ZeroRatio,
                        format!("{name}: a density of zero is not a density"),
                    );
                }
            }
            if let Some(unit_mass) = substance.unit_mass_g {
                if unit_mass.is_zero() {
                    defects.refuse(
                        RefusalKind::ZeroRatio,
                        format!("{name}: a unit mass of zero is not a mass"),
                    );
                }
            }

            // A gas nobody can hold: a gas that declares `holding` is refused.
            if substance.unit == Unit::Gas && substance.sink == Sink::Holding {
                defects.refuse(
                    RefusalKind::OutOfDeclaredRange,
                    format!("{name}: a Gas must declare its sink, and `holding` is not a gas's sink"),
                );
            }
            if substance.unit != Unit::Gas && substance.sink == Sink::Atmosphere {
                defects.refuse(
                    RefusalKind::OutOfDeclaredRange,
                    format!("{name}: only a Gas may sink to the atmosphere"),
                );
            }

            // Porosity and capacity (H-24.1): both or neither, and a void cannot
            // hold more than it is.
            for (field, value) in [
                ("porosity", substance.porosity),
                ("fluid_capacity", substance.fluid_capacity),
            ] {
                if let Some(value) = value {
                    if value.is_zero() || value.is_greater_than(Ratio::ONE) {
                        defects.refuse(
                            RefusalKind::OutOfDeclaredRange,
                            format!("{name}: {field} = {value} is not a fraction of the bulk"),
                        );
                    }
                }
            }
            if let (Some(capacity), Some(porosity)) =
                (substance.fluid_capacity, substance.porosity)
            {
                if capacity.is_greater_than(porosity) {
                    defects.refuse(
                        RefusalKind::FluidCapacityExceedsPorosity,
                        format!(
                            "{name}: fluid_capacity {capacity} exceeds porosity {porosity}"
                        ),
                    );
                }
            }

            // `[NS]` is an empty column, never a number.
            if substance.provenance == Availability::NotYetSourced {
                let carries = substance.density_g_ml.is_some()
                    || substance.unit_mass_g.is_some()
                    || !substance.rot_per_day.is_zero()
                    || substance.porosity.is_some()
                    || substance.fluid_capacity.is_some();
                if carries {
                    defects.refuse(
                        RefusalKind::UnsubstantiatedEvidence,
                        format!(
                            "{name}: marked [NS] but carries a value; an empty column is not a zero"
                        ),
                    );
                }
            }

            // The consumer rule: something must use it, or the row must say why
            // it is an end product. An unfinished row is admitted only when it
            // says so.
            if !self.is_consumed(name) && substance.no_consumer.is_none() {
                defects.refuse(
                    RefusalKind::OrphanSubstance,
                    format!("{name}: no process consumes it and the row states no reason"),
                );
            }
        }
    }

    fn gate_processes(&self, defects: &mut RefusalSet) {
        let tier_names: BTreeSet<&str> = self.tiers.iter().map(|tier| tier.name.as_str()).collect();
        let mut names = BTreeSet::new();
        for process in &self.processes {
            let name = &process.name;
            if !names.insert(name.clone()) {
                defects.refuse(
                    RefusalKind::UndeclaredName,
                    format!("process {name:?} is declared twice"),
                );
            }
            if !tier_names.contains(process.tier.as_str()) {
                defects.refuse(
                    RefusalKind::UndeclaredName,
                    format!("{name}: tier {:?} is not declared, so the work cannot be reached", process.tier),
                );
            }

            // Every name it touches must be declared.
            for qty in process.inputs.iter().chain(process.outputs.iter()) {
                if !self.substances.contains_key(&qty.substance) {
                    defects.refuse(
                        RefusalKind::UndeclaredName,
                        format!("{name}: {:?} is not a declared substance", qty.substance),
                    );
                }
                if qty.amount.is_zero() {
                    defects.refuse(
                        RefusalKind::ZeroRatio,
                        format!("{name}: a zero quantity of {} is a no-op pretending to be a part", qty.substance),
                    );
                }
            }

            // Q37's stopping rule.
            if !process.mechanism.is_stated() {
                defects.refuse(
                    RefusalKind::MissingMechanism,
                    format!("{name}: no mechanism with a number — heat, time, labour or power"),
                );
            }
            if process.mechanism.heat_c.is_some() && process.mechanism.heat_kind.is_none() {
                defects.refuse(
                    RefusalKind::FieldDataMissing,
                    format!("{name}: heat with no kind — material or flame, which differ by ~600 °C"),
                );
            }

            // What must stand.
            for (kind, declared) in [
                (VocabKind::Structure, &process.requires.structure),
                (VocabKind::Tool, &process.requires.tool),
                (VocabKind::Skill, &process.requires.skill),
            ] {
                if let Some(required) = declared {
                    let known = self
                        .vocabulary
                        .iter()
                        .any(|entry| entry.kind == kind && &entry.name == required);
                    if !known {
                        defects.refuse(
                            RefusalKind::UndeclaredName,
                            format!("{name}: requires {} {required:?}, which nothing declares", kind.name()),
                        );
                    }
                }
            }

            // Balance. A gather is not an exception: its input side is the
            // world, and it may only hand out what the world already holds.
            if process.is_gather() {
                for qty in &process.outputs {
                    match self.substance(&qty.substance) {
                        Ok(substance) if substance.source.is_held_by_world() => {}
                        Ok(substance) => {
                            defects.refuse(
                            RefusalKind::UnbalancedProcess,
                            format!(
                                "{name}: makes {} from nothing, and {} is `{}` — a gather may only \
                                 hand out what the world holds",
                                qty.substance,
                                qty.substance,
                                substance.source.name()
                            ),
                            );
                        }
                        Err(_) => {} // already refused as an undeclared name
                    }
                }
            } else {
                let input = self.grams_total(&process.inputs);
                let output = self.grams_total(&process.outputs);
                match (input, output) {
                    (Ok(input), Ok(output)) => {
                        if input != output {
                            defects.refuse(
                                RefusalKind::UnbalancedProcess,
                                format!("{name}: in {input} g vs out {output} g"),
                            );
                        }
                    }
                    (Err(refusal), _) | (_, Err(refusal)) => {
                        defects.refuse(refusal.kind(), refusal.detail().to_string());
                    }
                }
            }
        }
    }

    fn gate_vocabulary(&self, defects: &mut RefusalSet) {
        let mut names = BTreeSet::new();
        for entry in &self.vocabulary {
            let key = (entry.kind, entry.name.clone());
            if !names.insert(key) {
                defects.refuse(
                    RefusalKind::UndeclaredName,
                    format!("vocabulary {}/{} is declared twice", entry.kind.name(), entry.name),
                );
            }
            let required = self.processes.iter().any(|process| {
                let declared = match entry.kind {
                    VocabKind::Structure => &process.requires.structure,
                    VocabKind::Tool => &process.requires.tool,
                    VocabKind::Skill => &process.requires.skill,
                };
                declared.as_deref() == Some(entry.name.as_str())
            });
            if !required {
                defects.refuse(
                    RefusalKind::OrphanSubstance,
                    format!(
                        "vocabulary {}:{} is required by no process, and this is a vocabulary, not a wish list",
                        entry.kind.name(),
                        entry.name
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ratio(num: u64, den: u64) -> Ratio {
        Ratio::new(num, den).expect("a rational in a test")
    }

    fn mass(name: &str, source: Source) -> Substance {
        Substance {
            name: name.into(),
            family: Family::Ceramic,
            hue: Hue::Natural,
            unit: Unit::Mass,
            density_g_ml: None,
            unit_mass_g: None,
            rot_per_day: Ratio::ZERO,
            tags: vec![],
            source,
            sink: Sink::Holding,
            display: ("kg".into(), Some(Ratio::from_u64(1000))),
            porosity: None,
            fluid_capacity: None,
            provenance: Availability::Declared,
            note: String::new(),
            no_consumer: None,
        }
    }

    fn mechanism() -> Mechanism {
        Mechanism {
            heat_c: None,
            heat_kind: None,
            hours: ratio(1, 2),
            labour_hours: ratio(1, 2),
            power_kw: Ratio::ZERO,
        }
    }

    fn process(name: &str, inputs: Vec<Qty>, outputs: Vec<Qty>) -> Process {
        Process {
            name: name.into(),
            tier: "the tier".into(),
            inputs,
            outputs,
            mechanism: mechanism(),
            requires: Requires::default(),
            provenance: Availability::Declared,
            note: String::new(),
        }
    }

    fn tiers() -> Vec<Tier> {
        vec![Tier { name: "the tier".into(), ordinal: 0, note: String::new() }]
    }

    /// The reference's own worked chain, at the row level: gather → riven haft →
    /// knap a core → twist cord → assemble the hatchet, balancing at every step
    /// (3295 g in, 1 hatchet @ 2900 g + 395 g waste out).
    fn worked_table() -> Table {
        let mut timber = mass("timber", Source::Gathered);
        timber.tags = vec![Tag::Fuel, Tag::Structure];
        let haft = mass("haft_blank", Source::Made);
        let mut offcuts = mass("timber_offcuts", Source::Made);
        offcuts.no_consumer = Some("an end product".into());
        let stone = mass("stone", Source::Mined);
        let edge = mass("knapped_edge", Source::Made);
        let mut flakes = mass("stone_flakes", Source::Made);
        flakes.no_consumer = Some("an end product".into());
        let fibre = mass("plant_fibre", Source::Gathered);
        let cord = mass("cord", Source::Made);
        let mut dust = mass("fibre_dust", Source::Made);
        dust.no_consumer = Some("an end product".into());
        let mut hatchet = mass("hatchet", Source::Made);
        hatchet.unit = Unit::Count;
        hatchet.unit_mass_g = Some(Ratio::from_u64(2900));
        hatchet.display = ("hatchet".into(), None);
        hatchet.no_consumer = Some("held and used, not consumed".into());
        let mut trim = mass("trim_waste", Source::Made);
        trim.no_consumer = Some("an end product".into());
        Table::new(
            tiers(),
            vec![
                timber, haft, offcuts, stone, edge, flakes, fibre, cord, dust, hatchet, trim,
            ],
            vec![
                process(
                    "gather timber",
                    vec![],
                    vec![Qty::new("timber", Ratio::from_u64(3000))],
                ),
                process(
                    "riven haft",
                    vec![Qty::new("timber", Ratio::from_u64(3000))],
                    vec![
                        Qty::new("haft_blank", Ratio::from_u64(2400)),
                        Qty::new("timber_offcuts", Ratio::from_u64(600)),
                    ],
                ),
                process(
                    "gather stone",
                    vec![],
                    vec![Qty::new("stone", Ratio::from_u64(1000))],
                ),
                process(
                    "knap a core",
                    vec![Qty::new("stone", Ratio::from_u64(1000))],
                    vec![
                        Qty::new("knapped_edge", Ratio::from_u64(800)),
                        Qty::new("stone_flakes", Ratio::from_u64(200)),
                    ],
                ),
                process(
                    "gather plant fibre",
                    vec![],
                    vec![Qty::new("plant_fibre", Ratio::from_u64(100))],
                ),
                process(
                    "twist cord",
                    vec![Qty::new("plant_fibre", Ratio::from_u64(100))],
                    vec![
                        Qty::new("cord", Ratio::from_u64(95)),
                        Qty::new("fibre_dust", Ratio::from_u64(5)),
                    ],
                ),
                process(
                    "assemble the hatchet",
                    vec![
                        Qty::new("knapped_edge", Ratio::from_u64(800)),
                        Qty::new("haft_blank", Ratio::from_u64(2400)),
                        Qty::new("cord", Ratio::from_u64(95)),
                    ],
                    vec![
                        Qty::new("hatchet", Ratio::ONE),
                        Qty::new("trim_waste", Ratio::from_u64(395)),
                    ],
                ),
            ],
            vec![],
        )
    }

    #[test]
    fn the_worked_chain_balances_and_gates_clean() {
        let defects = worked_table().gate();
        assert!(defects.is_empty(), "unexpected defects: {defects}");
    }

    #[test]
    fn a_gather_may_only_hand_out_what_the_world_holds() {
        // `made` output from no inputs is the thing the gate exists to refuse.
        let mut table = worked_table();
        table.processes[0].outputs[0] = Qty::new("haft_blank", Ratio::from_u64(3000));
        let defects = table.gate();
        assert!(
            defects.all().iter().any(|r| r.kind() == RefusalKind::UnbalancedProcess
                && r.detail().contains("from nothing")),
            "unexpected defects: {defects}"
        );
    }

    #[test]
    fn an_unbalanced_process_quotes_both_totals() {
        let mut table = worked_table();
        table.processes[1].outputs[1] = Qty::new("timber_offcuts", Ratio::from_u64(595));
        let defects = table.gate();
        let refusal = defects
            .all()
            .iter()
            .find(|r| r.kind() == RefusalKind::UnbalancedProcess)
            .expect("the balance must be refused");
        assert!(
            refusal.detail().contains("3000") && refusal.detail().contains("2995"),
            "a balance refusal quotes both totals: {}",
            refusal.detail()
        );
    }

    #[test]
    fn a_volume_without_a_density_is_refused() {
        let mut water = mass("water", Source::Gathered);
        water.unit = Unit::Volume;
        water.no_consumer = Some("the world draws it".into());
        let table = Table::new(tiers(), vec![water], vec![], vec![]);
        let defects = table.gate();
        assert!(
            defects.all().iter().any(|r| r.kind() == RefusalKind::FieldDataMissing
                && r.detail().contains("declares no density")),
            "unexpected defects: {defects}"
        );
    }

    #[test]
    fn a_count_without_a_unit_mass_is_refused() {
        let mut hatchet = mass("hatchet", Source::Made);
        hatchet.unit = Unit::Count;
        hatchet.display = ("hatchet".into(), None);
        hatchet.no_consumer = Some("held, not consumed".into());
        let table = Table::new(tiers(), vec![hatchet], vec![], vec![]);
        let defects = table.gate();
        assert!(
            defects.all().iter().any(|r| r.kind() == RefusalKind::FieldDataMissing
                && r.detail().contains("unit mass")),
            "unexpected defects: {defects}"
        );
    }

    #[test]
    fn a_gas_that_declares_holding_is_refused() {
        let mut gas = mass("flue_gas", Source::Made);
        gas.unit = Unit::Gas;
        gas.density_g_ml = Some(ratio(13, 10_000));
        gas.sink = Sink::Holding;
        gas.no_consumer = Some("up the chimney".into());
        let table = Table::new(tiers(), vec![gas], vec![], vec![]);
        let defects = table.gate();
        assert!(
            defects.all().iter().any(|r| r.kind() == RefusalKind::OutOfDeclaredRange
                && r.detail().contains("must declare its sink")),
            "unexpected defects: {defects}"
        );
    }

    #[test]
    fn a_substance_with_no_consumer_and_no_reason_is_refused() {
        let orphan = mass("orphan", Source::Made);
        let table = Table::new(tiers(), vec![orphan], vec![], vec![]);
        let defects = table.gate();
        assert_eq!(defects.len(), 1);
        assert_eq!(defects.all()[0].kind(), RefusalKind::OrphanSubstance);
    }

    #[test]
    fn a_reason_is_open_work_and_not_a_defect() {
        let mut orphan = mass("sand", Source::Mined);
        orphan.no_consumer = Some("glass waits for the kiln".into());
        let table = Table::new(tiers(), vec![orphan], vec![], vec![]);
        assert!(table.gate().is_empty());
        assert_eq!(table.open_reasons(), vec!["sand: glass waits for the kiln"]);
    }

    #[test]
    fn a_process_with_no_mechanism_is_refused() {
        let mut table = worked_table();
        table.processes[1].mechanism = Mechanism {
            heat_c: None,
            heat_kind: None,
            hours: Ratio::ZERO,
            labour_hours: Ratio::ZERO,
            power_kw: Ratio::ZERO,
        };
        let defects = table.gate();
        assert!(
            defects.all().iter().any(|r| r.kind() == RefusalKind::MissingMechanism),
            "unexpected defects: {defects}"
        );
    }

    #[test]
    fn heat_without_a_kind_is_refused() {
        let mut table = worked_table();
        table.processes[1].mechanism.heat_c = Some(Ratio::from_u64(950));
        let defects = table.gate();
        assert!(
            defects.all().iter().any(|r| r.kind() == RefusalKind::FieldDataMissing
                && r.detail().contains("heat with no kind")),
            "unexpected defects: {defects}"
        );
    }

    #[test]
    fn fluid_capacity_may_not_exceed_porosity() {
        let mut soil = mass("soil", Source::Mined);
        soil.no_consumer = Some("ground".into());
        soil.porosity = Some(ratio(15, 100));
        soil.fluid_capacity = Some(ratio(30, 100));
        let table = Table::new(tiers(), vec![soil], vec![], vec![]);
        let defects = table.gate();
        assert!(
            defects.all().iter().any(|r| r.kind() == RefusalKind::FluidCapacityExceedsPorosity),
            "unexpected defects: {defects}"
        );
    }

    #[test]
    fn a_not_yet_sourced_row_may_not_carry_a_number() {
        let mut ore = mass("iron_ore", Source::Mined);
        ore.provenance = Availability::NotYetSourced;
        ore.no_consumer = Some("the metal ages".into());
        let table = Table::new(tiers(), vec![ore], vec![], vec![]);
        assert!(table.gate().is_empty(), "identity alone is not a number");

        let mut with_density = mass("iron_ore", Source::Mined);
        with_density.provenance = Availability::NotYetSourced;
        with_density.density_g_ml = Some(ratio(27, 10));
        with_density.no_consumer = Some("the metal ages".into());
        let table = Table::new(tiers(), vec![with_density], vec![], vec![]);
        let defects = table.gate();
        assert!(
            defects.all().iter().any(|r| r.kind() == RefusalKind::UnsubstantiatedEvidence),
            "unexpected defects: {defects}"
        );
    }

    #[test]
    fn a_vocabulary_entry_nothing_requires_is_refused() {
        let table = Table::new(
            tiers(),
            worked_table().substances.into_values().collect(),
            worked_table().processes,
            vec![Vocabulary {
                kind: VocabKind::Structure,
                name: "kiln".into(),
                note: "a fire under clay".into(),
            }],
        );
        let defects = table.gate();
        assert!(
            defects.all().iter().any(|r| r.kind() == RefusalKind::OrphanSubstance
                && r.detail().contains("wish list")),
            "unexpected defects: {defects}"
        );
    }

    #[test]
    fn an_undeclared_name_is_refused_wherever_it_appears() {
        let mut table = worked_table();
        table.processes[1].inputs[0] = Qty::new("unobtainium", Ratio::from_u64(3000));
        table.processes[1].tier = "the lost age".into();
        table.processes[1].requires.structure = Some("kiln".into());
        let defects = table.gate();
        let kinds: Vec<RefusalKind> = defects.all().iter().map(|r| r.kind()).collect();
        assert!(kinds.contains(&RefusalKind::UndeclaredName));
        assert!(
            defects.all().iter().filter(|r| r.kind() == RefusalKind::UndeclaredName).count() >= 3,
            "substance, tier and structure are three names: {defects}"
        );
    }

    #[test]
    fn the_converters_are_exact() {
        // The reference's own arithmetic: 60 000 mL of flue gas at 13/10000 g/mL
        // is exactly 78 g — clay's loss on ignition, whole in both units.
        let mut gas = mass("flue_gas", Source::Made);
        gas.unit = Unit::Gas;
        gas.density_g_ml = Some(ratio(13, 10_000));
        gas.sink = Sink::Atmosphere;
        assert_eq!(
            gas.to_grams(Ratio::from_u64(60_000)),
            Ok(Ratio::from_u64(78))
        );

        let mut hatchet = mass("hatchet", Source::Made);
        hatchet.unit = Unit::Count;
        hatchet.unit_mass_g = Some(Ratio::from_u64(2900));
        assert_eq!(hatchet.to_grams(Ratio::ONE), Ok(Ratio::from_u64(2900)));

        let mut water = mass("water", Source::Gathered);
        water.unit = Unit::Volume;
        water.density_g_ml = Some(Ratio::ONE);
        assert_eq!(water.to_grams(Ratio::from_u64(1000)), Ok(Ratio::from_u64(1000)));
    }

    #[test]
    fn floats_are_refused_by_name() {
        for token in ["1.5", "0.1", "1e3", "-2", "3/", "/4", ""] {
            let refusal = parse_ratio(token).expect_err("not a rational");
            assert_eq!(refusal.kind(), RefusalKind::NonRationalQuantity, "for {token:?}");
        }
        assert_eq!(parse_ratio("26/10"), Ok(ratio(13, 5)));
        assert_eq!(parse_ratio("7"), Ok(Ratio::from_u64(7)));
    }

    #[test]
    fn the_level_deltas_are_exact() {
        assert_eq!(Level::Body.lightness_delta(), Ok(Rational::ZERO));
        assert_eq!(Level::Edge.lightness_delta(), Rational::new(1, 10));
        let deep = Level::Deep.lightness_delta().expect("a rational");
        assert_eq!(deep.numerator(), -9);
        assert_eq!(deep.denominator(), 50);
    }

    #[test]
    fn every_enum_round_trips_through_its_name() {
        for family in Family::ALL {
            assert_eq!(Family::parse(family.name()), Ok(family));
        }
        for hue in Hue::ALL {
            assert_eq!(Hue::parse(hue.name()), Ok(hue));
        }
        for unit in Unit::ALL {
            assert_eq!(Unit::parse(unit.name()), Ok(unit));
        }
        for tag in Tag::ALL {
            assert_eq!(Tag::parse(tag.name()), Ok(tag));
        }
        for state in Availability::ALL {
            assert_eq!(Availability::parse(state.marker()), Ok(state));
        }
        for level in Level::ALL {
            assert_eq!(Level::parse(level.name()), Ok(level));
        }
    }
}
