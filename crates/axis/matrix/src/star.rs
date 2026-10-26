//! **The five stars, and the predicate that has to have run.**
//!
//! The verification vocabulary the user names is *"the 5 stars and the bronze-platinum tiers"*, and
//! the tree already states the rule that makes a star mean something. `voxel-package/src/bridge.rs`
//! quotes it, from the sibling's `POLICY-CIVILIANS.md`:
//!
//! > *"predicates are rows, never a score … a star lit by a predicate that did not run is the defect
//! > the five-star rule forbids."*
//!
//! So a star here is **not** a count of things that look done. It is the number of [`Predicate`] rows
//! that **ran and passed**, and [`require`] refuses a claim of four stars backed by three rows — the
//! message naming the row that did not run, because "three of four" sends a reader to the wrong
//! place.
//!
//! # `passed` and `ran` are two facts, and the pair is checked
//!
//! A predicate that reports `passed: true, ran: false` is not a weak pass; it is the defect itself,
//! and [`Predicate::check`] refuses it **on its own**, before any total is computed. A total that
//! quietly skipped it would make the defect arithmetically invisible, which is how a score comes to
//! replace a row.
//!
//! # There is no zero star
//!
//! [`Star::of_ordinal`] takes `u8` and returns `Option<Star>`; `0` is `None`. Absence of a reading is
//! a **state** — the tree's §17 rule 4, *"absence of certification is a state, never a floor"* — and
//! manufacturing a `★0` would be the same error as reading an unobserved metric as `0`.

use crate::MatrixRefusal;

/// One star, ★1 to ★5. **Five members, and there is no sixth**: a verification scale that grows is a
/// score, and a score is what the five-star rule replaced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Star {
    /// One predicate ran and passed.
    One,
    /// Two.
    Two,
    /// Three.
    Three,
    /// Four.
    Four,
    /// The whole vector: every declared predicate ran and passed.
    Five,
}

/// How many stars the scale has. Pinned by [`STARS_ARE_TOTAL`].
pub const STAR_COUNT: usize = 5;

impl Star {
    /// Every star, weakest first.
    pub const ALL: [Star; STAR_COUNT] = [Star::One, Star::Two, Star::Three, Star::Four, Star::Five];

    /// The ordinal, `1..=5`. **Explicit, never inferred from the way a star is drawn**, because a
    /// glyph is a rendering choice and a rank is a fact.
    pub const fn ordinal(self) -> u8 {
        match self {
            Star::One => 1,
            Star::Two => 2,
            Star::Three => 3,
            Star::Four => 4,
            Star::Five => 5,
        }
    }

    /// The star for an ordinal, or `None` when the ordinal is not a star.
    pub const fn of_ordinal(ordinal: u8) -> Option<Star> {
        match ordinal {
            1 => Some(Star::One),
            2 => Some(Star::Two),
            3 => Some(Star::Three),
            4 => Some(Star::Four),
            5 => Some(Star::Five),
            _ => None,
        }
    }

    /// The recorded name, `star_1` … `star_5`. The glyphs are for the pane; this is for the record.
    pub const fn name(self) -> &'static str {
        match self {
            Star::One => "star_1",
            Star::Two => "star_2",
            Star::Three => "star_3",
            Star::Four => "star_4",
            Star::Five => "star_5",
        }
    }

    /// Five slots, lit ones filled. **A cell with three stars draws three filled and two empty**, so a
    /// reader can see how much of the vector ran without reading a number.
    pub fn glyphs(self) -> String {
        let lit = self.ordinal() as usize;
        let mut text = String::with_capacity(STAR_COUNT * 3);
        for slot in 1..=STAR_COUNT {
            text.push(if slot <= lit { '★' } else { '☆' });
        }
        text
    }

    /// Parse a recorded name, or its glyph run (`★★★☆☆`), or a bare ordinal (`3`).
    ///
    /// Three spellings and no fourth: the record's name, the pane's drawing, and the number a ticket
    /// already carries. Anything else is refused rather than guessed at.
    pub fn parse(word: &str) -> Result<Self, MatrixRefusal> {
        let word = word.trim();
        if let Ok(ordinal) = word.parse::<u8>() {
            if let Some(star) = Self::of_ordinal(ordinal) {
                return Ok(star);
            }
        }
        if let Some(star) = Self::ALL
            .into_iter()
            .find(|star| star.name() == word || star.glyphs() == word)
        {
            return Ok(star);
        }
        Err(MatrixRefusal::UnknownStar {
            word: word.to_string(),
        })
    }
}

