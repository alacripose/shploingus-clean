use super::*;
// ---------------------------------------------------------------------------
// The gate.
// ---------------------------------------------------------------------------

/// **The gate.** It owns the declaration it dispatches through (U3's roster and U4's ladder and
/// residency horizon), the world whose truth it reads (U2's), the record it appends to (U2's chain),
/// the geometry cache (U4's `Resident`) and the work ledger U4 counts into. It copies none of them —
/// including the two declarations a row's **conformance** is judged against: U3's roster, which the
/// gate dispatches through anyway, and U2's material registry, which it holds **borrowed** so an
/// admission can ask the world what it is made of without this module keeping a copy of the answer.
pub struct Gate<'a> {
    roster: &'a Declared,
    mesher: Mesher<'a>,
    world: World,
    chain: Chain,
    resident: Resident,
    ledger: WorkLedger,
    budget: Budget,
    workers: usize,
    /// **The ruleset identity this gate saves under and loads into** — U9's own declaration, and the
    /// only one of this gate's declarations that is a value rather than a borrow, because a save names
    /// it and a load declares it against the document rather than against another owner's seam. It is a
    /// **required** constructor argument for the reason the registry is: a `load` row with no identity
    /// to load into would have nothing to answer with, and a default would silently accept a save from
    /// another world.
    ruleset: Ruleset,
    /// The content gate's door: the root a `admit` row's references are resolved against, and the
    /// world's own declarations a row's **conformance** is judged against. The root is pinned with
    /// [`Gate::set_intake_root`] when the caller's content lives somewhere specific (a fixture, a mod
    /// tree); otherwise a relative reference means what a relative path always means — the process's
    /// own directory — which is the definition of a relative path rather than a value the gate
    /// invents. The two declarations are not a knob: they are the same roster and registry this gate
    /// was built over, borrowed, so conformance is against the world rather than against a list.
    intake: Intake<'a>,
    /// **Who is acting through this gate**, and with what authority
    /// ([`crate::policy`], `docs/HYPERGRILLING.md` §L-5). Like the ruleset it is a **required**
    /// constructor argument: a gate with no declared actor would be a gate answering as nobody, and the
    /// one decision function would have nothing to decide about. It is a value rather than a borrow
    /// because a session's actor is a declaration about the session, and it is the one thing a caller
    /// may pin differently with [`Gate::set_caller`] — the same shape as [`Gate::set_intake_root`].
    caller: policy::Caller,
}

/// What the gate held before a call, so the call's effect is a **delta** rather than a total.
#[derive(Clone, Copy)]
struct Counters {
    ledger: WorkLedger,
    held: usize,
    events: usize,
}

/// What one verb answered: the answer, the claims it makes, and — when the call **derived** geometry
/// rather than served it — the mesh whose derivation the record now holds. Gate machinery, never
/// part of a response: a receipt is what a caller sees, and this is what the gate holds in between.
struct Answered {
    answer: Answer,
    claims: Vec<Claim>,
    derived: Option<Mesh>,
}

impl<'a> Gate<'a> {
    /// A gate over a fresh world at tick 0 and an empty record. The budget is unlimited and the worker
    /// count is one: both are this gate's declared knobs, and neither changes an answer — which is why
    /// they are **setters** ([`Gate::set_budget`], [`Gate::set_workers`]) rather than constructor
    /// arguments: what a gate cannot be built without is what it *is* (the world, the record's ruleset,
    /// and who acts), and what it merely runs at is a knob.
    ///
    /// **The registry is the world's**, not the gate's: it arrives beside the roster, which the gate
    /// already dispatches through, so an admission that requires materials conforms against the same
    /// declaration the geometry was built from. A gate cannot be built without one, because a row that
    /// requires materials would otherwise have nothing to be judged against — and an axis nothing can
    /// consult is the one thing this gate never reports as a pass.
    pub fn new(
        roster: &'a Declared,
        registry: &'a MaterialRegistry,
        ladder: Ladder,
        residency: ResidencyHorizon,
        identity: WorldIdentity,
        ruleset: Ruleset,
        caller: policy::Caller,
    ) -> Self {
        Gate {
            roster,
            mesher: Mesher::new(roster, ladder, residency),
            world: World::new(identity),
            chain: Chain::new(),
            resident: Resident::new(),
            ledger: WorkLedger::default(),
            budget: Budget::unlimited(),
            workers: 1,
            ruleset,
            intake: Intake::over(".", WorldDeclarations { roster, registry }),
            caller,
        }
    }

