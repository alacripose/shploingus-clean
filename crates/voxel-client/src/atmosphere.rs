//! **The atmosphere's own derivation, and the frame that no longer pays it.**
//!
//! `Stage::Atmosphere` is due every four ticks (H-8.3), and everything behind that tick — the sky,
//! the rig, the field's solver, the dome and the two bodies — is **one pure function of
//! `(spec, tick, inputs, eye, new_moon_tick)`**. It is also **19 ms**: measured on the shipped
//! binary, `SHPLOINGUS_FRAMES=1` puts `sky-light 14.4 | sky-dome 4.3` on every cadence frame, on a
//! frame that otherwise draws in 10–11 ms. That is the judder, and it is four stalls a second of a
//! fifth of a second's frame.
//!
//! # Why a look-ahead and not a cheaper derivation
//!
//! There is no cheaper derivation of the same numbers: the cost is the solver's own beams through
//! the spectra and the dome's five stencil rays, and this project's rule is that the light is
//! *measured*, not approximated for speed. What there is instead is **time**: the next cadence is
//! known four ticks before it is due, and the derivation for it does not depend on anything that
//! happens in between except the eye — which [`AtmosphereAhead::take`] checks rather than assumes.
//!
//! So the frame asks for the next cadence's atmosphere as soon as it has adopted this one, the
//! worker derives it while four frames draw, and the frame that is due **adopts a value that is
//! already in hand**. Nothing is skipped and nothing is blended: the adopted value is the same
//! function of the same arguments as the one the frame would have derived itself, and
//! [`AtmosphereAhead::take`] takes it only when the arguments match to the bit.
//!
//! # What a mismatch costs, stated
//!
//! If the eye moved between the ask and the due tick, the inputs the frame samples
//! (`sky_inputs_at` reads the air at the player's own `x`/`z`) are not the inputs the answer was
//! derived from, so the answer is **not** adopted: the frame derives the cadence itself, exactly as
//! it did before this module existed. That is the honest cost of a look-ahead, and it is **counted**
//! ([`AtmosphereAhead::derived_here`]) rather than hidden — a look-ahead that silently served a
//! derivation of other arguments would be a frame lit by an atmosphere nobody asked for.

use std::sync::mpsc::{Receiver, Sender};

use voxel_appearance::sky::SkyInputs;
use voxel_shapers::WorldSpec;

use crate::quad::BodyRecord;
use crate::skylight::{
    daylight_of, peak_sun_luminance, Daylight, SkyModel, WorldLight,
};

/// **One tick's atmosphere**, as the frame needs it: the inputs it was derived from, the rig, the
/// field's solver, the sky the background is drawn against, and the bodies drawn in it.
///
/// The inputs travel with it because they are *arguments*, not context: the frame re-samples them at
/// the due tick and compares, and an answer whose arguments do not match is not this tick's.
pub struct Atmosphere {
    /// The tick this was derived for.
    pub tick: u64,
    /// The eye it was derived for — the anchor of the bodies and the `x`/`z` the inputs were read at.
    pub eye_m: [f64; 3],
    /// The atmosphere it was derived in.
    pub inputs: SkyInputs,
    /// The rig, or `None` when the atmosphere is refused — **the same refusal** the frame's
    /// `write_rig` holds through.
    pub daylight: Option<Daylight>,
    /// The field's solver, derived from the same `Daylight` rather than a second one.
    pub light: Option<WorldLight>,
    /// The dome's model and the peak sun it was normalised by, or `None` when there is no sky to
    /// draw.
    pub sky: Option<(SkyModel, f64)>,
    /// The Sun and the Moon, in that order, at this eye's grade. Empty when there is none to draw.
    pub bodies: Vec<BodyRecord>,
}

