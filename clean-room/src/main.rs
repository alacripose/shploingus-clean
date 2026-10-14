//! **The clean room's entry point — three ways in, and nothing else.**
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §4 (the spec-kit structure) and Constitution P4
//! (evidence over assertion). With no argument it prints the unified-systems registry and the
//! measured evidence, exiting non-zero if any built system's bar is not met; with `serve` it is a
//! **session** — one request line in, one response record out on stdin/stdout, with the session's own
//! count on stderr where a diagnostic belongs; with `window` it is **U1's visible client** — a real
//! window, one tick of the plane's own law per frame, bounded by a declared count, printing the measured
//! frame numbers it presented.
//!
//! The measurements live in `evidence` and the session's world in `reference`, so this file holds the
//! two entry points and nothing else. It imports nothing from the legacy tree; the crate's only
//! dependency is `sha2`, for content identity.

#![forbid(unsafe_code)]

use shploingus_clean_room::body::Traversal;
use shploingus_clean_room::controller;
use shploingus_clean_room::evidence;
use shploingus_clean_room::presentation;
use shploingus_clean_room::process;
use shploingus_clean_room::reference;
use shploingus_clean_room::refusal::{Refusal, RefusalKind};
use shploingus_clean_room::shell;
use shploingus_clean_room::surface;
use shploingus_clean_room::systems;
use shploingus_clean_room::viewpoint::Wish;

fn main() {
    // **The transport's own entry.** `serve` is a session, not a report: a client speaks one
    // protocol on stdin and reads its answers on stdout, so a banner or an evidence line would be
    // noise on that stream. The session's own count goes to stderr, where a diagnostic belongs.
    if std::env::args().nth(1).as_deref() == Some("serve") {
        std::process::exit(serve_stdio_session());
    }

    // **U8's own way in, and the one a person interacts with.** `shell` draws the frame and reads one
    // command per line from stdin, redrawing after each, so §5 row 7's check — *a visibly drawn frame;
    // pointer ownership read once* — is a thing you look at rather than a thing this report claims.
    if std::env::args().nth(1).as_deref() == Some("shell") {
        std::process::exit(shell_session());
    }

    // **U1's own way in, and the one the eye checks (C-40).** `window [frames] [frame.ppm]` opens a real
    // window and presents the world the tree already builds — one tick of the plane's own law per frame,
    // bounded by the declared count — then prints the measured frame numbers it presented and exits. The
    // window is **driven by the person's own keys** (2026-10-14e): `W`/`S`, `D`/`A` and `Space`/`Control`
    // move the eye along the parent's own axes and the arrow keys turn it, so a person who closes the window
    // stops the loop early and the report says how many frames were actually presented and how far their own
    // input carried the eye.
    if std::env::args().nth(1).as_deref() == Some("window") {
        std::process::exit(window_session());
    }

    // **The client's own camera, in a terminal (2026-10-14e).** `drive` reads one line of intent per step
    // from stdin, moves the eye, streams the world around the chunk the eye now stands in and draws the
    // frame — so *a person moves the camera, the residency follows it and the picture changes* is something a
    // terminal can be driven through rather than something a report claims.
    if std::env::args().nth(1).as_deref() == Some("drive") {
        std::process::exit(drive_session());
    }

    println!("shploingus clean room — new code beside the old tree (Constitution P7)");
    println!("derive-from documents: docs/CLEAN-ROOM-PLAN-2026-10-11.md, docs/DESIGN-SPEC.md, docs/APPEARANCE.md\n");
    print!("{}", systems::render());

    let u1 = evidence::check_u1();
    // U1's own client: the frame the world draws, measured on the frame's own pixels — and the window the
    // eye looks at, which is the `window` mode's own report.
    let u1_client = evidence::check_u1_client();
    // U1's own controller: the camera a person can move (2026-10-14e) — the walk that changes the picture,
    // the anchor that follows the eye's own chunk, and the round trip that draws the recorded frame again.
    let u1_controller = controller::check_u1_controller();
    let u2_world = evidence::check_u2_world();
    let u2_shapes = evidence::check_u2_shapes();
    let u2_chain = evidence::check_u2_chain();
    let u3 = evidence::check_u3();
    let u4 = evidence::check_u4();
    let u6 = evidence::check_u6();
    let u9 = evidence::check_u9();
    // Instrumentation, light and audio: the one sensor concept and the wall built from it, the
    // weighing gate whose answer is a mass, and the band table the sky, the beam and the Moon's own
    // reflected light are read out of — with a quantity that cannot be weighed refused by name.
    let u5 = evidence::check_u5();
    // The gameplay tree: one entry per reference in `docs/MODS-MAP.md` §1–§5, one row per technology,
    // admitted through the gate the way any row is — with the four parts the record has not decided
    // printed as owed.
    let u10 = evidence::check_u10();
    // The decision layer: the seven ladders and their three prohibitions, the ballot whose floor is
    // declared before the answer exists, the classifier calibrated over the sixteen labelled records,
    // and the four fail-closed rules — each proved to bite by name.
    let u7 = evidence::check_u7();
    // The shell: a visibly drawn frame over the systems already built, and the one pointer-ownership
    // decision — read once and handed to the drawing by value.
    let u8 = evidence::check_u8();
    // The first mob: a member of an organisation whose need is met through the game's own cells, whose
    // choice is a laya question read by von, whose every act is a governed event — and whose refusals are
    // by name and recorded.
    let u12 = evidence::check_u12();
    // …and the plane it reads **steps**: the first mob's need is met through the game's own cellular
    // automata over time (§34) — the same being, at the same cell, taking a different next step on the plane
    // after eight ticks of the world's own routing than on the plane it was placed in.
    let u12_stepping = evidence::check_u12_stepping();

    if u1
        && u1_client
        && u1_controller
        && u2_world
        && u2_shapes
        && u2_chain
        && u3
        && u4
        && u5
        && u6
        && u7
        && u8
        && u9
        && u10
        && u12
        && u12_stepping
    {
        println!("Clean room OK: every built system's acceptance bar is measured and passing.");
    } else {
        eprintln!(
            "Clean room FAILED: a measured bar is not met (U1={u1}, U1 client={u1_client}, U1 controller={u1_controller}, \
             U2 world={u2_world}, U2 shapes={u2_shapes}, U2 chain={u2_chain}, U3={u3}, U4={u4}, \
             U5={u5}, U6={u6}, U7={u7}, U8={u8}, U9={u9}, U10={u10}, U12={u12}, \
             U12 stepping={u12_stepping})."
        );
        std::process::exit(1);
    }
}

