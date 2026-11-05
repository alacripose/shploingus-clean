//! **The classified biomes library** — the atlas's catalog as a package the shapers classify from:
//! the **climate envelopes** that partition the climate plane, the **classified types** that
//! express an envelope on the ground a column actually has, and §15b's **overlays**.
//!
//! # Source
//!
//! * `docs/earth_biomes_subbiomes_terrain.pdf` — §2's broad biomes and their subtypes, and the
//!   premise the library is built on: *"climate sets the envelope → terrain redistributes energy and
//!   water → geology/soil controls storage and access → disturbance selects which vegetation
//!   persists"*.
//! * `docs/earth_biomes_subbiomes_continentalness_mountains.pdf` — §5's broad systems, §15's
//!   *"practical sub-biome catalog: climate and structure"* with its hydrology/substrate overlays,
//!   and §16's ruleset (climate envelope → continentalness → topographic position → orographic
//!   correction → water routing → substrate → potential vegetation).
//!
//! # The package
//!
//! `display-packages/biomes/package.shspec` is the source. Its rows are the catalog, one row per
//! catalog row, and each row's text is `key=value` fields separated by ` | ` — the envelope's bands
//! and formation, the type's envelope/water/position/structure, the overlay's water and position,
//! or a deferred row's reason. [`SHIPPED`] embeds that file, so a test or a tool with no workspace
//! still classifies; [`mount`] installs a copy read at runtime, and the game mounts the folder at
//! boot, so editing the catalog changes what the world classifies without a rebuild. **The first
//! mount wins** for the process's life: a catalog that changed under a running classification would
//! make two columns disagree about what the same climate is.
//!
//! # The three levels
//!
//! * [`Library::envelopes`] — **the climate envelope**. Holdridge's arms, which is the classifier
//!   this world already had, now rows carrying the atlas's own name and system on each arm. The
//!   arms **partition** the climate plane, so a column's two Holdridge numbers name exactly one
//!   envelope, and the envelope is also the *type* when no local expression applies — the atlas's
//!   "broad biome" as the regional default. Its `biome` is the formation
//!   ([`crate::ecology::Biome`]) the ground's materials, flora and map views are drawn from.
//! * [`Library::types`] — **the classified type**: an envelope *mixed* with the water
//!   ([`Hydrology`]) and the terrain position ([`Position`]) the column has, so the wet flat, the
//!   fog belt, the rain-shadow slope and the dry ridge of one climate are four different types.
//!   That mixture is what §15 is a catalog of, and it is *generative*: every row names the
//!   envelope, water and position that produce it, and [`classify`] narrows in the atlas's own
//!   order — envelope, then water, then position — without a branch per climate.
//! * [`Library::overlays`] — §15b's hydrology and substrate overlays, which the atlas says can sit
//!   inside almost any envelope and are what make a terrain read as a place.
//!
//! # What the atlas has and this world cannot read yet
//!
//! [`Library::deferred`] names the §15 and §15b rows that are catalogued but **not** classified,
//! with the field each one waits on: salinity, flow accumulation, flood recurrence, tide, distance
//! to the sea, standing water and groundwater discharge have no producer in the field plane yet, so
//! a row that needed one would be a label nothing could ever be given. It also holds the two peat
//! overlays, which share their water and position with the marsh and are shadowed by it until a
//! reading separates them, and the wet slope type, whose water and slope the ground's own laws
//! cannot produce together. A reason may not outlive the thing it explained — the tests refuse a
//! deferred row that is also in the catalog, and refuse a classified row the ground cannot produce.
//!
//! # The 1/8 scale
//!
//! Every length the atlas quotes is a real-world one and this world is modelled **eight times
//! smaller** ([`crate::scale`]), so the belts are read in *engine* metres ([`Features::above_sea_m`],
//! [`Features::relief_m`]) and a length taken from the atlas goes through [`crate::scale::voxels`].
//! The bands are in the two Holdridge numbers the shapers already aggregate, so nothing here
//! re-derives a climate.

use std::sync::OnceLock;

use crate::ecology::Biome;

/// The recipe header every display package starts with.
const HEADER: &str = "shploingus.recipe/3";

/// `envelope` on a type that stands inside any of them — the elevation belts, which are a matter of
/// height rather than of climate.
const ANY_ENVELOPE: &str = "*";

/// The shipped catalog: the package's own bytes, embedded so the library exists without a workspace.
const SHIPPED: &str = include_str!("../../../display-packages/biomes/package.shspec");

/// A climate envelope's own arms: biotemperature `[low, high)` °C, precipitation `[low, high)` mm.
///
/// The upper bound is open on the wettest arm of each temperature band, so the arms **partition**
/// the climate plane and a climate names exactly one envelope.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bands {
    pub biotemperature_c: [f64; 2],
    pub precipitation_mm: [f64; 2],
}

impl Bands {
    /// Whether a column's own Holdridge pair sits inside this arm.
    pub fn holds(&self, biotemperature_c: f64, precipitation_mm: f64) -> bool {
        (self.biotemperature_c[0]..self.biotemperature_c[1]).contains(&biotemperature_c)
            && (self.precipitation_mm[0]..self.precipitation_mm[1]).contains(&precipitation_mm)
    }
}

