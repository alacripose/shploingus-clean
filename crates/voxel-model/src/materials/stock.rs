//! **The tree from the floor: a stock, the offers it opens, and the act that spends it.**
//!
//! PATCH-057, PATCH-058 and PATCH-060 each end by naming the same absence: *"No inventory, no recipe
//! lookup, no craft action"* — the declaration is walkable by the gate from an imagined starting
//! set, but nothing in the game could ask *what can I do with what I hold* and get an answer.
//!
//! Three pieces close that, and they are deliberately separate from the tree walk:
//!
//! * [`Stock`] is the inventory: grams of declared substances, and the structures, tools and skills
//!   the player has. It owns no declarations, so it cannot disagree about a row.
//! * [`Offer`] is one option the tree can present: a process, its rung, and exactly what is short.
//!   **Every process is an offer**; readiness is a field, not a filter, so a UI can show a locked
//!   branch instead of hiding it.
//! * [`Table::craft`] is the act: it checks the whole row against the stock before it mutates a
//!   single gram, then spends the inputs and adds the outputs.
//!
//! **States are not interchangeable.** `offers` and `craft` resolve each input through its own
//! declared row, so holding `copper` does not satisfy `molten_copper`. That is the point of
//! PATCH-058's state axis, and it is proven in `a_melt_is_not_a_stand_in_for_the_solid_it_came_from`.
//!
//! Quantities in a stock are **grams**, whatever unit the row is declared in: a process converts
//! its own inputs and outputs with [`crate::materials::schema::Substance::to_grams`], so a 2 L
//! solvent charge is held as its measured 1 580 g and the row's own density is the only converter. The API never invents a
//! density to make a number work.

use std::collections::{BTreeMap, BTreeSet};

use crate::material::Ratio;
use crate::refusal::{Refusal, RefusalKind, Refused};

use super::schema::{Process, Qty, Table, Tier, VocabKind};

/// What one process is short of, in grams of a declared substance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Missing {
    /// The substance the row consumes.
    pub substance: String,
    /// How many grams the stock would have to gain for the row to run.
    pub grams: Ratio,
}

/// One process the tree can offer, with everything that stands between it and the act.
///
/// **Every process is returned**, ready or not: a branch a player cannot take yet is still a
/// choice a game may show them, and filtering it out here would put the decision in the wrong
/// place.
#[derive(Clone, Debug)]
pub struct Offer<'a> {
    /// The row itself.
    pub process: &'a Process,
    /// The rung the row belongs to.
    pub tier: &'a Tier,
    /// Grams short of each declared input, sorted by substance.
    pub missing: Vec<Missing>,
    /// The structures, tools and skills the stock does not hold.
    pub missing_words: Vec<(VocabKind, String)>,
}

impl Offer<'_> {
    /// Whether the stock satisfies every input and every requirement.
    pub fn is_ready(&self) -> bool {
        self.missing.is_empty() && self.missing_words.is_empty()
    }

    /// One line naming what is short, for a console or a test's failure message.
    pub fn shortfall(&self) -> String {
        let mut parts: Vec<String> = self
            .missing
            .iter()
            .map(|missing| format!("{} g {}", missing.grams, missing.substance))
            .collect();
        parts.extend(
            self.missing_words
                .iter()
                .map(|(kind, name)| format!("{} {name}", kind.name())),
        );
        parts.join(", ")
    }
}

/// The inventory: grams held of each substance, and the words the player has earned.
///
/// A stock is **not** validated against a table on construction: it is a ledger of what a player
/// holds, and a name no row consumes is inert rather than an error. The table is the authority on
/// whether an act is legal, and [`Table::craft`] is where that check lives.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stock {
    grams: BTreeMap<String, Ratio>,
    words: BTreeMap<VocabKind, BTreeSet<String>>,
}

impl Stock {
    /// An empty inventory.
    pub fn new() -> Self {
        Stock::default()
    }

    /// Add `grams` of a substance. Exact; no declaration is consulted.
    pub fn hold(&mut self, substance: impl Into<String>, grams: Ratio) {
        *self.grams.entry(substance.into()).or_insert(Ratio::ZERO) += grams;
    }

    /// Hold a structure, tool or skill.
    pub fn learn(&mut self, kind: VocabKind, name: impl Into<String>) {
        self.words.entry(kind).or_default().insert(name.into());
    }

