//! **One append-only record, two views.** (H-8.1, H-11.7, C-49 ruling 1, C-54.1)
//!
//! # The ruling, in the record's own words
//!
//! H-8.1: *"**One append-only record, two views.** The same canonical record carries
//! world actions (dug here, shaped there) and development facts (this check ran, this
//! feature is missing). `docs/LEDGER.md` and the in-game ledger window are **two
//! projections of one record**, so a disagreement between them is impossible by
//! construction."*
//!
//! H-11.7 / C-54.1: *"**One chain for everything, namespaced**"* — one repo-root ledger,
//! one hash chain, world events and development facts sharing it, every event carrying its
//! `world`/`year`/`season`/`campaign` namespace.
//!
//! So this module is the **canonical record**, and the two views are derived from it. A view
//! that could disagree with the record would defeat the ruling, which is why
//! [`Chain::project`] exists and why nothing here is a second copy of anything.
//!
//! # What is *not* here, and why that matters
//!
//! **This is not the world.** [`voxel_shapers::earthworks::Earthworks`] is the world's
//! modification record — *"an unrecorded modification is invisible"* — and it is a different
//! record with a different job. C-54.1 says world events and development facts share **one
//! chain**, and that is an event-level statement: this module records *that* a world event
//! happened and what it was, with a digest binding it to the world that produced it. It does
//! not re-model voxels, and a second voxel-level record would be exactly the drift the record
//! forbids.
//!
//! **This is not a signature.** See the honesty rule below.
//!
//! # The honesty rule, which is the most important thing in this file
//!
//! The record states it twice, and it is the reason this module has a `not_evaluable` state
//! rather than a boolean:
//!
//! > **`previous_event_hash=not_evaluable` means never call that state tamper-evident** — and
//! > a hash, when computed, is evidence about bytes and chain order, *not permission*.
//!
//! [`Linkage`] carries that as a type. [`Linkage::NotEvaluable`] is a **real, constructible
//! state**, not an error path: a chain whose linkage was never computed *is* append-only, and
//! saying so is the truth. A `bool` named `tamper_evident` would have to be `false` for such a
//! chain, and `false` reads as *"checked and found wanting"* rather than *"never checked"* —
//! the same conflation [`crate::compendium`] refuses when it dropped a check that read a
//! filing location as a promotion.
//!
//! [`Chain::is_tamper_evident`] therefore returns `false` for a `NotEvaluable` chain **and
//! says so in [`Chain::tamper_evident_description`]**, which is what a caller should print.
//!
//! # The consequences C-54.1 records as binding, and how each is honoured
//!
//! | recorded consequence | how this module honours it |
//! |---|---|
//! | **One writer** — a single chain cannot be appended by two processes | [`Chain::append`] takes `&mut self`, so the borrow checker enforces it; [`WriterLock`] makes it explicit on disk and [`RefusalKind::ConcurrentWriter`] names the violation |
//! | **Seasons are fields, never directories** | [`Namespace::season`] is a field of type [`Season`], and [`Season`] has **no `Governance`** variant — H-10.5's standing correction, as a type |
//! | **The file grows without bound; the read path must fold, not load** | [`Chain::fold`] is the only read path. There is no `events()` accessor returning `&[Event]`, because one would be the thing that tempts a caller to load the whole file |
//! | **The whole file is the tamper boundary** | [`Chain::digest`] covers every event in order, so a prefix-only check cannot be mistaken for the whole |
//! | **A corrupt record loses that record, not the file** | [`FoldReport`] counts corrupt lines, keeps folding, and carries the line numbers — [`RefusalKind::CorruptRecord`] is reported, never swallowed |
//!
//! # Why the namespace is a type and not four strings
//!
//! H-10.5 is a **standing correction**: *"every past agent that read 'season' as an
//! organisational period made the same mistake."* If `season` were a `String` this module
//! would happily record `"campaign-3"`. [`Season`] is a closed four-variant enum — winter,
//! spring, summer, autumn — so that mistake **does not compile**. The campaign is a
//! *separate* field, which is the distinction the correction is about.

use crate::identity::Digest;
use crate::refusal::{RefusalKind, RefusalSet};

/// The chain's format version. The pin, and what a future format is refused against.
pub const CHAIN_FORMAT: &str = "shploingus.chain/1";

/// The domain tag every chain digest is taken under.
///
/// **A digest over a domain string folded in ahead of the payload**, so a chain digest and
/// a document digest can never be mistaken for one another. This tree has exactly one hashing
/// implementation ([`Digest`]) and exactly one place that decides *what* is hashed; a second
/// scheme would be a second answer to "what is the identity of a thing".
pub const CHAIN_DOMAIN: &str = "shploingus/chain-event";

/// The literal that means *"this link was never computed"*, and which therefore forbids the
/// word *tamper-evident* for the whole chain.
pub const NOT_EVALUABLE: &str = "not_evaluable";

/// The world's season. **Four, and only four.** (H-10.5)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Season {
    Winter,
    Spring,
    Summer,
    Autumn,
}

impl Season {
    pub const ALL: [Season; 4] = [Season::Winter, Season::Spring, Season::Summer, Season::Autumn];

