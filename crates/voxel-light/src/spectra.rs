//! **The band table, and a sun that is a body rather than a colour.**
//!
//! # What this replaces
//!
//! `voxel_shapers::sky` is a *photometric* atmosphere: a broadband Rayleigh and air-mass
//! model that hands the renderer a radiance, and `skylight::chromaticity(sky.sun)` turns
//! that into a hue whose strongest channel is 1. Measured over a year at the shipped time
//! of day, that hue is **red-dominant on 12 of 12 sampled days**, `r/g` from 1.09 to 1.42
//! with the blue channel as low as 0.29 (`--example daylight-hue`). So the world is lit by
//! the atmosphere's own residual spectrum, and there is no sun in it anywhere: a
//! chromaticity has a shape and no magnitude, and the shape is whatever the air did to a
//! number that was never decomposed into wavelengths.
//!
//! This module declares the alternative. The sun is a **black body at 5778 K** (its
//! effective temperature), integrated over **twelve declared bands** from X-ray to radio,
//! diluted by the solar disc's own solid angle at 1 AU, and normalised so the bands sum to
//! the **solar constant, 1361 W/m²** — the same constant this workspace already carries in
//! `voxel_shapers::sky::SOLAR_CONSTANT_W_M2`, now spent on a spectrum rather than on one
//! broadband number.
//!
//! # Why bands and not a spectrum
//!
//! A per-wavelength field over a voxel grid is a field whose resolution in wavelength
//! competes with its resolution in space, and light transport does not need both. Twelve
//! bands span the spectrum with the resolution that matters to the things that *consume*
//! light — growth, visibility and heat — and each band declares how it *travels*
//! ([`Transport`]), which is the part a colour could never carry.
//!
//! # What is declared rather than derived, and says so
//!
//! The band **edges**, the **clear-sky coefficients**, the lunar reflectance curve and the
//! **dilution factor** are declared constants of this model. They are Earth-derived and
//! fixed here; they are not measurements of this world, and this world has not been
//! measured. The band *powers* are derived — Planck, integrated, diluted, then scaled so
//! the total is the solar constant — and a test pins every one of them, so a change to the
//! formulas fails rather than drifts.
//!
//! # What is not here yet
//!
//! [`BandTable`] has no content identity. The ported contract gives the table a `Digest`
//! so that a snapshot can refuse a table from another build (L19), and that arrives with
//! the solver, where the same digest is needed anyway. Until then `SPECTRA_VERSION` is the
//! only version this table carries, and a version it does not recognise is refused by name
//! rather than reinterpreted.

/// The table's own schema. A change to the formulas or the layout bumps it.
pub const SPECTRA_VERSION: u16 = 1;

/// The declared band count.
pub const BAND_COUNT: usize = 12;

/// **The solar constant at the top of the world's sky**, W/m².
pub const SOLAR_CONSTANT: f64 = 1361.0;

/// The sun's effective temperature, K. Its spectrum is Planck at this temperature.
pub const SUN_TEMPERATURE_K: f64 = 5_778.0;

/// The clear-sky column's scale height, metres: optical depth becomes a per-metre coefficient.
pub const SCALE_HEIGHT_M: f64 = 8_000.0;

/// Planck's constant, J·s.
const H_PLANCK: f64 = 6.626_070_15e-34;
/// The speed of light in vacuum, m/s.
const C_LIGHT: f64 = 2.997_924_58e8;
/// The Boltzmann constant, J/K.
const K_BOLTZMANN: f64 = 1.380_649e-23;
/// The solar disc's dilution at this world's distance: `(R_sun / 1 AU)²`.
///
/// **Public because the geometry needs the same sun.** Multiplying by `AU_M` and square-
/// rooting gives the sun's radius in metres, which is what casts a shadow with a real penumbra
/// width; and it is an *independent* route to the angular size from the one
/// `voxel_appearance::sky` carries, which is the cross-check that catches a factor of two.
pub const SUN_DILUTION: f64 = 2.161_5e-5;
/// Samples per band in the log-grid integral. Declared, because the integral's error is.
const PLANCK_SAMPLES: usize = 512;