/// The vegetation structure a type produces — the atlas's *"vegetation / structure"* column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Structure {
    ClosedCanopy,
    DeciduousCanopy,
    ConiferCanopy,
    Grass,
    Shrub,
    Sparse,
    Wetland,
    Cryptogam,
}

/// **A climate envelope**: one arm of the ladder, with the atlas's name for that climate, the
/// structure its ground carries when nothing local overrides it, and the formation it resolves to.
#[derive(Clone, Copy, Debug)]
pub struct Envelope {
    /// The atlas's own label for this climate.
    pub name: &'static str,
    /// The broad system the atlas's §5 overview files it under.
    pub system: &'static str,
    pub bands: Bands,
    /// The structure the envelope's own regional default carries — read from §15's *"vegetation /
    /// structure"* column for the type that occupies the arm.
    pub structure: Structure,
    /// The formation the ground's materials, the map's views and the flora read.
    pub biome: Biome,
    pub source: &'static str,
}

/// Where a type stands in the landscape — the atlas's *"landscape position"* column, kept to the
/// positions this world's terrain can actually read (see [`Library::deferred`] for the coastal,
/// basin and playa positions and the readings they wait on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// Any ground the envelope's own structure stands on.
    Any,
    /// Broad plains and uplands.
    Plains,
    /// A hillside or rain-shadow slope.
    Slope,
    /// A ridge, talus or summit: exposed, thin soil.
    Ridge,
    /// Flat, poorly drained ground.
    WetFlat,
}

/// The water the column's own ground has — §15's hydrology overlays as a property of a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hydrology {
    /// Whatever water the envelope's own ground is given.
    Any,
    /// Water deficit: ridges, dry uplands, rain-shadow ground.
    Dry,
    /// Water neither scarce nor standing — the zonal case.
    Fresh,
    /// Saturated or seasonally flooded ground.
    Wet,
    /// Immersion in fog or cloud water — the atlas's cloud belt and coastal marine fog.
    Fog,
}

/// **A classified type**: one envelope, the water and the position that produce it, and the
/// structure it carries. The atlas's §15 catalog, row for row.
#[derive(Clone, Copy, Debug)]
pub struct Type {
    /// The atlas's own label for the type.
    pub name: &'static str,
    /// The envelope this type is a ground expression of.
    pub envelope: &'static str,
    pub hydrology: Hydrology,
    pub position: Position,
    pub structure: Structure,
    pub source: &'static str,
}

/// **A §15b hydrology and substrate overlay**, which the atlas says can sit inside almost any
/// envelope and is what makes a terrain read as a place.
#[derive(Clone, Copy, Debug)]
pub struct Overlay {
    pub name: &'static str,
    pub hydrology: Hydrology,
    pub position: Position,
    pub source: &'static str,
}

/// **The catalog a process classifies from**: every row of the mounted package, parsed once.
///
/// The order is the package's order — envelopes as the atlas files them, types in reading order
/// (envelope, then dry to wet, then up the hillside), overlays in the atlas's order, deferred rows
/// last — because [`classify`] narrows by taking the **last** matching type and the **first**
/// matching overlay.
#[derive(Debug)]
pub struct Library {
    /// The package's own name and version, as its `package` line states them.
    pub package: &'static str,
    pub version: &'static str,
    pub envelopes: Vec<Envelope>,
    pub types: Vec<Type>,
    pub overlays: Vec<Overlay>,
    /// `(row, the field it waits on)` — catalogued, not classified.
    pub deferred: Vec<(&'static str, &'static str)>,
}

static LIBRARY: OnceLock<Library> = OnceLock::new();

/// **The catalog in force**: the one [`mount`] installed, or the shipped package's rows.
///
/// A test or a tool with no workspace gets the embedded copy; the game calls [`mount`] with the
/// folder on disk before it opens the world, so what it classifies from is the file it mounted.
pub fn library() -> &'static Library {
    LIBRARY.get_or_init(|| {
        Library::read(SHIPPED).unwrap_or_else(|why| {
            panic!("the embedded biomes package does not read: {why}")
        })
    })
}

/// **Mount a catalog read at runtime.** The first mount wins for the process's life: a second
/// package is refused rather than swapped in under a classification that may already hold rows,
/// and the catalog already in force keeps answering.
pub fn mount(source: &str) -> Result<&'static Library, String> {
    if LIBRARY.get().is_some() {
        return Err(
            "a biomes catalog is already mounted; the first mount is the process's catalog"
                .to_string(),
        );
    }
    let parsed = Library::read(source)?;
    let _ = LIBRARY.set(parsed);
    Ok(LIBRARY.get().expect("set immediately above"))
}

