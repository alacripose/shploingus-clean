//! **The field plane — one seam, and the declaring stage answers.**
//!
//! Derived from `docs/CLEAN-ROOM-PLAN-2026-10-11.md` §2 U3 (*“the field plane's `field_at` seam”*),
//! `docs/MISSING-SYSTEMS.md` §1 (the field plane is the *production seam*: a declared field is routed
//! through the stage the field's own `outputs` declaration names) and `docs/SHAPERS.md` §2 rule 4
//! (*“Fields are stage-coupled, not independent noise”: a climate field must be a function of the
//! terrain above it, not a second unrelated fBm*). The binding seam is
//! `specs/003-u3-shapers/contracts/u3-seams.md` C3.
//!
//! # Why this is the only way to read a field
//!
//! There is exactly one producer per field (admission refuses two), and this function **asks that
//! producer** rather than sampling anything itself. So there is no second path that could answer with
//! unrelated noise, and a field with no producer cannot be answered at all: it is refused by name.
//!
//! The distinction the contract draws between its two refusals is kept here:
//!
//! * `region_required` — the *request* is wrong: this law is about a relation between columns and a
//!   `Unit` request cannot express one.
//! * `site_alone` — the *region* is too small: the request was a region, but a patch with no
//!   neighbours still cannot evaluate a relation.

use crate::process::{evaluate, Outcome};
use crate::refusal::{Refusal, RefusalKind};
use crate::stage::{Declared, FieldId, ProcessRequest};
use crate::world::{Tick, WorldIdentity};

/// The ordinal of the stage that declares `field`, once the field is known to **have** a declaring
/// stage and that stage accepts the shape of the request. One set of checks, so an ask for one field
/// and an ask for many cannot disagree about what "declared" means.
fn declaring_ordinal(
    roster: &Declared,
    field: FieldId,
    request: ProcessRequest,
) -> Result<usize, Refusal> {
    let stage = roster.producer_of(field).ok_or_else(|| {
        Refusal::new(
            RefusalKind::FieldUndeclared,
            format!("no stage in this roster declares {}", field.name()),
        )
    })?;
    if stage.requires_region && matches!(request, ProcessRequest::Unit { .. }) {
        return Err(Refusal::new(
            RefusalKind::RegionRequired,
            format!(
                "{} is a law about a relation between columns: it needs a region, and the request is a lone site",
                stage.name
            ),
        ));
    }
    roster.ordinal_of(stage.name).ok_or_else(|| {
        Refusal::new(
            RefusalKind::FieldUndeclared,
            format!("{} is not in the declared order", stage.name),
        )
    })
}

/// Read one field out of an evaluation that has already run. A refusal from the declaring stage is
/// the answer: it is passed through, not swallowed.
fn read_field(
    roster: &Declared,
    evaluation: &crate::process::Evaluation,
    field: FieldId,
) -> Result<Vec<f64>, Refusal> {
    let stage = roster.producer_of(field).ok_or_else(|| {
        Refusal::new(
            RefusalKind::FieldUndeclared,
            format!("no stage in this roster declares {}", field.name()),
        )
    })?;
    let ordinal = roster.ordinal_of(stage.name).ok_or_else(|| {
        Refusal::new(
            RefusalKind::FieldUndeclared,
            format!("{} is not in the declared order", stage.name),
        )
    })?;
    match evaluation.outcomes.get(ordinal) {
        Some(Outcome::Produced(produced)) => produced
            .field(field)
            .map(<[f64]>::to_vec)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::FieldUndeclared,
                    format!("{} produced a value, but not for {}", stage.name, field.name()),
                )
            }),
        Some(Outcome::Refused(refusal)) => Err(refusal.clone()),
        None => Err(Refusal::new(
            RefusalKind::FieldUndeclared,
            format!("{} was never reached in this run", stage.name),
        )),
    }
}

/// Ask a declared field for its values over a request. The declaring stage produces them, or the
/// call refuses by name.
pub fn field_at(
    roster: &Declared,
    identity: WorldIdentity,
    tick: Tick,
    field: FieldId,
    request: ProcessRequest,
    workers: usize,
) -> Result<Vec<f64>, Refusal> {
    declaring_ordinal(roster, field, request)?;
    let evaluation = evaluate(roster, identity, tick, request, workers)?;
    read_field(roster, &evaluation, field)
}

