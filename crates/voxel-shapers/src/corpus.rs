//! The **corpus** — one admitted material per declared substance, and the
//! appearance rule that gives each its colour (`LEDGER.md` C-122, C-141).
//!
//! The declaration under `voxel-model/materials/substances.tsv` says what exists;
//! this module is where each declared substance **becomes a material**, so the
//! registry and the declaration cannot disagree about the set. `voxel-client`'s
//! corpus census is the sensor: it ranges over the declaration and reports every
//! substance that is not admitted, by name.
//!
//! **Appearance derives from composition (`C-141`).** A row is a list of
//! constituents — each the *substance's* own measured USGS v7 sample, or the
//! sample of the mineral it is made of — and the row's reflectance is the exact
//! rational mass-weighted blend of theirs. Where a substance is one measured
//! substance (chert, halite, hematite), the row **is** that sample and carries
//! its provenance verbatim rather than restating it. Where a substance is a rock,
//! a mixture or a made good, the row is composed and states the composition and
//! where it came from.
//!
//! **What this is not.** The blend is a *declared linear mix*, not a
//! radiative-transfer model: real granular surfaces scatter non-linearly, so a
//! composed row is this project's stated rule and its source string says so. And
//! where a row's constituents' blend density differs from the declaration's own
//! bulk figure, the two are **different quantities** — a mineral's solid density
//! against a deposit's bulk or porous density — not a disagreement to be averaged
//! away.
//!
//! **A row that cannot be composed is refused by name, not approximated.** Five
//! declared substances have neither a sample nor a composition that could carry
//! one honestly; they are listed in [`NO_REFLECTANCE`] with their reasons, and
//! the census fails if one of them *gains* a reflectance without the entry being
//! removed — a reason may not outlive the thing it explained.

use std::collections::BTreeMap;

use voxel_model::materials::spectra;
use voxel_model::{
    blend_reflectance, Component, ComponentId, Material, MaterialId, MaterialRegistry,
    OpticalClass, Ratio, Refusal, RefusalKind, Refused, SourcedReflectance,
};
use voxel_noise::mix;

use crate::geology::{self, Lithology, OreDials, OreKind};
use crate::spec::WorldSpec;

/// The first material id the corpus claims. Ids 1..=13 are the world's own
/// materials in [`crate::column::materials`]; the corpus is additive, so the
/// world's ids do not move when content lands.
pub const FIRST_MATERIAL_ID: u16 = 14;

/// The first component id the corpus claims. The world's own rows use their
/// material id as their component id (1..=13), so the corpus starts clear of them.
pub const FIRST_COMPONENT_ID: u16 = 100;

/// One constituent of a row: a substance, its mass share, its density, and the
/// measured sample that gives it a reflectance.
#[derive(Clone, Copy)]
pub struct Part {
    /// The constituent's own name — what it *is*, not what the row is called.
    pub name: &'static str,
    /// Its mass share, in hundredths, so the ratios stay exact integers.
    pub percent: u64,
    pub density_kg_m3: u64,
    /// The measured sample this constituent's reflectance comes from.
    pub sample: fn() -> Refused<SourcedReflectance>,
}

/// One declared substance's appearance.
pub struct Row {
    /// The declared substance's own name; the census matches on it.
    pub name: &'static str,
    pub optical: OpticalClass,
    pub parts: &'static [Part],
    /// The composition statement for a **composed** row, or `None` when the row
    /// is one measured substance and must carry that sample's provenance whole.
    pub mix: Option<&'static str>,
}

/// A constituent, so the table below reads as a composition rather than as a call.
macro_rules! p {
    ($name:literal, $percent:literal, $density:literal, $sample:path) => {
        Part { name: $name, percent: $percent, density_kg_m3: $density, sample: $sample }
    };
}

