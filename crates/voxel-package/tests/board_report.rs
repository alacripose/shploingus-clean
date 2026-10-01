//! **The board reports its own duplicate filings, on the real record.**
//!
//! # Why this is a test and not a measurement I printed once
//!
//! This repository's chain was filed with its 42 work tickets **three times** in one session.
//! A one-shot guard compared a marker-prefixed objective (`"[BLD] resize a pane…"`) against
//! the marker-stripped text [`voxel_model::ticket::Board`] actually holds
//! (`" resize a pane…"`), so the guard could never succeed; three runs filed three times.
//!
//! **Every test stayed green.** `Board::from_chain` merged on `id` — which is
//! `{prefix}-{index}` — so two filings of one objective at different positions were two
//! different tickets by the fold's own rule, and there was no duplicate check anywhere: not
//! on the objective, not on the digest, not on anything. The reader could not see its own
//! class of defect.
//!
//! So this asserts a **property, not a count.** A test that pinned `84` would be a snapshot
//! that breaks the moment the duplicates are retired — which is the repair the record owes —
//! and a snapshot that breaks on repair teaches the next reader that repair is a defect.

use voxel_model::ticket::Board;
use voxel_package::features::workspace_root;
use voxel_package::filing;

fn root() -> std::path::PathBuf {
    workspace_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).expect("a workspace root")
}

#[test]
fn the_board_names_every_duplicate_filing_and_says_the_count_is_not_a_count_of_work() {
    let board = Board::from_chain(&filing::read(&root()));
    let report = board.describe();
    // **The whole history, every run.** C-82.9 filed `BLD-0017` for exactly this: the report is
    // built, asserted against, and never printed, so the board is a view only a reader of this
    // file can see. C-45 is the rule it offends — *the run is the user's to read* — and C-40
    // names the other half: a finding that can be made visible is made visible. The assertions
    // below check the report; **this line is what lets a person read it.**
    println!("{report}");

    // **The counts are always printed**, duplicated or not — a bare count is the
    // absence-as-zero defect D107 names, because 128 reads exactly like 128 real tickets.
    assert!(report.contains(&format!("board-tickets\t{}", board.len())), "{report}");
    assert!(
        report.contains(&format!("board-duplicate-filings\t{}", board.duplicates().len())),
        "{report}"
    );

    if board.duplicates().is_empty() {
        // **Absence rendered as absence.** Zero is a measurement and says so; a silent empty
        // line would be indistinguishable from a check that did not run.
        assert!(
            report.contains("every ticket on this board was filed once"),
            "{report}"
        );
    } else {
        assert!(
            report.contains("is NOT a count of"),
            "**a ticket count that does not say it is inflated is the defect that let this \
             happen.** {report}"
        );
        // **Every repeat filing is accounted for by a named group.** The first version of this
        // assertion was `len - repeats == distinct` and it failed with `44` against `42` — and
        // the code was right: the board also holds tickets filed exactly **once**, which are not
        // duplicates and so are in neither list. The correct invariant is that every ticket
        // belongs either to a duplicate group or to the singles, and none is unaccounted for.
        let repeated: usize = board.duplicates().iter().map(|d| d.repeated_at.len()).sum();
        let in_groups = board.duplicates().len() + repeated;
        let singles = board.len().saturating_sub(in_groups);
        assert!(
            board.len() >= in_groups,
            "{} tickets cannot hold {in_groups} positions across {} duplicate groups",
            board.len(),
            board.duplicates().len()
        );
        println!(
            "board: {} tickets = {} distinct duplicated objectives x their filings + {singles} \
             filed exactly once",
            board.len(),
            board.duplicates().len()
        );
        // **Every duplicate is named in full, and there is no deferral branch at all.** The
        // first version of this printed five groups plus `board-duplicates-more 37`, and the
        // arithmetic here multiplied by five — a reader could see the count of a defect it
        // could see all of. Hyperskill's selector has no such branch either: it either
        // publishes a complete selection or returns `IncompleteSelection` carrying every
        // omitted id. So the assertion is that the count is *exactly* the number of groups,
        // and that the deferral line is gone from the source of truth, not merely unused.
        let named = report.matches("board-duplicate\t").count();
        assert_eq!(
            named,
            board.duplicates().len(),
            "the report names every duplicate group exactly once and defers none"
        );
        assert!(
            !report.contains("board-duplicates-more"),
            "**a report that samples a defect it can see all of is the session's own defect.** \
             {report}"
        );
        // **Re-derived, not counted.** A correct count over wrong contents would pass the
        // equality above, so each group's positions and its full objective are checked
        // against the board's own list rather than against a snapshot.
        for duplicate in board.duplicates() {
            let repeats = duplicate
                .repeated_at
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            assert!(
                report.contains(&format!(
                    "board-duplicate\t{} and {repeats}\t{}",
                    duplicate.first_index, duplicate.objective
                )),
                "group filed first at {} must appear with its full objective and every position: \
                 {report}",
                duplicate.first_index
            );
        }
        for duplicate in board.duplicates() {
            assert!(duplicate.first_index < duplicate.repeated_at[0]);
            assert!(
                duplicate.repeated_at.windows(2).all(|w| w[0] < w[1]),
                "positions ascend in chain order: {:?}",
                duplicate.repeated_at
            );
        }
    }
}

