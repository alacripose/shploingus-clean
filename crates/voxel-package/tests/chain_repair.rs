//! **Repair the live chain — deliberately, once, with the record as it was found kept.**
//!
//! # What this removes, and why removing it is not a truncation
//!
//! `filing::grow` used to append `linked_records`' whole output — `prior` followed by the new
//! records — to a file that already held `prior`. Every grow doubled the record, and on this record
//! it happened **more than once**: 105,323,302 → 842,609,979 → 1,685,220,970 is ×8 and then ×2 of a
//! chain that kept gaining records.
//!
//! **The first version of this repair was a cut, and the fold refused it.** It found the outermost
//! repeat exactly (`842,609,979`; `842,614,525` bytes, 50.0% of the file) and the chain that would
//! have been written held **166,406** records that did not link — because cutting off the outermost
//! copy leaves the inner ones behind. That refusal is why the repair is a **compaction**: keep every
//! frame that folds, drop every frame that did not, decided by the reader's own acceptance rule
//! (`compiler::compact` → `append_event`).
//!
//! Nothing is lost by that, because the dropped frames are *copies* of frames that are kept — but
//! "nothing is lost" is a claim, so this refuses unless the compacted chain folds with **zero**
//! corrupt records, folds the **same number of records**, and carries the board's tickets
//! **identically, id for id and objective for objective**. The file as it was found is renamed into
//! `logs/chain-repair/` rather than deleted, and a manifest of both digests is filed beside it.
//!
//! # Why it is a runner and not a build step
//!
//! It is the one operation in the project that can make the record smaller, and D98 says a finding
//! a person cannot choose to run is incomplete. Run it with:
//!
//! ```text
//! cargo test --offline -p voxel-package --test chain_repair -- --ignored --nocapture
//! ```
//!
//! # The cost it fixes
//!
//! Every read of this record loads 1.68 GB into memory and folds all of it. The game's `file` verb
//! does that twice, plus a read-back and a 1.68 GB byte comparison, **on the winit main thread** —
//! which is the 102-second freeze the record already carries. More than half the file is copies of
//! frames already in it, so more than half of that time and memory is being spent on a defect.

use voxel_package::features::workspace_root;
use voxel_package::filing;

fn root() -> std::path::PathBuf {
    workspace_root(&std::env::current_dir().expect("a current directory"))
        .expect("the workspace root")
}

#[test]
#[ignore = "compacts the canonical record; run it deliberately, once"]
fn repair_the_chain_onto_the_record() {
    let root = root();
    let path = filing::chain_path(&root);
    let keep = root.join("logs/chain-repair");

    let before_bytes = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
    println!("chain-repair-file\t{}", path.display());
    println!("chain-repair-bytes-before\t{before_bytes}");

    // **What the file is, before anything is written.** The fold is the same one every reader uses,
    // so the number of unlinked frames reported here is the number that will be dropped — and a file
    // with nothing to drop stops here with that word rather than being rewritten.
    let bytes = std::fs::read(&path).expect("the record reads");
    let before = voxel_package::compiler::fold_bytes(&bytes).expect("the file folds");
    println!("chain-repair-frames-before\tfolded={} corrupt={}", before.folded, before.corrupt);
    if before.corrupt == 0 {
        println!(
            "chain-repair\tnothing-to-repair\tevery frame links, so no repair is owed and nothing \
             was written"
        );
        return;
    }
    let compaction =
        voxel_package::compiler::compact(&bytes).expect("the frame walk completes");
    println!("chain-repair-frames-would-keep\t{}", compaction.kept);
    println!("chain-repair-frames-would-drop\t{}", compaction.dropped);
    println!(
        "chain-repair-would-write\t{} bytes of {} ({:.1}%)",
        compaction.bytes.len(),
        bytes.len(),
        100.0 * compaction.bytes.len() as f64 / bytes.len() as f64
    );
    // The doubled bytes do not need to be resident while the repaired chain is folded; `repair`
    // reads the file again for itself.
    drop(bytes);

    let repair = filing::repair(&root, &keep).expect("the unlinked frames are dropped");
    let manifest = repair.describe();
    println!("{manifest}");
    filing::file(&keep.join("workspace.chain.repair.txt"), manifest.as_bytes())
        .expect("the manifest is filed, and a second run cannot overwrite it");

    // **Read the repaired chain back off disk, the way every other reader does.** A repair that
    // reported its own in-memory result would be a claim about a file nobody had read.
    let read = voxel_package::compiler::read_back(&path).expect("the repaired chain reads");
    println!("chain-repair-after-read\tfolded={}", read.folded);
    println!("chain-repair-after-corrupt\t{}", read.corrupt);
    println!("chain-repair-after-tickets\t{}", read.board.tickets().len());
    println!(
        "chain-repair-after-bytes\t{}",
        std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
    );
    if read.corrupt != 0 {
        for detail in read.corrupt_detail.iter().take(5) {
            println!("chain-repair-after-corrupt-detail\t{detail}");
        }
    }
    println!(
        "chain-repair-kept\t{} (the record as it was found)",
        keep.join("workspace.chain.as-found").display()
    );
}