/// Every declared substance's appearance, in declaration order.
pub const ROWS: &[Row] = &[
    // ---- the rung substances and the made goods -------------------------
    Row {
        name: "stone",
        optical: OpticalClass::Opaque,
        mix: Some(
            "stone: v7 holds no whole-stone spectrum, so the row is the composition of the common \
             crustal rock-forming minerals (quartz 30/100, plagioclase 40/100, alkali feldspar \
             30/100) — this project's stated choice for the generic deposit kind, and the declared \
             linear mix of C-141 rather than a radiative-transfer model",
        ),
        parts: &[
            p!("quartz", 30, 2650, spectra::quartz),
            p!("plagioclase", 40, 2680, spectra::plagioclase),
            p!("alkali feldspar", 30, 2560, spectra::alkali_feldspar),
        ],
    },
    Row {
        name: "clay",
        optical: OpticalClass::Opaque,
        mix: Some(
            "clay: the deposit kind is a mixture of clay minerals; the row is its two dominant \
             ones, kaolinite 60/100 and illite 40/100 — this project's stated choice — and the \
             declared linear mix of C-141",
        ),
        parts: &[
            p!("kaolinite", 60, 2600, spectra::kaolinite),
            p!("illite", 40, 2750, spectra::illite),
        ],
    },
    Row {
        name: "coal",
        optical: OpticalClass::Opaque,
        mix: Some(
            "coal: v7 holds no coal spectrum, so the row is its organic matter, kerogen 100/100, \
             as the measured stand-in for the maceral mixture — flagged, because the mineral \
             (ash) fraction is a different material and is not pretended into this one",
        ),
        parts: &[p!("kerogen", 100, 1300, spectra::kerogen)],
    },
    Row {
        name: "iron_ore",
        optical: OpticalClass::Opaque,
        mix: Some(
            "iron ore: the hematite/magnetite grade — the row is the ore minerals with quartz \
             gangue (hematite 60/100, magnetite 30/100, quartz 10/100), this project's stated \
             choice; the blend is the solid ore's density, while the declaration's 2700 kg/m³ is \
             the deposit's bulk figure, and the two are different quantities",
        ),
        parts: &[
            p!("hematite", 60, 5200, spectra::hematite),
            p!("magnetite", 30, 5170, spectra::magnetite),
            p!("quartz", 10, 2650, spectra::quartz),
        ],
    },
    Row {
        name: "timber",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("timber", 100, 700, spectra::wood_beam)],
    },
    Row {
        name: "plant_fibre",
        optical: OpticalClass::Opaque,
        mix: Some(
            "plant fibre: v7 holds no loose fibre, so the cotton fabric sample stands for the \
             spun fibre it is made of — flagged, because a fabric is an arrangement of the fibre \
             and not the fibre itself",
        ),
        parts: &[p!("cotton fabric", 100, 300, spectra::cotton_fabric)],
    },
    Row {
        name: "haft_blank",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("timber", 100, 700, spectra::wood_beam)],
    },
    Row {
        name: "knapped_edge",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("chert", 100, 2600, spectra::chert)],
    },
    Row {
        name: "cord",
        optical: OpticalClass::Opaque,
        mix: Some(
            "cord: twisted fibre — the cotton sample stands for the fibre, flagged; the twist \
             adds no measured appearance of its own",
        ),
        parts: &[p!("cotton fabric", 100, 500, spectra::cotton_fabric)],
    },
    Row {
        name: "hatchet",
        optical: OpticalClass::Opaque,
        mix: Some(
            "hatchet: the Count row's density is composed here — a chert head and a timber haft \
             at 70/100 and 30/100 by mass, this project's stated choice, giving 2030 kg/m³ — and \
             its appearance is the same two parts' measured samples blended",
        ),
        parts: &[
            p!("chert", 70, 2600, spectra::chert),
            p!("timber", 30, 700, spectra::wood_beam),
        ],
    },
    Row {
        name: "timber_offcuts",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("timber", 100, 700, spectra::wood_beam)],
    },
    Row {
        name: "stone_flakes",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("chert", 100, 2600, spectra::chert)],
    },
    Row {
        name: "fibre_dust",
        optical: OpticalClass::Opaque,
        mix: Some("fibre dust: short fibres too fine to twist — the cotton sample, flagged"),
        parts: &[p!("cotton fabric", 100, 300, spectra::cotton_fabric)],
    },
    Row {
        name: "trim_waste",
        optical: OpticalClass::Opaque,
        mix: Some(
            "trim waste: the declaration's own description is a mixture of stone, timber and \
             fibre, so the row is those three in equal shares (34/100, 33/100, 33/100, exactly \
             accounting to the whole) — this project's stated choice",
        ),
        parts: &[
            p!("chert", 34, 2600, spectra::chert),
            p!("timber", 33, 700, spectra::wood_beam),
            p!("cotton fabric", 33, 300, spectra::cotton_fabric),
        ],
    },
    Row {
        name: "forage",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("cheatgrass", 100, 600, spectra::cheatgrass)],
    },
    Row {
        name: "charcoal",
        optical: OpticalClass::Opaque,
        mix: Some(
            "charcoal: carbonised wood — v7's only measured carbon is an amorphous soot, so the \
             row is carbon 100/100 as the carbon fraction, flagged; the mineral ash it leaves is \
             the `ash` row's business, not this one's",
        ),
        parts: &[p!("carbon", 100, 500, spectra::carbon)],
    },
    Row {
        name: "brick",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("brick", 100, 1900, spectra::brick)],
    },
    Row {
        name: "ash",
        optical: OpticalClass::Opaque,
        mix: Some(
            "ash: the mineral part of wood and charcoal — v7 holds no ash sample, so the row is \
             its two dominant mineral constituents, calcite 50/100 and quartz 50/100, this \
             project's stated choice",
        ),
        parts: &[
            p!("calcite", 50, 2710, spectra::calcite),
            p!("quartz", 50, 2650, spectra::quartz),
        ],
    },
    // ---- the igneous rocks (IUGS QAPF fields) --------------------------
    Row {
        name: "granite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "granite: the declared modal mineralogy (quartz 25/100, plagioclase 35/100, alkali \
             feldspar 30/100, biotite 10/100) sits inside the IUGS QAPF granite field — this \
             project's choice stated inside that published range; each constituent's own source \
             names its USGS sample, and the blend is the declared linear mix of C-141",
        ),
        parts: &[
            p!("quartz", 25, 2650, spectra::quartz),
            p!("plagioclase", 35, 2680, spectra::plagioclase),
            p!("alkali feldspar", 30, 2560, spectra::alkali_feldspar),
            p!("biotite", 10, 3050, spectra::biotite),
        ],
    },
    Row {
        name: "diorite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "diorite: plagioclase 55/100 with hornblende 30/100 and biotite 15/100 — inside the \
             IUGS diorite field, this project's stated choice; declared linear mix (C-141)",
        ),
        parts: &[
            p!("plagioclase", 55, 2680, spectra::plagioclase),
            p!("hornblende", 30, 3200, spectra::hornblende),
            p!("biotite", 15, 3050, spectra::biotite),
        ],
    },
    Row {
        name: "gabbro",
        optical: OpticalClass::Opaque,
        mix: Some(
            "gabbro: plagioclase 50/100 with pyroxene 40/100 and olivine 10/100 — inside the IUGS \
             gabbro field, this project's stated choice; declared linear mix (C-141)",
        ),
        parts: &[
            p!("plagioclase", 50, 2680, spectra::plagioclase),
            p!("pyroxene", 40, 3300, spectra::pyroxene),
            p!("olivine", 10, 3300, spectra::olivine),
        ],
    },
    Row {
        name: "rhyolite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "rhyolite: the extrusive equivalent of granite — quartz 30/100, alkali feldspar \
             40/100, plagioclase 25/100, biotite 5/100, inside the IUGS rhyolite field; this \
             project's stated choice, declared linear mix (C-141)",
        ),
        parts: &[
            p!("quartz", 30, 2650, spectra::quartz),
            p!("alkali feldspar", 40, 2560, spectra::alkali_feldspar),
            p!("plagioclase", 25, 2680, spectra::plagioclase),
            p!("biotite", 5, 3050, spectra::biotite),
        ],
    },
    Row {
        name: "basalt",
        optical: OpticalClass::Opaque,
        mix: Some(
            "basalt: plagioclase 45/100, pyroxene 40/100, olivine 15/100 — the IUGS basalt field's \
             common mineralogy, this project's stated choice; declared linear mix (C-141)",
        ),
        parts: &[
            p!("plagioclase", 45, 2680, spectra::plagioclase),
            p!("pyroxene", 40, 3300, spectra::pyroxene),
            p!("olivine", 15, 3300, spectra::olivine),
        ],
    },
    Row {
        name: "andesite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "andesite: plagioclase 60/100 with hornblende 25/100 and pyroxene 15/100 — the IUGS \
             andesite field's common mineralogy, this project's stated choice; declared linear \
             mix (C-141)",
        ),
        parts: &[
            p!("plagioclase", 60, 2680, spectra::plagioclase),
            p!("hornblende", 25, 3200, spectra::hornblende),
            p!("pyroxene", 15, 3300, spectra::pyroxene),
        ],
    },
    Row {
        name: "dacite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "dacite: plagioclase 55/100, quartz 25/100 and hornblende 20/100 — the IUGS dacite \
             field's common mineralogy, this project's stated choice; declared linear mix (C-141)",
        ),
        parts: &[
            p!("plagioclase", 55, 2680, spectra::plagioclase),
            p!("quartz", 25, 2650, spectra::quartz),
            p!("hornblende", 20, 3200, spectra::hornblende),
        ],
    },
    // ---- the sedimentary rocks -----------------------------------------
    Row {
        name: "shale",
        optical: OpticalClass::Opaque,
        mix: Some(
            "shale: clay minerals with silt — illite 40/100, kaolinite 25/100, quartz 35/100, \
             this project's stated choice; declared linear mix (C-141)",
        ),
        parts: &[
            p!("illite", 40, 2750, spectra::illite),
            p!("kaolinite", 25, 2600, spectra::kaolinite),
            p!("quartz", 35, 2650, spectra::quartz),
        ],
    },
    Row {
        name: "claystone",
        optical: OpticalClass::Opaque,
        mix: Some(
            "claystone: clay minerals with less silt than shale — kaolinite 50/100, illite \
             35/100, quartz 15/100, this project's stated choice; declared linear mix (C-141)",
        ),
        parts: &[
            p!("kaolinite", 50, 2600, spectra::kaolinite),
            p!("illite", 35, 2750, spectra::illite),
            p!("quartz", 15, 2650, spectra::quartz),
        ],
    },
    Row {
        name: "limestone",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("limestone", 100, 2700, spectra::limestone_cu02)],
    },
    Row {
        name: "dolomite",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("dolomite", 100, 2850, spectra::dolomite)],
    },
    Row {
        name: "conglomerate",
        optical: OpticalClass::Opaque,
        mix: Some(
            "conglomerate: clasts and cement — quartz 45/100, chert 30/100 and calcite 25/100 as \
             the cementing carbonate, this project's stated choice; declared linear mix (C-141)",
        ),
        parts: &[
            p!("quartz", 45, 2650, spectra::quartz),
            p!("chert", 30, 2650, spectra::chert),
            p!("calcite", 25, 2710, spectra::calcite),
        ],
    },
    Row {
        name: "chert",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("chert", 100, 2650, spectra::chert)],
    },
    Row {
        name: "chalk",
        optical: OpticalClass::Opaque,
        mix: Some(
            "chalk: a porous calcite rock — calcite 100/100, its own measured sample, whose \
             density this row takes as the rock's (2300 kg/m³); the mineral's own solid density \
             is a different quantity and is not averaged into it",
        ),
        parts: &[p!("calcite", 100, 2300, spectra::calcite)],
    },
    Row {
        name: "tuff",
        optical: OpticalClass::Opaque,
        mix: None,
        parts: &[p!("volcanic tuff", 100, 2300, spectra::volcanic_tuff)],
    },
    // ---- the metamorphic rocks -----------------------------------------
    Row {
        name: "quartzite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "quartzite: metamorphosed quartz sandstone — quartz 100/100; the declaration's \
             2700 kg/m³ is the rock's figure and this blend is the mineral's, which is a \
             different quantity",
        ),
        parts: &[p!("quartz", 100, 2700, spectra::quartz)],
    },
    Row {
        name: "slate",
        optical: OpticalClass::Opaque,
        mix: Some(
            "slate: a low-grade pelite — quartz 40/100, muscovite 30/100, chlorite 30/100, this \
             project's stated choice; declared linear mix (C-141)",
        ),
        parts: &[
            p!("quartz", 40, 2650, spectra::quartz),
            p!("muscovite", 30, 2800, spectra::muscovite),
            p!("chlorite", 30, 2900, spectra::chlorite),
        ],
    },
    Row {
        name: "phyllite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "phyllite: slate's next grade — quartz 40/100, muscovite 35/100, chlorite 25/100, \
             this project's stated choice; declared linear mix (C-141)",
        ),
        parts: &[
            p!("quartz", 40, 2650, spectra::quartz),
            p!("muscovite", 35, 2800, spectra::muscovite),
            p!("chlorite", 25, 2900, spectra::chlorite),
        ],
    },
    Row {
        name: "schist",
        optical: OpticalClass::Opaque,
        mix: Some(
            "schist: quartz 35/100, muscovite 25/100, biotite 25/100 and garnet 15/100 — the \
             garnet-mica schist assemblage, this project's stated choice; declared linear mix \
             (C-141)",
        ),
        parts: &[
            p!("quartz", 35, 2650, spectra::quartz),
            p!("muscovite", 25, 2800, spectra::muscovite),
            p!("biotite", 25, 3050, spectra::biotite),
            p!("garnet", 15, 4200, spectra::garnet),
        ],
    },
    Row {
        name: "gneiss",
        optical: OpticalClass::Opaque,
        mix: Some(
            "gneiss: quartz 25/100, alkali feldspar 30/100, plagioclase 30/100 and biotite \
             15/100 — a granitic gneiss, this project's stated choice; declared linear mix \
             (C-141)",
        ),
        parts: &[
            p!("quartz", 25, 2650, spectra::quartz),
            p!("alkali feldspar", 30, 2560, spectra::alkali_feldspar),
            p!("plagioclase", 30, 2680, spectra::plagioclase),
            p!("biotite", 15, 3050, spectra::biotite),
        ],
    },
    Row {
        name: "marble",
        optical: OpticalClass::Opaque,
        mix: Some(
            "marble: metamorphosed limestone — calcite 85/100 and dolomite 15/100, this project's \
             stated choice; declared linear mix (C-141)",
        ),
        parts: &[
            p!("calcite", 85, 2710, spectra::calcite),
            p!("dolomite", 15, 2850, spectra::dolomite),
        ],
    },
    // ---- the ores, as minerals -----------------------------------------
    Row { name: "hematite", optical: OpticalClass::Opaque, mix: None, parts: &[p!("hematite", 100, 5200, spectra::hematite)] },
    Row { name: "magnetite", optical: OpticalClass::Opaque, mix: None, parts: &[p!("magnetite", 100, 5170, spectra::magnetite)] },
    Row { name: "limonite", optical: OpticalClass::Opaque, mix: None, parts: &[p!("limonite", 100, 3500, spectra::limonite)] },
    Row { name: "malachite", optical: OpticalClass::Opaque, mix: None, parts: &[p!("malachite", 100, 4000, spectra::malachite)] },
    Row { name: "cassiterite", optical: OpticalClass::Opaque, mix: None, parts: &[p!("cassiterite", 100, 6900, spectra::cassiterite)] },
    Row {
        name: "garnierite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "garnierite: a hydrated nickel-magnesium silicate — the serpentine sample stands for \
             its silicate host, flagged; the nickel that colours it has no measured spectrum in \
             v7 and is not invented here",
        ),
        parts: &[p!("serpentine", 100, 2500, spectra::serpentine)],
    },
    Row { name: "sphalerite", optical: OpticalClass::Opaque, mix: None, parts: &[p!("sphalerite", 100, 4000, spectra::sphalerite)] },
    Row { name: "gypsum", optical: OpticalClass::Opaque, mix: None, parts: &[p!("gypsum", 100, 2350, spectra::gypsum)] },
    Row { name: "cinnabar", optical: OpticalClass::Opaque, mix: None, parts: &[p!("cinnabar", 100, 8100, spectra::cinnabar)] },
    Row {
        name: "borax",
        optical: OpticalClass::Opaque,
        mix: Some(
            "borax: v7 holds tincalconite, borax's own dehydration product and the same borate \
             series — the sample stands as the stand-in, flagged",
        ),
        parts: &[p!("tincalconite", 100, 1750, spectra::tincalconite)],
    },
    Row {
        name: "graphite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "graphite: native carbon — v7's only carbon sample is amorphous soot, flagged; \
             graphite's crystalline sheen is not in it, and a second source is owed for it",
        ),
        parts: &[p!("carbon", 100, 2200, spectra::carbon)],
    },
    Row { name: "saltpeter", optical: OpticalClass::Opaque, mix: None, parts: &[p!("niter", 100, 2100, spectra::niter)] },
    Row { name: "sulfur", optical: OpticalClass::Opaque, mix: None, parts: &[p!("sulfur", 100, 2070, spectra::sulfur)] },
    Row {
        name: "sylvite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "sylvite: the potassium analogue of halite and isostructural with it — the halite \
             sample stands for the colourless alkali halide, flagged",
        ),
        parts: &[p!("halite", 100, 1990, spectra::halite)],
    },
    // ---- the gems -------------------------------------------------------
    Row {
        name: "amethyst",
        optical: OpticalClass::Opaque,
        mix: Some(
            "amethyst: violet quartz — the quartz species sample, flagged; the iron colour centre \
             is not a separate measured mineral",
        ),
        parts: &[p!("quartz", 100, 2650, spectra::quartz)],
    },
    Row {
        name: "emerald",
        optical: OpticalClass::Opaque,
        mix: Some(
            "emerald: the green gem variety of beryl — the beryl species sample, flagged; the \
             chromium colour centre is not measured in it",
        ),
        parts: &[p!("beryl", 100, 2750, spectra::beryl)],
    },
    Row {
        name: "lapis_lazuli",
        optical: OpticalClass::Opaque,
        mix: Some(
            "lapis lazuli: a rock and not a mineral — lazurite 50/100, calcite 30/100 and pyrite \
             20/100, its own declared mixture, this project's stated choice; declared linear mix \
             (C-141)",
        ),
        parts: &[
            p!("lazurite", 50, 2400, spectra::lazurite),
            p!("calcite", 30, 2710, spectra::calcite),
            p!("pyrite", 20, 5100, spectra::pyrite),
        ],
    },
    Row { name: "opal", optical: OpticalClass::Opaque, mix: None, parts: &[p!("opal", 100, 2100, spectra::opal)] },
    Row { name: "pyrite", optical: OpticalClass::Opaque, mix: None, parts: &[p!("pyrite", 100, 5100, spectra::pyrite)] },
    Row {
        name: "ruby",
        optical: OpticalClass::Opaque,
        mix: Some(
            "ruby: the red gem variety of corundum — the corundum species sample, flagged; the \
             chromium colour centre is not measured in it",
        ),
        parts: &[p!("corundum", 100, 4000, spectra::corundum)],
    },
    Row {
        name: "sapphire",
        optical: OpticalClass::Opaque,
        mix: Some(
            "sapphire: the corundum varieties that are not red — the corundum species sample, \
             flagged; the iron and titanium colour centres are not measured in it",
        ),
        parts: &[p!("corundum", 100, 4000, spectra::corundum)],
    },
    Row { name: "topaz", optical: OpticalClass::Opaque, mix: None, parts: &[p!("topaz", 100, 3500, spectra::topaz)] },
    Row {
        name: "lignite",
        optical: OpticalClass::Opaque,
        mix: Some(
            "lignite: brown coal — the kerogen sample stands for its organic matter, flagged, \
             exactly as the `coal` row does and for the same reason",
        ),
        parts: &[p!("kerogen", 100, 1250, spectra::kerogen)],
    },
    Row { name: "halite", optical: OpticalClass::Opaque, mix: None, parts: &[p!("halite", 100, 2170, spectra::halite)] },
];

