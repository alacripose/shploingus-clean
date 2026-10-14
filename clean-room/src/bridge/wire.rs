use super::*;
// ---------------------------------------------------------------------------
// The receipt's declared external form.
// ---------------------------------------------------------------------------

/// The tag a receipt's wire form opens with. A reader refuses any other tag, so a change to the form
/// is a refusal rather than a misread — the discipline the chain's wire form uses.
///
/// **`/2` (2026-10-12):** an admission's axes travel **by name and in the gate's declared order**, so
/// appending the conformance column lengthened every admission's answer while leaving the other verbs'
/// answers byte for byte what they were. A `/1` reader would have read the new axis as the identity of
/// a claim, which is the one thing the tag exists to prevent.
pub const RECEIPT_WIRE_VERSION: &str = "shploingus.receipt/2";

/// The separators, declared **once** so a writer and a reader cannot disagree: parts of a receipt by
/// `~`, items of a list by `;`, a claim's fields by `,`, a check's fields by `|`, a key from its value
/// by `=`, and a triple's numbers by `:`.
pub const WIRE_PART: char = '~';
pub const WIRE_ITEM: char = ';';
pub const WIRE_FIELD: char = ',';
pub const WIRE_CHECK_FIELD: char = '|';

fn wire_refusal(what: &str, detail: impl fmt::Display) -> Refusal {
    Refusal::new(RefusalKind::MalformedWire, format!("{what}: {detail}"))
}

/// A wire form's `key=value` fields, refusing a part that is not one and a key given twice (the same
/// rule the request table applies to its arguments).
fn wire_pairs<'a>(text: &'a str, what: &str) -> Result<Vec<(&'a str, &'a str)>, Refusal> {
    let mut out: Vec<(&str, &str)> = Vec::new();
    for part in text.split(WIRE_FIELD) {
        let (key, value) = part
            .split_once('=')
            .ok_or_else(|| wire_refusal(what, format!("`{part}` is not a `key=value` field")))?;
        if out.iter().any(|(seen, _)| *seen == key) {
            return Err(wire_refusal(what, format!("`{key}` is given twice")));
        }
        out.push((key, value));
    }
    Ok(out)
}

fn wire_value<'a>(pairs: &[(&'a str, &'a str)], key: &str, what: &str) -> Result<&'a str, Refusal> {
    pairs
        .iter()
        .find(|(seen, _)| *seen == key)
        .map(|(_, value)| *value)
        .ok_or_else(|| wire_refusal(what, format!("`{key}` is absent")))
}

fn wire_number<T: std::str::FromStr>(
    pairs: &[(&str, &str)],
    key: &str,
    what: &str,
) -> Result<T, Refusal> {
    let raw = wire_value(pairs, key, what)?;
    raw.parse::<T>()
        .map_err(|_| wire_refusal(what, format!("`{key}={raw}` is not a number")))
}

fn wire_hex(pairs: &[(&str, &str)], key: &str, what: &str) -> Result<Digest, Refusal> {
    let raw = wire_value(pairs, key, what)?;
    Digest::from_hex(raw).ok_or_else(|| wire_refusal(what, format!("`{key}={raw}` is not a digest")))
}

fn wire_maybe_hex(
    pairs: &[(&str, &str)],
    key: &str,
    what: &str,
) -> Result<Option<Digest>, Refusal> {
    let raw = wire_value(pairs, key, what)?;
    if raw == "-" {
        return Ok(None);
    }
    Digest::from_hex(raw)
        .map(Some)
        .ok_or_else(|| wire_refusal(what, format!("`{key}={raw}` is neither a digest nor `-`")))
}

fn wire_triple(pairs: &[(&str, &str)], key: &str, what: &str) -> Result<(i32, i32, i32), Refusal> {
    let raw = wire_value(pairs, key, what)?;
    let mut parts = raw.split(':');
    let (x, y, z) = (parts.next(), parts.next(), parts.next());
    let parsed = match (x, y, z) {
        (Some(x), Some(y), Some(z)) if parts.next().is_none() => {
            (x.parse::<i32>(), y.parse::<i32>(), z.parse::<i32>())
        }
        _ => return Err(wire_refusal(what, format!("`{key}={raw}` is not three numbers"))),
    };
    match parsed {
        (Ok(x), Ok(y), Ok(z)) => Ok((x, y, z)),
        _ => Err(wire_refusal(what, format!("`{key}={raw}` is not three numbers"))),
    }
}