    pub const fn name(self) -> &'static str {
        match self {
            Season::Winter => "winter",
            Season::Spring => "spring",
            Season::Summer => "summer",
            Season::Autumn => "autumn",
        }
    }

    /// Parse, **refusing** anything else.
    ///
    /// The refusal names what it saw, because the whole point of H-10.5 is that a season is a
    /// specific thing and `"campaign-3"` is not one.
    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|season| season.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "{word:?} is not a season. A season is one of winter, spring, summer, autumn — \
                     the world's own year, never a governance period, a campaign or a historical \
                     boundary (H-10.5)"
                )
            })
    }

    /// The season a tick falls in, from the tick alone.
    ///
    /// **Deterministic and total**, and the constants are this tree's own: `TICKS_PER_DAY =
    /// 24_000` and `DAYS_PER_YEAR = 96` give 2 304 000 ticks per year, so one season is
    /// `DAYS_PER_YEAR / 4 = 24` days = **576 000 ticks** (H-10.5, C-49 ruling 8).
    ///
    /// `u64` throughout and **no wrapping arithmetic** in the modulus: a tick near `u64::MAX`
    /// would wrap and put a world in the wrong season, and a silently wrong season is worse
    /// than a refused one. `checked_mul` returning `None` is refused by [`Season::at_tick`].
    pub fn at_tick(tick: u64) -> Result<Self, String> {
        const TICKS_PER_DAY: u64 = 24_000;
        const DAYS_PER_YEAR: u64 = 96;
        let year = TICKS_PER_DAY
            .checked_mul(DAYS_PER_YEAR)
            .ok_or_else(|| "the tick clock overflowed".to_string())?;
        let day_of_year = (tick % year) / TICKS_PER_DAY;
        // 96 days per year over four seasons: exactly 24 each, and the division is exact so
        // there is no remainder to distribute and no boundary to argue about.
        let season_index = (day_of_year / (DAYS_PER_YEAR / 4)) as usize;
        Self::ALL
            .get(season_index)
            .copied()
            .ok_or_else(|| format!("tick {tick} produced season index {season_index}"))
    }
}

/// Where an event sits: the world it happened in, and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Namespace {
    /// The world this event belongs to. A seed, a name, an id — a `String`, because the
    /// record does not fix the spelling of a world's identity and inventing one would be
    /// C-61's mistake.
    pub world: String,
    /// The world's year, zero-based.
    pub year: u64,
    /// The world's season. **A field, never a directory** (C-54.1).
    pub season: Season,
    /// The development campaign this event belongs to.
    ///
    /// **Separate from `season`, and that separation is the point.** H-10.5 is a standing
    /// correction precisely because past agents conflated the two; here they are different
    /// fields with different types, so a reader cannot mistake one for the other and a writer
    /// cannot record one as the other.
    pub campaign: String,
}

impl Namespace {
    /// A namespace, refusing an empty world or campaign.
    ///
    /// Both are refused because an event with no world is unattributable and one with no
    /// campaign cannot be grouped — and an empty string is the shape that mistake arrives in.
    pub fn new(
        world: impl Into<String>,
        year: u64,
        season: Season,
        campaign: impl Into<String>,
    ) -> Result<Self, RefusalSet> {
        let world = world.into();
        let campaign = campaign.into();
        let mut refusals = RefusalSet::new();
        if world.trim().is_empty() {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                "an event with no world cannot be attributed to one",
            );
        }
        if campaign.trim().is_empty() {
            refusals.refuse(
                RefusalKind::MalformedIdentity,
                "an event with no campaign cannot be grouped. Note this is NOT a season: a season \
                 is the world's winter/spring/summer/autumn (H-10.5)",
            );
        }
        refusals.into_result(Namespace { world, year, season, campaign })
    }
}

/// Whether this event links to the one before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Linkage {
    /// The digest of the previous event. **The chain is hash-linked from here.**
    Linked(Digest),
    /// **The link was never computed.** The chain is append-only and *not*
    /// tamper-evident, and must never be called tamper-evident.
    NotEvaluable,
}

impl Linkage {
    /// The text form, which is what goes on disk and into a report.
    ///
    /// **And it is the exact inverse of [`Linkage::parse`], which it was not.**
    ///
    /// This used to return `digest.to_hex()` — **64 bare hex digits with no prefix** — while
    /// `parse` went through `Identity::parse`, which *refuses a bare digest with no prefix*.
    /// So the two were not inverses: **every linkage this project wrote was refused by its own
    /// reader** as `MalformedIdentity`. The bug was invisible for as long as every record said
    /// `not_evaluable`, because that word *does* round-trip.
    ///
    /// **It surfaced the moment a record carried a real digest**, which is what
    /// `filing::linked_records` does. That is the whole argument for a chokepoint that can
    /// produce a digest: a path nobody had taken until now was a path that did not work, and
    /// no test had been through it.
    ///
    /// **The prefix is added by asking `Identity`, not by hand**, so the render side and the
    /// parse side cannot drift apart again.
    pub fn render(self) -> String {
        match self {
            // **The prefix and the hex come from `identity`, which is the only place either is
            // defined** — `Identity::PREFIX` is what `Identity::parse` demands, and
            // `Digest::to_hex` is its documented body rendering. Writing either here would be
            // a second spelling, which is the bug this method just had.
            Linkage::Linked(digest) => {
                format!("{}{}", crate::identity::Identity::PREFIX, digest.to_hex())
            }
            Linkage::NotEvaluable => NOT_EVALUABLE.to_string(),
        }
    }