    /// **The ruleset identity this gate saves under and loads into.** Read by the two save rows and
    /// carried in [`Gate::owners`], so a receipt for either of them is re-established against the same
    /// declaration the call answered with — a verifier that declared another identity would be
    /// verifying an answer about another world.
    pub const fn ruleset(&self) -> Ruleset {
        self.ruleset
    }

    /// Pin the root a `admit` row's references resolve against. The gate owns no content and no
    /// document: this is where the caller declares which tree the rows' paths are relative to.
    pub fn set_intake_root(&mut self, root: impl Into<std::path::PathBuf>) {
        self.intake.set_root(root);
    }

    /// **Who is acting, restated.** A session's actor is declared when the gate is built; a caller
    /// that wants to act as someone else — the evidence does, to show a denial by name — says so here
    /// rather than by building a second world. The value is a declaration, never a per-request field:
    /// a request line cannot name its own actor, because a caller that could nominate the actor it is
    /// checked against would be authorising itself.
    pub fn set_caller(&mut self, caller: policy::Caller) {
        self.caller = caller;
    }

    /// The actor this gate answers as, and the grants it holds.
    pub const fn caller(&self) -> &policy::Caller {
        &self.caller
    }

    /// The one cost knob a caller can set: a ceiling on a derivation's work. U4 refuses past it by
    /// name; the gate passes that name through and records it.
    pub fn set_budget(&mut self, budget: Budget) {
        self.budget = budget;
    }

    /// The other declared knob: how many workers a derivation may use. One answer whatever it is set
    /// to (U3's invariant, re-measured by U4 and by U9's rebuilt world), which is why it is a knob
    /// rather than part of what the gate is.
    pub fn set_workers(&mut self, workers: usize) {
        self.workers = workers;
    }

    pub const fn world(&self) -> &World {
        &self.world
    }