/// The tag of a tagged wire form: the part before the first field.
fn wire_tag<'a>(text: &'a str, what: &str) -> Result<(&'a str, &'a str), Refusal> {
    text.split_once(WIRE_FIELD)
        .ok_or_else(|| wire_refusal(what, format!("`{text}` carries no tag")))
}

fn triple_wire(value: (i32, i32, i32)) -> String {
    format!("{}:{}:{}", value.0, value.1, value.2)
}

/// A digest this form may have to say is **absent**. The wire's own spelling for that is `-` — the
/// one [`wire_maybe_hex`] reads back — and not the report's word for it (`head_hex` says `none`,
/// which is prose rather than a value). The writer and the reader must agree on this spelling, or a
/// receipt about an **empty** record is written in a form its own reader refuses.
fn maybe_digest_wire(digest: Option<Digest>) -> String {
    digest
        .map(|digest| digest.to_hex())
        .unwrap_or_else(|| "-".to_string())
}

/// An admission's axes: one `axis:state` pair per declared axis, **in [`Axis::ALL`] order**, joined by
/// the separator a value inside one part may use (so it can never be confused with a field boundary).
/// The names travel, not their positions: a client reads which axis is in which state without having
/// to know this build's order — and the reader refuses an order that does not match, so a form that
/// moved an axis is a refusal rather than a silent relabelling.
fn axes_wire(axes: &[Establishment; Axis::COUNT]) -> String {
    Axis::ALL
        .iter()
        .zip(axes)
        .map(|(axis, state)| format!("{}:{}", axis.name(), state.name()))
        .collect::<Vec<_>>()
        .join(&WIRE_CHECK_FIELD.to_string())
}

fn wire_axes(
    pairs: &[(&str, &str)],
    what: &str,
) -> Result<[Establishment; Axis::COUNT], Refusal> {
    let raw = wire_value(pairs, "axes", what)?;
    let items: Vec<&str> = raw.split(WIRE_CHECK_FIELD).collect();
    if items.len() != Axis::COUNT {
        return Err(wire_refusal(
            what,
            format!(
                "`axes={raw}` names {} axes, and this gate declares {}",
                items.len(),
                Axis::COUNT
            ),
        ));
    }
    let mut axes = [Establishment::Unchecked; Axis::COUNT];
    for (index, item) in items.iter().enumerate() {
        let (axis, state) = item.split_once(':').ok_or_else(|| {
            wire_refusal(what, format!("`{item}` is not an `axis:state` pair"))
        })?;
        if axis != Axis::ALL[index].name() {
            return Err(wire_refusal(
                what,
                format!(
                    "`{axis}` is not `{}`, the axis this form declares in position {index}",
                    Axis::ALL[index].name()
                ),
            ));
        }
        axes[index] = Establishment::parse(state).ok_or_else(|| {
            wire_refusal(what, format!("`{state}` is not a declared establishment"))
        })?;
    }
    Ok(axes)
}

impl Answer {
    /// The answer in the wire form: its tag, then every described fact, keys in a declared order.
    pub fn wire(&self) -> String {
        match self {
            Answer::Truth { seed, period_s, tick, truth } => format!(
                "truth,seed={seed},period={period_s},tick={tick},truth={}",
                truth.to_hex()
            ),
            Answer::Fields { field, columns, values, low, high } => format!(
                "fields,field={field},columns={columns},values={},low_bits={},high_bits={}",
                values.to_hex(),
                low.to_bits(),
                high.to_bits()
            ),
            Answer::Geometry { chunk, level, quads, faces, digest, truth } => format!(
                "geometry,chunk={},level={level},quads={quads},faces={faces},mesh={},truth={}",
                triple_wire((chunk.x, chunk.y, chunk.z)),
                digest.to_hex(),
                truth.to_hex()
            ),
            Answer::Held { entries, stale, digest } => format!(
                "held,entries={entries},stale={stale},resident={}",
                digest.to_hex()
            ),
            Answer::Record { events, head } => format!(
                "record,events={events},head={}",
                maybe_digest_wire(*head)
            ),
            Answer::Admitted { feature, claim, found, axes } => format!(
                "admitted,feature={feature},claim={},signer={},found={},axes={}",
                claim.name(),
                claim.signer().unwrap_or("-"),
                found.to_hex(),
                axes_wire(axes)
            ),
            // A save document: the reference as declared, the digest it was found at, its length, and
            // the identity, ruleset, truth and record the bytes carry. The path can ride this form
            // because a declared reference may not contain any of the separators this form uses
            // (`crate::intake`'s own shape rule), so nothing here needs escaping.
            Answer::Saved(facts) => format!("saved,{}", facts.wire()),
            Answer::Loaded(facts) => format!("loaded,{}", facts.wire()),
        }
    }

