//! The clock — one global time, and the bound on how fast it may run.
//!
//! **Ticket `BLD-0013`**, from H-7.2 and H-8.3: a base **20 Hz** clock with
//! declared stage cadences, where the deferred tick is the *schedule* and never
//! state.
//!
//! # Why the tick did not advance, and what changes here
//!
//! `game::GAME_TICK` was a constant `0`, and its own doc said so: *"a HUD that
//! showed a moving tick would be reporting a simulation this build does not
//! have."* That was true then. It is not the reason a clock is absent, and this
//! module is where the difference is: the **tick now advances**, so the sun moves,
//! and the statement becomes false and is corrected rather than left standing.
//!
//! # The accumulator and its bound, which is the whole point
//!
//! A frame that takes longer than one tick must not hand the world every tick it
//! missed. An unbounded accumulator converts a two-second stall — a shader compile,
//! a window drag, a machine waking up — into **forty world ticks** of simulation,
//! so the world's speed becomes a function of how badly the renderer is doing.
//!
//! [`MAX_CATCH_UP`] is that refusal, and [`Clock::advance`] **counts what it
//! dropped** rather than discarding it silently: a caller can read
//! [`Clock::dropped`] and know the world is behind. A clock that quietly loses
//! time is worse than one that stutters, because nothing reports it.
//!
//! # What this is *not*
//!
//! **Not a source of determinism.** The clock reads `f64` seconds from a caller
//! and produces a tick; F1 still says the *world* is a function of
//! `(seed, tick)`, and two machines that reach tick 40,000 have the same world.
//! The clock decides *when* a tick happens, never *what* it is.

/// The base tick rate, hertz. H-7.2's answer, and the number
/// [`voxel_shapers::stages::Cadence`] measures its rungs against.
///
/// **The world's own declaration, not this file's.** The day's length is derived from it
/// ([`voxel_shapers::climate::TICKS_PER_DAY`]), so a rate typed twice would let the clock and the
/// calendar disagree about how long a day is — the same defect the tick accumulator's own doc
/// records one level down.
pub const TICK_HZ: u32 = voxel_shapers::climate::TICKS_PER_SECOND as u32;

/// Seconds per tick, derived from [`TICK_HZ`] rather than restated.
///
/// **The one place the two could disagree**, so it is a function and not a
/// literal: a base tick that is 50 ms here and 20 ms in the stage ladder would be
/// two different clocks wearing one name.
pub const fn tick_seconds() -> f64 {
    1.0 / TICK_HZ as f64
}

/// Milliseconds per tick, **derived from [`tick_seconds`]** rather than restated.
///
/// The unit the change view's lifetime is measured in (`shell::changes`): a 1500 ms outline is
/// exactly thirty ticks, and deriving it here means the view cannot be described in a clock the
/// world does not run on.
pub fn tick_millis() -> f64 {
    tick_seconds() * 1000.0
}

/// The most ticks one `advance` may hand the world, whatever time passed.
///
/// **A declared boundary, not a performance tuning.** Four ticks is one fifth of
/// a second of catch-up: long enough that an ordinary dropped frame or two does
/// not slow the world down at all, and short enough that a stall is refused
/// rather than simulated. Chosen because it is a **round fraction of a second**
/// rather than a round number of ticks, which is the same reasoning C-30 used when
/// it made the streaming window a box: the quantity has a unit the reader already
/// has.
pub const MAX_CATCH_UP: u32 = 4;

/// A fixed-rate world clock.
#[derive(Clone, PartialEq, Debug)]
pub struct Clock {
    /// The tick the world is at. The only value the world reads.
    pub tick: u64,
    /// **Ticks** carried but not yet a whole one.
    ///
    /// Carried in *ticks* rather than in seconds, and that is a correction rather
    /// than a style. The first version held seconds and divided by
    /// `tick_seconds()`, which is `1.0 / 20.0` — not representable in binary — so
    /// three and a half seconds divided by it gave `69.99999999999999`, and the
    /// clock **silently lost a tick every three and a half seconds**. A test that
    /// divided `3.5` into 64 exact pieces still failed on it, which is how it was
    /// found: the arithmetic was exact going in and wrong coming out. Ticks are
    /// the unit the accumulator wants, and seconds are converted once, on the way in.
    accumulator_ticks: f64,
    /// Ticks refused because a single `advance` exceeded [`MAX_CATCH_UP`]. **Kept,
    /// never discarded**: a caller that wants to know the world is behind reads
    /// this, and a clock that lost time silently would be worse than one that
    /// stutters.
    pub dropped: u64,
}