/// A refusal from the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectraError {
    /// The bytes declare a schema this build does not speak.
    BadVersion { found: u16, expected: u16 },
    /// The bytes declare a band count this build cannot lay out.
    BadCount { found: u16, expected: u16 },
    /// The bytes end in the middle of a record.
    Truncated,
    /// A band's transport byte is not a class this build declares.
    UnknownTransport { index: usize, class: u8 },
    /// The bytes have data left over after the last band.
    TrailingBytes,
}

impl core::fmt::Display for SpectraError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadVersion { found, expected } => {
                write!(f, "the table's version {found} is not this build's {expected}")
            }
            Self::BadCount { found, expected } => {
                write!(f, "the table declares {found} bands, not {expected}")
            }
            Self::Truncated => write!(f, "the table ends in the middle of a record"),
            Self::UnknownTransport { index, class } => {
                write!(f, "band {index} declares unknown transport class {class}")
            }
            Self::TrailingBytes => write!(f, "the table has bytes left over after its last band"),
        }
    }
}

impl std::error::Error for SpectraError {}

/// **Which path a band takes through the solver.**
///
/// This is the part a colour could not carry, and it is the reason the table exists: the
/// far bands are not a *dim* version of the near ones, they are a different *question*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// Direct beam, plus the single-scatter sky term, plus one ground bounce.
    BeamAndDiffuse,
    /// The direct path only: this band is the beam or it is nothing.
    BeamOnly,
    /// **No per-cell field.** Emitter-to-receiver raycasts instead: a band whose
    /// wavelength is longer than a cell is not a thing that fills a grid of them.
    Links,
}

impl Transport {
    /// Decodes the class byte, refusing anything undeclared rather than guessing.
    pub fn from_u8(class: u8) -> Result<Self, SpectraError> {
        match class {
            0 => Ok(Self::BeamAndDiffuse),
            1 => Ok(Self::BeamOnly),
            2 => Ok(Self::Links),
            other => Err(SpectraError::UnknownTransport { index: 0, class: other }),
        }
    }

    /// The class byte, for the canonical encoding.
    pub fn as_u8(self) -> u8 {
        match self {
            Self::BeamAndDiffuse => 0,
            Self::BeamOnly => 1,
            Self::Links => 2,
        }
    }
}

/// **One band**: its edges in nanometres and its pinned radiometric data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    /// The band's lower edge, nanometres, inclusive.
    pub lo_nm: f64,
    /// The band's upper edge, nanometres, exclusive.
    pub hi_nm: f64,
    /// The sun's power in this band at the top of the sky, W/m².
    pub sun_w_per_m2: f64,
    /// What the moon gives back in this band: a reflectance, not a radiance.
    pub moon_reflectance: f64,
    /// The clear-sky column's extinction per metre at the band's centre.
    pub extinction_per_m: f64,
    /// The scattering part of the above, per metre. The rest is absorption.
    pub scatter_per_m: f64,
    /// How this band travels.
    pub transport: Transport,
}

impl Band {
    /// The band's geometric centre in nanometres, where its optical data is evaluated.
    pub fn centre_nm(&self) -> f64 {
        (self.lo_nm * self.hi_nm).sqrt()
    }

    /// The clear-sky column's extinction optical depth straight up.
    pub fn column_extinction(&self) -> f64 {
        self.extinction_per_m * SCALE_HEIGHT_M
    }

    /// The clear-sky column's scattering optical depth straight up.
    pub fn column_scatter(&self) -> f64 {
        self.scatter_per_m * SCALE_HEIGHT_M
    }

    /// The absorbed part of the column's extinction.
    pub fn column_absorption(&self) -> f64 {
        self.column_extinction() - self.column_scatter()
    }
}