    /// The world, to be moved through **U2's own methods** — a new tick is a new truth, and the gate
    /// does not invent a way to change one.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub const fn chain(&self) -> &Chain {
        &self.chain
    }

    /// The record, to be read or appended to through **U2's own writer**. U6 is not the only writer
    /// of the chain; it is only forbidden a *second* record.
    pub fn chain_mut(&mut self) -> &mut Chain {
        &mut self.chain
    }

    pub const fn resident(&self) -> &Resident {
        &self.resident
    }

    /// The geometry cache, to be evicted through **U4's own exact invalidation** — the gate owns it,
    /// but nothing about which entries are coherent is the gate's decision.
    pub fn resident_mut(&mut self) -> &mut Resident {
        &mut self.resident
    }

    /// The owners this gate's answers are checked against — the same borrows [`Receipt::verify`] takes,
    /// and the same content gate's door a `admit` call went through: an admission's receipt is
    /// re-established by asking that door again, so it belongs to the owners rather than to the caller.
    pub fn owners(&self) -> Owners<'_> {
        Owners::new(
            &self.world,
            self.roster,
            &self.resident,
            &self.chain,
            self.workers,
            self.ruleset,
            &self.intake,
        )
    }

    /// The content gate's door this gate dispatches `admit` through: the root a row's references resolve
    /// against, and the world's own declarations they are judged against.
    pub const fn intake(&self) -> &Intake<'a> {
        &self.intake
    }

    /// **Hand the gate one declared request line.** A request that does not parse is refused and
    /// writes nothing. A request the world cannot honour is refused **by the owner's own name**, and
    /// that refusal is recorded, because it is a fact about the world. Anything else returns a
    /// [`Receipt`] whose claims hold against the owners.
    pub fn call(&mut self, line: &str) -> Result<Receipt, Refusal> {
        let request = parse(line)?;

        // **The one call site.** Every served row passes through the single decision function before it
        // dispatches, and the act it is checked by comes from the row's own declaration
        // ([`Tool::action`]) — so a door cannot be added without naming its act, and an act's class is
        // never a property of the door (`docs/HYPERGRILLING.md` §L-5: *"the one call site nobody
        // remembered cannot be the one that fails"*). A denial is refused **by name** and **recorded**
        // through the record's one writer, for the reason the record gives: a refusal nobody can read is
        // a refusal that did not happen.
        if let Err(denial) = policy::decide(&self.caller, request.tool.action) {
            self.record_refusal(request.canonical(), &denial)?;
            return Err(denial);
        }

        let before = self.counters();

        let answered = match request.tool.verb {
            Verb::Truth => Ok(self.answer_truth()),
            Verb::Fields => self.answer_fields(&request),
            Verb::Geometry => self.answer_geometry(&request),
            Verb::Held => Ok(self.answer_held()),
            Verb::Record => Ok(self.answer_record()),
            Verb::Admit => self.answer_admit(&request),
            Verb::Save => self.answer_save(&request),
            Verb::Load => self.answer_load(&request),
        };
        let mut answered = match answered {
            Ok(answered) => answered,
            Err(refusal) => {
                // Recording can itself fail (a payload the wire form cannot frame); when it does,
                // that refusal names both facts and *is* the answer.
                self.record_refusal(request.canonical(), &refusal)?;
                return Err(refusal);
            }
        };

        // A change to the record is appended **through the chain's one writer**, and only what
        // changed: a read appends nothing, and a cache hit appends nothing.
        let appended = match answered.derived.take() {
            Some(mesh) if request.tool.changes_record => {
                let key = mesh.key();
                let payload = format!(
                    "request={} chunk=({}, {}, {}) level={} quads={} state={} mesh={}",
                    request.canonical(),
                    key.chunk.x,
                    key.chunk.y,
                    key.chunk.z,
                    key.level.get(),
                    mesh.quads().len(),
                    key.state.to_hex(),
                    mesh.digest().to_hex()
                );
                let digest = self
                    .chain
                    .append(Namespace::Record, EventKind::GeometryDerived, payload)?
                    .digest();
                // The event is claimed too, so a receipt says the record holds it — and verification
                // re-reads the chain at that position rather than trusting the receipt's own copy.
                answered.claims.push(Claim::EventIs {
                    seq: self.chain.len() - 1,
                    recorded: digest,
                });
                Some(digest)
            }
            _ => None,
        };

        let checks = self.run_checks(&request, &answered, appended, before.events);
        if let Some(refusal) = checks.iter().find_map(|check| match &check.outcome {
            CheckOutcome::Refused(refusal) => Some(refusal.clone()),
            CheckOutcome::Passed => None,
        }) {
            self.record_refusal(request.canonical(), &refusal)?;
            return Err(refusal);
        }

        let effect = Effect {
            columns_produced: self.ledger.columns_produced - before.ledger.columns_produced,
            samples_read: self.ledger.samples_read - before.ledger.samples_read,
            quads: self.ledger.quads - before.ledger.quads,
            hits: self.ledger.hits - before.ledger.hits,
            misses: self.ledger.misses - before.ledger.misses,
            refusals: self.ledger.refusals - before.ledger.refusals,
            held_before: before.held,
            held_after: self.resident.len(),
            events_before: before.events,
            events_after: self.chain.len(),
            appended,
        };
        Ok(Receipt {
            request: request.canonical().to_string(),
            answer: answered.answer,
            claims: answered.claims,
            checks,
            effect,
        })
    }

    fn counters(&self) -> Counters {
        Counters { ledger: self.ledger, held: self.resident.len(), events: self.chain.len() }
    }

    /// Record a refusal through the chain's one writer. If the record cannot record it — a payload
    /// that cannot ride the wire form — the returned refusal **names both facts**: that the record
    /// failed and which refusal it was, so neither is dropped.
    ///
    /// **What is recorded is the decision, not the explanation.** The payload is the canonical request
    /// the gate judged and the name it judged it by, and it carries **no detail** — because the chain
    /// hashes this payload into the record's own identity, and a refusal's detail is prose for the
    /// caller: it may quote **this host** (an operating system's error text, a path as this host
    /// normalized it). A detail here would put this host's wording into the record, so one world
    /// refusing one request on two hosts would keep two different records. Everything the refusal is
    /// *about* the request already carries — the path, the chunk, the level the row declared — and the
    /// words belong to the answer.
    pub(super) fn record_refusal(&mut self, request: &str, refusal: &Refusal) -> Result<(), Refusal> {
        let payload = format!("request={request} kind={}", refusal.kind().name());
        match self.chain.append(Namespace::Record, EventKind::RefusalRecorded, payload) {
            Ok(_) => Ok(()),
            Err(record) => Err(Refusal::new(
                record.kind(),
                format!(
                    "could not record the refusal [{}] {}: {record}",
                    refusal.kind().name(),
                    refusal.detail()
                ),
            )),
        }
    }

    /// Run the checks the row declares, in the declared order — and **re-check the claims here**,
    /// where the answer is either handed out or refused.
    fn run_checks(
        &self,
        request: &Request,
        answered: &Answered,
        appended: Option<Digest>,
        events_before: usize,
    ) -> Vec<Check> {
        let owners = self.owners();
        let mut claim_failures = RefusalSet::new();
        for claim in &answered.claims {
            if let Err(refusal) = claim.check(&owners) {
                claim_failures.push(refusal);
            }
        }
        let mut checks = Vec::with_capacity(request.tool.checks.len());
        for name in request.tool.checks {
            let (subject, outcome) = match name {
                CheckName::RequestParsed => (request.canonical.clone(), CheckOutcome::Passed),
                CheckName::ArgumentsTyped => (typed_subject(request), CheckOutcome::Passed),
                CheckName::OwnerAnswered => (
                    request.tool.owner.to_string(),
                    CheckOutcome::Passed,
                ),
                CheckName::ClaimsHold => match claim_failures.all().first() {
                    None => (claim_subject(&answered.claims), CheckOutcome::Passed),
                    Some(refusal) => {
                        (claim_subject(&answered.claims), CheckOutcome::Refused(refusal.clone()))
                    }
                },
                CheckName::Recorded => {
                    let grew = self.chain.len() == events_before + usize::from(appended.is_some());
                    let subject = match appended {
                        Some(digest) => format!(
                            "event {} = {}",
                            self.chain.len().saturating_sub(1),
                            digest.to_hex()
                        ),
                        None => "nothing appended".to_string(),
                    };
                    if grew {
                        (subject, CheckOutcome::Passed)
                    } else {
                        (
                            subject,
                            CheckOutcome::Refused(Refusal::new(
                                RefusalKind::ChainBroken,
                                "the record's length does not match what this call changed",
                            )),
                        )
                    }
                }
            };
            checks.push(Check { name: *name, subject, outcome });
        }
        checks
    }

    // ---- One answer per verb, each through its owner's seam.

    fn answer_truth(&self) -> Answered {
        let identity = self.world.identity();
        let truth = self.world.truth_digest();
        Answered {
            answer: Answer::Truth {
                seed: identity.seed,
                period_s: identity.period.get(),
                tick: self.world.tick().0,
                truth,
            },
            claims: vec![Claim::TruthIs {
                seed: identity.seed,
                period_s: identity.period.get(),
                tick: self.world.tick().0,
                recorded: truth,
            }],
            derived: None,
        }
    }

    fn answer_fields(&self, request: &Request) -> Result<Answered, Refusal> {
        let field = request.field("field")?;
        let origin = request.voxel("at")?;
        let side = request.side("side")?;
        let ask = ProcessRequest::Region { origin, side };
        let values = field_at(
            self.roster,
            self.world.identity(),
            self.world.tick(),
            field,
            ask,
            self.workers,
        )?;
        let recorded = super::receipt::digest_values(&values);
        let low = values.iter().copied().fold(f64::INFINITY, f64::min);
        let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Ok(Answered {
            answer: Answer::Fields {
                field: field.name(),
                columns: values.len(),
                values: recorded,
                low,
                high,
            },
            claims: vec![Claim::FieldIs {
                field: field.name(),
                field_id: field,
                request: ask,
                columns: values.len(),
                recorded,
            }],
            derived: None,
        })
    }

    fn answer_geometry(&mut self, request: &Request) -> Result<Answered, Refusal> {
        let chunk = request.chunk("chunk")?;
        let level = request.level("level")?;
        let hits_before = self.ledger.hits;
        let mesh = self.mesher.derive(
            &self.world,
            chunk,
            level,
            Effort::new(self.budget, self.workers),
            &mut self.resident,
            &mut self.ledger,
        )?;
        let served = self.ledger.hits > hits_before;
        let truth = self.world.truth_digest();
        let answer = Answer::Geometry {
            chunk,
            level: level.get(),
            quads: mesh.quads().len(),
            faces: mesh.area(),
            digest: mesh.digest(),
            truth,
        };
        let claims = vec![
            Claim::TruthIs {
                seed: self.world.identity().seed,
                period_s: self.world.identity().period.get(),
                tick: self.world.tick().0,
                recorded: truth,
            },
            Claim::MeshIsHeld {
                chunk,
                level: level.get(),
                quads: mesh.quads().len(),
                recorded: mesh.digest(),
            },
        ];
        // A **miss** is a change the record will hold, so the mesh travels back to the caller; a hit
        // is nothing to record, and the receipt then claims nothing about the record's length.
        let derived = if served { None } else { Some(mesh) };
        Ok(Answered { answer, claims, derived })
    }

    fn answer_held(&self) -> Answered {
        let entries = self.resident.len();
        let stale = self.resident.stale(self.world.truth_digest());
        let digest = self.resident.digest();
        Answered {
            answer: Answer::Held { entries, stale, digest },
            claims: vec![Claim::HeldIs { entries, stale, recorded: digest }],
            derived: None,
        }
    }

    fn answer_record(&self) -> Answered {
        let events = self.chain.len();
        let head = self.chain.head();
        Answered {
            answer: Answer::Record { events, head },
            claims: vec![Claim::RecordHeadIs { events, recorded: head }],
            derived: None,
        }
    }

    /// **The content gate's door.** A declared row arrives **by reference** — its document and its own
    /// acceptance check each as `path + digest`, its feature, licence verdict, claim and carrier as
    /// declared words — and the answer is what the row became. The document is read **where it lives**
    /// (never carried in, never copied into a value) and judged on every declared axis; the verdict is
    /// not a `bool`, but a [`crate::intake::Established`] that says what *was* proven.
    ///
    /// This row makes **no claim**, and says so by not declaring [`CheckName::ClaimsHold`]: the
    /// evidence an admission carries is the digest the bytes were found to hash to, measured by the
    /// seam that read them — re-checking *that* needs the file, which is not one of the owners the
    /// gate's claims are re-checked against. A claim nothing can re-check is worse than no claim.
    ///
    /// **So an admission's receipt is re-established rather than believed.** It is the one receipt with
    /// no evidence in it to re-run, and every value it carries — the two references, the requirements,
    /// and the answer's axis states — is a carried value a writer could have chosen. [`Receipt::verify`]
    /// therefore reads the request as the row it declares, asks **this** seam what that row is now, and
    /// refuses any fact the request and the world do not support: the row and its answer are built in
    /// one place ([`Request::row`] and [`super::receipt::admitted_answer`]), so a call and a re-check
    /// cannot drift into two admissions.
    ///
    /// A row that refuses is refused by the intake's own name with the values in the message, and —
    /// because the line **parsed** and the content tree is part of the world this gate answers about —
    /// that refusal is recorded like any other world-level refusal. A line that does not parse still
    /// writes nothing.
    ///
    /// **Conformance is the last column, and it is judged against the world rather than against a
    /// list.** `requires_stages` and `requires_materials` are optional — a row that requires nothing of
    /// the world **reports** the axis as unproven rather than counting it as met — and a row that does
    /// require something is conformed against the two declarations this gate was built over: U3's
    /// roster, and U2's registry. What the wire cannot spell today, and what therefore stays in the
    /// intake's vocabulary rather than being invented here, is a requirement declared and **owed**: a
    /// key with no names is refused by the table's own shape rule (`argument_malformed`: nothing is not
    /// a value of a declared kind, the way a zero-column patch is not), exactly as an owed *acceptance
    /// check* is unreachable over the wire because `check` is a required reference.
    fn answer_admit(&self, request: &Request) -> Result<Answered, Refusal> {
        // The row the line declares, and what it becomes — both through the one place that decides
        // them ([`Request::row`], [`super::receipt::admitted_answer`]), which is what lets
        // verification re-establish a receipt from its request rather than believe it.
        let row = request.row()?;
        // The whole set, as one refusal ([`joined_refusals`]): a channel that carries one refusal
        // carries every name in it.
        let answer = super::receipt::admitted_answer(&row, &self.intake)
            .map_err(|refusals| {
                joined_refusals(
                    refusals,
                    RefusalKind::AxisUnchecked,
                    "a row was refused without a named reason",
                )
            })?;
        Ok(Answered { answer, claims: Vec::new(), derived: None })
    }
}