impl Library {
    /// Read a package's rows. Every error names the line and the row it was written on.
    ///
    /// The row names are **leaked**, deliberately: an [`Envelope`] hands its name out as
    /// `&'static str` so a [`Classified`] can be copied, and the package text has no lifetime of
    /// its own. The process's catalog is read once; the leak is a few kilobytes that live as long
    /// as the binary.
    fn read(source: &str) -> Result<Library, String> {
        let mut lines = source.lines().enumerate();
        let header = lines
            .next()
            .map(|(_, line)| line)
            .ok_or("the package is empty")?;
        if header != HEADER {
            return Err(format!("the first line is {header:?}, not {HEADER:?}"));
        }
        let mut package = None;
        let mut version = None;
        let mut envelopes: Vec<Envelope> = Vec::new();
        let mut types: Vec<Type> = Vec::new();
        let mut overlays: Vec<Overlay> = Vec::new();
        let mut deferred: Vec<(&'static str, &'static str)> = Vec::new();
        for (index, line) in lines {
            let number = index + 1;
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("package\t") {
                let fields: Vec<&str> = rest.split('\t').collect();
                match fields.as_slice() {
                    [name, stated] if package.is_none() => {
                        package = Some(leaked(name));
                        version = Some(leaked(stated));
                    }
                    _ => return Err(format!("line {number}: the package line is malformed or twice")),
                }
                continue;
            }
            let fields: Vec<&str> = line.splitn(3, '\t').collect();
            let ["row", id, text] = fields.as_slice() else {
                return Err(format!(
                    "line {number}: {line:?} is not a `package` or `row` line"
                ));
            };
            match row(id, text).map_err(|why| format!("line {number}: {why}"))? {
                Row::Envelope(row) => {
                    if envelopes.iter().any(|held| held.name == row.name) {
                        return Err(format!("line {number}: envelope {} is declared twice", row.name));
                    }
                    envelopes.push(row);
                }
                Row::Type(row) => {
                    if types.iter().any(|held| held.name == row.name) {
                        return Err(format!("line {number}: type {} is declared twice", row.name));
                    }
                    types.push(row);
                }
                Row::Overlay(row) => {
                    if overlays.iter().any(|held| held.name == row.name) {
                        return Err(format!("line {number}: overlay {} is declared twice", row.name));
                    }
                    overlays.push(row);
                }
                Row::Deferred(name, waits_on) => {
                    if deferred.iter().any(|(held, _)| *held == name) {
                        return Err(format!("line {number}: deferred row {name} is declared twice"));
                    }
                    deferred.push((name, waits_on));
                }
            }
        }
        let package = package.ok_or("the package line is missing")?;
        let version = version.ok_or("the package line is missing")?;
        if envelopes.is_empty() {
            return Err(format!("{package} {version} declares no climate envelope"));
        }
        for kind in &types {
            if kind.envelope != ANY_ENVELOPE
                && !envelopes.iter().any(|held| held.name == kind.envelope)
            {
                return Err(format!(
                    "type {} names the envelope {}, which the package does not declare",
                    kind.name, kind.envelope
                ));
            }
        }
        for (name, _) in &deferred {
            if types.iter().any(|held| held.name == *name)
                || overlays.iter().any(|held| held.name == *name)
            {
                return Err(format!(
                    "{name} is deferred and classified at once; a reason may not outlive the \
                     thing it explained"
                ));
            }
        }
        Ok(Library { package, version, envelopes, types, overlays, deferred })
    }
}

fn leaked(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

/// One parsed row, before it is sorted into the catalog.
enum Row {
    Envelope(Envelope),
    Type(Type),
    Overlay(Overlay),
    Deferred(&'static str, &'static str),
}

fn row(id: &str, text: &str) -> Result<Row, String> {
    let fields = Fields::read(id, text)?;
    let name = leaked(fields.get("name")?);
    match fields.get("role")? {
        "envelope" => {
            fields.counted(&[
                "role",
                "name",
                "system",
                "biotemperature",
                "precipitation",
                "structure",
                "biome",
                "source",
            ])?;
            Ok(Row::Envelope(Envelope {
                name,
                system: leaked(fields.get("system")?),
                bands: Bands {
                    biotemperature_c: band(id, "biotemperature", fields.get("biotemperature")?)?,
                    precipitation_mm: band(id, "precipitation", fields.get("precipitation")?)?,
                },
                structure: structure(id, fields.get("structure")?)?,
                biome: biome(id, fields.get("biome")?)?,
                source: leaked(fields.get("source")?),
            }))
        }
        "type" => {
            fields.counted(&[
                "role",
                "name",
                "envelope",
                "hydrology",
                "position",
                "structure",
                "source",
            ])?;
            Ok(Row::Type(Type {
                name,
                envelope: leaked(fields.get("envelope")?),
                hydrology: hydrology(id, fields.get("hydrology")?)?,
                position: position(id, fields.get("position")?)?,
                structure: structure(id, fields.get("structure")?)?,
                source: leaked(fields.get("source")?),
            }))
        }
        "overlay" => {
            fields.counted(&["role", "name", "hydrology", "position", "source"])?;
            Ok(Row::Overlay(Overlay {
                name,
                hydrology: hydrology(id, fields.get("hydrology")?)?,
                position: position(id, fields.get("position")?)?,
                source: leaked(fields.get("source")?),
            }))
        }
        "deferred" => {
            fields.counted(&["role", "name", "waits_on"])?;
            Ok(Row::Deferred(name, leaked(fields.get("waits_on")?)))
        }
        other => Err(format!("{id}: role {other:?} is not envelope, type, overlay or deferred")),
    }
}

/// A row's fields, from its text: `key=value` pairs separated by ` | `.
struct Fields<'a> {
    id: &'a str,
    pairs: Vec<(&'a str, &'a str)>,
}

