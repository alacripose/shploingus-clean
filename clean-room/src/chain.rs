//! **The chain — one namespaced, append-only event ledger, with its tickets and its wire form.**
//!
//! Derived from `docs/DESIGN-SPEC.md` §2 F1 (replay inputs are **recorded, not derived**; the world
//! and the record are one truth) and the record's ruling that there is **one** event chain, one
//! writer and two views (`LEDGER.md` H-8.1). The chain is the world model's memory: a modification
//! nobody recorded is invisible, and a record that can be rewritten is no record at all.
//!
//! # Append-only is a property, not a comment
//!
//! There is no method that edits, removes or reorders an event. Each event's digest covers its
//! sequence number, its namespace, its kind, its payload and the **previous** event's digest, so the
//! ledger is a hash chain: [`Chain::verify`] recomputes every link, and a tampered payload or a
//! broken link is refused by name ([`RefusalKind::ChainBroken`]). [`Chain::adopt`] is the only way a
//! caller can hand in a hand-built history, and it verifies before it trusts.
//!
//! # The namespace is a closed vocabulary
//!
//! An event names a [`Namespace`], and the set is **closed** — so an event with an unnamed
//! namespace is impossible to construct rather than refused at run time, which is the stronger
//! guarantee. The namespace rides the wire, so a parsed chain is identical to the one written.
//!
//! # Tickets are a **view** of the chain, never a second table
//!
//! A ticket is filed by appending a `TicketFiled` event and resolved by appending a
//! `TicketResolved` event; [`Chain::tickets`] derives the current state by reading the chain. So a
//! ticket cannot exist without an event behind it, and resolving a ticket twice is refused
//! ([`RefusalKind::TicketAlreadyResolved`]) because the chain already says it was resolved.
//!
//! # The wire form round-trips
//!
//! [`Chain::wire`] writes one line per event, tab-separated; [`Chain::from_wire`] parses it back and
//! **verifies**. A payload that would break the framing (a tab or a newline) is refused at append
//! time rather than escaped, because an escaping rule is a second format every reader must know.

use std::fmt;

use crate::digest::Digest;
use crate::refusal::{Refusal, RefusalKind, RefusalSet};

/// A ticket's identity, assigned by the chain in filing order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TicketId(pub u64);

impl fmt::Display for TicketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// The **closed** set of namespaces an event may belong to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Namespace {
    World,
    Tickets,
    Record,
}

impl Namespace {
    pub const ALL: [Namespace; 3] = [Namespace::World, Namespace::Tickets, Namespace::Record];

    pub const COUNT: usize = 3;

    pub const fn name(self) -> &'static str {
        match self {
            Namespace::World => "world",
            Namespace::Tickets => "tickets",
            Namespace::Record => "record",
        }
    }

    /// Parse, **refusing** anything else: an event in a namespace nobody declared is not an event
    /// this chain can read.
    pub fn parse(word: &str) -> Option<Self> {
        Namespace::ALL.iter().copied().find(|n| n.name() == word.trim())
    }
}

/// The recorded kinds of event. A closed set: an event kind nobody can name is a kind nobody meant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventKind {
    WorldOpened,
    TickAdvanced,
    RefusalRecorded,
    TicketFiled,
    TicketResolved,
    /// A derivation the API layer (U6) asked for and U4 performed: the payload names the canonical
    /// request and the geometry's own identity. It belongs **here**, at the record's owner — an API
    /// layer that invented its own event vocabulary would be a second record (SPEC-3).
    GeometryDerived,
    /// **A being acted** — U12's first mob (`src/being.rs`). One governed event per act a member of an
    /// organisation takes under its charter, carrying the being, the act, the cell and the stage the
    /// act's own record was read at. The kind lives here, at the record's owner, for the reason
    /// `GeometryDerived` does: a being that kept its acts in a ledger of its own would be a second
    /// record, and the row's own check says a member's acts go on the **one** chain.
    BeingActed,
}

