//! **The corpus path, not a fixture.** A prose record must produce its act from its own sentence,
//! and a record that already carries a block must survive being re-derived from the prose it says
//! it compiled — otherwise `derived 0` is a routing failure that a passing compile cannot see.

use std::path::PathBuf;

use voxel_mcp::patch_recipe_coverage;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("the crate sits two levels under the workspace root")
        .to_path_buf()
}

#[test]
fn a_prose_record_derives_and_an_existing_block_is_re_derived_from_its_prose() {
    let out = patch_recipe_coverage(&root(), "T-0954", false).expect("the corpus is readable");
    assert!(
        out.contains("records\t76"),
        "the corpus is 76 records, not blocks plus gaps:\n{out}"
    );
    assert!(
        out.contains("derived\tPATCH-045"),
        "a prose record with a stated site and check must derive an act:\n{out}"
    );
    assert!(
        out.contains("derived-compiled\t1"),
        "and that derived act must compile through the matrix:\n{out}"
    );
    assert!(
        out.lines().any(|line| line.starts_with("rederived\tPATCH-060")),
        "the block must agree with the act its own prose names:\n{out}"
    );
    assert!(
        out.lines().any(|line| {
            line.starts_with("rederived\tPATCH-060")
                && line.contains("block 1\tmatch\tsegment,site,check,witness")
        }),
        "the re-derivation compared the sourced coordinates and they agree:\n{out}"
    );
    assert!(
        out.contains("rederived-match\t1"),
        "one block was re-derived and it matched:\n{out}"
    );
    assert!(
        !out.lines().any(|line| line.starts_with("patches\t")),
        "the old mislabelled count must be gone:\n{out}"
    );
}
