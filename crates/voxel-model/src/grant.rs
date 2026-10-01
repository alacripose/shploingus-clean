//! **A grant: who authorised what, bounded.** One type, in the crate both can see.
//!
//! # Why this lives here and not in `voxel-package`
//!
//! It lived in `voxel_package::policy` as `Grant { granter, scope }`, refusing construction
//! on either being empty. That was correct and it was **in the wrong crate**:
//!
//! ```text
//! voxel-package  ->  voxel-shapers
//! ```
//!
//! So `voxel-shapers` — which owns [`crate`]'s stage set, the only authority-bearing object in
//! the world model, and the only thing in the repository that changes the world — **could not
//! see the type that says who authorised it.** The first attempt at closing that gap added an
//! `Authority { granter, scope, anchor }` enum to the stage set; that is a second answer to a
//! question this one already answers, with a third field no reference names, and it was
//! reverted.
//!
//! So the type moved down instead of up. **One grant, one refusal, one name** — and D-07's
//! *"two names for one meaning"* is what made the move necessary rather than tidy.
//!
//! # What a grant is not
//!
//! **It is not a signature.** `voxel_package::signing` states why: a digest establishes
//! integrity, never identity, and establishing identity needs asymmetric cryptography this
//! repository does not have. `granter` is **a name, never key material** — the same rule
//! `policy::Grant` carried, kept because it is the rule and not the location that matters.
//!
//! # The refusals, unchanged and for the same reasons
//!
//! * an empty granter — *"an authorization with no author is not an authorization"*;
//! * an empty scope — *"it authorizes everything, which is the same failure as authorizing
//!   nothing"*.
//!
//! Both are [`RefusalKind::FabricatedFeature`], as they were, because a caller filtering by
//! kind must get the same answer whichever crate it asked.

use crate::refusal::{Refusal, RefusalKind, RefusalSet};

/// Who authorised a bounded thing.
///
/// **Two fields, both required, both refused rather than defaulted.** A grant with no granter
/// is an authorization with no author; a grant with no scope authorizes everything, which is
/// the same failure as authorizing nothing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Grant {
    granter: String,
    scope: String,
}

impl Grant {
    /// Build a grant, or refuse.
    ///
    /// Refused, never defaulted: an empty granter and an empty scope are both
    /// plausible-looking values, and a plausible value is indistinguishable from a correct one
    /// until much later. **All refusals are collected**, so a caller fixes one round of them
    /// rather than one at a time.
    pub fn new(granter: impl Into<String>, scope: impl Into<String>) -> Result<Self, RefusalSet> {
        let granter = granter.into();
        let scope = scope.into();
        let mut refusals = RefusalSet::new();

        if granter.trim().is_empty() {
            refusals.push(Refusal::owned(
                RefusalKind::FabricatedFeature,
                "a grant with no granter is refused construction: an authorization with no \
                 author is not an authorization"
                    .to_owned(),
            ));
        }
        if scope.trim().is_empty() {
            refusals.push(Refusal::owned(
                RefusalKind::FabricatedFeature,
                "a grant with no scope is refused construction: it authorizes everything, which \
                 is the same failure as authorizing nothing"
                    .to_owned(),
            ));
        }

        refusals.into_result(Self { granter, scope })
    }

    /// Who authorised it. A **name**, never key material.
    pub fn granter(&self) -> &str {
        &self.granter
    }

    /// What it authorises, bounded. **A scope is a bound, not a description** — the refusal
    /// above is the reason an empty one cannot be defaulted into existence.
    pub fn scope(&self) -> &str {
        &self.scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grant_with_no_granter_is_refused_construction() {
        let refused = Grant::new("  ", "a world").expect_err("no author is not an authorization");
        assert_eq!(refused.len(), 1);
        assert_eq!(refused.all()[0].kind(), RefusalKind::FabricatedFeature);
        assert!(refused.all()[0].detail().contains("no author"));
    }

    #[test]
    fn a_grant_with_no_scope_is_refused_because_it_authorizes_everything() {
        let refused = Grant::new("player-1", "").expect_err("an unbounded grant authorizes all");
        assert!(refused.all()[0].detail().contains("authorizes everything"));
    }

    #[test]
    fn both_refusals_are_collected_rather_than_the_first_returned() {
        // **One round of fixes, not two.** `RefusalSet` is the reason this is a set and not an
        // `Option<Refusal>`, and a caller that has to call `new` twice to discover the second
        // empty field is a caller that will not.
        let refused = Grant::new("", "").expect_err("two empty fields");
        assert_eq!(refused.len(), 2, "both are reported at once: {refused}");
    }

    #[test]
    fn a_grant_carries_its_name_and_its_bound_and_nothing_else() {
        let grant = Grant::new("player-1", "world-2468 alpha").expect("a complete grant");
        assert_eq!(grant.granter(), "player-1");
        assert_eq!(grant.scope(), "world-2468 alpha");
    }
}
