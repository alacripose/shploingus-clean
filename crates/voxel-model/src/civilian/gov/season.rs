//! The governance period, **on the record the tree already owns** — and the seam's own boundary,
//! stated rather than crossed.
//!
//! This is not a season. `crate::chain::Season` is the world's year — winter, spring, summer,
//! autumn — and the record rules out a second meaning of the word (`event-0131`, H-10.5).
//! What the machine's `Season` section held was a **governance period** (`SeasonMeta { id,
//! purpose, campaigns, counters }`) plus a second record store beside the chain (`index.json`,
//! `events.jsonl`, `tickets/`, `retirements/`, `evidence/`, `corrections/`). Both defects are
//! refused here by construction:
//!
//! * the period is the namespace's **`campaign`** (`crate::chain::Namespace`, `chain.rs:166`) —
//!   `world / year / season / campaign` — so the machine's period rides the record's own field
//!   instead of inventing a type beside it;
//! * the store is **the chain** — `chains/workspace.chain`, the one append-only record. Tickets,
//!   retirements, evidence and corrections are filed as chain events and read back by folding,
//!   and their ids come **from the directory of positions, never from a counter**: the id of a
//!   record is `{prefix}-{position}`, which is what `Board::from_chain` already reads. A process
//!   killed mid-run cannot leave a counter behind its own directory, and a corrupt record loses
//!   that record — it is counted, and the fold carries on — not the whole file.
//!
//! # The boundary: this file does not read or write a chain file, and that is deliberate
//!
//! The first draft of this seam called `crate::chain_bytes::{read, write}`. **That module does
//! not exist and must not be written here.** A `.chain` file is a concatenation of
//! length-described frames, and the walk that finds a frame boundary — `frame_at`, and the fold
//! built on it — lives in `voxel-package`'s `compiler`, **one layer above this crate**
//! (`C-67.3`: *a file is a concatenation of length-described records, not a list of lines*).
//! `wire::CanonicalCodec::decode` decodes **one** frame, so it cannot walk a file on its own.
//! Writing a second framing implementation down here would be `C-67.0`'s defect exactly: a format
//! invented beside the one that already exists, in the tree that cannot see it.
//!
//! So the chain is **handed in by the caller**, which is the only layer that can read the file.
//! What this seam owns is the part it *can* own: the period is the namespace's campaign field,
//! the ids are positions, and the append is the chain's own.
//!
//! # Owed, and measured rather than guessed
//!
//! `gov/mod.rs` calls a record store by these names, and the call sites are measured —
//! `Season::open`/`Season::attach` and `read_tickets`/`read_retirements`/`read_evidence`/
//! `read_corrections` at 721–752, 827, 847, 868, 886, 894, `append_event` at 915–916, `next_id`
//! and `write_ticket` at 975, `write_correction` at 1112–1124, `write_evidence`/`write_retirement`
//! at 1147–1156. Each becomes a chain event kind and a fold here. Until they land the machine
//! does not compile, which is the state `docs/CIVILIAN-PORT.md`'s ordered list is written for.

use crate::chain::{Chain, EventKind, Namespace, Season as WorldSeason};
use crate::refusal::{Refusal, RefusalKind, RefusalSet, Refused};

/// The record this machine files into, relative to the workspace root. One chain file; a second
/// is refused by the reader, not merged.
pub const CHAIN_DIR: &str = "chains";
pub const CHAIN_FILE: &str = "workspace.chain";

/// The world's day, from the record's own ladder: one day per 24_000 ticks at the game's 20 Hz
/// clock (`voxel-client/src/clock.rs`: `TICK_HZ = 20`). A namespace needs a year; this is where
/// it comes from, and it is declared rather than guessed.
pub const TICKS_PER_DAY: u64 = 24_000;

/// The record's namespace: the machine never invents a world or a campaign.
///
/// The campaign is the governance period the machine's `Season` section used to carry; the season
/// is the world's own, at the tick the event is filed.
pub fn namespace(campaign: &str, tick: u64) -> Refused<Namespace> {
    let season = WorldSeason::at_tick(tick)
        .map_err(|reason| Refusal::owned(RefusalKind::MalformedIdentity, reason))?;
    Namespace::new("shploingus", tick / TICKS_PER_DAY, season, campaign).map_err(|set| {
        set.first_named().cloned().unwrap_or_else(|| {
            Refusal::owned(
                RefusalKind::MalformedIdentity,
                "the namespace was refused with no refusal named",
            )
        })
    })
}

/// The governance record: **a view over the one chain**, and the ids that come from it.
///
/// The chain is the caller's. `chain_path` is where that file is, kept so a caller that *can*
/// read and write it has the path the record states rather than one it re-derives.
pub struct Season {
    /// The governance period this record files under.
    pub name: String,
    /// The chain this view reads and appends to. **Handed in, never read here** — see the module
    /// header for why the file's framing walk is not this crate's.
    pub chain: Chain,
    chain_path: std::path::PathBuf,
    /// Records whose detail did not parse for their kind. Counted, never silently dropped.
    pub malformed: Vec<String>,
}