impl std::fmt::Display for Star {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.glyphs())
    }
}

/// The table is as wide as the scale. **Compile-checked**, like the segment pin.
pub const STARS_ARE_TOTAL: [(); STAR_COUNT] = [(); Star::ALL.len()];

/// The rule's own sentence, quoted once so a refusal and a doc comment cannot drift apart.
pub const FIVE_STAR_RULE: &str = "predicates are rows, never a score: a star lit by a predicate that \
                                   did not run is the defect the five-star rule forbids";

/// One row of a cell's verification. **A named thing that either ran or did not, and either passed or
/// did not** — `ran` is not implied by `passed`, which is the whole point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Predicate {
    /// Its recorded name, e.g. `warnings_fell`. Lowercase snake case, like every other token.
    pub name: &'static str,
    /// Whether it executed. **`false` with `passed: true` is refused**, not discounted.
    pub ran: bool,
    /// Whether it held, when it ran. Meaningless when it did not, which is why
    /// [`Predicate::check`] refuses that combination instead of reading it.
    pub passed: bool,
}

impl Predicate {
    /// A predicate that ran and held.
    pub const fn held(name: &'static str) -> Self {
        Predicate {
            name,
            ran: true,
            passed: true,
        }
    }

    /// A predicate that ran and failed.
    pub const fn failed(name: &'static str) -> Self {
        Predicate {
            name,
            ran: true,
            passed: false,
        }
    }

    /// A predicate that never ran. **Neither a pass nor a failure**, and it lights no star.
    pub const fn not_run(name: &'static str) -> Self {
        Predicate {
            name,
            ran: false,
            passed: false,
        }
    }

    /// Whether this row lights a star: it ran **and** it passed.
    pub const fn lit(&self) -> bool {
        self.ran && self.passed
    }

    /// Refuse the defect itself: `passed` on a predicate that never ran.
    pub fn check(&self) -> Result<(), MatrixRefusal> {
        if self.passed && !self.ran {
            return Err(MatrixRefusal::PredicateDidNotRun {
                predicate: self.name,
            });
        }
        Ok(())
    }
}

/// How many stars a set of predicates lights, or `None` when none does.
///
/// `None` rather than [`Star::One`]: a cell whose predicates all failed was *measured and unlit*, and
/// a measurement that reads as one star is a measurement that reads as a pass.
pub fn lit_star(predicates: &[Predicate]) -> Option<Star> {
    let lit = predicates.iter().filter(|predicate| predicate.lit()).count();
    let capped = lit.min(STAR_COUNT);
    Star::of_ordinal(capped as u8)
}

