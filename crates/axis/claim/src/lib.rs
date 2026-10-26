//! **A claim is a quote, a kind and a computed support — and nothing else.**
//!
//! Four rules are the crate:
//!
//! 1. **The quote is the unit's text, exactly.** One changed character makes the claim's id stop
//!    matching and [`Claim::verify`] refuses it by that id, not by a diff summary.
//! 2. **A kind records its basis.** [`Claim::kind_basis`] is `heuristic:<signals>` or
//!    `heuristic:default`; there is no empty basis, because *"the quote is always right, the kind
//!    is sometimes wrong"* and a reader has to be able to see which rule guessed.
//! 3. **Support is computed, never asserted.** The field is private, [`Claim::derive`] is the only
//!    producer, and there is no public constructor that takes a strength. A caller cannot hand
//!    this crate its own opinion of how strong a sentence is.
//! 4. **The origin is the derivation, not a model.** Every claim is
//!    [`EvidenceOrigin::Derived`] (`D`): this tree read the bytes. An oracle's answer is a
//!    different origin and arrives through a different layer.
//!
//! # What a claim is not
//!
//! It is not a section, a report line or a ticket. [`voxel_axis_view`] places claims in a document;
//! this crate only decides what one unit's claim is and whether the claim still describes it.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use voxel_axis_ladder::{EvidenceLevel, EvidenceOrigin, ProseKind};
use voxel_axis_unit::Unit;
use voxel_model::refusal::{Refusal, RefusalKind};
use voxel_model::Identity;

/// The domain tag under which a claim id is hashed.
const CLAIM_DOMAIN: &str = "shploingus/axis/claim";

/// How many hex digits a claim id keeps, the record's usual forty.
const ID_DIGITS: usize = 40;

/// One claim about one unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    /// `claim_` followed by forty hex digits.
    pub id: String,
    /// The unit this claim quotes, by the unit's id.
    pub unit: String,
    /// What kind of statement the unit is.
    pub kind: ProseKind,
    /// Why the kind was chosen: `heuristic:<signals>` or `heuristic:default`.
    pub kind_basis: String,
    /// The unit's text, character for character.
    pub quote: String,
    /// The strength the derivation computed. **Private on purpose**; read it through
    /// [`Claim::support`].
    support: EvidenceLevel,
    /// How the claim came to exist. Always [`EvidenceOrigin::Derived`] here.
    pub origin: EvidenceOrigin,
}

impl Claim {
    /// **The only constructor.** It reads the unit's text, derives a kind, records the signal
    /// that chose it, and computes the support from that basis.
    ///
    /// The quote is exact by construction, but exactness is checked separately by [`verify`];
    /// the support grades the claim's **kind**, which is the heuristic part. A specific signal is
    /// `Supported`; the default guess is `Inferred`.
    pub fn derive(unit: &Unit) -> Claim {
        let (kind, basis) = classify(&unit.text);
        let support = support_for(&basis);
        let id = Claim::id_of(&unit.id, kind, &basis, &unit.text);
        Claim {
            id,
            unit: unit.id.clone(),
            kind,
            kind_basis: basis,
            quote: unit.text.clone(),
            support,
            origin: EvidenceOrigin::Derived,
        }
    }

    /// The support the derivation computed from the kind's basis.
    ///
    /// **There is no setter and no public field.** A caller that wants a different strength must
    /// write a different sentence or a different instrument; it cannot edit this one.
    pub const fn support(&self) -> EvidenceLevel {
        self.support
    }

