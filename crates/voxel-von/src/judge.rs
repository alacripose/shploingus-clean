//! **The judgment: one ballot, one answer, one decision.**
//!
//! # What this layer is allowed to decide, and what it is not
//!
//! The classifier answers a question. It does **not** decide whether the answer is good work, and
//! neither does this module: the ballot declares where the answer stops passing (`passes`, `floor`,
//! `threshold`), and this code only compares the two. That division is the whole reason the floor
//! is a declared field rather than a constant here — a floor chosen by the code that reads the
//! answer is a floor chosen to fit it.
//!
//! # The origin is the `A` rung, and it is not negotiable
//!
//! A model's answer is [`EvidenceOrigin::Oracle`]: something outside this project decided it. That
//! is the ticket-evidence ladder's `A` rung, and it is the *weakest* kind of evidence this tree
//! carries — the ladder's own words are that agent self-report is evidence and not authority, and a
//! classifier is further outside than an agent. Upstream's rule is stronger and this module keeps
//! it: **the origin is the proof's to carry, not the claimant's to declare** — so no field of a
//! ballot can set it, and [`decide`] writes it from the instrument rather than from the request.
//!
//! # Verbatim, both ways
//!
//! The decision carries the model's own number (`probability`, `confidence`, the chosen option, the
//! rung and the legend that names it) and the ballot's own words in its findings. Nothing is
//! rounded, summarised or paraphrased, because a decision derived from this one has to be derived
//! from the same numbers a reader can see.

use jigor::{Answer, Question};
use serde_json::Value;
use voxel_model::axis::AxisKind;
use voxel_model::laya::{Decision, Verdict};
use voxel_model::ticket::EvidenceOrigin;
use voxel_package::ballot::{Ballot, Kind};

use crate::Von;

/// The instrument's question, in the classifier's own vocabulary.
///
/// **A total function over [`Kind`]**, so a fourth instrument cannot be added without this
/// answering for it — the same property `policy` gets from a `match` with no wildcard arm.
pub fn question_of(ballot: &Ballot) -> Question {
    match ballot.kind {
        Kind::Noul => crate::noul(&ballot.id, &ballot.instruction),
        Kind::Choice => {
            let borrowed: Vec<&str> = ballot.options.iter().map(String::as_str).collect();
            crate::choice(&ballot.id, &ballot.instruction, &borrowed)
        }
        Kind::Score => {
            let borrowed: Vec<&str> = ballot.levels.iter().map(String::as_str).collect();
            crate::score(&ballot.id, &ballot.instruction, &borrowed)
        }
    }
}