impl EventKind {
    pub const ALL: [EventKind; 7] = [
        EventKind::WorldOpened,
        EventKind::TickAdvanced,
        EventKind::RefusalRecorded,
        EventKind::TicketFiled,
        EventKind::TicketResolved,
        EventKind::GeometryDerived,
        EventKind::BeingActed,
    ];

    pub const COUNT: usize = 7;

    pub const fn name(self) -> &'static str {
        match self {
            EventKind::WorldOpened => "world_opened",
            EventKind::TickAdvanced => "tick_advanced",
            EventKind::RefusalRecorded => "refusal_recorded",
            EventKind::TicketFiled => "ticket_filed",
            EventKind::TicketResolved => "ticket_resolved",
            EventKind::GeometryDerived => "geometry_derived",
            EventKind::BeingActed => "being_acted",
        }
    }

    /// Parse, **refusing** anything else. Used by the wire reader.
    pub fn parse(word: &str) -> Option<Self> {
        EventKind::ALL.iter().copied().find(|k| k.name() == word.trim())
    }
}

/// One event. Immutable once appended: its digest is fixed at construction and covers the link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    seq: u64,
    namespace: Namespace,
    kind: EventKind,
    payload: String,
    prev: Option<Digest>,
    digest: Digest,
}

impl Event {
    /// **The one place a digest is computed**, so a reader and a writer cannot disagree.
    fn compute(
        seq: u64,
        namespace: Namespace,
        kind: EventKind,
        payload: &str,
        prev: Option<Digest>,
    ) -> Digest {
        let prev_bytes: Vec<u8> = match prev {
            Some(digest) => digest.as_bytes().to_vec(),
            None => b"genesis".to_vec(),
        };
        Digest::of_parts(&[
            b"shploingus.event/1",
            &seq.to_le_bytes(),
            namespace.name().as_bytes(),
            kind.name().as_bytes(),
            payload.as_bytes(),
            &prev_bytes,
        ])
    }

    pub fn new(
        seq: u64,
        namespace: Namespace,
        kind: EventKind,
        payload: String,
        prev: Option<Digest>,
    ) -> Self {
        let digest = Event::compute(seq, namespace, kind, &payload, prev);
        Event { seq, namespace, kind, payload, prev, digest }
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn namespace(&self) -> Namespace {
        self.namespace
    }

    pub fn kind(&self) -> EventKind {
        self.kind
    }

    pub fn payload(&self) -> &str {
        &self.payload
    }

    pub fn prev(&self) -> Option<Digest> {
        self.prev
    }

    pub fn digest(&self) -> Digest {
        self.digest
    }

    /// Whether this event's stored digest is the one its fields imply.
    pub fn links_correctly(&self) -> bool {
        self.digest == Event::compute(self.seq, self.namespace, self.kind, &self.payload, self.prev)
    }
}

/// The current state of one ticket, derived from the chain's events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub id: TicketId,
    pub subject: String,
    pub filed_seq: u64,
    pub resolved_seq: Option<u64>,
}

/// **The one place an event's meaning is checked**, shared by the writer (`append`), the wire
/// reader (`from_wire`) and the adopting reader (`adopt`), so none of the three can accept what
/// another refuses. `history` is every event before the candidate.
fn check_semantics(history: &[Event], candidate: &Event) -> Result<(), Refusal> {
    if candidate.kind == EventKind::TicketResolved {
        let Ok(id) = candidate.payload.parse::<u64>() else {
            return Err(Refusal::new(
                RefusalKind::UnknownTicket,
                format!("{:?} is not a ticket number", candidate.payload),
            ));
        };
        let already = history.iter().any(|event| {
            event.kind == EventKind::TicketResolved
                && event.payload.parse::<u64>().is_ok_and(|resolved| resolved == id)
        });
        if already {
            return Err(Refusal::new(
                RefusalKind::TicketAlreadyResolved,
                format!("ticket #{id} is already resolved"),
            ));
        }
        let filed = history
            .iter()
            .filter(|event| event.kind == EventKind::TicketFiled)
            .count();
        if id >= filed as u64 {
            return Err(Refusal::new(
                RefusalKind::UnknownTicket,
                format!("ticket #{id} was never filed"),
            ));
        }
    }
    Ok(())
}

