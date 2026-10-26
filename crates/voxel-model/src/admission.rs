//! The two doors: what enters the game and what enters the record, graded against declared floors.
//! Deny by default everywhere - a missing grade is a refusal, never a pass, and a floor never sits on a gate status.

use crate::axis::{AxisKind, CertificationTier, EvidenceGrade, EvidenceLevel, OrgRank};
use crate::refusal::{Refusal, RefusalKind, RefusalSet};
use crate::ticket::EvidenceOrigin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdmissionClass {
    Build,
    LiveLoad,
    WorldControl,
    Evidence,
}

impl AdmissionClass {
    pub const COUNT: usize = 4;
    pub const ALL: &[AdmissionClass] = &[
        AdmissionClass::Build,
        AdmissionClass::LiveLoad,
        AdmissionClass::WorldControl,
        AdmissionClass::Evidence,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            AdmissionClass::Build => "build",
            AdmissionClass::LiveLoad => "live_load",
            AdmissionClass::WorldControl => "world_control",
            AdmissionClass::Evidence => "evidence",
        }
    }
}

pub const CLASSES_ARE_TOTAL: [(); AdmissionClass::COUNT] = [(); AdmissionClass::ALL.len()];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Threshold {
    pub axis: AxisKind,
    pub floor: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grade<'a> {
    pub axis: AxisKind,
    pub token: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    pub class: AdmissionClass,
    pub refusals: RefusalSet,
}

impl Admission {
    pub fn is_admitted(&self) -> bool {
        self.refusals.is_empty()
    }
}

pub fn thresholds(class: AdmissionClass) -> &'static [Threshold] {
    match class {
        AdmissionClass::Build => &[
            Threshold { axis: AxisKind::TicketEvidence, floor: "B" },
            Threshold { axis: AxisKind::SupportLevel, floor: "supported" },
        ],
        AdmissionClass::LiveLoad => &[
            Threshold { axis: AxisKind::TicketEvidence, floor: "A" },
            Threshold { axis: AxisKind::SupportLevel, floor: "strongly_supported" },
        ],
        AdmissionClass::WorldControl => &[
            Threshold { axis: AxisKind::TicketEvidence, floor: "A" },
            Threshold { axis: AxisKind::SupportLevel, floor: "direct" },
        ],
        AdmissionClass::Evidence => &[
            Threshold { axis: AxisKind::TicketEvidence, floor: "B" },
            Threshold { axis: AxisKind::SupportLevel, floor: "supported" },
        ],
    }
}

pub fn admits(class: AdmissionClass, grades: &[Grade<'_>]) -> Admission {
    admits_with(class, thresholds(class), grades)
}

pub fn admits_with(class: AdmissionClass, table: &[Threshold], grades: &[Grade<'_>]) -> Admission {
    let mut refusals = RefusalSet::new();
    for threshold in table {
        let Some(grade) = grades.iter().find(|grade| grade.axis == threshold.axis) else {
            refusals.refuse(
                RefusalKind::UnsubstantiatedEvidence,
                format!(
                    "{}: no grade on {}; a missing instrument is a refusal, never a pass",
                    class.name(),
                    threshold.axis
                ),
            );
            continue;
        };
        let actual = match rank_of(threshold.axis, grade.token) {
            Ok(rank) => rank,
            Err(refusal) => {
                refusals.push(refusal);
                continue;
            }
        };
        let floor = match rank_of(threshold.axis, threshold.floor) {
            Ok(rank) => rank,
            Err(refusal) => {
                refusals.push(refusal);
                continue;
            }
        };
        if actual < floor {
            refusals.refuse(
                RefusalKind::UnsubstantiatedEvidence,
                format!(
                    "{}: {} {} is below the declared floor {}",
                    class.name(),
                    threshold.axis,
                    grade.token,
                    threshold.floor
                ),
            );
        }
    }
    Admission { class, refusals }
}

fn origin_letter_rank(token: &str) -> Result<u8, Refusal> {
    match token {
        "A" => Ok(3),
        "B" => Ok(2),
        "C" => Ok(1),
        "D" => Ok(0),
        other => Err(Refusal::owned(
            RefusalKind::UndeclaredName,
            format!("unknown evidence origin token {other:?}"),
        )),
    }
}

fn evidence_level_rank(token: &str) -> Result<u8, Refusal> {
    match token {
        "unverified" => Ok(0),
        "self_reported" => Ok(1),
        "inferred" => Ok(2),
        "supported" => Ok(3),
        "strongly_supported" => Ok(4),
        "direct" => Ok(5),
        other => Err(Refusal::owned(
            RefusalKind::UndeclaredName,
            format!("unknown evidence level token {other:?}"),
        )),
    }
}

fn rank_of(axis: AxisKind, token: &str) -> Result<u8, Refusal> {
    match axis {
        AxisKind::EvidenceGrade => EvidenceGrade::parse(token)
            .map(EvidenceGrade::strength)
            .map_err(|e| Refusal::owned(RefusalKind::UndeclaredName, e)),
        AxisKind::CertificationTier => CertificationTier::parse(token)
            .map(CertificationTier::ordinal)
            .map_err(|e| Refusal::owned(RefusalKind::UndeclaredName, e)),
        AxisKind::SupportLevel => evidence_level_rank(token),
        AxisKind::TicketEvidence => origin_letter_rank(token),
        AxisKind::OrgRank => OrgRank::parse(token)
            .map(OrgRank::ordinal)
            .map_err(|e| Refusal::owned(RefusalKind::UndeclaredName, e)),
        _ => Err(Refusal::owned(
            RefusalKind::UndeclaredName,
            format!("axis {} does not carry a ladder here", axis),
        )),
    }
}

#[cfg(test)]
mod tests {}