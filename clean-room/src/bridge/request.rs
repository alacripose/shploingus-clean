use super::*;
// ---------------------------------------------------------------------------
// The request: one declared line, and its canonical identity.
// ---------------------------------------------------------------------------

/// A parsed request: the row it matched, its arguments **in declared order**, and the canonical line
/// that is the request's identity. Fields are private, so the only way to build one is [`parse`] —
/// which is what makes "every argument is present and of its declared kind" a property rather than a
/// hope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub(super) tool: &'static Tool,
    pub(super) values: Vec<(&'static Argument, String)>,
    pub(super) canonical: String,
}

impl Request {
    pub fn tool(&self) -> &'static Tool {
        self.tool
    }

    /// The canonical line: verb first, then every declared argument in declared order, single-spaced.
    /// Two spellings of one request are one value, so they are one request and one answer.
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(argument, _)| argument.name == name)
            .map(|(_, value)| value.as_str())
    }

    // ---- The typed reads. The shape was checked at parse; the *domain* is decided here, and a
    // ---- domain refusal carries the owner's own name.

    fn required(&self, name: &str) -> Result<&str, Refusal> {
        self.value(name).ok_or_else(|| {
            Refusal::new(
                RefusalKind::ArgumentAbsent,
                format!("`{name}` is declared by `{}` and is absent", self.tool.name),
            )
        })
    }

    /// A chunk. Three integers, or the request was malformed before it got here.
    pub fn chunk(&self, name: &str) -> Result<ChunkKey, Refusal> {
        let value = self.required(name)?;
        let (x, y, z) = super::table::parse_triple(value)
            .ok_or_else(|| malformed(name, value, ArgKind::Chunk))?;
        Ok(ChunkKey::new(x, y, z))
    }

    /// A voxel position. Three integers, like a chunk.
    pub fn voxel(&self, name: &str) -> Result<VoxelPos, Refusal> {
        let value = self.required(name)?;
        let (x, y, z) = super::table::parse_triple(value)
            .ok_or_else(|| malformed(name, value, ArgKind::Voxel))?;
        Ok(VoxelPos::new(x, y, z))
    }

    /// A render level. The **ladder** is U4's decision: a number off it is refused by the owner's own
    /// name (`level_off_ladder`) rather than clamped to the nearest rung.
    pub fn level(&self, name: &str) -> Result<Level, Refusal> {
        let value = self.required(name)?;
        let raw = value
            .parse::<u8>()
            .map_err(|_| malformed(name, value, ArgKind::Level))?;
        Level::new(raw)
    }

    /// A declared field. A name no field declares is refused (`field_undeclared`) — which is a fact
    /// about the world's vocabulary, so the refusal is recorded rather than dropped.
    pub fn field(&self, name: &str) -> Result<FieldId, Refusal> {
        let value = self.required(name)?;
        FieldId::ALL
            .iter()
            .copied()
            .find(|field| field.name() == value)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::FieldUndeclared,
                    format!(
                        "`{value}` is not one of the {} declared fields",
                        FieldId::COUNT
                    ),
                )
            })
    }

    /// An argument as written, for the kinds whose value **is** its text (`Path`, `Name`): the shape
    /// was decided at parse, and nothing here consults a world.
    pub fn text(&self, name: &str) -> Result<String, Refusal> {
        Ok(self.required(name)?.to_string())
    }

    /// A **declared list** of names, as written, or `None` when the row says nothing about that
    /// declaration — which is the declaration *this row requires nothing here*, and not a fault. The
    /// list's shape (one or more names, `+`-joined) was decided at parse, so a value that got here is a
    /// list of names and never an empty one: what the wire **cannot** spell is a requirement declared
    /// and left unnamed, for the same reason it cannot spell a zero-column patch — nothing is not a
    /// value of a declared kind. That state lives in [`Row`](crate::intake::Row)'s own vocabulary and is
    /// reachable through the intake's seam, which is a caller building a row rather than a served line.
    pub fn names(&self, name: &str) -> Option<Vec<String>> {
        self.value(name)
            .map(|value| value.split('+').map(str::to_string).collect())
    }

    /// The **material ids** a row requires. An id is a `u16` — the registry's own vocabulary — so a
    /// value that is not a number is refused by the request's own name (`argument_malformed`) here, the
    /// way a level that is not a number is, rather than being carried onward to a seam that speaks ids.
    /// Whether the world *admits* an id is not decided here: that is the registry's own refusal.
    pub fn material_ids(&self, name: &str) -> Result<Option<Vec<MaterialId>>, Refusal> {
        let Some(value) = self.value(name) else {
            return Ok(None);
        };
        let mut ids = Vec::new();
        for part in value.split('+') {
            match part.parse::<u16>() {
                Ok(id) => ids.push(MaterialId(id)),
                Err(_) => return Err(malformed(name, value, ArgKind::Names)),
            }
        }
        Ok(Some(ids))
    }

    /// A content digest. The shape was checked at parse; a value that is somehow not one anyway is
    /// refused by the request's own name (`argument_malformed`) here rather than carried onward.
    pub fn digest(&self, name: &str) -> Result<Digest, Refusal> {
        let value = self.required(name)?;
        Digest::from_hex(value).ok_or_else(|| malformed(name, value, ArgKind::Digest))
    }

    /// The **licence verdict** a row relies on. The verdicts are closed, and a word outside them is
    /// refused `licence_undeclared` with the declared two in the message — the same way an undeclared
    /// carrier or field is refused, because the gate carries a verdict rather than inventing one.
    pub fn licence(&self, name: &str) -> Result<Licence, Refusal> {
        let value = self.required(name)?;
        Licence::parse(value).ok_or_else(|| {
            Refusal::new(
                RefusalKind::LicenceUndeclared,
                format!(
                    "`{value}` is not a licence verdict this gate declares; it declares {}",
                    Licence::ALL
                        .iter()
                        .map(|licence| licence.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        })
    }

    /// The **claim** a row is admitted under. The claims a *request line* can carry are closed: a pin
    /// names a signer and a digest that an operator states **out of band**, so there is nothing a line
    /// could carry and `pinned` is refused `claim_undeclared` rather than read as nearly-right.
    pub fn claim(&self, name: &str) -> Result<crate::intake::Claim, Refusal> {
        let value = self.required(name)?;
        crate::intake::Claim::parse(value).ok_or_else(|| {
            Refusal::new(
                RefusalKind::ClaimUndeclared,
                format!(
                    "`{value}` is not a claim a request line can carry; this table declares {} (a pin \
                     is an operator's statement out of band)",
                    crate::intake::Claim::ChecksumOnly.name()
                ),
            )
        })
    }

    /// A patch side. Positive: a patch of zero columns has no site to evaluate.
    pub fn side(&self, name: &str) -> Result<u32, Refusal> {
        let value = self.required(name)?;
        let side = value
            .parse::<u32>()
            .map_err(|_| malformed(name, value, ArgKind::Side))?;
        if side == 0 {
            return Err(malformed(name, value, ArgKind::Side));
        }
        Ok(side)
    }
}

impl Request {
    /// **The row this request declares.** Every argument an `admit` line carries is a declaration the
    /// row travels with — its document and its own acceptance check **by reference** (`path + digest`,
    /// never bytes), its feature, licence verdict, claim and carrier as declared words, and what it
    /// requires of the world — so *what a line means* is one value rather than a list of arguments each
    /// caller reads again.
    ///
    /// **One place decides it**, which is what lets verification re-establish a receipt from its
    /// request ([`Receipt::verify`]): the row a call was dispatched with and the row a receipt is
    /// re-checked against are built by the same code, so they cannot drift into two rows.
    ///
    /// The names are resolved here, by the vocabularies that declare them: a licence word outside the
    /// declared verdicts is refused `licence_undeclared`, a claim a request line cannot carry
    /// `claim_undeclared`, a carrier the gate does not declare is left to the intake's own axis, and an
    /// id that is not a `u16` is refused `argument_malformed` — so a row that reaches the intake has
    /// already been held to the gate's own vocabulary.
    pub(super) fn row(&self) -> Result<Row, Refusal> {
        let carrier = self.text("carrier")?;
        // **What the row requires of the world**, read as declarations rather than as words: an absent
        // key is *this row requires nothing here*, and a key with a value is a requirement the world's
        // own seams then answer — U3's roster for a stage name, U2's registry for a material id.
        let requirements = Requirements {
            stages: self.names("requires_stages").map(Requirement::Named),
            materials: self.material_ids("requires_materials")?.map(Requirement::Named),
        };
        Ok(Row::new(
            self.text("feature")?,
            Source::new(self.text("path")?, self.digest("digest")?),
            Some(self.licence("licence")?),
            Some(&carrier),
            Some(Acceptance::Provided(Source::new(
                self.text("check")?,
                self.digest("check_digest")?,
            ))),
            self.claim("claim")?,
            requirements,
        ))
    }
}

/// A value that is not of its declared kind, in the ordinary words: the shape rule's own refusal.
fn malformed(name: &str, value: &str, kind: ArgKind) -> Refusal {
    malformed_because(name, value, kind, "is not one")
}

/// The same refusal, with the shape rule's own reason for it — a value that is *shaped* like its kind
/// and is still refused for what it names.
fn malformed_because(name: &str, value: &str, kind: ArgKind, reason: &str) -> Refusal {
    Refusal::new(
        RefusalKind::ArgumentMalformed,
        format!("`{name}` is declared as a {}: `{value}` {reason}", kind.name()),
    )
}

/// Parse one declared request line. **Every** fault is refused by name here — an undeclared verb, an
/// undeclared argument, an absent required argument, a value that is not of its declared kind, a
/// duplicate key — and nothing is written, because a line that does not parse is not an event about
/// the world.
pub fn parse(line: &str) -> Result<Request, Refusal> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let Some((verb, rest)) = tokens.split_first() else {
        return Err(Refusal::new(
            RefusalKind::ToolUndeclared,
            "an empty request line names no verb",
        ));
    };
    let tool = tool(verb).ok_or_else(|| {
        Refusal::new(
            RefusalKind::ToolUndeclared,
            format!(
                "{verb:?} is not one of the {TOOL_COUNT} declared verbs: {}",
                TOOLS.iter().map(|tool| tool.name).collect::<Vec<_>>().join(", ")
            ),
        )
    })?;

    let mut given: Vec<(&'static Argument, &str)> = Vec::with_capacity(rest.len());
    for token in rest {
        let Some((key, value)) = token.split_once('=') else {
            return Err(Refusal::new(
                RefusalKind::ArgumentMalformed,
                format!("`{token}` is not a `key=value` argument of `{}`", tool.name),
            ));
        };
        let argument = tool
            .args
            .iter()
            .find(|argument| argument.name == key)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::ArgumentUndeclared,
                    format!(
                        "`{}` declares no `{key}` argument (it declares: {})",
                        tool.name,
                        declare_args(tool)
                    ),
                )
            })?;
        if given.iter().any(|(seen, _)| seen.name == argument.name) {
            return Err(Refusal::new(
                RefusalKind::ArgumentMalformed,
                format!(
                    "`{}` names `{key}` twice: the table declares one value per key",
                    tool.name
                ),
            ));
        }
        if let Some(reason) = argument.kind.accepts(value) {
            return Err(malformed_because(argument.name, value, argument.kind, reason));
        }
        given.push((argument, value));
    }

    for argument in tool.args {
        if argument.required && !given.iter().any(|(seen, _)| seen.name == argument.name) {
            return Err(Refusal::new(
                RefusalKind::ArgumentAbsent,
                format!("`{}` requires `{}` and the request is absent", tool.name, argument.name),
            ));
        }
    }

    let values: Vec<(&'static Argument, String)> = tool
        .args
        .iter()
        .filter_map(|argument| {
            given
                .iter()
                .find(|(seen, _)| seen.name == argument.name)
                .map(|(_, value)| (argument, (*value).to_string()))
        })
        .collect();
    let mut canonical = tool.name.to_string();
    for (argument, value) in &values {
        canonical.push(' ');
        canonical.push_str(argument.name);
        canonical.push('=');
        canonical.push_str(value);
    }
    Ok(Request { tool, values, canonical })
}

fn declare_args(tool: &Tool) -> String {
    if tool.args.is_empty() {
        return "no arguments".to_string();
    }
    tool.args
        .iter()
        .map(|argument| format!("{}:{}", argument.name, argument.kind.name()))
        .collect::<Vec<_>>()
        .join(", ")
}

