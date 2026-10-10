//! **The third view: the context an agent receives.** (`RUST_PORTING_RECOVERY_PROMPT_V2.md` §58)
//!
//! # The ruling, in the record's own words
//!
//! > **§58** — *"The agent should not need to read thousands of Markdown files to understand its
//! > work. Provide a queryable context projection."*
//! > **§57** — *"The parent/child relationship must be explicit. Never let semantic lineage exist
//! > only in prose."*
//! > **§39** — *"The project core is a **secure semantic diffusion ticket ledger**."*
//!
//! # Why this is a third view and not a second copy
//!
//! This tree already has two views of one record, and adding a third needs to say what it is not:
//!
//! | view | question it answers | shape |
//! |---|---|---|
//! | [`voxel_model::chain::Chain::project`] | *what does the whole record say, as a document?* | Markdown, every event |
//! | [`crate::projection`] | *how far has `docs/LEDGER.md` drifted from the chain?* | a measured gap |
//! | **this module** | ***what is this agent's work, and what is it blocked on?*** | typed buckets and a lineage |
//!
//! The distinction that matters is the last column. §58's complaint is precisely that an agent has
//! to **read a document** to find out what it owes; a projection that answers that by producing
//! another document has changed the format and not the problem. So nothing here is rendered by
//! default: [`Context`] is queried, and [`Context::brief`] exists only because an agent needs one
//! screen of orientation before it can ask a question.
//!
//! # Derivations, and the one thing this module refuses to be
//!
//! A [`Context`] is a **function of the chain**. It is not serialised, not written, and not
//! loadable — there is no `save`, no `to_bytes` and no `from_bytes`, because a stored context is a
//! second record of what the first record already says, and two records can disagree. That is
//! H-8.1's failure arriving through a third door.
//!
//! The read path is [`voxel_model::chain::Chain::fold`] and only `fold`. C-54.1 records the
//! consequence as binding — *"the file grows without bound, so the read path must be designed for a
//! long file from the start"* — and [`voxel_model::chain::Chain`] hands out no event slice for
//! exactly that reason, so this module cannot accidentally become a loader.

use std::fmt;

use voxel_model::chain::{Chain, EventKind, Linkage, Season};
use voxel_model::identity::Digest;

/// One event, as an agent needs to read it: where it happened, what kind it was, and what it said.
///
/// **The namespace is carried as its four fields rather than as a `Namespace`**, so a `Line` has no
/// borrow of the chain it came from and a context outlives the fold that built it without either
/// cloning a namespace the chain still owns or holding a lifetime nobody can name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// Its position in the chain. **Identity is a position in an append-only record.**
    pub index: u64,
    /// The world it happened in.
    pub world: String,
    /// The world's year, zero-based.
    pub year: u64,
    /// The world's season — the world's own year, never a governance period (H-10.5).
    pub season: Season,
    /// The development campaign it belongs to.
    pub campaign: String,
    /// What happened, verbatim. **Nothing is summarised away**, because a summary is a second
    /// reading of the record and this module does not re-read it.
    pub detail: String,
    /// The link this event claims to its predecessor, **carried rather than implied** (§57).
    ///
    /// This is what makes semantic lineage readable out of the record instead of out of prose. A
    /// caller that wants to know what an event descends from asks the record; it does not ask a
    /// document that might have been typed by hand.
    pub previous: Linkage,
}

impl Line {
    /// `"campaign/world/year-season"`, the address a caller filters and prints by.
    pub fn address(&self) -> String {
        format!(
            "{}/{}/{}-{}",
            self.campaign,
            self.world,
            self.year,
            self.season.name()
        )
    }

    /// Whether this event's link to its predecessor was ever computed.
    pub fn is_linked(&self) -> bool {
        self.previous.is_evaluated()
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} — {}", self.index, self.address(), self.detail)
    }
}

/// The agent's projection of the chain: **every event readable by kind, and the lineage it links**.
///
/// Built once by folding the chain ([`Context::over`]); after that it answers questions without
/// touching the record again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    /// The digest of the chain this was folded from — so a context can state *which* record it is
    /// a reading of, and two contexts can be compared without comparing their events.
    chain_digest: Digest,
    /// How many events the chain held.
    events: u64,
    /// The chain's own honesty statement about linkage, verbatim from
    /// [`voxel_model::chain::Chain::tamper_evident_description`]. **Never a bare `false`**, for the
    /// reason that method records.
    integrity: String,
    /// One bucket per [`EventKind`], in [`EventKind::ALL`]'s own order — so the partition is total
    /// by construction rather than by a caller remembering every arm.
    buckets: Vec<Vec<Line>>,
    /// Whether the chain was hash-linked, so an agent can tell *"never checked"* from *"checked and
    /// found wanting"* without parsing the sentence above.
    linked: bool,
}

