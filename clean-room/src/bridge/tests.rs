use super::*;
    use crate::process;
    use crate::world::{DeclaredPeriod, Tick};

    /// A gate over the **reference world** — through the one place that declaration is written down,
    /// rather than a second palette and registry assembled here: a gate this module builds differently
    /// from the one the binary serves would answer about a different world, and a test could not tell.
    fn a_gate<'a>(roster: &'a Declared) -> Gate<'a> {
        crate::reference::gate(roster, 1).expect("the reference declaration is admitted")
    }

    /// **The only optional arguments in the table are `admit`'s two conformance declarations.**
    /// Everything else a row declares must be there: an absent argument is a malformed line. This one
    /// pair is different in kind — what a row requires of the world is a *declaration about itself*, so
    /// saying nothing is a value (**this row requires nothing**), and the axis reports it rather than
    /// counting it. A third optional argument is a decision someone makes here as well as in the table.
    #[test]
    fn the_only_optional_arguments_are_what_a_row_requires_of_the_world() {
        let optional: Vec<&str> = TOOLS
            .iter()
            .flat_map(|tool| tool.args.iter())
            .filter(|argument| !argument.required)
            .map(|argument| argument.name)
            .collect();
        assert_eq!(optional, vec!["requires_stages", "requires_materials"]);
    }

    #[test]
    fn the_table_is_closed_and_bijective() {
        assert_eq!(TOOLS.len(), TOOL_COUNT, "the count and the table must agree");

        // One name per verb, and one verb per name.
        for (index, row) in TOOLS.iter().enumerate() {
            assert!(
                TOOLS[..index].iter().all(|other| other.name != row.name),
                "two rows are named {}",
                row.name
            );
            assert!(
                TOOLS[..index].iter().all(|other| other.verb != row.verb),
                "two rows serve the same verb"
            );
            assert_eq!(tool(row.name), Some(row), "the name must find the row");
        }

        // One value per key, and every declared kind checks something.
        for tool in &TOOLS {
            for (index, argument) in tool.args.iter().enumerate() {
                assert!(
                    tool.args[..index].iter().all(|other| other.name != argument.name),
                    "{} declares `{}` twice",
                    tool.name,
                    argument.name
                );
            }
            assert!(!tool.answers.is_empty());
            assert!(!tool.checks.is_empty(), "{} runs no checks", tool.name);
            for (index, check) in tool.checks.iter().enumerate() {
                assert!(
                    CheckName::ALL.contains(check),
                    "{} declares a check nobody runs",
                    tool.name
                );
                assert!(
                    tool.checks[..index].iter().all(|other| other != check),
                    "{} declares a check twice",
                    tool.name
                );
            }
            // The claims a row's receipts may carry are as declared as the checks it runs: the two
            // declarations agree, in both directions, and a claim name is declared once.
            assert_eq!(
                !tool.claims.is_empty(),
                tool.checks.contains(&CheckName::ClaimsHold),
                "{} declares claims without declaring `claims_hold`, or the other way round",
                tool.name
            );
            for (index, claim) in tool.claims.iter().enumerate() {
                assert!(
                    tool.claims[..index].iter().all(|other| other != claim),
                    "{} declares the claim `{claim}` twice",
                    tool.name
                );
            }
        }
        assert_eq!(CheckName::ALL.len(), CheckName::COUNT);

        // An undeclared verb is refused by name, and the table's own name finds nothing else.
        let refused = parse("abracadabra").expect_err("an undeclared verb");
        assert_eq!(refused.kind(), RefusalKind::ToolUndeclared);
        assert!(tool("abracadabra").is_none());
    }

    #[test]
    fn two_spellings_of_one_request_are_one_canonical_line() {
        let tight = parse("fields field=elevation at=0,0,0 side=8").expect("declared");
        let loose = parse("  fields   field=elevation\tat=0,0,0  side=8  ").expect("declared");
        assert_eq!(tight.canonical(), "fields field=elevation at=0,0,0 side=8");
        assert_eq!(tight, loose, "two spellings are one request");

        // The arguments are carried in **declared** order whatever order they were written in.
        let shuffled = parse("fields side=8 at=0,0,0 field=elevation").expect("declared");
        assert_eq!(shuffled.canonical(), tight.canonical());
        assert_eq!(shuffled, tight);

        // A verb with no arguments is its own name, with no trailing space.
        assert_eq!(parse("truth").expect("declared").canonical(), "truth");
    }

    #[test]
    fn every_argument_fault_is_refused_by_name() {
        // An argument the row does not declare.
        let refused = parse("truth field=elevation").expect_err("truth declares no arguments");
        assert_eq!(refused.kind(), RefusalKind::ArgumentUndeclared);

        // A required argument that is absent.
        let refused = parse("fields at=0,0,0 side=8").expect_err("field is required");
        assert_eq!(refused.kind(), RefusalKind::ArgumentAbsent);

        // Not of the declared kind: a chunk that is not three integers, a level that is not a
        // number, a side that is not positive.
        for line in [
            "geometry chunk=0,0 level=0",
            "geometry chunk=0,0,0,0 level=0",
            "geometry chunk=a,b,c level=0",
            "geometry chunk=0,0,0 level=lots",
            "geometry chunk=0,0,0 level=300",
            "fields field=elevation at=0,0 side=8",
            "fields field=elevation at=0,0,0 side=0",
            "fields field=elevation at=0,0,0 side=-1",
        ] {
            let refused = parse(line).expect_err(line);
            assert_eq!(refused.kind(), RefusalKind::ArgumentMalformed, "{line}");
        }

        // A key written twice.
        let refused = parse("geometry chunk=0,0,0 level=0 level=1").expect_err("one value per key");
        assert_eq!(refused.kind(), RefusalKind::ArgumentMalformed);

        // A reference that leaves the root, or that carries a separator the receipt's own fields use,
        // is refused by **shape** — owed before any world is read, and before a response that could not
        // be written.
        let hex = "0".repeat(64);
        for path in ["../outside.md", "/absolute.md", "so|urce.md", "a~b.md", "a;b.md"] {
            let line = format!(
                "admit path={path} digest={hex} feature=a-declared-row licence=permitted \
                 claim=checksum_only carrier=U4 check=check.md check_digest={hex}"
            );
            let refused = parse(&line).expect_err(path);
            assert_eq!(refused.kind(), RefusalKind::ArgumentMalformed, "{path}");
        }

        // A token with no `=` at all, and an empty line.
        let refused = parse("geometry chunk").expect_err("not a key=value argument");
        assert_eq!(refused.kind(), RefusalKind::ArgumentMalformed);
        let refused = parse("   ").expect_err("an empty request names no verb");
        assert_eq!(refused.kind(), RefusalKind::ToolUndeclared);
    }

    #[test]
    fn a_domain_fault_is_decided_by_the_owner_and_named_in_its_vocabulary() {
        // A level that is a number but is past U4's ladder: U4's own name, not a parse fault.
        let request = parse("geometry chunk=0,0,0 level=9").expect("a level-shaped argument");
        let refused = request.level("level").expect_err("level 9 is off the ladder");
        assert_eq!(refused.kind(), RefusalKind::LevelOffLadder);
        assert_eq!(refused.kind().name(), "level_off_ladder");

        // A field name no declared field declares.
        let request = parse("fields field=nope at=0,0,0 side=8").expect("a name-shaped argument");
        let refused = request.field("field").expect_err("no field is called nope");
        assert_eq!(refused.kind(), RefusalKind::FieldUndeclared);

        // The typed reads agree with the declared kinds when the value is in domain.
        let request = parse("geometry chunk=-1,2,3 level=1").expect("declared");
        assert_eq!(request.chunk("chunk").expect("a chunk"), ChunkKey::new(-1, 2, 3));
        assert_eq!(request.level("level").expect("a level").get(), 1);
    }

    /// **The wire form reads back every answer it writes** — including the two shapes only a served
    /// surface meets: an **empty** record, whose absent head is spelled `-` (the wire's own word for an
    /// absence, not the report's `none`), and an admission, whose axes travel by name.
    #[test]
    fn every_answer_the_wire_writes_is_an_answer_the_wire_reads() {
        let roster = process::roster().expect("the reference roster admits");
        let mut gate = a_gate(&roster);

        // An empty record: no event has been appended, so the head is absent — and a writer that said
        // `none` would write a form its own reader refuses.
        let receipt = gate.call("record").expect("the empty record answers");
        let wire = receipt.wire();
        assert!(wire.contains("head=-"), "{wire}");
        let rebuilt = Receipt::from_wire(&wire).expect("its own wire form reads back");
        assert_eq!(rebuilt, receipt, "the served record is the record");

        // An admission: a real document, read where it lives, with every axis the gate declares.
        let root = std::env::temp_dir().join("shploingus-u6-bridge-wire");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the fixture root is made");
        let source = b"# a declared source document\n";
        let check = b"#[test] fn the_bar() {}\n";
        std::fs::write(root.join("source.md"), source).expect("the source reaches the disk");
        std::fs::write(root.join("check.md"), check).expect("the check reaches the disk");
        gate.set_intake_root(&root);
        let receipt = gate
            .call(&format!(
                "admit path=source.md digest={} feature=a-declared-row licence=permitted \
                 claim=checksum_only carrier=U4 check=check.md check_digest={}",
                Digest::of(source).to_hex(),
                Digest::of(check).to_hex()
            ))
            .expect("the row is admitted");
        let wire = receipt.wire();
        let rebuilt = Receipt::from_wire(&wire).expect("its own wire form reads back");
        assert_eq!(rebuilt, receipt, "the served admission is the admission");
        assert_eq!(rebuilt.answer.digest(), receipt.answer.digest());

        // A **pin** is not a claim a request line can carry — but the form is total: what its writer
        // writes, its reader reads. A pin's digest **is** the digest the bytes were found at, because
        // if they had differed there would be no verdict to serve.
        let found = Digest::of(source);
        let pinned = Answer::Admitted {
            feature: "a-declared-row".to_string(),
            claim: crate::intake::Claim::pinned("an operator", found),
            found,
            axes: [Establishment::Active; Axis::COUNT],
        };
        assert_eq!(
            Answer::from_wire(&pinned.wire()).expect("a pinned verdict reads back"),
            pinned
        );

        // A claim whose shape contradicts itself is refused rather than half-read: a checksum-only
        // admission that names a signer, and an axis list that moved an axis or lost one.
        let contradictory = pinned.wire().replacen("claim=pinned", "claim=checksum_only", 1);
        assert!(Answer::from_wire(&contradictory).is_err(), "{contradictory}");
        let reordered = pinned.wire().replace("feature:active|source:active", "source:active|feature:active");
        assert!(Answer::from_wire(&reordered).is_err(), "{reordered}");
        let short = pinned.wire().replacen("|identity:active", "", 1);
        assert!(Answer::from_wire(&short).is_err(), "{short}");
    }

    #[test]
    fn a_receipt_verified_against_a_moved_world_is_refused_by_name_with_both_values() {
        let roster = process::roster().expect("the reference roster admits");
        let mut gate = a_gate(&roster);
        let receipt = gate.call("truth").expect("truth answers on any world");
        receipt
            .verify(&gate.owners())
            .expect("the answer corresponds to the world it was made in");

        // The same receipt, against a world that has moved: the claim does not hold, and both
        // values are named.
        let moved = gate.world().with_tick(Tick(1));
        let owners = Owners::new(
            &moved,
            &roster,
            gate.resident(),
            gate.chain(),
            1,
            gate.ruleset(),
            gate.intake(),
        );
        let failures = receipt.verify(&owners).expect_err("a moved world");
        assert_eq!(failures.len(), 1, "{failures}");
        assert!(failures.contains(RefusalKind::ClaimUnverified));
        let detail = failures.first_named().expect("one refusal").detail().to_string();
        assert!(detail.contains("truth_is"), "{detail}");
        // Both values, recorded and found: the identity the answer was made in, and the one it was
        // checked against.
        assert!(detail.contains("tick=0"), "{detail}");
        assert!(detail.contains("tick=1"), "{detail}");

        // A world moved by *identity* rather than by tick is refused the same way, and the truth
        // digest is the value that differs.
        let other = World::new(WorldIdentity::new(
            0x5EED_BEEF + 1,
            DeclaredPeriod::seconds(1).expect("a one-second period is not zero"),
        ));
        let owners = Owners::new(
            &other,
            &roster,
            gate.resident(),
            gate.chain(),
            1,
            gate.ruleset(),
            gate.intake(),
        );
        let failures = receipt.verify(&owners).expect_err("another identity");
        let detail = failures.first_named().expect("one refusal").detail().to_string();
        assert!(detail.contains("recorded seed=1592639215"), "{detail}");
        assert!(detail.contains("found seed=1592639216"), "{detail}");
    }

    #[test]
    fn a_refusal_that_cannot_ride_the_record_names_both_facts() {
        let roster = process::roster().expect("the reference roster admits");
        let mut gate = a_gate(&roster);
        // What a recorded refusal carries is the **decision** — the canonical request and the refusal's
        // name — so a detail can no longer make the payload unwritable, and neither can a host's wording
        // (see `a_refusal_is_recorded_by_the_decision_and_not_by_this_hosts_wording`). The path is
        // reached by a request that cannot ride the wire, and the record's refusal still names both
        // facts: the record failed, and which refusal it was.
        let tabbed = Refusal::new(RefusalKind::GeometryOutOfResidency, "a\ttabbed detail");
        let returned = gate
            .record_refusal("geometry chunk=9,9,9 level=0\ta tab", &tabbed)
            .expect_err("the record cannot frame a tab");
        // Neither fact is dropped: the record's failure is the kind, and the original refusal is
        // still named in the detail.
        assert_eq!(returned.kind(), RefusalKind::PayloadNotWirable);
        assert!(returned.detail().contains("could not record"), "{returned}");
        assert!(returned.detail().contains("geometry_out_of_residency"), "{returned}");
        assert!(gate.chain().is_empty(), "a refusal the record refused changes nothing");
    }

    #[test]
    fn a_refusal_is_recorded_by_the_decision_and_not_by_this_hosts_wording() {
        let roster = process::roster().expect("the reference roster admits");
        let request = "admit path=gone.md digest=05285b942c550cd7a426d39f538f75dbc418d06f27c9b6b3efe085cfdfb46441 \
                       feature=a-declared-row licence=permitted claim=checksum_only carrier=U4 \
                       check=check.md check_digest=bbf6713240b9adc99c12c2635b07b959368dd778d8678a5d39876e4a1d23fae4";
        // One failure, two hosts: the words an operating system puts beside it differ (`The system
        // cannot find the file specified. (os error 2)` here, `No such file or directory (os error 2)`
        // elsewhere) and so does the path each host normalizes. The record must be one row either way,
        // because the payload is the request the gate judged and the name it judged it by — and the
        // record hashes the payload.
        let here = Refusal::new(
            RefusalKind::SourceAbsent,
            "C:\\tmp\\gone.md could not be read in place: The system cannot find the file specified. \
             (os error 2)",
        );
        let there = Refusal::new(
            RefusalKind::SourceAbsent,
            "/tmp/gone.md could not be read in place: No such file or directory (os error 2)",
        );
        let mut here_gate = a_gate(&roster);
        let mut there_gate = a_gate(&roster);
        here_gate
            .record_refusal(request, &here)
            .expect("the payload is wirable");
        there_gate
            .record_refusal(request, &there)
            .expect("the payload is wirable");

        assert_eq!(
            here_gate.chain().head(),
            there_gate.chain().head(),
            "one refusal, one recorded row, on any host"
        );
        let payload = here_gate.chain().events()[0].payload();
        assert!(payload.contains("kind=source_absent"), "{payload}");
        assert!(payload.contains("path=gone.md"), "the request carries what it is about");
        assert!(
            !payload.contains("os error") && !payload.contains("could not be read"),
            "the record carries the decision, not the explanation: {payload}"
        );
    }

    #[test]
    fn a_reference_that_names_a_device_is_refused_by_shape_on_every_host() {
        let digest = "05285b942c550cd7a426d39f538f75dbc418d06f27c9b6b3efe085cfdfb46441";
        let line = |name: &str| {
            format!(
                "admit path={name} digest={digest} feature=a-declared-row licence=permitted \
                 claim=checksum_only carrier=U4 check=check.md check_digest={digest}"
            )
        };
        // A device is not a document, on any host. Measured before this rule was declared:
        // `admit path=NUL` was **admitted** here (reading `NUL` yields no bytes, so the empty digest was
        // what the row declared) while `path=CON` was refused `source_absent` — the same class of
        // reference getting two verdicts, decided by the operating system.
        for name in ["NUL", "nul", "NUL.txt", "con", "aux", "prn", "COM1", "com9", "LPT1", "lpt9.txt"] {
            let refused = parse(&line(name)).expect_err("a device is not a document");
            assert_eq!(refused.kind(), RefusalKind::ArgumentMalformed, "{name}");
            assert!(
                refused.detail().contains("reserved device"),
                "{name} must say why: {refused}"
            );
        }
        // …and a document whose name merely *starts* like one is a document: the rule names devices,
        // not spellings.
        for name in ["null.txt", "com10", "lpt", "aux_data", "console.md", "nulx"] {
            assert!(parse(&line(name)).is_ok(), "`{name}` is a path");
        }
    }

    #[test]
    fn a_receipt_that_does_not_bind_to_its_row_is_refused_by_name() {
        let roster = process::roster().expect("the reference roster admits");
        let mut gate = a_gate(&roster);
        let receipt = gate
            .call("geometry chunk=0,0,0 level=0")
            .expect("a derivation");
        receipt.verify(&gate.owners()).expect("a receipt that binds");

        let strip = |wire: &str, part: usize, replacement: &str| {
            let mut parts = wire.split(WIRE_PART).collect::<Vec<_>>();
            parts[part] = replacement;
            parts.join(&WIRE_PART.to_string())
        };
        // Strip the claims field — the evidence — and the receipt must not read back at all: without
        // the binding it was read back and `verify` looped over nothing and returned `Ok`.
        let claims = strip(&receipt.wire(), 3, "");
        let refused = Receipt::from_wire(&claims).expect_err("no evidence, no receipt");
        assert_eq!(refused.kind(), RefusalKind::ReceiptMismatch, "{refused}");
        assert_eq!(refused.kind().name(), "receipt_mismatch");

        // The checks field is the row's declaration too: an emptied one is refused, and so is one
        // naming a check this row does not declare.
        let checks = strip(&receipt.wire(), 4, "");
        assert_eq!(
            Receipt::from_wire(&checks).expect_err("the row declares five checks").kind(),
            RefusalKind::ReceiptMismatch
        );
        // A check the row *does* declare, but not the whole declared list in the declared order.
        let one_check = strip(&receipt.wire(), 4, "recorded|passed|nothing appended");
        let refused = Receipt::from_wire(&one_check).expect_err("five declared checks, not one");
        assert_eq!(refused.kind(), RefusalKind::ReceiptMismatch, "{refused}");
        assert!(refused.detail().contains("claims_hold"), "{refused}");

        // The claims must be the ones the row allows — including a row that allows *none*. Another
        // row's claim has to be refused even though it **holds** in this same world: a `geometry`
        // receipt claiming `held_is` would be re-checked against an owner that has nothing to do with
        // the geometry, which is exactly the vacuous verification this binding exists to stop.
        let held = gate.call("held").expect("a read");
        let foreign = held.wire().split(WIRE_PART).nth(3).expect("its claims field").to_string();
        let geometry = gate.call("geometry chunk=0,0,0 level=0").expect("a derivation");
        let crossbound = strip(&geometry.wire(), 3, &foreign);
        let refused = Receipt::from_wire(&crossbound).expect_err("`geometry` never claims `held_is`");
        assert_eq!(refused.kind(), RefusalKind::ReceiptMismatch, "{refused}");
        assert!(refused.detail().contains("held_is"), "{refused}");

        // …and a row that declares **no** claim refuses any claim it is handed, even a true one.
        let truth = gate.call("truth").expect("a read");
        let true_claim = truth.wire().split(WIRE_PART).nth(3).expect("its claims").to_string();
        // The `admit` row reads documents in place, so the fixture writes the two it points at and the
        // gate's root is set where they live.
        let root = std::env::temp_dir().join("shploingus-u6-bridge-binding");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the fixture root is made");
        let source = b"# a declared source document\n";
        let check = b"#[test] fn the_bar() {}\n";
        std::fs::write(root.join("source.md"), source).expect("the fixture reaches the disk");
        std::fs::write(root.join("check.md"), check).expect("the fixture reaches the disk");
        gate.set_intake_root(&root);
        let admission_line = format!(
            "admit path=source.md digest={} feature=a-declared-row licence=permitted \
             claim=checksum_only carrier=U4 check=check.md check_digest={}",
            Digest::of(source).to_hex(),
            Digest::of(check).to_hex()
        );
        let admission = gate
            .call(&admission_line)
            .expect("the fixture documents are read in place");
        assert!(admission.claims.is_empty(), "an admission makes no claim");
        let carrying = strip(&admission.wire(), 3, &true_claim);
        let refused = Receipt::from_wire(&carrying).expect_err("`admit` declares no claim");
        assert_eq!(refused.kind(), RefusalKind::ReceiptMismatch, "{refused}");
        assert!(refused.detail().contains("truth_is"), "{refused}");

        // And a receipt built in **this process** without binding refuses in `verify`, not only on the
        // wire: nothing left to verify is not a pass.
        let mut unbound = gate.call("held").expect("a read");
        unbound.claims.clear();
        let failures = unbound
            .verify(&gate.owners())
            .expect_err("a receipt with its evidence removed");
        assert!(failures.contains(RefusalKind::ReceiptMismatch));
        assert!(failures.first_named().expect("one refusal").detail().contains("no claim"));

        // The wire form of a receipt that binds still round-trips, of course.
        assert_eq!(
            Receipt::from_wire(&receipt.wire()).expect("its own wire form"),
            receipt
        );
    }

    /// **A save row's receipt is re-established, not believed.** The two rows make no claim — their
    /// answer is about a **document**, and a claim's owners are the world's own seams — so their
    /// verification reads the declared reference again, in place, through the content gate, parses it
    /// and loads it, and holds every carried fact to what that establishes now. Each edit below must
    /// refuse **by name**: a fact the document and the world do not support (`receipt_mismatch`, with
    /// both values), a document that is gone (`source_absent`), one whose bytes have moved
    /// (`source_digest_mismatch`), and a verifier that declares another ruleset (`identity_mismatch` —
    /// a save of another world is not this world's).
    #[test]
    fn a_save_rows_receipt_is_re_established_from_its_request_and_not_believed() {
        let roster = process::roster().expect("the reference roster admits");
        let mut gate = a_gate(&roster);
        let root = std::env::temp_dir().join("shploingus-bridge-save-document");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the fixture root is made");
        gate.set_intake_root(&root);

        let written = gate.call("save path=receipt.save").expect("a world writes down");
        written
            .verify(&gate.owners())
            .expect("the honest write is re-established");
        let facts = match &written.answer {
            Answer::Saved(facts) => facts.clone(),
            other => panic!("a write answers with its document, not {other:?}"),
        };
        let load_line = format!("load path=receipt.save digest={}", facts.digest.to_hex());
        let loaded = gate.call(&load_line).expect("the document reads back");
        loaded
            .verify(&gate.owners())
            .expect("the honest read is re-established");

        // The wire form is how a client hands a receipt back, and the wire form is where an edit can
        // be made without rebuilding an answer by hand.
        let wire = written.wire();
        let refusal_of = |wire: &str| -> String {
            match Receipt::from_wire(wire) {
                Err(refusal) => refusal.kind().name().to_string(),
                Ok(receipt) => match receipt.verify(&gate.owners()) {
                    Ok(()) => "verified".to_string(),
                    Err(refusals) => refusals
                        .first_named()
                        .map(|refusal| refusal.kind().name().to_string())
                        .unwrap_or_else(|| "unnamed".to_string()),
                },
            }
        };
        // A carried fact the document and the world do not support.
        assert_eq!(
            refusal_of(&wire.replace(
                &format!("len={},", facts.len),
                &format!("len={},", facts.len + 1)
            )),
            "receipt_mismatch",
            "a length that is not the document's own"
        );
        assert_eq!(
            refusal_of(&wire.replace(
                &format!("seed={},", facts.seed),
                &format!("seed={},", facts.seed + 1)
            )),
            "receipt_mismatch",
            "a seed that is not the saved world's own"
        );
        let (ruleset, revision) = (facts.ruleset.ruleset, facts.ruleset.revision);
        let doctored_ruleset = wire.replace(
            &format!("ruleset={ruleset},revision={revision}"),
            &format!("ruleset={},revision={revision}", ruleset + 1),
        );
        assert_eq!(
            refusal_of(&doctored_ruleset),
            "receipt_mismatch",
            "a ruleset the document was not written under"
        );

        // A document whose bytes have moved, and a document that is gone: the **gate's** own names,
        // reached because verification reads the reference rather than trusting the answer.
        std::fs::write(root.join("receipt.save"), b"# not a save at all\n")
            .expect("the fixture is written");
        assert_eq!(refusal_of(&wire), "source_digest_mismatch");
        let loaded_wire = loaded.wire();
        assert_eq!(refusal_of(&loaded_wire), "source_digest_mismatch");
        std::fs::remove_file(root.join("receipt.save")).expect("the document is removed");
        assert_eq!(refusal_of(&wire), "source_absent");
        assert_eq!(refusal_of(&loaded_wire), "source_absent");

        // A verifier that declares another ruleset: the document is re-loaded into **its** identity, so
        // a save of another world refuses `identity_mismatch` — the refusal U9 owns, passed through.
        let mut other = a_gate(&roster);
        other.set_intake_root(&root);
        let elsewhere = std::env::temp_dir().join("shploingus-bridge-save-elsewhere");
        let _ = std::fs::remove_dir_all(&elsewhere);
        std::fs::create_dir_all(&elsewhere).expect("the fixture root is made");
        let rewritten = gate.call("save path=receipt.save").expect("written again");
        let owners = Owners::new(
            other.world(),
            &roster,
            other.resident(),
            other.chain(),
            1,
            Ruleset::new(8, 3),
            other.intake(),
        );
        let failures = rewritten
            .verify(&owners)
            .expect_err("a save of another world is not this world's");
        assert!(failures.contains(RefusalKind::IdentityMismatch), "{failures}");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&elsewhere);
    }

    #[test]
    fn a_read_appends_nothing_and_a_parse_fault_is_not_an_event() {
        let roster = process::roster().expect("the reference roster admits");
        let mut gate = a_gate(&roster);

        for line in ["truth", "held", "record", "fields field=elevation at=0,0,0 side=8"] {
            let before = gate.chain().len();
            let receipt = gate.call(line).expect("the reference world answers");
            assert_eq!(receipt.effect.events_after, before, "{line} must append nothing");
            assert!(receipt.effect.appended.is_none(), "{line} appended an event");
            assert!(!receipt.effect.changed_the_record(), "{line} changed the record");
            assert!(receipt.effect.refusals == 0, "{line} refused something");
            receipt.verify(&gate.owners()).expect("the claims hold");
        }
        assert!(gate.chain().is_empty(), "the record holds only changes");

        // A request that does not parse is refused before the record is touched at all.
        let before = gate.chain().len();
        let refused = gate.call("geometry chunk=0,0 level=0").expect_err("a malformed chunk");
        assert_eq!(refused.kind(), RefusalKind::ArgumentMalformed);
        assert_eq!(gate.chain().len(), before, "a line that does not parse writes nothing");
        gate.chain().verify().expect("the record still verifies");
    }