    /// Read an answer back. A tag nobody declares, a missing fact or a malformed number is refused
    /// `malformed_wire`: the reader never guesses what the writer must have meant.
    pub fn from_wire(text: &str) -> Result<Self, Refusal> {
        let (tag, rest) = wire_tag(text, "an answer")?;
        let pairs = wire_pairs(rest, "an answer")?;
        match tag {
            "truth" => Ok(Answer::Truth {
                seed: wire_number(&pairs, "seed", "an answer")?,
                period_s: wire_number(&pairs, "period", "an answer")?,
                tick: wire_number(&pairs, "tick", "an answer")?,
                truth: wire_hex(&pairs, "truth", "an answer")?,
            }),
            "fields" => Ok(Answer::Fields {
                field: wire_field_name(&pairs, "field", "an answer")?,
                columns: wire_number(&pairs, "columns", "an answer")?,
                values: wire_hex(&pairs, "values", "an answer")?,
                low: f64::from_bits(wire_number(&pairs, "low_bits", "an answer")?),
                high: f64::from_bits(wire_number(&pairs, "high_bits", "an answer")?),
            }),
            "geometry" => {
                let (x, y, z) = wire_triple(&pairs, "chunk", "an answer")?;
                Ok(Answer::Geometry {
                    chunk: ChunkKey::new(x, y, z),
                    level: wire_number(&pairs, "level", "an answer")?,
                    quads: wire_number(&pairs, "quads", "an answer")?,
                    faces: wire_number(&pairs, "faces", "an answer")?,
                    digest: wire_hex(&pairs, "mesh", "an answer")?,
                    truth: wire_hex(&pairs, "truth", "an answer")?,
                })
            }
            "held" => Ok(Answer::Held {
                entries: wire_number(&pairs, "entries", "an answer")?,
                stale: wire_number(&pairs, "stale", "an answer")?,
                digest: wire_hex(&pairs, "resident", "an answer")?,
            }),
            "record" => Ok(Answer::Record {
                events: wire_number(&pairs, "events", "an answer")?,
                head: wire_maybe_hex(&pairs, "head", "an answer")?,
            }),
            "admitted" => {
                let found = wire_hex(&pairs, "found", "an answer")?;
                let claim_name = wire_value(&pairs, "claim", "an answer")?;
                let signer = match wire_value(&pairs, "signer", "an answer")? {
                    "-" => None,
                    name => Some(name),
                };
                let claim = crate::intake::Claim::from_wire_name(claim_name, signer, found)
                    .ok_or_else(|| {
                        wire_refusal(
                            "an answer",
                            format!(
                                "`claim={claim_name}` with `signer={}` is not a claim this form \
                                 declares",
                                signer.unwrap_or("-")
                            ),
                        )
                    })?;
                Ok(Answer::Admitted {
                    feature: wire_value(&pairs, "feature", "an answer")?.to_string(),
                    claim,
                    found,
                    axes: wire_axes(&pairs, "an answer")?,
                })
            }
            "saved" => Ok(Answer::Saved(wire_document_facts(&pairs)?)),
            "loaded" => Ok(Answer::Loaded(wire_document_facts(&pairs)?)),
            other => Err(wire_refusal("an answer", format!("`{other}` is not a declared verb"))),
        }
    }
}

impl DocumentFacts {
    /// The facts in the wire form: `key=value` fields, in the declared order, with the reference's own
    /// path carried verbatim (a declared reference carries no separator this form uses, which is the
    /// intake's rule rather than a coincidence).
    pub fn wire(&self) -> String {
        format!(
            "path={},document={},len={},seed={},period={},tick={},ruleset={},revision={},truth={},\
             record_len={},record_head={}",
            self.path,
            self.digest.to_hex(),
            self.len,
            self.seed,
            self.period_s,
            self.tick,
            self.ruleset.ruleset,
            self.ruleset.revision,
            self.truth.to_hex(),
            self.record_len,
            maybe_digest_wire(self.record_head)
        )
    }
}

