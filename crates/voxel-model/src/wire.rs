//! **The wire format: a declared schema, and a codec behind a trait.**
//!
//! # What this is a port of, and what it deliberately is not
//!
//! Two reference codecs exist and this is **neither of them copied**:
//!
//! * `shploingus/crates/diffusion-core/src/wire.rs` — **2,259 lines.** A binary codec with
//!   per-field schema version *histories* (`TypeChange` entries) and a coercion table.
//! * `shploingus-game/crates/diffusion-core/src/wire.rs` — **894 lines.** The same seam, one
//!   implementation, no version histories.
//!
//! **The line count is the lesson, not the goal.** The dirty tree's codec grew a version
//! history per field and a coercion table to answer a question this record does not yet have
//! a reader for, and the second tree answered the same question with a `trait Codec` and one
//! impl. That is the bloat the user named, and the fix is the *seam*, not a shorter file.
//!
//! So what is ported here is the **set of decisions**, each traceable, and no file:
//!
//! | decision | where it comes from |
//! |---|---|
//! | the schema is the documentation — every field carries a `doc` | both |
//! | `Value` is a closed set; a lookup returns `None`, never a default | both |
//! | **every field is length-described**, so an unknown tag can be *skipped* | game tree header |
//! | `decode` collects every defect; `encode` refuses | game tree header |
//! | a **domain string folded ahead of every payload** | dirty tree, `persistence/lineage.rs` |
//! | a **foreign format version refused, not migrated** | both, `observer/store.rs` |
//! | **two versions, two words** — frame vs declaration | dirty tree header |
//! | zero dependencies | L-6: *"a ruling, not an open question"* |
//!
//! # The property the format is built around
//!
//! **Every field is length-described.** A field frame is `name_len · name · payload_len ·
//! payload`, so a decoder that does not understand a field's tag can still **skip** it and keep
//! reading.
//!
//! That is not a style choice. A decoder that cannot skip can only ever report the first
//! problem it meets, which is the fail-fast behaviour this crate exists to replace.
//! **Collecting every refusal is a property of the format, not of the decoder's diligence.**
//!
//! # Canonical order is ascending by name, and that is the whole trick
//!
//! [`Projection`] holds values in a `BTreeMap`, so iteration **is** ascending by name. The
//! encoding is therefore canonical with no sorting step, and a decoder can hold a payload to a
//! declared order rather than accepting whatever order the writer happened to use — a payload
//! whose order is not canonical has no canonical digest, so it is refused rather than accepted
//! into a different one.
//!
//! # The bloat guard this format inherits
//!
//! The reference v3 ledger measured **81.5 % of its bytes inside an open `details{}` map**.
//! An open map has no vocabulary, so every writer invents keys. This format has no open map:
//! a field either is in [`Schema::fields`] or it is **refused by name**. That single property
//! is why a payload here cannot bloat the way the reference's did.

use std::collections::BTreeMap;

use crate::identity::Digest;
use crate::refusal::{RefusalKind, RefusalSet};

/// The magic a canonical payload begins with.
///
/// **`SHPC`, not the reference's `SHPD`.** `LEDGER.md` records the collision: `SHPD` is
/// `shploingus-game`'s binary container *and* the name of this tree's text recipe extension.
/// Two formats must not share a magic, because a reader that sees `SHPD` cannot tell which
/// tree wrote it.
pub const WIRE_MAGIC: [u8; 4] = *b"SHPC";

/// **The frame version** — where the schema version sits and how long a header is.
///
/// **Not the schema version.** See "two versions, two words": conflating them makes a routine
/// schema addition look like a framing break, which is a refusal spent on something that is
/// not one.
pub const WIRE_VERSION: u16 = 1;

/// The domain folded ahead of every payload digest, so a payload digest and a schema digest
/// can never be mistaken for one another (`persistence/lineage.rs`'s `stable_digest`).
pub const WIRE_DOMAIN: &str = "shploingus/wire-projection";

/// A field's declared type. **Closed**, so extending it is a contract change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldType {
    /// UTF-8 text, length-prefixed.
    Text,
    /// A signed 64-bit integer.
    Integer,
    /// A `sha256:` identity — **32 raw bytes on the wire**, not the hex rendering.
    Identity,
}

impl FieldType {
    pub const ALL: [FieldType; 3] = [FieldType::Text, FieldType::Integer, FieldType::Identity];

    pub const COUNT: usize = 3;

    pub const fn slug(self) -> &'static str {
        match self {
            FieldType::Text => "text",
            FieldType::Integer => "integer",
            FieldType::Identity => "identity",
        }
    }

    /// The byte tag a payload starts with. **`ALL[i]`'s tag is `i`**, so the type set and
    /// the wire cannot drift apart.
    pub const fn tag(self) -> u8 {
        match self {
            FieldType::Text => 0,
            FieldType::Integer => 1,
            FieldType::Identity => 2,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.tag() == tag)
    }
}

/// **Check 1.** Every type has a distinct tag — the invariant `from_tag` depends on.
///
/// Written as a function rather than a `const` so the compiler checks it: two variants
/// sharing a tag would make `from_tag` return the first and silently mis-decode the second.
pub const fn type_tags_are_distinct() -> bool {
    let mut i = 0;
    while i < FieldType::COUNT {
        let mut j = i + 1;
        while j < FieldType::COUNT {
            if FieldType::ALL[i].tag() == FieldType::ALL[j].tag() {
                return false;
            }
            j += 1;
        }
        i += 1;
    }
    true
}