#[test]
fn every_ticket_on_the_board_is_named_in_full_and_none_is_deferred() {
    // **The accounting is closed, not sampled.** The report names every duplicate because a
    // reader who cannot see all of a defect cannot see it — and it named **no** ticket that was
    // filed once, so the work that is *not* duplicated existed only as an arithmetic remainder.
    // This walks the board and requires every position to appear in exactly one of the two lists.
    let board = Board::from_chain(&filing::read(&root()));
    let report = board.describe();

    let mut named: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    for duplicate in board.duplicates() {
        named.insert(duplicate.first_index);
        named.extend(duplicate.repeated_at.iter().copied());
        for position in std::iter::once(&duplicate.first_index)
            .chain(duplicate.repeated_at.iter())
        {
            let row = report
                .lines()
                .find(|line| line.starts_with("board-duplicate\t") && {
                    line.split('\t').nth(1).is_some_and(|cell| {
                        cell.split(" and ")
                            .flat_map(|part| part.split(", "))
                            .any(|n| n == position.to_string())
                    })
                })
                .unwrap_or_else(|| {
                    panic!("the duplicate at {position} is named in the report:\n{report}")
                });
            assert!(
                row.contains(&duplicate.objective),
                "and it is named with its full objective: {row}"
            );
        }
    }
    for ticket in board.tickets() {
        if named.contains(&ticket.event_index) {
            continue;
        }
        let row = report
            .lines()
            .find(|line| {
                line.starts_with("board-ticket\t")
                    && line.split('\t').nth(1) == Some(ticket.event_index.to_string().as_str())
            })
            .unwrap_or_else(|| {
                panic!(
                    "the singly-filed ticket at {} is named in the report:\n{report}",
                    ticket.event_index
                )
            });
        assert!(
            row.contains(&ticket.objective),
            "**a ticket that exists only as a remainder is a ticket nobody can read**: {row}"
        );
    }
    // And the two lists together account for every ticket, with none in both.
    let singles = report.lines().filter(|line| line.starts_with("board-ticket\t")).count();
    assert_eq!(
        named.len() + singles,
        board.len(),
        "every one of the {} tickets is named exactly once, in one list or the other",
        board.len()
    );
    assert!(
        !report.contains("board-tickets-more") && !report.contains("board-duplicates-more"),
        "and there is no deferral line to hide behind:\n{report}"
    );
}

#[test]
fn the_record_is_filed_and_the_board_reads_it() {
    // **The assertion C-73 opened the whole chain question with**, and it is here because a
    // test over a temporary directory is evidence about the temporary directory. For the whole
    // of this project's existence every chain write went to `std::env::temp_dir()`.
    assert!(
        filing::is_filed(&root()),
        "**the record is on disk.** If this fails, every measurement in this file is a \
         measurement of nothing."
    );
    assert!(
        !filing::read(&root()).is_empty(),
        "and it holds events, so the board above is reading something"
    );
}