/// **The `window` mode — the client's own entry point, and the one the eye checks.** `window [frames]
/// [frame.ppm]`: the declared count when none is given, and a whole number of frames of at least one when one
/// is, with a malformed or zero count refused by name rather than defaulted. **The window shows the drawn
/// reading** (§41): the client's own world's **stitched region** — one footprint over `2 × 2` chunks — under
/// one camera with depth, so the report below is the **measured** one for that reading, plus what only the
/// window can answer: how many frames were presented, how long that took and at what measured rate, how many
/// ticks that was, and how many of the window's own pixels agreed with the frame's. All of it on stdout,
/// where the frame's own numbers belong; a refusal goes to stderr and the exit status is non-zero.
fn window_session() -> i32 {
    let mut words = std::env::args().skip(2);
    let frames = match words.next() {
        None => surface::DECLARED_FRAMES,
        Some(word) => match word.parse::<u32>() {
            Ok(frames) if frames > 0 => frames,
            Ok(_) => {
                eprintln!("window: a frame count of 0 presents nothing; the count is at least one");
                return 1;
            }
            Err(_) => {
                eprintln!(
                    "window: a frame count is a whole number of frames, and {word:?} is not one"
                );
                return 1;
            }
        },
    };
    let screenshot = words.next().map(std::path::PathBuf::from);
    let report = match surface::present_world(frames, screenshot.as_deref()) {
        Ok(report) => report,
        Err(refusal) => {
            eprintln!("window: the client is refused by name: {refusal}");
            return 1;
        }
    };
    let drawing = &report.drawing;
    let frame = &drawing.last;
    println!(
        "U1 window: a real window of {} x {} client pixel(s) presented {} of the declared {} frame(s) in \
         {:.1} s — **{:.2} frame(s)/s measured** against the declared {} frame(s)/s the loop paces itself \
         to (one tick per frame); dpi-aware = {}; the window shows the **drawn reading** — the client's own \
         world's stitched region under one camera with depth — while the plan reading stays the evidence \
         run's own frame",
        frame.size(),
        frame.size(),
        report.presented,
        report.frames,
        report.seconds,
        report.rate,
        surface::FRAMES_PER_SECOND,
        report.dpi_aware
    );
    // **The declared rate is a pacing target, and when the picture cannot keep it the product says so with
    // the measured number** (2026-10-13, §42). The region's mesh is derived from the frame's own state every
    // frame, and the resident set's key *is* that state — so no mesh can serve a second frame, and the cost
    // is the measured rate above rather than a rate the picture is declared to hold.
    if report.rate < f64::from(surface::FRAMES_PER_SECOND) {
        println!(
            "U1 window: the picture cannot keep the declared {} frame(s)/s — the region is derived from the \
             frame's own state every frame and a mesh keyed by that state can never serve the next one; the \
             measured rate is the cost, printed above rather than declared away",
            surface::FRAMES_PER_SECOND
        );
    }
    if report.closed {
        println!(
            "U1 window: the loop stopped after {} frame(s): the window was closed while it was open, so \
             the world below is the state it reached rather than the declared horizon",
            report.presented
        );
    }
    let deepest = match drawing.deepest {
        Some((cell, depth)) => format!("{depth} µm at column {cell}"),
        None => "no column stands water".to_string(),
    };
    println!(
        "U1 window: the plane's own law took {} tick(s) — one tick per frame — and stands at {} of {} \
         column(s) with water, deepest {deepest}; settled at the declared horizon = {}",
        drawing.ticks,
        drawing.standing,
        drawing.world_side * drawing.world_side,
        report.settled
    );
    let (dark, bright) = drawing.range();
    println!(
        "U1 window: the world it drew: surface read digest {}, the plane's own state digest {}, the drawn \
         frame's own digest {} (channels 0x{dark:02x}..0x{bright:02x})",
        drawing.world.to_hex(),
        drawing.state.to_hex(),
        frame.digest().to_hex()
    );
    println!(
        "U1 window: the window's own client area, read back through a {} bit, top-down, `BI_RGB` DIB, is \
         the drawn frame's own bytes at {} of {} pixel(s) — every pixel, every frame",
        surface::READBACK_BITS, report.agreeing, report.pixels
    );
    println!("U4 window drawn: {}", drawing.described());
    println!(
        "U4 window drawn: the drawn reading's own bars: {}",
        drawing.bars_described()
    );
    // **The body the window opened as, and what its own steps did** (2026-10-14g) — so the report says which
    // body the run's own keys were driving, and what walking under gravity did to it, rather than leaving
    // *is the eye on a body* to be assumed from the picture.
    println!(
        "U1 window: the window opened a person who is {}; over the run the body walked {:.2} m in plan, rose \
         {:.2} m, fell {:.2} m, was raised by the step-up {:.2} m, took {} jump(s) and was refused on {} \
         frame(s) — standing on the floor at the end = {}, traversing {}",
        report.opened_as,
        report.body.plan_m,
        report.body.rose_m,
        report.body.fell_m,
        report.body.climbed_m,
        report.body.jumps,
        report.body.blocked_frames,
        report.body.grounded_end,
        report.body.traversal_end.name(),
    );
    // **What the person's own input did (2026-10-14e)** — the numbers a `window` run now has that it did not
    // before: how much of the run was driven, how far the eye went, where it ended up and what the residency
    // paid for the world stepping under it.
    println!(
        "U1 window: the person's own input: {} of {} frame(s) carried a held key; the eye travelled {:.2} m \
         and its look turned {:.1}°; the last step stood over chunk ({}, {}, {}) with its frame a picture of \
         the region ({}, {}, {}); the residency derived {} chunk(s) and released {} because the law's own state \
         moved one tick a frame — a resident set keyed by that state can never serve the next one",
        report.driven_frames,
        report.presented,
        report.travelled_m,
        report.turned_deg,
        drawing.anchor.x,
        drawing.anchor.y,
        drawing.anchor.z,
        drawing.region.x,
        drawing.region.y,
        drawing.region.z,
        report.derived,
        report.stale
    );
    println!(
        "U4 window drawn: the headless drawn frame of **this run's own last state** — the same state, the \
         same call, no window — is the window's own last frame = {} (window frame {}, headless frame {}); \
         the law's own state {}",
        report.drawing.headless == frame.digest(),
        frame.digest().to_hex(),
        report.drawing.headless.to_hex(),
        drawing.state.to_hex()
    );
    let materials: Vec<String> = drawing
        .materials
        .iter()
        .map(|(class, id)| format!("{class}->{id}"))
        .collect();
    println!(
        "U1 window: {} distinct luminance band(s) on the drawn frame's own {} shaded rectangle(s) (>= {} \
         required); materials {} with water = material {}; the Sun at {}° with tint ({:.3}, {:.3}, {:.3})",
        drawing.bands(),
        frame.quads(),
        presentation::BAR_MINIMUM_BANDS,
        materials.join(" "),
        drawing.water,
        drawing.sun_altitude_deg,
        drawing.sun_tint[0],
        drawing.sun_tint[1],
        drawing.sun_tint[2]
    );
    if let Some(path) = &report.screenshot {
        println!(
            "U1 window: the presented frame is written at {} (P6, {} x {} pixel(s)) — the screenshot the \
             bar asks for, in the bytes the window was handed",
            path.display(),
            frame.size(),
            frame.size()
        );
    }
    // **The window's own bars are the window's own exit status**, the way the evidence run's are the
    // binary's: a drawn frame that failed one of them is a session that presented a picture it cannot
    // stand behind, and the refusal says which bar. A person who closes the window early is **not** a
    // failure — the loop stopping early is the client's own honest way of being spent (the headless
    // agreement says so on its own line, and only the drawn reading's own bars gate this status).
    if !report.drawing.bars_hold() {
        eprintln!(
            "window: a drawn bar is not met — {}",
            report.drawing.bars_described()
        );
        return 1;
    }
    0
}