/// Read a save row's facts back: every field required, and a missing or malformed one refused
/// `malformed_wire` — the reader never guesses what the writer must have meant.
fn wire_document_facts(pairs: &[(&str, &str)]) -> Result<DocumentFacts, Refusal> {
    Ok(DocumentFacts {
        path: wire_value(pairs, "path", "an answer")?.to_string(),
        digest: wire_hex(pairs, "document", "an answer")?,
        len: wire_number(pairs, "len", "an answer")?,
        seed: wire_number(pairs, "seed", "an answer")?,
        period_s: wire_number(pairs, "period", "an answer")?,
        tick: wire_number(pairs, "tick", "an answer")?,
        ruleset: Ruleset::new(
            wire_number(pairs, "ruleset", "an answer")?,
            wire_number(pairs, "revision", "an answer")?,
        ),
        truth: wire_hex(pairs, "truth", "an answer")?,
        record_len: wire_number(pairs, "record_len", "an answer")?,
        record_head: wire_maybe_hex(pairs, "record_head", "an answer")?,
    })
}

fn wire_field_name(
    pairs: &[(&str, &str)],
    key: &str,
    what: &str,
) -> Result<&'static str, Refusal> {
    let raw = wire_value(pairs, key, what)?;
    FieldId::ALL
        .iter()
        .find(|field| field.name() == raw)
        .map(|field| field.name())
        .ok_or_else(|| wire_refusal(what, format!("`{key}={raw}` is not a declared field")))
}

impl Claim {
    pub fn wire(&self) -> String {
        match self {
            Claim::TruthIs { seed, period_s, tick, recorded } => format!(
                "truth_is,seed={seed},period={period_s},tick={tick},recorded={}",
                recorded.to_hex()
            ),
            Claim::FieldIs { field, request, columns, recorded, .. } => {
                let (origin, side, kind) = match request {
                    ProcessRequest::Unit { at } => (*at, 1_u32, "unit"),
                    ProcessRequest::Region { origin, side } => (*origin, *side, "region"),
                };
                format!(
                    "field_is,field={field},kind={kind},at={},side={side},columns={columns},recorded={}",
                    triple_wire((origin.x, origin.y, origin.z)),
                    recorded.to_hex()
                )
            }
            Claim::MeshIsHeld { chunk, level, quads, recorded } => format!(
                "mesh_is_held,chunk={},level={level},quads={quads},recorded={}",
                triple_wire((chunk.x, chunk.y, chunk.z)),
                recorded.to_hex()
            ),
            Claim::HeldIs { entries, stale, recorded } => format!(
                "held_is,entries={entries},stale={stale},recorded={}",
                recorded.to_hex()
            ),
            Claim::RecordHeadIs { events, recorded } => format!(
                "record_head_is,events={events},recorded={}",
                maybe_digest_wire(*recorded)
            ),
            Claim::EventIs { seq, recorded } => format!(
                "event_is,seq={seq},recorded={}",
                recorded.to_hex()
            ),
        }
    }