/// One declared field.
///
/// The `doc` is **not decoration**: the rule is *the schema is the documentation*, so a field
/// that cannot say what it is for is a field nobody can refuse correctly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldSpec {
    pub name: &'static str,
    pub kind: FieldType,
    pub required: bool,
    pub doc: &'static str,
}

impl FieldSpec {
    pub const fn new(
        name: &'static str,
        kind: FieldType,
        required: bool,
        doc: &'static str,
    ) -> Self {
        FieldSpec {
            name,
            kind,
            required,
            doc,
        }
    }
}

/// A declared schema: a name, a version, and its fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schema {
    pub name: &'static str,
    /// **The declaration**, not the frame. It changes when the fields change; the frame
    /// changes when the layout does. Two words, two numbers.
    pub version: u16,
    pub fields: Vec<FieldSpec>,
}

impl Schema {
    pub fn new(name: &'static str, version: u16, fields: Vec<FieldSpec>) -> Self {
        Schema {
            name,
            version,
            fields,
        }
    }

    pub fn field(&self, name: &str) -> Option<&FieldSpec> {
        self.fields.iter().find(|field| field.name == name)
    }

    /// This schema's documentation, **as a report.** The schema *is* the documentation, and
    /// a report nobody can print is documentation only in the sense that matters to nobody.
    pub fn describe(&self) -> String {
        let mut out = format!("{} v{}", self.name, self.version);
        for field in &self.fields {
            out.push_str(&format!(
                "\n  {}{}: {} — {}",
                field.name,
                if field.required { "" } else { "?" },
                field.kind.slug(),
                field.doc
            ));
        }
        out
    }
}

/// A declared value. **Closed: three arms, because the record uses three.**
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Text(String),
    Integer(i64),
    Identity(Digest),
}

impl Value {
    pub const fn field_type(&self) -> FieldType {
        match self {
            Value::Text(_) => FieldType::Text,
            Value::Integer(_) => FieldType::Integer,
            Value::Identity(_) => FieldType::Identity,
        }
    }

    /// The value as text, or `None`. **Not a default and not a lossy render.**
    ///
    /// `u64 -> text` is refused unconditionally, and the reason is worth keeping: rendering
    /// `41` as `"41"` loses nothing *going that way*, but the reader would then have to parse
    /// it — **a second, unguessed interpretation of the same bytes.** A conversion that
    /// cannot be reversed exactly would be read twice, differently, by two readers.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(text) => Some(text.as_str()),
            Value::Integer(_) | Value::Identity(_) => None,
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(value) => Some(*value),
            Value::Text(_) | Value::Identity(_) => None,
        }
    }

    pub fn as_identity(&self) -> Option<Digest> {
        match self {
            Value::Identity(digest) => Some(*digest),
            Value::Text(_) | Value::Integer(_) => None,
        }
    }
}

/// A projection: a schema name, a schema version, and values by field name.
///
/// Values live in a `BTreeMap`, so iteration order **is** ascending by name. That is what
/// makes the encoding canonical with no separate sorting step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    schema: String,
    schema_version: u16,
    values: BTreeMap<String, Value>,
}

impl Projection {
    pub fn new(schema: impl Into<String>) -> Self {
        Projection {
            schema: schema.into(),
            schema_version: WIRE_VERSION,
            values: BTreeMap::new(),
        }
    }

    /// Add or replace a value.
    ///
    /// Deliberately not a `Result`: a `BTreeMap` cannot hold a duplicate key, so there is no
    /// duplicate to refuse here. Duplicates are refused at *decode*, where a hostile payload
    /// can actually produce one.
    pub fn with(mut self, name: impl Into<String>, value: Value) -> Self {
        self.values.insert(name.into(), value);
        self
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    pub fn schema(&self) -> &str {
        &self.schema
    }

    pub fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// **Ascending by name.** The canonical order, and the order the codec writes.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.values.iter()
    }