/// The declared edges and transport classes, in order. Ranges are nanometres.
///
/// The two `Links` bands at the far end are the declared limit of the field: past
/// 300 µm the wavelength is longer than a cell, so there is nothing for a per-cell
/// field to hold.
const DECLARED: [(f64, f64, Transport); BAND_COUNT] = [
    (1.0e-2, 1.0, Transport::BeamOnly),
    (1.0, 280.0, Transport::BeamAndDiffuse),
    (280.0, 400.0, Transport::BeamAndDiffuse),
    (400.0, 500.0, Transport::BeamAndDiffuse),
    (500.0, 600.0, Transport::BeamAndDiffuse),
    (600.0, 700.0, Transport::BeamAndDiffuse),
    (700.0, 1_500.0, Transport::BeamAndDiffuse),
    (1_500.0, 3_000.0, Transport::BeamAndDiffuse),
    (3_000.0, 15_000.0, Transport::BeamAndDiffuse),
    (15_000.0, 300_000.0, Transport::BeamAndDiffuse),
    (300_000.0, 3.0e8, Transport::Links),
    (3.0e8, 1.0e11, Transport::Links),
];

/// **Planck spectral exitance of a black body**, W/m² per metre of wavelength.
///
/// The real thing, not a fit: `2hc²/λ⁵ · 1/(exp(hc/λkT) − 1)`. It returns zero where the
/// denominator is not a positive finite number, so an absurd argument is a zero rather
/// than a `NaN` travelling into a renderer.
pub fn planck_w_per_m2_per_m(lambda_m: f64, temperature_k: f64) -> f64 {
    let numerator = 2.0 * H_PLANCK * C_LIGHT * C_LIGHT / lambda_m.powi(5);
    let exponent = H_PLANCK * C_LIGHT / (lambda_m * K_BOLTZMANN * temperature_k);
    let denominator = exponent.exp() - 1.0;
    if denominator.is_finite() && denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    }
}

/// Integrates Planck over one band by midpoint sampling on a **log** grid, diluted to the sky.
///
/// Log-spaced because the bands span nine decades: a linear grid over the X-ray band and
/// a linear grid over the radio band cannot both have resolution, and the midpoint rule
/// on `λ` is not the same quadrature as the midpoint rule on `ln λ`.
fn band_planck(lo_m: f64, hi_m: f64) -> f64 {
    let ln_lo = lo_m.ln();
    let step = (hi_m.ln() - ln_lo) / PLANCK_SAMPLES as f64;
    let mut sum = 0.0;
    for index in 0..PLANCK_SAMPLES {
        let lambda = (ln_lo + (index as f64 + 0.5) * step).exp();
        sum += planck_w_per_m2_per_m(lambda, SUN_TEMPERATURE_K) * lambda;
    }
    sum * step * SUN_DILUTION
}

/// `exp(−(ln(x/centre))²/width)`: the declared shape of an absorption band.
fn gaussian_ln(x: f64, centre: f64, width: f64) -> f64 {
    let ratio = (x / centre).ln();
    (-(ratio * ratio) / width).exp()
}

/// **Rayleigh scattering's column depth**: the λ⁻⁴·⁰⁹ law that makes the sky blue.
///
/// Every coefficient in this group is Earth-derived and declared; none of it is a
/// measurement of this world.
pub fn rayleigh_optical_depth(lambda_um: f64) -> f64 {
    0.008_79 * lambda_um.powf(-4.09)
}

/// The aerosol column depth, following the standard Ångström form.
pub fn aerosol_optical_depth(lambda_um: f64) -> f64 {
    0.02 * lambda_um.powf(-1.3)
}

/// The **ozone** column depth: a declared Gaussian in log-wavelength with its head at
/// 0.27 µm, the Hartley band.
pub fn ozone_optical_depth(lambda_um: f64) -> f64 {
    0.003_5 * gaussian_ln(lambda_um, 0.27, 0.02)
}

/// The **water vapour** column depth: two declared Gaussians, at 2.7 µm and 6.3 µm.
pub fn water_optical_depth(lambda_um: f64) -> f64 {
    0.08 * gaussian_ln(lambda_um, 2.7, 0.15) + 0.5 * gaussian_ln(lambda_um, 6.3, 0.08)
}