/// The declared substances whose appearance is a **named absence** rather than a
/// reflectance.
///
/// Each entry is `(substance, why a reflectance is the wrong thing to ask for)`.
/// Four are minerals v7 never measured and that no composition here can carry
/// honestly; `flue_gas` is a participating medium with no surface at all. The
/// census prints them and refuses if one of them *gains* a reflectance, so an
/// entry cannot outlive its reason.
pub const NO_REFLECTANCE: &[(&str, &str)] = &[
    (
        "bismuthinite",
        "no bismuthinite spectrum in USGS v7, and no composition of measured constituents stands \
         for a bismuth sulfide; a second spectral source is owed",
    ),
    (
        "tetrahedrite",
        "no tetrahedrite spectrum in USGS v7, and no composition of measured constituents stands \
         for a copper-antimony sulfosalt; a second spectral source is owed",
    ),
    (
        "cryolite",
        "no cryolite spectrum in USGS v7; the colourless fluoride's appearance is its boundary \
         and transmission, not a surface albedo",
    ),
    (
        "diamond",
        "no diamond spectrum in USGS v7; a transparent gem's appearance is its boundary and \
         dispersion, which the transmission model owes and no diffuse albedo can carry",
    ),
    (
        "flue_gas",
        "a participating medium: it has no surface, so a reflectance is the wrong quantity — the \
         Scattering class and the atmosphere path are its own account",
    ),
];