/// The one append-only ledger.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Chain {
    events: Vec<Event>,
}

impl Chain {
    pub const fn new() -> Self {
        Chain { events: Vec::new() }
    }

    /// Append one event to the head. Refuses a payload that cannot survive the wire form, **by
    /// name** — a payload with a tab or a newline is refused rather than escaped, because escaping
    /// would be a second format — and refuses an event whose meaning the chain cannot represent (a
    /// resolution that names no filed ticket), so the writer and the reader agree.
    pub fn append(
        &mut self,
        namespace: Namespace,
        kind: EventKind,
        payload: impl Into<String>,
    ) -> Result<&Event, Refusal> {
        let payload = payload.into();
        if payload.contains('\n') || payload.contains('\t') {
            return Err(Refusal::new(
                RefusalKind::PayloadNotWirable,
                format!(
                    "payload {:?} contains a tab or a newline, which the wire form frames on",
                    payload
                ),
            ));
        }
        let seq = self.events.len() as u64;
        let prev = self.head();
        let event = Event::new(seq, namespace, kind, payload, prev);
        check_semantics(&self.events, &event)?;
        self.events.push(event);
        Ok(self.events.last().expect("just pushed"))
    }

    pub fn head(&self) -> Option<Digest> {
        self.events.last().map(|event| event.digest)
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Re-walk the whole chain: every sequence number in order, every link to the head so far, and
    /// every digest the one its fields imply. **Every** fault is collected, not the first.
    pub fn verify(&self) -> Result<(), RefusalSet> {
        let mut set = RefusalSet::new();
        let mut expected_prev: Option<Digest> = None;
        for (index, event) in self.events.iter().enumerate() {
            if event.seq != index as u64 {
                set.refuse(
                    RefusalKind::ChainBroken,
                    format!("event at index {index} claims seq {}", event.seq),
                );
            }
            if event.prev != expected_prev {
                set.refuse(
                    RefusalKind::ChainBroken,
                    format!("event {} does not link to the event before it", event.seq),
                );
            }
            if !event.links_correctly() {
                set.refuse(
                    RefusalKind::ChainBroken,
                    format!("event {}'s digest is not the one its fields imply", event.seq),
                );
            }
            expected_prev = Some(event.digest);
        }
        set.into_result(())
    }

    /// The wire form: one line per event, tab-separated `seq`, `namespace`, `kind`, `prev` (`-` for
    /// genesis), `digest` (hex) and payload.
    pub fn wire(&self) -> String {
        let mut out = String::new();
        for event in &self.events {
            let prev = event
                .prev
                .map(|digest| digest.to_hex())
                .unwrap_or_else(|| "-".to_string());
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                event.seq,
                event.namespace.name(),
                event.kind.name(),
                prev,
                event.digest.to_hex(),
                event.payload
            ));
        }
        out
    }

    /// Parse a wire form and **verify** it. A malformed line is refused `MalformedWire`; a tampered
    /// event is refused `ChainBroken`; and an event the writer could never have made is refused by
    /// its own name (see [`check_semantics`]).
    pub fn from_wire(text: &str) -> Result<Chain, RefusalSet> {
        let mut set = RefusalSet::new();
        let mut events = Vec::new();
        for (line_number, line) in text.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            // Six fields exactly: a payload must not swallow a tab, because a payload with one is
            // exactly what the writer refuses to emit.
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != 6 {
                set.refuse(
                    RefusalKind::MalformedWire,
                    format!("line {} has {} fields, not 6", line_number + 1, fields.len()),
                );
                continue;
            }
            let seq = match fields[0].parse::<u64>() {
                Ok(seq) => seq,
                Err(_) => {
                    set.refuse(
                        RefusalKind::MalformedWire,
                        format!("line {}: {:?} is not a sequence number", line_number + 1, fields[0]),
                    );
                    continue;
                }
            };
            let namespace = match Namespace::parse(fields[1]) {
                Some(namespace) => namespace,
                None => {
                    set.refuse(
                        RefusalKind::MalformedWire,
                        format!("line {}: unknown namespace {:?}", line_number + 1, fields[1]),
                    );
                    continue;
                }
            };
            let kind = match EventKind::parse(fields[2]) {
                Some(kind) => kind,
                None => {
                    set.refuse(
                        RefusalKind::MalformedWire,
                        format!("line {}: unknown event kind {:?}", line_number + 1, fields[2]),
                    );
                    continue;
                }
            };
            let prev = if fields[3] == "-" {
                None
            } else {
                match Digest::from_hex(fields[3]) {
                    Some(digest) => Some(digest),
                    None => {
                        set.refuse(
                            RefusalKind::MalformedWire,
                            format!("line {}: {:?} is not a digest", line_number + 1, fields[3]),
                        );
                        continue;
                    }
                }
            };
            let stored = match Digest::from_hex(fields[4]) {
                Some(digest) => digest,
                None => {
                    set.refuse(
                        RefusalKind::MalformedWire,
                        format!("line {}: {:?} is not a digest", line_number + 1, fields[4]),
                    );
                    continue;
                }
            };
            // **The stored digest must be the one the fields imply.** The digest is recomputed in
            // one place, then compared with what the wire claimed — which is what makes a tampered
            // payload detectable rather than silently re-blessed.
            let event = Event::new(seq, namespace, kind, fields[5].to_string(), prev);
            if event.digest() != stored {
                set.refuse(
                    RefusalKind::ChainBroken,
                    format!(
                        "line {}: event {seq}'s stored digest is not the one its fields imply",
                        line_number + 1
                    ),
                );
                continue;
            }
            if let Err(refusal) = check_semantics(&events, &event) {
                set.push(refusal);
                continue;
            }
            events.push(event);
        }
        if !set.is_empty() {
            return Err(set);
        }
        Chain::adopt(events)
    }

    /// Adopt a hand-built history, but only if it verifies **and** every event means something: a
    /// wrong link or digest is refused `ChainBroken`, and an event the writer could not have made
    /// (a resolution that names no filed ticket) is refused by its own name. This is the illegal
    /// append of a *reader*.
    pub fn adopt(events: Vec<Event>) -> Result<Chain, RefusalSet> {
        let chain = Chain { events };
        chain.verify()?;
        let mut meaning = RefusalSet::new();
        for index in 0..chain.events.len() {
            if let Err(refusal) = check_semantics(&chain.events[..index], &chain.events[index]) {
                meaning.push(refusal);
            }
        }
        meaning.into_result(chain)
    }

    /// File a ticket: append a `TicketFiled` event and return the id the chain assigned.
    pub fn file_ticket(&mut self, subject: impl Into<String>) -> Result<TicketId, Refusal> {
        let subject = subject.into();
        let id = TicketId(self.tickets().len() as u64);
        self.append(Namespace::Tickets, EventKind::TicketFiled, subject)?;
        Ok(id)
    }

    /// Resolve a ticket. Refuses an unknown id and a ticket already resolved — the chain is the
    /// state, so a second resolution is a statement the record already makes. The checks live in
    /// [`check_semantics`], so this and the raw [`Chain::append`] cannot disagree.
    pub fn resolve_ticket(&mut self, id: TicketId) -> Result<(), Refusal> {
        // The payload is the **number**, not the ticket's display form: a reader parses it, so
        // writing `#0` here would make every resolution unreadable (a real bug the tests caught).
        self.append(Namespace::Tickets, EventKind::TicketResolved, id.0.to_string())?;
        Ok(())
    }

    /// The tickets, derived by reading the chain — never a second table.
    pub fn tickets(&self) -> Vec<Ticket> {
        let mut tickets: Vec<Ticket> = Vec::new();
        for event in &self.events {
            match event.kind {
                EventKind::TicketFiled => tickets.push(Ticket {
                    id: TicketId(tickets.len() as u64),
                    subject: event.payload.clone(),
                    filed_seq: event.seq,
                    resolved_seq: None,
                }),
                EventKind::TicketResolved => {
                    if let Ok(id) = event.payload.parse::<u64>() {
                        if let Some(ticket) = tickets.get_mut(id as usize) {
                            ticket.resolved_seq = Some(event.seq);
                        }
                    }
                }
                _ => {}
            }
        }
        tickets
    }

    /// The un-resolved tickets.
    pub fn open_tickets(&self) -> Vec<TicketId> {
        self.tickets()
            .into_iter()
            .filter(|ticket| ticket.resolved_seq.is_none())
            .map(|ticket| ticket.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_chain() -> Chain {
        let mut chain = Chain::new();
        chain.append(Namespace::World, EventKind::WorldOpened, "seed=2468").unwrap();
        chain.append(Namespace::World, EventKind::TickAdvanced, "tick=1").unwrap();
        chain.file_ticket("the spawn surface reads as one pale material").unwrap();
        chain
    }

    #[test]
    fn an_event_links_to_the_head_and_the_chain_verifies() {
        let chain = a_chain();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain.events()[0].prev(), None, "the first event is the genesis");
        assert_eq!(chain.events()[1].prev(), Some(chain.events()[0].digest()));
        assert_eq!(chain.events()[2].prev(), Some(chain.events()[1].digest()));
        assert_eq!(chain.head(), Some(chain.events()[2].digest()));
        chain.verify().expect("a freshly appended chain verifies");
        assert_eq!(chain.events()[1].namespace(), Namespace::World);
        assert_eq!(chain.events()[2].namespace(), Namespace::Tickets);
    }

    #[test]
    fn a_payload_that_cannot_ride_the_wire_is_refused() {
        let mut chain = Chain::new();
        let err = chain
            .append(Namespace::World, EventKind::WorldOpened, "two\nlines")
            .unwrap_err();
        assert_eq!(err.kind(), RefusalKind::PayloadNotWirable);
        let err = chain
            .append(Namespace::World, EventKind::WorldOpened, "a\tb")
            .unwrap_err();
        assert_eq!(err.kind(), RefusalKind::PayloadNotWirable);
        assert!(chain.is_empty(), "a refused append changes nothing");
    }

    #[test]
    fn the_wire_form_round_trips_exactly_and_is_deterministic() {
        let mut chain = a_chain();
        chain.resolve_ticket(TicketId(0)).unwrap();

        let wire = chain.wire();
        let parsed = Chain::from_wire(&wire).expect("the wire form parses");
        // **Exact**: including the namespace, so the head and every digest agree.
        assert_eq!(parsed, chain);
        assert_eq!(parsed.head(), chain.head());
        parsed.verify().expect("the parsed chain verifies");
        assert_eq!(parsed.wire(), wire);

        // Determinism: the same sequence of appends yields the same digests.
        let mut again = Chain::new();
        again.append(Namespace::World, EventKind::WorldOpened, "seed=2468").unwrap();
        again.append(Namespace::World, EventKind::TickAdvanced, "tick=1").unwrap();
        again.file_ticket("the spawn surface reads as one pale material").unwrap();
        again.resolve_ticket(TicketId(0)).unwrap();
        assert_eq!(again, chain, "same events, same digests");
    }

    #[test]
    fn a_tampered_wire_is_refused_by_name() {
        let mut chain = a_chain();
        chain.append(Namespace::World, EventKind::TickAdvanced, "tick=2").unwrap();
        let wire = chain.wire();

        // Change one payload character: the recomputed digest will not match the stored one.
        let tampered = wire.replacen("tick=2", "tick=9", 1);
        let err = Chain::from_wire(&tampered).unwrap_err();
        assert!(err.contains(RefusalKind::ChainBroken), "tampering must be caught: {err}");

        // A malformed line (too few fields) is refused `MalformedWire`.
        let broken = format!("0\tworld\tworld_opened\n{wire}");
        let err = Chain::from_wire(&broken).unwrap_err();
        assert!(err.contains(RefusalKind::MalformedWire));

        // An unknown namespace is refused rather than guessed.
        let unknown = wire.replacen("\tworld\t", "\telsewhere\t", 1);
        let err = Chain::from_wire(&unknown).unwrap_err();
        assert!(err.contains(RefusalKind::MalformedWire));

        // Adopting a hand-built event with the wrong link is refused.
        let forged = Event::new(1, Namespace::World, EventKind::TickAdvanced, "tick=1".into(), None);
        let err = Chain::adopt(vec![forged]).unwrap_err();
        assert!(err.contains(RefusalKind::ChainBroken));
    }

    #[test]
    fn a_ticket_is_filed_and_resolved_once_and_the_chain_is_the_state() {
        let mut chain = Chain::new();
        let id = chain.file_ticket("the world reads pale").unwrap();
        assert_eq!(id, TicketId(0));
        assert_eq!(chain.open_tickets(), vec![TicketId(0)]);
        assert_eq!(chain.tickets()[0].subject, "the world reads pale");

        chain.resolve_ticket(id).unwrap();
        assert!(chain.open_tickets().is_empty(), "it is resolved now");
        assert!(chain.tickets()[0].resolved_seq.is_some());

        // A second resolution is refused: the record already says it happened.
        let err = chain.resolve_ticket(id).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::TicketAlreadyResolved);

        // An unknown id is refused.
        let err = chain.resolve_ticket(TicketId(99)).unwrap_err();
        assert_eq!(err.kind(), RefusalKind::UnknownTicket);

        // Two tickets, one resolved, one open.
        let second = chain.file_ticket("a second thing").unwrap();
        assert_eq!(second, TicketId(1));
        assert_eq!(chain.open_tickets(), vec![TicketId(1)]);
        chain.verify().expect("still an intact chain");
    }

    #[test]
    fn a_resolution_that_names_no_ticket_is_refused_at_append() {
        let mut chain = Chain::new();
        let err = chain
            .append(Namespace::Tickets, EventKind::TicketResolved, "99")
            .unwrap_err();
        assert_eq!(err.kind(), RefusalKind::UnknownTicket);
        let err = chain
            .append(Namespace::Tickets, EventKind::TicketResolved, "not-a-number")
            .unwrap_err();
        assert_eq!(err.kind(), RefusalKind::UnknownTicket);
        assert!(chain.is_empty(), "a refused append changes nothing");

        // Through the raw layer, resolving twice is the same refusal as through the ticket view.
        let id = chain.file_ticket("a thing").unwrap();
        chain.resolve_ticket(id).unwrap();
        let err = chain
            .append(Namespace::Tickets, EventKind::TicketResolved, id.0.to_string())
            .unwrap_err();
        assert_eq!(err.kind(), RefusalKind::TicketAlreadyResolved);

        // A hand-built history with the same bogus event is refused by the adopting reader too:
        // the meaning check is shared, so `adopt` cannot accept what `append` refuses.
        let bogus = Event::new(
            0,
            Namespace::Tickets,
            EventKind::TicketResolved,
            "99".to_string(),
            None,
        );
        let err = Chain::adopt(vec![bogus]).unwrap_err();
        assert!(err.contains(RefusalKind::UnknownTicket), "{err}");
    }

    #[test]
    fn the_wire_reader_refuses_what_the_writer_refuses() {
        // A tab in the payload cannot ride the wire (the writer refuses it); a hand-built line that
        // carries one is refused by the reader too.
        let tabbed = Event::new(0, Namespace::World, EventKind::WorldOpened, "a\tb".to_string(), None);
        let line = format!("0\tworld\tworld_opened\t-\t{}\ta\tb\n", tabbed.digest().to_hex());
        let err = Chain::from_wire(&line).unwrap_err();
        assert!(err.contains(RefusalKind::MalformedWire), "{err}");

        // And a resolution the writer could never make is refused on read as well.
        let bogus = Event::new(0, Namespace::Tickets, EventKind::TicketResolved, "99".to_string(), None);
        let line = format!("0\ttickets\tticket_resolved\t-\t{}\t99\n", bogus.digest().to_hex());
        let err = Chain::from_wire(&line).unwrap_err();
        assert!(err.contains(RefusalKind::UnknownTicket), "{err}");
    }
}