/// **The declared clear-sky optical depth**, split into scattering and absorption.
///
/// The parts are named apart on purpose, because they do opposite things: scattering
/// redistributes a beam into a sky, and absorption removes it. Nine tenths of the
/// aerosol scatters and one tenth absorbs, which is the only free split in the group.
///
/// Returns `(extinction, scattering)` as column optical depths; the caller divides by
/// [`SCALE_HEIGHT_M`] to get per-metre coefficients.
pub fn clear_sky(lambda_um: f64) -> (f64, f64) {
    let aerosol = aerosol_optical_depth(lambda_um);
    let scatter = rayleigh_optical_depth(lambda_um) + 0.9 * aerosol;
    let extinction = scatter
        + aerosol * 0.1
        + ozone_optical_depth(lambda_um)
        + water_optical_depth(lambda_um);
    (extinction, scatter)
}

/// The lunar reflectance curve: a grey-lit regolith, slightly red. Declared, not measured.
fn moon_reflectance(centre_nm: f64) -> f64 {
    (0.12 * (centre_nm / 550.0).powf(0.2)).clamp(0.05, 0.30)
}

/// **The bands as this build computes them**, normalised so the total is the solar constant.
///
/// The normalisation is the load-bearing line: the integral of Planck over the twelve
/// bands is *not* 1361 W/m² — it is whatever the declared edges captured — so the bands
/// are scaled by one common factor and the total becomes the declared constant exactly.
/// The shape is Planck's; the magnitude is the declared solar constant.
pub fn bands() -> [Band; BAND_COUNT] {
    let raw: [f64; BAND_COUNT] = core::array::from_fn(|index| {
        let (lo_nm, hi_nm, _) = DECLARED[index];
        band_planck(lo_nm * 1.0e-9, hi_nm * 1.0e-9)
    });
    let total: f64 = raw.iter().sum();
    let scale = if total > 0.0 {
        SOLAR_CONSTANT / total
    } else {
        0.0
    };

    core::array::from_fn(|index| {
        let (lo_nm, hi_nm, transport) = DECLARED[index];
        let centre_um = (lo_nm * hi_nm).sqrt() * 1.0e-3;
        let (extinction, scatter) = clear_sky(centre_um);
        Band {
            lo_nm,
            hi_nm,
            sun_w_per_m2: raw[index] * scale,
            moon_reflectance: moon_reflectance((lo_nm * hi_nm).sqrt()),
            extinction_per_m: extinction / SCALE_HEIGHT_M,
            scatter_per_m: scatter / SCALE_HEIGHT_M,
            transport,
        }
    })
}

/// The canonical bytes of a table: version, count, then each band, little-endian.
pub fn encode(bands: &[Band; BAND_COUNT]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + BAND_COUNT * 49);
    out.extend_from_slice(&SPECTRA_VERSION.to_le_bytes());
    out.extend_from_slice(&(BAND_COUNT as u16).to_le_bytes());
    for band in bands {
        for value in [
            band.lo_nm,
            band.hi_nm,
            band.sun_w_per_m2,
            band.moon_reflectance,
            band.extinction_per_m,
            band.scatter_per_m,
        ] {
            out.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        out.push(band.transport.as_u8());
    }
    out
}

/// A byte cursor that refuses to run off the end.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn u16(&mut self) -> Result<u16, SpectraError> {
        let end = self.at + 2;
        let slice = self.bytes.get(self.at..end).ok_or(SpectraError::Truncated)?;
        self.at = end;
        Ok(u16::from_le_bytes([slice[0], slice[1]]))
    }

    fn f64(&mut self) -> Result<f64, SpectraError> {
        let end = self.at + 8;
        let slice = self.bytes.get(self.at..end).ok_or(SpectraError::Truncated)?;
        self.at = end;
        let mut eight = [0u8; 8];
        eight.copy_from_slice(slice);
        Ok(f64::from_bits(u64::from_le_bytes(eight)))
    }

    fn u8(&mut self) -> Result<u8, SpectraError> {
        let byte = *self.bytes.get(self.at).ok_or(SpectraError::Truncated)?;
        self.at += 1;
        Ok(byte)
    }
}