    /// The canonical encoding, or every reason it has none.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RefusalSet> {
        CanonicalCodec.encode(self)
    }

    /// The content identity of this projection.
    ///
    /// **Under a domain tag**, so a projection digest and a chain-event digest can never be
    /// the same number. `Digest::of_fields` length-prefixes each field, so two different
    /// field splits cannot collide.
    pub fn digest(&self) -> Result<Digest, RefusalSet> {
        let bytes = self.canonical_bytes()?;
        Ok(Digest::of_fields(
            WIRE_DOMAIN,
            &[WIRE_MAGIC.as_slice(), &bytes],
        ))
    }

    /// Every way this projection fails to satisfy a declared schema, **all of them.**
    ///
    /// Four checks, and each is a defect the payload could otherwise get away with:
    ///
    /// 1. a field the payload carries that the schema does not declare — **refused by
    ///    name**, because an undeclared field is a field nobody wrote a rule for;
    /// 2. a required field that is absent — **refused by name**, and *being old is not a
    ///    defence for a field the writer was obliged to write*;
    /// 3. a field whose value's type is not the declared type;
    /// 4. a payload whose declared schema name is not this schema's name.
    pub fn validate(&self, schema: &Schema) -> RefusalSet {
        let mut refusals = RefusalSet::new();

        if self.schema != schema.name {
            refusals.refuse(
                RefusalKind::UnknownComponent,
                format!(
                    "payload declares schema {:?} and the schema in hand is {:?}. Refusing \
                     rather than reading a payload against a declaration it was not written to",
                    self.schema, schema.name
                ),
            );
        }
        if self.schema_version != schema.version {
            // **Refused, not migrated.** This build has no record of what the other
            // declaration was, so reading it would mean inventing one. Forward compatibility
            // is not a property this codec claims, for the same reason a foreign frame
            // version is refused.
            refusals.refuse(
                RefusalKind::UnknownComponent,
                format!(
                    "payload declares schema version {} and the declaration in hand is {}. Refused, \
                 not migrated: reading it would mean inventing a declaration this build does \
                 not have",
                    self.schema_version, schema.version
                ),
            );
        }

        for (name, value) in self.iter() {
            match schema.field(name) {
                None => {
                    refusals.refuse(
                        RefusalKind::UnknownComponent,
                        format!(
                            "field {name:?} is not declared by {}. Refused by name: an undeclared \
                             field is a field no rule was written for, and guessing which rule \
                             applies is how a payload becomes a different payload",
                            schema.name
                        ),
                    );
                }
                Some(spec) => {
                    if value.field_type() != spec.kind {
                        refusals.refuse(
                            RefusalKind::UnknownComponent,
                            format!(
                                "field {name:?} is declared {} and the payload carries {}. Refused \
                                 rather than read as either: a value read as the wrong type is a \
                                 value nobody declared",
                                spec.kind.slug(),
                                value.field_type().slug()
                            ),
                        );
                    }
                }
            }
        }

        for spec in &schema.fields {
            if spec.required && !self.contains(spec.name) {
                refusals.refuse(
                    RefusalKind::NoReading,
                    format!(
                        "field {:?} is required by {} and absent. {}",
                        spec.name, schema.name, spec.doc
                    ),
                );
            }
        }

        refusals
    }
}

/// The three guarantees an implementation must make.
///
/// 1. **Total.** No implementation panics. A malformed input is a refusal.
/// 2. **Collecting.** `decode` reports every defect it can reach, not the first.
/// 3. **Stable.** The same projection encodes to the same bytes in every version of the
///    implementation, or the version is a contract change.
pub trait Codec {
    /// The implementation's identity, recorded beside anything it produced.
    fn name(&self) -> &'static str;

    fn encode(&self, projection: &Projection) -> Result<Vec<u8>, RefusalSet>;

    fn decode(&self, bytes: &[u8]) -> Result<Projection, RefusalSet>;
}

/// The in-tree codec. Dependency-free, field-length-describing, and canonical.
#[derive(Clone, Copy, Debug, Default)]
pub struct CanonicalCodec;

impl CanonicalCodec {
    pub const NAME: &'static str = "shploingus/canonical/v1";
}

impl Codec for CanonicalCodec {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn encode(&self, projection: &Projection) -> Result<Vec<u8>, RefusalSet> {
        let mut refusals = RefusalSet::new();
        let mut out = Vec::new();

        out.extend_from_slice(&WIRE_MAGIC);
        out.extend_from_slice(&WIRE_VERSION.to_le_bytes());
        push_bytes(&mut out, projection.schema().as_bytes());
        out.extend_from_slice(&projection.schema_version().to_le_bytes());
        out.extend_from_slice(&(projection.len() as u32).to_le_bytes());

        for (name, value) in projection.iter() {
            if name.is_empty() {
                refusals.refuse(
                    RefusalKind::UnknownComponent,
                    "a field name is empty; a field nobody can name is a field nobody can refuse",
                );
                continue;
            }
            let payload = encode_payload(value);
            push_bytes(&mut out, name.as_bytes());
            push_bytes(&mut out, &payload);
        }

        if refusals.is_empty() {
            Ok(out)
        } else {
            Err(refusals)
        }
    }