    /// Parse the text form, **refusing** anything that is neither.
    ///
    /// A reader that treats an unrecognised link as `NotEvaluable` would silently downgrade
    /// a broken chain to an unevaluated one — which is the exact confusion the honesty rule
    /// exists to prevent. So garbage is a refusal, not a state.
    ///
    /// **The exact inverse of [`Linkage::render`]**, which is a property worth asserting
    /// rather than assuming — see that method for the bug that made it false.
    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim();
        if trimmed == NOT_EVALUABLE {
            return Ok(Linkage::NotEvaluable);
        }
        crate::identity::Identity::parse(trimmed)
            .map(|identity| Linkage::Linked(identity.digest()))
            .map_err(|_| {
                format!(
                    "{trimmed:?} is neither a sha256: identity nor the literal {NOT_EVALUABLE:?}. \
                     Refusing rather than assuming: reading an unreadable link as \
                     {NOT_EVALUABLE} would downgrade a possibly-broken chain to an unevaluated one"
                )
            })
    }

    /// Whether this link was actually computed.
    pub const fn is_evaluated(self) -> bool {
        matches!(self, Linkage::Linked(_))
    }
}

/// What kind of thing happened. **Closed**, and deliberately small.
///
/// The record's requirement is that world events and development facts **share one chain**.
/// This enum is where that is expressed, and it is closed so a new kind is a decision rather
/// than a string a caller invents at a call site.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventKind {
    /// A world action: a dig, a build, a shaper run.
    WorldAction,
    /// A development fact: a check ran, a feature is missing, a gate refused.
    DevelopmentFact,
    /// A ticket changed state.
    TicketTransition,
    /// A ticket was retired — **a separate event kind, never a status flip** (see
    /// [`crate::ticket`]).
    Retirement,
    /// Something was refused. **A refusal is an event**, because a refusal nobody can read is
    /// a refusal that did not happen.
    RefusalRecorded,
    /// A milestone was filed on the plan.
    MilestoneFiled,
    /// A blocker was filed on the plan.
    BlockerFiled,
}

impl EventKind {
    pub const ALL: [EventKind; 7] = [
        EventKind::WorldAction,
        EventKind::DevelopmentFact,
        EventKind::TicketTransition,
        EventKind::Retirement,
        EventKind::RefusalRecorded,
        EventKind::MilestoneFiled,
        EventKind::BlockerFiled,
    ];

    /// A literal, for the same reason `FEATURE_COUNT` is: the pin's value is
    /// meaningless and its **type** is the assertion.
    pub const KIND_COUNT: usize = 7;

    pub const fn name(self) -> &'static str {
        match self {
            EventKind::WorldAction => "world_action",
            EventKind::DevelopmentFact => "development_fact",
            EventKind::TicketTransition => "ticket_transition",
            EventKind::Retirement => "retirement",
            EventKind::RefusalRecorded => "refusal_recorded",
            EventKind::MilestoneFiled => "milestone_filed",
            EventKind::BlockerFiled => "blocker_filed",
        }
    }

    pub fn parse(word: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == word.trim())
            .ok_or_else(|| {
                format!(
                    "unknown event kind {word:?}; this chain has {} kinds: {}",
                    Self::KIND_COUNT,
                    Self::ALL.map(|k| k.name()).join(", ")
                )
            })
    }
}

/// **Check 1 of 2.** Every kind has a row, in a `const`.
pub const KINDS_ARE_TOTAL: [(); EventKind::KIND_COUNT] = [(); EventKind::ALL.len()];

/// One event on the chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    /// Its position in the chain, from 0. **Identity is a position in an append-only
    /// record, never a counter that can be reused** — the reference's own lesson was that ids
    /// from a counter hand out an id that already exists.
    pub index: u64,
    pub namespace: Namespace,
    pub kind: EventKind,
    /// What happened, in prose. The record's own content; nothing is summarised away.
    pub detail: String,
    /// The link to the previous event. [`Linkage::NotEvaluable`] on the first event of an
    /// unlinked chain, and **permitted anywhere**, because a chain that was never linked is
    /// still append-only.
    pub previous: Linkage,
    /// The digest of this event's own bytes.
    pub digest: Digest,
}

impl Event {
    /// This event's digest, computed over its **canonical** fields.
    ///
    /// [`Digest::of_fields`] length-prefixes every field under a domain tag, which is what
    /// makes `("ab", "c")` and `("a", "bc")` different events. A digest that cannot tell two
    /// different things apart is a digest that will.
    pub fn compute_digest(
        index: u64,
        namespace: &Namespace,
        kind: EventKind,
        detail: &str,
        previous: Linkage,
    ) -> Digest {
        let year = index.to_le_bytes();
        let season = [namespace.season as u8];
        Digest::of_fields(
            CHAIN_DOMAIN,
            &[
                CHAIN_FORMAT.as_bytes(),
                &year,
                namespace.world.as_bytes(),
                &namespace.year.to_le_bytes(),
                &season,
                namespace.campaign.as_bytes(),
                kind.name().as_bytes(),
                detail.as_bytes(),
                previous.render().as_bytes(),
            ],
        )
    }
}

