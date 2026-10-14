use super::*;
// ---------------------------------------------------------------------------
// The answer.
// ---------------------------------------------------------------------------

/// What an owner answered, as the facts it **describes**. No variant carries a worker count, a cache
/// hit or a ledger total, so [`Answer::digest`] is a pure function of what the answer is about: a
/// derivation and a cache hit are one answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    Truth {
        seed: u64,
        period_s: u64,
        tick: u64,
        truth: Digest,
    },
    Fields {
        field: &'static str,
        columns: usize,
        values: Digest,
        low: f64,
        high: f64,
    },
    Geometry {
        chunk: ChunkKey,
        level: u8,
        quads: usize,
        cells: u64,
        digest: Digest,
        truth: Digest,
    },
    Held {
        entries: usize,
        stale: usize,
        digest: Digest,
    },
    Record {
        events: usize,
        head: Option<Digest>,
    },
    /// **What a declared row became.** The feature it names, the claim it was admitted under (which is
    /// where *whether identity was established* is read — this variant holds no `bool` for it), the
    /// digest its document hashed to **when it was read in place**, and every declared axis's state in
    /// [`Axis::ALL`] order. The bytes are deliberately absent: an admission is about a document that
    /// stays where it lives, so nothing here can be mistaken for a copy of it.
    Admitted {
        feature: String,
        claim: crate::intake::Claim,
        found: Digest,
        axes: [Establishment; Axis::COUNT],
    },
    /// **A world written down as a document** (U9's document form, 2026-10-12 §28): what is now at the
    /// declared reference, and the world and record those bytes carry.
    Saved(DocumentFacts),
    /// **A document read back** (the same form's other half): what the declared reference turned out to
    /// be, and the world and truth the load reproduces from it.
    Loaded(DocumentFacts),
}

/// **What a save document is, established now.** The reference, the digest its bytes hash to, and the
/// identity, ruleset, truth and record they carry — the facts a call reports and the facts a receipt
/// is re-established against, built in **one** place ([`document_facts`]) so a call and a re-check
/// cannot drift into two readings of one document.
///
/// It is deliberately a **value and not the bytes**: a document is read where it lives, so a copy of
/// it here would be a second authority (SPEC-3) — and the digest is exactly what makes a document
/// *this* document rather than one that has since moved.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentFacts {
    /// The reference as declared (relative to the gate's root), never the path this host normalized.
    pub path: String,
    /// The digest the bytes at that path must hash to — and did.
    pub digest: Digest,
    /// How many bytes the document holds.
    pub len: usize,
    /// The saved world's seed, its declared period and its tick (the seed is a property of the world
    /// and the tick is not the seed — `LEDGER` C-96.1).
    pub seed: u64,
    pub period_s: u64,
    pub tick: u64,
    /// The configuration/ruleset identity the save names, and the one a load declares — the save is
    /// portable **and** explainable only because this is carried (`docs/GAIA.md` §5c/§11).
    pub ruleset: Ruleset,
    /// The truth the load reproduces (**derived** at load time, never read from the save), so printing
    /// it beside a live world's digest is two derivations compared rather than a claim with itself.
    pub truth: Digest,
    /// The record the save declares and carries: its length and its head.
    pub record_len: usize,
    pub record_head: Option<Digest>,
}

impl DocumentFacts {
    /// The facts in words a report prints — one rendering, used by the answer and by the refuse-from
    /// -both-values comparison of a re-established receipt.
    pub fn render(&self) -> String {
        format!(
            "{} ({} bytes, {}) seed={} period={}s tick={} ruleset {} revision {} truth={} record {} \
             event(s) head={}",
            self.path,
            self.len,
            self.digest.to_hex(),
            self.seed,
            self.period_s,
            self.tick,
            self.ruleset.ruleset,
            self.ruleset.revision,
            self.truth.to_hex(),
            self.record_len,
            head_hex(self.record_head)
        )
    }
}