    pub fn from_wire(text: &str) -> Result<Self, Refusal> {
        let (tag, rest) = wire_tag(text, "a claim")?;
        let pairs = wire_pairs(rest, "a claim")?;
        match tag {
            "truth_is" => Ok(Claim::TruthIs {
                seed: wire_number(&pairs, "seed", "a claim")?,
                period_s: wire_number(&pairs, "period", "a claim")?,
                tick: wire_number(&pairs, "tick", "a claim")?,
                recorded: wire_hex(&pairs, "recorded", "a claim")?,
            }),
            "field_is" => {
                let field = wire_field_name(&pairs, "field", "a claim")?;
                let field_id = FieldId::ALL
                    .iter()
                    .copied()
                    .find(|id| id.name() == field)
                    .ok_or_else(|| wire_refusal("a claim", format!("`{field}` is not a field")))?;
                let (x, y, z) = wire_triple(&pairs, "at", "a claim")?;
                let origin = VoxelPos::new(x, y, z);
                let command = wire_value(&pairs, "kind", "a claim")?;
                let request = match command {
                    "unit" => ProcessRequest::Unit { at: origin },
                    "region" => ProcessRequest::Region {
                        origin,
                        side: wire_number(&pairs, "side", "a claim")?,
                    },
                    other => {
                        return Err(wire_refusal(
                            "a claim",
                            format!("`kind={other}` is neither `unit` nor `region`"),
                        ))
                    }
                };
                Ok(Claim::FieldIs {
                    field,
                    field_id,
                    request,
                    columns: wire_number(&pairs, "columns", "a claim")?,
                    recorded: wire_hex(&pairs, "recorded", "a claim")?,
                })
            }
            "mesh_is_held" => {
                let (x, y, z) = wire_triple(&pairs, "chunk", "a claim")?;
                Ok(Claim::MeshIsHeld {
                    chunk: ChunkKey::new(x, y, z),
                    level: wire_number(&pairs, "level", "a claim")?,
                    quads: wire_number(&pairs, "quads", "a claim")?,
                    recorded: wire_hex(&pairs, "recorded", "a claim")?,
                })
            }
            "held_is" => Ok(Claim::HeldIs {
                entries: wire_number(&pairs, "entries", "a claim")?,
                stale: wire_number(&pairs, "stale", "a claim")?,
                recorded: wire_hex(&pairs, "recorded", "a claim")?,
            }),
            "record_head_is" => Ok(Claim::RecordHeadIs {
                events: wire_number(&pairs, "events", "a claim")?,
                recorded: wire_maybe_hex(&pairs, "recorded", "a claim")?,
            }),
            "event_is" => Ok(Claim::EventIs {
                seq: wire_number(&pairs, "seq", "a claim")?,
                recorded: wire_hex(&pairs, "recorded", "a claim")?,
            }),
            other => Err(wire_refusal("a claim", format!("`{other}` is not a declared claim"))),
        }
    }
}

impl Check {
    pub fn wire(&self) -> String {
        let outcome = match &self.outcome {
            CheckOutcome::Passed => "passed".to_string(),
            CheckOutcome::Refused(refusal) => {
                format!("refused:{}:{}", refusal.kind().name(), refusal.detail())
            }
        };
        format!(
            "{}|{outcome}|{}",
            self.name.name(),
            self.subject
        )
    }

    pub fn from_wire(text: &str) -> Result<Self, Refusal> {
        let parts: Vec<&str> = text.split(WIRE_CHECK_FIELD).collect();
        if parts.len() != 3 {
            return Err(wire_refusal(
                "a check",
                format!("has {} fields, not 3", parts.len()),
            ));
        }
        let name = CheckName::ALL
            .iter()
            .copied()
            .find(|check| check.name() == parts[0])
            .ok_or_else(|| wire_refusal("a check", format!("`{}` is not a declared check", parts[0])))?;
        let outcome = match parts[1].split_once(':') {
            None if parts[1] == "passed" => CheckOutcome::Passed,
            Some(("refused", rest)) => CheckOutcome::Refused(unwire_refusal(rest)),
            _ => {
                return Err(wire_refusal(
                    "a check",
                    format!("`{}` is neither `passed` nor `refused`", parts[1]),
                ))
            }
        };
        Ok(Check { name, subject: parts[2].to_string(), outcome })
    }
}

/// A refusal as its wire form carries it: a **name** and the words beside it. The name *is* the
/// vocabulary (a caller matches on it), so an unfamiliar name is carried verbatim rather than
/// dropped — a client sees an unfamiliar refusal, never no refusal.
fn unwire_refusal(text: &str) -> Refusal {
    let (name, detail) = text.split_once(':').unwrap_or((text, ""));
    match RefusalKind::ALL.iter().copied().find(|kind| kind.name() == name) {
        Some(kind) => Refusal::new(kind, detail),
        None => Refusal::new(
            RefusalKind::MalformedWire,
            format!("a refusal named {name:?}, which this build does not declare: {detail}"),
        ),
    }
}

impl Effect {
    pub fn wire(&self) -> String {
        format!(
            "columns={},samples={},quads={},hits={},misses={},refusals={},held={}:{},events={}:{},appended={}",
            self.columns_produced,
            self.samples_read,
            self.quads,
            self.hits,
            self.misses,
            self.refusals,
            self.held_before,
            self.held_after,
            self.events_before,
            self.events_after,
            self.appended.map(|digest| digest.to_hex()).unwrap_or_else(|| "-".to_string())
        )
    }