/// The scale at which a rock's identity holds, in metres — the same correlation length
/// the bed model uses ([`crate::geology::CORRELATION_LENGTH_M`]), so a rock's region and
/// the bed it sits in agree about where "here" is.
pub const ROCK_REGION_M: f64 = geology::CORRELATION_LENGTH_M;

/// Which declared substances each rock class is made of.
///
/// A class is not a colour. `IGNEOUS` is one material, so basalt, granite and gabbro —
/// three declared rows with three measured spectra — are indistinguishable in the world
/// until the class names its own set and the region picks one. `Regolith` and `Bedrock`
/// are absent **on purpose**: neither is a declared substance (loose cover and basement
/// are the world's own two rows), so they keep them.
pub const ROCK_KINDS: &[(Lithology, &[&str])] = &[
    (
        Lithology::Igneous,
        &[
            "granite", "diorite", "gabbro", "rhyolite", "andesite", "dacite", "basalt", "tuff",
        ],
    ),
    (
        Lithology::Sedimentary,
        &["limestone", "shale", "claystone", "conglomerate", "chert", "chalk", "dolomite"],
    ),
    (
        Lithology::Metamorphic,
        &["slate", "phyllite", "schist", "gneiss", "marble", "quartzite"],
    ),
];

/// The declared substance an ore kind is a deposit of, where the declaration carries one.
///
/// **`Gold` has no row** — there is no gold substance in `substances.tsv` — so a gold vein
/// draws its **host rock** rather than a fabricated metal. The drop is named here rather
/// than hidden, and it closes the day the declaration gains the row.
pub const ORE_SUBSTANCE: &[(OreKind, &str)] = &[
    (OreKind::Coal, "coal"),
    (OreKind::Iron, "iron_ore"),
    (OreKind::Copper, "malachite"),
    (OreKind::Tin, "cassiterite"),
];