    fn decode(&self, bytes: &[u8]) -> Result<Projection, RefusalSet> {
        let mut refusals = RefusalSet::new();
        let mut cursor = Cursor::new(bytes);

        // **A wrong magic is the one defect that cannot be continued past**: there is no
        // frame to resynchronise to. It is reported alone and honestly, rather than followed
        // by a cascade of nonsense about bytes that are not a payload at all.
        match cursor.take(WIRE_MAGIC.len()) {
            Some(found) if found == WIRE_MAGIC => {}
            Some(found) => {
                let mut set = RefusalSet::new();
                set.refuse(
                    RefusalKind::CorruptRecord,
                    format!(
                        "expected magic {:?} and found {:?}. These bytes are not a payload, and \
                         reading them as one would invent refusals about bytes that were never a \
                         record",
                        WIRE_MAGIC, found
                    ),
                );
                return Err(set);
            }
            None => {
                let mut set = RefusalSet::new();
                set.refuse(
                    RefusalKind::CorruptRecord,
                    "payload is shorter than its own magic; it is truncated",
                );
                return Err(set);
            }
        }

        let Some(version) = cursor.u16() else {
            refusals.refuse(
                RefusalKind::CorruptRecord,
                "payload ends before its frame version",
            );
            return Err(refusals);
        };
        if version != WIRE_VERSION {
            refusals.refuse(
                RefusalKind::UnknownComponent,
                format!(
                    "frame version {version} is not {WIRE_VERSION}. Refused, not migrated: the \
                     layout under another version is not this layout, and reading it would mean \
                     guessing where the fields are"
                ),
            );
        }

        let schema = match cursor.length_prefixed_string("schema name") {
            Ok(value) => value,
            Err(refusal) => {
                refusals.absorb(refusal);
                return Err(refusals);
            }
        };
        let Some(schema_version) = cursor.u16() else {
            refusals.refuse(
                RefusalKind::CorruptRecord,
                "payload ends before the schema version",
            );
            return Err(refusals);
        };
        let Some(field_count) = cursor.u32() else {
            refusals.refuse(
                RefusalKind::CorruptRecord,
                "payload ends before the field count",
            );
            return Err(refusals);
        };

        let mut values: BTreeMap<String, Value> = BTreeMap::new();
        let mut previous: Option<String> = None;

        for _ in 0..field_count {
            let name = match cursor.length_prefixed_string("field name") {
                Ok(value) => value,
                Err(refusal) => {
                    // The frame's own lengths are unreadable, so there is no next frame to
                    // skip to. Report and stop, rather than inventing refusals about bytes
                    // that may be the middle of a value.
                    refusals.absorb(refusal);
                    break;
                }
            };
            let payload = match cursor.length_prefixed("field payload") {
                Ok(value) => value.to_vec(),
                Err(refusal) => {
                    refusals.absorb(refusal);
                    break;
                }
            };

            // **Order is declared, not conventional.** A payload whose fields are not in
            // ascending name order is not a payload this codec produced, so it is refused
            // rather than accepted into a different digest.
            if let Some(previous) = &previous {
                if name.as_str() <= previous.as_str() {
                    refusals.refuse(
                        RefusalKind::UnknownComponent,
                        format!(
                            "field {name:?} does not follow {previous:?} in ascending name order. \
                             A payload whose order is not the declared canonical order has no \
                             canonical digest"
                        ),
                    );
                }
            }
            if values.contains_key(&name) {
                refusals.refuse(
                    RefusalKind::UnknownComponent,
                    format!("field {name:?} appears twice; refusing to decide which one counts"),
                );
            }

            match decode_payload(&payload) {
                Ok(value) => {
                    values.insert(name.clone(), value);
                }
                Err(refusal) => {
                    refusals.absorb(refusal);
                }
            }
            previous = Some(name);
        }

        if cursor.remaining() > 0 {
            refusals.refuse(
                RefusalKind::CorruptRecord,
                format!(
                    "{} bytes remain after {field_count} fields. A payload with a tail is a \
                     payload whose length nobody can account for, so it is refused rather than \
                     read up to its own end",
                    cursor.remaining()
                ),
            );
        }

        if refusals.is_empty() {
            Ok(Projection {
                schema,
                schema_version,
                values,
            })
        } else {
            Err(refusals)
        }
    }
}

/// One chain event, as a payload. **The schema is the declaration; this is its name.**
pub const CHAIN_EVENT_SCHEMA: &str = "shploingus.chain-event/1";

/// The declared fields of a chain event.
///
/// **Sixteen fields, and every one carries its own `doc`** — because the schema is the
/// documentation, and the two fields that want to say more (`question`, `answer`) are bounded
/// by `envelope::MAX_FIELD_BYTES` and **refused** rather than truncated. There is no `details`
/// map: an open map has no vocabulary, so every writer invents keys, and 81.5 % of the
/// reference ledger's bytes were inside one.
pub fn chain_event_schema() -> Schema {
    Schema::new(
        CHAIN_EVENT_SCHEMA,
        1,
        vec![
            FieldSpec::new(
                "actor",
                FieldType::Text,
                true,
                "who filed it; an unattributable record is not a record",
            ),
            FieldSpec::new(
                "answer",
                FieldType::Text,
                false,
                "the answer, when there is one; empty only when state is unresolved",
            ),
            FieldSpec::new(
                "campaign",
                FieldType::Text,
                true,
                "the development campaign. NOT a season: a season is winter/spring/summer/autumn (H-10.5)",
            ),
            FieldSpec::new(
                "control",
                FieldType::Text,
                true,
                "how the round ended: continue, one_more_round, pause, stop",
            ),
            FieldSpec::new(
                "event_kind",
                FieldType::Text,
                true,
                "the chain's own kind: world_action, development_fact, ticket_transition, retirement, refusal_recorded",
            ),
            FieldSpec::new(
                "event_type",
                FieldType::Text,
                true,
                "the design record's axis: user_decision, answer, answer_supersedes, boundary_ruling, child_escalation",
            ),
            FieldSpec::new(
                "kind",
                FieldType::Text,
                true,
                "what the record is about: authority, evidence, routine, dependency, scope, unresolved_ambiguity",
            ),
            FieldSpec::new(
                "previous",
                FieldType::Text,
                true,
                "the link to the previous event, or the literal not_evaluable. Never a bare status",
            ),
            FieldSpec::new(
                "question",
                FieldType::Text,
                true,
                "the question, bounded at 4 KiB and refused rather than truncated",
            ),
            FieldSpec::new(
                "round_id",
                FieldType::Text,
                true,
                "the round this belongs to, i.e. the campaign",
            ),
            FieldSpec::new(
                "season",
                FieldType::Text,
                true,
                "the world's season: winter, spring, summer, autumn. A field, never a directory",
            ),
            FieldSpec::new(
                "state",
                FieldType::Text,
                true,
                "whether the question is settled: answered, unresolved, superseded",
            ),
            FieldSpec::new(
                "supersedes",
                FieldType::Text,
                false,
                "the id this record supersedes; a later answer never rewrites an earlier one",
            ),
            FieldSpec::new(
                "world",
                FieldType::Text,
                true,
                "the world this event belongs to",
            ),
            FieldSpec::new(
                "year",
                FieldType::Integer,
                true,
                "the world's year, zero-based",
            ),
        ],
    )
}