impl Context {
    /// **The only constructor.** Fold the chain into the agent's view of it.
    ///
    /// There is deliberately no `Context::new`: a context that did not come from a chain would be a
    /// second source of truth about what happened, which is the one thing a projection may not be.
    /// The same rule `Board::from_chain` states for the ticket board, for the same reason.
    pub fn over(chain: &Chain) -> Self {
        let mut buckets: Vec<Vec<Line>> = vec![Vec::new(); EventKind::KIND_COUNT];
        let events = chain.fold(0u64, |seen, event| {
            // The bucket is found by asking `EventKind::ALL` for the kind's own position, so a new
            // kind added to the enum is placed by the enum rather than by a match here that a
            // reader has to keep in step.
            if let Some(position) = EventKind::ALL.iter().position(|kind| *kind == event.kind) {
                buckets[position].push(Line {
                    index: event.index,
                    world: event.namespace.world.clone(),
                    year: event.namespace.year,
                    season: event.namespace.season,
                    campaign: event.namespace.campaign.clone(),
                    detail: event.detail.clone(),
                    previous: event.previous,
                });
            }
            seen + 1
        });

        Context {
            chain_digest: chain.digest(),
            events,
            integrity: chain.tamper_evident_description(),
            buckets,
            linked: chain.is_tamper_evident(),
        }
    }

    /// The digest of the chain this is a reading of.
    pub fn chain_digest(&self) -> Digest {
        self.chain_digest
    }

    /// How many events the reading covered.
    pub fn events(&self) -> u64 {
        self.events
    }

    /// Whether the chain this came from was hash-linked.
    pub fn is_linked(&self) -> bool {
        self.linked
    }

    /// The chain's own sentence about linkage. **Print this; never print a bare boolean.**
    pub fn integrity(&self) -> &str {
        &self.integrity
    }

    /// Whether the record holds nothing at all. An empty projection is a real state, and the
    /// integrity sentence above says which kind of empty it is.
    pub fn is_empty(&self) -> bool {
        self.events == 0
    }

    /// Everything of one kind, in chain order.
    pub fn of_kind(&self, kind: EventKind) -> &[Line] {
        match EventKind::ALL.iter().position(|candidate| *candidate == kind) {
            Some(position) => &self.buckets[position],
            // Unreachable while `EventKind::ALL` lists every variant, which
            // `KINDS_ARE_TOTAL` pins. Returning empty rather than panicking keeps a projection
            // (which is a *reading*) from being able to take a process down.
            None => &[],
        }
    }

    /// **Every refusal on the record.** §39's ledger rule is that *"a refusal nobody can read is a
    /// refusal that did not happen"*, so a context that an agent receives must carry them rather
    /// than leave them to be found.
    pub fn refusals(&self) -> &[Line] {
        self.of_kind(EventKind::RefusalRecorded)
    }

    /// Every filed blocker — what the work is waiting on.
    pub fn blockers(&self) -> &[Line] {
        self.of_kind(EventKind::BlockerFiled)
    }

    /// Every filed milestone.
    pub fn milestones(&self) -> &[Line] {
        self.of_kind(EventKind::MilestoneFiled)
    }

    /// Every ticket transition, in order.
    pub fn ticket_transitions(&self) -> &[Line] {
        self.of_kind(EventKind::TicketTransition)
    }

    /// Every retirement. **A retirement is a separate kind, never a status flip**, so it is a
    /// separate question here too.
    pub fn retirements(&self) -> &[Line] {
        self.of_kind(EventKind::Retirement)
    }

    /// Every world action.
    pub fn world_actions(&self) -> &[Line] {
        self.of_kind(EventKind::WorldAction)
    }

    /// Every development fact.
    pub fn development_facts(&self) -> &[Line] {
        self.of_kind(EventKind::DevelopmentFact)
    }