impl Gate<'_> {
    /// **U9's document form, written.** The world this gate serves is written down by U9's one encoder
    /// ([`Save::of`], so a save never carries a record it could not re-adopt) and placed by U9's one
    /// writer ([`Document::write`]) at the declared reference **under this gate's own root** — the root
    /// its `admit` rows are read against, so a session cannot write a save where it would not look.
    ///
    /// The answer is what is now at that path: the digest those bytes hash to, their length, and the
    /// world and record they carry. It changes the world in **nothing** — the record is not touched,
    /// because writing a world down is not an event about it (F1: a read is not a change).
    ///
    /// What is *not* built here is a second reading of the document: the facts this answer carries are
    /// the writer's own, and the same facts are derived from the bytes at verification time by
    /// [`super::receipt::document_facts`] — so a call and a re-check of one document cannot drift apart.
    fn answer_save(&self, request: &Request) -> Result<Answered, Refusal> {
        let path = request.text("path")?;
        let bytes = Save::of(&self.world, &self.chain, self.ruleset).map_err(|refusals| {
            joined_refusals(refusals, RefusalKind::ChainBroken, "the world could not be written down")
        })?;
        let document = Document::write(self.intake.root(), &path, &bytes).map_err(|refusals| {
            joined_refusals(
                refusals,
                RefusalKind::DocumentUnwritable,
                "the document could not be written",
            )
        })?;
        let facts = DocumentFacts {
            path: document.path().to_string(),
            digest: document.digest(),
            len: document.len(),
            seed: self.world.identity().seed,
            period_s: self.world.identity().period.get(),
            tick: self.world.tick().0,
            ruleset: self.ruleset,
            // The truth of the world that was written down: derived here the way a report derives it,
            // never read out of the save (a save carries inputs, not derived state — F1).
            truth: self.world.truth_digest(),
            record_len: self.chain.len(),
            record_head: self.chain.head(),
        };
        Ok(Answered { answer: Answer::Saved(facts), claims: Vec::new(), derived: None })
    }

    /// **U9's document form, read.** The declared reference is consumed **in place** by the content
    /// gate's one reader — never carried in as bytes — and the bytes that are the declared ones are
    /// parsed by U9's one parser and loaded against the ruleset this gate declares. So a document that
    /// is not there refuses `source_absent`, one whose bytes have moved `source_digest_mismatch`, bytes
    /// that are not a save `not_a_save`/`save_truncated`/`save_tampered`, and a save of another
    /// world — another seed, another ruleset, another record — `identity_mismatch`/`record_diverged`,
    /// each by its own name.
    ///
    /// The answer is what those bytes are of: the identity, the record, and the truth the load
    /// reproduces. It changes the world in nothing: a loaded world is a **value** the caller compares,
    /// not a new authority — this gate keeps serving the world it was built over (SPEC-3).
    fn answer_load(&self, request: &Request) -> Result<Answered, Refusal> {
        let path = request.text("path")?;
        let digest = request.digest("digest")?;
        let facts = super::receipt::document_facts(
            &self.intake,
            self.ruleset,
            &Source::new(path, digest),
        )
        .map_err(|refusals| {
            joined_refusals(
                refusals,
                RefusalKind::NotASave,
                "the declared document is not a save of this world",
            )
        })?;
        Ok(Answered { answer: Answer::Loaded(facts), claims: Vec::new(), derived: None })
    }
}

/// A refusal set as one refusal, for a seam whose answer is one value: the first name is carried and
/// **every** refusal's name and words are in the message, so a channel that carries one refusal never
/// silently drops the rest (`RefusalSet::joined`). The `fallback` kind is for a seam that refused
/// **without naming** anything — which no seam of this gate does, and which is therefore reported as a
/// fault of this module's own rather than passed off as a refusal of the world's.
fn joined_refusals(refusals: RefusalSet, fallback: RefusalKind, what: &str) -> Refusal {
    refusals.joined().unwrap_or_else(|| {
        Refusal::new(
            fallback,
            format!("{what}, and no refusal was named — which no seam of this gate does"),
        )
    })
}

fn typed_subject(request: &Request) -> String {
    if request.values.is_empty() {
        return format!("no arguments ({})", request.tool.name);
    }
    request
        .values
        .iter()
        .map(|(argument, value)| format!("{}={}", argument.name, value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn claim_subject(claims: &[Claim]) -> String {
    if claims.is_empty() {
        return "no claims".to_string();
    }
    claims.iter().map(Claim::name).collect::<Vec<_>>().join(" + ")
}