/// One field's payload: a type tag, then the body.
fn encode_payload(value: &Value) -> Vec<u8> {
    let mut out = vec![value.field_type().tag()];
    match value {
        Value::Text(text) => {
            out.extend_from_slice(&(text.len() as u32).to_le_bytes());
            out.extend_from_slice(text.as_bytes());
        }
        Value::Integer(number) => out.extend_from_slice(&number.to_le_bytes()),
        Value::Identity(digest) => out.extend_from_slice(digest.as_bytes()),
    }
    out
}

/// Decode one payload, or refuse and say which part was wrong.
fn decode_payload(payload: &[u8]) -> Result<Value, RefusalSet> {
    let mut refusals = RefusalSet::new();

    let Some((&tag, body)) = payload.split_first() else {
        refusals.refuse(
            RefusalKind::CorruptRecord,
            "a field payload is empty, so it carries no type and no value",
        );
        return Err(refusals);
    };

    let Some(kind) = FieldType::from_tag(tag) else {
        refusals.refuse(
            RefusalKind::UnknownComponent,
            format!(
                "field payload has type tag {tag}, which this build has no arm for. Refused by \
                 tag: a payload read as the wrong type is a value nobody declared"
            ),
        );
        return Err(refusals);
    };

    match kind {
        FieldType::Text => {
            if body.len() < 4 {
                refusals.refuse(
                    RefusalKind::CorruptRecord,
                    format!("a text payload is {} bytes, too short for its own length", body.len()),
                );
                return Err(refusals);
            }
            let mut length = [0u8; 4];
            length.copy_from_slice(&body[..4]);
            let length = u32::from_le_bytes(length) as usize;
            let rest = &body[4..];
            if rest.len() != length {
                refusals.refuse(
                    RefusalKind::CorruptRecord,
                    format!(
                        "a text payload declares {length} bytes and carries {}. Refused rather \
                         than read up to its own end",
                        rest.len()
                    ),
                );
                return Err(refusals);
            }
            match std::str::from_utf8(rest) {
                Ok(text) => Ok(Value::Text(text.to_string())),
                Err(error) => {
                    refusals.refuse(
                        RefusalKind::CorruptRecord,
                        format!("a text payload is not utf-8: {error}"),
                    );
                    Err(refusals)
                }
            }
        }
        FieldType::Integer => {
            if body.len() != 8 {
                refusals.refuse(
                    RefusalKind::CorruptRecord,
                    format!(
                        "an integer payload is {} bytes, not 8. Refused rather than read as \
                         whatever prefix happened to be there",
                        body.len()
                    ),
                );
                return Err(refusals);
            }
            let mut number = [0u8; 8];
            number.copy_from_slice(body);
            Ok(Value::Integer(i64::from_le_bytes(number)))
        }
        FieldType::Identity => {
            if body.len() != 32 {
                refusals.refuse(
                    RefusalKind::CorruptRecord,
                    format!(
                        "an identity payload is {} bytes, not 32. A digest is 32 raw bytes; a \
                         hex rendering is 64 characters and is NOT the same field",
                        body.len()
                    ),
                );
                return Err(refusals);
            }
            let mut raw = [0u8; 32];
            raw.copy_from_slice(body);
            Ok(Value::Identity(Digest::from_bytes(raw)))
        }
    }
}

