//! The memory ceiling, as a type that **refuses**.
//!
//! "Do not devour the machine's memory" is not a property a program has; it is a
//! number somebody checks. So the ceiling is a value, every allocation that grows
//! with the world is charged against it, and a charge that would cross it is a
//! refusal naming both numbers rather than a plausible success.
//!
//! # What is charged, and the one thing that dominates
//!
//! Measured, not assumed. The single largest allocation in this program is **one
//! [`Mesher`](voxel_mesh::Mesher)'s scratch space** — a `136³` [`MaterialId`]
//! raster, about **4.8 MB** — and a pool needs **one per worker**, because the
//! scratch is never shared (that is what lets worker count be a performance knob
//! and never a change in output, F12).
//!
//! So eight workers cost about **38 MB before a single chunk is held**, while the
//! entire mesh residency of the declared render horizon costs about **6 MB**
//! (`docs/STATUS.md`, and `voxel-stream`'s memory measurement). The pool is
//! therefore chosen from the budget rather than from the CPU count:
//! [`Budget::how_many_fit`] answers "how many workers fit", and the answer is the
//! worker count. A budget that did not charge the scratch would be a budget that
//! misses most of the memory.

use voxel_model::{Refusal, RefusalKind, Refused};

/// A memory ceiling in bytes, charged and released explicitly.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Budget {
    ceiling: usize,
    charged: usize,
}

impl Budget {
    /// Refuses a ceiling of zero: a budget that can hold nothing is not a
    /// ceiling, it is a program that cannot run, and saying so is better than
    /// refusing every allocation one at a time.
    pub fn new(ceiling: usize) -> Refused<Self> {
        if ceiling == 0 {
            return Err(Refusal::new(
                RefusalKind::OutOfDeclaredRange,
                "a budget of 0 bytes cannot hold anything, so it is not a ceiling",
            ));
        }
        Ok(Budget { ceiling, charged: 0 })
    }

    pub const fn ceiling(&self) -> usize {
        self.ceiling
    }

    pub const fn charged(&self) -> usize {
        self.charged
    }

    pub fn free(&self) -> usize {
        self.ceiling.saturating_sub(self.charged)
    }

    pub fn fits(&self, bytes: usize) -> bool {
        bytes <= self.free()
    }

    /// Charge `bytes`, refusing when the ceiling would be crossed.
    ///
    /// The refusal names the request, the total it would reach, the ceiling, and
    /// what is already charged — because "out of memory" that does not say *how
    /// much* is a message that cannot be acted on.
    pub fn charge(&mut self, bytes: usize) -> Refused<()> {
        let want = self.charged.saturating_add(bytes);
        if want > self.ceiling {
            return Err(Refusal::owned(
                RefusalKind::OutOfDeclaredRange,
                format!(
                    "charging {bytes} B would reach {want} B of a {} B ceiling ({}, {} B free)",
                    self.ceiling, self.charged, self.free()
                ),
            ));
        }
        self.charged = want;
        Ok(())
    }

    /// Give `bytes` back. Saturating, so a double release cannot underflow into a
    /// budget that believes it has room it does not have.
    pub fn release(&mut self, bytes: usize) {
        self.charged = self.charged.saturating_sub(bytes);
    }

    /// How full the budget is, `0.0..=1.0`. What a HUD shows.
    pub fn fill(&self) -> f64 {
        self.charged as f64 / self.ceiling as f64
    }

    /// How many allocations of `each` bytes fit, after holding `reserve` back.
    ///
    /// This is how the worker count is chosen: `how_many_fit(scratch_bytes,
    /// reserve)` is the number of threads whose scratch the machine can afford,
    /// and it is a measurement of the mesher rather than a guess about the CPU.
    pub fn how_many_fit(&self, each: usize, reserve: usize) -> usize {
        if each == 0 {
            return 0;
        }
        self.ceiling.saturating_sub(reserve).saturating_sub(self.charged) / each
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_mesh::Mesher;

    #[test]
    fn a_zero_ceiling_is_refused_rather_than_called_a_budget() {
        assert_eq!(
            Budget::new(0).unwrap_err().kind(),
            RefusalKind::OutOfDeclaredRange
        );
    }

    #[test]
    fn a_charge_that_crosses_the_ceiling_is_refused_and_names_both_numbers() {
        let mut b = Budget::new(1000).unwrap();
        b.charge(900).unwrap();
        assert_eq!(b.charged(), 900);
        assert_eq!(b.free(), 100);
        assert!(b.fits(100));
        assert!(!b.fits(101));

        let r = b.charge(101).unwrap_err();
        assert_eq!(r.kind(), RefusalKind::OutOfDeclaredRange);
        // The message has to be actionable: the request, the total, the ceiling.
        assert!(r.detail().contains("101"), "{}", r.detail());
        assert!(r.detail().contains("1000"), "{}", r.detail());
        // And the failed charge changed nothing.
        assert_eq!(b.charged(), 900);
    }

    #[test]
    fn a_release_gives_room_back_and_a_double_release_cannot_inflate_it() {
        let mut b = Budget::new(1000).unwrap();
        b.charge(800).unwrap();
        b.release(300);
        assert_eq!(b.charged(), 500);
        assert!(b.fits(500));
        b.release(10_000);
        assert_eq!(b.charged(), 0);
        assert!(b.fits(1000));
    }

    #[test]
    fn fill_is_the_number_a_hud_shows() {
        let mut b = Budget::new(400).unwrap();
        assert_eq!(b.fill(), 0.0);
        b.charge(100).unwrap();
        assert!((b.fill() - 0.25).abs() < 1e-12);
    }

    #[test]
    fn the_worker_count_comes_from_the_mesher_s_own_scratch_size() {
        // The load-bearing use of `how_many_fit`: the pool is sized by what the
        // machine can hold, not by how many cores it has.
        let scratch = Mesher::new().scratch_bytes();
        // Measured: ~4.8 MB. If the raster ever changes size this catches it.
        assert!(
            (4_000_000..6_000_000).contains(&scratch),
            "one mesher's scratch is {scratch} B, which is not the measured ~4.8 MB"
        );

        let budget = Budget::new(256 * 1024 * 1024).unwrap();
        // With 64 MB held back for meshes and the rest of the client.
        let workers = budget.how_many_fit(scratch, 64 * 1024 * 1024);
        assert_eq!(workers, (256 - 64) * 1024 * 1024 / scratch);
        assert!(workers > 8, "a 256 MB budget affords {workers} workers");

        // And a budget too small for one worker affords none, honestly.
        let tiny = Budget::new(scratch / 2).unwrap();
        assert_eq!(tiny.how_many_fit(scratch, 0), 0);
        assert_eq!(tiny.how_many_fit(0, 0), 0, "a zero-size allocation is not a worker");
    }
}
