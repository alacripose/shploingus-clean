//! **What each segment is measured through: its crates and the file that witnesses it.**
//!
//! [`voxel_axis_matrix::Segment`] names the seven systems the game is verified in, but a segment is
//! not a crate: the world is three crates, the pattern system is a file inside the client, and the
//! axis layer is a folder. This table is the join, and it lives here rather than in the matrix
//! because it is a fact about **where evidence is found**, which is this crate's subject.
//!
//! # Every crate is in at least one segment, and a test says so
//!
//! The `noise`, `light`, `audio`, `stream`, `work`, `appearance` and `instrument` crates are not
//! named by any segment's one-sentence definition, and they are real: the world's noise, its sun,
//! its rain and the pool that bounds a frame are all things a player sees or pays for. They are
//! assigned to the segment that runs them, and [`SUBJECTS`] is checked against
//! [`voxel_package::census::Crate::ALL`] so a crate added to the workspace cannot be verified by
//! nothing — the same blindness the census's own table had for four crates until it compared itself
//! against the manifest.
//!
//! # The witness is the segment's own site
//!
//! Each subject names one file that **is** the segment, and the cell's witness is that file: the
//! shell's mount, the pattern module, the matrix's crate root. A witness borrowed from a sibling
//! crate would be a reading about a different system, so the site is checked to sit inside one of
//! the subject's own crates.

use voxel_axis_matrix::{Segment, SEGMENT_COUNT};
use voxel_package::census::Crate;

/// One segment's subjects: the crates that carry it and the file that witnesses it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Subject {
    /// The segment key.
    pub segment: Segment,
    /// The census crates this segment is read through. **At least one**, and every crate of the
    /// workspace appears in at least one subject.
    pub crates: &'static [Crate],
    /// The segment's own site: a file inside one of its crates, and the cell's witness.
    pub site: &'static str,
    /// The artifact format this subject measures, when it measures one — its schema id, which is
    /// also the row the census writes. `None` for the seven segments, which measure code.
    pub format: Option<&'static str>,
}

/// Every segment, in [`Segment::ALL`] order. **The order is asserted**, so a report that lists the
/// table reads the same order the matrix prints.
pub const SUBJECTS: [Subject; SEGMENT_COUNT] = [
    Subject {
        segment: Segment::Shell,
        crates: &[Crate::Client],
        site: "crates/voxel-client/src/shell/mod.rs",
        format: None,
    },
    Subject {
        segment: Segment::Ui,
        crates: &[Crate::Ui],
        site: "crates/voxel-ui/src/lib.rs",
        format: None,
    },
    Subject {
        segment: Segment::Game,
        crates: &[Crate::Shploingus, Crate::Audio, Crate::Stream, Crate::Work],
        site: "crates/shploingus/src/main.rs",
        format: None,
    },
    Subject {
        segment: Segment::World,
        crates: &[
            Crate::Model,
            Crate::Noise,
            Crate::Light,
            Crate::Mesh,
            Crate::Shapers,
            Crate::Appearance,
        ],
        site: "crates/voxel-model/src/lib.rs",
        format: None,
    },
    Subject {
        segment: Segment::Pattern,
        crates: &[Crate::Client],
        site: "crates/voxel-client/src/pattern.rs",
        format: None,
    },
    Subject {
        segment: Segment::Axis,
        crates: &[
            Crate::Ladder,
            Crate::Coverage,
            Crate::Unit,
            Crate::Claim,
            Crate::View,
            Crate::Civilians,
            Crate::Being,
            Crate::Matrix,
            Crate::Verify,
            Crate::Artifact,
            Crate::Security,
            Crate::AxisMcp,
            Crate::Balance,
        ],
        site: "crates/axis/matrix/src/lib.rs",
        format: None,
    },
    Subject {
        segment: Segment::Tooling,
        crates: &[Crate::Package, Crate::Mcp, Crate::Von, Crate::Instrument],
        site: "crates/voxel-package/src/lib.rs",
        format: None,
    },
];

/// **The artifact format's subject**, measured through the same [`Subject`] shape as a segment:
/// a witness on disk, rows, a star and a decline. It is not in [`SUBJECTS`] because it measures a
/// format rather than one of the seven segments, and its reading is the unit `grade` ladder rather
/// than `evidence_grade`.
///
/// The witness is the frame the reader grades, so a cell about the format points at the artifact
/// it read rather than at the crate that happens to implement the reader.
pub const FORMAT: Subject = Subject {
    segment: Segment::Tooling,
    crates: &[Crate::Package],
    site: "display-packages/alpha-shell/frames/MAP-0001.frame",
    format: Some("shploingus.cellframe/1"),
};

/// The subject for a segment. **Total by construction**: [`SUBJECTS`] is as long as the segment
/// table and is asserted to be in the same order, so this cannot fail to find one.
pub fn subject_of(segment: Segment) -> &'static Subject {
    SUBJECTS
        .iter()
        .find(|subject| subject.segment == segment)
        .expect("the subject table is as wide as the segment table")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn root() -> std::path::PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    #[test]
    fn the_table_is_as_wide_as_the_segments_and_in_their_order() {
        assert_eq!(SUBJECTS.len(), SEGMENT_COUNT);
        for (subject, segment) in SUBJECTS.iter().zip(Segment::ALL) {
            assert_eq!(subject.segment, segment, "the table is out of order");
            assert_eq!(subject_of(segment).site, subject.site);
            assert!(
                !subject.crates.is_empty(),
                "{} has no crate",
                segment.name()
            );
        }
    }

    #[test]
    fn every_crate_is_measured_by_at_least_one_segment() {
        for crate_ in Crate::ALL {
            assert!(
                SUBJECTS
                    .iter()
                    .any(|subject| subject.crates.contains(&crate_)),
                "{} is in no segment, so the verifier would never read it",
                crate_.name()
            );
        }
    }

    #[test]
    fn every_witness_is_a_file_inside_one_of_its_own_crates() {
        let root = root();
        for subject in SUBJECTS {
            assert!(
                root.join(subject.site).is_file(),
                "{}'s witness `{}` is not on disk",
                subject.segment.name(),
                subject.site
            );
            assert!(
                subject
                    .crates
                    .iter()
                    .any(|crate_| subject.site.starts_with(&format!("{}/", crate_.path()))),
                "{}'s witness `{}` is outside every crate it names ({:?})",
                subject.segment.name(),
                subject.site,
                subject.crates.iter().map(|c| c.name()).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn the_format_subject_names_the_frame_its_reader_grades() {
        let root = root();
        assert_eq!(FORMAT.format, Some("shploingus.cellframe/1"));
        assert_eq!(FORMAT.segment, Segment::Tooling);
        assert!(
            root.join(FORMAT.site).is_file(),
            "the format's witness `{}` is not on disk",
            FORMAT.site
        );
        assert!(!SUBJECTS.iter().any(|subject| subject.site == FORMAT.site));
    }

    #[test]
    fn two_segments_do_not_share_a_witness_they_cannot_relate() {
        // The shell and the pattern system are both in `voxel-client`; they must still be witnessed
        // by different files, or one reading would be used for two systems.
        let mut sites: Vec<&str> = SUBJECTS.iter().map(|subject| subject.site).collect();
        sites.sort_unstable();
        sites.dedup();
        assert_eq!(sites.len(), SEGMENT_COUNT, "two segments share a witness");
    }
}