/// The canonical record. **Append-only, one writer, hash-chained.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chain {
    events: Vec<Event>,
    /// The linkage of the next event to be appended, or `NotEvaluable` for a chain that has
    /// never been linked.
    head: Linkage,
}

impl Default for Chain {
    /// [`Chain::new`] — an empty, unlinked chain.
    fn default() -> Self {
        Chain::new()
    }
}

impl Chain {
    /// An empty chain. Unlinked, and honest about it.
    pub fn new() -> Self {
        Chain {
            events: Vec::new(),
            head: Linkage::NotEvaluable,
        }
    }

    /// An empty chain whose head is already `head`.
    ///
    /// **The reader's counterpart to [`Chain::linked`], and it exists because a record on disk
    /// says what its own predecessor was.** A fold that always started at `Chain::new()` would
    /// carry head `NotEvaluable`, so the first record's claim could never be honoured and
    /// `is_tamper_event` would be false for every chain forever — which is exactly what
    /// happened before this existed.
    ///
    /// **A caller chooses the head, so the first record decides the chain's integrity**, and
    /// that choice is visible at the call site rather than defaulted. It is a reader of the
    /// first record's `previous` field and nothing else; it does not invent a linkage.
    pub fn with_head(head: Linkage) -> Self {
        Chain { events: Vec::new(), head }
    }

    /// An empty chain that will link its events from the first one.
    ///
    /// The difference from [`Chain::new`] is the difference between *append-only* and
    /// *tamper-evident*, and it is offered explicitly so the choice is visible at the call
    /// site rather than defaulted.
    pub fn linked() -> Self {
        Chain {
            events: Vec::new(),
            // The genesis link. `Digest::of(&[])` is a real digest of no bytes — not a
            // sentinel — so the first event links to something computable.
            head: Linkage::Linked(Digest::of(&[])),
        }
    }

    /// Append one event, or refuse and change nothing.
    ///
    /// `&mut self` **is** the one-writer guarantee at the type level: two processes cannot
    /// hold two `&mut Chain`. [`WriterLock`] carries the same guarantee across process
    /// boundaries, where the borrow checker cannot reach.
    ///
    /// An event with an empty `detail` is refused: an event that records nothing cannot be
    /// read back, and a chain of them is a file that grows while saying less.
    pub fn append(
        &mut self,
        namespace: Namespace,
        kind: EventKind,
        detail: impl Into<String>,
    ) -> Result<&Event, RefusalSet> {
        let detail = detail.into();
        let mut refusals = RefusalSet::new();

        if detail.trim().is_empty() {
            refusals.refuse(
                RefusalKind::NoReading,
                format!(
                    "a {} event with an empty detail cannot be read back, so it records nothing. \
                     Refusing rather than appending a blank line to an append-only record, which \
                     cannot be taken back",
                    kind.name()
                ),
            );
        }
        refusals.into_result(())?;

        let index = self.events.len() as u64;
        let digest = Event::compute_digest(index, &namespace, kind, &detail, self.head);
        let event = Event {
            index,
            namespace,
            kind,
            detail,
            previous: self.head,
            digest,
        };
        self.head = Linkage::Linked(digest);
        self.events.push(event);
        Ok(self.events.last().expect("just pushed"))
    }

    /// How many events. **Counted, not claimed.**
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// The digest of the whole chain, in order.
    ///
    /// **The whole file is the tamper boundary** (C-54.1), so this is a digest over every
    /// event in order rather than over the head alone. A head-only digest would let a
    /// rewritten prefix pass.
    ///
    /// **Recomputed from the events' content, never from their stored digests.** The first
    /// version folded `event.digest` — each event's *recorded* digest — and a test caught
    /// that this is worthless for its stated purpose: an event whose `detail` was rewritten
    /// in memory while its stored digest was left alone produced an **identical** chain
    /// digest, because the digest it folded was the stale one. A tamper boundary that a
    /// tamper does not move is not a boundary. So this recomputes each event's digest from
    /// its own fields, and a rewritten event changes the chain digest even if nothing else
    /// was touched. [`verify`] then also catches the mismatch on the event itself.
    pub fn digest(&self) -> Digest {
        // Owned buffers, because `event.index.to_le_bytes()` is a temporary and borrowing
        // it into the field list would outlive the statement.
        let mut fields: Vec<Vec<u8>> = vec![CHAIN_FORMAT.as_bytes().to_vec()];
        for event in &self.events {
            fields.push(event.index.to_le_bytes().to_vec());
            let recomputed = Event::compute_digest(
                event.index,
                &event.namespace,
                event.kind,
                &event.detail,
                event.previous,
            );
            fields.push(recomputed.as_bytes().to_vec());
        }
        let borrowed: Vec<&[u8]> = fields.iter().map(|field| field.as_slice()).collect();
        Digest::of_fields("shploingus/chain", &borrowed)
    }

