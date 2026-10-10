// architecture/workspace.dsl — the system operating contract for Shploingus.
//
// This file is the hard boundary no agent may violate. It declares the governance
// stack — ticket -> chain -> board -> back-compiler -> solution -> recipe -> package,
// with the decision layer over all of it — as the one place the project's structure
// is stated. It exists because agents that "flub up" a project almost always fail
// the same way: context-window collapse — lost boundaries, over-engineering a small
// feature, breaking upstream contracts, inventing dependencies.
//
// Enforced by four rules (docs/FUTURE_PLANS_FOR_ANY_CURIOUS_MODELS.md):
//   1. Zero-erosion rule   — no software system, crate, store or third-party
//                            connection is introduced unless this model declares it.
//   2. Interface contracts — a change to a communication path requires its arrow
//                            to exist here first. Change the DSL first, and say so.
//   3. Flub mitigation     — on an implementation error, check whether the design
//                            has drifted from these container blocks. Do not rewrite
//                            adjacent modules or inject global helpers.
//   4. Pre-flight block    — every task response begins with:
//                              Target Container: ...
//                              Permitted Upstream/Downstream: ...
//                              Drift Evaluation: ...
//
// Provenance (ruled 2026-10-01): the declaration is README.new.md's
// "The architecture contract (Structurizr DSL)" section. The container set follows
// the record's stack statement (C-97.2), which adds `board`. The auditor is Laya --
// the decision layer, inside the stack -- and no auditor is built beside it
// (C-82.10). `voxel-audio` is a container as of 2026-10-01 (user-ruled: "DECLARE THE
// AUDIO PACKAGE ALREADY" / "THE FUCKING AUDIO WORK HASN'T EVEN BEEN STARTED"): it
// was a workspace member in NO declaration before this line, and that silence is
// what the change closes.
//
// `civilians` is a container as of 2026-10-02 (user-ruled: "the jobs organizations
// contracts security commissions all of it lying around in the past projects waiting
// to be wired into laya" / "civilians need the entire process (just not the physical
// mobs, probably useful for the laya observer and sound engine though)"). It is the
// ala-cities process, ported whole as the agency whose evidence is the proof of work
// the judge reads; declared here first (rule 2), with the code following.
//
// The decision layer is decomposed rather than drawn as one box, because a flat box
// drew it as *beside* the stack: the seam (decision::Decider), the action matrix
// (policy::decide), the in-project decider (decision::Deterministic), the model
// instrument (oracle::Oracle) and the judge (laya). It sits **on** the compiler's
// path, so the Compile stage's artifacts are emitted through it and a denied stage
// stops the compile — `Auditor::step` is the only way between stages (compiler.rs,
// C-82.10).
//
// Parsed by this host's validator: logs/structurizr-validate.log -- "Valid workspace:
// Shploingus", 0 errors (the earlier line here claimed the opposite and was false).
//
// Render it live while working:
//   docker run -it --rm -p 8080:8080 -v $(pwd)/architecture:/usr/local/structurizr structurizr/lite