/// **The `drive` mode (2026-10-14e; the body, 2026-10-14g) — the client's own person, driven from a
/// terminal.**
///
/// One line is one **hold** of intent and a count of steps: the words before the last are the held intents
/// ([`drive_wish`]), and a trailing whole number is how many steps to take them for (one when none is given).
/// After each line it prints the eye it reached **and what the body did to get there** — the plan distance, how
/// far it rose and fell, how far a step-up raised it and whether it jumped — beside the chunk the person's own
/// anchor moved to, what the residency held and did over those steps and the frame's own numbers. So *a person
/// walks the world, is stopped by what is there, falls under gravity and the picture follows* is something a
/// person drives rather than something a run claims.
///
/// **`walk` and `fly` are the two traversals and walking is what a person is** (2026-10-14g): the words set the
/// body's own mode through its one door ([`controller::Driven::traversed`]), so flight is a thing a person takes
/// up in the session and not a state they start in. `where` prints the state without stepping and `quit` ends
/// the session; an unknown word is refused **by name** rather than ignored.
fn drive_session() -> i32 {
    let mut client = match controller::Driven::open() {
        Ok(client) => client,
        Err(refusal) => {
            eprintln!("drive: the client is refused by name: {refusal}");
            return 1;
        }
    };
    println!(
        "drive: the person the client opens as is {} — walking, on the world's own floor, in the plan column \
         ({}, {}) the search for a place to stand chose (the declared view's own is ({}, {})); one line is one \
         hold of intent and a count of steps (forward, back, left, right, up, down, turn-left, turn-right, \
         tilt-up, tilt-down), `jump` holds the walk's own vertical intent, `walk`/`fly` take up one of the two \
         traversals, `where` prints the state, `quit` ends the session",
        client.described(),
        client.viewpoint().feet()[0].floor() as i32,
        client.viewpoint().feet()[1].floor() as i32,
        client.lens_column().0,
        client.lens_column().1,
    );
    let stdin = std::io::stdin();
    for line in std::io::BufRead::lines(stdin.lock()) {
        let line = match line {
            Ok(line) => line.trim().to_string(),
            Err(refusal) => {
                eprintln!("drive: stdin could not be read: {refusal}");
                return 1;
            }
        };
        if line.is_empty() {
            continue;
        }
        if line == "quit" {
            break;
        }
        if line == "walk" || line == "fly" {
            let traversal = if line == "fly" { Traversal::Flying } else { Traversal::Walking };
            client.traversed(traversal);
            println!(
                "drive: the person is now {}, and stands at {} — walking is where a person starts and flight is \
                 a thing they take up, so the same keys below move a different body",
                traversal.name(),
                client.described()
            );
            continue;
        }
        if line == "where" {
            match drive_state(&client) {
                Ok(note) => println!("drive: {note}"),
                Err(refusal) => println!("drive: refused by name: {refusal}"),
            }
            continue;
        }
        let (wish, steps) = match drive_wish(&line) {
            Ok(parsed) => parsed,
            Err(refusal) => {
                println!("drive: refused by name: {refusal}");
                continue;
            }
        };
        let before = client.viewpoint().eye();
        let derived_before = client.derived();
        let evicted_before = client.evicted();
        let mut taken = 0_usize;
        let mut moved_m = 0.0_f64;
        let mut turned_deg = 0.0_f64;
        // **What the body did over those steps** (2026-10-14g) — the parent's own quantities, summed: how far
        // it walked in plan, how far it rose and fell, how far a step-up raised it, and how many jumps it took.
        let mut plan_m = 0.0_f64;
        let mut rose_m = 0.0_f64;
        let mut fell_m = 0.0_f64;
        let mut climbed_m = 0.0_f64;
        let mut jumps = 0_usize;
        let mut last = None;
        for _ in 0..steps {
            match client.step(&wish) {
                Ok(step) => {
                    taken += 1;
                    moved_m += step.movement.moved_m;
                    turned_deg += step.movement.turned_deg;
                    plan_m += step.movement.body.plan_m;
                    rose_m += step.movement.body.rose_m;
                    fell_m += step.movement.body.fell_m;
                    climbed_m += step.movement.body.climbed_m;
                    jumps += usize::from(step.movement.body.jumped);
                    last = Some(step);
                }
                Err(refusal) => {
                    println!("drive: a step is refused by name: {refusal}");
                    break;
                }
            }
        }
        let Some(last) = last else {
            continue;
        };
        let after = client.viewpoint().eye();
        println!(
            "drive: {taken} step(s) of {wish:?}: the eye ({:.2}, {:.2}, {:.2}) m -> ({:.2}, {:.2}, {:.2}) m \
             ({moved_m:.2} m moved, {turned_deg:.1}° turned), over chunk ({}, {}, {}) with its frame a picture \
             of the region ({}, {}, {}); the residency held {} of this world's {} chunk(s) — derived {}, evicted \
             {} over the {taken} step(s) — and the last frame drew {} rectangle(s), {} of {} pixel(s), digest {} \
             (mesh state {})",
            before[0],
            before[1],
            before[2],
            after[0],
            after[1],
            after[2],
            last.anchor.x,
            last.anchor.y,
            last.anchor.z,
            last.region.x,
            last.region.y,
            last.region.z,
            last.stream.resident,
            client.plane().world().chunks(),
            client.derived() - derived_before,
            client.evicted() - evicted_before,
            last.drawn.quads(),
            last.drawn.shaded(),
            last.drawn.pixels_count(),
            last.drawn.digest().to_hex(),
            last.drawn.mesh_state().to_hex(),
        );
        println!(
            "drive the body: {} — over the {taken} step(s) it walked {plan_m:.2} m in plan, rose {rose_m:.2} m, \
             fell {fell_m:.2} m, was raised by the step-up {climbed_m:.2} m and took {jumps} jump(s); grounded \
             = {}, traversing {}",
            client.viewpoint().body().described(),
            last.movement.body.grounded,
            last.movement.body.traversal.name(),
        );
    }
    0
}