impl Default for Clock {
    fn default() -> Self {
        Clock::new()
    }
}

impl Clock {
    /// A clock at tick 0 with nothing carried.
    pub fn new() -> Self {
        Clock {
            tick: 0,
            accumulator_ticks: 0.0,
            dropped: 0,
        }
    }

    /// A clock already at a tick, with nothing carried. For a world resuming from
    /// a save: `LEDGER.md` H-7.16 says the save is record + seed + tick, so a
    /// resumed world starts at the tick it was saved at and not at zero.
    pub fn at(tick: u64) -> Self {
        Clock {
            tick,
            accumulator_ticks: 0.0,
            dropped: 0,
        }
    }

    /// How many whole ticks are due for `seconds` of elapsed time, **and what to
    /// do with the ones past [`MAX_CATCH_UP`]**.
    ///
    /// Split out from the mutation so the arithmetic is testable without a
    /// clock, and so the *decision* — how many ticks, how many dropped — is one
    /// value a caller can read rather than a side effect to infer.
    pub fn due(&self, seconds: f64) -> Due {
        if !seconds.is_finite() || seconds < 0.0 {
            // **A negative or non-finite duration is refused, not clamped.** A
            // frame loop that measured a negative `dt` has a bug, and clamping it
            // to zero would hide it while the world quietly ran slow. `Due` is
            // therefore able to say *no*, which a bare count could not.
            return Due {
                ticks: 0,
                dropped: 0,
                refused: Some(format!("{seconds} seconds is not a duration")),
                // **The accumulator is left exactly as it was.** A refused frame
                // contributes nothing and clears nothing: it is not a duration, so
                // it cannot be a remainder either. Zeroing it would silently
                // discard the fraction of a tick the world had already earned.
                remainder_ticks: self.accumulator_ticks,
            };
        }
        // **Seconds become ticks once, here**, by multiplying by the rate rather
        // than dividing by its reciprocal: `3.5 * 20.0` is exactly `70.0` in
        // binary, where `3.5 / (1.0/20.0)` is `69.99999999999999`. That one
        // character is the whole difference between a clock that keeps time and
        // one that quietly runs slow.
        let carried = self.accumulator_ticks + seconds * TICK_HZ as f64;
        let whole = carried.floor();
        let ticks = whole as u32;
        if ticks > MAX_CATCH_UP {
            let dropped = ticks - MAX_CATCH_UP;
            // **The remainder is kept, not zeroed.** Zeroing it would discard the
            // sub-tick remainder as well and lose time every time the bound was
            // hit, which is how an accumulator drifts.
            return Due {
                ticks: MAX_CATCH_UP,
                dropped: dropped as u64,
                refused: None,
                remainder_ticks: carried - whole,
            };
        }
        Due {
            ticks,
            dropped: 0,
            refused: None,
            remainder_ticks: carried - whole,
        }
    }

    /// Advance the world by `seconds`, returning how many ticks it moved.
    pub fn advance(&mut self, seconds: f64) -> u32 {
        let due = self.due(seconds);
        self.tick = self.tick.saturating_add(due.ticks as u64);
        self.accumulator_ticks = due.remainder_ticks;
        self.dropped = self.dropped.saturating_add(due.dropped);
        due.ticks
    }

    /// How far through the current tick the clock is, `[0, 1)`. For **render
    /// interpolation**: a client drawing at 144 Hz while the world ticks at 20 Hz
    /// draws the same world four times between ticks, and this is the fraction
    /// that says where between them it is.
    pub fn interpolation(&self) -> f64 {
        self.accumulator_ticks.clamp(0.0, 1.0)
    }
}

