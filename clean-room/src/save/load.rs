//! **The load — is this save of the world the caller is loading, and is its record the record it
//! declares?**
//!
//! [`Save::load`] is the only way a world comes back, and it is built out of *correspondence* checks
//! rather than integrity ones (integrity is the layout's, in [`super::layout`]): the ruleset the save
//! was written under against the one this load declares, the record it carries against the head and
//! length it declares, the declared period against U2's own rule, and every declared stream's seed
//! against the world's own. [`Loaded`] exists only when all of them passed, so there is no partially
//! loaded world for a caller to trip over.
//!
//! **The expected ruleset is the caller's, deliberately.** A save stays portable across mod changes
//! *because* each record names the identity it was made under (`docs/GAIA.md` §5c/§11), so a load must
//! be told which identity it is loading into — that comparison *is* the check, and defaulting it would
//! silently accept a save from another ruleset. The reference world's own ruleset and seed are declared
//! once, in [`crate::reference`], which is where the binary, its evidence and its tests read them.

use crate::chain::Chain;
use crate::digest::Digest;
use crate::refusal::{RefusalKind, RefusalSet};
use crate::world::{DeclaredPeriod, Domain, Tick, World, WorldIdentity};

use super::{Ruleset, Save, SaveIdentity, StreamSeed};

/// What a load reproduced, and the numbers it was checked with. `truth` is **derived** at load time
/// (never read from the save), so printing it beside the live world's digest is a comparison of two
/// derivations rather than of a claim with itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadReport {
    pub identity: SaveIdentity,
    pub tick: Tick,
    pub truth: Digest,
    pub record_len: usize,
    pub record_head: Option<Digest>,
    pub streams: Vec<StreamSeed>,
}

/// The result of a load that passed every check: the world rebuilt from the identity, the record it
/// carried, and the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loaded {
    pub world: World,
    pub chain: Chain,
    pub report: LoadReport,
}

impl Save {
    /// **Read a world back.** Every check below is a question about correspondence rather than about
    /// integrity: is this save of the world the caller is loading, and is its record the record it
    /// declares? `Loaded` is produced only when all of them pass.
    pub fn load(&self, expected: Ruleset) -> Result<Loaded, RefusalSet> {
        let mut set = RefusalSet::new();

        if self.ruleset != expected {
            set.refuse(
                RefusalKind::IdentityMismatch,
                format!(
                    "the save was written under ruleset {} revision {} and this load declares \
                     ruleset {} revision {}",
                    self.ruleset.ruleset, self.ruleset.revision, expected.ruleset, expected.revision
                ),
            );
        }

        let chain = match Chain::from_wire(&self.record_wire) {
            Ok(chain) => Some(chain),
            Err(refusals) => {
                set.extend(refusals);
                None
            }
        };
        if let Some(chain) = &chain {
            if chain.head() != self.record_head || chain.len() as u64 != self.record_len {
                set.refuse(
                    RefusalKind::RecordDiverged,
                    format!(
                        "the save declares a record of {} event(s) with head {}, and it carries {} \
                         event(s) with head {}",
                        self.record_len,
                        hex_or_none(self.record_head),
                        chain.len(),
                        hex_or_none(chain.head())
                    ),
                );
            }
        }

        let period = match DeclaredPeriod::seconds(self.identity.period) {
            Ok(period) => Some(period),
            Err(kind) => {
                set.refuse(
                    kind,
                    format!(
                        "the save's declared period is {} second(s)",
                        self.identity.period
                    ),
                );
                None
            }
        };

        let world = period.map(|period| {
            World::new(WorldIdentity::new(self.identity.seed, period))
                .with_tick(Tick(self.identity.tick))
        });

        if let Some(world) = &world {
            for domain in Domain::ALL {
                match self.streams.iter().find(|stream| stream.domain == domain) {
                    None => set.refuse(
                        RefusalKind::SeedUndeclared,
                        format!("the save declares no seed for the {} stream", domain.name()),
                    ),
                    Some(declared) => {
                        let own = world.stream_seed(domain);
                        if declared.seed != own {
                            set.refuse(
                                RefusalKind::IdentityMismatch,
                                format!(
                                    "the save's {} stream seed is {} and this world's is {own}",
                                    domain.name(),
                                    declared.seed
                                ),
                            );
                        }
                    }
                }
            }
        }

        let (Some(world), Some(chain)) = (world, chain) else {
            if set.is_empty() {
                set.refuse(
                    RefusalKind::NotASave,
                    "the save's identity or record could not be rebuilt",
                );
            }
            return Err(set);
        };

        let report = LoadReport {
            identity: self.identity,
            tick: world.tick(),
            truth: world.truth_digest(),
            record_len: chain.len(),
            record_head: chain.head(),
            streams: self.streams.clone(),
        };
        set.into_result(Loaded {
            world,
            chain,
            report,
        })
    }
}

fn hex_or_none(digest: Option<Digest>) -> String {
    digest.map(|digest| digest.to_hex()).unwrap_or_else(|| "-".to_string())
}