    /// Whether this chain may be called tamper-evident.
    ///
    /// **`false` for a chain that was never linked**, and that is the whole honesty rule.
    /// A `NotEvaluable` chain is genuinely append-only — this process will not rewrite it —
    /// but nothing *verifies* that, so the word *tamper-evident* would be a claim about a
    /// check that never ran.
    pub fn is_tamper_evident(&self) -> bool {
        !self.events.is_empty() && self.events.iter().all(|event| event.previous.is_evaluated())
    }

    /// The sentence a caller should print about linkage. **Never a bare `false`.**
    pub fn tamper_evident_description(&self) -> String {
        if self.is_tamper_evident() {
            format!(
                "hash-linked: every event's previous_event_hash is the digest of the one before it, \
                 so a rewritten event is detectable across the whole chain ({} events)",
                self.events.len()
            )
        } else if self.events.is_empty() {
            "empty chain: nothing has been recorded, so no linkage exists to evaluate".to_string()
        } else {
            format!(
                "append-only but NOT tamper-evident: at least one event's previous_event_hash is \
                 {NOT_EVALUABLE:?}, so this state must never be described as tamper-evident \
                 ({} of {} events linked)",
                self.events.iter().filter(|e| e.previous.is_evaluated()).count(),
                self.events.len()
            )
        }
    }

    /// **The only read path.** Fold the chain; never load it whole.
    ///
    /// C-54.1 records the consequence as binding: *"the file grows without bound, so the read
    /// path must be designed for a long file from the start: a reader that folds the chain is
    /// fine; a reader that loads it whole is not."*
    ///
    /// So there is **no `events()` accessor** returning a slice, and a caller wanting one
    /// event must fold for it. [`Chain::digest`] and [`Chain::len`] are O(1)-ish and
    /// O(n)-without-copy respectively; nothing hands out the vector.
    pub fn fold<T, F>(&self, initial: T, mut step: F) -> T
    where
        F: FnMut(T, &Event) -> T,
    {
        let mut state = initial;
        for event in &self.events {
            state = step(state, event);
        }
        state
    }

    /// **Fold with the position alongside each event.**
    ///
    /// Split from [`Chain::fold`] rather than replacing it, because most callers want the
    /// event alone and a tuple everywhere would be noise. This exists for the checks that are
    /// *about* position — `verify`'s index check — so that the position an event claims and
    /// the position it holds are compared as two values rather than a counter maintained
    /// beside a loop.
    pub fn fold_indexed<T, F>(&self, initial: T, mut step: F) -> T
    where
        F: FnMut(T, &Event, u64) -> T,
    {
        let mut state = initial;
        for (index, event) in self.events.iter().enumerate() {
            state = step(state, event, index as u64);
        }
        state
    }

    /// **The first projection.** Render this chain as the record's Markdown.
    ///
    /// H-8.1: *"`docs/LEDGER.md` and the in-game ledger window are **two projections of one
    /// record**."* This is the Markdown side; the in-game side is the tickets/ledger window,
    /// which reads the same chain.
    ///
    /// **Derived, never stored as a second copy.** The rule that makes the two views unable to
    /// disagree is that neither is authored — both are functions of this chain.
    pub fn project(&self) -> String {
        let mut out = String::new();
        out.push_str("# LEDGER — decisions taken in shploingus-clean\n\n");
        out.push_str(&format!(
            "> **This file is a projection of `{}`.**\n\
             > It is derived from the chain by `voxel_model::chain::Chain::project` and is not \
             edited by hand; a disagreement between this file and the chain is a bug in the \
             projector, not a disagreement (H-8.1).\n\
             > chain digest: `{}` over {} events.\n\n",
            CHAIN_FORMAT,
            self.digest().to_hex(),
            self.events.len()
        ));
        let mut last_campaign = String::new();
        let mut last_season: Option<Season> = None;
        for event in &self.events {
            // **Campaigns and dates group; seasons never do** (H-10.5, C-49 ruling 8).
            if event.namespace.campaign != last_campaign {
                last_campaign = event.namespace.campaign.clone();
                out.push_str(&format!("\n## campaign: {last_campaign}\n\n"));
            }
            if last_season != Some(event.namespace.season) {
                last_season = Some(event.namespace.season);
                out.push_str(&format!("\n### {}\n\n", event.namespace.season.name()));
            }
            out.push_str(&format!(
                "- `{}` **{}** (world `{}`, year {}) — {}\n",
                event.index,
                event.kind.name(),
                event.namespace.world,
                event.namespace.year,
                event.detail
            ));
        }
        out
    }
}