    /// Whether this claim still describes `unit` exactly.
    ///
    /// The refusal names the claim's own id, so a changed quote is traceable to the claim it
    /// invalidated rather than to the file around it.
    pub fn verify(&self, unit: &Unit) -> Result<(), Refusal> {
        if self.unit != unit.id {
            return Err(refuse(
                &self.id,
                format!("the claim quotes unit {} but was handed unit {}", self.unit, unit.id),
            ));
        }
        if self.quote != unit.text {
            return Err(refuse(
                &self.id,
                "the quote diverges from its unit's text; a claim that quotes a sentence it was \
                 not derived from is a claim about a different sentence"
                    .to_string(),
            ));
        }
        if !valid_basis(&self.kind_basis) {
            return Err(refuse(
                &self.id,
                format!("kind_basis {:?} is not heuristic:<signals> or heuristic:default", self.kind_basis),
            ));
        }
        let expected = support_for(&self.kind_basis);
        if self.support != expected {
            return Err(refuse(
                &self.id,
                format!(
                    "support {} was not computed from basis {:?}; that basis derives {}",
                    self.support.as_str(),
                    self.kind_basis,
                    expected.as_str()
                ),
            ));
        }
        if self.origin != EvidenceOrigin::Derived {
            return Err(refuse(
                &self.id,
                "origin is not the derivation rung; a derived claim cannot wear an oracle's letter"
                    .to_string(),
            ));
        }
        if self.id != Claim::id_of(&unit.id, self.kind, &self.kind_basis, &unit.text) {
            return Err(refuse(
                &self.id,
                "the claim id does not hash its own unit, kind, basis and quote".to_string(),
            ));
        }
        Ok(())
    }

    /// The deterministic id of a claim over its unit, kind, basis and quote.
    fn id_of(unit: &str, kind: ProseKind, basis: &str, quote: &str) -> String {
        let digest = Identity::of_fields(
            CLAIM_DOMAIN,
            &[unit.as_bytes(), kind.as_str().as_bytes(), basis.as_bytes(), quote.as_bytes()],
        )
        .to_string();
        let body: String = digest
            .trim_start_matches("sha256:")
            .chars()
            .take(ID_DIGITS)
            .collect();
        format!("claim_{body}")
    }
}

fn refuse(id: &str, detail: String) -> Refusal {
    Refusal::owned(
        RefusalKind::UnsubstantiatedEvidence,
        format!("{id}: {detail}"),
    )
}

fn valid_basis(basis: &str) -> bool {
    let Some(rest) = basis.strip_prefix("heuristic:") else {
        return false;
    };
    !rest.trim().is_empty()
}

/// **The one support rule.** A claim whose kind came from a specific signal is `Supported`; a
/// claim that fell through to `heuristic:default` is a recorded guess, so `Inferred`.
///
/// No claim is `Direct`: the quote is exact, but the kind is a heuristic, and the quote's
/// exactness is enforced by [`Claim::verify`] rather than claimed by this strength.
fn support_for(basis: &str) -> EvidenceLevel {
    if basis == "heuristic:default" {
        EvidenceLevel::Inferred
    } else {
        EvidenceLevel::Supported
    }
}

/// **The classifier's whole rule, stated where a reader can check it.**
///
/// The signals are deliberately few and ordered. The first matching rule wins, and the basis names
/// **that signal** (`heuristic:<signal>`). A sentence that matches none is a [`ProseKind::Fact`]
/// with `heuristic:default`, which is a guess recorded as a guess.
fn classify(text: &str) -> (ProseKind, String) {
    let lower = text.trim_start().to_lowercase();
    let rules: [(&str, ProseKind); 7] = [
        ("prohibition", ProseKind::Prohibition),
        ("obligation", ProseKind::Obligation),
        ("permission", ProseKind::Permission),
        ("condition", ProseKind::Condition),
        ("procedure", ProseKind::Procedure),
        ("definition", ProseKind::Definition),
        ("principle", ProseKind::Principle),
    ];
    for (signal, kind) in rules {
        if matches(signal, &lower) {
            return (kind, format!("heuristic:{signal}") );
        }
    }
    (ProseKind::Fact, "heuristic:default".to_string())
}