/// **The five-star rule as a check**: a claimed star must be lit by that many rows that ran and
/// passed, and any row claiming a pass it did not run is refused first.
///
/// The detail names every row that is not lit — did not run, or ran and failed — because the reader's
/// next act is to look at one of them.
pub fn require(star: Star, predicates: &[Predicate]) -> Result<(), MatrixRefusal> {
    for predicate in predicates {
        predicate.check()?;
    }
    let lit = predicates.iter().filter(|predicate| predicate.lit()).count();
    if lit >= star.ordinal() as usize {
        return Ok(());
    }
    let missing: Vec<String> = predicates
        .iter()
        .filter(|predicate| !predicate.lit())
        .map(|predicate| {
            if predicate.ran {
                format!("{} failed", predicate.name)
            } else {
                format!("{} never ran", predicate.name)
            }
        })
        .collect();
    Err(MatrixRefusal::StarUnsupported {
        claimed: star.name(),
        detail: if missing.is_empty() {
            format!(
                "the cell claims {} and {} predicates ran and passed — {FIVE_STAR_RULE}",
                star.glyphs(),
                lit
            )
        } else {
            format!(
                "the cell claims {} and {} predicates ran and passed, but {} — {FIVE_STAR_RULE}",
                star.glyphs(),
                lit,
                missing.join(", ")
            )
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_has_five_members_and_no_zero() {
        assert_eq!(Star::ALL.len(), STAR_COUNT);
        assert_eq!(STARS_ARE_TOTAL.len(), STAR_COUNT);
        assert!(Star::of_ordinal(0).is_none(), "zero is a state, not a star");
        assert!(Star::of_ordinal(6).is_none());
        for star in Star::ALL {
            assert_eq!(Star::of_ordinal(star.ordinal()), Some(star));
        }
    }

    #[test]
    fn the_glyphs_draw_all_five_slots() {
        assert_eq!(Star::Three.glyphs(), "★★★☆☆");
        assert_eq!(Star::Five.glyphs(), "★★★★★");
        assert_eq!(Star::One.glyphs(), "★☆☆☆☆");
        for star in Star::ALL {
            assert_eq!(star.glyphs().chars().count(), STAR_COUNT);
        }
    }

    #[test]
    fn the_name_the_glyphs_and_the_ordinal_all_parse() {
        assert_eq!(Star::parse("star_4").unwrap(), Star::Four);
        assert_eq!(Star::parse("★★★☆☆").unwrap(), Star::Three);
        assert_eq!(Star::parse("2").unwrap(), Star::Two);
        assert_eq!(Star::parse(" 5 \n").unwrap(), Star::Five);
        let refused = Star::parse("six").expect_err("six is not a star");
        assert!(refused.detail().contains("\"six\""), "got {refused}");
        assert!(Star::parse("0").is_err(), "zero does not parse as a star");
    }

    #[test]
    fn a_star_is_the_count_of_rows_that_ran_and_passed() {
        let predicates = [
            Predicate::held("builds"),
            Predicate::held("warnings_fell"),
            Predicate::failed("no_new_ticket"),
            Predicate::not_run("witness_pinned"),
        ];
        assert_eq!(lit_star(&predicates), Some(Star::Two));
        require(Star::Two, &predicates).expect("two stars are lit by two rows");
        let refused = require(Star::Three, &predicates).expect_err("only two rows are lit");
        assert!(refused.detail().contains("no_new_ticket failed"), "got {refused}");
        assert!(refused.detail().contains("witness_pinned never ran"), "got {refused}");
        assert!(refused.detail().contains("five-star rule"), "got {refused}");
    }

    #[test]
    fn a_row_that_passed_without_running_is_refused_before_any_total() {
        let fraudulent = Predicate {
            name: "gate_cleared",
            ran: false,
            passed: true,
        };
        let refused = fraudulent.check().expect_err("a pass it did not run is the defect");
        assert!(refused.detail().contains("gate_cleared"), "got {refused}");
        let set = [fraudulent, Predicate::held("builds")];
        let refused = require(Star::One, &set).expect_err("the defect is refused even when a star is lit");
        assert!(refused.detail().contains("never ran"), "got {refused}");
    }

    #[test]
    fn a_cell_measured_and_unlit_has_no_star_rather_than_one() {
        let predicates = [Predicate::failed("builds"), Predicate::not_run("gate")];
        assert_eq!(lit_star(&predicates), None);
        let refused = require(Star::One, &predicates).expect_err("nothing ran and passed");
        assert!(refused.detail().contains("0 predicates ran and passed"), "got {refused}");
        assert_eq!(lit_star(&[]), None, "no predicates is no measurement");
    }

    #[test]
    fn more_than_five_lit_rows_do_not_invent_a_sixth_star() {
        let predicates = [
            Predicate::held("a"),
            Predicate::held("b"),
            Predicate::held("c"),
            Predicate::held("d"),
            Predicate::held("e"),
            Predicate::held("f"),
        ];
        assert_eq!(lit_star(&predicates), Some(Star::Five));
        require(Star::Five, &predicates).expect("five stars are lit");
    }
}