/// **A commanded move of the world's time, without touching the tick rate.**
///
/// The console's `time` verb names an instant; this states how the frame gets there. It never
/// changes [`Clock::advance`]: ticks keep arriving at [`TICK_HZ`], and what moves is the **epoch
/// offset** the calendar reads, `world_tick = clock_tick + offset`. One tick of simulation stays
/// one tick of simulation; the date slides across it.
///
/// **The distance defines the window.** A seek remembers where it started and where it is going in
/// *ticks of world time*; the caller gives a `ratio` of **ticks of travel per tick of distance**, so
/// a target ten days away at `ratio = 2.0` arrives after twenty days of simulated ticks however
/// near or far that is from the frame rate. The **shape** is a cubic ease-in-out
/// (`smoothstep`'s curve, in its polynomial form): it starts and ends at rest, which is what keeps
/// the sky from snapping at either end of a seek.
#[derive(Clone, PartialEq, Debug)]
pub struct TimeSeek {
    /// The epoch offset this seek began from, in ticks.
    pub from_offset: f64,
    /// The epoch offset it is heading for, in ticks.
    pub to_offset: f64,
    /// Ticks of the seek that have elapsed.
    pub elapsed: u64,
    /// Ticks the seek takes, derived from the distance and the ratio.
    pub duration: u64,
}

impl TimeSeek {
    /// **A seek from one world tick to another.** `ratio` is ticks of travel per tick of distance:
    /// `1.0` takes exactly as long as the distance it has to cover (a pause at the destination),
    /// `0.5` twice as long, `2.0` half as long. A ratio that is not a positive finite number, or a
    /// target equal to the start, is **no seek** — `None`, not a zero-length one that divides by it.
    pub fn between(from_tick: u64, target_tick: u64, ratio: f64) -> Option<Self> {
        if !ratio.is_finite() || ratio <= 0.0 {
            return None;
        }
        let distance = target_tick.abs_diff(from_tick) as f64;
        if distance == 0.0 {
            return None;
        }
        let duration = (distance * ratio).round().max(1.0) as u64;
        Some(Self {
            from_offset: 0.0,
            to_offset: target_tick as f64 - from_tick as f64,
            elapsed: 0,
            duration,
        })
    }

    /// The offset at a seek that began at `start_offset`, **cubic ease-in-out**.
    ///
    /// `s(t) = 4t³` for the first half and `1 − 4(1−t)³` for the second: the same curve as
    /// `smoothstep`, written out so the two halves and their meeting point are visible. At `t = 1`
    /// it is exactly the destination, so a seek that has run its duration does not leave a
    /// fraction of a day standing.
    pub fn offset_at(&self, start_offset: f64) -> f64 {
        let t = (self.elapsed as f64 / self.duration as f64).clamp(0.0, 1.0);
        let eased = if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - 4.0 * (1.0 - t) * (1.0 - t) * (1.0 - t)
        };
        start_offset + (self.to_offset - start_offset) * eased
    }

    /// Whether the seek has reached its destination.
    pub fn done(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// Advance the seek by `ticks` of simulation time, returning how many it took.
    pub fn advance(&mut self, ticks: u64) -> u64 {
        let before = self.elapsed;
        self.elapsed = self.elapsed.saturating_add(ticks);
        self.elapsed - before
    }
}