impl<'a> Fields<'a> {
    fn read(id: &'a str, text: &'a str) -> Result<Fields<'a>, String> {
        let mut pairs: Vec<(&'a str, &'a str)> = Vec::new();
        for field in text.split('|') {
            let (key, value) = field.split_once('=').ok_or_else(|| {
                format!("{id}: {:?} is not a `key=value` field", field.trim())
            })?;
            let (key, value) = (key.trim(), value.trim());
            if key.is_empty() || value.is_empty() {
                return Err(format!("{id}: {:?} names no key or no value", field.trim()));
            }
            if pairs.iter().any(|(held, _)| *held == key) {
                return Err(format!("{id}: {key} is written twice"));
            }
            pairs.push((key, value));
        }
        Ok(Fields { id, pairs })
    }

    fn get(&self, key: &str) -> Result<&'a str, String> {
        self.pairs
            .iter()
            .find(|(held, _)| *held == key)
            .map(|(_, value)| *value)
            .ok_or_else(|| format!("{}: no {key}", self.id))
    }

    /// Refuse a field the row's role does not have: a plausible value read by nothing and checked
    /// by nothing is the failure the package format is built against.
    fn counted(&self, keys: &[&str]) -> Result<(), String> {
        for (key, _) in &self.pairs {
            if !keys.contains(key) {
                return Err(format!("{}: {key} is not a field of its row", self.id));
            }
        }
        Ok(())
    }
}

fn band(id: &str, key: &str, text: &str) -> Result<[f64; 2], String> {
    let (low, high) = text
        .split_once("..")
        .ok_or_else(|| format!("{id}: {key} {text:?} is not a `low..high` band"))?;
    let end = |word: &str, open: f64| -> Result<f64, String> {
        if word == "open" {
            return Ok(open);
        }
        word.parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("{id}: {key} {text:?} has no finite number {word:?}"))
    };
    let low = end(low, f64::NEG_INFINITY)?;
    let high = end(high, f64::INFINITY)?;
    if low >= high {
        return Err(format!("{id}: {key} {text:?} is empty"));
    }
    Ok([low, high])
}

fn structure(id: &str, text: &str) -> Result<Structure, String> {
    match text {
        "ClosedCanopy" => Ok(Structure::ClosedCanopy),
        "DeciduousCanopy" => Ok(Structure::DeciduousCanopy),
        "ConiferCanopy" => Ok(Structure::ConiferCanopy),
        "Grass" => Ok(Structure::Grass),
        "Shrub" => Ok(Structure::Shrub),
        "Sparse" => Ok(Structure::Sparse),
        "Wetland" => Ok(Structure::Wetland),
        "Cryptogam" => Ok(Structure::Cryptogam),
        other => Err(format!("{id}: structure {other:?} is not one the ground has")),
    }
}

fn biome(id: &str, text: &str) -> Result<Biome, String> {
    Biome::ALL
        .iter()
        .copied()
        .find(|biome| biome.name() == text)
        .ok_or_else(|| format!("{id}: biome {text:?} is not a formation"))
}

fn hydrology(id: &str, text: &str) -> Result<Hydrology, String> {
    match text {
        "Any" => Ok(Hydrology::Any),
        "Dry" => Ok(Hydrology::Dry),
        "Fresh" => Ok(Hydrology::Fresh),
        "Wet" => Ok(Hydrology::Wet),
        "Fog" => Ok(Hydrology::Fog),
        other => Err(format!("{id}: hydrology {other:?} is not one the ground reads")),
    }
}

fn position(id: &str, text: &str) -> Result<Position, String> {
    match text {
        "Any" => Ok(Position::Any),
        "Plains" => Ok(Position::Plains),
        "Slope" => Ok(Position::Slope),
        "Ridge" => Ok(Position::Ridge),
        "WetFlat" => Ok(Position::WetFlat),
        other => Err(format!("{id}: position {other:?} is not one the ground reads")),
    }
}

/// **The features the classification mixes**: the two climate aggregates, the water the ground
/// actually has, and the terrain its position is read from — all in engine metres, the 1/8 scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Features {
    pub biotemperature_c: f64,
    pub precipitation_mm: f64,
    /// Boundary-layer moisture, `[0, 1]` ([`crate::climate::moisture`]).
    pub moisture: f64,
    pub above_sea_m: f64,
    /// The world's own relief in engine metres ([`crate::spec::Config::relief_m`]) — the
    /// denominator of the elevation belts.
    pub relief_m: f64,
    /// The surface's gradient magnitude ([`crate::terrain::TerrainSample::slope`]).
    pub slope: f64,
}

/// The slope at or above which ground reads as a **ridge**; the moisture at or above which it reads
/// as **wet** and at or below which it reads as **dry**. Read from the habitat layer's own lines
/// rather than guessed again, so the classification and the ground's material answer agree about
/// where a rock face and a marsh are.
pub const RIDGE_SLOPE: f64 = crate::habitat::STEEP_SLOPE;
pub const WET_MOISTURE: f64 = crate::habitat::WET_MOISTURE;
pub const DRY_MOISTURE: f64 = crate::habitat::DRY_MOISTURE;

impl Features {
    /// The water the column's own ground has.
    pub fn hydrology(&self) -> Hydrology {
        if self.soaked() {
            Hydrology::Wet
        } else if self.slope >= RIDGE_SLOPE || self.moisture <= DRY_MOISTURE {
            Hydrology::Dry
        } else {
            Hydrology::Fresh
        }
    }