workspace "Shploingus" "ticket -> back-compiler -> recipe -> code, with the decision layer inside" {

    properties {
        "rule.zero-erosion" "no software system, crate, store, or third-party connection is introduced unless this model declares it"
        "rule.interface-contracts" "a change to a communication path requires the relationship arrow to exist here first"
        "rule.flub-mitigation" "on an implementation error, check whether the design has drifted from the container blocks; do not rewrite adjacent modules or inject global helpers"
        "rule.pre-flight" "every task response begins with Target Container, Permitted Upstream/Downstream, and Drift Evaluation"
        "forbidden.second-record" "a second record beside the chain -- the chain is the one append-only record, and a scratch ledger beside it drifts"
        "forbidden.foreign-queue-format" "a queue format outside the declared codec -- a hand-rolled line format is an A08 finding the gate convicts"
        "forbidden.detached-judge" "a decision system that does not sit on the compiler's path -- the judge is a layer of the stack, never a parallel auditor that reports on the stack from outside"
        "audit.who" "laya -- the decision layer is the auditor: it maps a change against this model's container blocks (ruled 2026-10-01)"
        "rule.one-thing" "the tick system, the tests and world generation are one thing: world = f(seed, tick, declared period); the tests' subject is the generated world, the client shows the suite (H-7.13), and a layer that verifies a parallel layer is the defect (user-ruled 2026-10-01)"
        "forbidden.box-selection" "the world selection is not a cuboid projection: it is a declared, validated region that refuses rather than clamps, whose budget and bounded estimate are reported before the work, and whose view outlines the cells the modification record actually changed (C-48, C-85, C-86 86.3, C-87, H-7.3, H-7.9; the wand is one instrument with three uses and its making is decided per use)"
        "forbidden.hardcoded-view" "a view hardcoded in the shell: views are declared data loaded from packages and drawn by one generic renderer, and the bar loads the visible UI plugins rather than naming them in Rust (H-7.5, H-7.19, H-7.23, user-ruled 2026-10-01)"
        "declared.rodio" "rodio is the audio output wheel this model admits (rule.zero-erosion): the synthesis is voxel-audio's as f(seed, tick), the device is voxel-client's, and the music is procedural from the feeling wheel with laya deciding (user-ruled 2026-10-01)"
    }

    model {

        player = person "The player" "files plain prose tickets at the console"

        // **The agent, declared as an actor rather than left implicit.** Q425 rules that the
        // agent is player-equivalent in ordinary play, and the queue was already written for a
        // reader outside the window; what was missing was that an agent could not *act* in the
        // shell at all. It runs the console's own closed verb vocabulary through the same parser
        // and the same runner a typed line reaches, so there is one vocabulary and no second way
        // to change the shell. Added 2026-10-02 with the inbox it declares (rule 2: the arrow
        // first, and say so).
        agent = person "The agent" "runs the console's verbs through the api, player-equivalent"

        // The stack, as the record states it (C-97.2). The compiler's own loop,
        // in its own words (crates/voxel-package/src/compiler.rs):
        //   user files a ticket in-game
        //     -> Chain.append(TicketTransition)      the one append-only record
        //     -> Board::from_chain(&chain)           the tickets/governance view
        //     -> Solution::for_ticket(...)           back-compiled, deterministically
        //     -> render(...) -> compile(...)         recipe source -> package bytes
        //     -> verify_provenance(...)              governance + security audit
        shploingus = softwareSystem "Shploingus" "the voxel world, its governance stack, and the window a person sees it through" {

            chain = container "The chain" "chains/workspace.chain — the one append-only record" "chain"
            board = container "The board" "voxel-model::ticket — Board::from_chain, the tickets/governance view" "Rust / voxel-model"
            decision = container "The decision layer" "laya + decision::Decider — decides every stage, full axis suite" "Rust / voxel-model" {
                seam = component "The seam" "decision::Decider — Subject -> Assessment, one vocabulary for every decision; a missing instrument is a refusal, never a pass" "Rust"
                matrix = component "The action matrix" "policy::decide — one matrix, one decision function, one audit sink; deny by default" "Rust"
                deterministic = component "The in-project decider" "decision::Deterministic — answers by delegating to the readers, never by re-deciding" "Rust"
                oracle = component "The model instrument" "oracle::Oracle — one implementation of the seam, with oracle::UnreachableOracle for the skipped case; model-oracle not-evaluated" "Rust"
                judge = component "The judge" "laya — typed questions (Choice / Score / Noul) and calibrated answers; no test succeeds unless the client carries one notification per decision" "Rust / voxel-model"
            }

            // **The civilian layer, declared 2026-10-02.** One container of seven components
            // rather than seven containers, because it is one process with one record store
            // and `rule.one-thing` refuses a second stack beside the first. **It does not
            // judge.** Its grades are computed from evidence and decided by the decision
            // layer — a grade that promoted itself would be a self-report, which is
            // `forbidden.detached-judge` in the other direction. The record it ports:
            // `seasons/civilians/` in the ticket-ledger prototype (civilian_ada, job_buffy_001,
            // claim_*, ticket_*, evidence_*), the §16 contract and kinds book in the dirty
            // branch's `legacyledger/`, and the ladders `axis.rs` already carries.
            civilians = container "The civilian layer" "the ala-cities process — citizens, jobs, organizations, contracts, commissions and security; its evidence is the proof of work for solutions, graded against axis.rs. No physical mobs; the records feed the observer and the sound engine" "Rust / voxel-model" {
                citizen = component "Citizens" "persistent entities: lifecycle, skills, needs, jobs, claims, promotions, retirement — the store's own record (civilian_buffy.json: craftsman, unit grade D, bronze tier)" "Rust"
                organization = component "Organizations" "premises + roster + scope; an office is held and answerable up the chain; the government is an organization (GRILLING-C9 rounds 15-16)" "Rust"
                contract = component "Contracts" "the §16 envelope — scope, authority, rate, term, evidence, completion, change control (legacyledger/gov.rs)" "Rust"
                job = component "Jobs" "a citizen's scoped work: rate, term, the evidence required per ticket; a job closes tickets with evidence (job_buffy_001.json)" "Rust"
                commissions = component "Commissions" "the ticket kinds and their obligations — chat, question, commission; a commission is the one kind whose reply may start work in the world (legacyledger/kinds.rs)" "Rust"
                evidence = component "Evidence and grading" "the evidence ladder graded on axis.rs's ladders — the proof of work for solutions (gov.rs §30; unit grades, certification tiers, evidence grades A-D)" "Rust"
                security = component "Security" "the findings lane, cited by line (laya::security_findings), and the high-security organization's phases as the record decides them (GOD_AGENTS §39)" "Rust"
            }
            compiler = container "Back-compiler" "voxel-package::compiler — Solution::for_ticket -> render -> compile" "Rust"
            packages = container "Packages" "packages/ — the loader's search path IS the compiler's output root; a package carries the recipe it was compiled from, and the recipe carries the rows it implements" "shpkg"
            queue = container "Notification queue" "logs/laya-decisions/ (the run's judgement) and logs/agent-inbox/ (the recipes) — musli-wire records, sha256-verified, streamed" "api layer"
            game = container "The game" "shploingus + voxel-client — the window and the shell" "Rust / wgpu" {

                // **The world, declared 2026-10-02 with the record it carries (rule 2: the
                // arrow first, and say so).** The civilians commit into the world's
                // modification record, so the world is named here before the code moves.
                //
                // Four sources agree on what a world *is*, and none of them is mine:
                //   * `voxel-shapers/src/column.rs` — "A world is `generator(x, y, z)`
                //     **composed with** the `Earthworks` record", and "**a saved world is a
                //     seed, a tick and a list of changes**";
                //   * `docs/LEDGER.md` C-96.1 — the value is `voxel_client::game::World`,
                //     "**a seed, a tick and a configuration held together**", `spec()`
                //     borrowed, `App` holds one, and "**a save is record + seed + tick**";
                //   * `voxel-shapers/src/stages.rs` — `Earthworks` is a declared **stage**,
                //     and player edits run at the **×1** rung, which is "part of the
                //     determinism contract, not a performance knob";
                //   * `docs/SHAPERS.md` §1 row 15 — earthworks is "**recorded** player/agent
                //     modification of the world, each entry carrying its mass ledger".
                //
                // It is **not** a second record (`forbidden.second-record`): it is the one
                // the generator already composes with. The governed fields — actor, task,
                // verb, reservation, transition id — **widen** it rather than sit beside it
                // (user's ruling, 2026-10-02). And the world is **the game's**: the civilians
                // are handed it across `game -> civilians` and hand the change back across
                // `civilians -> world`; nothing here is a third world.
                world = component "The world" "voxel-client::game::World — seed, tick and configuration held together (C-96.1) — composed with the earthworks record (voxel-shapers::earthworks); a saved world is a seed, a tick and a list of changes" "Rust / voxel-client + voxel-shapers"
            }
            audio = container "voxel-audio" "crates/voxel-audio — the onset and the bed, as f(seed, tick)" "Rust / voxel-audio"
        }

        // The arrows are the contract. Nothing communicates outside them.
        player -> chain "files tickets through the handler, never by hand"
        chain -> board "tickets read back"
        board -> compiler "the rows the compiler back-compiles from"
        compiler -> decision "every stage: Auditor::step consults policy::decide"
        compiler -> matrix "the action each stage needs"
        decision -> packages "the compile stage, permitted — a denied stage stops the compile"
        decision -> queue "one notification per decision"
        packages -> game "loaded and verified before use"
        queue -> game "the notifications window streams it"

        // **The agent's line in, corrected 2026-10-02.** The first version of this arrow had the
        // agent writing commands into a file the client ran. That was wrong in both halves: the
        // agent authors nothing, and the inbox is not a command channel. **The recipes are
        // derived from the solutions and written by the decision layer** into the same streaming
        // system the window already reads — one musli record per emitted package, sha256-bound to
        // the package, because the package carries the recipe. The agent's work is to read it and
        // implement it; the change then enters `crates/`, which is `compiler -> decision` again.
        queue -> agent "the recipes the game derived from the solutions, one record per package, sha256-verified"
        agent -> compiler "implements the recipe it was given; every stage judges the change"

        // **The game asks the decision layer for this run's judgement, on its own clock.**
        // Added 2026-10-02 with the writer it declares (rule 2: the arrow first, and say so).
        // Before this the game ran no judgement at all, so `logs/laya-decisions/` was written
        // only from tests — and this arrow did not exist to say what was missing. The pass is
        // laya's own (`decision -> queue` above is the write), it is the same judge the
        // compiler's stages pass through, and the game asks for it when the world opens, then
        // streams the queue at its own tick rate through `queue -> game`. It is a caller of the
        // one judge, not a second auditor beside it (`forbidden.detached-judge`).
        game -> decision "asks for this run's judgement at world-open; the judge writes the queue"

        // **The civilian process's lines, declared 2026-10-02 with the container (rule 2).**
        // The user's words: *"they work with organizations and the jobs ticking system when the
        // game compiles to provide proof of work for solutions against axis.rs"*. So a compile
        // hands the solutions over, the game ticks the jobs, and what the civilians produce
        // goes to the decision layer as the proof of work the judge reads. The grading itself
        // is a verdict, so it is not drawn as a second judge: the ladders are axis.rs's and the
        // verdict is laya's.
        compiler -> civilians "the solutions the jobs prove; a compile is when the jobs run"
        game -> civilians "the jobs tick, on the world's clock — voxel-work::tick and voxel-client::clock"
        civilians -> decision "the evidence: proof of work for the solutions, graded against axis.rs"

        // **The board's cases reach the evidence reader** — declared 2026-10-02 with the reading
        // itself (rule 2: the arrow first, and say so). The civilians observe the record's ticket
        // rows — the answers filed against them — and the ticket-evidence ladder grades each one
        // (read back from the document it cites, or self-report, or broken), with the rule in
        // `decision::rule_for_civilian`. Without this arrow the reading would travel a path this
        // model does not declare, which is the drift the contract exists to catch.
        board -> civilians "the cases whose citations the ticket-evidence ladder grades"

        // **The civilians' commit, declared 2026-10-02 with the record it writes.**
        // `game -> civilians` above hands them the world on the world's clock; this is the
        // return path: what a governed transition actually changed, into the world's record,
        // on earthworks' ×1 rung. The governed fields widen `Modification` — the user's
        // ruling of 2026-10-02 — rather than a book beside it, so this arrow is the path the
        // widening serves. Without it `Modification` would gain an actor and this model would
        // not say so, which is what rule 2 exists to prevent. The evidence rows are the other
        // half and travel the arrow above, to the judge.
        civilians -> world "the governed change: actor, task, verb, reservation and transition id, recorded in the earthworks entry"

        // The audio wheel, declared by the user and not before (A11), and not built
        // (A13). The synthesis is voxel-audio's as f(seed, tick); the device is the
        // game's, because voxel-audio deliberately depends on no rodio, no cpal and
        // nothing that opens a device. rodio is the output wheel this model admits
        // (declared.rodio) and the game is where it opens.
        game -> audio "asks for this tick's sound; the game opens the device"
        decision -> audio "the emotion state laya decides, which the music is made from"

        // The decision layer's own wiring. `oracle -> judge` is deliberately absent:
        // the gate reports `oracle-is-laya no`, so drawing that link would assert a
        // relationship the code does not have.
        deterministic -> seam "one implementation of the seam (the in-project layer)"
        oracle -> seam "one implementation of the seam (the model instrument)"
        judge -> queue "one notification per decision, never fewer"
    }

    views {

        systemContext shploingus "SystemContext" {
            include *
            autolayout lr
        }

        container shploingus "Containers" {
            include *
            autolayout lr
        }

        // Components do not appear in a container view, so the decision layer's own
        // parts need this view or they are drawn nowhere.
        component decision "DecisionLayer" "the decision layer's own parts" {
            include *
            autolayout lr
        }

        // The same reason, one layer over: the civilian process is seven parts and a
        // container view draws none of them.
        component civilians "CivilianLayer" "the civilian process's own parts" {
            include *
            autolayout lr
        }

        // The same reason, once more: `world` is a component of `game`, and a container view
        // draws none of them — so the civilians' commit would reach a box nobody can see.
        component game "GameLayer" "the game's own parts" {
            include *
            autolayout lr
        }

        // One ticket, end to end, in the order the compiler's stages run.
        dynamic shploingus "TicketPipeline" "one ticket, end to end" {
            player -> chain "files a plain prose ticket"
            chain -> board "tickets read back"
            board -> compiler "the rows the compiler back-compiles from"
            compiler -> decision "every stage: Auditor::step consults policy::decide"
            compiler -> civilians "the solutions the jobs prove"
            civilians -> world "the governed change lands in the record"
            civilians -> decision "the proof of work for them, graded against axis.rs"
            decision -> packages "the compile stage, permitted"
            decision -> queue "one notification per decision"
            packages -> game "loaded and verified before use"
            queue -> game "the notifications window streams it"
            queue -> agent "the recipe, on the inbox the same stream carries"
            agent -> compiler "the implemented recipe, judged by every stage"
        }
    }
}
