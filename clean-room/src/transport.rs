//! **The transport — the way in, and nothing more.**
//!
//! Derived from the **U6** entry of `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 — *"**`bridge::TOOLS`** —
//! one bijective tool table over the action vocabulary, with `bridge::call` returning a `Receipt`
//! naming **which checks ran**"* — and §5's slice 5, *"the record/gate with `bridge::TOOLS` served over
//! stdio"*. Nothing under `crates/voxel-mcp/` was read (P7); the legacy transport is a witness that the
//! tool bridge is *served*, not a source for how.
//!
//! # What this module owns, and what it must never own
//!
//! It owns **framing**: one request line in, one response record out. It owns no truth, no record and
//! no geometry — it borrows a [`Gate`] and hands it lines, and every answer it writes is the receipt
//! the gate produced. A transport that could answer on its own would be a second owner, so the type
//! signatures make that impossible: [`serve`] takes `&mut Gate` and has nothing else to answer from.
//!
//! # The declared line protocol
//!
//! A request is **one line** whose grammar is the table's own canonical line, and a response is a
//! bounded group of `key ⇥ value` lines closed by a blank line:
//!
//! ```text
//! status  ⇥ ok | err
//! request ⇥ <the line as received>            (- when the line could not be framed)
//! record  ⇥ <events>:<head hex | ->           the record **after** the call
//! receipt ⇥ <receipt wire>                    ok only — the whole receipt, one field
//! kind    ⇥ <refusal kind name>               err only
//! detail  ⇥ <the refusal's own words>         err only
//! <blank line>                                end of response
//! ```
//!
//! A request line carries none of the characters the response's framing uses — tab, newline, carriage
//! return, and the receipt wire's `~`, `;` and `|` — because a line that cannot cross intact is
//! refused (`payload_not_wirable`) rather than silently reshaped. That refusal is the transport's
//! **own** and writes nothing: the table never sees the line, so it is not an event about the world.
//!
//! Everything else is the gate's: an undeclared verb, an argument fault, a level off the ladder, a
//! chunk outside the residency horizon — each is refused by the name its owner gave it, and the
//! world-level ones are recorded in the chain, which is why every response carries the record's length
//! and head: a client can see that a refusal was recorded **and** that a line that did not parse was
//! not, which is the discipline the table has in-process.
//!
//! # What this module deliberately does not know
//!
//! **Which world a caller serves.** A session is `serve(&mut Gate, …)`: the gate arrives built, so the
//! framing never learns a palette, a horizon or an identity. The one reference declaration two
//! processes compare answers through lives in [`crate::reference`], and the characters below are byte
//! for byte the ones [`crate::bridge::wire`] separates its parts with — read from there, never
//! respelled here.

use std::fmt;
use std::io::{BufRead, Write};

use crate::bridge::{parse, Gate, Receipt};
use crate::digest::Digest;
use crate::refusal::{Refusal, RefusalKind};

/// The characters a request line may not carry: the tab that separates a response's fields, the
/// newline that ends them, and the three the **receipt's own wire form** separates its parts with.
/// Those three are taken from [`crate::bridge::wire`], where that format is declared, so the framing
/// cannot drift from the format it has to be able to quote. A comma is deliberately absent: `chunk=
/// 0,0,0` is a request.
pub const REQUEST_FORBIDDEN: [char; 6] = [
    '\t',
    '\n',
    '\r',
    crate::bridge::WIRE_PART,
    crate::bridge::WIRE_ITEM,
    crate::bridge::WIRE_CHECK_FIELD,
];

/// Refuse a request line whose characters cannot cross: the response's framing uses them, so a line
/// carrying one is refused by name rather than reshaped.
pub fn frame_request(line: &str) -> Result<(), Refusal> {
    for forbidden in REQUEST_FORBIDDEN {
        if line.contains(forbidden) {
            return Err(Refusal::new(
                RefusalKind::PayloadNotWirable,
                format!(
                    "the request carries {:?}, which the response's framing uses: it cannot cross \
                     intact, and a line that cannot cross is refused rather than reshaped",
                    forbidden
                ),
            ));
        }
    }
    Ok(())
}