/// The material id of a declared substance, by name.
pub fn id_of(name: &str) -> Option<MaterialId> {
    ROWS.iter()
        .position(|row| row.name == name)
        .map(|index| MaterialId(FIRST_MATERIAL_ID + index as u16))
}

/// The declared name of a corpus material id, if it is one.
pub fn name_of(id: MaterialId) -> Option<&'static str> {
    let index = (id.0 as usize).checked_sub(FIRST_MATERIAL_ID as usize)?;
    ROWS.get(index).map(|row| row.name)
}

/// The **declared rock** at a point: the region's choice inside the lithology's own set.
///
/// This is the rule that turns a class into a substance. It is a **field over position** —
/// a stationary seed and a region cell — because a bed's rock is a property of the region
/// and not of the tick, exactly the reason [`geology`] mixes a region into the bed's
/// class. `Regolith` and `Bedrock` fall back to the world's own rows; any other class must
/// resolve, and a class whose set were empty refuses rather than return a rock it lacks.
pub fn rock_of(spec: &WorldSpec, lithology: Lithology, x: f64, z: f64) -> Refused<MaterialId> {
    let Some((_, kinds)) = ROCK_KINDS.iter().find(|(class, _)| *class == lithology) else {
        return Ok(crate::column::materials::for_lithology(lithology));
    };
    let seed = spec.stationary_stage_seed("corpus");
    let region_x = (x / ROCK_REGION_M).floor() as i64 as u64;
    let region_z = (z / ROCK_REGION_M).floor() as i64 as u64;
    let mut h = mix(seed);
    h = mix(h ^ region_x.wrapping_mul(0xD6E8_FEB8_6659_FD93));
    h = mix(h ^ region_z.wrapping_mul(0xA076_1D64_78BD_642F));
    let name = kinds[(h % kinds.len() as u64) as usize];
    id_of(name).ok_or_else(|| {
        Refusal::owned(
            RefusalKind::InvalidFieldSpec,
            format!("the rock `{name}` is named as a kind but is not a corpus row"),
        )
    })
}