    /// Whether a structure, tool or skill is held.
    pub fn holds_word(&self, kind: VocabKind, name: &str) -> bool {
        self.words.get(&kind).is_some_and(|names| names.contains(name))
    }

    /// The grams held of one substance; zero when it was never stocked.
    pub fn grams(&self, substance: &str) -> Ratio {
        self.grams.get(substance).copied().unwrap_or(Ratio::ZERO)
    }

    /// Everything the stock holds, for a display that wants the whole ledger.
    pub fn ledger(&self) -> &BTreeMap<String, Ratio> {
        &self.grams
    }

    fn add(&mut self, substance: &str, grams: Ratio) {
        *self.grams.entry(substance.to_string()).or_insert(Ratio::ZERO) += grams;
    }

    /// Remove an amount that the caller has already proven is held.
    fn spend(&mut self, substance: &str, grams: Ratio) -> Refused<()> {
        let held = self.grams(substance);
        let left = held.checked_sub(grams)?;
        if left.is_zero() {
            self.grams.remove(substance);
        } else {
            self.grams.insert(substance.to_string(), left);
        }
        Ok(())
    }
}

/// Why a process cannot run, in the order a player would fix it.
fn shortfalls(table: &Table, process: &Process, stock: &Stock) -> Refused<(Vec<Missing>, Vec<(VocabKind, String)>)> {
    let mut missing = Vec::new();
    for qty in &process.inputs {
        let need = table.substance(&qty.substance)?.to_grams(qty.amount)?;
        let held = stock.grams(&qty.substance);
        // `checked_sub` is the comparison: a successful subtraction is `held <= need`, and a
        // successful non-zero subtraction is a real shortfall. A numeric `<` is unavailable because
        // `Ratio`'s `Ord` is field order, not magnitude.
        match need.checked_sub(held) {
            Ok(short) if !short.is_zero() => missing.push(Missing {
                substance: qty.substance.clone(),
                grams: short,
            }),
            _ => {}
        }
    }
    missing.sort_by(|a, b| a.substance.cmp(&b.substance));

    let mut missing_words = Vec::new();
    for (kind, name) in [
        (VocabKind::Structure, process.requires.structure.as_deref()),
        (VocabKind::Tool, process.requires.tool.as_deref()),
        (VocabKind::Skill, process.requires.skill.as_deref()),
    ] {
        if let Some(name) = name {
            if !stock.holds_word(kind, name) {
                missing_words.push((kind, name.to_string()));
            }
        }
    }
    Ok((missing, missing_words))
}