impl Season {
    /// Open the record for the named period over the chain the caller read.
    ///
    /// Nothing is created and nothing is written: the file's bytes are the caller's, because the
    /// caller is the layer that can walk them.
    pub fn over(
        workspace: impl AsRef<std::path::Path>,
        name: &str,
        chain: Chain,
        malformed: Vec<String>,
    ) -> Refused<Self> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Refusal::owned(
                RefusalKind::MalformedIdentity,
                "a governance period with no name cannot be grouped",
            ));
        }
        let chain_path = workspace.as_ref().join(CHAIN_DIR).join(CHAIN_FILE);
        Ok(Self {
            name: name.to_owned(),
            chain,
            chain_path,
            malformed,
        })
    }

    /// Where the record lives, from the record's own declaration of it.
    pub fn chain_path(&self) -> &std::path::Path {
        &self.chain_path
    }

    /// The id the **next** record with this prefix will carry: the position it will sit at in the
    /// append-only record. From the directory, never a counter — a restart folds the same
    /// positions and lands on the same id.
    ///
    /// The first argument is the store the copied machine names (`"tickets"`, `"evidence"`,
    /// `"corrections"`, `"retirements"`). **The store is the chain**, so that name selects
    /// nothing and is kept only because the machine's call sites are byte-identical — the
    /// measured ones are `gov/mod.rs` 847, 894, 1112, 1147, 1174.
    pub fn next_id(&self, _store: &str, prefix: &str) -> String {
        format!("{prefix}-{:04}", self.chain.len())
    }

    /// Append one record to the chain and return the position it took.
    ///
    /// The append is the chain's own: `Chain::append` takes `&mut self`, which **is** the
    /// one-writer guarantee at the type level, and it refuses an empty detail rather than growing
    /// a file that says less. Persisting the result is the caller's, because writing the file
    /// would mean a second framing implementation one layer below the one that owns it.
    pub fn append(
        &mut self,
        tick: u64,
        kind: EventKind,
        detail: impl Into<String>,
    ) -> Result<u64, RefusalSet> {
        let namespace = namespace(&self.name, tick).map_err(|refusal| {
            let mut set = RefusalSet::new();
            set.push(refusal);
            set
        })?;
        let event = self.chain.append(namespace, kind, detail)?;
        Ok(event.index)
    }

    /// Fold the record the chain's own way. **The only read path** — no accessor hands out the
    /// events, because one is what would tempt a caller to load the file whole.
    pub fn fold<T, F>(&self, initial: T, step: F) -> T
    where
        F: FnMut(T, &crate::chain::Event) -> T,
    {
        self.chain.fold(initial, step)
    }

    /// The number of events the record holds. Counted, not claimed.
    pub fn len(&self) -> usize {
        self.chain.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "shploingus-season-{name}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn the_governance_period_is_the_campaign_field_and_the_store_is_the_chain() {
        let dir = scratch("over");
        let record = Season::over(&dir, "gate-2026-10-02", Chain::new(), Vec::new())
            .expect("a named period");
        assert_eq!(record.name, "gate-2026-10-02");
        assert_eq!(
            record.chain_path(),
            dir.join(CHAIN_DIR).join(CHAIN_FILE),
            "the path is the record's own declaration of where it lives"
        );
        assert!(record.is_empty(), "a view over an empty chain holds nothing");
    }

    #[test]
    fn an_unnamed_period_is_refused() {
        let dir = scratch("unnamed");
        assert!(Season::over(&dir, "  ", Chain::new(), Vec::new()).is_err());
    }

    #[test]
    fn ids_come_from_positions_in_the_record() {
        let dir = scratch("ids");
        let mut record =
            Season::over(&dir, "gate", Chain::new(), Vec::new()).expect("a named period");
        assert_eq!(
            record.next_id("evidence", "EVD"),
            "EVD-0000",
            "an empty record's next position is zero"
        );
        let first = record
            .append(0, EventKind::DevelopmentFact, "the first record")
            .expect("append");
        assert_eq!(first, 0);
        assert_eq!(
            record.next_id("evidence", "EVD"),
            "EVD-0001",
            "the id is the position the next record sits at"
        );
        assert_eq!(record.len(), 1);
    }

    #[test]
    fn the_period_rides_the_namespace_campaign_and_the_world_keeps_its_season() {
        let namespace = namespace("gate-2026-10-02", TICKS_PER_DAY * 3).expect("a namespace");
        assert_eq!(namespace.campaign, "gate-2026-10-02");
        assert_eq!(namespace.world, "shploingus");
        assert_eq!(namespace.year, 3, "one day per TICKS_PER_DAY, from the tick");
        // The season is the world's own, derived from the tick — never the governance period.
        assert_eq!(
            namespace.season,
            WorldSeason::at_tick(TICKS_PER_DAY * 3).expect("the world's season at this tick")
        );
    }

    #[test]
    fn a_record_with_an_empty_detail_is_refused_rather_than_appended() {
        let dir = scratch("blank");
        let mut record =
            Season::over(&dir, "gate", Chain::new(), Vec::new()).expect("a named period");
        assert!(
            record
                .append(0, EventKind::DevelopmentFact, "   ")
                .is_err(),
            "a blank record cannot be read back, and an append-only record cannot take it back"
        );
        assert!(record.is_empty(), "a refused append changes nothing");
    }

    #[test]
    fn a_caller_supplied_chain_is_the_record_the_view_reads() {
        let dir = scratch("supplied");
        let mut chain = Chain::new();
        chain
            .append(
                namespace("gate", 0).expect("a namespace"),
                EventKind::DevelopmentFact,
                "filed by the caller that could read the file",
            )
            .expect("append");
        let record = Season::over(&dir, "gate", chain, vec!["one record did not parse".into()])
            .expect("a named period");
        assert_eq!(record.len(), 1, "the view reads the chain it was handed");
        assert_eq!(
            record.next_id("tickets", "BLD"),
            "BLD-0001",
            "and its ids are that record's positions"
        );
        assert_eq!(
            record.malformed.len(),
            1,
            "a record that did not parse is counted, never swallowed"
        );
        assert!(!record.is_empty());
    }
}