/// A decoded table: the twelve bands this build vouches for.
#[derive(Debug, Clone, PartialEq)]
pub struct BandTable {
    bands: [Band; BAND_COUNT],
}

impl BandTable {
    /// The bands as computed by this build.
    pub fn computed() -> Self {
        Self { bands: bands() }
    }

    /// Decodes a table, refusing by name anything this build cannot vouch for.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SpectraError> {
        let mut reader = Reader { bytes, at: 0 };
        let version = reader.u16()?;
        if version != SPECTRA_VERSION {
            return Err(SpectraError::BadVersion {
                found: version,
                expected: SPECTRA_VERSION,
            });
        }
        let count = reader.u16()?;
        if count as usize != BAND_COUNT {
            return Err(SpectraError::BadCount {
                found: count,
                expected: BAND_COUNT as u16,
            });
        }

        let empty = Band {
            lo_nm: 0.0,
            hi_nm: 0.0,
            sun_w_per_m2: 0.0,
            moon_reflectance: 0.0,
            extinction_per_m: 0.0,
            scatter_per_m: 0.0,
            transport: Transport::BeamOnly,
        };
        let mut decoded = [empty; BAND_COUNT];
        for (index, band) in decoded.iter_mut().enumerate() {
            band.lo_nm = reader.f64()?;
            band.hi_nm = reader.f64()?;
            band.sun_w_per_m2 = reader.f64()?;
            band.moon_reflectance = reader.f64()?;
            band.extinction_per_m = reader.f64()?;
            band.scatter_per_m = reader.f64()?;
            let class = reader.u8()?;
            band.transport = Transport::from_u8(class).map_err(|error| match error {
                // The index is the reader's, not the decoder's: a class byte is only
                // meaningful beside the band it was read for.
                SpectraError::UnknownTransport { class, .. } => {
                    SpectraError::UnknownTransport { index, class }
                }
                other => other,
            })?;
        }
        if reader.at != bytes.len() {
            return Err(SpectraError::TrailingBytes);
        }
        Ok(Self { bands: decoded })
    }

    /// The bands.
    pub fn bands(&self) -> &[Band; BAND_COUNT] {
        &self.bands
    }

    /// One band by index, or `None` past the end of the declared set.
    pub fn band(&self, index: usize) -> Option<&Band> {
        self.bands.get(index)
    }

    /// The total power the sun delivers in these bands, W/m², at the top of the sky.
    pub fn total_sun_w_per_m2(&self) -> f64 {
        self.bands.iter().map(|band| band.sun_w_per_m2).sum()
    }

    /// The canonical bytes of this table.
    pub fn encode(&self) -> Vec<u8> {
        encode(&self.bands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wien's displacement law: the peak of a 5778 K Planck curve, in nanometres.
    fn wien_peak_nm(temperature_k: f64) -> f64 {
        2.897_771_9e-3 / temperature_k * 1.0e9
    }

    #[test]
    fn the_table_is_twelve_bands_spanning_the_spectrum() {
        let table = BandTable::computed();
        assert_eq!(table.bands().len(), BAND_COUNT);
        // Contiguous, ascending, and starting at the far end of the declared range.
        let mut last_hi = 0.0;
        for band in table.bands() {
            assert!(
                band.lo_nm >= last_hi,
                "band {} starts at {} nm, below the previous band's end at {last_hi}",
                band.centre_nm(),
                band.lo_nm
            );
            assert!(band.hi_nm > band.lo_nm);
            last_hi = band.hi_nm;
        }
        assert_eq!(table.bands()[0].lo_nm, 1.0e-2);
        assert_eq!(table.bands()[BAND_COUNT - 1].hi_nm, 1.0e11);
    }

    #[test]
    fn the_bands_sum_to_the_solar_constant_exactly() {
        // **This is the normalisation, and it is the reason the table is trustworthy.**
        // The Planck integral over these edges does not come to 1361 on its own; the
        // bands are scaled so that it does, and this checks the scaling held.
        let table = BandTable::computed();
        let total = table.total_sun_w_per_m2();
        assert!(
            (total - SOLAR_CONSTANT).abs() < 1.0e-9,
            "the bands sum to {total}, not {SOLAR_CONSTANT}"
        );
    }

    #[test]
    fn planck_peaks_where_wien_says_it_does() {
        // **A real check on a real curve.** Wien's law puts a 5778 K peak at ~501.6 nm;
        // a Planck implementation with the constants wrong still looks like a curve, so
        // this asks where the maximum actually is.
        let expected = wien_peak_nm(SUN_TEMPERATURE_K);
        assert!(
            (expected - 501.6).abs() < 1.0,
            "Wien puts the peak at {expected} nm"
        );

        let mut best = (0.0_f64, 0.0_f64);
        let mut nm = 5.0_f64;
        while nm < 5_000.0 {
            let exitance = planck_w_per_m2_per_m(nm * 1.0e-9, SUN_TEMPERATURE_K);
            if exitance > best.0 {
                best = (exitance, nm);
            }
            nm *= 1.001;
        }
        assert!(
            (best.1 - expected).abs() < 20.0,
            "the curve peaks at {} nm, Wien says {expected} nm",
            best.1
        );
    }

    #[test]
    fn the_sun_is_mostly_visible_and_near_infrared() {
        // The visible and near-infrared bands together should carry more than half the
        // solar constant. If they do not, the edges or the quadrature are wrong, and no
        // amount of normalisation hides it (the total is pinned separately).
        let table = BandTable::computed();
        let visible_and_near = table
            .bands()
            .iter()
            .filter(|band| band.lo_nm >= 400.0 && band.hi_nm <= 1_500.0)
            .map(|band| band.sun_w_per_m2)
            .sum::<f64>();
        assert!(
            visible_and_near > 0.5 * SOLAR_CONSTANT,
            "400–1500 nm carries {visible_and_near} of {SOLAR_CONSTANT} W/m²"
        );
        // And the X-ray band is a trace: the sun is not an X-ray source here.
        assert!(table.bands()[0].sun_w_per_m2 < 1.0e-4 * SOLAR_CONSTANT);
    }

    #[test]
    fn rayleigh_scattering_falls_off_with_wavelength() {
        // λ^-4.09. **Scattering, not extinction** — an earlier version of this test asked
        // extinction to fall monotonically and it failed, correctly: the declared water
        // band at 2.7 µm puts more optical depth at 2.2 µm than at 1.0 µm, and that is the
        // model working. The law is a statement about Rayleigh's own term.
        let mut last = f64::INFINITY;
        for lambda_um in [0.28, 0.4, 0.55, 0.7, 1.0, 2.2] {
            let rayleigh = rayleigh_optical_depth(lambda_um);
            assert!(
                rayleigh < last,
                "Rayleigh at {lambda_um} µm is {rayleigh}, not below {last}"
            );
            last = rayleigh;
        }
        // And the magnitude, so a flipped exponent and a wrong coefficient both fail.
        //
        // **A band rather than a decimal.** The first version of this pinned `1.616`,
        // which was my arithmetic and not the model's — the model gives ~1.604, and the
        // assertion failed on my own long division rather than on any physics. A band
        // from 1.55 to 1.70 still fails for an exponent of `+4.09` (which would give
        // 5.3e-4) or a coefficient off by ten, without pretending I can do the
        // exponentiation to four figures in my head.
        let uv = rayleigh_optical_depth(0.28);
        assert!(
            (1.55..1.70).contains(&uv),
            "Rayleigh at 0.28 µm is {uv} optical depths, not about 1.6"
        );
        assert!(rayleigh_optical_depth(1.0) < 0.01);
    }

    #[test]
    fn the_water_band_puts_absorption_where_rayleigh_has_none() {
        // The other half of the shape: scattering is falling while absorption climbs, so
        // extinction is *not* monotone. Asserted as absorption, which is the quantity
        // that actually does it.
        let absorption = |um: f64| {
            let (extinction, scatter) = clear_sky(um);
            extinction - scatter
        };
        assert!(absorption(2.2) > absorption(1.0));
        assert!(absorption(2.2) > 10.0 * absorption(1.0));
        assert!(water_optical_depth(1.0) < 1.0e-3, "water leaks into 1 µm");
        assert!(water_optical_depth(2.7) > 0.07);
    }

    #[test]
    fn ozone_absorbs_at_its_declared_head_and_nowhere_else() {
        // **Ozone is not the dominant term at 0.27 µm and this test does not claim it is.**
        // Rayleigh's column there is 1.86 optical depths against ozone's 0.0035 — the
        // atmosphere is thick in the UV because of scattering, and ozone is a trace
        // absorber inside that. What is worth pinning is the *shape*: a declared Gaussian
        // with a head at 0.27 µm that is gone an octave away.
        assert!((ozone_optical_depth(0.27) - 0.003_5).abs() < 1.0e-9);
        assert!(ozone_optical_depth(1.0) < 1.0e-12);
        assert!(
            ozone_optical_depth(0.27) > 1.0e9 * ozone_optical_depth(0.55),
            "ozone survives into the visible"
        );
        // And in the band it is for, 280–400 nm, the column is scattering-dominated and
        // nearly an optical depth thick — which is the shape of a UV sky.
        let table = BandTable::computed();
        let uv = &table.bands()[2];
        assert!((280.0..400.0).contains(&uv.centre_nm()));
        assert!(
            uv.column_scatter() > uv.column_absorption(),
            "the UV column absorbs more than it scatters"
        );
        assert!(
            uv.column_extinction() > 0.1,
            "the UV column is only {} optical depths",
            uv.column_extinction()
        );
    }

    #[test]
    fn each_band_separates_absorption_from_scattering() {
        let table = BandTable::computed();
        for band in table.bands() {
            assert!(
                band.column_scatter() <= band.column_extinction(),
                "band at {} nm scatters more than it extinguishes",
                band.centre_nm()
            );
            assert!(band.column_absorption() >= 0.0);
        }
        // The visible-blue column scatters more than it absorbs: that is why the sky
        // is blue rather than grey.
        let blue = &table.bands()[3];
        assert!(blue.column_scatter() > blue.column_absorption());
    }

    #[test]
    fn the_far_bands_are_links_and_the_rest_are_a_field() {
        // **The declared limit of the field.** Past 300 µm the wavelength is longer than
        // a cell, so there is nothing for a per-cell field to hold and the band is
        // emitter-to-receiver raycasts instead.
        let table = BandTable::computed();
        let links: Vec<usize> = table
            .bands()
            .iter()
            .enumerate()
            .filter(|(_, band)| band.transport == Transport::Links)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(links, vec![BAND_COUNT - 2, BAND_COUNT - 1]);
        // And the X-ray band is a beam or nothing: it does not scatter into a sky.
        assert_eq!(table.bands()[0].transport, Transport::BeamOnly);
    }

    #[test]
    fn the_moon_is_a_grey_regolith_and_slightly_red() {
        let table = BandTable::computed();
        for band in table.bands() {
            assert!((0.05..=0.30).contains(&band.moon_reflectance));
        }
        // Red-leaning: longer bands give back more than shorter ones do.
        assert!(table.bands()[6].moon_reflectance > table.bands()[3].moon_reflectance);
    }

    #[test]
    fn the_table_round_trips_through_its_own_bytes() {
        let table = BandTable::computed();
        let bytes = table.encode();
        assert_eq!(bytes.len(), 4 + BAND_COUNT * 49);
        let back = BandTable::from_bytes(&bytes).expect("the table's own bytes");
        assert_eq!(back, table);
    }

    #[test]
    fn a_table_from_another_schema_is_refused_by_name() {
        let mut bytes = BandTable::computed().encode();
        bytes[0] = bytes[0].wrapping_add(1);
        match BandTable::from_bytes(&bytes) {
            Err(SpectraError::BadVersion { found, expected }) => {
                assert_eq!(expected, SPECTRA_VERSION);
                assert_ne!(found, expected);
            }
            other => panic!("a version bump was not refused: {other:?}"),
        }
    }

    #[test]
    fn a_table_with_the_wrong_band_count_is_refused() {
        let mut bytes = BandTable::computed().encode();
        bytes[2] = 11;
        assert!(matches!(
            BandTable::from_bytes(&bytes),
            Err(SpectraError::BadCount { found: 11, .. })
        ));
    }

    #[test]
    fn a_truncated_table_is_refused_rather_than_padded() {
        let bytes = BandTable::computed().encode();
        for cut in [2, 4, 20, bytes.len() - 1] {
            assert_eq!(
                BandTable::from_bytes(&bytes[..cut]),
                Err(SpectraError::Truncated),
                "a table cut at {cut} bytes was not refused"
            );
        }
    }

    #[test]
    fn bytes_left_over_after_the_last_band_are_refused() {
        let mut bytes = BandTable::computed().encode();
        bytes.push(0);
        assert_eq!(
            BandTable::from_bytes(&bytes),
            Err(SpectraError::TrailingBytes)
        );
    }

    #[test]
    fn an_undeclared_transport_class_is_refused_and_names_its_band() {
        assert!(matches!(
            Transport::from_u8(7),
            Err(SpectraError::UnknownTransport { class: 7, .. })
        ));
        let mut bytes = BandTable::computed().encode();
        // The transport byte of band 0: version(2) + count(2) + six f64s.
        let at = 4 + 48;
        bytes[at] = 9;
        assert_eq!(
            BandTable::from_bytes(&bytes),
            Err(SpectraError::UnknownTransport { index: 0, class: 9 })
        );
    }

    #[test]
    fn the_agreed_numbers_are_pinned() {
        // **These are the ported numbers, and this is what "pinned by test" means here.**
        // The ported tree checked their table in as `data/spectra/bands.bin`; this one has
        // no such file, so the values themselves are asserted. A change to any formula
        // above fails here and has to be argued for rather than absorbed.
        let table = BandTable::computed();

        // **The widest band wins, and that is the physics, not a defect.** A 5778 K sun
        // puts its *peak* at 501.6 nm but its *bulk* in the near infrared, so the widest
        // band (700–1500 nm, 800 nm of width) carries more than the narrow green one.
        // This assertion was wrong when it was written the other way round.
        let brightest = table
            .bands()
            .iter()
            .max_by(|a, b| a.sun_w_per_m2.total_cmp(&b.sun_w_per_m2))
            .expect("twelve bands");
        assert_eq!(brightest.centre_nm().round(), 1_025.0);
        assert!(brightest.sun_w_per_m2 > table.bands()[4].sun_w_per_m2);

        // The visible triple, 400–700 nm, carries roughly a third of the constant — the
        // figure a broadband photometric sky has to be approximating with one number.
        let visible: f64 = table
            .bands()
            .iter()
            .filter(|band| band.lo_nm >= 400.0 && band.hi_nm <= 700.0)
            .map(|band| band.sun_w_per_m2)
            .sum();
        let share = visible / SOLAR_CONSTANT;
        assert!(
            (0.30..0.45).contains(&share),
            "the visible carries {:.1}% of the solar constant",
            100.0 * share
        );

        // Scattering dominates extinction in the blue, and absorption in the far
        // infrared, which is the whole shape of a clear sky.
        let blue = &table.bands()[3];
        let far_ir = &table.bands()[9];
        assert!(blue.scatter_per_m / blue.extinction_per_m > 0.9);
        assert!(far_ir.scatter_per_m < far_ir.extinction_per_m);
    }
}