impl Table {
    /// **What the stock can do now, and what every other row is waiting for.**
    ///
    /// Returns one [`Offer`] per declared process, sorted by rung and then by name, with ready
    /// options first inside a rung. A row whose inputs cannot be turned into grams (a volume with
    /// no density, say) refuses rather than being silently left out: a missing measurement is not
    /// an option.
    pub fn offers<'a>(&'a self, stock: &Stock) -> Refused<Vec<Offer<'a>>> {
        let mut offers = Vec::with_capacity(self.processes.len());
        for process in &self.processes {
            let tier = self
                .tiers
                .iter()
                .find(|tier| tier.name == process.tier)
                .ok_or_else(|| {
                    Refusal::owned(
                        RefusalKind::UndeclaredName,
                        format!(
                            "{}: tier {:?} is not declared, so the row has no rung to offer",
                            process.name, process.tier
                        ),
                    )
                })?;
            let (missing, missing_words) = shortfalls(self, process, stock)?;
            offers.push(Offer {
                process,
                tier,
                missing,
                missing_words,
            });
        }
        offers.sort_by(|a, b| {
            a.tier
                .ordinal
                .cmp(&b.tier.ordinal)
                .then_with(|| b.is_ready().cmp(&a.is_ready()))
                .then_with(|| a.process.name.cmp(&b.process.name))
        });
        Ok(offers)
    }

    /// The process a name refers to, or a refusal naming the name.
    pub fn process(&self, name: &str) -> Refused<&Process> {
        self.processes
            .iter()
            .find(|process| process.name == name)
            .ok_or_else(|| {
                Refusal::owned(
                    RefusalKind::UndeclaredName,
                    format!("{name:?} is not a declared process"),
                )
            })
    }

    /// **Perform one row against a stock.**
    ///
    /// The whole row is checked before anything is spent: every input must be held in grams and
    /// every required structure, tool and skill must be held. On success the inputs leave the
    /// stock, the outputs join it, and the outputs are returned as declared. A refusal leaves the
    /// stock **exactly** as it was, which is what lets a caller offer a choice, be refused, and
    /// still offer the same choice again.
    pub fn craft(&self, process: &str, stock: &mut Stock) -> Refused<Vec<Qty>> {
        let row = self.process(process)?;
        let (missing, missing_words) = shortfalls(self, row, stock)?;
        if !missing.is_empty() || !missing_words.is_empty() {
            let mut detail = format!("{process}: cannot run");
            for short in &missing {
                detail.push_str(&format!("; {} g of {} short", short.grams, short.substance));
            }
            for (kind, name) in &missing_words {
                detail.push_str(&format!("; no {} {name}", kind.name()));
            }
            return Err(Refusal::owned(RefusalKind::OutOfDeclaredRange, detail));
        }

        // Every check passed; the mutation below cannot half-run.
        for qty in &row.inputs {
            let need = self.substance(&qty.substance)?.to_grams(qty.amount)?;
            stock.spend(&qty.substance, need)?;
        }
        for qty in &row.outputs {
            let grams = self.substance(&qty.substance)?.to_grams(qty.amount)?;
            stock.add(&qty.substance, grams);
        }
        Ok(row.outputs.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::generated;

    fn ratio(num: u64, den: u64) -> Ratio {
        Ratio::new(num, den).expect("a rational in a test")
    }

    #[test]
    fn an_empty_stock_offers_every_process_and_only_gathers_are_ready() {
        let table = generated::table();
        let offers = table.offers(&Stock::new()).expect("the table's rows all resolve");
        assert_eq!(
            offers.len(),
            table.processes.len(),
            "every process is an option, ready or not"
        );
        for offer in &offers {
            let bare = offer.process.inputs.is_empty() && offer.process.requires.is_empty();
            assert_eq!(
                offer.is_ready(),
                bare,
                "{}: readiness disagrees with inputs {:?} and requires {:?}",
                offer.process.name,
                offer.process.inputs,
                offer.process.requires
            );
            if !bare {
                assert!(
                    !offer.shortfall().is_empty(),
                    "{}: a blocked offer says nothing about why",
                    offer.process.name
                );
            }
        }
        assert!(
            offers.iter().any(Offer::is_ready),
            "an empty stock leaves every gather available"
        );
    }

    /// The state axis, exercised through the lookup rather than the gate: copper is not a melt.
    #[test]
    fn a_melt_is_not_a_stand_in_for_the_solid_it_came_from() {
        let table = generated::table();
        let mut stock = Stock::new();
        stock.hold("copper", Ratio::from_u64(10_000));
        stock.hold("tin", Ratio::from_u64(10_000));

        let offers = table.offers(&stock).expect("the table resolves");
        let alloy = offers
            .iter()
            .find(|offer| offer.process.name == "alloy bronze")
            .expect("alloy bronze is a declared row");
        assert!(
            alloy.missing.iter().any(|miss| miss.substance == "molten_copper"),
            "solid copper satisfied a molten input: {}",
            alloy.shortfall()
        );
        assert!(
            !alloy.is_ready(),
            "the alloy ran without its melts or its furnace"
        );

        stock.hold("molten_copper", Ratio::from_u64(100_000));
        stock.hold("molten_tin", Ratio::from_u64(100_000));
        stock.learn(VocabKind::Structure, "furnace");
        stock.learn(VocabKind::Skill, "alloying");
        let offers = table.offers(&stock).expect("the table resolves");
        let alloy = offers
            .iter()
            .find(|offer| offer.process.name == "alloy bronze")
            .expect("alloy bronze is a declared row");
        assert!(alloy.is_ready(), "still blocked: {}", alloy.shortfall());
    }

    #[test]
    fn a_missing_structure_is_offered_rather_than_hidden() {
        let table = generated::table();
        let mut stock = Stock::new();
        stock.hold("molten_copper", Ratio::from_u64(100_000));
        stock.hold("molten_tin", Ratio::from_u64(100_000));
        stock.learn(VocabKind::Skill, "alloying");

        let offers = table.offers(&stock).expect("the table resolves");
        let alloy = offers
            .iter()
            .find(|offer| offer.process.name == "alloy bronze")
            .expect("alloy bronze is a declared row");
        assert!(!alloy.is_ready());
        assert_eq!(
            alloy.missing_words,
            vec![(VocabKind::Structure, "furnace".to_string())],
            "the offer must name the structure it lacks"
        );
        assert_eq!(alloy.missing, vec![]);

        stock.learn(VocabKind::Structure, "furnace");
        let offers = table.offers(&stock).expect("the table resolves");
        assert!(
            offers
                .iter()
                .find(|offer| offer.process.name == "alloy bronze")
                .is_some_and(Offer::is_ready),
            "learning the structure did not unlock the row"
        );
    }

    #[test]
    fn craft_spends_exactly_the_row_and_refuses_short_stock_without_moving_any() {
        let table = generated::table();
        let copper = table.substance("copper").expect("copper is declared");
        let charge = copper
            .to_grams(ratio(1000, 1))
            .expect("copper is a mass");
        let melt = table
            .process("melt copper")
            .expect("melt copper is a declared row");

        let mut short = Stock::new();
        short.learn(VocabKind::Structure, "furnace");
        short.learn(VocabKind::Skill, "smelting");
        short.hold("copper", charge.checked_sub(Ratio::from_u64(1)).expect("one gram less"));
        let before = short.clone();
        let refusal = table
            .craft("melt copper", &mut short)
            .expect_err("one gram short must refuse");
        assert!(
            refusal.detail().contains("copper"),
            "the refusal must name the short input: {}",
            refusal.detail()
        );
        assert_eq!(short, before, "a refused craft moved the ledger");

        short.hold("copper", Ratio::from_u64(1));
        let outputs = table.craft("melt copper", &mut short).expect("the exact charge melts");
        assert_eq!(outputs, melt.outputs);
        assert_eq!(short.grams("copper"), Ratio::ZERO);
        let produced = table
            .substance(&outputs[0].substance)
            .expect("the output is declared")
            .to_grams(outputs[0].amount)
            .expect("the output has a mass");
        assert_eq!(short.grams(&outputs[0].substance), produced);
    }

    /// PATCH-060's solvent is charged in millilitres; the stock is grams, and only the row's own
    /// density may convert it.
    #[test]
    fn a_volume_charge_is_stocked_and_returned_in_its_own_grams() {
        let table = generated::table();
        let row = table
            .process("extract the press cake")
            .expect("the solvent extraction is a declared row");
        let mut stock = Stock::new();
        for qty in &row.inputs {
            let grams = table
                .substance(&qty.substance)
                .and_then(|substance| substance.to_grams(qty.amount))
                .expect("a declared charge has a mass");
            stock.hold(&qty.substance, grams);
        }
        stock.learn(VocabKind::Structure, "vat");
        stock.learn(VocabKind::Skill, "chemistry");

        let ethanol_in = stock.grams("ethanol");
        assert!(!ethanol_in.is_zero(), "the charge was not read as grams");
        table
            .craft(&row.name, &mut stock)
            .expect("the exact charges and words run the row");

        let recovered = row
            .outputs
            .iter()
            .find(|qty| qty.substance == "ethanol")
            .expect("the solvent is recovered on the output side");
        let recovered = table
            .substance(&recovered.substance)
            .and_then(|substance| substance.to_grams(recovered.amount))
            .expect("the recovered solvent has a mass");
        assert_eq!(
            stock.grams("ethanol"),
            recovered,
            "the stock did not end with exactly the recovered millilitres' grams"
        );
    }

    #[test]
    fn a_missing_word_refuses_and_the_stock_does_not_move() {
        let table = generated::table();
        let mut stock = Stock::new();
        // Enough of both melts, but no furnace and no alloying skill.
        stock.hold("molten_copper", Ratio::from_u64(100_000));
        stock.hold("molten_tin", Ratio::from_u64(100_000));
        let before = stock.clone();
        let refusal = table
            .craft("alloy bronze", &mut stock)
            .expect_err("a row cannot run without its structure and skill");
        assert!(
            refusal.detail().contains("furnace") && refusal.detail().contains("alloying"),
            "the refusal must name both missing words: {}",
            refusal.detail()
        );
        assert_eq!(stock, before);
    }
}