impl Atmosphere {
    /// The tick's atmosphere **at the inputs the caller already sampled**.
    ///
    /// One `daylight_of`, one `WorldLight` from it, one model, two bodies: the derivation the frame
    /// loop used to do inline, in one place so that the worker and the fallback cannot be two
    /// dialects of it.
    pub fn of(
        spec: &WorldSpec,
        tick: u64,
        inputs: SkyInputs,
        eye_m: [f64; 3],
        new_moon_tick: u64,
    ) -> Self {
        let daylight = daylight_of(spec, tick, inputs, new_moon_tick);
        let light = daylight
            .as_ref()
            .and_then(|daylight| WorldLight::with_daylight(spec, tick, inputs, new_moon_tick, daylight));
        let sky = daylight
            .as_ref()
            .and_then(|daylight| sky_model_at(spec, daylight, inputs, tick, eye_m, new_moon_tick));
        let bodies = match (&daylight, &sky) {
            // **A body is drawn against a sky**, so no sky is no body: leaving the last tick's in
            // place would put a sun in a frame that just said it has none.
            (Some(daylight), Some((_, peak))) => {
                bodies_at(spec, daylight, *peak, tick, eye_m, new_moon_tick)
            }
            _ => Vec::new(),
        };
        Atmosphere { tick, eye_m, inputs, daylight, light, sky, bodies }
    }

    /// Whether this answer is the derivation for `(tick, inputs, eye)` **to the bit**.
    ///
    /// The eye is compared as bits rather than as a distance: this is not a tolerance, it is "the
    /// same arguments", and an answer derived for a different anchor is a different picture.
    pub fn is_of(&self, tick: u64, inputs: SkyInputs, eye_m: [f64; 3]) -> bool {
        self.tick == tick
            && self.inputs == inputs
            && self.eye_m.map(f64::to_bits) == eye_m.map(f64::to_bits)
    }
}

/// **The sky's transport, as the GPU reads it**, at this `x`/`z`: the model and the peak sun it was
/// normalised by.
///
/// **Absence is a refusal, not a dark sky.** A non-finite or non-positive peak is refused
/// explicitly, because `peak <= 0.0` is false for a NaN and one would otherwise reach the model as a
/// radiance.
pub fn sky_model_at(
    spec: &WorldSpec,
    daylight: &Daylight,
    inputs: SkyInputs,
    tick: u64,
    eye_m: [f64; 3],
    new_moon_tick: u64,
) -> Option<(SkyModel, f64)> {
    let peak = peak_sun_luminance(spec, inputs);
    if !peak.is_finite() || peak <= 0.0 {
        return None;
    }
    let air = crate::sun::air_at(spec, eye_m[0], eye_m[2], tick);
    Some((
        SkyModel::of_graded(
            &daylight.sky,
            &air,
            peak,
            &voxel_shapers::sky::moon_position(spec, tick, new_moon_tick),
            daylight.grade,
        ),
        peak,
    ))
}

/// The Sun and the Moon, in that order, from the one `Sky` and the one normaliser the dome was
/// drawn against.
fn bodies_at(
    spec: &WorldSpec,
    daylight: &Daylight,
    peak: f64,
    tick: u64,
    eye_m: [f64; 3],
    new_moon_tick: u64,
) -> Vec<BodyRecord> {
    let mut bodies = Vec::new();
    if let Some(sun) = crate::sun::DrawnSun::of(&daylight.sky, peak, eye_m) {
        bodies.push(sun.body_record_graded(eye_m, daylight.grade));
    }
    if let Some(moon) = crate::sun::DrawnMoon::of(
        &voxel_shapers::sky::moon_position(spec, tick, new_moon_tick),
        &crate::sun::air_at(spec, eye_m[0], eye_m[2], tick),
        peak,
        eye_m,
        daylight.sky.sun_altitude_deg,
        voxel_shapers::sky::solar_position(spec, tick).azimuth_deg,
    ) {
        bodies.push(moon.body_record_graded(eye_m, daylight.grade));
    }
    bodies
}

/// **The atmosphere cadence, one cadence ahead, on its own thread.**
///
/// The worker owns nothing but the world's spec: the ask is `(tick, eye)` and the answer is an
/// [`Atmosphere`], so the thread cannot be reading a world the frame is not drawing.
pub struct AtmosphereAhead {
    /// What the worker is asked: the tick, the inputs the frame will derive it in, and the eye it
    /// is derived at. **The inputs are an argument and not a sample** — the ozone is a *held*
    /// quantity (its stage is daily), so a worker that read them itself would be answering a
    /// different atmosphere than the frame asked about.
    asks: Sender<(u64, SkyInputs, [f64; 3])>,
    /// What it has finished, in the order it finished.
    answers: Receiver<Atmosphere>,
    /// The tick asked for and not yet answered, so one cadence's question is asked **once**.
    in_flight: Option<u64>,
    /// The newest answer that has arrived and not been taken or dropped.
    ready: Option<Atmosphere>,
    /// Cadences the frame had to derive itself: the answer was not there, or was for other
    /// arguments. **Counted rather than hidden** — it is the whole of what this costs.
    derived_here: u64,
}

