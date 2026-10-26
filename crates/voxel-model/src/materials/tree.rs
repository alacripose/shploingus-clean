//! The technology tree's owner: reachability and coverage over the declared table.
//!
//! The gate in [`super::schema`] checks each row **on its own**: it balances, it
//! names only declared things, it states a mechanism. None of that proves the
//! rows form a tree. A table can be perfectly well-formed and still contain a
//! tier nobody can reach, a substance nothing makes, or a process whose inputs
//! never exist — every row honest, the set unusable.
//!
//! This module is the owner of that second question. It walks the table the way
//! a person would: it starts with what the world already holds (`mined`,
//! `grown`, `gathered`, `salvaged`), takes the tiers in ordinal order, and fires
//! every process in a tier whose inputs are available **at or before** that
//! tier. Within one tier it iterates to a fixpoint, because a tier is allowed to
//! build on itself (ore → crushed ore → washed concentrate, all in the copper
//! age) and a single pass would call the second step dead.
//!
//! The four facts it refuses:
//!
//! 1. **A tier no work reaches.** A rung with no fired process is unreachable,
//!    whatever its ordinal says; a forward edge (a process in an early tier
//!    consuming a later tier's product) is exactly how that happens, so it is
//!    named as the tier's fault and quoted with the missing inputs.
//! 2. **A made substance nothing produces.** `Source::Made` is a claim that a
//!    process row makes it. An edit that deletes the producer leaves the claim
//!    standing and the substance unobtainable, which is an orphan.
//! 3. **A dead process.** Every process must fire in some run of the tree. One
//!    that never fires is a row describing work that cannot happen.
//! 4. **An unknown name.** Checked here too, because this walk is the thing that
//!    would otherwise silently skip a row that names a substance the table does
//!    not hold; the schema gate also refuses it, and two refusals for one
//!    missing name is better than a tree that quietly pretends the row is fine.
//!
//! The [`Coverage`] the walk produces is the tree's report card: how many tiers,
//! substances and processes there are, and how many of each are reachable. The
//! tests in this module run it over the committed table and fail loudly if an
//! edit orphans a node.

use std::collections::BTreeSet;

use crate::refusal::{RefusalKind, RefusalSet};

use super::schema::{Process, Table};

/// What the walk found: the counts the tree can actually support.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Coverage {
    /// Tiers declared.
    pub tiers: usize,
    /// Tiers that some process fires in.
    pub tiers_reached: usize,
    /// Substances declared, of every source.
    pub substances: usize,
    /// Substances obtainable: held by the world or produced by a fired process.
    pub substances_available: usize,
    /// Substances whose row claims `made`.
    pub substances_made: usize,
    /// `made` substances a fired process actually produces.
    pub substances_made_produced: usize,
    /// Processes declared.
    pub processes: usize,
    /// Processes that fire in the walk.
    pub processes_fired: usize,
}

impl Coverage {
    /// One line per count, for the gate's report and the test's failure message.
    pub fn report(&self) -> String {
        format!(
            "tiers {}/{} reached, substances {}/{} available ({} made, {} produced), \
             processes {}/{} fired",
            self.tiers_reached,
            self.tiers,
            self.substances_available,
            self.substances,
            self.substances_made_produced,
            self.substances_made,
            self.processes_fired,
            self.processes,
        )
    }

    /// Whether every count is at its ceiling — the tree is whole.
    pub fn is_complete(&self) -> bool {
        self.tiers_reached == self.tiers
            && self.substances_available == self.substances
            && self.substances_made_produced == self.substances_made
            && self.processes_fired == self.processes
    }
}

/// The result of walking the table once: what is reachable, and what is not.
#[derive(Clone, Debug, Default)]
pub struct Tree {
    coverage: Coverage,
    defects: RefusalSet,
    available: BTreeSet<String>,
    fired: BTreeSet<String>,
    reached: BTreeSet<String>,
}

impl Tree {
    /// The counts the walk supports.
    pub fn coverage(&self) -> Coverage {
        self.coverage
    }

    /// Everything the walk refused, collected rather than stopped at.
    pub fn defects(&self) -> &RefusalSet {
        &self.defects
    }

    /// Whether the tree is whole: no defects and every count at its ceiling.
    pub fn is_complete(&self) -> bool {
        self.defects.is_empty() && self.coverage.is_complete()
    }

    /// Whether the walk can obtain this substance.
    pub fn is_available(&self, substance: &str) -> bool {
        self.available.contains(substance)
    }

    /// Whether this process fires somewhere in the walk.
    pub fn is_fired(&self, process: &str) -> bool {
        self.fired.contains(process)
    }

    /// Whether some process fires in this tier.
    pub fn is_reached(&self, tier: &str) -> bool {
        self.reached.contains(tier)
    }