    pub fn from_wire(text: &str) -> Result<Self, Refusal> {
        let pairs = wire_pairs(text, "an effect")?;
        let pair = |key: &str| -> Result<(usize, usize), Refusal> {
            let raw = wire_value(&pairs, key, "an effect")?;
            let (before, after) = raw
                .split_once(':')
                .ok_or_else(|| wire_refusal("an effect", format!("`{key}={raw}` is not `before:after`")))?;
            let before = before
                .parse::<usize>()
                .map_err(|_| wire_refusal("an effect", format!("`{key}={raw}` is not two numbers")))?;
            let after = after
                .parse::<usize>()
                .map_err(|_| wire_refusal("an effect", format!("`{key}={raw}` is not two numbers")))?;
            Ok((before, after))
        };
        let (held_before, held_after) = pair("held")?;
        let (events_before, events_after) = pair("events")?;
        Ok(Effect {
            columns_produced: wire_number(&pairs, "columns", "an effect")?,
            samples_read: wire_number(&pairs, "samples", "an effect")?,
            quads: wire_number(&pairs, "quads", "an effect")?,
            hits: wire_number(&pairs, "hits", "an effect")?,
            misses: wire_number(&pairs, "misses", "an effect")?,
            refusals: wire_number(&pairs, "refusals", "an effect")?,
            held_before,
            held_after,
            events_before,
            events_after,
            appended: wire_maybe_hex(&pairs, "appended", "an effect")?,
        })
    }
}

impl Receipt {
    /// **The receipt's declared external form** — what crosses a boundary: one tab-free, newline-free
    /// field carrying the version tag, the canonical request, the answer, the claims, the checks and
    /// the effect. A reader ([`Receipt::from_wire`]) rebuilds the receipt, checks the request against
    /// the table, **and binds the evidence to the row the request names**, so a served receipt is the
    /// same value the gate handed out rather than a summary of it — and a receipt with its evidence
    /// stripped is refused rather than read back as one that verifies over nothing.
    pub fn wire(&self) -> String {
        let claims = self
            .claims
            .iter()
            .map(Claim::wire)
            .collect::<Vec<_>>()
            .join(&WIRE_ITEM.to_string());
        let checks = self
            .checks
            .iter()
            .map(Check::wire)
            .collect::<Vec<_>>()
            .join(&WIRE_ITEM.to_string());
        [
            RECEIPT_WIRE_VERSION.to_string(),
            self.request.clone(),
            self.answer.wire(),
            claims,
            checks,
            self.effect.wire(),
        ]
        .join(&WIRE_PART.to_string())
    }

    pub fn from_wire(text: &str) -> Result<Self, Refusal> {
        let parts: Vec<&str> = text.split(WIRE_PART).collect();
        if parts.len() != 6 {
            return Err(wire_refusal(
                "a receipt",
                format!("has {} parts, not 6", parts.len()),
            ));
        }
        if parts[0] != RECEIPT_WIRE_VERSION {
            return Err(wire_refusal(
                "a receipt",
                format!("opens with {:?}, not {RECEIPT_WIRE_VERSION:?}", parts[0]),
            ));
        }
        // The request must be the canonical line of the table's own parse, so a served receipt is
        // anchored to the declared table rather than to whatever a writer chose to put in a field.
        let request = parts[1].to_string();
        match parse(&request) {
            Ok(parsed) if parsed.canonical() == request => {}
            Ok(_) => {
                return Err(wire_refusal(
                    "a receipt",
                    "its request is not the canonical line of the table's own parse",
                ))
            }
            Err(refusal) => {
                return Err(wire_refusal(
                    "a receipt",
                    format!("its request does not parse against the table: {refusal}"),
                ))
            }
        }
        let mut claims = Vec::new();
        if !parts[3].is_empty() {
            for item in parts[3].split(WIRE_ITEM) {
                claims.push(Claim::from_wire(item)?);
            }
        }
        let mut checks = Vec::new();
        if !parts[4].is_empty() {
            for item in parts[4].split(WIRE_ITEM) {
                checks.push(Check::from_wire(item)?);
            }
        }
        let receipt = Receipt {
            request,
            answer: Answer::from_wire(parts[2])?,
            claims,
            checks,
            effect: Effect::from_wire(parts[5])?,
        };
        // The fields are read; **the evidence is bound to the row the request names**, so a receipt
        // whose claims or checks were emptied or swapped is refused by name here rather than read back
        // as a receipt that verifies over nothing ([`Receipt::bind_to_row`]).
        receipt.bind_to_row()?;
        Ok(receipt)
    }
}