/// Ask **several** declared fields of one request, in one evaluation. The seam is widened because a
/// consumer needs a *set* of fields describing the *same* columns (a mesh reads height, sea level and
/// lithology of one patch), and asking three times would evaluate the roster three times and, worse,
/// would make "the same patch" mean three different runs. One evaluation, one answer, and the same
/// checks per field: an undeclared field is still refused by name, before any work is done.
pub fn fields_at(
    roster: &Declared,
    identity: WorldIdentity,
    tick: Tick,
    fields: &[FieldId],
    request: ProcessRequest,
    workers: usize,
) -> Result<Vec<Vec<f64>>, Refusal> {
    for field in fields {
        declaring_ordinal(roster, *field, request)?;
    }
    if fields.is_empty() {
        return Ok(Vec::new());
    }
    let evaluation = evaluate(roster, identity, tick, request, workers)?;
    let mut values = Vec::with_capacity(fields.len());
    for field in fields {
        values.push(read_field(roster, &evaluation, *field)?);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::VoxelPos;
    use crate::process::{declared_request, roster, DECLARED_PATCH_SIDE};
    use crate::stage::Dials;
    use crate::world::DeclaredPeriod;

    fn identity() -> WorldIdentity {
        WorldIdentity::new(0x5EED, DeclaredPeriod::seconds(1).expect("a period"))
    }

    #[test]
    fn a_field_comes_from_the_stage_that_declares_it() {
        let roster = roster().expect("the reference roster must admit");
        // `elevation` declares `terrain` as its producer, and the value is that stage's output.
        assert_eq!(
            roster.producer_of(FieldId::Elevation).map(|stage| stage.name),
            Some("terrain")
        );
        let elevation = field_at(
            &roster,
            identity(),
            Tick(0),
            FieldId::Elevation,
            declared_request(),
            1,
        )
        .expect("a declared field with a producing stage");
        assert_eq!(elevation.len(), (DECLARED_PATCH_SIDE * DECLARED_PATCH_SIDE) as usize);
        assert!(
            elevation.iter().all(|height| height.is_finite()),
            "every column has an elevation"
        );
        assert!(
            elevation.iter().any(|height| *height != elevation[0]),
            "a terrain field is not constant"
        );
    }

    #[test]
    fn several_fields_of_one_request_are_answered_as_the_narrow_seam_answers_them() {
        let roster = roster().expect("the reference roster must admit");
        let request = declared_request();
        let asked = [FieldId::Elevation, FieldId::WaterLevel, FieldId::Lithology];
        let values = fields_at(&roster, identity(), Tick(0), &asked, request, 1)
            .expect("all three are declared");
        assert_eq!(values.len(), asked.len());
        for (index, field) in asked.iter().enumerate() {
            let alone = field_at(&roster, identity(), Tick(0), *field, request, 1)
                .expect("the same field on its own");
            assert_eq!(
                values[index], alone,
                "the widened seam must answer {} exactly as the narrow one does",
                field.name()
            );
        }
        // A worker count stays a pure performance knob on the widened seam too.
        let four = fields_at(&roster, identity(), Tick(0), &asked, request, 4).expect("four workers");
        assert_eq!(four, values);

        // A field with no producer is refused by name, and the reason is the declaring stage's own.
        let refused = fields_at(
            &roster,
            identity(),
            Tick(0),
            &[FieldId::Elevation, FieldId::CaveOccupancy],
            request,
            1,
        )
        .expect_err("cave occupancy is owed");
        assert_eq!(refused.kind(), RefusalKind::BodyOwed);
        assert!(refused.detail().contains("dissolution field"), "{refused}");

        // An empty ask is not a refusal: nothing was named, so nothing is owed.
        assert!(fields_at(&roster, identity(), Tick(0), &[], request, 1)
            .expect("no fields")
            .is_empty());
    }

    #[test]
    fn a_regions_only_law_asked_for_a_lone_site_is_refused_by_name() {
        let roster = roster().expect("the reference roster must admit");
        let lone = ProcessRequest::Unit { at: VoxelPos::new(3, 4, 0) };
        let refused = field_at(&roster, identity(), Tick(0), FieldId::Discharge, lone, 1)
            .expect_err("discharge is a relation between columns");
        assert_eq!(refused.kind(), RefusalKind::RegionRequired);
        assert_eq!(refused.kind().name(), "region_required");
        assert!(refused.detail().contains("hydrology"), "{refused}");

        // A region request that is too small is a different refusal, with a different reason.
        let tiny = ProcessRequest::Region { origin: VoxelPos::new(0, 0, 0), side: 1 };
        let refused = field_at(&roster, identity(), Tick(0), FieldId::Discharge, tiny, 1)
            .expect_err("one column has no neighbour");
        assert_eq!(refused.kind(), RefusalKind::SiteAlone);
        assert!(refused.detail().contains("lone site"), "{refused}");
    }

    #[test]
    fn a_field_no_stage_declares_is_refused_by_name() {
        // A roster without `terrain` in it: `elevation` has no producer at all.
        let partial = Declared::new(vec![crate::stage::Stage {
            name: "tectonics",
            source: "Kreischer, Bommes et al., Procedural Tectonic Planets, CGF 2019",
            reads: &[],
            writes: &[FieldId::Uplift, FieldId::PlateId],
            cadence: crate::stage::Cadence::Detail,
            requires_region: false,
            dials: Dials::new(&[
                ("plate_scale_cells", 64.0),
                ("boundary_uplift_m", 120.0),
                ("plate_bias_m", 40.0),
            ]),
            body: crate::shapers::terrain::tectonics,
        }])
        .expect("a one-stage roster is a valid graph");
        let refused = field_at(
            &partial,
            identity(),
            Tick(0),
            FieldId::Elevation,
            declared_request(),
            1,
        )
        .expect_err("nothing declares elevation here");
        assert_eq!(refused.kind(), RefusalKind::FieldUndeclared);
        assert!(refused.detail().contains("elevation"), "{refused}");

        // And the field that *is* declared still answers, with four workers as well as one.
        let uplift = field_at(&partial, identity(), Tick(0), FieldId::Uplift, declared_request(), 4)
            .expect("tectonics declares uplift");
        assert_eq!(uplift.len(), (DECLARED_PATCH_SIDE * DECLARED_PATCH_SIDE) as usize);
    }

    #[test]
    fn an_owed_field_passes_the_declaring_stage_s_refusal_through() {
        let roster = roster().expect("the reference roster must admit");
        let refused = field_at(
            &roster,
            identity(),
            Tick(0),
            FieldId::CaveOccupancy,
            declared_request(),
            1,
        )
        .expect_err("cave occupancy is owed");
        assert_eq!(refused.kind(), RefusalKind::BodyOwed);
        assert!(refused.detail().contains("dissolution field"), "{refused}");
    }
}