    /// The report the gate prints: the counts, then every defect.
    pub fn report(&self) -> String {
        let mut report = self.coverage.report();
        for defect in self.defects.all() {
            report.push_str("\n  ");
            report.push_str(&defect.to_string());
        }
        report
    }
}

impl Table {
    /// Walk the tree: what the world holds, what the tiers produce, what is
    /// stranded. Pure over the table; no state is cached.
    pub fn tree(&self) -> Tree {
        let mut defects = RefusalSet::new();
        let mut available: BTreeSet<String> = self
            .substances
            .values()
            .filter(|substance| substance.source.is_held_by_world())
            .map(|substance| substance.name.clone())
            .collect();
        let mut fired: BTreeSet<String> = BTreeSet::new();
        let mut reached: BTreeSet<String> = BTreeSet::new();

        // Tiers in ordinal order: a tier may consume only what earlier tiers
        // (and its own earlier steps) made available. An unrecognised tier name
        // is already a schema refusal; here it is simply never walked.
        // An unknown name is refused once per row, before the walk, so the
        // readiness test below is only ever about availability.
        for process in &self.processes {
            for qty in process.inputs.iter().chain(process.outputs.iter()) {
                if !self.substances.contains_key(&qty.substance) {
                    defects.refuse(
                        RefusalKind::UndeclaredName,
                        format!(
                            "{}: {:?} is not a declared substance, so the row can never run",
                            process.name, qty.substance
                        ),
                    );
                }
            }
        }

        let mut tiers = self.tiers.clone();
        tiers.sort_by_key(|tier| tier.ordinal);

        for tier in &tiers {
            // A tier is allowed to build on itself, so iterate to a fixpoint
            // before deciding the tier is done.
            loop {
                let mut progressed = false;
                for index in 0..self.processes.len() {
                    let process = &self.processes[index];
                    if process.tier != tier.name || fired.contains(&process.name) {
                        continue;
                    }
                    let ready = process
                        .inputs
                        .iter()
                        .all(|qty| available.contains(&qty.substance));
                    if !ready {
                        continue;
                    }
                    fired.insert(process.name.clone());
                    reached.insert(tier.name.clone());
                    for qty in &process.outputs {
                        if self.substances.contains_key(&qty.substance) {
                            available.insert(qty.substance.clone());
                        }
                    }
                    progressed = true;
                }
                if !progressed {
                    break;
                }
            }

            if !reached.contains(&tier.name) {
                let stranded: Vec<String> = self
                    .processes
                    .iter()
                    .filter(|process| process.tier == tier.name && !fired.contains(&process.name))
                    .map(|process| process.name.clone())
                    .collect();
                let detail = if stranded.is_empty() {
                    format!(
                        "tier {:?} (ordinal {}) declares no work at all, so nothing can reach it",
                        tier.name, tier.ordinal
                    )
                } else {
                    let missing: BTreeSet<&str> = self
                        .processes
                        .iter()
                        .filter(|process| process.tier == tier.name)
                        .flat_map(|process| process.inputs.iter())
                        .map(|qty| qty.substance.as_str())
                        .filter(|name| !available.contains(*name))
                        .collect();
                    format!(
                        "tier {:?} (ordinal {}) is unreachable from the first: {:?} never fire \
                         because {:?} is never available at or before this tier",
                        tier.name, tier.ordinal, stranded, missing
                    )
                };
                defects.refuse(RefusalKind::UndeclaredName, detail);
            }
        }

        // Every process must fire; one that never does describes work that cannot
        // happen. The inputs it wanted are quoted, because that is the edit to fix.
        for process in &self.processes {
            if fired.contains(&process.name) {
                continue;
            }
            let missing: Vec<&str> = process
                .inputs
                .iter()
                .map(|qty| qty.substance.as_str())
                .filter(|name| !available.contains(*name))
                .collect();
            defects.refuse(
                RefusalKind::UnbalancedProcess,
                format!(
                    "{}: dead — it never fires, because {:?} is never available in tier {:?}",
                    process.name, missing, process.tier
                ),
            );
        }

        // A `made` row is a promise that a process makes it.
        let made: Vec<&str> = self
            .substances
            .values()
            .filter(|substance| substance.source == super::schema::Source::Made)
            .map(|substance| substance.name.as_str())
            .collect();
        for name in &made {
            if available.contains(*name) {
                continue;
            }
            let producers: Vec<&Process> = self
                .processes
                .iter()
                .filter(|process| process.outputs.iter().any(|qty| qty.substance == *name))
                .collect();
            let detail = if producers.is_empty() {
                format!("{name}: declared `made`, and no process produces it — an orphan")
            } else {
                let names: Vec<&str> = producers
                    .iter()
                    .map(|process| process.name.as_str())
                    .collect();
                format!(
                    "{name}: declared `made`, and its only producers {names:?} never fire — an \
                     orphan behind dead work"
                )
            };
            defects.refuse(RefusalKind::OrphanSubstance, detail);
        }

        let coverage = Coverage {
            tiers: tiers.len(),
            tiers_reached: reached.len(),
            substances: self.substances.len(),
            substances_available: available.len(),
            substances_made: made.len(),
            substances_made_produced: made.iter().filter(|name| available.contains(**name)).count(),
            processes: self.processes.len(),
            processes_fired: fired.len(),
        };

        Tree { coverage, defects, available, fired, reached }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Ratio;
    use crate::materials::schema::{
        Availability, Family, Hue, Mechanism, Process, Qty, Requires, Sink, Source, State, Substance,
        Tier, Unit,
    };

    fn ratio(num: u64, den: u64) -> Ratio {
        Ratio::new(num, den).expect("a rational in a test")
    }

    fn substance(name: &str, source: Source) -> Substance {
        Substance {
            name: name.into(),
            family: Family::Ceramic,
            hue: Hue::Natural,
            unit: Unit::Mass,
            state: crate::materials::schema::State::Solid,
            phase_of: None,
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

    /// A molten row: a volume of a solid at its melting point.
    fn molten(name: &str, phase_of: &str) -> Substance {
        let mut row = substance(name, Source::Made);
        row.unit = Unit::Volume;
        row.density_g_ml = Some(Ratio::from_u64(1));
        row.state = State::Molten;
        row.phase_of = Some(phase_of.into());
        row
    }

    fn process(name: &str, tier: &str, inputs: &[&str], outputs: &[&str]) -> Process {
        Process {
            name: name.into(),
            tier: tier.into(),
            inputs: inputs.iter().map(|name| Qty::new(*name, Ratio::from_u64(1000))).collect(),
            outputs: outputs.iter().map(|name| Qty::new(*name, Ratio::from_u64(1000))).collect(),
            mechanism: Mechanism {
                heat_c: None,
                heat_kind: None,
                hours: ratio(1, 2),
                labour_hours: ratio(1, 2),
                power_kw: Ratio::ZERO,
            },
            requires: Requires::default(),
            source: "a test row".into(),
            provenance: Availability::Declared,
            note: String::new(),
        }
    }

    fn tier(name: &str, ordinal: u32) -> Tier {
        Tier { name: name.into(), ordinal, note: String::new() }
    }

    /// Two rungs, one process each, in order: stone → haft in the first, haft →
    /// hatchet in the second. The smallest tree that is whole.
    fn whole_table() -> Table {
        Table::new(
            vec![tier("first", 0), tier("second", 1)],
            vec![
                substance("stone", Source::Mined),
                substance("haft", Source::Made),
                substance("hatchet", Source::Made),
            ],
            vec![
                process("riven haft", "first", &["stone"], &["haft"]),
                process("assemble", "second", &["haft"], &["hatchet"]),
            ],
            vec![],
        )
    }

    #[test]
    fn the_whole_table_is_complete_and_reachable() {
        let tree = whole_table().tree();
        assert!(tree.defects().is_empty(), "unexpected defects: {}", tree.report());
        assert!(tree.is_complete(), "{}", tree.report());
        let coverage = tree.coverage();
        assert_eq!((coverage.tiers_reached, coverage.tiers), (2, 2));
        assert_eq!((coverage.processes_fired, coverage.processes), (2, 2));
        assert_eq!((coverage.substances_made_produced, coverage.substances_made), (2, 2));
        assert_eq!((coverage.substances_available, coverage.substances), (3, 3));
    }

    /// The loud failure: a tier whose only process consumes a later tier's
    /// product is unreachable, and the row is named rather than skipped.
    #[test]
    fn a_forward_edge_leaves_its_tier_unreachable() {
        let table = Table::new(
            vec![tier("first", 0), tier("second", 1)],
            vec![
                substance("stone", Source::Mined),
                substance("billet", Source::Made),
                substance("edge", Source::Made),
            ],
            vec![
                // The early tier reaches forward for the later tier's product.
                process("use the billet", "first", &["billet"], &["stone"]),
                process("cast the billet", "second", &["stone"], &["billet"]),
                process("knap the edge", "second", &["stone"], &["edge"]),
            ],
            vec![],
        );
        let tree = table.tree();
        let report = tree.report();
        assert!(!tree.is_complete(), "a forward edge must not pass: {report}");
        assert!(
            report.contains("unreachable from the first"),
            "the unreachable tier must be named: {report}"
        );
        assert!(report.contains("use the billet"), "the stranded row must be quoted: {report}");
        assert!(!tree.is_reached("first"), "no work in the first tier can fire: {report}");
        assert!(tree.is_fired("cast the billet"), "the second tier is still described: {report}");
    }

    /// A `made` substance whose producer was deleted is an orphan, and the row
    /// that still consumes it is dead. Both are reported.
    #[test]
    fn a_made_substance_with_no_producer_is_an_orphan() {
        let mut table = whole_table();
        table.substances.insert("ghost".into(), substance("ghost", Source::Made));
        table.processes.push(process("use the ghost", "second", &["ghost"], &["hatchet"]));
        let tree = table.tree();
        let report = tree.report();
        assert!(!tree.is_complete(), "{report}");
        assert!(report.contains("ghost: declared `made`"), "the orphan must be named: {report}");
        assert!(report.contains("use the ghost: dead"), "the dead row must be named: {report}");
        assert!(report.contains("orphan"), "{report}");
    }

    /// A tier with no work at all is refused by name, not treated as an empty
    /// rung that is fine.
    #[test]
    fn a_tier_with_no_work_is_unreachable() {
        let mut table = whole_table();
        table.tiers.push(tier("empty", 2));
        let tree = table.tree();
        let report = tree.report();
        assert!(!tree.is_complete(), "{report}");
        assert!(
            report.contains("declares no work at all"),
            "the empty tier must be named: {report}"
        );
    }

    /// A state with no transition into it: the melt is declared and nothing
    /// melts it, so it is unobtainable and the cast that wants it is dead. The
    /// proof must fail loudly with both names, not skip the state.
    #[test]
    fn a_molten_row_with_no_melt_is_unreachable() {
        let mut table = whole_table();
        table.substances.insert("molten_copper".into(), molten("molten_copper", "stone"));
        table.processes.push(process(
            "cast the melt",
            "second",
            &["molten_copper"],
            &["hatchet"],
        ));
        let tree = table.tree();
        let report = tree.report();
        assert!(!tree.is_complete(), "a melt no process makes must not pass: {report}");
        assert!(
            report.contains("molten_copper: declared `made`"),
            "the unobtainable melt must be named: {report}"
        );
        assert!(
            report.contains("cast the melt: dead"),
            "the process that consumes it must be named dead: {report}"
        );
    }

    /// An input the table does not hold is refused as an unknown name here too,
    /// so a typo cannot make a process look merely idle.
    #[test]
    fn an_unknown_input_is_refused_by_name() {
        let mut table = whole_table();
        table.processes.push(process("use a typo", "second", &["hatchet_"], &["hatchet"]));
        let tree = table.tree();
        let report = tree.report();
        assert!(!tree.is_complete(), "{report}");
        assert!(report.contains("not a declared substance"), "{report}");
        assert!(report.contains("hatchet_"), "the missing name must be quoted: {report}");
    }

    /// The committed table, walked end to end. This is the proof the mission asks
    /// for: every rung of the ladder is reached, every substance is obtainable,
    /// every process fires — and an edit that orphans any node fails here, loudly,
    /// naming the node. The counts it prints are the tree's report card.
    #[test]
    fn the_committed_tree_is_complete_and_reachable() {
        const LADDER: [&str; 8] = [
            "hands & stone",
            "bound & composite",
            "fire & clay",
            "copper & bronze",
            "iron & steel",
            "mill & mechanism",
            "chemistry & weathering",
            "electricity & precision",
        ];

        let table = crate::materials::generated::table();
        let tree = table.tree();
        let report = tree.report();
        assert!(tree.is_complete(), "the committed tree is not whole: {report}");

        // The ladder is the tree's spine: all eight rungs declared, all reached.
        for rung in LADDER {
            assert!(
                table.tiers.iter().any(|tier| tier.name == rung),
                "the ladder's rung {rung:?} is not declared: {report}"
            );
            assert!(tree.is_reached(rung), "the rung {rung:?} is not reached: {report}");
        }

        // No node may be stranded, whatever the counts say.
        for substance in table.substances.values() {
            assert!(
                tree.is_available(&substance.name),
                "{} is declared but unobtainable: {report}",
                substance.name
            );
        }
        for process in &table.processes {
            assert!(
                tree.is_fired(&process.name),
                "the process {:?} never fires: {report}",
                process.name
            );
        }

        let coverage = tree.coverage();
        assert_eq!(coverage.tiers, LADDER.len(), "{report}");
        assert_eq!(coverage.tiers_reached, coverage.tiers, "{report}");
        assert_eq!(coverage.substances_available, coverage.substances, "{report}");
        assert_eq!(coverage.substances_made_produced, coverage.substances_made, "{report}");
        assert_eq!(coverage.processes_fired, coverage.processes, "{report}");

        // Printed so the gate's log carries the numbers the proof produced.
        println!("the committed tree: {}", coverage.report());
    }
}