/// One framed request, ready to write: the line and its terminator, or the refusal that it cannot
/// cross.
pub fn request(line: &str) -> Result<String, Refusal> {
    frame_request(line)?;
    Ok(format!("{line}\n"))
}

/// Send one request and flush it, so a server waiting on input sees it rather than a buffer.
pub fn send(output: &mut impl Write, line: &str) -> Result<(), Refusal> {
    let framed = request(line)?;
    output
        .write_all(framed.as_bytes())
        .and_then(|()| output.flush())
        .map_err(|error| {
            Refusal::new(
                RefusalKind::MalformedWire,
                format!("the request could not be written: {error}"),
            )
        })
}

/// What a session did: how many lines crossed, and how they were answered. Transport bookkeeping,
/// never world state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Session {
    pub requests: usize,
    pub answered: usize,
    pub refused: usize,
}

/// The record as a response reports it: how many events it holds, and its head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServedRecord {
    pub events: usize,
    pub head: Option<Digest>,
}

impl ServedRecord {
    fn wire(self) -> String {
        format!(
            "{}:{}",
            self.events,
            self.head
                .map(|digest| digest.to_hex())
                .unwrap_or_else(|| "-".to_string())
        )
    }

    fn from_wire(text: &str) -> Result<Self, Refusal> {
        let (events, head) = text
            .split_once(':')
            .ok_or_else(|| malformed(format!("`{text}` is not `<events>:<head>`")))?;
        let events = events
            .parse::<usize>()
            .map_err(|_| malformed(format!("`{events}` is not an event count")))?;
        let head = if head == "-" {
            None
        } else {
            Some(
                Digest::from_hex(head)
                    .ok_or_else(|| malformed(format!("`{head}` is neither a digest nor `-`")))?,
            )
        };
        Ok(ServedRecord { events, head })
    }
}

fn malformed(detail: impl fmt::Display) -> Refusal {
    Refusal::new(RefusalKind::MalformedWire, format!("a response: {detail}"))
}

fn record_of(gate: &Gate<'_>) -> ServedRecord {
    ServedRecord { events: gate.chain().len(), head: gate.chain().head() }
}

/// Refuse a value the response's own framing cannot carry. Interiors such as `~` are the receipt
/// wire's business, and its round trip is what verifies them.
fn frame_value(text: &str) -> Result<&str, Refusal> {
    if text.contains('\t') || text.contains('\n') || text.contains('\r') {
        return Err(Refusal::new(
            RefusalKind::PayloadNotWirable,
            "a response field carries a tab or a newline, which the response frames on",
        ));
    }
    Ok(text)
}

fn write_lines(output: &mut impl Write, lines: &[String]) -> Result<(), Refusal> {
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    text.push('\n');
    output.write_all(text.as_bytes()).and_then(|()| output.flush()).map_err(
        |error| malformed(format!("the response could not be written: {error}")),
    )
}

fn write_ok(
    output: &mut impl Write,
    echo: &str,
    receipt: &Receipt,
    record: ServedRecord,
) -> Result<(), Refusal> {
    let lines = vec![
        "status\tok".to_string(),
        format!("request\t{}", frame_value(echo)?),
        format!("record\t{}", record.wire()),
        format!("receipt\t{}", frame_value(&receipt.wire())?),
    ];
    write_lines(output, &lines)
}

fn write_error(
    output: &mut impl Write,
    echo: &str,
    kind: &str,
    detail: &str,
    record: ServedRecord,
) -> Result<(), Refusal> {
    // The echo is `-` when the received line itself could not be framed: there is nothing this
    // response could quote without breaking its own framing.
    let echo = frame_value(echo).unwrap_or("-");
    let lines = vec![
        "status\terr".to_string(),
        format!("request\t{echo}"),
        format!("record\t{}", record.wire()),
        format!("kind\t{}", frame_value(kind)?),
        format!("detail\t{}", frame_value(detail)?),
    ];
    write_lines(output, &lines)
}