/// Length-prefix a byte run. **Every field carries its own length**, which is what makes a fold
/// unambiguous: `("ab","c")` and `("a","bc")` cannot collide.
fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// A byte cursor that **reports a short read instead of panicking.**
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Cursor { bytes, at: 0 }
    }

    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(count)?;
        let slice = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(slice)
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.at)
    }

    fn u16(&mut self) -> Option<u16> {
        let bytes = self.take(2)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Option<u32> {
        let bytes = self.take(4)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// A length-prefixed run that must be text. **A field name and a schema name are names**,
    /// and a name that is not utf-8 is not a name — so this refuses rather than lossily
    /// replacing the bad bytes, which would produce a name nobody declared.
    fn length_prefixed_string(&mut self, what: &str) -> Result<String, RefusalSet> {
        let bytes = self.length_prefixed(what)?;
        match std::str::from_utf8(bytes) {
            Ok(text) => Ok(text.to_string()),
            Err(error) => {
                let mut refusals = RefusalSet::new();
                refusals.refuse(
                    RefusalKind::CorruptRecord,
                    format!(
                        "the {what} is not utf-8: {error}. Refused rather than repaired: a name \
                         with substituted bytes is a name nobody declared"
                    ),
                );
                Err(refusals)
            }
        }
    }

    fn length_prefixed(&mut self, what: &str) -> Result<&'a [u8], RefusalSet> {
        let Some(length) = self.u32() else {
            let mut refusals = RefusalSet::new();
            refusals.refuse(
                RefusalKind::CorruptRecord,
                format!("payload ends before the {what}'s length"),
            );
            return Err(refusals);
        };
        match self.take(length as usize) {
            Some(slice) => Ok(slice),
            None => {
                let mut refusals = RefusalSet::new();
                refusals.refuse(
                    RefusalKind::CorruptRecord,
                    format!(
                        "the {what} declares {length} bytes and {} remain. Refused rather than read \
                         up to its own end",
                        self.remaining()
                    ),
                );
                Err(refusals)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;

    fn sample() -> Projection {
        Projection::new(CHAIN_EVENT_SCHEMA)
            .with("actor", Value::Text("user".into()))
            .with("question", Value::Text("should the tab be pinned".into()))
            .with("year", Value::Integer(3))
            .with(
                "previous",
                Value::Text(crate::chain::NOT_EVALUABLE.to_string()),
            )
    }

    #[test]
    fn a_projection_round_trips_to_identical_bytes() {
        let original = sample();
        let bytes = original.canonical_bytes().expect("encodes");
        let decoded = CanonicalCodec.decode(&bytes).expect("decodes");
        assert_eq!(decoded, original, "the round trip is exact, not near-exact");
        assert_eq!(
            decoded.canonical_bytes().expect("re-encodes"),
            bytes,
            "and the encoding is stable, which is what makes the digest meaningful"
        );
    }

    #[test]
    fn the_encoding_does_not_depend_on_insertion_order() {
        // **Canonical order is a property of the format, not of the writer.** Same values,
        // opposite construction order, identical bytes.
        let forwards = Projection::new(CHAIN_EVENT_SCHEMA)
            .with("actor", Value::Text("user".into()))
            .with("year", Value::Integer(3));
        let backwards = Projection::new(CHAIN_EVENT_SCHEMA)
            .with("year", Value::Integer(3))
            .with("actor", Value::Text("user".into()));
        assert_eq!(
            forwards.canonical_bytes().expect("a"),
            backwards.canonical_bytes().expect("b")
        );
    }

    #[test]
    fn a_field_that_looks_like_two_fields_cannot_be_misread() {
        // The ancestor's rule: length-prefixed fields, so ("ab","c") and ("a","bc") differ.
        let one = Projection::new(CHAIN_EVENT_SCHEMA).with("ab", Value::Text("c".into()));
        let two = Projection::new(CHAIN_EVENT_SCHEMA).with("a", Value::Text("bc".into()));
        assert_ne!(
            one.digest().expect("a"),
            two.digest().expect("b"),
            "two different field splits must not collide"
        );
    }

    #[test]
    fn a_wrong_magic_is_refused_alone_and_honestly() {
        let mut bytes = sample().canonical_bytes().expect("encodes");
        bytes[0] = b'X';
        let refusals = CanonicalCodec.decode(&bytes).expect_err("refused");
        assert_eq!(
            refusals.len(),
            1,
            "a wrong magic is the one defect that cannot be continued past, so it is reported \
             alone rather than followed by a cascade about bytes that are not a payload: {}",
            refusals
                .all()
                .iter()
                .map(|r| r.detail().to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[test]
    fn a_truncated_payload_is_refused_rather_than_read_up_to_its_own_end() {
        let bytes = sample().canonical_bytes().expect("encodes");
        let refusals = CanonicalCodec
            .decode(&bytes[..bytes.len() - 3])
            .expect_err("refused");
        assert!(
            refusals.all().iter().any(|r| r.detail().contains("truncated")
                || r.detail().contains("remain")),
            "a short payload is named as short, not padded: {}",
            refusals
                .all()
                .iter()
                .map(|r| r.detail().to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[test]
    fn a_payload_with_a_tail_is_refused() {
        let mut bytes = sample().canonical_bytes().expect("encodes");
        bytes.push(0);
        bytes.push(0);
        let refusals = CanonicalCodec.decode(&bytes).expect_err("refused");
        assert!(
            refusals
                .all()
                .iter()
                .any(|r| r.detail().contains("remain after")),
            "a payload nobody can account for the length of is refused, not read up to its end"
        );
    }

    #[test]
    fn an_unknown_type_tag_is_refused_by_tag() {
        // **Built by corrupting a real payload, not by hand.** A hand-built frame would test
        // my arithmetic; this tests the decoder. The first field in ascending name order is
        // `actor`, so its tag is the first payload byte after the header and the name.
        let mut bytes = sample().canonical_bytes().expect("encodes");
        let name_at = bytes
            .windows(5)
            .position(|window| window == b"actor")
            .expect("the field is present");
        // 4 bytes of name length, 5 of name, 4 of payload length, then the tag.
        bytes[name_at + 5 + 4] = 99;

        let refusals = CanonicalCodec.decode(&bytes).expect_err("refused");
        assert!(
            refusals
                .all()
                .iter()
                .any(|r| r.detail().contains("type tag 99")),
            "an undeclared tag is refused by the tag, not read as whichever arm happens to be \
             first: {}",
            refusals
                .all()
                .iter()
                .map(|r| r.detail().to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[test]
    fn an_identity_is_thirty_two_raw_bytes_and_not_a_hex_rendering() {
        let digest = Digest::of(b"a digest");
        let identity = Identity::parse(&format!("sha256:{}", digest.to_hex())).expect("parses");
        let projection = Projection::new(CHAIN_EVENT_SCHEMA)
            .with("previous", Value::Identity(identity.digest()));
        let bytes = projection.canonical_bytes().expect("encodes");

        let at = bytes
            .windows(8)
            .position(|window| window == b"previous")
            .expect("the field is present");
        // 4 bytes of name length, 8 of name, then the payload: 4 of payload length, the tag,
        // and the body. The payload's declared length is the claim under test — the fields
        // that follow it in canonical order are somebody else's problem.
        let length_at = at + 8;
        assert_eq!(bytes[length_at + 4], FieldType::Identity.tag());
        let mut length = [0u8; 4];
        length.copy_from_slice(&bytes[length_at..length_at + 4]);
        assert_eq!(
            u32::from_le_bytes(length),
            33,
            "a digest payload is 1 type tag + 32 raw bytes. The 64-character hex rendering is a \
             different field with a different length, and reading one as the other would be a \
             second interpretation of the same bytes"
        );
    }

    #[test]
    fn a_hex_rendering_is_refused_where_a_digest_is_declared() {
        // The other direction, and the one a writer actually gets wrong.
        let projection = Projection::new(CHAIN_EVENT_SCHEMA)
            .with("previous", Value::Text("a".repeat(64)));
        let mut bytes = projection.canonical_bytes().expect("encodes");
        // Re-tag the payload as an Identity and shorten it to 32 bytes: a hex string that
        // *almost* fits.
        let at = bytes
            .windows(8)
            .position(|window| window == b"previous")
            .expect("the field is present");
        let length_at = at + 8;
        let tag_at = length_at + 4;
        bytes[tag_at] = FieldType::Identity.tag();
        let mut length = [0u8; 4];
        length.copy_from_slice(&bytes[length_at..length_at + 4]);
        let declared = u32::from_le_bytes(length) as usize;
        let body = declared - 1; // the tag byte came out of the declared length
        assert!(body != 32, "the fixture is only meaningful if it is the wrong size");
        bytes.truncate(tag_at + 1 + body);

        let refusals = CanonicalCodec.decode(&bytes).expect_err("refused");
        assert!(
            refusals
                .all()
                .iter()
                .any(|r| r.detail().contains("not 32")),
            "a digest field carrying the wrong number of bytes is refused by count, and the \
             refusal names what a digest is: {}",
            refusals
                .all()
                .iter()
                .map(|r| r.detail().to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[test]
    fn a_foreign_frame_version_is_refused_not_migrated() {
        let mut bytes = sample().canonical_bytes().expect("encodes");
        bytes[4] = 9; // the low byte of the frame version
        let refusals = CanonicalCodec.decode(&bytes).expect_err("refused");
        assert!(
            refusals
                .all()
                .iter()
                .any(|r| r.detail().contains("Refused, not migrated")),
            "forward compatibility is not a property this codec claims"
        );
    }

    #[test]
    fn a_foreign_schema_version_is_refused_not_migrated() {
        // **Decode is declaration-blind and that is not a gap, it is the design.** A decoder
        // that skipped an unknown field tag cannot know which declaration governs it, so
        // `decode` reports what it read and `validate` is where a foreign version is refused.
        // The two halves are separated so a decoder stays total.
        let mut bytes = sample().canonical_bytes().expect("encodes");
        // The schema version sits after the magic, the frame version, and the length-prefixed
        // schema name.
        let at = 4 + 2 + 4 + CHAIN_EVENT_SCHEMA.len();
        bytes[at] = 7;
        let decoded = CanonicalCodec
            .decode(&bytes)
            .expect("decode reads the bytes without ruling on them");
        assert_eq!(decoded.schema_version(), 7);
        let refusals = decoded.validate(&chain_event_schema());
        assert!(
            refusals
                .all()
                .iter()
                .any(|r| r.detail().contains("Refused, not migrated")),
            "and validate refuses a payload written under a declaration this build does not have"
        );
    }

    #[test]
    fn an_undeclared_field_is_refused_by_name() {
        let projection = Projection::new(CHAIN_EVENT_SCHEMA)
            .with("actor", Value::Text("user".into()))
            .with("details", Value::Text("anything".into()));
        let refusals = projection.validate(&chain_event_schema());
        assert!(
            refusals
                .all()
                .iter()
                .any(|r| r.detail().contains("\"details\"") && r.detail().contains("not declared")),
            "an undeclared field is refused by name. This is the bloat guard: the reference \
             ledger put 81.5% of its bytes in an open map, and an open map has no vocabulary, \
             so every writer invents keys"
        );
    }

    #[test]
    fn a_missing_required_field_is_refused_by_name_and_quotes_its_doc() {
        let projection = Projection::new(CHAIN_EVENT_SCHEMA).with("actor", Value::Text("u".into()));
        let schema = chain_event_schema();
        let refusals = projection.validate(&schema);
        let reported = refusals
            .all()
            .iter()
            .map(|r| r.detail().to_string())
            .collect::<Vec<_>>()
            .join(" | ");

        // **Every** missing required field is named, and **every one carries its own doc**.
        // The doc travelling with the refusal is the whole of "the schema is the
        // documentation": a reader holding only the refusal still knows what the field was
        // for, which is what lets them fix the writer.
        for spec in schema.fields.iter().filter(|f| f.required && f.name != "actor") {
            assert!(
                reported.contains(&format!("{:?}", spec.name)),
                "field {:?} is missing and must be named: {reported}",
                spec.name
            );
            assert!(
                reported.contains(spec.doc),
                "field {:?}'s own doc must travel with its refusal, or the schema is not the \
                 documentation: {reported}",
                spec.name
            );
        }
    }

    #[test]
    fn every_refusal_is_collected_rather_than_the_first_returned() {
        // The header says decoding collects. Prove it with a payload missing four required
        // fields: a fail-fast decoder would report one.
        let projection = Projection::new(CHAIN_EVENT_SCHEMA).with("actor", Value::Text("u".into()));
        let refusals = projection.validate(&chain_event_schema());
        assert!(
            refusals.len() >= 4,
            "every missing required field is reported, so one reading is enough to fix the \
             writer: {} refusals",
            refusals.len()
        );
    }

    #[test]
    fn a_wrongly_typed_field_is_refused_rather_than_read_as_either() {
        let projection = Projection::new(CHAIN_EVENT_SCHEMA)
            .with("actor", Value::Text("user".into()))
            // `year` is declared Integer and carried as Text.
            .with("year", Value::Text("three".into()));
        let refusals = projection.validate(&chain_event_schema());
        assert!(
            refusals
                .all()
                .iter()
                .any(|r| r.detail().contains("\"year\"") && r.detail().contains("integer")),
            "a value read as the wrong type is a value nobody declared"
        );
    }

    #[test]
    fn an_integer_is_never_silently_rendered_as_text() {
        // The reference refuses `u64 -> text` unconditionally, and the reason is kept in the
        // doc comment on `as_text`: the reader would have to parse it, which is a second
        // interpretation of the same bytes.
        assert_eq!(Value::Integer(41).as_text(), None);
        assert_eq!(Value::Integer(41).as_integer(), Some(41));
        assert_eq!(Value::Text("41".into()).as_integer(), None);
    }

    #[test]
    fn the_type_tags_are_distinct_so_from_tag_cannot_return_the_wrong_arm() {
        assert!(
            type_tags_are_distinct(),
            "two variants sharing a tag would make `from_tag` return the first and silently \
             mis-decode the second"
        );
    }

    #[test]
    fn the_schema_documents_itself() {
        let text = chain_event_schema().describe();
        for field in chain_event_schema().fields {
            assert!(
                text.contains(field.doc),
                "field {:?} has a doc that the report does not print, which would make the \
                 'the schema is the documentation' rule false for exactly that field",
                field.name
            );
        }
    }

    #[test]
    fn the_magic_is_not_the_references_magic() {
        // `LEDGER.md` records the collision: `SHPD` is `shploingus-game`'s binary container
        // and this tree's recipe extension is also called a shspec. A shared magic would make
        // "which tree wrote this" unanswerable from the bytes.
        assert_ne!(&WIRE_MAGIC, b"SHPD");
    }

    #[test]
    fn the_digest_is_domain_separated() {
        // The same bytes under a different domain must not be the same number, or a
        // projection digest could be mistaken for a chain-event digest.
        let bytes = sample().canonical_bytes().expect("encodes");
        let under_wire = Digest::of_fields(WIRE_DOMAIN, &[WIRE_MAGIC.as_slice(), &bytes]);
        let under_chain = Digest::of_fields(
            crate::chain::CHAIN_DOMAIN,
            &[WIRE_MAGIC.as_slice(), &bytes],
        );
        assert_ne!(under_wire, under_chain);
    }

    #[test]
    fn a_valid_chain_event_validates() {
        let projection = chain_event_projection_for_test();
        let refusals = projection.validate(&chain_event_schema());
        assert!(
            refusals.is_empty(),
            "a projection built from the schema validates: {}",
            refusals
                .all()
                .iter()
                .map(|r| r.detail().to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    /// A projection carrying **every** declared field, so the "a valid event validates" claim
    /// is a claim about the whole schema and not about four of its fifteen fields.
    fn chain_event_projection_for_test() -> Projection {
        let mut projection = Projection::new(CHAIN_EVENT_SCHEMA);
        for field in chain_event_schema().fields {
            let value = match field.kind {
                FieldType::Text => Value::Text(match field.name {
                    "event_kind" => "ticket_transition".to_string(),
                    "event_type" => "user_decision".to_string(),
                    "kind" => "authority".to_string(),
                    "state" => "answered".to_string(),
                    "control" => "continue".to_string(),
                    "previous" => crate::chain::NOT_EVALUABLE.to_string(),
                    "world" => "overworld".to_string(),
                    "campaign" => "c-2026".to_string(),
                    "round_id" => "c-2026".to_string(),
                    "season" => "winter".to_string(),
                    "actor" => "user".to_string(),
                    "question" => "should the tab be pinned".to_string(),
                    // The two optional text fields, present and empty, so the "absent" and
                    // "present but empty" cases are both exercised.
                    _ => String::new(),
                }),
                FieldType::Integer => Value::Integer(1),
                FieldType::Identity => Value::Identity(Digest::of(b"link")),
            };
            projection = projection.with(field.name, value);
        }
        projection
    }
}