/// **One line of a drive, read as intent and a count** — the shell's own translation from words to the one
/// intent vocabulary ([`Wish`]), and the second of the two translations a person has (the window's own keys
/// are the other). An unknown word, or a count of zero steps, is refused by name.
fn drive_wish(line: &str) -> Result<(Wish, usize), Refusal> {
    let mut words: Vec<&str> = line.split_whitespace().collect();
    let mut steps = 1_usize;
    if let Some(last) = words.last() {
        if let Ok(count) = last.parse::<usize>() {
            if count == 0 {
                return Err(Refusal::new(
                    RefusalKind::ArgumentMalformed,
                    "a count of 0 step(s) takes no step: a drive's own count is at least one".to_string(),
                ));
            }
            steps = count;
            words.pop();
        }
    }
    let mut wish = Wish::STILL;
    for word in words {
        match word {
            "forward" => wish.forward += 1.0,
            "back" => wish.forward -= 1.0,
            "right" => wish.strafe += 1.0,
            "left" => wish.strafe -= 1.0,
            "up" => wish.rise += 1.0,
            "down" => wish.rise -= 1.0,
            // **The walk's own vertical intent** (2026-10-14g): a held jump, the parent's own key. A walking
            // body jumps with it; a flying body ignores it and rises along `rise`, which is why `up` is a
            // word of its own.
            "jump" => wish.jump = true,
            "turn-right" => wish.turn += 1.0,
            "turn-left" => wish.turn -= 1.0,
            "tilt-up" => wish.tilt += 1.0,
            "tilt-down" => wish.tilt -= 1.0,
            other => {
                return Err(Refusal::new(
                    RefusalKind::ArgumentMalformed,
                    format!(
                        "`{other}` is not a drive word (forward, back, left, right, up, down, turn-left, \
                         turn-right, tilt-up, tilt-down, jump, walk, fly), a count of steps, `where` or `quit`"
                    ),
                ))
            }
        }
    }
    Ok((wish, steps))
}