    /// **The lineage of one event, walking backwards to the chain's own beginning.**
    ///
    /// This is §57's requirement made queryable: *"never let semantic lineage exist only in prose."*
    /// The walk is over the record's own positions, so what comes back is the descent the chain
    /// states rather than one this module inferred.
    ///
    /// It **stops where the record stops being readable as a descent** — at an unlinked event, whose
    /// `previous` was never computed. Carrying on past that point would be this module inventing the
    /// ancestry it is supposed to be reading, which is `Linkage::NotEvaluable`'s whole subject.
    pub fn lineage(&self, index: u64) -> Vec<&Line> {
        let mut walk = Vec::new();
        let mut cursor = Some(index);
        while let Some(at) = cursor {
            let Some(line) = self.event(at) else { break };
            walk.push(line);
            if !line.is_linked() || at == 0 {
                break;
            }
            cursor = Some(at - 1);
        }
        walk
    }

    /// One event by its position, or `None` if no event holds it.
    pub fn event(&self, index: u64) -> Option<&Line> {
        self.buckets
            .iter()
            .flat_map(|bucket| bucket.iter())
            .find(|line| line.index == index)
    }

    /// **One screen of orientation**, for an agent that has nothing else.
    ///
    /// Deliberately not the record and not a document: it names the chain, states the integrity
    /// sentence, counts each kind, and leads with the refusals and the blockers — because those are
    /// the two things an agent that read nothing else would get wrong.
    pub fn brief(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "context: {} events, chain {}\n",
            self.events,
            self.chain_digest.to_hex()
        ));
        out.push_str(&format!("integrity: {}\n", self.integrity));
        if self.is_empty() {
            out.push_str(
                "the record is empty, so there is no work, no refusal and no blocker to read\n",
            );
            return out;
        }
        for kind in EventKind::ALL {
            let lines = self.of_kind(kind);
            if lines.is_empty() {
                continue;
            }
            out.push_str(&format!("{}: {}\n", kind.name(), lines.len()));
            // Namespaces and the last line of each kind: enough to act, not enough to replace the
            // record. A caller that wants more asks the queries above.
            let mut addresses: Vec<String> = lines.iter().map(Line::address).collect();
            addresses.sort();
            addresses.dedup();
            out.push_str(&format!("  in {}\n", addresses.join(", ")));
            if let Some(last) = lines.last() {
                out.push_str(&format!("  last: {}\n", last.detail));
            }
        }
        if !self.refusals().is_empty() {
            out.push_str(&format!(
                "read first: {} — a refusal nobody reads is a refusal that did not happen\n",
                self.refusals()
                    .iter()
                    .map(|line| line.detail.as_str())
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }
        if !self.blockers().is_empty() {
            out.push_str(&format!(
                "waiting on: {}\n",
                self.blockers()
                    .iter()
                    .map(|line| line.detail.as_str())
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }
        out
    }
}

/// **What an agent receives when it asks the ledger what its work is** — and the rule that the
/// answer is a reading, not a store.
///
/// Named so a caller can hold the two apart in prose: a `Context` is taken, never filed.
pub type AgentContext = Context;

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_model::chain::{Namespace, Season};

    fn namespace(campaign: &str, year: u64, season: Season) -> Namespace {
        Namespace::new("shploingus", year, season, campaign).expect("a well-formed namespace")
    }

    /// A three-event chain: one of each of the kinds this module's tests care about.
    fn worked_chain() -> Chain {
        let mut chain = Chain::linked();
        chain
            .append(
                namespace("C-1", 0, Season::Winter),
                EventKind::TicketTransition,
                "BLD-0001 opened",
            )
            .expect("an appending chain");
        chain
            .append(
                namespace("C-1", 0, Season::Winter),
                EventKind::BlockerFiled,
                "BLD-0001 waits on the shaper port",
            )
            .expect("an appending chain");
        chain
            .append(
                namespace("C-1", 1, Season::Spring),
                EventKind::RefusalRecorded,
                "the door refused a presentation with no presenter",
            )
            .expect("an appending chain");
        chain
    }

    #[test]
    fn the_projection_partitions_every_event_exactly_once() {
        // The totality claim: every event lands in exactly one bucket. A projection that could drop
        // an event would be a view that disagrees with the record while looking complete, which is
        // the failure H-8.1 exists to make impossible.
        let chain = worked_chain();
        let context = Context::over(&chain);
        let counted: usize = EventKind::ALL
            .iter()
            .map(|kind| context.of_kind(*kind).len())
            .sum();
        assert_eq!(counted, chain.len(), "every event sits in one bucket");
        assert_eq!(context.events(), chain.len() as u64);

        // And the positions are distinct, so no event was filed twice.
        let mut seen: Vec<u64> = EventKind::ALL
            .iter()
            .flat_map(|kind| context.of_kind(*kind).iter().map(|line| line.index))
            .collect();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), total, "no event appears in two buckets");
    }

    #[test]
    fn an_empty_chain_gives_an_empty_projection_that_says_which_empty_it_is() {
        let context = Context::over(&Chain::linked());
        assert!(context.is_empty());
        assert_eq!(context.events(), 0);
        // The honesty rule: an empty chain is not "tamper-evident", and never a bare `false`.
        assert!(!context.is_linked());
        assert!(
            context.integrity().contains("nothing has been recorded"),
            "an empty chain says so: {}",
            context.integrity()
        );
        assert!(context.refusals().is_empty());
        assert!(context.brief().contains("no work, no refusal and no blocker"));
    }

    #[test]
    fn the_refusals_are_carried_rather_than_left_to_be_found() {
        let context = Context::over(&worked_chain());
        assert_eq!(context.refusals().len(), 1);
        assert!(context.refusals()[0].detail.contains("no presenter"));
        assert_eq!(context.blockers().len(), 1);
        assert_eq!(context.ticket_transitions().len(), 1);
        // And the brief leads with them, because that is what an agent that read nothing else
        // would get wrong.
        let brief = context.brief();
        assert!(brief.contains("a refusal nobody reads is a refusal that did not happen"));
        assert!(brief.contains("waiting on:"));
    }

    #[test]
    fn the_lineage_walks_back_to_the_first_event() {
        // §57: the parent/child relationship is explicit. Each `Line` carries the link it claims,
        // and the walk reads the record's own positions rather than inventing an ancestry.
        let chain = worked_chain();
        let context = Context::over(&chain);
        let lineage = context.lineage(2);
        assert_eq!(lineage.len(), 3, "the third event descends from two");
        assert_eq!(
            lineage.iter().map(|line| line.index).collect::<Vec<_>>(),
            vec![2, 1, 0]
        );
        assert!(lineage.iter().all(|line| line.is_linked()));
        assert_eq!(context.lineage(0).len(), 1, "the first event is its own root");
        assert!(context.lineage(99).is_empty(), "no such position, no lineage");
    }

    #[test]
    fn the_lineage_stops_where_the_record_stops_being_a_descent() {
        // A chain whose first record says `not_evaluable` (the old shape) must not be walked past:
        // carrying on would be this module inventing the ancestry it is reading. The second event
        // is linked, so the walk gets one line and stops at the unlinked one beneath it.
        let mut chain = Chain::new();
        chain
            .append(
                namespace("C-1", 0, Season::Winter),
                EventKind::DevelopmentFact,
                "written by a build that did not link",
            )
            .expect("an appending chain");
        chain
            .append(
                namespace("C-1", 0, Season::Winter),
                EventKind::DevelopmentFact,
                "written by a build that does",
            )
            .expect("an appending chain");

        let context = Context::over(&chain);
        let lineage = context.lineage(1);
        assert_eq!(lineage.len(), 2, "the walk reaches the unlinked event");
        assert!(!lineage[1].is_linked(), "and reports it as unlinked");
        assert!(!context.is_linked());
    }

    #[test]
    fn the_projection_is_a_function_of_the_chain_alone() {
        // Two readings of one chain are the same reading — so a context can be compared without a
        // second record existing to compare it to. And a context of a *different* chain is not it.
        let chain = worked_chain();
        let a = Context::over(&chain);
        let b = Context::over(&chain);
        assert_eq!(a, b);
        assert_eq!(a.chain_digest(), b.chain_digest());
        assert_eq!(a, a.clone(), "a reading is a value, and equality is over its content");

        let mut grown = chain.clone();
        grown
            .append(
                namespace("C-1", 1, Season::Spring),
                EventKind::MilestoneFiled,
                "the projection landed",
            )
            .expect("an appending chain");
        let c = Context::over(&grown);
        assert_ne!(a, c, "the record moved, so the reading moved with it");
        assert_eq!(c.milestones().len(), 1);
    }

    #[test]
    fn the_address_names_the_campaign_the_world_and_the_season_separately() {
        // The one conflation this tree has a standing correction against (H-10.5): a season is the
        // world's year and a campaign is a development period. The address keeps them apart, and
        // the year is neither.
        let context = Context::over(&worked_chain());
        let line = &context.refusals()[0];
        assert_eq!(line.address(), "C-1/shploingus/1-spring");
        assert_eq!(line.season, Season::Spring);
        assert_ne!(line.campaign, line.season.name());
    }
}