    /// The position the column's own terrain puts it in.
    pub fn position(&self) -> Position {
        if self.slope >= RIDGE_SLOPE || self.above_sea_m >= self.alpine_m() {
            Position::Ridge
        } else if self.soaked() {
            Position::WetFlat
        } else if self.slope >= crate::habitat::DRY_SLOPE || self.above_sea_m >= self.montane_m() {
            Position::Slope
        } else {
            Position::Plains
        }
    }

    /// Whether the column stands in a fog or cloud belt: moist ground on the altitude band where the
    /// air is dropping its water — where the atlas's cloud forest, fog desert and temperate rain
    /// forest all sit. It is the same anti-correlation [`crate::climate::moisture_parts`] already
    /// applies between height and moisture.
    pub fn fog(&self) -> bool {
        let height = (self.above_sea_m / self.relief_m.max(1.0)).clamp(0.0, 1.0);
        self.moisture >= WET_MOISTURE && (0.25..=0.75).contains(&height)
    }

    /// The water the classification reads: the ground's own regime, or the fog belt when the column
    /// stands in one.
    pub fn water(&self) -> Hydrology {
        if self.fog() {
            Hydrology::Fog
        } else {
            self.hydrology()
        }
    }

    fn soaked(&self) -> bool {
        self.moisture >= WET_MOISTURE && self.slope <= crate::habitat::WET_SLOPE
    }

    /// The height above sea level at which a column has left its envelope for the alpine — the
    /// habitat layer's own alpine override, so the two cannot disagree.
    fn alpine_m(&self) -> f64 {
        self.relief_m * crate::ecology::ALPINE_HEIGHT_ABOVE_RELIEF
    }

    /// The height at which the montane belt begins.
    fn montane_m(&self) -> f64 {
        self.relief_m * crate::habitat::MONTANE_HEIGHT_FRACTION
    }
}

/// **One column's classification**: the envelope its climate occupies, the type that envelope takes
/// on this ground — `None` when the envelope's own regional default stands, which is the atlas's
/// "broad biome" with no local expression — and the §15b overlay the ground carries.
#[derive(Clone, Copy, Debug)]
pub struct Classified {
    pub envelope: &'static Envelope,
    pub kind: Option<&'static Type>,
    pub overlay: Option<&'static Overlay>,
}

impl Classified {
    /// The classified type's name: the terrain expression's, or the envelope's own when no local
    /// expression applies.
    pub fn name(&self) -> &'static str {
        self.kind.map_or(self.envelope.name, |kind| kind.name)
    }

    /// The structure the ground carries.
    pub fn structure(&self) -> Structure {
        self.kind.map_or(self.envelope.structure, |kind| kind.structure)
    }

    /// The formation the ground's materials and the map are drawn from.
    pub fn biome(&self) -> Biome {
        self.envelope.biome
    }
}

/// **Classify a column**, progressively: the envelope from the climate, the type from that envelope
/// mixed with the column's water and terrain position, the overlay from the ground's own hydrology.
///
/// Each step keeps what the step before it kept and a step that keeps nothing leaves the previous
/// answer standing — so the classification is total for every column in the world, and a column the
/// atlas does not name locally is its envelope rather than a hole. Within a step the **last** type
/// wins, which is why the types are ordered regional default last, and the **first** overlay wins,
/// which is why the marsh is before the peat that shares its flat.
pub fn classify(features: &Features) -> Classified {
    let catalog = library();
    let envelope = envelope_of(features.biotemperature_c, features.precipitation_mm);
    let water = features.water();
    let place = features.position();
    // **The elevation belts are a vertical ladder, not a later row.** Ground above the montane line
    // is classified by the belts; ground below it by its climate envelope, and the belts are not
    // candidates there — otherwise the alpine's dry ridge would answer for a tundra column that
    // merely stands on a ridge.
    let belted = features.above_sea_m >= features.montane_m();
    let in_scope = |kind: &&'static Type| {
        if belted {
            kind.envelope == ANY_ENVELOPE
        } else {
            kind.envelope == envelope.name
        }
    };
    let wetter = |kind: &&'static Type| {
        (kind.hydrology == water || kind.hydrology == Hydrology::Any) && in_scope(kind)
    };
    let kind = if belted {
        catalog
            .types
            .iter()
            .rev()
            .find(|kind| wetter(kind) && (kind.position == place || kind.position == Position::Any))
            .or_else(|| catalog.types.iter().rev().find(|kind| wetter(kind)))
            .or_else(|| catalog.types.iter().rev().find(in_scope))
    } else {
        // A ground expression the column's own water does not match is *not* its type: a fresh
        // flat is not the cove, the swamp or the cloud forest, it is the envelope's regional
        // default — the atlas's broad biome, which is why this step may end in nothing.
        catalog
            .types
            .iter()
            .rev()
            .find(|kind| in_scope(kind) && wetter(kind) && (kind.position == place || kind.position == Position::Any))
            .or_else(|| catalog.types.iter().rev().find(|kind| in_scope(kind) && wetter(kind)))
    };
    // **The atlas's own order is the overlays' priority**: marsh before the peat rows, because a
    // marsh is what saturated flat ground is until a peat reading says otherwise.
    let overlay = catalog
        .overlays
        .iter()
        .find(|overlay| overlay.hydrology == water && (overlay.position == place || overlay.position == Position::Any))
        .or_else(|| catalog.overlays.iter().find(|overlay| overlay.hydrology == water));
    Classified { envelope, kind, overlay }
}