fn matches(signal: &str, lower: &str) -> bool {
    match signal {
        "prohibition" => {
            lower.starts_with("do not ")
                || lower.starts_with("must not ")
                || lower.contains(" never ")
                || lower.starts_with("never ")
        }
        "obligation" => {
            lower.starts_with("must ")
                || lower.contains(" must ")
                || lower.starts_with("shall ")
                || lower.contains(" shall ")
                || lower.contains(" is required")
        }
        "permission" => {
            lower.starts_with("may ")
                || lower.contains(" may ")
                || lower.contains(" is permitted")
                || lower.contains(" is allowed")
        }
        "condition" => {
            lower.starts_with("if ")
                || lower.starts_with("when ")
                || lower.starts_with("unless ")
                || lower.starts_with("where ")
        }
        "procedure" => {
            lower.starts_with("first")
                || lower.starts_with("second")
                || lower.starts_with("then ")
                || lower.starts_with("finally")
                || lower.starts_with("step ")
                || lower.contains("\n1.")
                || lower.contains("\n- ")
        }
        "definition" => {
            lower.contains(" is defined as")
                || lower.contains(" means ")
                || lower.contains(" refers to ")
        }
        "principle" => lower.contains(" principle") || lower.contains(" rule:") || lower.starts_with("rule "),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(text: &str) -> Unit {
        Unit::tile_text("artifact_test", text)
            .expect("the fixture is one exact tiling")
            .into_iter()
            .next()
            .expect("a paragraph")
    }

    #[test]
    fn a_claim_quotes_its_unit_exactly_and_verifies() {
        let unit = unit("The gate must refuse a missing witness.");
        let claim = Claim::derive(&unit);
        assert_eq!(claim.quote, unit.text);
        assert_eq!(claim.support(), EvidenceLevel::Supported);
        assert_eq!(claim.origin, EvidenceOrigin::Derived);
        assert!(claim.verify(&unit).is_ok(), "{claim:?}");
    }

    #[test]
    fn a_one_character_change_to_the_quote_is_refused_by_claim_id() {
        let unit = unit("The gate must refuse a missing witness.");
        let mut claim = Claim::derive(&unit);
        claim.quote = claim.quote.replace("gate", "Gate");
        let refused = claim.verify(&unit).expect_err("the quote moved");
        assert!(refused.detail().contains(&claim.id), "the id is in the refusal");
        assert!(refused.detail().contains("diverges"));
    }

    #[test]
    fn a_kind_with_no_basis_is_refused() {
        let unit = unit("The gate must refuse a missing witness.");
        let mut claim = Claim::derive(&unit);
        claim.kind_basis.clear();
        let refused = claim.verify(&unit).expect_err("an empty basis is a guess with no record");
        assert!(refused.detail().contains("kind_basis"));
    }

    #[test]
    fn the_kind_is_a_label_with_a_recorded_rule_and_a_default_is_named() {
        let obligation = Claim::derive(&unit("The gate must refuse a missing witness."));
        assert_eq!(obligation.kind, ProseKind::Obligation);
        assert_eq!(obligation.kind_basis, "heuristic:obligation");
        assert_eq!(obligation.support(), EvidenceLevel::Supported);

        let fact = Claim::derive(&unit("The gate refuses a missing witness."));
        assert_eq!(fact.kind, ProseKind::Fact);
        assert_eq!(fact.kind_basis, "heuristic:default");
        assert_eq!(fact.support(), EvidenceLevel::Inferred);
    }

    #[test]
    fn a_support_that_was_not_computed_from_the_basis_is_refused() {
        let unit = unit("The gate must refuse a missing witness.");
        let mut claim = Claim::derive(&unit);
        claim.support = EvidenceLevel::Direct;
        let refused = claim.verify(&unit).expect_err("a support nobody computed");
        assert!(refused.detail().contains(&claim.id));
        assert!(refused.detail().contains("support"));
        assert!(refused.detail().contains("heuristic:obligation"));
    }

    #[test]
    fn a_claim_that_quotes_a_different_unit_is_refused_by_both_ids() {
        let first = unit("The gate must refuse a missing witness.");
        let second = Unit::tile_text("artifact_test", "A second paragraph.").unwrap().remove(0);
        let claim = Claim::derive(&first);
        let refused = claim.verify(&second).expect_err("the units differ");
        assert!(refused.detail().contains(&claim.id));
        assert!(refused.detail().contains(&second.id));
    }
}