/// **Serve one session**: read request lines, hand each to the gate, write one response per request
/// until the input ends. The gate is the only thing that can answer — this function has no other
/// source — and nothing is written for a line the transport itself refuses.
pub fn serve(
    gate: &mut Gate<'_>,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<Session, Refusal> {
    let mut session = Session::default();
    let mut line = String::new();
    loop {
        line.clear();
        let read = input.read_line(&mut line).map_err(|error| {
            malformed(format!("the request stream could not be read: {error}"))
        })?;
        if read == 0 {
            break;
        }
        let received = line.trim_end_matches(['\n', '\r']);
        session.requests += 1;
        if let Err(refusal) = frame_request(received) {
            session.refused += 1;
            // The transport's own refusal never reaches the gate, so the record is unchanged by it —
            // and the response says so by carrying the record as it stands. The echo is `-`: the line
            // is exactly the one the transport could not frame, so there is nothing it may quote.
            write_error(
                output,
                "-",
                refusal.kind().name(),
                refusal.detail(),
                record_of(gate),
            )?;
            continue;
        }
        match gate.call(received) {
            Ok(receipt) => {
                session.answered += 1;
                write_ok(output, received, &receipt, record_of(gate))?;
            }
            Err(refusal) => {
                session.refused += 1;
                // The record **after** the call: a world-level refusal is recorded by the gate, and a
                // client must be able to see that it is — otherwise a served session would hide the
                // difference between a refusal that counts and a line that does not parse.
                write_error(
                    output,
                    received,
                    refusal.kind().name(),
                    refusal.detail(),
                    record_of(gate),
                )?;
            }
        }
    }
    Ok(session)
}

/// One response, as a client reads it: the receipt the server handed out, or the refusal it named —
/// with the record's state either way, which is how a client sees that a refusal was recorded.
#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    /// Boxed: a `Receipt` is far larger than a refusal, and responses move between the reader and the
    /// caller — the box keeps the common value small without hiding it.
    Ok { receipt: Box<Receipt>, record: ServedRecord },
    /// The refusal, **and the line it is about**: the echo is kept rather than discarded, so a client
    /// can check that what it read answers what it sent. `-` is the transport's own echo, used when
    /// the line could not be framed at all.
    Err { request: String, kind: String, detail: String, record: ServedRecord },
}

impl Response {
    pub fn record(&self) -> ServedRecord {
        match self {
            Response::Ok { record, .. } | Response::Err { record, .. } => *record,
        }
    }

    /// The receipt, or the refusal's own words — the two things a caller acts on.
    pub fn receipt(&self) -> Option<&Receipt> {
        match self {
            Response::Ok { receipt, .. } => Some(&**receipt),
            Response::Err { .. } => None,
        }
    }

    /// The refusal's **name**, which is the vocabulary a caller matches on.
    pub fn refusal_kind(&self) -> Option<&str> {
        match self {
            Response::Ok { .. } => None,
            Response::Err { kind, .. } => Some(kind),
        }
    }

    /// **The line this response is about.** An `ok` answer is anchored to the receipt's own canonical
    /// request; a refusal quotes the line as it was received (or `-`, when the transport could not
    /// frame it). Either way a caller can compare it with what it sent.
    pub fn request(&self) -> &str {
        match self {
            Response::Ok { receipt, .. } => &receipt.request,
            Response::Err { request, .. } => request,
        }
    }

    /// Is this response an answer to `line`? An `ok` answer is about the table's **canonical** request,
    /// which is `line` itself or its canonical spelling; a refusal is about the line **as received**,
    /// so it must quote `line` exactly. Nothing else counts — an answer about a different line is not
    /// an answer to this one, however well-formed it is.
    pub fn is_about(&self, line: &str) -> bool {
        match self {
            Response::Ok { receipt, .. } => {
                receipt.request == line
                    || parse(line)
                        .map(|parsed| parsed.canonical() == receipt.request)
                        .unwrap_or(false)
            }
            Response::Err { request, .. } => request == line,
        }
    }
}

/// Read one response, or `None` when the stream ends cleanly between responses. A stream that ends in
/// the middle of a response is refused rather than truncated.
pub fn read_response(input: &mut impl BufRead) -> Result<Option<Response>, Refusal> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        let read = input
            .read_line(&mut line)
            .map_err(|error| malformed(format!("the response stream could not be read: {error}")))?;
        if read == 0 {
            if lines.is_empty() {
                return Ok(None);
            }
            return Err(malformed("the stream ended before the response's blank line"));
        }
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed.is_empty() {
            break;
        }
        lines.push(trimmed.to_string());
    }
    parse_response(&lines).map(Some)
}