/// The envelope a climate occupies: the last arm whose bands hold. The arms partition the plane, so
/// this is a lookup and not a search, and the last arm is the floor for a climate outside them.
pub fn envelope_of(biotemperature_c: f64, precipitation_mm: f64) -> &'static Envelope {
    let envelopes = &library().envelopes;
    envelopes
        .iter()
        .rev()
        .find(|envelope| envelope.bands.holds(biotemperature_c, precipitation_mm))
        .unwrap_or_else(|| envelopes.last().expect("a package with no envelope is refused"))
}

/// The **formation** of a climate — what [`crate::ecology::biome_for`] answers, and the one every
/// reader of a [`Biome`] gets.
pub fn climate_biome(biotemperature_c: f64, precipitation_mm: f64) -> Biome {
    envelope_of(biotemperature_c, precipitation_mm).biome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Config;

    fn features(biotemperature_c: f64, precipitation_mm: f64) -> Features {
        Features {
            biotemperature_c,
            precipitation_mm,
            moisture: 0.5,
            above_sea_m: 0.0,
            relief_m: Config::default().relief_m,
            slope: 0.0,
        }
    }

    /// A climate inside an envelope's own arms, in the middle of the first precipitation arm.
    fn climate_of(envelope: &Envelope) -> (f64, f64) {
        let t = (envelope.bands.biotemperature_c[0].max(-40.0)
            + envelope.bands.biotemperature_c[1].min(40.0))
            / 2.0;
        (t, envelope.bands.precipitation_mm[0] + 10.0)
    }

    /// **A column the ground laws can actually produce**, asked for by the water and position its
    /// row names.
    ///
    /// The fields are not independent: saturation and slope cannot both read (`WET_SLOPE` is below
    /// `DRY_SLOPE`), a column above the montane line is classified by the belts rather than by its
    /// envelope, and a ridge is either steep or high. A generator that set each field on its own
    /// would "reach" rows no column can — which is exactly the mistake the deferred rows name.
    fn column(water: Hydrology, place: Position, belted: bool) -> Features {
        let relief_m = Config::default().relief_m;
        let mut f = features(0.0, 0.0);
        f.relief_m = relief_m;
        match water {
            Hydrology::Wet | Hydrology::Fog => f.moisture = WET_MOISTURE + 0.1,
            Hydrology::Dry => f.moisture = DRY_MOISTURE - 0.1,
            _ => {}
        }
        match place {
            Position::Plains | Position::Any => {}
            Position::Slope => f.slope = crate::habitat::DRY_SLOPE,
            Position::WetFlat => f.moisture = WET_MOISTURE + 0.1,
            // A ridge the belts classify is a high one, because a belt's ground is above the
            // montane line whether it is steep or not; an envelope's ridge is steep ground.
            Position::Ridge if belted => f.above_sea_m = relief_m * 0.9,
            Position::Ridge => f.slope = RIDGE_SLOPE + 0.1,
        }
        if water == Hydrology::Fog {
            // Inside the fog belt, below the montane line unless the row is a belt's own.
            f.above_sea_m = f.above_sea_m.max(relief_m * 0.3);
        }
        if belted && place != Position::Ridge {
            // Above the montane line but below the alpine override: the belts' Slope.
            f.above_sea_m = relief_m * 0.6;
        }
        f
    }

    #[test]
    fn the_envelopes_partition_the_climate_plane() {
        // **One climate, one envelope.** A hole would make the classification depend on the
        // fallback, and an overlap would make it depend on the package's order.
        let envelopes = &library().envelopes;
        for t in [-30.0, -12.0, -5.0, 0.0, 9.0, 15.0, 22.0, 35.0] {
            for p in [0.0, 149.0, 249.0, 449.0, 749.0, 901.0, 2_500.0] {
                let holding: Vec<&Envelope> = envelopes
                    .iter()
                    .filter(|envelope| envelope.bands.holds(t, p))
                    .collect();
                assert_eq!(
                    holding.len(),
                    1,
                    "the arms do not partition the plane at {t} C / {p} mm: {:?}",
                    holding.iter().map(|e| e.name).collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn the_package_is_the_shipped_source() {
        // The embedded copy and the folder are one declaration. The bytes are the file's, so this
        // fails the moment a build is made against a different package than the one on disk.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../display-packages/biomes/package.shspec");
        let on_disk = std::fs::read_to_string(path).expect("display-packages/biomes/package.shspec");
        assert_eq!(
            SHIPPED, on_disk,
            "the embedded biomes package and {path} are not the same bytes"
        );
        let catalog = library();
        assert!(!catalog.envelopes.is_empty());
        assert!(!catalog.types.is_empty());
        assert!(!catalog.overlays.is_empty());
    }

    #[test]
    fn the_envelope_rows_are_the_ladder_it_replaced() {
        // The arms carry Holdridge's own numbers, so the envelope lookup must answer exactly what
        // `ecology::biome_for` answered before the library existed — otherwise this is a second
        // classifier wearing the first one's name.
        let mut t = -40.0;
        while t < 45.0 {
            let mut p = 0.0;
            while p < 2_500.0 {
                assert_eq!(
                    climate_biome(t, p),
                    ladder_it_replaced(t, p),
                    "the library and the ladder disagree at {t} C / {p} mm"
                );
                p += 25.0;
            }
            t += 0.5;
        }
    }

    /// **The ladder the library replaced**, kept as the test's own oracle so the upgrade is
    /// checkable against the behaviour it preserved. It is a copy of the arms that were in
    /// `ecology::biome_for` when the catalog landed, and nothing in the world reads it.
    fn ladder_it_replaced(temperature_c: f64, precipitation_mm: f64) -> Biome {
        if temperature_c < -5.0 {
            return if precipitation_mm < 250.0 { Biome::PolarDesert } else { Biome::Tundra };
        }
        if temperature_c < 3.0 {
            return if precipitation_mm < 250.0 { Biome::Tundra } else { Biome::BorealForest };
        }
        if temperature_c < 12.0 {
            return match precipitation_mm {
                p if p < 200.0 => Biome::Tundra,
                p if p < 450.0 => Biome::Grassland,
                p if p < 900.0 => Biome::CoolTemperateForest,
                _ => Biome::BorealForest,
            };
        }
        if temperature_c < 20.0 {
            return match precipitation_mm {
                p if p < 180.0 => Biome::Desert,
                p if p < 400.0 => Biome::Grassland,
                p if p < 750.0 => Biome::SubtropicalScrub,
                _ => Biome::TemperateForest,
            };
        }
        match precipitation_mm {
            p if p < 150.0 => Biome::Desert,
            p if p < 350.0 => Biome::SubtropicalScrub,
            p if p < 800.0 => Biome::TropicalDryForest,
            _ => Biome::TropicalRainforest,
        }
    }

    #[test]
    fn every_type_is_reachable_and_stands_in_its_own_envelope() {
        // A catalog with an unreachable row is a catalog with a dead entry, and the dead entry is
        // the one a later change trusts: each type is asked for with its own envelope, water and
        // position, and the ground laws must produce it.
        for wanted in &library().types {
            let belted = wanted.envelope == ANY_ENVELOPE;
            let envelope = if belted {
                library().envelopes.last().expect("a package declares an envelope")
            } else {
                library()
                    .envelopes
                    .iter()
                    .find(|envelope| envelope.name == wanted.envelope)
                    .unwrap_or_else(|| {
                        panic!("{} names an envelope the package does not have", wanted.name)
                    })
            };
            let (t, p) = climate_of(envelope);
            let mut f = column(wanted.hydrology, wanted.position, belted);
            (f.biotemperature_c, f.precipitation_mm) = (t, p);
            let got = classify(&f);
            assert_eq!(
                got.name(),
                wanted.name,
                "the catalog cannot generate its own row: asked for {} at {t:.1} C / {p:.0} mm \
                 (water {:?}, position {:?}), got {} (water {:?}, position {:?})",
                wanted.name,
                wanted.hydrology,
                wanted.position,
                got.name(),
                f.water(),
                f.position()
            );
        }
    }

    #[test]
    fn every_overlay_is_reachable() {
        // An overlay is reachable iff its own water and position are, *and* no earlier overlay
        // shares them — which is why the two peat rows are deferred rather than listed.
        for wanted in &library().overlays {
            // A saturating flat would sit in the fog belt if it stood above the montane line, and
            // fog is a different water: the flat is asked for at the ground, the ridge in the belt.
            let f = column(wanted.hydrology, wanted.position, wanted.position == Position::Ridge);
            let got = classify(&f);
            assert_eq!(
                got.overlay.map(|overlay| overlay.name),
                Some(wanted.name),
                "asked for the {} overlay (water {:?}, position {:?}), got {:?}",
                wanted.name,
                wanted.hydrology,
                wanted.position,
                got.overlay.map(|overlay| overlay.name)
            );
        }
    }

    #[test]
    fn every_deferred_row_names_what_it_waits_on_and_is_not_in_the_tables() {
        // **A reason may not outlive the thing it explained**: a deferred row that has quietly
        // joined the catalog is a row whose reason is now a lie.
        for (name, reason) in &library().deferred {
            assert!(reason.len() > 20, "{name} does not say what it waits on");
            assert!(
                !library().types.iter().any(|kind| kind.name == *name),
                "{name} is deferred and classified at once"
            );
            assert!(
                !library().overlays.iter().any(|overlay| overlay.name == *name),
                "{name} is deferred and overlaid at once"
            );
        }
    }

    #[test]
    fn the_classification_is_total_and_deterministic() {
        // Total at the catalog's edges — no hole, no panic — and stable for a column, which is what
        // lets a material and a map view be read from it.
        for (t, p, slope, above) in [
            (-80.0, 0.0, 0.0, 0.0),
            (-40.0, 12_000.0, 0.1, 0.0),
            (0.0, 0.0, 0.9, 60.0),
            (60.0, 20_000.0, 0.0, 0.0),
            (25.0, 900.0, 0.3, 30.0),
        ] {
            let mut f = features(t, p);
            f.slope = slope;
            f.above_sea_m = above;
            assert_eq!(classify(&f).name(), classify(&f).name());
            assert!(!classify(&f).name().is_empty());
        }
    }

    #[test]
    fn every_row_names_the_atlas_section_it_was_read_from() {
        for envelope in &library().envelopes {
            assert!(!envelope.name.is_empty() && !envelope.system.is_empty());
            assert!(envelope.source.contains('§'), "{} has no section", envelope.name);
        }
        for kind in &library().types {
            assert!(kind.source.contains('§'), "{} has no section", kind.name);
        }
        for overlay in &library().overlays {
            assert!(overlay.source.contains('§'), "{} has no section", overlay.name);
        }
    }

    #[test]
    fn the_record_and_the_library_name_the_same_rows_in_the_same_order() {
        // **The baselines record is not a copy that can drift**: `docs/baselines/biomes.json` carries
        // the package's rows, with their roles, in the package's own order, and this is the check
        // that keeps the recorded artifact and the catalog from disagreeing. The record's measured
        // rows follow the catalog's, so only its head is compared.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/baselines/biomes.json");
        let record = std::fs::read_to_string(path).expect("docs/baselines/biomes.json");
        let field = |line: &str, key: &str| -> Option<String> {
            let start = line.find(&format!("\"{key}\": \""))? + key.len() + 5;
            let rest = &line[start..];
            Some(rest[..rest.find('"')?].to_string())
        };
        let mut named: Vec<(String, String)> = Vec::new();
        for line in record.lines() {
            let Some(name) = field(line, "name") else { continue };
            let Some(role) = field(line, "role") else { continue };
            named.push((role, name));
        }
        let catalog = library();
        let mut declared: Vec<(String, String)> = Vec::new();
        declared.extend(
            catalog
                .envelopes
                .iter()
                .map(|row| ("envelope".to_string(), row.name.to_string())),
        );
        declared.extend(
            catalog
                .types
                .iter()
                .map(|row| ("type".to_string(), row.name.to_string())),
        );
        declared.extend(
            catalog
                .overlays
                .iter()
                .map(|row| ("overlay".to_string(), row.name.to_string())),
        );
        declared.extend(
            catalog
                .deferred
                .iter()
                .map(|(name, _)| ("deferred".to_string(), (*name).to_string())),
        );
        assert!(
            named.len() >= declared.len(),
            "the record names {} rows against the catalog's {}",
            named.len(),
            declared.len()
        );
        assert_eq!(
            &named[..declared.len()],
            declared.as_slice(),
            "docs/baselines/biomes.json does not name the catalog's rows, with their roles, in order"
        );
    }

    #[test]
    fn the_water_and_the_position_are_read_from_the_ground() {
        let mut f = features(15.0, 900.0);
        f.moisture = WET_MOISTURE + 0.1;
        f.slope = 0.0;
        assert_eq!(f.hydrology(), Hydrology::Wet);
        assert_eq!(f.position(), Position::WetFlat);
        assert_eq!(classify(&f).overlay.map(|o| o.name), Some("marsh_wet_meadow"));

        f.moisture = 0.5;
        f.slope = RIDGE_SLOPE + 0.1;
        assert_eq!(f.hydrology(), Hydrology::Dry);
        assert_eq!(f.position(), Position::Ridge);
        assert_eq!(classify(&f).overlay.map(|o| o.name), Some("rockland_talus"));

        // Dry ground with no ridge keeps the first dry overlay the atlas lists; zonal ground whose
        // water has no overlay yet (§15b's lake littoral and springs are deferred) has none.
        f.slope = 0.2;
        f.moisture = 0.2;
        assert_eq!(f.hydrology(), Hydrology::Dry);
        assert_eq!(classify(&f).overlay.map(|o| o.name), Some("rockland_talus"));
        f.moisture = 0.5;
        assert_eq!(f.hydrology(), Hydrology::Fresh);
        assert!(classify(&f).overlay.is_none());
    }

    #[test]
    fn a_column_on_the_alpine_is_a_belt_and_a_column_below_it_is_its_envelope() {
        // The belts are a vertical ladder: the same climate classifies by its envelope low down and
        // by the belt high up, and the alpine override is the habitat layer's own line.
        let mut f = features(15.0, 900.0);
        f.moisture = 0.5;
        f.slope = 0.2;
        assert_eq!(classify(&f).envelope.name, "temperate_deciduous_forest");
        assert_eq!(classify(&f).name(), "temperate_deciduous_forest");
        f.above_sea_m = f.relief_m * 0.5;
        assert!(classify(&f).kind.is_some_and(|kind| kind.envelope == ANY_ENVELOPE));
    }

    #[test]
    fn the_catalog_is_what_the_report_reads() {
        // The classifier's answer is the package's, not a table's: a doctored copy of the shipped
        // bytes changes what a climate resolves to, and a row that cannot be read is refused by
        // line rather than skipped.
        let doctored = SHIPPED.replacen(
            "biome=polar_desert",
            "biome=desert",
            1,
        );
        assert_ne!(doctored, SHIPPED, "the polar desert row moved");
        let catalog = Library::read(&doctored).expect("the doctored package reads");
        assert_eq!(catalog.envelopes[0].name, "polar_desert");
        assert_eq!(catalog.envelopes[0].biome, Biome::Desert);

        let missing = SHIPPED.replacen("role=envelope", "role=enveloppe", 1);
        let refused = Library::read(&missing).expect_err("an unknown role is refused");
        assert!(refused.contains("ENV-0001"), "{refused}");
        assert!(refused.contains("enveloppe"), "{refused}");
    }
}