/// **Check 2 of 2.** A chain with a bad link, or a fabricated digest, is refused.
pub fn verify(chain: &Chain) -> Result<(), RefusalSet> {
    let mut refusals = RefusalSet::new();
    let mut previous_digest: Option<Digest> = None;

    // The fold yields the **expected index** alongside each event, rather than the loop
    // carrying a counter beside it. Clippy caught the counter, and the fix is the better
    // shape: the position an event claims to occupy and the position it actually occupies
    // are then compared as values, which is what the check is about.
    for (event, expected_index) in chain.fold_indexed(Vec::new(), |mut acc, event, index| {
        acc.push((event.clone(), index));
        acc
    }) {
        if event.index != expected_index {
            refusals.refuse(
                RefusalKind::ChainBroken,
                format!(
                    "event {} claims index {} but sits at position {expected_index}. An \
                     append-only record's positions are its identity",
                    event.digest.to_hex(),
                    event.index
                ),
            );
        }

        // The link must name the previous event's digest.
        //
        // The **first** event is the case that needs care: a chain may legitimately begin
        // unlinked (`Chain::new`) or linked-to-genesis (`Chain::linked`), so neither is a
        // break, and asserting the genesis value here would refuse the first honest chain
        // above. From the second event on, the link must equal the previous event's digest,
        // and that is the check that detects a rewritten prefix.
        if let Linkage::Linked(claimed) = event.previous {
            if let Some(previous) = previous_digest {
                if claimed != previous {
                    refusals.refuse(
                        RefusalKind::ChainBroken,
                        format!(
                            "event {} links to {} but the event before it hashes to {}. **This is \
                             a fact about bytes and order, and not about permission** — a broken \
                             link says the record was rewritten, and says nothing about who may \
                             write",
                            event.index,
                            claimed.to_hex(),
                            previous.to_hex()
                        ),
                    );
                }
            } else {
                // The first event: the only two honest openings are unlinked and
                // linked-to-genesis. Anything else claims a predecessor that is not there.
                if claimed != Digest::of(&[]) {
                    refusals.refuse(
                        RefusalKind::ChainBroken,
                        format!(
                            "the first event links to {}, which is neither unlinked nor the \
                             genesis digest, so the chain claims a predecessor that is not there",
                            claimed.to_hex()
                        ),
                    );
                }
            }
        }

        // The event's own digest must be what its fields produce, or the event was edited
        // after it was written.
        let recomputed = Event::compute_digest(
            event.index,
            &event.namespace,
            event.kind,
            &event.detail,
            event.previous,
        );
        if recomputed != event.digest {
            refusals.refuse(
                RefusalKind::DigestMismatch,
                format!(
                    "event {} carries digest {} but its fields hash to {}. The event's bytes were \
                     changed after they were written",
                    event.index,
                    event.digest.to_hex(),
                    recomputed.to_hex()
                ),
            );
        }

        previous_digest = Some(event.digest);
    }

    refusals.into_result(())
}

/// The one-writer lock, held for as long as a process is appending.
///
/// C-54.1 names one writer as a **consequence** of "one chain for everything", not an
/// optional discipline, and the borrow checker only enforces it within a process. This is
/// the cross-process half, and [`WriterLock::acquire`] **refuses** rather than stealing a
/// held lock.
#[derive(Debug)]
pub struct WriterLock {
    path: std::path::PathBuf,
}