/// The **declared ore** an ore kind draws, where one exists (see [`ORE_SUBSTANCE`]).
pub fn ore_of(kind: OreKind) -> Option<MaterialId> {
    ORE_SUBSTANCE
        .iter()
        .find(|(candidate, _)| *candidate == kind)
        .and_then(|(_, name)| id_of(name))
}

/// How rich a vein must be before a voxel is **drawn** as its ore rather than as its
/// host rock.
///
/// **The deposit model and the drawn world are two different questions, and this is
/// where they part.** [`geology::ore_at`] answers "is a deposit here" — S-22's survey
/// predicate, whose dials are ruled and untouched — and the census measured what happens
/// if a deposit is drawn at that answer: **603,431 of 643,883 rock voxels in a 1 km slab
/// were `coal` — 90 % of the entire subsurface**, so the world was a coal mine with a
/// crust. `ore_grade` already declares the vein's own richness (richer toward its core);
/// drawing only the voxels above this cut-off draws the **cores**, and the rock keeps the
/// rest of its own space. The number is this project's stated choice, not a measured
/// value, and it is the dial to move if the deposits are too sparse or too fat.
pub const ORE_DRAW_CUTOFF: f64 = 0.9;

/// The material a **rock voxel** holds: an ore where the vein field's core puts one, else
/// the region's declared rock.
///
/// Both material paths — [`crate::column::ColumnProfile::material_at`] and
/// [`crate::chunk_fields::material_of`] — call this one function, so they cannot draw two
/// different rocks at the same voxel. The ore field is read at the voxel's own depth with
/// the world's dials, exactly as [`crate::column::ore_at`] exposes them; no surface rule
/// and no exposure rule is consulted (S-22).
pub fn rock_voxel(
    spec: &WorldSpec,
    lithology: Lithology,
    x: f64,
    y: f64,
    z: f64,
    depth_m: f64,
) -> Refused<MaterialId> {
    let dials = OreDials::default();
    if let Some(kind) = geology::ore_at(spec, &dials, x, y, z, depth_m) {
        if geology::ore_grade(spec, &dials, x, y, z, depth_m) >= ORE_DRAW_CUTOFF {
            if let Some(material) = ore_of(kind) {
                return Ok(material);
            }
        }
    }
    rock_of(spec, lithology, x, z)
}