/// **Where a driven client stands, and what it holds** — the state a `where` asks for: the eye and the two
/// keys, what the residency holds and what the frame the eye draws came to.
fn drive_state(client: &controller::Driven) -> Result<String, Refusal> {
    let frame = client.frame()?;
    Ok(format!(
        "{}; the residency holds {} of this world's {} chunk(s) / {} cell(s) of the state the client shows \
         ({}) and has derived {} and evicted {} over {} step(s); the frame the eye draws is {} rectangle(s), \
         {} of {} pixel(s), digest {}",
        client.described(),
        client.residency().len(),
        client.plane().world().chunks(),
        client.residency().cells(),
        client.residency().digest().to_hex(),
        client.derived(),
        client.evicted(),
        client.steps(),
        frame.quads(),
        frame.shaded(),
        frame.pixels_count(),
        frame.digest().to_hex(),
    ))
}

/// **The `shell` mode — U8's own entry point, and the one a person interacts with.** It draws the frame
/// (`shell::frame`) and then reads one command per line from stdin, redrawing after each, so the person
/// sees the real state behind the panes and watches it change as they act. The frame goes to stdout,
/// where the thing it draws is; the session's own diagnostics never do.
///
/// Commands: `split <pane> h|v`, `close <pane>`, `focus <window>`, `layer`, `grab on|off`, `quit`. An
/// unknown command, pane or orientation is refused **by name** rather than ignored — and printed, so the
/// refusal is visible where the refusal happened.
fn shell_session() -> i32 {
    let recipe = shell::Recipe::declared();
    let mut mount = shell::Mount::of(&recipe);
    let area = shell::declared_area();
    // A frame with no grab would hand the pointer to the world and draw no glyph, which is a correct
    // frame but a dull first impression: the session starts holding its own pointer.
    mount.set_grab(true);
    if let Err(refusal) = draw_frame(&mount, area) {
        eprintln!("shell: the frame is refused by name: {refusal}");
        return 1;
    }
    let stdin = std::io::stdin();
    for line in std::io::BufRead::lines(stdin.lock()) {
        let line = match line {
            Ok(line) => line,
            Err(refusal) => {
                eprintln!("shell: stdin could not be read: {refusal}");
                return 1;
            }
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        if line == "quit" {
            break;
        }
        match apply_command(&mut mount, &line) {
            Ok(note) => println!("shell: {note}"),
            Err(refusal) => println!("shell: refused by name: {refusal}"),
        }
        if let Err(refusal) = draw_frame(&mount, area) {
            println!("shell: the frame is refused by name: {refusal}");
        }
    }
    0
}

/// Draw the frame once, for the shell session. The pointer decision is read here and passed **by value**,
/// which is the same shape the evidence and the tests use.
fn draw_frame(mount: &shell::Mount, area: shell::Area) -> Result<(), Refusal> {
    let drawn = shell::frame(mount, area, mount.read_pointer())?;
    print!("{drawn}");
    Ok(())
}

/// Apply one shell command. Every failure is a **named** refusal, never a silent no-op.
fn apply_command(mount: &mut shell::Mount, line: &str) -> Result<String, Refusal> {
    let mut words = line.split_whitespace();
    let verb = words.next().unwrap_or("");
    match verb {
        "layer" => {
            mount.toggle_layer();
            Ok(format!("layer={}", mount.layer().name()))
        }
        "grab" => match words.next() {
            Some("on") => {
                mount.set_grab(true);
                Ok("grab=on".to_string())
            }
            Some("off") => {
                mount.set_grab(false);
                Ok("grab=off".to_string())
            }
            Some(other) => Err(Refusal::new(
                RefusalKind::ArgumentMalformed,
                format!("`grab` takes `on` or `off`, not `{other}`"),
            )),
            None => Err(Refusal::new(
                RefusalKind::ArgumentMalformed,
                "`grab` takes `on` or `off`, and none was given".to_string(),
            )),
        },
        "split" => {
            let pane = parse_pane(words.next())?;
            let orientation = match words.next() {
                Some("h") => shell::Orientation::Horizontal,
                Some("v") => shell::Orientation::Vertical,
                Some(other) => {
                    return Err(Refusal::new(
                        RefusalKind::ArgumentMalformed,
                        format!("`split` takes `h` or `v`, not `{other}`"),
                    ))
                }
                None => {
                    return Err(Refusal::new(
                        RefusalKind::ArgumentMalformed,
                        "`split` takes `h` or `v`, and none was given".to_string(),
                    ))
                }
            };
            let new = mount.split(pane, orientation)?;
            Ok(format!("split pane #{} → #{} ({orientation:?})", pane.0, new.0))
        }
        "close" => {
            let pane = parse_pane(words.next())?;
            mount.close(pane)?;
            Ok(format!("closed pane #{}", pane.0))
        }
        "focus" => {
            let name = match words.next() {
                Some(name) => name,
                None => {
                    return Err(Refusal::new(
                        RefusalKind::ArgumentMalformed,
                        "`focus` takes a window's own name, and none was given".to_string(),
                    ))
                }
            };
            let window = mount
                .windows()
                .iter()
                .find(|window| window.id.name() == name)
                .map(|window| window.id)
                .ok_or_else(|| {
                    Refusal::new(
                        RefusalKind::PaneUnknown,
                        format!("the recipe declares no window `{name}`"),
                    )
                })?;
            let pane = mount.focus_window(window)?;
            Ok(format!("focus {name} → pane #{}", pane.0))
        }
        other => Err(Refusal::new(
            RefusalKind::ToolUndeclared,
            format!(
                "`{other}` is not a shell command (split, close, focus, layer, grab, quit)"
            ),
        )),
    }
}

fn parse_pane(word: Option<&str>) -> Result<shell::PaneId, Refusal> {
    let word = word.unwrap_or("");
    word.parse::<u64>().map(shell::PaneId).map_err(|_| {
        Refusal::new(
            RefusalKind::ArgumentMalformed,
            format!("a pane is named by its own number, not {word:?}"),
        )
    })
}

/// The `serve` mode: the reference world's gate, over stdin/stdout, until the client's input ends.
/// Exit 0 when the session ran to a clean end; 1 when it could not be read or written — a refusal
/// the client could not otherwise see, printed where diagnostics belong.
fn serve_stdio_session() -> i32 {
    let roster = match process::roster() {
        Ok(roster) => roster,
        Err(refusals) => {
            eprintln!("serve: the reference roster is refused by name: {refusals}");
            return 1;
        }
    };
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match reference::serve_stdio(&roster, 1, &mut stdin.lock(), &mut stdout.lock()) {
        Ok(session) => {
            eprintln!(
                "serve: {} request(s): {} answered, {} refused by name",
                session.requests, session.answered, session.refused
            );
            0
        }
        Err(refusal) => {
            eprintln!("serve: the session could not continue: {refusal}");
            1
        }
    }
}