impl AtmosphereAhead {
    /// Start the one thread that derives the cadence for the life of the program.
    ///
    /// The spec goes over at the start: it is `Copy`, it does not change, and the worker cannot ask
    /// a different world than the frame draws.
    pub fn new(spec: WorldSpec, new_moon_tick: u64) -> std::io::Result<Self> {
        let (asks, ask_rx) = std::sync::mpsc::channel::<(u64, SkyInputs, [f64; 3])>();
        let (answer_tx, answers) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("atmosphere".to_owned())
            .spawn(move || {
                while let Ok((tick, inputs, eye_m)) = ask_rx.recv() {
                    let atmosphere = Atmosphere::of(&spec, tick, inputs, eye_m, new_moon_tick);
                    // A closed channel means the program is going: this is the last thing the
                    // thread does, and the frame keeps what it has.
                    if answer_tx.send(atmosphere).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            asks,
            answers,
            in_flight: None,
            ready: None,
            derived_here: 0,
        })
    }

    /// Ask for `tick`'s atmosphere **in `inputs` at `eye_m`**. A tick already in flight is not
    /// asked twice.
    pub fn ask(&mut self, tick: u64, inputs: SkyInputs, eye_m: [f64; 3]) {
        if self.in_flight == Some(tick) {
            return;
        }
        if self.asks.send((tick, inputs, eye_m)).is_ok() {
            self.in_flight = Some(tick);
        }
    }

    /// The tick's atmosphere: **the answer in hand when it is the same derivation in every
    /// argument**, and otherwise one derived here, counted.
    ///
    /// `inputs` are the frame's own reading at this tick — the same call the frame made before this
    /// module existed — so a fallback is exactly the old path and an adoption is exactly its
    /// result.
    pub fn cadence(
        &mut self,
        spec: &WorldSpec,
        tick: u64,
        inputs: SkyInputs,
        eye_m: [f64; 3],
        new_moon_tick: u64,
    ) -> Atmosphere {
        if let Some(answer) = self.take(tick, inputs, eye_m) {
            return answer;
        }
        self.derived_here += 1;
        Atmosphere::of(spec, tick, inputs, eye_m, new_moon_tick)
    }

    /// Whatever has finished since the last call; the **newest** answer is the one kept.
    fn drain(&mut self) {
        while let Ok(answer) = self.answers.try_recv() {
            if self.in_flight == Some(answer.tick) {
                self.in_flight = None;
            }
            self.ready = Some(answer);
        }
    }

    /// Take the answer **if it is this tick's derivation**, and nothing otherwise.
    ///
    /// An answer for another tick, or for other arguments, is dropped rather than returned: a frame
    /// that drew it would be drawing an atmosphere it did not ask for.
    pub fn take(&mut self, tick: u64, inputs: SkyInputs, eye_m: [f64; 3]) -> Option<Atmosphere> {
        loop {
            self.drain();
            let answer = self.ready.take()?;
            // **An answer for a cadence already drawn is dropped without costing this one.** It is
            // what a late worker leaves behind, and letting it occupy the slot would make the frame
            // derive a cadence the worker had already answered.
            if answer.tick < tick {
                continue;
            }
            return answer.is_of(tick, inputs, eye_m).then_some(answer);
        }
    }

    /// Cadences the frame derived itself because no matching answer was in hand.
    pub fn derived_here(&self) -> u64 {
        self.derived_here
    }

    /// Whether `tick` is already in flight or in hand — so a caller that asks every frame does not
    /// re-sample the inputs for a question that has already been put.
    pub fn asked(&self, tick: u64) -> bool {
        self.in_flight == Some(tick) || self.ready.as_ref().map(|answer| answer.tick) == Some(tick)
    }

    /// Whether a tick's answer is already in hand — for a caller that wants to ask for the cadence
    /// after it only once the cadence before it has landed.
    pub fn ready_tick(&mut self) -> Option<u64> {
        self.drain();
        self.ready.as_ref().map(|answer| answer.tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn a_world() -> WorldSpec {
        WorldSpec::new(2468, voxel_shapers::Config::default())
    }

    /// **A fingerprint of everything a frame can see in one atmosphere** — the inputs, the rig's
    /// grade, the field's beam, the dome's own bytes and the two bodies — so "the worker's value is
    /// the frame's" is one equality rather than a list of fields a later change could forget.
    fn fingerprint(atmosphere: &Atmosphere) -> u64 {
        // FNV-1a: a test's identity check, not a published digest.
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let mut eat = |bytes: &[u8]| {
            for byte in bytes {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        eat(&atmosphere.tick.to_le_bytes());
        for axis in atmosphere.eye_m {
            eat(&axis.to_bits().to_le_bytes());
        }
        eat(&atmosphere.inputs.turbidity.to_bits().to_le_bytes());
        eat(&atmosphere.inputs.ozone_atm_cm.to_bits().to_le_bytes());
        if let Some(daylight) = &atmosphere.daylight {
            eat(&daylight.grade.to_bits().to_le_bytes());
            eat(&daylight.sun_altitude_deg.to_bits().to_le_bytes());
        }
        if let Some(light) = &atmosphere.light {
            for axis in light.toward_beam() {
                eat(&axis.to_bits().to_le_bytes());
            }
        }
        if let Some((model, peak)) = &atmosphere.sky {
            eat(&peak.to_bits().to_le_bytes());
            eat(&model.bytes());
        }
        for body in &atmosphere.bodies {
            for axis in body.centre_m {
                eat(&axis.to_bits().to_le_bytes());
            }
            for band in body.radiance {
                eat(&band.to_bits().to_le_bytes());
            }
        }
        hash
    }

    /// **The look-ahead is the same derivation, and it is adopted rather than repeated.**
    ///
    /// This is the pin the mission asks for in the only form that survives a refactor: the worker's
    /// value and the frame's own value are equal, and the frame that takes a matching answer
    /// **derives nothing** (`derived_here` does not move). A revert to deriving on the frame — the
    /// full rebuild returning — leaves `derived_here` incrementing on every cadence and fails the
    /// second assertion.
    #[test]
    fn the_look_ahead_is_the_same_derivation_and_the_frame_adopts_it() {
        let spec = a_world();
        let tick = 96_012u64;
        use crate::skylight::sky_inputs_at;
        let eye = [12.0, 40.5, -7.25];
        let inputs = sky_inputs_at(&spec, eye[0], eye[2], tick);
        let here = Atmosphere::of(&spec, tick, inputs, eye, 0);
        assert!(here.daylight.is_some(), "a tick with an atmosphere to derive");

        let mut ahead = AtmosphereAhead::new(spec, 0).expect("a thread to derive on");
        ahead.ask(tick, inputs, eye);
        // The worker is a thread and the derivation is milliseconds; poll it rather than sleep a
        // guessed duration. The bound is generous because this is a test, not a frame.
        let deadline = Instant::now() + Duration::from_secs(20);
        let adopted = loop {
            let before = ahead.derived_here();
            let answer = ahead.cadence(&spec, tick, inputs, eye, 0);
            if ahead.derived_here() == before {
                break answer;
            }
            assert!(Instant::now() < deadline, "the worker never answered");
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(fingerprint(&adopted), fingerprint(&here), "the worker's value is the frame's");

        // And the same answer, for other arguments, is **not** adopted: the frame derives rather
        // than serving an atmosphere nobody asked for.
        ahead.ask(tick, inputs, eye);
        let deadline = Instant::now() + Duration::from_secs(20);
        while ahead.ready_tick() != Some(tick) {
            assert!(Instant::now() < deadline, "the worker never answered");
            std::thread::sleep(Duration::from_millis(20));
        }
        let other = SkyInputs {
            turbidity: inputs.turbidity + 0.5,
            ozone_atm_cm: inputs.ozone_atm_cm,
        };
        assert!(
            ahead.take(tick, other, eye).is_none(),
            "an answer for other inputs is not this tick's"
        );
        ahead.ask(tick, inputs, eye);
        let deadline = Instant::now() + Duration::from_secs(20);
        while ahead.ready_tick() != Some(tick) {
            assert!(Instant::now() < deadline, "the worker never answered");
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            ahead.take(tick, inputs, [eye[0] + 1.0, eye[1], eye[2]]).is_none(),
            "an answer for another eye is not this tick's"
        );
    }
}