fn percent_ratio(percent: u64) -> Refused<Ratio> {
    Ratio::new(percent, 100)
}

/// Admit every corpus row into `into`, in declaration order.
///
/// Components are shared by name, so two rows that both contain quartz share one
/// component id — a constituent is one substance however many rows contain it.
pub fn admit(into: &mut MaterialRegistry) -> Refused<()> {
    let mut component_ids: BTreeMap<&'static str, ComponentId> = BTreeMap::new();
    let mut next_component = FIRST_COMPONENT_ID;

    for (index, row) in ROWS.iter().enumerate() {
        let mut components: Vec<Component> = Vec::with_capacity(row.parts.len());
        let mut parts: Vec<(Ratio, SourcedReflectance)> = Vec::with_capacity(row.parts.len());
        for part in row.parts {
            let id = match component_ids.get(part.name) {
                Some(id) => *id,
                None => {
                    let id = ComponentId(next_component);
                    next_component += 1;
                    component_ids.insert(part.name, id);
                    id
                }
            };
            into.admit_component(id, part.name);
            let ratio = percent_ratio(part.percent)?;
            let reflectance = (part.sample)()?;
            parts.push((ratio, reflectance));
            components.push(Component {
                id,
                ratio,
                density_kg_m3: Ratio::from_u64(part.density_kg_m3),
                reflectance: Some(reflectance),
            });
        }

        let reflectance = match row.mix {
            // One measured substance: the row carries the sample's provenance whole
            // rather than restating it, so the two cannot drift apart.
            None => parts[0].1,
            Some(statement) => SourcedReflectance {
                value: blend_reflectance(&parts)?,
                source: statement,
            },
        };

        let material = Material {
            id: MaterialId(FIRST_MATERIAL_ID + index as u16),
            name: row.name.to_string(),
            components,
            optical_class: row.optical,
            reflectance: Some(reflectance),
            emission: None,
        };
        if let Err(set) = into.admit_material(material) {
            return Err(Refusal::owned(
                RefusalKind::RatiosDoNotSumToOne,
                format!("the corpus row `{}` is not admissible: {set}", row.name),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_sums_to_exactly_one() {
        let mut bad: Vec<&str> = Vec::new();
        for row in ROWS {
            let mut sum = 0u64;
            for part in row.parts {
                sum += part.percent;
            }
            if sum != 100 {
                bad.push(row.name);
            }
        }
        assert!(bad.is_empty(), "rows whose shares do not sum to 100: {bad:?}");
    }

    #[test]
    fn no_row_is_declared_twice_and_none_is_named_in_the_absence_list() {
        let mut names: Vec<&str> = ROWS.iter().map(|row| row.name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "a substance is declared twice in ROWS");
        for (absent, _) in NO_REFLECTANCE {
            assert!(
                !ROWS.iter().any(|row| row.name == *absent),
                "{absent} is both a row and a named absence"
            );
        }
    }

    #[test]
    fn the_absence_list_states_a_reason_for_every_entry() {
        for (name, reason) in NO_REFLECTANCE {
            assert!(!reason.is_empty(), "{name} is excused with no reason");
        }
    }

    #[test]
    fn the_corpus_admits_and_every_row_carries_a_reflectance() {
        let mut registry = MaterialRegistry::new();
        admit(&mut registry).expect("the corpus admits");
        assert_eq!(registry.admitted_material_count(), ROWS.len());
        for row in ROWS {
            let id = MaterialId(FIRST_MATERIAL_ID + ROWS.iter().position(|r| r.name == row.name).unwrap() as u16);
            let material = registry.material(id).expect("an admitted row");
            assert!(material.reflectance.is_some(), "{} has no reflectance", row.name);
            assert_eq!(material.name, row.name);
        }
    }

    #[test]
    fn a_rows_reflectance_is_the_blend_of_its_own_parts() {
        // **The invariant C-141 asks for, held by measurement.** For every row the
        // stated value is recomputed from the components, so a composed row cannot
        // drift from its composition and a direct row cannot drift from its sample.
        let mut registry = MaterialRegistry::new();
        admit(&mut registry).expect("the corpus admits");
        for index in 0..ROWS.len() {
            let material = registry
                .material(MaterialId(FIRST_MATERIAL_ID + index as u16))
                .expect("an admitted row");
            let parts: Vec<(Ratio, SourcedReflectance)> = material
                .components
                .iter()
                .map(|c| (c.ratio, c.reflectance.expect("a component with a reflectance")))
                .collect();
            let recomputed = blend_reflectance(&parts).expect("a blend");
            let stated = material.reflectance.expect("a stated reflectance");
            assert_eq!(
                recomputed, stated.value,
                "{} states a value its composition does not produce",
                material.name
            );
        }
    }

    #[test]
    fn two_rows_that_share_a_constituent_share_its_component_id() {
        // A constituent is one substance however many rows contain it, so quartz
        // has one id across granite, basalt and the rest — otherwise the same
        // mineral would arrive at the appearance layer as several different things.
        let mut registry = MaterialRegistry::new();
        admit(&mut registry).expect("the corpus admits");
        let row_id = |name: &str| {
            MaterialId(FIRST_MATERIAL_ID + ROWS.iter().position(|r| r.name == name).unwrap() as u16)
        };
        let constituent_id = |material: &Material, want: &str| {
            material
                .components
                .iter()
                .map(|c| c.id)
                .find(|id| registry.component_name(*id) == Some(want))
                .unwrap_or_else(|| {
                    let seen: Vec<(&str, u16)> = material
                        .components
                        .iter()
                        .map(|c| {
                            (registry.component_name(c.id).unwrap_or("<unnamed>"), c.id.0)
                        })
                        .collect();
                    panic!("{want} is not one of {}'s constituents: {seen:?}", material.name)
                })
        };
        let granite = registry.material(row_id("granite")).expect("granite");
        let rhyolite = registry.material(row_id("rhyolite")).expect("rhyolite");
        assert_eq!(
            constituent_id(granite, "quartz"),
            constituent_id(rhyolite, "quartz"),
            "two rows that both contain quartz disagree about which substance it is"
        );
        assert_eq!(
            constituent_id(granite, "plagioclase"),
            constituent_id(rhyolite, "plagioclase")
        );
        assert_eq!(
            constituent_id(granite, "biotite"),
            constituent_id(rhyolite, "biotite"),
            "the two rows hold biotite in different shares; the substance is still one"
        );
    }

    #[test]
    fn every_rock_and_ore_kind_names_a_declared_row() {
        // The mapping is only honest if every name it holds is a substance the census
        // admits: a kind that named a row that does not exist would refuse at runtime, and
        // a kind that named nothing would be a silent hole in the world's rock.
        for (class, kinds) in ROCK_KINDS {
            assert!(!kinds.is_empty(), "{class:?} names no rock");
            for name in *kinds {
                assert!(
                    id_of(name).is_some(),
                    "the {class:?} kind `{name}` is not a declared substance"
                );
            }
        }
        for (kind, name) in ORE_SUBSTANCE {
            assert!(
                id_of(name).is_some(),
                "the ore {kind:?} names `{name}`, which is not a declared substance"
            );
            assert!(ore_of(*kind).is_some(), "{kind:?} draws no material");
        }
    }

    #[test]
    fn a_class_becomes_several_rocks_across_regions_and_a_region_is_stable() {
        let spec = WorldSpec::new(2468, crate::spec::Config::default());
        let mut seen = std::collections::BTreeSet::new();
        for step in 0..80u64 {
            let x = step as f64 * ROCK_REGION_M * 0.5;
            let first = rock_of(&spec, Lithology::Igneous, x, 0.0).expect("a rock");
            let again = rock_of(&spec, Lithology::Igneous, x, 0.0).expect("a rock");
            assert_eq!(first, again, "the rock moved between two reads at x={x}");
            seen.insert(first);
        }
        assert!(
            seen.len() > 1,
            "a whole world of igneous rock is one substance: {seen:?}"
        );
        // `Regolith` and `Bedrock` are not declared substances, so they keep the world's
        // own rows rather than borrowing a rock that is not them.
        assert_eq!(
            rock_of(&spec, Lithology::Regolith, 0.0, 0.0).expect("a rock"),
            crate::column::materials::REGOLITH
        );
        assert_eq!(
            rock_of(&spec, Lithology::Bedrock, 0.0, 0.0).expect("a rock"),
            crate::column::materials::BEDROCK
        );
    }

    #[test]
    fn gold_is_named_absent_rather_than_fabricated() {
        // The declaration carries no gold, so a gold vein draws its host rock. The drop is
        // asserted so it cannot be quietly papered over by a later edit.
        assert_eq!(ore_of(OreKind::Gold), None, "gold has no declared substance");
    }
}