/// What a [`Clock::due`] decided. See [`Clock::due`] for why this is a type
/// rather than a bare tick count.
#[derive(Clone, PartialEq, Debug)]
pub struct Due {
    /// Ticks the world may advance.
    pub ticks: u32,
    /// Ticks refused because the frame was too long.
    pub dropped: u64,
    /// Set when the elapsed time was not a duration at all.
    pub refused: Option<String>,
    /// The sub-tick remainder carried forward, **in ticks**, so no time is lost.
    pub remainder_ticks: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_rate_is_the_ruling_and_the_seconds_are_derived_from_it() {
        // H-7.14 confirmed the 20 Hz *prototype* as evidence of shape, and H-8.3
        // fixed the base at 20 Hz. So the number is asserted, and `tick_seconds`
        // is asserted as its reciprocal so the two cannot drift into being two
        // clocks wearing one name.
        assert_eq!(TICK_HZ, 20);
        assert!((tick_seconds() - 0.05).abs() < 1e-12);
        assert!((tick_seconds() * TICK_HZ as f64 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_clock_at_rest_does_not_move_and_a_tick_advances_by_exactly_one() {
        let mut clock = Clock::new();
        assert_eq!(clock.advance(0.0), 0);
        assert_eq!(clock.tick, 0);
        assert_eq!(clock.advance(tick_seconds()), 1);
        assert_eq!(clock.tick, 1);
        assert_eq!(clock.advance(tick_seconds()), 1);
        assert_eq!(clock.tick, 2);
    }

    #[test]
    fn twenty_ticks_of_real_time_are_twenty_ticks_of_world() {
        // **The claim H-8.3 makes, measured.** Feeding the clock a second of
        // frames — 50 frames of 20 ms, the rate it claims — must produce exactly
        // 20 ticks. A clock that drifts by even one tick a second would make the
        // world's day length a function of the frame rate.
        let mut clock = Clock::new();
        let frame = 1.0 / 60.0; // deliberately NOT 20 ms: a real client at 60 fps
        let mut frames = 0;
        while clock.tick < 20 && frames < 100_000 {
            clock.advance(frame);
            frames += 1;
        }
        assert_eq!(
            clock.tick, 20,
            "a second of frames at 60 fps did not give 20 ticks"
        );
        assert_eq!(
            clock.dropped, 0,
            "a normal frame rate was treated as a stall"
        );
        // And the frame count is right: 60 frames of 1/60 s is one second.
        assert!(
            (59..=61).contains(&frames),
            "it took {frames} frames to reach a second"
        );
    }

    #[test]
    fn the_sub_tick_remainder_is_carried_so_no_time_is_lost() {
        // **A quarter of a tick, four times, is one tick.** If the remainder were
        // discarded the clock would lose a tick per frame and the world would run
        // slow forever — the classic accumulator bug, and one a test that only
        // fed whole ticks would never catch.
        let mut clock = Clock::new();
        let quarter = tick_seconds() / 4.0;
        for _ in 0..3 {
            assert_eq!(clock.advance(quarter), 0, "a quarter tick moved the world");
        }
        assert_eq!(
            clock.advance(quarter),
            1,
            "the fourth quarter did not complete a tick"
        );
        assert_eq!(clock.tick, 1);
    }

    #[test]
    fn a_long_frame_is_capped_and_what_it_dropped_is_counted() {
        // **The bound, and the counting.** Ten seconds of elapsed time is two
        // hundred ticks. Without the bound the world would run 200 ticks in one
        // frame; with it the world runs 4 and the other 196 are **recorded**.
        let mut clock = Clock::new();
        let moved = clock.advance(10.0);
        assert_eq!(moved, MAX_CATCH_UP, "the bound let {moved} ticks through");
        assert_eq!(clock.tick, MAX_CATCH_UP as u64);
        assert_eq!(
            clock.dropped,
            200 - MAX_CATCH_UP as u64,
            "the dropped ticks were not counted"
        );
    }

    #[test]
    fn the_bound_is_a_round_fraction_of_a_second_not_a_round_number_of_ticks() {
        // C-30's reasoning, applied: the constant is meaningful because the unit
        // the reader already has is *seconds*, so the value is asserted as one.
        assert_eq!(MAX_CATCH_UP, 4);
        assert!(
            (MAX_CATCH_UP as f64 * tick_seconds() - 0.2).abs() < 1e-12,
            "the catch-up window is not one fifth of a second"
        );
    }

    #[test]
    fn a_frame_at_exactly_the_bound_is_honoured_and_one_past_it_is_not() {
        // Both sides of the bound, because a bound tested from one side only does
        // not say which side owns the boundary.
        let mut at_bound = Clock::new();
        assert_eq!(
            at_bound.advance(MAX_CATCH_UP as f64 * tick_seconds()),
            MAX_CATCH_UP
        );
        assert_eq!(at_bound.dropped, 0);

        // **One whole tick past the bound, not a fraction of one past it.** The
        // first version added `0.001` of a tick to the window and expected a
        // dropped tick, and got none — correctly, because `4.001` ticks *floor* to
        // four. The bound is on whole ticks, so the first frame that is refused
        // one is the one that owes **five** of them, and that is what this asserts.
        // A bound that fired on a millisecond would refuse frames for rounding.
        let mut past_bound = Clock::new();
        let one_tick_past = (MAX_CATCH_UP as f64 + 1.0) * tick_seconds();
        assert_eq!(past_bound.advance(one_tick_past), MAX_CATCH_UP);
        assert_eq!(
            past_bound.dropped, 1,
            "the tick past the bound was not reported"
        );

        // And the fraction just inside is still honoured whole.
        let mut just_inside = Clock::new();
        let almost = (MAX_CATCH_UP as f64 + 0.999) * tick_seconds();
        assert_eq!(just_inside.advance(almost), MAX_CATCH_UP);
        assert_eq!(
            just_inside.dropped, 0,
            "a frame a millisecond under five ticks was refused"
        );
    }

    #[test]
    fn a_negative_or_non_finite_frame_is_refused_rather_than_clamped() {
        // **A frame loop that measured a negative `dt` has a bug.** Clamping it to
        // zero would hide that while the world quietly ran slow, which is the
        // failure this repository's first rule exists to prevent.
        for bad in [-0.001, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut clock = Clock::new();
            let due = clock.due(bad);
            assert!(
                due.refused.is_some(),
                "{bad} seconds was accepted as a duration"
            );
            assert_eq!(due.ticks, 0);
            clock.advance(bad);
            assert_eq!(clock.tick, 0, "{bad} seconds moved the world");
            assert_eq!(
                clock.dropped, 0,
                "{bad} seconds was counted as dropped ticks"
            );
        }
    }

    #[test]
    fn hitting_the_bound_does_not_drift_the_remainder() {
        // **The bug the remainder comment names.** Zeroing the remainder on every
        // capped frame would lose the sub-tick remainder too, so a world at a low
        // frame rate would drift steadily behind. Fed exactly the bound repeatedly,
        // the clock must land on a tick multiple, not on a fraction of one.
        let mut clock = Clock::new();
        let window = MAX_CATCH_UP as f64 * tick_seconds();
        for _ in 0..100 {
            clock.advance(window);
        }
        let remainder_ticks = clock.interpolation();
        assert!(
            remainder_ticks.abs() < 1e-9,
            "after 100 capped frames the clock sits {remainder_ticks} of a tick in, so it \\
             has drifted; the remainder is being discarded"
        );
    }

    #[test]
    fn interpolation_is_the_fraction_of_the_current_tick() {
        let mut clock = Clock::new();
        assert_eq!(clock.interpolation(), 0.0);
        clock.advance(tick_seconds() / 2.0);
        assert!((clock.interpolation() - 0.5).abs() < 1e-9);
        clock.advance(tick_seconds() / 2.0);
        assert_eq!(clock.tick, 1);
        assert_eq!(
            clock.interpolation(),
            0.0,
            "a completed tick left a remainder"
        );
    }

    #[test]
    fn a_resumed_world_starts_at_its_saved_tick_not_at_zero() {
        // H-7.16: the save is record + seed + tick. A resumed world that started at
        // tick 0 would be a different world, and the sun would be in the wrong
        // place — which is the first thing the sky makes visible.
        let mut clock = Clock::at(1_234_567);
        assert_eq!(clock.tick, 1_234_567);
        assert_eq!(clock.advance(0.0), 0);
        assert_eq!(clock.interpolation(), 0.0);
    }

    #[test]
    fn two_clocks_given_the_same_time_reach_the_same_tick() {
        // F12's shape on the time axis: the clock decides *when* a tick happens,
        // never *what* it is, so two processes fed the same schedule agree. A clock
        // holding state would break that and a replay with it.
        //
        // **The partition must stay inside the bound for this to be true, and that
        // is the finding.** The first version fed one clock a single 3.5-second gulp
        // and the other 210 frames of 1/60 s, and got 4 against 70 — the bound
        // refusing 66 ticks in the first case. So the accumulator's partition
        // invariance holds *within* `MAX_CATCH_UP` and not beyond it, which is
        // exactly the trade the bound makes: a client rendering at 60 fps and one
        // rendering at 1 fps do **not** see the same world, and the second one
        // says so through `dropped` rather than silently fast-forwarding.
        //
        // Both halves are asserted, because the second half is the interesting one.
        let total = 3.5;
        let frames_at_60 = (total * 60.0) as u32;

        let mut one_gulp = Clock::new();
        one_gulp.advance(total);
        assert_eq!(
            one_gulp.tick, MAX_CATCH_UP as u64,
            "a 3.5 s gulp was not capped"
        );
        assert_eq!(one_gulp.dropped, 70 - MAX_CATCH_UP as u64);

        let mut at_60 = Clock::new();
        for _ in 0..frames_at_60 {
            at_60.advance(1.0 / 60.0);
        }
        assert_eq!(
            at_60.tick, 70,
            "three and a half seconds is seventy ticks at 20 Hz"
        );
        assert_eq!(at_60.dropped, 0);

        // And the invariance itself: two *different* fine partitions of the same
        // 3.5 seconds agree exactly, because neither ever meets the bound.
        //
        // **Both partitions are powers of two.** The first two attempts used 105 and
        // 210 frames — and `3.5 / 105` is not representable in binary, so a
        // hundred and five of them sum to 3.4999999999999996 and the clock
        // correctly answered 69. The clock was right both times and the test's
        // arithmetic was wrong: a test of an accumulator must not introduce an
        // accumulator error of its own and then blame the clock for it.
        //
        // With 64 and 128 frames the step is `7/128` and `7/256`, both exactly
        // representable, so the sum is exactly 3.5 and the assertion is about the
        // clock's arithmetic rather than about binary floating point.
        let run_partition = |frames: u32| {
            let mut clock = Clock::new();
            let frame = total / frames as f64;
            for _ in 0..frames {
                clock.advance(frame);
            }
            clock
        };
        let at_30 = run_partition(64);
        let at_60 = run_partition(128);
        assert_eq!(
            at_30.tick, at_60.tick,
            "a 30 fps and a 60 fps client saw different worlds: {} against {}",
            at_30.tick, at_60.tick
        );
        assert_eq!(
            at_30.tick, 70,
            "an exact partition of 3.5 s is seventy ticks"
        );
    }

    #[test]
    fn a_time_seek_takes_the_distance_times_the_ratio_and_lands_exactly() {
        // Ten days away at `ratio = 2.0` is twenty days of ticks, and the *tick rate* is not a
        // term in it: the seek is in world ticks, so the clock's 20 Hz is untouched.
        let day = voxel_shapers::climate::TICKS_PER_DAY;
        let mut seek = TimeSeek::between(0, 10 * day, 2.0).expect("a seek");
        assert_eq!(
            seek.duration,
            20 * day,
            "the window is not the distance times the ratio"
        );
        assert!(!seek.done());
        // Cubic ease-in-out: it leaves at rest, is halfway at half the window, and arrives at rest.
        assert_eq!(
            seek.offset_at(0.0),
            0.0,
            "a seek that has not started moved the date"
        );
        seek.advance(10 * day);
        assert!(
            (seek.offset_at(0.0) - 5.0 * day as f64).abs() < 1e-6,
            "not halfway at half time"
        );
        seek.advance(10 * day);
        assert!(seek.done());
        assert_eq!(
            seek.offset_at(0.0),
            10.0 * day as f64,
            "a finished seek did not land on the instant exactly"
        );
        // And the curve is slow at the start: a tenth of the window moves less than a tenth.
        let mut early = TimeSeek::between(0, 10 * day, 2.0).expect("a seek");
        early.advance(2 * day);
        assert!(
            early.offset_at(0.0) < 2.0 * day as f64,
            "the ease-in did not ease in"
        );
        // A ratio the arithmetic cannot use is no seek rather than a division by it.
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(
                TimeSeek::between(0, day, bad).is_none(),
                "ratio {bad} was accepted"
            );
        }
        assert!(
            TimeSeek::between(5, 5, 1.0).is_none(),
            "a zero-distance seek was accepted"
        );
        // A backwards seek is the same law in the other direction.
        let back = TimeSeek::between(10 * day, 0, 1.0).expect("a backward seek");
        assert_eq!(back.to_offset, -(10.0 * day as f64));
    }
}