/// Read one response and require that it is **about the line this caller sent** — the check a client
/// needs so that an answer to a different request can never be mistaken for its own. A mismatch is
/// refused by name with **both** lines in the message. Reading is otherwise [`read_response`].
pub fn read_response_for(
    input: &mut impl BufRead,
    expected: &str,
) -> Result<Option<Response>, Refusal> {
    let Some(response) = read_response(input)? else {
        return Ok(None);
    };
    if !response.is_about(expected) {
        return Err(malformed(format!(
            "a response to `{expected}` is about `{}`, which is a different line",
            response.request()
        )));
    }
    Ok(Some(response))
}

/// Parse one response's lines. Every field is declared: an unknown key, a key given twice, a missing
/// required key or a field that does not parse is refused — the reader never guesses.
pub fn parse_response(lines: &[String]) -> Result<Response, Refusal> {
    let mut status: Option<String> = None;
    let mut request: Option<String> = None;
    let mut record: Option<ServedRecord> = None;
    let mut receipt: Option<String> = None;
    let mut kind: Option<String> = None;
    let mut detail: Option<String> = None;

    for (index, line) in lines.iter().enumerate() {
        let (key, value) = line
            .split_once('\t')
            .ok_or_else(|| malformed(format!("line {} is not `key⇥value`", index + 1)))?;
        let slot = match key {
            "status" => &mut status,
            "request" => &mut request,
            "record" => {
                if record.is_some() {
                    return Err(malformed("`record` is given twice"));
                }
                record = Some(ServedRecord::from_wire(value)?);
                continue;
            }
            "receipt" => &mut receipt,
            "kind" => &mut kind,
            "detail" => &mut detail,
            other => {
                return Err(malformed(format!("`{other}` is not a declared response field")));
            }
        };
        if slot.is_some() {
            return Err(malformed(format!("`{key}` is given twice")));
        }
        *slot = Some(value.to_string());
    }

    let status = status.ok_or_else(|| malformed("`status` is absent"))?;
    let record = record.ok_or_else(|| malformed("`record` is absent"))?;
    match status.as_str() {
        "ok" => {
            let receipt = Receipt::from_wire(
                &receipt.ok_or_else(|| malformed("an `ok` response carries no `receipt`"))?,
            )?;
            Ok(Response::Ok { receipt: Box::new(receipt), record })
        }
        "err" => {
            // **The echo is the line this response is about**, so it is required and checked: `-` is
            // the transport's own "the line could not be framed", and anything else must be a line
            // that *could* have crossed. A response whose echo carries the framing's own characters
            // could not have been an answer to any real request, so it is refused rather than kept.
            let request = request
                .ok_or_else(|| malformed("an `err` response carries no `request`"))?;
            if request != "-" {
                if let Err(refusal) = frame_request(&request) {
                    return Err(malformed(format!(
                        "an `err` response echoes `{request}`, which is not a line that could have \
                         crossed: {}",
                        refusal.detail()
                    )));
                }
            }
            Ok(Response::Err {
                request,
                kind: kind.ok_or_else(|| malformed("an `err` response carries no `kind`"))?,
                detail: detail.unwrap_or_else(|| "-".to_string()),
                record,
            })
        }
        other => Err(malformed(format!("`{other}` is neither `ok` nor `err`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::parse;
    use crate::process;
    use crate::reference;
    use crate::stage::Declared;

    fn a_gate<'a>(roster: &'a Declared) -> Gate<'a> {
        reference::gate(roster, 1).expect("the reference declaration is admitted")
    }

    /// Serve the given lines in-process and return the raw response text, so the protocol's own
    /// framing (which the real session uses unchanged) is exercised without a process.
    fn raw_session(roster: &Declared, lines: &[&str]) -> String {
        let mut gate = a_gate(roster);
        let mut input = lines.join("\n").into_bytes();
        input.push(b'\n');
        let mut output: Vec<u8> = Vec::new();
        let session =
            serve(&mut gate, &mut input.as_slice(), &mut output).expect("serving an in-memory session");
        assert_eq!(session.requests, lines.len(), "one request per line");
        assert_eq!(session.requests, session.answered + session.refused);
        String::from_utf8(output).expect("the responses are utf-8")
    }

    /// The same session, read back as responses.
    fn converse(roster: &Declared, lines: &[&str]) -> Vec<Response> {
        let text = raw_session(roster, lines);
        let mut reader: &[u8] = text.as_bytes();
        let mut responses = Vec::new();
        while let Some(response) = read_response(&mut reader).expect("a well-formed response") {
            responses.push(response);
        }
        assert_eq!(responses.len(), lines.len(), "one response per request");
        responses
    }

    #[test]
    fn a_session_round_trips_everything_the_gate_answers() {
        let roster = process::roster().expect("the reference roster admits");
        let responses = converse(
            &roster,
            &[
                "truth",
                "geometry chunk=0,0,0 level=0",
                "geometry chunk=0,0,0 level=0",
                "held",
                "record",
            ],
        );

        // The read: a receipt whose answer is the world's, and a record that did not move.
        match &responses[0] {
            Response::Ok { receipt, record } => {
                assert_eq!(receipt.request, "truth");
                assert_eq!(receipt.answer.verb(), "truth");
                assert_eq!(record.events, 0, "a read writes nothing");
                assert_eq!(record.head, None);
                assert_eq!(receipt.effect.events_before, receipt.effect.events_after);
                assert_eq!(
                    Receipt::from_wire(&receipt.wire()).expect("its own wire form reads back"),
                    **receipt
                );
            }
            other => panic!("truth must be answered, not refused: {other:?}"),
        }

        // The derivation, then the hit: **one** answer digest, two effects.
        let (first, second) = match (&responses[1], &responses[2]) {
            (Response::Ok { receipt: first, .. }, Response::Ok { receipt: second, .. }) => {
                (first, second)
            }
            other => panic!("both derivations must be answered: {other:?}"),
        };
        assert_eq!(
            first.answer.digest(),
            second.answer.digest(),
            "the same request twice is one answer"
        );
        assert!(first.effect.did_work() && first.effect.appended.is_some());
        assert_eq!(second.effect.columns_produced, 0);
        assert!(second.effect.appended.is_none());
        assert_eq!(first.effect.events_after, 1, "one derivation, one event");
        assert_eq!(second.effect.events_after, 1, "a hit appends nothing");
        assert_eq!(responses[2].record().events, 1);

        // The record, read back from the wire: the length and head the server holds.
        match &responses[4] {
            Response::Ok { receipt, record } => {
                assert_eq!(record.events, 1);
                assert_eq!(receipt.answer.verb(), "record");
            }
            other => panic!("record must be answered: {other:?}"),
        }
    }

    #[test]
    fn a_world_refusal_keeps_its_owners_name_and_the_record_shows_it() {
        let roster = process::roster().expect("the reference roster admits");
        let responses = converse(
            &roster,
            &[
                "geometry chunk=9,9,0 level=0",
                "geometry chunk=0,0,0 level=9",
                "fields field=nope at=0,0,0 side=8",
            ],
        );
        let mut events = 0;
        for (index, response) in responses.iter().enumerate() {
            assert_eq!(
                response.refusal_kind(),
                Some(
                    [
                        "geometry_out_of_residency",
                        "level_off_ladder",
                        "field_undeclared"
                    ][index]
                ),
                "{response:?}"
            );
            let record = response.record();
            assert_eq!(
                record.events,
                events + 1,
                "an owner's refusal is a fact about the world, so the record holds exactly one more event"
            );
            events = record.events;
            assert!(record.head.is_some());
        }
    }

    #[test]
    fn a_line_that_does_not_parse_writes_nothing_and_a_line_that_cannot_cross_is_refused() {
        let roster = process::roster().expect("the reference roster admits");
        let responses = converse(
            &roster,
            &[
                "abracadabra",
                "geometry chunk=0,0 level=0",
                "truth field=elevation",
                "",
                "   ",
                "fields field=el|evation at=0,0,0 side=8",
            ],
        );
        // Each fault is refused by the name its **own** check gives it: an undeclared verb, a value
        // that is not the declared kind, an argument the row does not declare, two empty lines, and
        // the one refusal the transport itself makes. All of them write nothing.
        let expected = [
            "tool_undeclared",
            "argument_malformed",
            "argument_undeclared",
            "tool_undeclared",
            "tool_undeclared",
            "payload_not_wirable",
        ];
        for (index, response) in responses.iter().enumerate() {
            let kind = response.refusal_kind().expect("every one of these is refused");
            assert_eq!(kind, expected[index], "response {index}: {response:?}");
            assert_eq!(response.record().events, 0, "nothing was written");
        }

        // The refusal the **transport** made is its own, and it echoes `-`: the line it received is
        // exactly the line it cannot quote without breaking its framing.
        let text = raw_session(&roster, &["fields field=el|evation at=0,0,0 side=8"]);
        assert!(text.starts_with("status\terr\nrequest\t-\n"), "{text}");
        assert!(text.contains("kind\tpayload_not_wirable"), "{text}");

        // And a line the table refuses is echoed as it was received, so a caller sees what it sent.
        let text = raw_session(&roster, &["abracadabra"]);
        assert!(text.contains("request\tabracadabra"), "{text}");
        assert!(text.contains("kind\ttool_undeclared"), "{text}");
    }

    #[test]
    fn the_response_reader_refuses_what_the_writer_could_not_have_made() {
        let roster = process::roster().expect("the reference roster admits");
        let mut gate = a_gate(&roster);
        let mut input = b"truth\n".as_slice();
        let mut output: Vec<u8> = Vec::new();
        serve(&mut gate, &mut input, &mut output).expect("a session");
        let text = String::from_utf8(output).expect("the responses are utf-8");
        let lines: Vec<String> = text.lines().map(str::to_string).collect();

        // The declared shape, in order, and nothing else.
        assert_eq!(lines[0], "status\tok");
        assert!(lines[1].starts_with("request\ttruth"));
        assert!(lines[2].starts_with("record\t0:"));
        assert!(lines[3].starts_with("receipt\tshploingus.receipt/2~truth~"));
        assert_eq!(lines[4], "", "the response ends with a blank line");

        // An unknown field, a field given twice, a missing status, a bad record and a bad version are
        // each refused by name.
        let base = |extra: Vec<String>| {
            let mut lines = vec!["status\tok".to_string(), "request\ttruth".to_string(), "record\t0:-".to_string()];
            lines.extend(extra);
            lines
        };
        let receipt = lines[3].clone();
        assert!(parse_response(&base(vec![receipt.clone(), "nonsense\tx".to_string()])).is_err());
        assert!(parse_response(&base(vec![receipt.clone(), "status\tok".to_string()])).is_err());
        // A receipt field that is a version nobody declares is refused, not half-read.
        assert!(parse_response(&base(vec![receipt.replacen(
            "shploingus.receipt/2",
            "shploingus.receipt/0",
            1
        )]))
        .is_err());
        assert!(parse_response(&["request\ttruth".to_string(), "record\t0:-".to_string()]).is_err());
        assert!(parse_response(&["status\tok".to_string(), "record\tlots:-".to_string()]).is_err());
        // **An older version is refused rather than misread**: `/1` carried six axes, and a reader that
        // accepted it would read this receipt's conformance column as a claim's identity — which is the
        // one mistake the tag exists to make impossible.
        let older_version = receipt.replacen("shploingus.receipt/2", "shploingus.receipt/1", 1);
        assert!(parse_response(&base(vec![older_version])).is_err());
        // And the good one parses, of course — a refusal set that refuses everything reads nothing.
        assert!(parse_response(&base(vec![receipt])).is_ok());

        // A stream that ends inside a response is refused rather than truncated.
        let mut half: &[u8] = b"status\tok\nrequest\ttruth\n";
        assert!(read_response(&mut half).is_err());
        let mut empty: &[u8] = b"";
        assert!(read_response(&mut empty).expect("a clean end").is_none());

        // A request the client cannot frame is refused before it is written.
        assert_eq!(
            request("truth\ngeometry chunk=0,0,0 level=0")
                .expect_err("two requests in one line")
                .kind(),
            RefusalKind::PayloadNotWirable
        );
        assert_eq!(
            request("truth\ttruth").expect_err("a tab").kind(),
            RefusalKind::PayloadNotWirable
        );
        assert!(request("geometry chunk=0,0,0 level=0").is_ok());
    }

    #[test]
    fn a_served_receipt_is_the_value_the_gate_handed_out_and_not_a_summary_of_it() {
        let roster = process::roster().expect("the reference roster admits");
        let mut gate = a_gate(&roster);
        let receipt = gate
            .call("geometry chunk=0,0,0 level=0")
            .expect("a derivation");
        let wire = receipt.wire();
        let rebuilt = Receipt::from_wire(&wire).expect("its own wire form reads back");
        assert_eq!(rebuilt, receipt, "the served receipt is the receipt");
        assert_eq!(rebuilt.claims, receipt.claims);
        assert_eq!(rebuilt.effect, receipt.effect);
        assert_eq!(rebuilt.answer.digest(), receipt.answer.digest());
        assert_eq!(parse(&rebuilt.request).expect("a declared line").canonical(), rebuilt.request);

        // A receipt carrying a form nobody declares is refused rather than half-read.
        assert!(Receipt::from_wire(&wire.replacen(",quads=", ",quadz=", 1)).is_err());
        assert!(Receipt::from_wire(&wire.replacen("mesh_is_held,", "mesh_is_kept,", 1)).is_err());
        assert!(Receipt::from_wire(&format!("{wire}~extra")).is_err());

        // The wire form is **not signed**, and it does not need to be: a tampered field is caught in
        // the place it was tampered. A tampered **answer** is a different answer — its identity moves,
        // and a caller that derived the same answer sees the difference — while the claims it makes
        // still describe what the gate holds. A tampered **claim** is caught by verification itself.
        // The tampering is of the receipt's **own** numbers: the quads the gate answered are read off
        // the receipt, so this test moves with the world rather than with a literal.
        let quads = match &receipt.answer {
            crate::bridge::Answer::Geometry { quads, .. } => *quads,
            other => panic!("a geometry call answers geometry, not {other:?}"),
        };
        let answer_tamper = Receipt::from_wire(&wire.replacen(
            &format!("geometry,chunk=0:0:0,level=0,quads={quads},"),
            &format!("geometry,chunk=0:0:0,level=0,quads={},", quads + 1),
            1,
        ))
        .expect("the form still reads back");
        assert_ne!(answer_tamper.answer.digest(), receipt.answer.digest());
        assert_eq!(answer_tamper.claims, receipt.claims, "the claims are untouched");
        assert!(
            answer_tamper.verify(&gate.owners()).is_ok(),
            "the answer's own fields are the caller's to compare; the claims are the gate's to re-check"
        );

        let claim_tamper = Receipt::from_wire(&wire.replacen(
            &format!("mesh_is_held,chunk=0:0:0,level=0,quads={quads},"),
            &format!("mesh_is_held,chunk=0:0:0,level=0,quads={},", quads + 1),
            1,
        ))
        .expect("the form still reads back");
        assert_eq!(claim_tamper.answer.digest(), receipt.answer.digest());
        let failures = claim_tamper
            .verify(&gate.owners())
            .expect_err(&format!("the mesh holds {quads} quads, not {}", quads + 1));
        assert!(failures.contains(RefusalKind::ClaimUnverified));
        assert!(failures
            .first_named()
            .expect("one refusal")
            .detail()                .contains("mesh_is_held"));
    }

    #[test]
    fn every_refusal_the_session_writes_is_about_the_line_it_answers() {
        let roster = process::roster().expect("the reference roster admits");
        let text = raw_session(
            &roster,
            &[
                "geometry chunk=9,9,0 level=0",
                "abracadabra",
                "fields field=el|evation at=0,0,0 side=8",
            ],
        );
        let mut reader: &[u8] = text.as_bytes();

        // A refusal is about the line it quotes: the world's line, then the table's own line — read
        // back through the check a client uses, so the writer's refusals all pass the reader's rule.
        for line in ["geometry chunk=9,9,0 level=0", "abracadabra"] {
            let response = read_response_for(&mut reader, line)
                .expect("a refusal the server wrote reads back")
                .expect("the server answers every request");
            assert!(matches!(response, Response::Err { .. }), "{response:?}");
            assert_eq!(response.request(), line, "the echo is the line as received");
        }

        // The one line the transport could not frame is echoed `-`: it has nothing it may quote, and
        // such a response is about no line at all rather than about the one that never crossed.
        let unwirable = read_response(&mut reader)
            .expect("the transport's own refusal reads back")
            .expect("it answers the line it could not frame");
        assert_eq!(unwirable.request(), "-");
        assert_eq!(unwirable.refusal_kind(), Some("payload_not_wirable"));
        assert!(!unwirable.is_about("fields field=el|evation at=0,0,0 side=8"));
    }

    #[test]
    fn an_err_response_whose_echo_names_no_line_that_could_have_crossed_is_refused() {
        // The echo carries the framing's own tab: no request line could have carried it, so a response
        // claiming to answer one is refused rather than read — the writer could not have made it.
        let crossed = vec![
            "status\terr".to_string(),
            "request\ttruth\tboxed".to_string(),
            "record\t0:-".to_string(),
            "kind\ttool_undeclared".to_string(),
        ];
        assert!(parse_response(&crossed).is_err());

        // An `err` with no echo at all names no line either — the same fault from the other side.
        let nameless = vec![
            "status\terr".to_string(),
            "record\t0:-".to_string(),
            "kind\ttool_undeclared".to_string(),
        ];
        assert!(parse_response(&nameless).is_err());

        // The declared `-` echo reads back, so the rule refuses what could not have crossed and not
        // the transport's own answer.
        let unwirable = vec![
            "status\terr".to_string(),
            "request\t-".to_string(),
            "record\t0:-".to_string(),
            "kind\tpayload_not_wirable".to_string(),
            "detail\tthe request carries '\\t'".to_string(),
        ];
        let response = parse_response(&unwirable).expect("a declared echo reads back");
        assert_eq!(response.request(), "-");
        assert_eq!(response.refusal_kind(), Some("payload_not_wirable"));
    }

    #[test]
    fn a_response_about_a_different_line_is_refused_by_name_with_both_values() {
        // Every field is well formed and the refusal is a real one; the only fault is that the echo
        // names a request this client never sent.
        let other = "status\terr\nrequest\trecord\nrecord\t0:-\nkind\ttool_undeclared\n\
                     detail\tnothing there\n\n";
        let mut stream: &[u8] = other.as_bytes();
        let refusal = read_response_for(&mut stream, "truth").expect_err("it is about another line");
        assert_eq!(refusal.kind(), RefusalKind::MalformedWire);
        assert_eq!(refusal.kind().name(), "malformed_wire");
        assert!(
            refusal.detail().contains("`truth`") && refusal.detail().contains("`record`"),
            "both lines are named: {}",
            refusal.detail()
        );

        // And an answer that **is** about the line reads back — the check refuses mismatches, not
        // responses.
        let mut stream: &[u8] = other.as_bytes();
        assert!(read_response_for(&mut stream, "record")
            .expect("the line it names")
            .is_some());
    }

    #[test]
    fn an_answer_to_a_non_canonical_spelling_is_still_that_line_s_own_answer() {
        let roster = process::roster().expect("the reference roster admits");
        let text = raw_session(&roster, &["geometry level=0 chunk=0,0,0"]);
        let mut reader: &[u8] = text.as_bytes();

        // An `ok` answer is anchored to the table's canonical request, so a client's own spelling is
        // still recognised as its own — the check is about **which** request, not which spelling.
        let response = read_response_for(&mut reader, "geometry level=0 chunk=0,0,0")
            .expect("the canonical request is this line's own")
            .expect("it is answered");
        assert!(matches!(response, Response::Ok { .. }), "{response:?}");
        assert_eq!(response.request(), "geometry chunk=0,0,0 level=0");

        // A different request is not this line's answer, canonical spelling or not.
        let mut stream: &[u8] = text.as_bytes();
        assert!(read_response_for(&mut stream, "truth").is_err());
    }
}