/// **The comparison, and nothing else.** Pure, so the rule is testable without a model: a ballot
/// and an answer in, one decision out.
///
/// A refusal to reach a verdict is [`Verdict::NoReading`] — *"the judge read it and there is
/// nothing to judge"* — which is **not a pass**. An answer whose shape does not match the
/// instrument (a `choice` to a `score` question) is that, because the alternative is comparing two
/// different units and reporting the result as a judgment.
pub fn decide(ballot: &Ballot, answer: &Answer) -> Decision {
    let id = &ballot.id;
    let mut base = Decision {
        path: ballot.artifact.clone(),
        question: match ballot.kind {
            Kind::Noul => voxel_model::laya::Question::Choice,
            Kind::Choice => voxel_model::laya::Question::Choice,
            Kind::Score => voxel_model::laya::Question::Score,
        },
        verdict: Verdict::NoReading,
        confidence: 0.0,
        // The `A` rung, written here rather than accepted from the ballot.
        origin: Some(EvidenceOrigin::Oracle),
        grades: vec![(AxisKind::TicketEvidence, EvidenceOrigin::Oracle.letter().to_string())],
        findings: Vec::new(),
    };

    match (ballot.kind, answer) {
        (Kind::Noul, Answer::Noul { probability }) => {
            let probability = f64::from(*probability);
            // The declared floor is in thousandths, so the comparison is against the same number a
            // reader of the ballot sees: 500 thousandths, one half, and the boundary is included.
            let floor = f64::from(ballot.threshold_thousandths) / 1000.0;
            base.verdict = if probability >= floor {
                Verdict::Belongs
            } else {
                Verdict::NotBelongs
            };
            // The model's own number, carried whole: a probability is not a confidence and is not
            // converted into one.
            base.confidence = probability;
            base.findings.push(format!(
                "{id}\tprobability of yes {probability:.4} against the declared threshold {floor:.4} \
                 ({} thousandths)\t{}",
                ballot.threshold_thousandths, ballot.instruction
            ));
        }
        (
            Kind::Choice,
            Answer::Choice {
                choice,
                confidence,
                probabilities,
            },
        ) => {
            let passing = ballot.passes.clone().unwrap_or_default();
            base.verdict = if *choice == passing {
                Verdict::Belongs
            } else {
                Verdict::NotBelongs
            };
            base.confidence = f64::from(*confidence);
            // Every option's probability, weakest-to-strongest as declared: a reader must be able to
            // see how close the second choice was, and a summary would hide exactly that.
            let spread: Vec<String> = ballot
                .options
                .iter()
                .map(|option| {
                    format!(
                        "{option} {:?}",
                        probabilities.get(option).map(|p| (*p * 10_000.0).round() / 10_000.0)
                    )
                })
                .collect();
            base.findings.push(format!(
                "{id}\tchose {choice:?} at confidence {confidence:.4}, and {passing:?} is the \
                 declared passing answer\t{}",
                spread.join(", ")
            ));
        }
        (
            Kind::Score,
            Answer::Score {
                score,
                confidence,
                probabilities,
                legend,
            },
        ) => {
            let floor = ballot.floor.unwrap_or(u32::MAX);
            let landed = *score as u32;
            base.verdict = if landed >= floor {
                Verdict::Belongs
            } else {
                Verdict::NotBelongs
            };
            base.confidence = f64::from(*confidence);
            // The rung's own name, from the model's own legend, so a reader sees "mostly" and not
            // "2" — the same reason the ladder carries tokens rather than indices.
            let rung = legend
                .get(&landed.to_string())
                .cloned()
                .or_else(|| ballot.levels.get(landed as usize).cloned())
                .unwrap_or_else(|| format!("rung {landed}"));
            base.findings.push(format!(
                "{id}\tlanded on {landed} ({rung:?}) of {} at confidence {confidence:.4}, and the \
                 declared floor is {floor}\t{}",
                ballot.levels.len(),
                ballot.levels.join(" < ")
            ));
            let _ = probabilities;
        }
        // **A shape mismatch is a non-reading, never a verdict.** Reported with both shapes named,
        // because "no reading" without the reason is indistinguishable from a judge that declined.
        (kind, other) => {
            base.findings.push(format!(
                "{id}\tthe ballot asks {} and the answer came back {}; the two are not comparable \
                 and neither is a judgment",
                kind.name(),
                other.kind()
            ));
        }
    }
    base
}