impl WriterLock {
    /// Take the lock for the chain at `path`, or refuse because someone holds it.
    ///
    /// Uses `create_new`, which is atomic: two processes racing here produce one winner and
    /// one refusal, and never two winners. A lock file that already exists is
    /// [`RefusalKind::ConcurrentWriter`] — **not** a lock to be broken on a timer, because a
    /// lock this code breaks is a lock this code cannot trust.
    pub fn acquire(path: impl Into<std::path::PathBuf>) -> Result<Self, RefusalSet> {
        use std::io::ErrorKind;
        let path = path.into();
        let mut refusals = RefusalSet::new();
        // Every arm is a **block**, so they all evaluate to `()`. Mixing a statement-arm
        // with an expression-arm is a type error the compiler caught here, and the shape is
        // kept uniform so the next refusal added to this match cannot reintroduce it.
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                use std::io::Write;
                let _ = writeln!(file, "pid {}", std::process::id());
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                refusals.refuse(
                    RefusalKind::ConcurrentWriter,
                    format!(
                        "{} exists, so another writer holds this chain. C-54.1 makes one writer a \
                         consequence of one chain for everything; this lock is not broken \
                         automatically, because a lock this code breaks is a lock this code \
                         cannot trust",
                        path.display()
                    ),
                );
            }
            Err(error) => {
                refusals.refuse(
                    RefusalKind::ConcurrentWriter,
                    format!("the writer lock at {} could not be taken: {error}", path.display()),
                );
            }
        }
        // **Check the refusals BEFORE constructing the lock.** The first version ended with
        // `refusals.into_result(WriterLock { path })`, which is a real bug and one the test
        // caught: `into_result` takes the value by value, so on the `Err` path the
        // `WriterLock` is **dropped**, its `Drop` runs, and it **deletes the lock file the
        // other writer still holds**. A refused acquire was releasing somebody else's lock —
        // which would have let two processes both believe they hold one chain, the exact
        // failure C-54.1 makes binding.
        //
        // So the value is only built once the set is empty, and the `Err` path constructs
        // nothing at all.
        if !refusals.is_empty() {
            return Err(refusals);
        }
        Ok(WriterLock { path })
    }

    /// Where the lock lives.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        // Removing our own lock on the way out. A lock left behind by a killed process is a
        // real problem, and it is **reported** by whoever finds it rather than aged out.
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ns() -> Namespace {
        Namespace::new("world-2468", 0, Season::Spring, "C-66").expect("a named namespace")
    }

    // ---- the honesty rule, which is the point ---------------------------

    #[test]
    fn an_unlinked_chain_is_never_called_tamper_evident() {
        let mut chain = Chain::new();
        chain
            .append(ns(), EventKind::DevelopmentFact, "a fact")
            .expect("one event");
        assert!(
            !chain.is_tamper_evident(),
            "an unlinked chain must not claim tamper-evidence"
        );
        let description = chain.tamper_evident_description();
        assert!(
            description.contains("NOT tamper-evident") && description.contains(NOT_EVALUABLE),
            "the description must say why, in words a reader can act on: {description}"
        );
    }

    #[test]
    fn a_linked_chain_is_tamper_evident_and_says_so() {
        let mut chain = Chain::linked();
        chain.append(ns(), EventKind::DevelopmentFact, "a").unwrap();
        chain.append(ns(), EventKind::WorldAction, "b").unwrap();
        assert!(chain.is_tamper_evident());
        assert!(chain.tamper_evident_description().contains("hash-linked"));
        verify(&chain).expect("a well-formed linked chain verifies");
    }

    #[test]
    fn an_empty_chain_is_neither_and_says_which() {
        let chain = Chain::new();
        assert!(!chain.is_tamper_evident());
        assert!(
            chain.tamper_evident_description().contains("empty chain"),
            "an empty chain must not be described as unlinked: {}",
            chain.tamper_evident_description()
        );
    }

    #[test]
    fn a_hash_is_never_called_permission() {
        // The record's rule, quoted in the module header, checked as a string: any future
        // edit that starts printing "verified" or "authorised" about a hash breaks here.
        let mut chain = Chain::linked();
        chain.append(ns(), EventKind::WorldAction, "dug here").unwrap();
        let text = chain.tamper_evident_description().to_lowercase();
        assert!(!text.contains("permission"));
        assert!(!text.contains("authoris"));
        assert!(!text.contains("authoriz"));
        assert!(!text.contains("trusted"));
    }

    // ---- the correction, as a type ---------------------------------------

    #[test]
    fn a_season_is_one_of_four_and_a_campaign_is_not_a_season() {
        assert_eq!(Season::ALL.len(), 4);
        for word in ["winter", "spring", "summer", "autumn"] {
            assert_eq!(Season::parse(word).map(|s| s.name()), Ok(word));
        }
        for word in ["campaign-3", "governance", "q1", "season-1", ""] {
            let error = Season::parse(word).expect_err("this is not a season");
            assert!(error.contains("H-10.5"), "the refusal cites the correction: {error}");
        }
    }

    #[test]
    fn a_season_follows_the_tick_and_the_constants_are_this_trees_own() {
        // 24_000 ticks/day, 96 days/year, so 24 days = 576_000 ticks per season.
        assert_eq!(Season::at_tick(0).unwrap(), Season::Winter);
        assert_eq!(Season::at_tick(24 * 24_000 - 1).unwrap(), Season::Winter);
        assert_eq!(Season::at_tick(24 * 24_000).unwrap(), Season::Spring);
        assert_eq!(Season::at_tick(48 * 24_000).unwrap(), Season::Summer);
        assert_eq!(Season::at_tick(72 * 24_000).unwrap(), Season::Autumn);
        // And it wraps on the year, deterministically.
        let year = 2_304_000u64;
        for tick in [0, 1, 123_456, year - 1, year, year * 7 + 5] {
            assert_eq!(
                Season::at_tick(tick),
                Season::at_tick(tick % year),
                "the season must depend only on the tick within the year"
            );
        }
    }

    #[test]
    fn a_namespace_with_no_world_or_no_campaign_is_refused() {
        assert!(Namespace::new("", 0, Season::Spring, "c").is_err());
        assert!(Namespace::new("w", 0, Season::Spring, "  ").is_err());
        assert!(Namespace::new("w", 0, Season::Spring, "c").is_ok());
    }

    // ---- the chain, and its refusals -------------------------------------

    #[test]
    fn an_empty_detail_is_refused_because_it_records_nothing() {
        let mut chain = Chain::new();
        let error = chain
            .append(ns(), EventKind::DevelopmentFact, "   ")
            .expect_err("a blank event records nothing");
        assert_eq!(error.all()[0].kind(), RefusalKind::NoReading);
        assert_eq!(chain.len(), 0, "a refused append changes nothing");
    }

    #[test]
    fn appending_is_deterministic_and_the_chain_digest_covers_every_event() {
        let build = || {
            let mut chain = Chain::linked();
            for i in 0..5 {
                chain
                    .append(ns(), EventKind::DevelopmentFact, format!("event {i}"))
                    .unwrap();
            }
            chain
        };
        let a = build();
        let b = build();
        assert_eq!(a, b, "the same events in the same order must produce the same chain");
        assert_eq!(a.digest(), b.digest());

        // A whole-chain digest, not a head digest: changing the *first* event changes it.
        let mut tampered = build();
        tampered.events[0].detail = "event 0 was rewritten".to_string();
        assert_ne!(
            a.digest(),
            tampered.digest(),
            "a whole-file digest must not be satisfiable by a matching tail"
        );
    }

    #[test]
    fn a_rewritten_event_is_detected_by_verify() {
        let mut chain = Chain::linked();
        chain.append(ns(), EventKind::WorldAction, "dug here").unwrap();
        chain.append(ns(), EventKind::WorldAction, "built there").unwrap();
        verify(&chain).expect("the honest chain verifies");

        chain.events[1].detail = "dug somewhere else entirely".to_string();
        let error = verify(&chain).expect_err("a rewritten event is detected");
        let kinds: Vec<RefusalKind> = error.all().iter().map(|r| r.kind()).collect();
        assert!(
            kinds.contains(&RefusalKind::DigestMismatch) || kinds.contains(&RefusalKind::ChainBroken),
            "a rewritten event must be caught: {error}"
        );
    }

    #[test]
    fn a_relinked_chain_is_detected_because_the_link_no_longer_matches() {
        let mut chain = Chain::linked();
        chain.append(ns(), EventKind::WorldAction, "first").unwrap();
        let good_link = chain.events[0].previous;
        chain.append(ns(), EventKind::WorldAction, "second").unwrap();
        verify(&chain).unwrap();

        // Point the first event somewhere else: its own digest is recomputed to match, so
        // only the *link* check can catch this.
        chain.events[0].previous = Linkage::Linked(Digest::of(b"a different genesis"));
        chain.events[0].digest = Event::compute_digest(
            0,
            &chain.events[0].namespace,
            chain.events[0].kind,
            &chain.events[0].detail,
            chain.events[0].previous,
        );
        let error = verify(&chain).expect_err("a forged genesis link is detected");
        assert!(
            error.all().iter().any(|r| r.kind() == RefusalKind::ChainBroken),
            "{error}"
        );
        assert_ne!(good_link, chain.events[0].previous);
    }

    #[test]
    fn every_event_kind_has_a_row_and_round_trips() {
        let _ = KINDS_ARE_TOTAL;
        assert_eq!(EventKind::ALL.len(), EventKind::KIND_COUNT);
        for kind in EventKind::ALL {
            assert_eq!(EventKind::parse(kind.name()), Ok(kind));
        }
        assert!(EventKind::parse("something_else").is_err());
    }

    #[test]
    fn a_link_parses_and_garbage_is_refused_rather_than_read_as_unevaluated() {
        let digest = Digest::of(b"an event");
        let text = format!("sha256:{}", digest.to_hex());
        assert_eq!(Linkage::parse(&text), Ok(Linkage::Linked(digest)));
        assert_eq!(Linkage::parse(NOT_EVALUABLE), Ok(Linkage::NotEvaluable));
        // The important one: unreadable is NOT unevaluated.
        let error = Linkage::parse("sha256:tooshort").expect_err("garbage is not a state");
        assert!(error.contains("Refusing rather than assuming"), "{error}");
    }

    // ---- one writer ------------------------------------------------------

    #[test]
    fn a_second_writer_is_refused_and_the_first_keeps_its_lock() {
        // A **process-unique** directory: this test runs alongside every other test in the
        // crate, and a path keyed only on the pid collided with a sibling test's lock
        // directory. Two tests sharing a lock path is the very race this module refuses to
        // allow, so the test must not manufacture one.
        let dir = std::env::temp_dir().join(format!(
            "shploingus-chain-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp dir");
        let path = dir.join("chain.lock");

        let first = WriterLock::acquire(&path).expect("the first writer takes the lock");
        let error = WriterLock::acquire(&path).expect_err("the second is refused");
        assert_eq!(error.all()[0].kind(), RefusalKind::ConcurrentWriter);
        assert!(path.exists(), "the refusal must not have stolen the held lock");
        assert_eq!(first.path(), path.as_path());
        drop(first);
        assert!(!path.exists(), "dropping releases the lock");
        // And the lock is takeable again afterwards.
        let _again = WriterLock::acquire(&path).expect("the lock is free once released");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- the two views ---------------------------------------------------

    #[test]
    fn the_projection_groups_by_campaign_and_never_by_season_as_a_period() {
        let mut chain = Chain::new();
        chain
            .append(
                Namespace::new("w", 0, Season::Winter, "campaign-one").unwrap(),
                EventKind::DevelopmentFact,
                "first fact",
            )
            .unwrap();
        chain
            .append(
                // **The same campaign, a different season** — the case H-10.5 is about.
                Namespace::new("w", 0, Season::Summer, "campaign-one").unwrap(),
                EventKind::WorldAction,
                "dug here",
            )
            .unwrap();
        let text = chain.project();
        assert!(text.contains("shploingus.chain/1"), "the projection names its format");
        assert!(text.contains("a projection of"), "and says it is derived");
        assert!(text.contains("campaign: campaign-one"));
        // One campaign heading, two season headings: the campaign groups, the season labels.
        assert_eq!(
            text.matches("## campaign: campaign-one").count(),
            1,
            "a campaign must be one group even across seasons:\n{text}"
        );
        assert!(text.contains("### winter") && text.contains("### summer"));
    }

    #[test]
    fn the_projection_is_a_function_of_the_chain_and_carries_its_digest() {
        let mut chain = Chain::linked();
        chain.append(ns(), EventKind::DevelopmentFact, "a fact").unwrap();
        assert_eq!(chain.project(), chain.project(), "the projection is deterministic");
        assert!(chain.project().contains(&chain.digest().to_hex()));
    }
}