impl Answer {
    /// The verb this answer is of.
    pub const fn verb(&self) -> &'static str {
        match self {
            Answer::Truth { .. } => "truth",
            Answer::Fields { .. } => "fields",
            Answer::Geometry { .. } => "geometry",
            Answer::Held { .. } => "held",
            Answer::Record { .. } => "record",
            Answer::Admitted { .. } => "admit",
            Answer::Saved(_) => "save",
            Answer::Loaded(_) => "load",
        }
    }

    /// The document facts this answer is about, when it is about one. The two save rows are the only
    /// answers whose subject is a document on a disk, and verification asks this rather than matching
    /// the variants in each reader.
    pub const fn document(&self) -> Option<&DocumentFacts> {
        match self {
            Answer::Saved(facts) | Answer::Loaded(facts) => Some(facts),
            _ => None,
        }
    }

    /// **The answer's identity: a pure function of what it describes.** The worker count, the ledger
    /// and whether the geometry came from a derivation or from the resident set are deliberately
    /// absent — that is what makes one request twice one answer (FR-004).
    pub fn digest(&self) -> Digest {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"shploingus.api-answer/1");
        match self {
            Answer::Truth { seed, period_s, tick, truth } => {
                bytes.push(1);
                bytes.extend_from_slice(&seed.to_le_bytes());
                bytes.extend_from_slice(&period_s.to_le_bytes());
                bytes.extend_from_slice(&tick.to_le_bytes());
                bytes.extend_from_slice(truth.as_bytes());
            }
            Answer::Fields { field, columns, values, .. } => {
                bytes.push(2);
                bytes.extend_from_slice(field.as_bytes());
                bytes.extend_from_slice(&(*columns as u64).to_le_bytes());
                bytes.extend_from_slice(values.as_bytes());
            }
            Answer::Geometry { chunk, level, quads, cells, digest, truth } => {
                bytes.push(3);
                bytes.extend_from_slice(&chunk.x.to_le_bytes());
                bytes.extend_from_slice(&chunk.y.to_le_bytes());
                bytes.extend_from_slice(&chunk.z.to_le_bytes());
                bytes.push(*level);
                bytes.extend_from_slice(&(*quads as u64).to_le_bytes());
                bytes.extend_from_slice(&cells.to_le_bytes());
                bytes.extend_from_slice(digest.as_bytes());
                bytes.extend_from_slice(truth.as_bytes());
            }
            Answer::Held { entries, stale, digest } => {
                bytes.push(4);
                bytes.extend_from_slice(&(*entries as u64).to_le_bytes());
                bytes.extend_from_slice(&(*stale as u64).to_le_bytes());
                bytes.extend_from_slice(digest.as_bytes());
            }
            Answer::Record { events, head } => {
                bytes.push(5);
                bytes.extend_from_slice(&(*events as u64).to_le_bytes());
                match head {
                    Some(digest) => {
                        bytes.push(1);
                        bytes.extend_from_slice(digest.as_bytes());
                    }
                    None => bytes.push(0),
                }
            }
            // A save document: every fact the two save rows are about, framed the way a digest is
            // (the reference, the digest, the lengths and the record) with the names of nothing — a
            // document's identity is its bytes, not a vocabulary.
            Answer::Saved(facts) | Answer::Loaded(facts) => {
                bytes.push(if matches!(self, Answer::Saved(_)) { 7 } else { 8 });
                bytes.extend_from_slice(facts.path.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(facts.digest.as_bytes());
                bytes.extend_from_slice(&(facts.len as u64).to_le_bytes());
                bytes.extend_from_slice(&facts.seed.to_le_bytes());
                bytes.extend_from_slice(&facts.period_s.to_le_bytes());
                bytes.extend_from_slice(&facts.tick.to_le_bytes());
                bytes.extend_from_slice(&facts.ruleset.ruleset.to_le_bytes());
                bytes.extend_from_slice(&facts.ruleset.revision.to_le_bytes());
                bytes.extend_from_slice(facts.truth.as_bytes());
                bytes.extend_from_slice(&(facts.record_len as u64).to_le_bytes());
                match facts.record_head {
                    Some(head) => {
                        bytes.push(1);
                        bytes.extend_from_slice(head.as_bytes());
                    }
                    None => bytes.push(0),
                }
            }
            // What the row became: the feature, the claim (with its signer, which is what identity is
            // read from), the digest the bytes were found at, and every axis's declared name. NUL-
            // framed rather than length-prefixed, because no name in these vocabularies can contain a
            // NUL and the closure of the framing is what matters, not its elegance.
            Answer::Admitted { feature, claim, found, axes } => {
                bytes.push(6);
                bytes.extend_from_slice(feature.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(claim.name().as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(claim.signer().unwrap_or("-").as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(found.as_bytes());
                for state in axes {
                    bytes.push(0);
                    bytes.extend_from_slice(state.name().as_bytes());
                }
            }
        }
        Digest::of(&bytes)
    }

    pub fn render(&self) -> String {
        match self {
            Answer::Truth { seed, period_s, tick, truth } => format!(
                "seed={seed} period={period_s}s tick={tick} truth={}",
                truth.to_hex()
            ),
            Answer::Fields { field, columns, values, low, high } => format!(
                "{field}: {columns} columns in [{low:.6}, {high:.6}], values={}",
                values.to_hex()
            ),
            Answer::Geometry { chunk, level, quads, cells, digest, truth } => format!(
                "chunk=({}, {}, {}) level={level} {quads} quads over {cells} cells, mesh={} of truth={}",
                chunk.x,
                chunk.y,
                chunk.z,
                digest.to_hex(),
                truth.to_hex()
            ),
            Answer::Held { entries, stale, digest } => {
                format!("{entries} entries ({stale} stale), resident={}", digest.to_hex())
            }
            Answer::Record { events, head } => {
                format!("{events} events, head={}", head_hex(*head))
            }
            Answer::Admitted { feature, claim, found, axes } => format!(
                "{feature}: admitted as {} (identity established = {}), the document read in place at \
                 {}; axes {}",
                claim.name(),
                claim.establishes_identity(),
                found.to_hex(),
                Axis::ALL
                    .iter()
                    .zip(axes)
                    .map(|(axis, state)| format!("{}={}", axis.name(), state.name()))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Answer::Saved(facts) => format!("saved {}", facts.render()),
            Answer::Loaded(facts) => format!("loaded {}", facts.render()),
        }
    }
}

/// **The facts a save document establishes now**, asked of U9's own seam. One place decides it — the
/// gate's `save`/`load` rows and [`Receipt::re_establish_document`] both come through here — so the
/// answer a call hands out and the answer verification re-derives from the request are the same value
/// rather than two readings that could drift apart.
///
/// It reads the declared reference **in place** through the content gate's one reader (so a document
/// that is absent or has moved refuses by the gate's own name, with both values), parses those bytes
/// with U9's one parser, and loads them against the ruleset this gate declares (so a save of another
/// world refuses `identity_mismatch` naming both). Nothing is carried in and nothing is kept.
pub(super) fn document_facts(
    intake: &Intake<'_>,
    ruleset: Ruleset,
    source: &Source,
) -> Result<DocumentFacts, RefusalSet> {
    let (save, document) = Save::read_document(intake, source)?;
    let loaded = save.load(ruleset)?;
    Ok(DocumentFacts {
        path: document.path().to_string(),
        digest: document.digest(),
        len: document.len(),
        seed: save.identity().seed,
        period_s: save.identity().period,
        tick: save.identity().tick,
        ruleset: save.ruleset(),
        truth: loaded.report.truth,
        record_len: loaded.report.record_len,
        record_head: loaded.report.record_head,
    })
}

pub(super) fn head_hex(head: Option<Digest>) -> String {
    head.map(|digest| digest.to_hex())
        .unwrap_or_else(|| "none".to_string())
}

/// **What a row becomes, asked of the intake's own seam.** One place decides it, so the answer a call
/// hands out and the answer verification re-establishes are the same value rather than two
/// constructions that could drift apart: the seam is asked what the row is now, and the refusal — if
/// the document cannot be read where the request says it lives, or a stage the row requires is not
/// declared, or a material was never admitted — is the owner's own, passed through by name.
pub(super) fn admitted_answer(row: &Row, intake: &Intake<'_>) -> Result<Answer, RefusalSet> {
    let inspection = intake.inspect(row);
    let verdict = inspection.verdict()?;
    let axes = std::array::from_fn(|index| {
        inspection
            .state(Axis::ALL[index])
            .expect("the intake's inspection asks every declared axis")
    });
    Ok(Answer::Admitted {
        feature: row.feature().to_string(),
        claim: row.claim().clone(),
        found: verdict.found(),
        axes,
    })
}

/// The identity of a field's values: the values' bytes, framed by their count, so two different
/// readings are two different identities.
pub(super) fn digest_values(values: &[f64]) -> Digest {
    let mut bytes = Vec::with_capacity(16 + values.len() * 8);
    bytes.extend_from_slice(b"shploingus.api-field-values/1");
    bytes.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for value in values {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    Digest::of(&bytes)
}

// ---------------------------------------------------------------------------
// The claims.
// ---------------------------------------------------------------------------

/// One thing an answer asserts, with everything needed to re-check it **against the owner it
/// describes**. A claim is not a copy of a value: it is a question the gate can ask again.
#[derive(Clone, Debug, PartialEq)]
pub enum Claim {
    /// The world's identity and truth are what the answer said.
    TruthIs {
        seed: u64,
        period_s: u64,
        tick: u64,
        recorded: Digest,
    },
    /// The declaring stage still produces these values over this request.
    FieldIs {
        field: &'static str,
        field_id: FieldId,
        request: ProcessRequest,
        columns: usize,
        recorded: Digest,
    },
    /// This geometry is what the gate holds for this chunk, level and world.
    MeshIsHeld {
        chunk: ChunkKey,
        level: u8,
        quads: usize,
        recorded: Digest,
    },
    /// The resident set holds exactly this, and this much of it is stale.
    HeldIs {
        entries: usize,
        stale: usize,
        recorded: Digest,
    },
    /// The record is this long and ends here.
    RecordHeadIs {
        events: usize,
        recorded: Option<Digest>,
    },
    /// The event this call appended is still the event at this position.
    EventIs { seq: usize, recorded: Digest },
}

impl Claim {
    pub const fn name(&self) -> &'static str {
        match self {
            Claim::TruthIs { .. } => "truth_is",
            Claim::FieldIs { .. } => "field_is",
            Claim::MeshIsHeld { .. } => "mesh_is_held",
            Claim::HeldIs { .. } => "held_is",
            Claim::RecordHeadIs { .. } => "record_head_is",
            Claim::EventIs { .. } => "event_is",
        }
    }

    /// What was recorded, in words a report can print beside the found value.
    pub fn describe(&self) -> String {
        match self {
            Claim::TruthIs { seed, period_s, tick, recorded } => format!(
                "seed={seed} period={period_s}s tick={tick} truth={}",
                recorded.to_hex()
            ),
            Claim::FieldIs { field, columns, recorded, .. } => format!(
                "{field}: {columns} columns, values={}",
                recorded.to_hex()
            ),
            Claim::MeshIsHeld { chunk, level, quads, recorded } => format!(
                "chunk=({}, {}, {}) level={level} {quads} quads, mesh={}",
                chunk.x,
                chunk.y,
                chunk.z,
                recorded.to_hex()
            ),
            Claim::HeldIs { entries, stale, recorded } => {
                format!("{entries} entries ({stale} stale), resident={}", recorded.to_hex())
            }
            Claim::RecordHeadIs { events, recorded } => {
                format!("{events} events, head={}", head_hex(*recorded))
            }
            Claim::EventIs { seq, recorded } => {
                format!("event {seq} = {}", recorded.to_hex())
            }
        }
    }

    /// **Re-check against the thing it describes.** A claim that does not hold is refused
    /// `claim_unverified` with the claim named, the recorded value and the value that was found.
    pub fn check(&self, owners: &Owners<'_>) -> Result<(), Refusal> {
        match self {
            Claim::TruthIs { seed, period_s, tick, recorded } => {
                let identity = owners.world.identity();
                let found_seed = identity.seed;
                let found_period = identity.period.get();
                let found_tick = owners.world.tick().0;
                if found_seed != *seed || found_period != *period_s || found_tick != *tick {
                    return Err(unverified(
                        self,
                        format!(
                            "recorded seed={seed} period={period_s}s tick={tick}, found seed={found_seed} \
                             period={found_period}s tick={found_tick}"
                        ),
                    ));
                }
                let found = owners.world.truth_digest();
                if found != *recorded {
                    return Err(unverified(
                        self,
                        format!(
                            "recorded truth {}, found {}",
                            recorded.to_hex(),
                            found.to_hex()
                        ),
                    ));
                }
                Ok(())
            }
            Claim::FieldIs { field, field_id, request, columns, recorded } => {
                // The one claim that is **re-evaluated**: a field has no stored form, so checking it
                // means asking its declaring stage again, over the same request.
                let values = field_at(
                    owners.roster,
                    owners.world.identity(),
                    owners.world.tick(),
                    *field_id,
                    *request,
                    owners.workers,
                )
                .map_err(|refusal| {
                    unverified(self, format!("the declaring stage could not answer: {refusal}"))
                })?;
                if values.len() != *columns {
                    return Err(unverified(
                        self,
                        format!("recorded {columns} columns, found {}", values.len()),
                    ));
                }
                let found = digest_values(&values);
                if found != *recorded {
                    return Err(unverified(
                        self,
                        format!(
                            "recorded values {} over `{field}`, found {}",
                            recorded.to_hex(),
                            found.to_hex()
                        ),
                    ));
                }
                Ok(())
            }
            Claim::MeshIsHeld { chunk, level, quads, recorded } => {
                let level = Level::new(*level).map_err(|refusal| {
                    unverified(self, format!("the level cannot be named: {refusal}"))
                })?;
                match owners.resident.serve(owners.world, *chunk, level) {
                    None => Err(unverified(
                        self,
                        format!(
                            "no geometry is held for chunk ({}, {}, {}) at level {} in this world",
                            chunk.x,
                            chunk.y,
                            chunk.z,
                            level.get()
                        ),
                    )),
                    Some(mesh) => {
                        let found = mesh.digest();
                        let found_quads = mesh.quads().len();
                        if found != *recorded || found_quads != *quads {
                            return Err(unverified(
                                self,
                                format!(
                                    "recorded {} with {quads} quads, found {} with {found_quads} quads",
                                    recorded.to_hex(),
                                    found.to_hex()
                                ),
                            ));
                        }
                        Ok(())
                    }
                }
            }
            Claim::HeldIs { entries, stale, recorded } => {
                let found_entries = owners.resident.len();
                let found_stale = owners.resident.stale(owners.world);
                let found = owners.resident.digest();
                if found_entries != *entries || found_stale != *stale || found != *recorded {
                    return Err(unverified(
                        self,
                        format!(
                            "recorded {entries} entries ({stale} stale) at {}, found {found_entries} \
                             entries ({found_stale} stale) at {}",
                            recorded.to_hex(),
                            found.to_hex()
                        ),
                    ));
                }
                Ok(())
            }
            Claim::RecordHeadIs { events, recorded } => {
                let found_events = owners.chain.len();
                let found = owners.chain.head();
                if found_events != *events || found != *recorded {
                    return Err(unverified(
                        self,
                        format!(
                            "recorded {events} events with head {}, found {found_events} events with \
                             head {}",
                            head_hex(*recorded),
                            head_hex(found)
                        ),
                    ));
                }
                Ok(())
            }
            Claim::EventIs { seq, recorded } => match owners.chain.events().get(*seq) {
                Some(event) if event.digest() == *recorded => Ok(()),
                Some(event) => Err(unverified(
                    self,
                    format!(
                        "recorded event {seq} = {}, found {}",
                        recorded.to_hex(),
                        event.digest().to_hex()
                    ),
                )),
                None => Err(unverified(
                    self,
                    format!(
                        "event {seq} ({}) is no longer in the record",
                        recorded.to_hex()
                    ),
                )),
            },
        }
    }
}

fn unverified(claim: &Claim, detail: String) -> Refusal {
    Refusal::new(
        RefusalKind::ClaimUnverified,
        format!("claim `{}`: {detail}", claim.name()),
    )
}

fn unbound(detail: String) -> Refusal {
    Refusal::new(RefusalKind::ReceiptMismatch, detail)
}

/// The claim names one row declares, spelled for a refusal's message.
fn claim_names(names: &[&str]) -> String {
    if names.is_empty() {
        return "nothing (its row declares no `claims_hold`)".to_string();
    }
    names.join(", ")
}

/// The checks a receipt carries, spelled for a refusal's message.
fn check_names(names: impl IntoIterator<Item = CheckName>) -> String {
    let names = names
        .into_iter()
        .map(CheckName::name)
        .collect::<Vec<_>>();
    if names.is_empty() {
        return "none".to_string();
    }
    names.join(", ")
}

// ---------------------------------------------------------------------------
// The effect: what the call changed.
// ---------------------------------------------------------------------------

/// What one call changed: the work U4 counted where it happened, what the gate holds before and
/// after, the record's length before and after, and the event it appended (if any).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Effect {
    pub columns_produced: u64,
    pub samples_read: u64,
    pub quads: u64,
    pub hits: u64,
    pub misses: u64,
    pub refusals: u64,
    pub held_before: usize,
    pub held_after: usize,
    pub events_before: usize,
    pub events_after: usize,
    pub appended: Option<Digest>,
}

impl Effect {
    /// Whether the call did the work of deriving something.
    pub const fn did_work(&self) -> bool {
        self.columns_produced > 0
    }

    /// Whether the call changed the record.
    pub const fn changed_the_record(&self) -> bool {
        self.events_after != self.events_before
    }

    pub fn render(&self) -> String {
        format!(
            "work {}/{} columns ({} misses, {} hits, {} refusals) -> {} quads; held {} -> {}; record \
             {} -> {} events{}",
            self.columns_produced,
            self.samples_read,
            self.misses,
            self.hits,
            self.refusals,
            self.quads,
            self.held_before,
            self.held_after,
            self.events_before,
            self.events_after,
            match self.appended {
                Some(digest) => format!(", appended {}", digest.to_hex()),
                None => ", appended nothing".to_string(),
            }
        )
    }
}

// ---------------------------------------------------------------------------
// The receipt, and the owners it is verified against.
// ---------------------------------------------------------------------------

/// **The record of one call**: the request, the answer, the claims the answer makes, the checks that
/// ran, and what the call changed. It is the unit [`Receipt::verify`] checks — and verification is
/// the only thing that makes an answer an answer this layer hands out.
#[derive(Clone, Debug, PartialEq)]
pub struct Receipt {
    pub request: String,
    pub answer: Answer,
    pub claims: Vec<Claim>,
    pub checks: Vec<Check>,
    pub effect: Effect,
}

impl Receipt {
    /// The answer's identity — a pure function of what the answer describes.
    pub fn answer_digest(&self) -> Digest {
        self.answer.digest()
    }

    /// **Bind this receipt to the row it names.** The request anchors a receipt to the table's own parse
    /// ([`Receipt::from_wire`]); this anchors its **evidence**: the answer is the answer of the verb its
    /// request names, every claim is one the row's declaration allows ([`Tool::claims`]) with at least
    /// one where the row declares [`CheckName::ClaimsHold`], and the checks that ran are exactly the
    /// checks the row declares, in the declared order.
    ///
    /// Without this, verification over an **emptied** claims field finds nothing wrong and returns `Ok`:
    /// a receipt that did not match its row would verify by having nothing left to verify. A receipt is
    /// the evidence for an answer — evidence that was removed is not evidence that holds.
    ///
    /// One place decides it, and both readers ask it: [`Receipt::from_wire`] refuses a receipt that does
    /// not bind, and [`Receipt::verify`] refuses one that was built in this process and does not.
    pub fn bind_to_row(&self) -> Result<(), Refusal> {
        let tool = parse(&self.request)?.tool();
        if self.answer.verb() != tool.name {
            return Err(unbound(format!(
                "the request is `{}` and the answer is a `{}` answer",
                tool.name,
                self.answer.verb()
            )));
        }
        let declares_claims = tool.checks.contains(&CheckName::ClaimsHold);
        if declares_claims && self.claims.is_empty() {
            return Err(unbound(format!(
                "`{}` declares `{}` and its receipt carries no claim: there is no evidence here to \
                 re-check",
                tool.name,
                CheckName::ClaimsHold.name()
            )));
        }
        // Every claim must be one **this row declares**: a receipt carrying another row's claim would be
        // re-checked against an owner that has nothing to do with this answer — a `held` receipt claiming
        // `record_head_is` holds wherever the record is its own length — and a row that declares no claim
        // at all (the `admit` row) therefore refuses any claim it is handed.
        for claim in &self.claims {
            if !tool.claims.contains(&claim.name()) {
                return Err(unbound(format!(
                    "`{}` may claim {} and this receipt claims `{}`",
                    tool.name,
                    claim_names(tool.claims),
                    claim.name()
                )));
            }
        }
        let ran = self.checks.iter().map(|check| check.name).collect::<Vec<_>>();
        if !ran.iter().copied().eq(tool.checks.iter().copied()) {
            return Err(unbound(format!(
                "`{}` declares the {} check(s) {} and its receipt carries {}: {}",
                tool.name,
                tool.checks.len(),
                check_names(tool.checks.iter().copied()),
                ran.len(),
                check_names(ran.into_iter())
            )));
        }
        Ok(())
    }

    /// Re-check **every** claim against the owners, collecting **every** failure rather than the
    /// first. A receipt verified against the world it was made in passes; the same receipt verified
    /// against a world that has since moved refuses by name (SC-004) — and one that does not match the
    /// row it names refuses by name before a single claim is re-checked, because a receipt whose
    /// evidence is not its row's evidence is not a receipt for that row.
    ///
    /// **An admission is re-established rather than believed.** The `admit` row is the one row that
    /// makes no claim, so a receipt for it holds no evidence to re-check — and a receipt with no
    /// evidence can be edited into evidence of an admission that never happened: the request's two
    /// references and its requirements, and the answer's own axis states, are all carried values. So
    /// verification asks the seam again, through [`Receipt::re_establish`], and holds the receipt to
    /// what the request and the world establish now. Every other row's evidence is the claims it
    /// carries, and those are re-checked here as they always were.
    pub fn verify(&self, owners: &Owners<'_>) -> Result<(), RefusalSet> {
        let mut set = RefusalSet::new();
        if let Err(refusal) = self.bind_to_row() {
            set.push(refusal);
        }
        if matches!(self.answer, Answer::Admitted { .. }) {
            if let Err(refusals) = self.re_establish(owners) {
                for refusal in refusals.all() {
                    set.push(refusal.clone());
                }
            }
        }
        // The two save rows make no claim either, and for the same reason: what they are about is a
        // **document**, and a claim's owners are the world's own seams. So they are re-established too
        // — the declared reference is read again, in place, through the gate's own door.
        if self.answer.document().is_some() {
            if let Err(refusals) = self.re_establish_document(owners) {
                for refusal in refusals.all() {
                    set.push(refusal.clone());
                }
            }
        }
        for claim in &self.claims {
            if let Err(refusal) = claim.check(owners) {
                set.push(refusal);
            }
        }
        set.into_result(())
    }

    /// **Read the request as the row it declares, ask the seam what that row is now, and hold the
    /// carried answer to it.** This is the whole of the `admit` row's verification, because that row
    /// makes no claim: an admission is not a statement about a stored thing (a digest, a held mesh, a
    /// record head) but a decision made *about* a document where it lives — so the honest way to check
    /// one is to decide it again.
    ///
    /// Three kinds of edit are caught and none of them is trusted:
    ///
    /// * **A request that no longer describes a row the world can decide** — a document that is not
    ///   readable at the path it names (`source_absent`), bytes that have moved under it
    ///   (`source_digest_mismatch`), a stage the roster does not declare (`stage_undeclared`), a
    ///   material the registry never admitted (`unadmitted_material`), a licence word or a claim the
    ///   gate does not declare. Each is the owner's own refusal, by name: a receipt that cannot be
    ///   re-established because the document or the declaration it cites is not reachable refuses
    ///   rather than passing.
    /// * **An answer the request and the world do not support** — a carried feature, claim, document
    ///   digest or axis state that differs from the one the seam establishes now refuses
    ///   `receipt_mismatch`, with the fact and both values in the message. This is what a doctored
    ///   state (`conformance=active` where the row requires nothing the world holds, `identity=active`
    ///   where the claim establishes no identity) runs into.
    /// * **An effect an admission cannot have had** — it appends nothing, holds nothing new, moves no
    ///   record and derives nothing, so a receipt claiming otherwise refuses `receipt_mismatch`. Its
    ///   *absolute* positions are the call's and not the receipt's (the record has moved since), so
    ///   they are compared with themselves rather than re-derived.
    fn re_establish(&self, owners: &Owners<'_>) -> Result<(), RefusalSet> {
        let mut set = RefusalSet::new();
        let request = match parse(&self.request) {
            Ok(request) => request,
            Err(refusal) => {
                set.push(refusal);
                return Err(set);
            }
        };
        let row = match request.row() {
            Ok(row) => row,
            Err(refusal) => {
                set.push(refusal);
                return Err(set);
            }
        };
        // The seam answers, or refuses by its own name: a document that is not there, a stage nobody
        // declares, a material never admitted.
        let established = admitted_answer(&row, owners.intake)?;

        let Answer::Admitted { feature, claim, found, axes } = &self.answer else {
            return Ok(());
        };
        let Answer::Admitted {
            feature: established_feature,
            claim: established_claim,
            found: established_found,
            axes: established_axes,
        } = &established
        else {
            // `admitted_answer` answers with an admission or refuses: nothing else can arrive here.
            return Ok(());
        };

        if feature != established_feature {
            set.push(unbound(format!(
                "this receipt carries the feature `{feature}` and the request declares \
                 `{established_feature}`"
            )));
        }
        if claim != established_claim {
            set.push(unbound(format!(
                "this receipt says the row was admitted as `{}` and the request declares `{}`",
                claim.name(),
                established_claim.name()
            )));
        }
        if found != established_found {
            set.push(unbound(format!(
                "this receipt carries the document digest {} and the bytes the request names hash to \
                 {}",
                found.to_hex(),
                established_found.to_hex()
            )));
        }
        for (axis, (carried, established)) in
            Axis::ALL.iter().zip(axes.iter().zip(established_axes.iter()))
        {
            if carried != established {
                set.push(unbound(format!(
                    "the request and the world establish `{}={}` and this receipt carries \
                     `{}={}`",
                    axis.name(),
                    established.name(),
                    axis.name(),
                    carried.name()
                )));
            }
        }

        let effect = &self.effect;
        if let Some(appended) = effect.appended {
            set.push(unbound(format!(
                "an admission appends nothing and this receipt carries the appended event {}",
                appended.to_hex()
            )));
        }
        if effect.held_before != effect.held_after {
            set.push(unbound(format!(
                "an admission holds nothing new and this receipt moves the geometry from {} to {}",
                effect.held_before, effect.held_after
            )));
        }
        if effect.events_before != effect.events_after {
            set.push(unbound(format!(
                "an admission changes no record and this receipt moves it from {} to {} event(s)",
                effect.events_before, effect.events_after
            )));
        }
        if effect.columns_produced != 0
            || effect.samples_read != 0
            || effect.quads != 0
            || effect.hits != 0
            || effect.misses != 0
            || effect.refusals != 0
        {
            set.push(unbound(format!(
                "an admission derives nothing and this receipt carries the work it did: {}/{} \
                 column(s) ({} misses, {} hits, {} refusals) -> {} quad(s)",
                effect.columns_produced,
                effect.samples_read,
                effect.misses,
                effect.hits,
                effect.refusals,
                effect.quads
            )));
        }
        set.into_result(())
    }

    /// **Re-establish a save document's receipt from its request.** The other family with no evidence
    /// to re-check: a saved or loaded document's answer carries a path, a digest, an identity and a
    /// record, and every one of those is a value a writer could have chosen. So verification asks
    /// U9's seam again ([`document_facts`]) — the reference is read **in place** through the content
    /// gate, parsed, and loaded against the ruleset this gate declares — and holds the carried answer
    /// to what that establishes now.
    ///
    /// The two rows differ in one thing, and it is the request's to say: a `load` line **declares** the
    /// digest it expects (so the document it reads is the one the line names), while a `save` line
    /// declares only where it wrote and the answer carries the digest it placed there. In both cases
    /// the digest compared against the bytes is the carried answer's — a doctored digest therefore
    /// refuses `source_digest_mismatch`, which is the document's own name for *these are not the bytes
    /// you describe* — and for `load` the request's digest must equal it as well.
    ///
    /// Every edit this catches is named: a document that is gone (`source_absent`), one whose bytes
    /// have moved (`source_digest_mismatch`), bytes that are not a save (`not_a_save`,
    /// `save_truncated`, `save_tampered`), a save of another world or ruleset (`identity_mismatch`,
    /// `record_diverged`), a carried fact that disagrees with what the document and the world establish
    /// (`receipt_mismatch`, with both values), and an effect a document row cannot have had — it
    /// appends nothing, holds nothing, derives nothing and moves no record.
    fn re_establish_document(&self, owners: &Owners<'_>) -> Result<(), RefusalSet> {
        let mut set = RefusalSet::new();
        let request = match parse(&self.request) {
            Ok(request) => request,
            Err(refusal) => {
                set.push(refusal);
                return Err(set);
            }
        };
        let path = match request.text("path") {
            Ok(path) => path,
            Err(refusal) => {
                set.push(refusal);
                return Err(set);
            }
        };
        let carried = match self.answer.document() {
            Some(facts) => facts,
            None => return Ok(()),
        };
        // A `load` request declares the digest; a `save` request does not, and the answer's own digest
        // is then the one the bytes must hash to.
        let digest = match request.value("digest") {
            Some(_) => match request.digest("digest") {
                Ok(digest) => digest,
                Err(refusal) => {
                    set.push(refusal);
                    return Err(set);
                }
            },
            None => carried.digest,
        };
        let established = document_facts(owners.intake, owners.ruleset, &Source::new(path, digest))?;

        // Every carried fact, against the fact the document and the world establish now: one name for
        // the disagreement, both values in the message, and the field named.
        let mut disagree = |field: &str, carried: String, established: String| {
            if carried != established {
                set.push(unbound(format!(
                    "this receipt carries {field} = {carried} and the document and the world \
                     establish {field} = {established}"
                )));
            }
        };
        disagree("path", carried.path.clone(), established.path.clone());
        disagree("digest", carried.digest.to_hex(), established.digest.to_hex());
        disagree("length", carried.len.to_string(), established.len.to_string());
        disagree("seed", carried.seed.to_string(), established.seed.to_string());
        disagree(
            "declared period",
            carried.period_s.to_string(),
            established.period_s.to_string(),
        );
        disagree("tick", carried.tick.to_string(), established.tick.to_string());
        disagree(
            "ruleset",
            format!("{} revision {}", carried.ruleset.ruleset, carried.ruleset.revision),
            format!("{} revision {}", established.ruleset.ruleset, established.ruleset.revision),
        );
        disagree("truth", carried.truth.to_hex(), established.truth.to_hex());
        disagree(
            "record length",
            carried.record_len.to_string(),
            established.record_len.to_string(),
        );
        disagree(
            "record head",
            head_hex(carried.record_head),
            head_hex(established.record_head),
        );

        let effect = &self.effect;
        if let Some(appended) = effect.appended {
            set.push(unbound(format!(
                "a save document changes no record and this receipt carries the appended event {}",
                appended.to_hex()
            )));
        }
        if effect.held_before != effect.held_after {
            set.push(unbound(format!(
                "a save document holds nothing and this receipt moves the geometry from {} to {}",
                effect.held_before, effect.held_after
            )));
        }
        if effect.events_before != effect.events_after {
            set.push(unbound(format!(
                "a save document changes no record and this receipt moves it from {} to {} event(s)",
                effect.events_before, effect.events_after
            )));
        }
        if effect.columns_produced != 0
            || effect.samples_read != 0
            || effect.quads != 0
            || effect.hits != 0
            || effect.misses != 0
            || effect.refusals != 0
        {
            set.push(unbound(format!(
                "a save document derives nothing and this receipt carries the work it did: {}/{} \
                 column(s) ({} misses, {} hits, {} refusals) -> {} quad(s)",
                effect.columns_produced,
                effect.samples_read,
                effect.misses,
                effect.hits,
                effect.refusals,
                effect.quads
            )));
        }
        set.into_result(())
    }

    pub fn render(&self) -> String {
        let checks = self
            .checks
            .iter()
            .map(|check| {
                let outcome = match &check.outcome {
                    CheckOutcome::Passed => "passed".to_string(),
                    CheckOutcome::Refused(refusal) => format!("refused: {refusal}"),
                };
                format!("{}[{outcome}] ({})", check.name.name(), check.subject)
            })
            .collect::<Vec<_>>()
            .join("; ");
        let claims = self
            .claims
            .iter()
            .map(|claim| format!("{} ({})", claim.name(), claim.describe()))
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "request {}\n  answer {}\n  answer digest {}\n  checks {checks}\n  claims {claims}\n  \
             effect {}",
            self.request,
            self.answer.render(),
            self.answer.digest().to_hex(),
            self.effect.render()
        )
    }
}

/// **The owners, borrowed**: everything a claim is checked against, and nothing the gate owns. A
/// receipt can therefore be verified against a *different* world than the one it was made in — which
/// is exactly the check that an answer corresponds to the world rather than to itself.
#[derive(Clone, Copy, Debug)]
pub struct Owners<'a> {
    pub world: &'a World,
    pub roster: &'a Declared,
    pub resident: &'a Resident,
    pub chain: &'a Chain,
    /// The worker count a field claim is re-evaluated with. It is a pure performance knob (U3), so
    /// re-checking at a different count is the same values and the same claim.
    pub workers: usize,
    /// **The ruleset identity this gate saves under and loads into** — U9's own declaration, carried
    /// beside the world rather than held as a knob: a save names the configuration it was written
    /// under and a load declares the one it loads into, so a receipt for a save row can only be
    /// re-established (the document read again, parsed, and loaded) if the verifier declares the same
    /// identity the gate did. There is no default: a gate is built with one.
    pub ruleset: Ruleset,
    /// **The content gate's door**: the root a row's references resolve against, and the world's own
    /// declarations a row is conformed against. Verification needs it for the same reason a call does
    /// — an admission is re-established by asking this door again ([`Receipt::re_establish`]) — so it
    /// travels with the owners rather than being a knob a verifier could leave out. It is **required**
    /// on purpose: an admission receipt verified without a door would have nothing to re-read, and a
    /// missing declaration must refuse rather than pass.
    pub intake: &'a Intake<'a>,
}

impl<'a> Owners<'a> {
    pub const fn new(
        world: &'a World,
        roster: &'a Declared,
        resident: &'a Resident,
        chain: &'a Chain,
        workers: usize,
        ruleset: Ruleset,
        intake: &'a Intake<'a>,
    ) -> Self {
        Owners { world, roster, resident, chain, workers, ruleset, intake }
    }
}