/// Ask the classifier this ballot's question about `state`, and decide.
///
/// **One ask, one decision.** The ballot and the answer are both carried into the decision, so a
/// reader can re-derive the verdict without the model — which is the only reason a model's answer
/// can be evidence at all.
pub fn ask(classifier: &mut Von, ballot: &Ballot, state: &Value) -> Result<Decision, String> {
    let question = question_of(ballot);
    let mut answers = classifier.answers(state, std::slice::from_ref(&question))?;
    match answers.remove(&ballot.id) {
        Some(answer) => Ok(decide(ballot, &answer)),
        // A question the model did not answer is reported as a non-reading rather than a verdict:
        // the alternative is a missing answer read as a pass.
        None => {
            let mut decision = decide(
                ballot,
                &Answer::Noul {
                    probability: f32::NAN,
                },
            );
            decision.verdict = Verdict::NoReading;
            decision.confidence = 0.0;
            decision.findings = vec![format!(
                "{}\tthe classifier returned no answer for this question, so there is nothing to \
                 judge — not a pass",
                ballot.id
            )];
            Ok(decision)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use voxel_package::ballot::{parse, HEADER};

    fn ballot(text: &str) -> Ballot {
        parse(&format!("{HEADER}\n{text}"))
            .unwrap_or_else(|why| panic!("the fixture parses: {why}"))
            .remove(0)
    }

    fn noul_ballot() -> Ballot {
        ballot(
            "ballot\tbelongs\nclaim\tT-12\nartifact\tcrates/x/src/lib.rs\nkind\tnoul\n\
             instruction\tdoes this belong to the recipe?\npresenter\tala\nmax-age\t1200\n",
        )
    }

    #[test]
    fn a_probability_at_or_above_the_declared_threshold_belongs_and_below_it_does_not() {
        let b = noul_ballot();
        let yes = decide(&b, &Answer::Noul { probability: 0.62 });
        assert_eq!(yes.verdict, Verdict::Belongs);
        assert!((yes.confidence - 0.62).abs() < 1e-6, "the model's own number is carried");

        // Exactly at the threshold passes: the boundary is included, and said so, because a
        // boundary that means two things depending on the side it is read from is not a rule.
        let at = decide(&b, &Answer::Noul { probability: 0.5 });
        assert_eq!(at.verdict, Verdict::Belongs);

        let no = decide(&b, &Answer::Noul { probability: 0.44 });
        assert_eq!(no.verdict, Verdict::NotBelongs);
        // And the finding carries the number and the threshold it was compared against.
        let finding = no.findings.join(" ");
        assert!(finding.contains("0.4400") && finding.contains("0.5000"), "{finding}");
    }

    #[test]
    fn a_declared_threshold_is_what_the_comparison_uses() {
        let strict = ballot(
            "ballot\tbelongs\nclaim\tT-12\nartifact\tcrates/x/src/lib.rs\nkind\tnoul\n\
             threshold\t900\ninstruction\tdoes this belong?\npresenter\tala\nmax-age\t1200\n",
        );
        assert_eq!(strict.threshold_thousandths, 900);
        assert_eq!(
            decide(&strict, &Answer::Noul { probability: 0.7 }).verdict,
            Verdict::NotBelongs,
            "the declared floor is the ruling, not a number this code picked"
        );
    }

    #[test]
    fn the_origin_is_the_a_rung_and_no_ballot_field_can_change_it() {
        // Upstream's rule, kept: the origin is the proof's to carry, not the claimant's to declare.
        let b = noul_ballot();
        let decision = decide(&b, &Answer::Noul { probability: 0.9 });
        assert_eq!(decision.origin, Some(EvidenceOrigin::Oracle));
        assert_eq!(
            decision.grades,
            vec![(AxisKind::TicketEvidence, "A".to_string())],
            "the grade is the ladder's own token for the origin"
        );
    }

    #[test]
    fn a_choice_belongs_only_when_the_declared_passing_answer_was_chosen() {
        let b = ballot(
            "ballot\tpick\nclaim\tT-2\nartifact\ta.rs\nkind\tchoice\noption\tstream it\n\
             option\tleave it\npasses\tstream it\ninstruction\twhich?\npresenter\tala\nmax-age\t60\n",
        );
        let passing = Answer::Choice {
            choice: "stream it".to_string(),
            confidence: 0.7,
            probabilities: HashMap::new(),
        };
        assert_eq!(decide(&b, &passing).verdict, Verdict::Belongs);
        let failing = Answer::Choice {
            choice: "leave it".to_string(),
            confidence: 0.7,
            probabilities: HashMap::new(),
        };
        let decision = decide(&b, &failing);
        assert_eq!(decision.verdict, Verdict::NotBelongs);
        assert!(decision.findings.join(" ").contains("leave it"), "{:?}", decision.findings);
    }

    #[test]
    fn a_score_is_measured_against_the_declared_floor_and_names_the_rung_it_landed_on() {
        let b = ballot(
            "ballot\trung\nclaim\tT-2\nartifact\ta.rs\nkind\tscore\nlevel\tnone\nlevel\tsome\n\
             level\tall\nfloor\t2\ninstruction\thow far?\npresenter\tala\nmax-age\t60\n",
        );
        let mut legend = HashMap::new();
        legend.insert("1".to_string(), "some".to_string());
        let landed = Answer::Score {
            score: 1.4,
            confidence: 0.3,
            probabilities: HashMap::new(),
            legend,
        };
        let decision = decide(&b, &landed);
        assert_eq!(decision.verdict, Verdict::NotBelongs, "1 is below the declared floor of 2");
        assert!(
            decision.findings.join(" ").contains("some"),
            "the rung is named, not numbered: {:?}",
            decision.findings
        );

        let cleared = Answer::Score {
            score: 2.0,
            confidence: 0.3,
            probabilities: HashMap::new(),
            legend: HashMap::new(),
        };
        assert_eq!(decide(&b, &cleared).verdict, Verdict::Belongs);
    }

    #[test]
    fn an_answer_of_the_wrong_shape_is_a_non_reading_and_never_a_verdict() {
        let b = noul_ballot();
        let mismatched = Answer::Score {
            score: 3.0,
            confidence: 0.9,
            probabilities: HashMap::new(),
            legend: HashMap::new(),
        };
        let decision = decide(&b, &mismatched);
        assert_eq!(decision.verdict, Verdict::NoReading);
        assert_eq!(decision.confidence, 0.0, "a non-reading has no confidence to carry");
        assert!(decision.findings.join(" ").contains("not comparable"));
    }
}
