#![forbid(unsafe_code)]
//! The design system: **Material Design 1** with the **Human Interface
//! Guidelines** overlay, and the four-bucket rule for every constant in it.
//!
//! # Why this is one module and not five
//!
//! The four-bucket rule says **every geometry constant is labelled** — *MD1
//! rule* / *MD1 example* / *platform translation* / *project extension* — and
//! that a missing value is **never turned into an invented one**. The rule is
//! only checkable if a number and its label are impossible to separate, so the
//! label travels *inside* [`Constant`] and a test holds every constant in
//! [`CONSTANTS`] to a name, a bucket and a source. Splitting this into
//! `colour.rs`, `layout.rs` and `motion.rs` would keep the rule's *intent* and
//! lose its *enforcement*, which is the trade this repository keeps refusing.
//!
//! # The two things this crate will not do
//!
//! 1. **Name a colour by its value.** A component says
//!    [`Role::SurfaceAlt`], never `#332F45`. The palette is the only place a
//!    channel lives, and the only way to read one out of it is
//!    [`Palette::get`], which **refuses** a role that has no admitted value.
//!    Raw hex in a component is not refused — it is unrepresentable.
//! 2. **Invent a value the record does not have.** Two palette roles have
//!    values in the record (`surfaceAlt #332F45`, and `chrome #1A1A1A`, which
//!    MD1 requires to be **darker than the canvas**). The other six are
//!    **named as unadmitted** and counted by a test, exactly as
//!    `voxel-appearance` counts its project extensions. A screen cannot be
//!    drawn from a palette with holes — so the holes are a measured debt rather
//!    than six plausible-looking swatches.
//!
//! # What the overlay adds
//!
//! HIG contributes the parts MD1 has no opinion about: a **48 dp** minimum
//! target (MD1 says 48 for touch as well, and the two agree, which is recorded
//! rather than exploited), an **8 dp** minimum gap between targets, honouring
//! **`prefers-reduced-motion`** with a *reduced alternative for meaningful
//! motion* rather than merely shortening it, and a contrast floor measured
//! **against the actual blurred backdrop** rather than against an ideal colour.

// ---------------------------------------------------------------------------
// The four-bucket rule
// ---------------------------------------------------------------------------

/// Where a constant's value comes from. Every constant in this crate carries
/// one, because "the design spec said so" is four different statements.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Bucket {
    /// A normative MD1 statement: *"touch targets are 48 dp"*.
    Md1Rule,
    /// A value MD1 offers as an example of its own rule, not as the rule.
    Md1Example,
    /// A number that exists because a platform constrains it, not because a
    /// design chose it — a look-at basis that cannot reach the pole, say.
    PlatformTranslation,
    /// **This project invented it.** The bucket that must be counted, because
    /// it is the one that can grow silently.
    ProjectExtension,
}

impl Bucket {
    pub const ALL: [Bucket; 4] =
        [Bucket::Md1Rule, Bucket::Md1Example, Bucket::PlatformTranslation, Bucket::ProjectExtension];
}

/// A number, the bucket it belongs to, and where it came from. The three are
/// inseparable on purpose.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Constant {
    pub name: &'static str,
    pub value: f64,
    pub bucket: Bucket,
    pub source: &'static str,
}

/// Every numeric constant this crate declares, in one place so the four-bucket
/// rule is **countable** rather than reviewable.
pub const CONSTANTS: &[Constant] = &[
    Constant {
        name: "INK_SECONDARY",
        value: 0.70,
        bucket: Bucket::Md1Rule,
        source: "MD1: the white-ink opacity ladder is 100 / 70 / 50 %, with 12 % for dividers",
    },
    Constant {
        name: "INK_TERTIARY",
        value: 0.50,
        bucket: Bucket::Md1Rule,
        source: "MD1: the white-ink opacity ladder is 100 / 70 / 50 %, with 12 % for dividers",
    },
    Constant {
        name: "INK_DIVIDER",
        value: 0.12,
        bucket: Bucket::Md1Rule,
        source: "MD1: the white-ink opacity ladder is 100 / 70 / 50 %, with 12 % for dividers",
    },
    Constant {
        name: "RADIUS_DP",
        value: 2.0,
        bucket: Bucket::Md1Rule,
        source: "MD1: corner radius is 2 dp",
    },
    Constant {
        name: "MIN_TARGET_DP",
        value: 48.0,
        bucket: Bucket::Md1Rule,
        source: "MD1 and HIG agree: a touch target is at least 48 dp",
    },
    Constant {
        name: "MIN_GAP_DP",
        value: 8.0,
        bucket: Bucket::Md1Rule,
        source: "HIG: targets are separated by at least 8 dp",
    },
    Constant {
        name: "MIN_CONTRAST",
        value: 4.5,
        bucket: Bucket::Md1Rule,
        source: "WCAG 2.1 AA: text contrast ratio at least 4.5:1",
    },
    Constant {
        name: "SURFACE_ALT_CHROMA",
        value: 0.0,
        bucket: Bucket::Md1Example,
        source: "MD1 example value: surfaceAlt is #332F45",
    },
    Constant {
        name: "ELEV_STEP",
        value: 0.05,
        bucket: Bucket::ProjectExtension,
        source: "this project: elevation is a brightness tint of 5 % toward white per level; \
                 MD1 states the *form* of the rule and no magnitude",
    },
    Constant {
        name: "MAX_ELEVATION",
        value: 4.0,
        bucket: Bucket::ProjectExtension,
        source: "this project: five elevation levels (0..4); the record states no ladder length",
    },
    Constant {
        name: "REDUCED_CROSSFADE_MS",
        value: 80.0,
        bucket: Bucket::ProjectExtension,
        source: "this project: the reduced alternative for meaningful motion is an 80 ms crossfade; \
                 HIG requires *an* alternative and names no duration",
    },
    Constant {
        name: "HUD_PANEL_WIDTH_DP",
        value: 300.0,
        bucket: Bucket::ProjectExtension,
        source: "this project: a HUD panel is 300 dp wide — enough for a 14 sp label and a \
                 right-aligned value, at any DPI, without the panel resizing as numbers change. \
                 The record names no HUD and therefore no width, so a panel sized to its own \
                 longest line would be a width that moves when a frame rate changes",
    },
    Constant {
        name: "SPACING_DP",
        value: 8.0,
        bucket: Bucket::Md1Rule,
        source: "MD1: layout sits on an 8 dp grid, so component spacing is a multiple of 8",
    },
    Constant {
        name: "HUD_MARGIN_DP",
        value: 16.0,
        bucket: Bucket::Md1Rule,
        source: "MD1: screen content sits 16 dp from the edge of the screen",
    },
    Constant {
        name: "ROW_HEIGHT_DP",
        value: 48.0,
        bucket: Bucket::Md1Rule,
        source: "MD1: a single-line list row is at least 48 dp tall — the same 48 as the touch target",
    },
    Constant {
        name: "DIVIDER_DP",
        value: 1.0,
        bucket: Bucket::Md1Rule,
        source: "MD1: a divider is a 1 dp line",
    },
    Constant {
        name: "TEXT_SIZE_DP",
        value: 14.0,
        bucket: Bucket::Md1Example,
        source: "MD1 example value: body text is 14 sp",
    },
    Constant {
        name: "TITLE_SIZE_DP",
        value: 20.0,
        bucket: Bucket::Md1Example,
        source: "MD1 example value: a title is 20 sp",
    },
    Constant {
        name: "PANEL_OPACITY",
        value: 0.93,
        bucket: Bucket::ProjectExtension,
        source: "this project: a HUD panel is 93 % opaque. The record names no HUD opacity, so this is \
                 a project value — but it is not chosen by taste. It is the least opacity at which \
                 the whole ink ladder clears the contrast floor over the brightest backdrop an SDR \
                 swapchain can present (white), rounded up to a hundredth, and \
                 `the_panel_opacity_is_the_derived_minimum` fails if it drifts from that",
    },
    Constant {
        name: "BAR_HEIGHT_DP",
        value: 48.0,
        bucket: Bucket::Md1Rule,
        source: "MD1: a single-line row of targets is at least 48 dp tall, and the upper bar is one \
                 such row of tabs and entries (H-7.15: the shell is the upper bar)",
    },
    Constant {
        name: "TITLE_BAR_DP",
        value: 48.0,
        bucket: Bucket::Md1Rule,
        source: "MD1: a window's title bar is a single-line row holding its controls, so it is the \
                 same 48 dp row — the controls inside it meet the 48 dp target floor",
    },
    Constant {
        name: "GLYPH_DP",
        value: 24.0,
        bucket: Bucket::Md1Example,
        source: "MD1 example value: a system icon is drawn at 24 dp inside its 48 dp target",
    },
    Constant {
        name: "BAR_ENTRY_WIDTH_DP",
        value: 112.0,
        bucket: Bucket::ProjectExtension,
        source: "this project: a bar entry is 112 dp wide so the longest tab label fits at 14 sp \
                 without the layout measuring text. The record fixes no bar width; an entry sized \
                 to its own label would move the strip when a title changed, and this width is \
                 counted as an extension — the shell narrows it toward the 48 dp target floor \
                 rather than clipping a target",
    },
];

/// The project extensions, which are the bucket that can grow without anybody
/// noticing. A test pins the count, so adding one is a visible act.
pub fn project_extensions() -> Vec<&'static Constant> {
    CONSTANTS.iter().filter(|c| c.bucket == Bucket::ProjectExtension).collect()
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

/// Why a UI declaration was refused.
///
/// Not a `voxel_model::RefusalKind`. Those ten variants are statements about
/// values *in the world* — a coordinate out of range, a ratio that does not sum,
/// an unadmitted material — and "this button is 47 dp wide" is not one of them.
/// Adding it there would put a layout accident into the world's vocabulary. The
/// same reasoning as `voxel_client::ClientError`, and the same rule: a new class
/// of refusal gets a new variant, here rather than there.
#[derive(Clone, PartialEq, Debug)]
pub enum UiRefusal {
    BelowMinimumTarget { what: &'static str, given: u32, minimum: u32 },
    BelowMinimumGap { given: u32, minimum: u32 },
    EmptyViewWithoutNote { what: &'static str },
    TokenUnadmitted { role: Role },
    ContrastBelowFloor { measured: f64, floor: f64 },
    UnknownElevation { level: u8, maximum: u8 },
    MeaningfulMotionWithoutReducedAlternative { duration_ms: u32 },
    NonFiniteValue { what: &'static str, value: f64 },

    // ---- the pane tree (UI-SPEC §3) -----------------------------------------
    //
    // **Two layout refusals, and they are not appearance refusals.** The eight above are all
    // about how a surface *looks* — contrast, elevation, target size, motion. A pane tree is
    // about how a screen is *divided*, and squeezing either of these into an existing variant
    // would be D-07's collision: one name, two meanings, and a reader of
    // `BelowMinimumTarget` would reasonably conclude it is about touch targets.

    /// A window id appeared in two panes of one arrangement.
    ///
    /// **The reference does not check this, and a data model has to.** `portal_simulator.py`
    /// can render a tree holding one window twice — the two panes both draw it, and the user
    /// sees a window that is in two places. An arrangement is **data** (§3: *"the arrangement
    /// is data … a loadable package"*), and data that contradicts itself is refused rather
    /// than loaded.
    DuplicateWindow { window: String },

    /// A split named a window that is not in the arrangement.
    ///
    /// **The reference falls back to an outer split here, and a data model refuses.** A Qt
    /// widget can be re-parented away between the click and the handler, so `split_pane` with
    /// a `target_pane` that is no longer in the tree does the only thing a UI can do — splits
    /// the window instead. An arrangement has no such race, so a target that is not there is a
    /// **caller that named a window the tree does not hold**, and guessing which pane it meant
    /// is exactly the fabrication this project refuses everywhere else.
    UnknownWindow { window: String },

    /// A split cannot give every child the 40 × 40 floor.
    ///
    /// **Carries the arithmetic, not a verdict.** `span / count < floor` is the whole rule and
    /// a reader can check it. §3's warning is that *"without it, nested splitters'
    /// `minimumSizeHint`s accumulate and the layout locks"* — and Qt discovers that while
    /// laying out, which is **after** the window is on screen and stuck. This is discovered
    /// before anything is drawn, with the numbers.
    PaneBelowFloor { span: u32, count: u32, floor: u32, axis: Axis },

    /// A declaration's `theme-role` names a string this tree has no role for.
    ///
    /// **Refused by name, never defaulted to a surface nobody chose.** The
    /// vocabulary is closed and declared in [`Role::THEME_ROLE_NAMES`]; a package
    /// that invents a name is a package whose windows this shell cannot theme,
    /// and drawing them on the nearest role would be the plausible substitution
    /// this project refuses everywhere else.
    UnknownThemeRole { theme_role: String },

    /// A panel is taller than the pane it was placed in.
    ///
    /// `UNIFIED_DESIGN.md` §5.7: *"a [surface] may not reserve space with a constant and then
    /// place content with a different one; the two numbers must be the same number."* The pane
    /// tree's own floor is about **splits**; this is about the content a pane hosts, and a panel
    /// that spills past its window's declared rect is a layout that does not fit. The numbers
    /// travel with the refusal so a reader can see which one is wrong.
    ContentExceedsPane { window: String, panel_dp: f64, pane_dp: f64 },
}

/// Which of the two screen axes a measurement or constraint is about.
///
/// **A type rather than a `bool`, and it lives here rather than in the pane tree** because
/// [`UiRefusal::PaneBelowFloor`] has to name it. A `constrained_along_width: bool` at four
/// call sites is four chances to pass `true` where `false` was meant, and the compiler cannot
/// see it — D-07's shape, at the smallest possible scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    Width,
    Height,
}

impl Axis {
    pub const ALL: [Axis; 2] = [Axis::Width, Axis::Height];

    pub const fn name(self) -> &'static str {
        match self {
            Axis::Width => "width",
            Axis::Height => "height",
        }
    }
}

pub type UiResult<T> = Result<T, UiRefusal>;

impl std::fmt::Display for UiRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UiRefusal::BelowMinimumTarget { what, given, minimum } => {
                write!(f, "the {what} target is {given} dp; the floor is {minimum}")
            }
            UiRefusal::BelowMinimumGap { given, minimum } => {
                write!(f, "targets are {given} dp apart; the floor is {minimum}")
            }
            UiRefusal::EmptyViewWithoutNote { what } => {
                write!(f, "the {what} view is empty and carries no note")
            }
            UiRefusal::TokenUnadmitted { role } => {
                write!(f, "{role:?} has no admitted value in the palette")
            }
            UiRefusal::ContrastBelowFloor { measured, floor } => {
                write!(f, "contrast is {measured:.2}:1; the floor is {floor:.2}:1")
            }
            UiRefusal::UnknownElevation { level, maximum } => {
                write!(f, "elevation {level} is above the ladder's {maximum}")
            }
            UiRefusal::MeaningfulMotionWithoutReducedAlternative { duration_ms } => {
                write!(
                    f,
                    "a meaningful {duration_ms} ms motion has no reduced alternative"
                )
            }
            UiRefusal::NonFiniteValue { what, value } => {
                write!(f, "{what} is not finite: {value}")
            }
            UiRefusal::DuplicateWindow { window } => {
                write!(f, "window {window:?} is in two panes of one arrangement")
            }
            UiRefusal::UnknownWindow { window } => {
                write!(f, "window {window:?} is not in this arrangement, so there is no pane to split")
            }
            UiRefusal::PaneBelowFloor { span, count, floor, axis } => {
                let axis = match axis {
                    Axis::Width => "width",
                    Axis::Height => "height",
                };
                write!(
                    f,
                    "{span} dp of {axis} split {count} ways is {} dp each; the floor is {floor}",
                    span / count
                )
            }
            UiRefusal::UnknownThemeRole { theme_role } => {
                write!(
                    f,
                    "the theme role {theme_role:?} is not one of the declared names: {}",
                    Role::THEME_ROLE_NAMES
                        .iter()
                        .map(|(name, _)| *name)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            UiRefusal::ContentExceedsPane { window, panel_dp, pane_dp } => {
                write!(
                    f,
                    "window {window:?} places a {panel_dp:.0} dp panel in a {pane_dp:.0} dp \
                     pane; the two numbers have to be the same number"
                )
            }
        }
    }
}

impl std::error::Error for UiRefusal {}

// ---------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------

/// An 8-bit sRGB triple. Constructible only inside this crate's palette, so a
/// component that wants a colour has to name a [`Role`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    fn channel_to_linear(c: u8) -> f64 {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    fn channel_from_linear(linear: f64) -> u8 {
        let c = if linear <= 0.0031308 {
            linear * 12.92
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        (c * 255.0).round().clamp(0.0, 255.0) as u8
    }

    /// The three channels in **linear** space. Every composite in this crate
    /// happens here and not on the 8-bit values, because blending two sRGB
    /// triples by averaging their bytes is a different (and wrong) colour.
    pub fn linear(self) -> [f64; 3] {
        [
            Self::channel_to_linear(self.r),
            Self::channel_to_linear(self.g),
            Self::channel_to_linear(self.b),
        ]
    }

    pub fn from_linear(linear: [f64; 3]) -> Rgb {
        Rgb::new(
            Self::channel_from_linear(linear[0]),
            Self::channel_from_linear(linear[1]),
            Self::channel_from_linear(linear[2]),
        )
    }

    /// `self` over `backdrop`, with `alpha` of `self` showing. Linear-space
    /// compositing, which is what a pipeline with an sRGB target does.
    pub fn blend_over(self, backdrop: Rgb, alpha: f64) -> Rgb {
        let a = alpha.clamp(0.0, 1.0);
        let over = self.linear();
        let under = backdrop.linear();
        let mut out = [0.0; 3];
        for i in 0..3 {
            out[i] = over[i] * a + under[i] * (1.0 - a);
        }
        Rgb::from_linear(out)
    }

    /// The same composite done on the 8-bit values instead.
    ///
    /// Not used to draw anything — used to check that a legibility conclusion
    /// does not depend on this choice. Whether a renderer blends in linear or
    /// in the encoded space is an implementation detail of the renderer; which
    /// ink steps are readable over a panel is a design claim, and it should not
    /// flip when that detail changes.
    pub fn blend_over_encoded(self, backdrop: Rgb, alpha: f64) -> Rgb {
        let a = alpha.clamp(0.0, 1.0);
        let mix = |over: u8, under: u8| {
            ((over as f64) * a + (under as f64) * (1.0 - a)).round().clamp(0.0, 255.0) as u8
        };
        Rgb::new(
            mix(self.r, backdrop.r),
            mix(self.g, backdrop.g),
            mix(self.b, backdrop.b),
        )
    }

    /// Relative luminance, **the same Rec. 709 triple `voxel-appearance` uses**,
    /// so a luminance here and a luminance in the shading model cannot disagree
    /// about what luminance means.
    pub fn luminance(self) -> f64 {
        0.2126 * Self::channel_to_linear(self.r)
            + 0.7152 * Self::channel_to_linear(self.g)
            + 0.0722 * Self::channel_to_linear(self.b)
    }

    /// **OKLab** — the perceptually uniform space `UNIFIED_DESIGN.md` §3.2 says
    /// palette tokens should be defined in, and the space §3.2.1's chroma
    /// ceilings are stated in.
    ///
    /// Björn Ottosson's matrices, on linear sRGB. `(L, a, b)`, with `L` 0..1.
    pub fn oklab(self) -> (f64, f64, f64) {
        let [r, g, b] = self.linear();
        let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
        let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
        let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
        let (l_, m_, s_) = (l.cbrt(), m.cbrt(), s.cbrt());
        let big_l = 0.210_454_255_3 * l_ + 0.793_617_785_0 * m_ - 0.004_072_046_8 * s_;
        let a = 1.977_998_495_1 * l_ - 2.428_592_205_0 * m_ + 0.450_593_709_9 * s_;
        let b = 0.025_904_037_1 * l_ + 0.782_771_766_2 * m_ - 0.808_675_766_0 * s_;
        (big_l, a, b)
    }

    /// OKLCH's **C**: how far the colour is from the neutral axis.
    ///
    /// This is the number §3.2.1 puts a ceiling on, per mechanism — so it is
    /// measured here rather than described in a comment.
    pub fn chroma(self) -> f64 {
        let (_, a, b) = self.oklab();
        (a * a + b * b).sqrt()
    }

    /// The WCAG 2.1 contrast ratio, 1.0 for identical colours and 21.0 for
    /// black against white. Always computed from **two actual colours**, never
    /// from a foreground and an ideal background — see
    /// [`contrast_over_backdrop`].
    pub fn contrast(self, other: Rgb) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        let (hi, lo) = if a > b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// MD1 elevation is a **brightness tint**: a raised surface is lighter, not
    /// shadowed. `level` above [`max_elevation`] is refused rather than clamped,
    /// because a clamped elevation is a surface that lies about how high it is.
    pub fn elevated(self, level: u8) -> UiResult<Rgb> {
        let step = constant("ELEV_STEP");
        let max = max_elevation();
        if level > max {
            return Err(UiRefusal::UnknownElevation { level, maximum: max });
        }
        let t = step * level as f64;
        let mix = |c: u8| ((c as f64) + (255.0 - c as f64) * t).round().clamp(0.0, 255.0) as u8;
        Ok(Rgb::new(mix(self.r), mix(self.g), mix(self.b)))
    }
}

fn constant(name: &str) -> f64 {
    CONSTANTS
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.value)
        .unwrap_or(0.0)
}

pub fn max_elevation() -> u8 {
    constant("MAX_ELEVATION") as u8
}

/// The MD1 **white-ink opacity ladder**. Four steps, and the divider step is
/// the one that is not for text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ink {
    /// 100 % — primary text.
    Full,
    /// 70 % — secondary text.
    Secondary,
    /// 50 % — tertiary text and disabled **labels**.
    Tertiary,
    /// 12 % — dividers. Not a text weight: a line.
    Divider,
}

impl Ink {
    pub const ALL: [Ink; 4] = [Ink::Full, Ink::Secondary, Ink::Tertiary, Ink::Divider];

    pub fn opacity(self) -> f64 {
        match self {
            Ink::Full => 1.0,
            Ink::Secondary => constant("INK_SECONDARY"),
            Ink::Tertiary => constant("INK_TERTIARY"),
            Ink::Divider => constant("INK_DIVIDER"),
        }
    }

    /// Whether this step may carry a **label**. The divider step may not, and
    /// that is a rule rather than a preference — a 12 % glyph is not text, it
    /// is a smudge, and calling it text is how an unreadable screen passes a
    /// review.
    pub fn is_text(self) -> bool {
        !matches!(self, Ink::Divider)
    }

    /// The next step down the ladder. Used for the **label** of a readout, which
    /// MD1 sets one step quieter than the value it names. The bottom step
    /// saturates rather than wrapping, so a label can be quieter than its value
    /// but never louder.
    pub fn dimmer(self) -> Ink {
        match self {
            Ink::Full => Ink::Secondary,
            Ink::Secondary => Ink::Tertiary,
            Ink::Tertiary => Ink::Tertiary,
            Ink::Divider => Ink::Divider,
        }
    }

    /// What this step is when it is drawn: the ink's colour and its alpha. The
    /// renderer receives the pair and does no design thinking of its own.
    pub fn paint(self) -> Paint {
        Paint { rgb: WHITE, alpha: self.opacity() }
    }

    /// The colour this step **is** once it sits on `surface`. The pixel a player
    /// actually sees, which is the only thing a contrast ratio may be measured
    /// against.
    pub fn composited_over(self, surface: Rgb) -> Rgb {
        WHITE.blend_over(surface, self.opacity())
    }

    /// The same, with the composite done in the encoded space. Only a test uses
    /// this, and only to show the conclusion does not hinge on the choice.
    pub fn composited_over_encoded(self, surface: Rgb) -> Rgb {
        WHITE.blend_over_encoded(surface, self.opacity())
    }
}

/// MD1's ink ladder is a **white** ladder: every step is the same white at a
/// different alpha. Named once, so no component can introduce a second ink
/// colour by accident.
pub const WHITE: Rgb = Rgb::new(0xFF, 0xFF, 0xFF);

/// A colour and the alpha it is drawn with. The unit the renderer consumes —
/// it is not told *why* a pixel is that colour, so a component cannot repaint
/// itself outside the palette.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Paint {
    pub rgb: Rgb,
    pub alpha: f64,
}

impl Paint {
    pub fn opaque(rgb: Rgb) -> Self {
        Paint { rgb, alpha: 1.0 }
    }

    /// The colour this paint becomes over a backdrop — the pixel a player sees.
    pub fn over(self, backdrop: Rgb) -> Rgb {
        self.rgb.blend_over(backdrop, self.alpha)
    }

    /// This paint's contrast against a backdrop, which is the HIG rule: the
    /// measurement is against the **composited** result, never against the
    /// colour the paint was authored with.
    pub fn contrast_over(self, backdrop: Rgb) -> UiResult<f64> {
        contrast_over_backdrop(self.over(backdrop), backdrop)
    }
}

/// The eight palette roles. The **names** are a project extension; two of the
/// eight have values in the record and six do not.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Role {
    Canvas,
    Surface,
    SurfaceAlt,
    Chrome,
    Primary,
    Secondary,
    Accent,
    Divider,
}

impl Role {
    pub const ALL: [Role; 8] = [
        Role::Canvas,
        Role::Surface,
        Role::SurfaceAlt,
        Role::Chrome,
        Role::Primary,
        Role::Secondary,
        Role::Accent,
        Role::Divider,
    ];

    /// The identifiers a package's `theme-role` row may carry, and the role each
    /// names.
    ///
    /// **`hud` is a theme, not a role, and it maps to the HUD's own body
    /// surface.** H-9.8 makes the pinned windows one layer of one window set, and
    /// the panels hosted in that layer declare their own roles — so the pinned
    /// window's body is MD1's chrome, which is what the HUD has always been drawn
    /// on. A package that writes a name outside this table is refused by name by
    /// [`Role::from_theme_role`], not defaulted to a surface nobody chose.
    pub const THEME_ROLE_NAMES: [(&'static str, Role); 9] = [
        ("canvas", Role::Canvas),
        ("surface", Role::Surface),
        ("surface-alt", Role::SurfaceAlt),
        ("chrome", Role::Chrome),
        ("hud", Role::Chrome),
        ("primary", Role::Primary),
        ("secondary", Role::Secondary),
        ("accent", Role::Accent),
        ("divider", Role::Divider),
    ];

    /// The role a declaration's `theme-role` names, or a refusal naming the
    /// string. Nothing is inferred from position or hue; the table above is the
    /// whole vocabulary.
    pub fn from_theme_role(text: &str) -> UiResult<Role> {
        Self::THEME_ROLE_NAMES
            .iter()
            .find(|(name, _)| *name == text)
            .map(|(_, role)| *role)
            .ok_or_else(|| UiRefusal::UnknownThemeRole { theme_role: text.to_string() })
    }
}

/// One roster slot: a value with its bucket, or a **named** absence.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Slot {
    Declared { rgb: Rgb, bucket: Bucket, source: &'static str },
    Unadmitted { why: &'static str },
}

/// The palette. Dark theme only, and the declared values are the record's two
/// plus the body surface this tree had to derive to draw a window on.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Palette {
    slots: [Slot; 8],
}

impl Palette {
    /// The palette as the record leaves it, plus the one value the shell cannot
    /// mount without: [`Role::Surface`].
    ///
    /// **The derivation is stated on the slot rather than implied by the code.**
    /// The record names chrome and the alternative surface; it names no window
    /// body, so the body is this tree's own value — an author choice under
    /// `UNIFIED_DESIGN.md` §3.2.1, within the polymer ceiling, and pinned by the
    /// test that measures the whole ink ladder on it over a white world.
    pub fn declared() -> Self {
        let unadmitted = Slot::Unadmitted {
            why: "the design record names no value for this role",
        };
        let mut slots = [unadmitted; 8];
        slots[Role::SurfaceAlt as usize] = Slot::Declared {
            rgb: Rgb::new(0x33, 0x2F, 0x45),
            bucket: Bucket::Md1Example,
            source: "MD1 example: surfaceAlt is #332F45",
        };
        slots[Role::Chrome as usize] = Slot::Declared {
            rgb: Rgb::new(0x1A, 0x1A, 0x1A),
            bucket: Bucket::Md1Example,
            source: "MD1 example: chrome is #1A1A1A, and it must be darker than the canvas",
        };
        slots[Role::Surface as usize] = Slot::Declared {
            rgb: Rgb::new(0x1B, 0x19, 0x22),
            bucket: Bucket::ProjectExtension,
            source: "this project: the window body on the dark theme. The record names no body \
                     value, so this is an author choice under UNIFIED_DESIGN §3.2.1 — a polymer \
                     (ceiling 0.050 chroma). It is not picked by taste: it is a dark body whose \
                     whole ink ladder (100/70/50 %) clears 4.5:1 over a white world at \
                     PANEL_OPACITY, which `the_window_body_carries_the_whole_ink_ladder_over_white` \
                     measures, and `every_admitted_role_states_a_material_and_stays_under_its_ceiling` \
                     measures its chroma against the polymer ceiling",
        };
        Palette { slots }
    }

    /// The value for a role, or **a refusal naming the role**. Never a default:
    /// a grey that stands in for an unadmitted surface is indistinguishable
    /// from a chosen one.
    pub fn get(&self, role: Role) -> UiResult<Rgb> {
        match self.slots[role as usize] {
            Slot::Declared { rgb, .. } => Ok(rgb),
            Slot::Unadmitted { .. } => Err(UiRefusal::TokenUnadmitted { role }),
        }
    }

    pub fn slot(&self, role: Role) -> Slot {
        self.slots[role as usize]
    }

    /// The roles with no admitted value — the measured debt.
    pub fn unadmitted(&self) -> Vec<Role> {
        Role::ALL
            .into_iter()
            .filter(|r| matches!(self.slots[*r as usize], Slot::Unadmitted { .. }))
            .collect()
    }

    /// How many roles *can* be drawn. A screen needs all eight; this is the
    /// number that has to reach eight before a UI is drawable.
    pub fn drawable_roles(&self) -> usize {
        Role::ALL.len() - self.unadmitted().len()
    }

    /// MD1's own rule about its own example: chrome is **darker** than the
    /// canvas. This can only be checked when both have values, so it reports
    /// **which role is blocking** rather than passing vacuously.
    pub fn chrome_is_darker_than_canvas(&self) -> UiResult<bool> {
        let chrome = self.get(Role::Chrome)?;
        let canvas = self.get(Role::Canvas)?;
        Ok(chrome.luminance() < canvas.luminance())
    }
}

/// **The mechanism a declared colour comes from, and the chroma it cannot
/// exceed** — `UNIFIED_DESIGN.md` §3.2.1, carried in as this tree's own table.
///
/// A material's colour is produced by a named physical process, and past its
/// ceiling the colour has stopped being that material: a *blue metal body* is a
/// pigment coating on metal — a different material with its own name — not a
/// stronger metal. So the ceiling is checked **arithmetically**, in OKLCH, by a
/// test rather than reviewed by eye.
///
/// **This table is this repository's re-derivation of the doctrine's**, and it
/// exists because the shell draws surfaces: a role with a value and no stated
/// mechanism is a colour nobody can account for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Material {
    Metal,
    Glass,
    Ceramic,
    Polymer,
    Paper,
    Road,
    Enamel,
}

impl Material {
    pub const ALL: [Material; 7] = [
        Material::Metal,
        Material::Glass,
        Material::Ceramic,
        Material::Polymer,
        Material::Paper,
        Material::Road,
        Material::Enamel,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Material::Metal => "metal",
            Material::Glass => "glass",
            Material::Ceramic => "ceramic",
            Material::Polymer => "polymer",
            Material::Paper => "paper",
            Material::Road => "road",
            Material::Enamel => "enamel",
        }
    }

    /// The physical process the family is made by. §3.2.1 states the ceiling
    /// *because* of the mechanism, so the two travel together.
    pub const fn mechanism(self) -> &'static str {
        match self {
            Material::Metal => "anodised film — an oxide layer on the substrate",
            Material::Glass => "body-tinted — colour held in the glass itself",
            Material::Ceramic => "fired glaze over the body",
            Material::Polymer => "pigmented resin, colour compounded in",
            Material::Paper => "dyed stock, pigment in the sheet",
            Material::Road => "aggregate — asphalt and stone, near-neutral",
            Material::Enamel => "painted colour — a pigmented coating",
        }
    }

    /// The most OKLCH chroma this mechanism can carry before the colour has
    /// become a different material.
    pub const fn ceiling(self) -> f64 {
        match self {
            Material::Metal => 0.060,
            Material::Glass => 0.100,
            Material::Ceramic => 0.050,
            Material::Polymer => 0.050,
            Material::Paper => 0.060,
            Material::Road => 0.040,
            Material::Enamel => 0.190,
        }
    }
}

/// Which mechanism each **admitted** role's value comes from.
///
/// `None` for a role the palette has not admitted, and the correspondence is
/// checked **both ways**: a value without a mechanism fails the test named above,
/// and a mechanism without a value fails it too — a mechanism with no colour
/// decides nothing.
pub const fn material_of(role: Role) -> Option<Material> {
    match role {
        Role::Surface | Role::SurfaceAlt => Some(Material::Polymer),
        Role::Chrome => Some(Material::Metal),
        _ => None,
    }
}

/// MD1 is a **dark theme** in this project, and that is a ruling rather than a
/// preference. A light variant would be a second palette, and a second palette
/// is a second set of unadmitted roles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Dark,
}

impl Theme {
    pub const ONLY: Theme = Theme::Dark;
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// A rectangle in **device-independent pixels**. Layout is dp; only the
/// renderer knows about physical pixels, so every rule here survives a DPI
/// change without being restated.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width_dp: f64,
    pub height_dp: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width_dp: f64, height_dp: f64) -> Self {
        Rect { x, y, width_dp, height_dp }
    }

    /// The MD1 floor is on **both** dimensions, because a 200 × 20 strip is not
    /// a target a thumb can hit.
    pub fn check_target(&self, what: &'static str) -> UiResult<()> {
        let minimum = constant("MIN_TARGET_DP") as u32;
        for given in [self.width_dp, self.height_dp] {
            if !given.is_finite() {
                return Err(UiRefusal::NonFiniteValue { what, value: given });
            }
            if given < minimum as f64 {
                return Err(UiRefusal::BelowMinimumTarget {
                    what,
                    given: given as u32,
                    minimum,
                });
            }
        }
        Ok(())
    }

    pub fn right(&self) -> f64 {
        self.x + self.width_dp
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height_dp
    }

    /// Whether a point is inside the rect — **half-open on the far edges**, so two targets
    /// that touch share an edge and a click on the seam belongs to exactly one of them.
    /// Which one is decided by the caller's draw order, not by this predicate.
    pub fn contains(&self, at: (f64, f64)) -> bool {
        at.0 >= self.x && at.0 < self.right() && at.1 >= self.y && at.1 < self.bottom()
    }

    /// The gap to another target: zero or more when they do not overlap, and
    /// **negative** when they do — which HIG forbids, because a target a thumb
    /// cannot separate from its neighbour is a target that gets mis-hit.
    pub fn gap(&self, other: &Rect) -> f64 {
        let dx = (other.x - self.right()).max(self.x - other.right());
        let dy = (other.y - self.bottom()).max(self.y - other.bottom());
        dx.max(dy)
    }
}

/// The HIG separation rule between targets.
pub fn check_gap(a: &Rect, b: &Rect) -> UiResult<()> {
    let gap = a.gap(b);
    let minimum = constant("MIN_GAP_DP");
    if gap < minimum {
        return Err(UiRefusal::BelowMinimumGap {
            given: gap.max(0.0) as u32,
            minimum: minimum as u32,
        });
    }
    Ok(())
}

/// MD1's corner radius. A single value, so a rounded rectangle cannot quietly
/// acquire a second one.
pub fn radius_dp() -> f64 {
    constant("RADIUS_DP")
}

/// The opacity a HUD panel is drawn at. Derived, not chosen — see
/// [`least_opacity_keeping_the_ladder_legible`].
pub fn panel_opacity() -> f64 {
    constant("PANEL_OPACITY")
}

/// The width every HUD panel is drawn at. Fixed, so a panel does not resize
/// when a readout's digits change — a HUD that jumps when the frame rate goes
/// from 99 to 100 is a HUD that moves under the eye.
pub fn hud_panel_width_dp() -> f64 {
    constant("HUD_PANEL_WIDTH_DP")
}

/// The upper bar's height — one MD1 row of targets (H-7.15: the shell *is* the
/// upper bar).
pub fn bar_height_dp() -> f64 {
    constant("BAR_HEIGHT_DP")
}

/// A window's title bar: one row, holding that window's controls.
pub fn title_bar_dp() -> f64 {
    constant("TITLE_BAR_DP")
}

/// A control glyph's size, inside the lane that is its target.
pub fn glyph_dp() -> f64 {
    constant("GLYPH_DP")
}

/// How wide one bar entry is drawn, before a narrow viewport narrows it toward
/// [`min_target_dp`].
pub fn bar_entry_width_dp() -> f64 {
    constant("BAR_ENTRY_WIDTH_DP")
}

/// The touch-target floor, in dp, for the layers that lay out targets
/// themselves.
pub fn min_target_dp() -> f64 {
    constant("MIN_TARGET_DP")
}

/// The layout grid step a component insets its content by.
pub fn spacing_dp() -> f64 {
    constant("SPACING_DP")
}

/// A single-line row: the height a readout or a list row occupies.
pub fn row_height_dp() -> f64 {
    constant("ROW_HEIGHT_DP")
}

/// Body text's size. MD1's example value, used wherever a component draws a
/// readout or a label.
pub fn text_size_dp() -> f64 {
    constant("TEXT_SIZE_DP")
}

/// A title's size — a panel's or a window's. MD1's example value.
pub fn title_size_dp() -> f64 {
    constant("TITLE_SIZE_DP")
}

// ---------------------------------------------------------------------------
// Components and their states
// ---------------------------------------------------------------------------

/// The four interaction states MD1 names, **plus empty** — the state a screen
/// with nothing to show is in, which MD1 does not name and every real UI has.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum State {
    Normal,
    Hover,
    Pressed,
    Disabled,
    Empty,
}

impl State {
    pub const ALL: [State; 5] =
        [State::Normal, State::Hover, State::Pressed, State::Disabled, State::Empty];
}

/// How one state *looks*. The pair `(ink, tint)` is the whole appearance of a
/// flat MD1 surface.
///
/// The match is **exhaustive and has no wildcard arm** on purpose: adding a
/// state to [`State`] will not compile until somebody decides what it looks
/// like. A `_ =>` here is how "we forgot to style disabled" becomes a button
/// that looks enabled.
pub fn appearance(state: State) -> (Ink, f64) {
    match state {
        State::Normal => (Ink::Full, 0.0),
        State::Hover => (Ink::Full, constant("ELEV_STEP")),
        State::Pressed => (Ink::Full, constant("ELEV_STEP") * 2.0),
        State::Disabled => (Ink::Tertiary, 0.0),
        State::Empty => (Ink::Secondary, 0.0),
    }
}

/// A piece of the UI. It names roles, never colours.
#[derive(Clone, PartialEq, Debug)]
pub struct Component {
    pub what: &'static str,
    pub role: Role,
    pub state: State,
    pub bounds: Rect,
    /// The text an empty view carries **instead of being blank**, and the text a
    /// non-empty one may carry or not.
    pub note: Option<String>,
}

impl Component {
    /// The only way to build an empty component, and it **refuses a note that
    /// says nothing**. "Empty views get a text note not a blank pane" is only a
    /// rule if a whitespace note is not a note.
    pub fn empty(what: &'static str, role: Role, bounds: Rect, note: &str) -> UiResult<Self> {
        if note.trim().is_empty() {
            return Err(UiRefusal::EmptyViewWithoutNote { what });
        }
        Ok(Component {
            what,
            role,
            state: State::Empty,
            bounds,
            note: Some(note.to_string()),
        })
    }

    pub fn normal(what: &'static str, role: Role, bounds: Rect) -> Self {
        Component { what, role, state: State::Normal, bounds, note: None }
    }

    /// Everything a component has to satisfy before it is drawn.
    pub fn validate(&self) -> Vec<UiRefusal> {
        let mut out = Vec::new();
        if let Err(e) = self.bounds.check_target(self.what) {
            out.push(e);
        }
        if self.state == State::Empty {
            match self.note.as_ref() {
                None => out.push(UiRefusal::EmptyViewWithoutNote { what: self.what }),
                Some(note) if note.trim().is_empty() => {
                    out.push(UiRefusal::EmptyViewWithoutNote { what: self.what })
                }
                Some(_) => {}
            }
        }
        // A divider may carry no label, so a component asking to draw text in
        // the divider role is refused rather than dimmed into unreadability.
        if self.role == Role::Divider && self.note.is_some() {
            out.push(UiRefusal::TokenUnadmitted { role: Role::Divider });
        }
        out
    }

    /// The ink step to draw the label in, or a refusal when the role may not
    /// carry a label at all.
    pub fn ink(&self) -> UiResult<Ink> {
        let (ink, _) = appearance(self.state);
        if !ink.is_text() {
            return Err(UiRefusal::ContrastBelowFloor { measured: 0.0, floor: 0.0 });
        }
        Ok(ink)
    }

    pub fn surface_tint(&self) -> f64 {
        appearance(self.state).1
    }
}

// ---------------------------------------------------------------------------
// Motion
// ---------------------------------------------------------------------------

/// What a reduced-motion preference means for one animation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reduced {
    /// Drop it. Correct for motion that carries no information.
    Omit,
    /// Replace it with a short crossfade, which keeps the *change* visible
    /// without carrying it on movement.
    Crossfade,
}

/// Whether the motion is carrying meaning or merely decorating.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MotionKind {
    /// Decoration. Dropping it loses nothing, so `Omit` is always acceptable.
    Decorative,
    /// The motion *is* the information — a panel arriving from the edge says
    /// where it came from. HIG requires a **reduced alternative**, not merely a
    /// shortening, because a shortened slide still moves.
    Meaningful,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Motion {
    pub kind: MotionKind,
    pub duration_ms: u32,
    pub reduced: Option<Reduced>,
}

impl Motion {
    pub fn decorative(duration_ms: u32) -> Self {
        Motion { kind: MotionKind::Decorative, duration_ms, reduced: None }
    }

    pub fn meaningful(duration_ms: u32, reduced: Reduced) -> Self {
        Motion { kind: MotionKind::Meaningful, duration_ms, reduced: Some(reduced) }
    }

    pub fn validate(&self) -> Vec<UiRefusal> {
        match (self.kind, self.reduced) {
            (MotionKind::Meaningful, None) => vec![
                UiRefusal::MeaningfulMotionWithoutReducedAlternative { duration_ms: self.duration_ms },
            ],
            _ => Vec::new(),
        }
    }

    /// What actually plays, given the preference. A decorative motion resolves
    /// to `None` — dropped — because dropping decoration is what the preference
    /// asks for.
    pub fn effective_ms(&self, prefers_reduced: bool) -> Option<u32> {
        if !prefers_reduced {
            return Some(self.duration_ms);
        }
        match self.reduced {
            Some(Reduced::Crossfade) => Some(constant("REDUCED_CROSSFADE_MS") as u32),
            Some(Reduced::Omit) | None => match self.kind {
                MotionKind::Decorative => None,
                // Refused by `validate`, so it is never drawn either way. The
                // honest answer to "what plays" is "nothing", not the full
                // animation with a warning nobody reads.
                MotionKind::Meaningful => None,
            },
        }
    }
}

/// HIG requires that **nothing is visible only under the finger**. A tooltip
/// whose only trigger is a touch is invisible to a keyboard, and this is the
/// check for it.
pub fn tooltip_is_reachable_without_touch(has_keyboard_focus_path: bool) -> UiResult<()> {
    if has_keyboard_focus_path {
        Ok(())
    } else {
        Err(UiRefusal::EmptyViewWithoutNote {
            what: "a tooltip reachable only by touch",
        })
    }
}

/// The contrast floor applied to a **real backdrop**.
///
/// HIG's rule is stronger than WCAG's default case: a translucent panel sitting
/// over a moving world has no fixed background colour, so the ratio has to be
/// measured against what is actually behind it. `backdrop` is the colour the
/// text will really sit on — the blurred, blended result — and not the surface
/// the panel was authored against.
pub fn contrast_over_backdrop(ink: Rgb, backdrop: Rgb) -> UiResult<f64> {
    let measured = ink.contrast(backdrop);
    let floor = constant("MIN_CONTRAST");
    if measured < floor {
        return Err(UiRefusal::ContrastBelowFloor { measured, floor });
    }
    Ok(measured)
}

/// The ink steps that can carry a **label** over `panel` sitting at `opacity`
/// on top of `worst_backdrop`.
///
/// This is the function that turns the ladder from a convention into a
/// measurement. A translucent panel over a moving world has no fixed
/// background, so "secondary ink is readable" is not a fact about the ink — it
/// is a fact about the ink *and* the world behind it. Asked with the brightest
/// backdrop the world can present, the answer is the worst case, which is the
/// only case worth designing to.
///
/// The divider step is never returned: it is not text, and this is where that
/// stops being a comment.
pub fn legible_steps(panel: Rgb, opacity: f64, worst_backdrop: Rgb) -> Vec<Ink> {
    let backdrop = panel.blend_over(worst_backdrop, opacity);
    Ink::ALL
        .into_iter()
        .filter(|ink| ink.is_text())
        .filter(|ink| contrast_over_backdrop(ink.composited_over(backdrop), backdrop).is_ok())
        .collect()
}

/// The least panel opacity, to a thousandth, at which **every** text step of the
/// ladder clears the floor over the worst backdrop.
///
/// This is where `PANEL_OPACITY` comes from. It is a project
/// extension — the record names no HUD opacity — but its *value* is derived from
/// the contrast floor, and the test that derives it is the test that pins it. An
/// opacity that is merely pleasing and one that is sufficient look identical in a
/// diff, and only one of them is a decision.
pub fn least_opacity_keeping_the_ladder_legible(panel: Rgb, worst_backdrop: Rgb) -> f64 {
    let steps: Vec<Ink> = Ink::ALL.into_iter().filter(|i| i.is_text()).collect();
    for thousandths in 0..=1000u32 {
        let alpha = thousandths as f64 / 1000.0;
        let backdrop = panel.blend_over(worst_backdrop, alpha);
        if steps
            .iter()
            .all(|ink| contrast_over_backdrop(ink.composited_over(backdrop), backdrop).is_ok())
        {
            return alpha;
        }
    }
    1.0
}

// ---------------------------------------------------------------------------
// Chrome glyphs
// ---------------------------------------------------------------------------

/// A window control a title bar can carry. The order is the order the controls
/// sit in the title bar, and it is the only order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlyphAction {
    Minimize,
    Maximize,
    Close,
}

impl GlyphAction {
    pub const ALL: [GlyphAction; 3] = [
        GlyphAction::Minimize,
        GlyphAction::Maximize,
        GlyphAction::Close,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            GlyphAction::Minimize => "minimize",
            GlyphAction::Maximize => "maximize",
            GlyphAction::Close => "close",
        }
    }
}

/// **This tree's own glyphs, drawn rather than loaded.**
///
/// H-15.6 refuses the old icon *substrate* — the traced `materialicons` marks, the
/// seven-family material matrix, the corpus ladder over the MD1 / TouchWiz / iOS 6
/// sets — and keeps the rule that came with the dirty shell: *the hit region, the
/// drawn glyph and the declared lane are one rectangle*. So a control is one
/// declared mark at [`GLYPH_DP`] inside its 48 dp lane, and there is no icon asset
/// path for a component to draw from.
///
/// The mark is a glyph from the same shaping engine the HUD's text uses, which is
/// why the source line says *drawn*: the shaper resolves it, the atlas rasterises
/// it, and no file outside this repository is consulted. **The cost, stated:** a
/// host font without the mark would rasterise the notdef box, so the drawn-glyph
/// test measures ink rather than trusting the codepoint.
pub struct Glyph {
    pub action: GlyphAction,
    pub text: &'static str,
    pub source: &'static str,
}

/// Every control mark, in title-bar order. A control the layout policy does not
/// permit is **absent**, never drawn disabled (HIG: absence is preferred over a
/// control that looks like a grant).
pub const GLYPHS: [Glyph; 3] = [
    Glyph {
        action: GlyphAction::Minimize,
        text: "\u{2013}",
        source: "this tree: the minimise mark is a 24 dp en dash",
    },
    Glyph {
        action: GlyphAction::Maximize,
        text: "\u{25A1}",
        source: "this tree: the maximise mark is a 24 dp open square",
    },
    Glyph {
        action: GlyphAction::Close,
        text: "\u{2715}",
        source: "this tree: the close mark is a 24 dp multiplication cross",
    },
];

/// The mark for one action, out of the declared table.
pub fn glyph_for(action: GlyphAction) -> &'static Glyph {
    GLYPHS
        .iter()
        .find(|glyph| glyph.action == action)
        .expect("every action in GlyphAction::ALL has a row in GLYPHS")
}

// ---------------------------------------------------------------------------
// The HUD
// ---------------------------------------------------------------------------

/// Which way a text run is aligned inside its rect.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Start,
    End,
}

/// One drawn thing, in device-independent pixels, with its paint already
/// resolved.
///
/// The renderer consumes a `Vec<Element>` and is told nothing about the design
/// system: not the ladder, not the palette, not the spacing grid. That is
/// deliberate — a renderer that can compute a colour can compute the wrong one,
/// and the only way a component can repaint itself outside the palette is if it
/// is handed something other than a resolved [`Paint`].
#[derive(Clone, PartialEq, Debug)]
pub enum Element {
    Surface { rect: Rect, paint: Paint },
    Rule { rect: Rect, paint: Paint },
    Text { rect: Rect, text: String, paint: Paint, size_dp: f64, align: Align },
}

impl Element {
    pub fn rect(&self) -> Rect {
        match self {
            Element::Surface { rect, .. }
            | Element::Rule { rect, .. }
            | Element::Text { rect, .. } => *rect,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Element::Text { text, .. } => Some(text),
            _ => None,
        }
    }
}

/// One line of a panel: what it is, what it says, and how sure the value is.
#[derive(Clone, PartialEq, Debug)]
pub struct Readout {
    pub label: String,
    pub value: String,
    pub state: State,
}

impl Readout {
    /// A value the instrument measured, and can stand behind.
    pub fn measured(label: &str, value: String) -> Self {
        Readout { label: label.to_string(), value, state: State::Normal }
    }

    /// A value the instrument **cannot** give, which is a state and not an
    /// omission: the row is still drawn, and it says why it is blank.
    /// A whitespace note is refused, because an empty view that *looks* empty is
    /// the failure the note exists to prevent.
    pub fn absent(label: &str, note: &str) -> UiResult<Self> {
        if note.trim().is_empty() {
            return Err(UiRefusal::EmptyViewWithoutNote { what: "a readout" });
        }
        Ok(Readout { label: label.to_string(), value: note.to_string(), state: State::Empty })
    }

    pub fn disabled(label: &str, value: String) -> Self {
        Readout { label: label.to_string(), value, state: State::Disabled }
    }

    /// `(label ink, value ink)`. The value carries the state's own step; the
    /// label is one step quieter, so a value can never be quieter than its own
    /// name and a reader always knows which half of the row is the number.
    pub fn inks(&self) -> (Ink, Ink) {
        let value = appearance(self.state).0;
        (value.dimmer(), value)
    }
}

/// A titled, bordered block of readouts on a named surface role.
#[derive(Clone, PartialEq, Debug)]
pub struct Panel {
    pub what: &'static str,
    /// The surface the panel is drawn on. A panel names a role, never a colour,
    /// so the palette keeps the only copy of the value.
    pub role: Role,
    pub title: String,
    pub rows: Vec<Readout>,
    /// A **full-width** sentence beneath the rows.
    ///
    /// Two jobs, and the second is why it is not called `empty_note`. When the
    /// panel has no rows it is what the panel says **instead of being blank** —
    /// the MD1 empty-view rule, and required. When it *does* have rows it is
    /// where an explanation goes, because a labelled readout's value column is
    /// about nineteen characters wide at 14 sp: prose squeezed into it is prose
    /// that gets silently cut off, and a truncated explanation is worse than a
    /// short one.
    pub note: Option<String>,
}

impl Panel {
    pub fn new(what: &'static str, role: Role, title: &str, rows: Vec<Readout>) -> Self {
        Panel { what, role, title: title.to_string(), rows, note: None }
    }

    /// Attach the panel's note. Refuses a note that says nothing, at the same
    /// moment [`Component::empty`] does.
    pub fn with_note(mut self, note: &str) -> UiResult<Self> {
        if note.trim().is_empty() {
            return Err(UiRefusal::EmptyViewWithoutNote { what: self.what });
        }
        self.note = Some(note.to_string());
        Ok(self)
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn validate(&self) -> Vec<UiRefusal> {
        let mut out = Vec::new();
        if self.is_empty() {
            match self.note.as_ref() {
                Some(note) if !note.trim().is_empty() => {}
                _ => out.push(UiRefusal::EmptyViewWithoutNote { what: self.what }),
            }
        }
        if self.title.trim().is_empty() {
            out.push(UiRefusal::EmptyViewWithoutNote { what: self.what });
        }
        out
    }

    /// How many rows get drawn: the readouts plus the note, if there is one.
    /// An empty panel is one row tall, not zero — a zero-height panel is exactly
    /// the blank pane the rule forbids.
    pub fn drawn_rows(&self) -> usize {
        self.rows.len() + usize::from(self.note.is_some())
    }

    pub fn height_dp(&self) -> f64 {
        let spacing = constant("SPACING_DP");
        let row = constant("ROW_HEIGHT_DP");
        let rule = constant("DIVIDER_DP");
        let n = self.drawn_rows().max(1) as f64;
        // top pad + title + (rule once) + n rows + (n-1) rules between them + pad
        2.0 * spacing + row + rule + n * row + (n - 1.0) * rule
    }
}

/// Lay a panel out at `at`, `width_dp` wide, over the worst backdrop the world
/// can put behind it.
///
/// Every number here comes from [`CONSTANTS`] or from the caller's width, and
/// every colour comes from `palette`. What is left is the refusals: a role with
/// no admitted value, an empty panel with no note, a width that is not a width,
/// and **any text whose composited contrast against the real backdrop is under
/// the floor**.
///
/// That last one is the reason this function returns a `Result` rather than a
/// `Vec`. A HUD that quietly draws unreadable text has failed at the only job a
/// HUD has.
pub fn layout_panel(
    palette: &Palette,
    panel: &Panel,
    at: (f64, f64),
    width_dp: f64,
    worst_backdrop: Rgb,
) -> UiResult<Vec<Element>> {
    if let Some(refusal) = panel.validate().into_iter().next() {
        return Err(refusal);
    }
    if !width_dp.is_finite() || width_dp <= 0.0 {
        return Err(UiRefusal::NonFiniteValue { what: "panel width", value: width_dp });
    }
    if !at.0.is_finite() || !at.1.is_finite() {
        return Err(UiRefusal::NonFiniteValue { what: "panel origin", value: at.0 });
    }

    let spacing = constant("SPACING_DP");
    let row = constant("ROW_HEIGHT_DP");
    let rule = constant("DIVIDER_DP");
    let title_size = constant("TITLE_SIZE_DP");
    let text_size = constant("TEXT_SIZE_DP");
    let opacity = constant("PANEL_OPACITY");

    let surface = palette.get(panel.role)?;
    // The surface as it actually appears: the panel at its own opacity over the
    // brightest thing the world can put behind it. Not the panel's own colour —
    // that colour is never on screen.
    let backdrop = surface.blend_over(worst_backdrop, opacity);

    let mut out = Vec::new();
    out.push(Element::Surface {
        rect: Rect::new(at.0, at.1, width_dp, panel.height_dp()),
        paint: Paint { rgb: surface, alpha: opacity },
    });

    // Every text run goes through this, so no element can reach the renderer
    // without having been measured against the backdrop it will really sit on.
    let text = |rect: Rect, s: String, ink: Ink, size_dp: f64, align: Align| -> UiResult<Element> {
        contrast_over_backdrop(ink.composited_over(backdrop), backdrop)?;
        Ok(Element::Text { rect, text: s, paint: ink.paint(), size_dp, align })
    };

    let left = at.0 + spacing;
    let inner_width = width_dp - 2.0 * spacing;
    let mut y = at.1;
    let centred = |top: f64, size: f64| top + (row - size) / 2.0;

    // The title.
    y += spacing;
    out.push(text(
        Rect::new(left, centred(y, title_size), inner_width, title_size),
        panel.title.clone(),
        Ink::Full,
        title_size,
        Align::Start,
    )?);
    y += row;

    // One rule under the title, and one between each pair of rows — never a
    // trailing rule, which would be a line to nothing.
    let rule_at = |top: f64, out: &mut Vec<Element>| {
        out.push(Element::Rule {
            rect: Rect::new(at.0, top, width_dp, rule),
            paint: Ink::Divider.paint(),
        });
    };
    rule_at(y, &mut out);
    y += rule;

    for (index, readout) in panel.rows.iter().enumerate() {
        if index > 0 {
            rule_at(y, &mut out);
            y += rule;
        }
        let (label_ink, value_ink) = readout.inks();
        // The label gets the leading 55 % of the row and the value the rest,
        // right-aligned, so a long label cannot push a value off the panel.
        let label_width = inner_width * 0.55;
        out.push(text(
            Rect::new(left, centred(y, text_size), label_width, text_size),
            readout.label.clone(),
            label_ink,
            text_size,
            Align::Start,
        )?);
        out.push(text(
            Rect::new(left + label_width, centred(y, text_size), inner_width - label_width, text_size),
            readout.value.clone(),
            value_ink,
            text_size,
            Align::End,
        )?);
        y += row;
    }

    if let Some(note) = panel.note.as_deref() {
        if !panel.is_empty() {
            rule_at(y, &mut out);
            y += rule;
        }
        // The note takes the **whole** inner width. When the panel has no rows
        // it is drawn in the empty state's own ink, so a panel with nothing to
        // report looks like an empty view rather than a broken one.
        let note_ink = if panel.is_empty() { appearance(State::Empty).0 } else { Ink::Secondary };
        out.push(text(
            Rect::new(left, centred(y, text_size), inner_width, text_size),
            note.to_string(),
            note_ink,
            text_size,
            Align::Start,
        )?);
    }

    Ok(out)
}

/// Where a HUD panel sits on the screen. MD1 puts primary content at the
/// leading edge; a diagnostic HUD is not primary content, so it takes a corner
/// and leaves the middle of the screen to the world.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Corner {
    TopStart,
    TopEnd,
    BottomStart,
    BottomEnd,
}

impl Corner {
    pub const ALL: [Corner; 4] = [Corner::TopStart, Corner::TopEnd, Corner::BottomStart, Corner::BottomEnd];

    /// The top-left corner of a `width_dp` × `height_dp` box in this corner of a
    /// viewport, at MD1's edge margin.
    pub fn origin(self, viewport_dp: Rect, width_dp: f64, height_dp: f64) -> (f64, f64) {
        let margin = constant("HUD_MARGIN_DP");
        let right = viewport_dp.width_dp - margin - width_dp;
        let bottom = viewport_dp.height_dp - margin - height_dp;
        let x = match self {
            Corner::TopStart | Corner::BottomStart => margin,
            Corner::TopEnd | Corner::BottomEnd => right,
        };
        let y = match self {
            Corner::TopStart | Corner::TopEnd => margin,
            Corner::BottomStart | Corner::BottomEnd => bottom,
        };
        (x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> Rect {
        Rect::new(0.0, 0.0, 48.0, 48.0)
    }

    #[test]
    fn every_constant_carries_a_name_a_bucket_and_a_source() {
        assert!(!CONSTANTS.is_empty());
        for c in CONSTANTS {
            assert!(!c.name.is_empty(), "a constant has no name");
            assert!(!c.source.is_empty(), "{name} has no source", name = c.name);
            assert!(c.value.is_finite(), "{} is not finite", c.name);
            assert!(Bucket::ALL.contains(&c.bucket), "{} has no bucket", c.name);
        }
        // Names are unique, so `constant(name)` cannot silently pick one of two.
        let mut names: Vec<&str> = CONSTANTS.iter().map(|c| c.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "two constants share a name");
    }

    #[test]
    fn the_project_extensions_are_counted_and_are_exactly_these() {
        // The bucket that can grow silently, pinned by count and by name.
        let extensions: Vec<&str> = project_extensions().iter().map(|c| c.name).collect();
        assert_eq!(
            extensions,
            vec![
                "ELEV_STEP",
                "MAX_ELEVATION",
                "REDUCED_CROSSFADE_MS",
                "HUD_PANEL_WIDTH_DP",
                "PANEL_OPACITY",
                "BAR_ENTRY_WIDTH_DP"
            ],
            "a project extension was added or removed; that is a decision, not a detail"
        );
    }

    #[test]
    fn the_ink_ladder_is_the_ladder() {
        assert_eq!(Ink::Full.opacity(), 1.0);
        assert_eq!(Ink::Secondary.opacity(), 0.70);
        assert_eq!(Ink::Tertiary.opacity(), 0.50);
        assert_eq!(Ink::Divider.opacity(), 0.12);
        // Monotonically decreasing, and only the divider is not for text.
        assert!(Ink::Full.opacity() > Ink::Secondary.opacity());
        assert!(Ink::Secondary.opacity() > Ink::Tertiary.opacity());
        assert!(Ink::Tertiary.opacity() > Ink::Divider.opacity());
        assert!(Ink::Full.is_text() && Ink::Secondary.is_text() && Ink::Tertiary.is_text());
        assert!(!Ink::Divider.is_text(), "a 12% glyph is a smudge, not a label");
    }

    #[test]
    fn what_is_unadmitted_is_counted_and_named() {
        let p = Palette::declared();
        assert_eq!(p.drawable_roles(), 3, "the record names two roles and the shell derives one");
        let missing = p.unadmitted();
        assert_eq!(missing.len(), 5, "expected five holes, found {}", missing.len());
        assert!(missing.contains(&Role::Canvas));
        assert!(!missing.contains(&Role::Surface), "the body surface is what the shell draws on");
        // And the admitted ones really are the declared values.
        assert_eq!(p.get(Role::SurfaceAlt).unwrap(), Rgb::new(0x33, 0x2F, 0x45));
        assert_eq!(p.get(Role::Chrome).unwrap(), Rgb::new(0x1A, 0x1A, 0x1A));
        assert_eq!(p.get(Role::Surface).unwrap(), Rgb::new(0x1B, 0x19, 0x22));
    }

    #[test]
    fn an_unadmitted_role_is_refused_by_name_and_never_defaulted() {
        let p = Palette::declared();
        let err = p.get(Role::Canvas).unwrap_err();
        assert_eq!(err, UiRefusal::TokenUnadmitted { role: Role::Canvas });
        assert!(err.to_string().contains("Canvas"));
    }

    #[test]
    fn md1s_rule_about_chrome_reports_which_role_blocks_it() {
        // The rule cannot be checked until the canvas has a value, and the
        // refusal says so instead of passing vacuously.
        let p = Palette::declared();
        assert_eq!(
            p.chrome_is_darker_than_canvas().unwrap_err(),
            UiRefusal::TokenUnadmitted { role: Role::Canvas }
        );
    }

    #[test]
    fn contrast_is_wcag_and_refuses_below_the_floor() {
        // The two ends of the scale, so the formula is pinned rather than
        // merely plausible.
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        assert!((black.contrast(white) - 21.0).abs() < 1e-9);
        assert!((white.contrast(white) - 1.0).abs() < 1e-9);
        // White on MD1 chrome #1A1A1A clears the floor comfortably.
        let chrome = Rgb::new(0x1A, 0x1A, 0x1A);
        assert!(contrast_over_backdrop(white, chrome).is_ok());
        // White on #A0A0A0 does not, and the refusal carries the measured ratio.
        let mid = Rgb::new(0xA0, 0xA0, 0xA0);
        let err = contrast_over_backdrop(white, mid).unwrap_err();
        match err {
            UiRefusal::ContrastBelowFloor { measured, floor } => {
                assert!(measured < floor, "{measured} < {floor}");
                assert!(measured > 1.0);
            }
            other => panic!("wrong refusal: {other}"),
        }
    }

    #[test]
    fn contrast_is_symmetric_and_measured_against_the_real_backdrop() {
        // Two backdrops: the authored surface, and what actually shows through a
        // translucent panel. The measurement has to be about the second, which
        // is why it takes a backdrop rather than assuming one.
        let white = Rgb::new(255, 255, 255);
        let authored = Rgb::new(0x1A, 0x1A, 0x1A);
        let blended = Rgb::new(0x77, 0x77, 0x77); // a bright world behind the glass
        assert!(contrast_over_backdrop(white, authored).is_ok());
        assert!(
            contrast_over_backdrop(white, blended).is_err(),
            "text that only passes against the authored surface is not passing"
        );
        assert!((white.contrast(blended) - blended.contrast(white)).abs() < 1e-12);
    }

    #[test]
    fn elevation_is_a_brightness_tint_and_a_level_above_the_ladder_is_refused() {
        let base = Rgb::new(0x40, 0x40, 0x40);
        let up = base.elevated(1).unwrap();
        assert!(up.luminance() > base.luminance(), "elevation must lighten");
        assert_eq!(base.elevated(0).unwrap(), base, "level zero is the surface itself");
        // Monotone up the ladder.
        let mut last = base;
        for level in 1..=max_elevation() {
            let next = base.elevated(level).unwrap();
            assert!(next.luminance() >= last.luminance());
            last = next;
        }
        assert_eq!(
            base.elevated(max_elevation() + 1).unwrap_err(),
            UiRefusal::UnknownElevation { level: max_elevation() + 1, maximum: max_elevation() }
        );
    }

    #[test]
    fn the_target_floor_is_on_both_dimensions() {
        assert!(Rect::new(0.0, 0.0, 48.0, 48.0).check_target("button").is_ok());
        assert!(Rect::new(0.0, 0.0, 47.0, 48.0).check_target("button").is_err());
        assert!(
            Rect::new(0.0, 0.0, 200.0, 20.0).check_target("strip").is_err(),
            "a 200x20 strip is not a thumb target"
        );
    }

    #[test]
    fn the_gap_rule_notices_targets_that_touch() {
        let a = Rect::new(0.0, 0.0, 48.0, 48.0);
        let touching = Rect::new(48.0, 0.0, 48.0, 48.0);
        let separated = Rect::new(56.0, 0.0, 48.0, 48.0);
        assert!(check_gap(&a, &separated).is_ok());
        assert_eq!(touching.gap(&a), 0.0);
        assert!(check_gap(&a, &touching).is_err());
        // Overlapping is worse than touching, and reports a **negative** gap —
        // the amount by which they overlap — because that is the more useful
        // fact. The checker floors it at zero when it names the shortfall, so a
        // refusal never reports a negative distance to a human.
        let overlapping = Rect::new(30.0, 0.0, 48.0, 48.0);
        assert_eq!(overlapping.gap(&a), -18.0);
        match check_gap(&a, &overlapping).unwrap_err() {
            UiRefusal::BelowMinimumGap { given, minimum } => {
                assert_eq!(given, 0, "a shortfall is never reported as negative");
                assert_eq!(minimum, 8);
            }
            other => panic!("wrong refusal: {other}"),
        }
    }

    #[test]
    fn every_declared_state_looks_different_from_every_other() {
        // The discriminating check. A `_ =>` arm in `appearance` would let two
        // states collapse into one and every test above would still pass.
        for a in State::ALL {
            for b in State::ALL {
                if a == b {
                    continue;
                }
                assert_ne!(
                    appearance(a),
                    appearance(b),
                    "{a:?} and {b:?} look identical, so one of them is not drawn"
                );
            }
        }
    }

    #[test]
    fn an_empty_view_gets_a_note_and_a_blank_note_is_not_a_note() {
        let ok = Component::empty("stock list", Role::Surface, target(), "No stock has been weighed yet.")
            .unwrap();
        assert!(ok.validate().is_empty());
        assert!(ok.note.is_some());

        assert_eq!(
            Component::empty("stock list", Role::Surface, target(), "   \n  ").unwrap_err(),
            UiRefusal::EmptyViewWithoutNote { what: "stock list" }
        );
        assert_eq!(
            Component::empty("stock list", Role::Surface, target(), "").unwrap_err(),
            UiRefusal::EmptyViewWithoutNote { what: "stock list" }
        );
    }

    #[test]
    fn a_component_that_claims_to_be_empty_but_carries_no_note_is_refused_at_validation() {
        // The constructor is not the only way in, so the rule is checked again
        // where it is drawn.
        let mut c = Component::normal("stock list", Role::Surface, target());
        c.state = State::Empty;
        let refusals = c.validate();
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert_eq!(refusals[0], UiRefusal::EmptyViewWithoutNote { what: "stock list" });
    }

    #[test]
    fn a_divider_may_not_carry_a_label() {
        let mut divider = Component::normal("rule", Role::Divider, Rect::new(0.0, 0.0, 48.0, 48.0));
        divider.note = Some("Section".into());
        assert_eq!(divider.validate().len(), 1);
    }

    #[test]
    fn a_component_under_the_target_floor_fails_validation() {
        let c = Component::normal("close", Role::Surface, Rect::new(0.0, 0.0, 24.0, 24.0));
        let refusals = c.validate();
        assert_eq!(refusals.len(), 1);
        assert!(matches!(refusals[0], UiRefusal::BelowMinimumTarget { .. }));
    }

    #[test]
    fn meaningful_motion_needs_a_reduced_alternative_and_decorative_does_not() {
        let decorative = Motion::decorative(220);
        assert!(decorative.validate().is_empty());
        assert_eq!(decorative.effective_ms(true), None, "decoration is dropped");

        // `meaningful` cannot be built without one, so the failure mode is
        // reached only by hand — and then it is caught.
        let mut broken = Motion { kind: MotionKind::Meaningful, duration_ms: 220, reduced: None };
        assert_eq!(broken.validate().len(), 1);
        broken.reduced = Some(Reduced::Crossfade);
        assert!(broken.validate().is_empty());

        let with_motion = broken;
        assert_eq!(with_motion.effective_ms(false), Some(220));
        assert_eq!(with_motion.effective_ms(true), Some(constant("REDUCED_CROSSFADE_MS") as u32));

        // And a meaningful motion that opted to be omitted plays nothing when
        // the preference is set, rather than playing in full.
        let omitted = Motion::meaningful(220, Reduced::Omit);
        assert!(omitted.validate().is_empty());
        assert_eq!(omitted.effective_ms(true), None);
    }

    #[test]
    fn nothing_may_be_visible_only_under_the_finger() {
        assert!(tooltip_is_reachable_without_touch(true).is_ok());
        assert!(tooltip_is_reachable_without_touch(false).is_err());
    }

    #[test]
    fn the_theme_is_dark_and_there_is_no_second_one() {
        assert_eq!(Theme::ONLY, Theme::Dark);
    }

    #[test]
    fn the_radius_is_two_device_independent_pixels() {
        assert_eq!(radius_dp(), 2.0);
    }

    // -----------------------------------------------------------------------
    // The HUD, and the ladder as a measurement
    // -----------------------------------------------------------------------

    /// MD1's chrome, and the brightest thing an SDR swapchain can present.
    fn chrome() -> Rgb {
        Rgb::new(0x1A, 0x1A, 0x1A)
    }

    fn white() -> Rgb {
        Rgb::new(0xFF, 0xFF, 0xFF)
    }

    #[test]
    fn the_panel_opacity_is_the_derived_minimum() {
        // The number is not chosen. It is the least opacity at which every text
        // step of the ladder clears the floor over a **white** world, and the
        // declared value is that minimum rounded up to a hundredth. Pinning the
        // derivation rather than the value is the point: an opacity can be
        // changed by anyone, but the floor cannot be talked out of.
        let least = least_opacity_keeping_the_ladder_legible(chrome(), white());
        let rounded_up = (least * 100.0).ceil() / 100.0;
        assert_eq!(
            rounded_up,
            constant("PANEL_OPACITY"),
            "the declared opacity is {rounded_up}, and the declaration says {}",
            constant("PANEL_OPACITY")
        );
        // And one hundredth less is genuinely not enough — which is what makes
        // the claim "least" rather than "sufficient".
        let one_less = constant("PANEL_OPACITY") - 0.01;
        assert!(one_less < least, "{one_less} would have been enough too");
    }

    #[test]
    fn every_text_step_is_legible_over_a_white_world_and_the_divider_step_is_not() {
        // `is_text()` stops being a preference here. Over the worst backdrop,
        // the three text steps clear the floor and the divider step does not —
        // so a divider carrying a label is refused by arithmetic, not by taste.
        let opacity = constant("PANEL_OPACITY");
        let legible = legible_steps(chrome(), opacity, white());
        assert_eq!(legible, vec![Ink::Full, Ink::Secondary, Ink::Tertiary], "{legible:?}");
        assert!(!legible.contains(&Ink::Divider));

        let backdrop = chrome().blend_over(white(), opacity);
        let divider = contrast_over_backdrop(Ink::Divider.composited_over(backdrop), backdrop)
            .unwrap_err();
        match divider {
            UiRefusal::ContrastBelowFloor { measured, floor } => {
                assert!(measured < floor, "{measured} < {floor}");
            }
            other => panic!("wrong refusal: {other}"),
        }
    }

    #[test]
    fn the_ladder_conclusion_does_not_depend_on_which_space_blending_happens_in() {
        // Whether the renderer blends in linear or in the encoded space is its
        // business. Which ink steps are readable is a design claim, and it must
        // not flip when that detail changes.
        let opacity = constant("PANEL_OPACITY");
        let backdrop_linear = chrome().blend_over(white(), opacity);
        let backdrop_encoded = chrome().blend_over_encoded(white(), opacity);
        assert_ne!(backdrop_linear, backdrop_encoded, "the two spaces do differ");
        for ink in [Ink::Full, Ink::Secondary, Ink::Tertiary] {
            assert!(
                contrast_over_backdrop(ink.composited_over(backdrop_linear), backdrop_linear).is_ok(),
                "{ink:?} fails under linear compositing"
            );
            assert!(
                contrast_over_backdrop(
                    ink.composited_over_encoded(backdrop_encoded),
                    backdrop_encoded
                )
                .is_ok(),
                "{ink:?} fails under encoded compositing"
            );
        }
        assert!(
            contrast_over_backdrop(
                Ink::Divider.composited_over_encoded(backdrop_encoded),
                backdrop_encoded
            )
            .is_err(),
            "the divider step must fail the floor in either space"
        );
    }

    #[test]
    fn a_readout_sets_its_label_one_step_quieter_than_its_value() {
        let measured = Readout::measured("chunks", "605 / 605".to_string());
        assert_eq!(measured.inks(), (Ink::Secondary, Ink::Full));
        // A value that is not available is still a value, and it says so.
        let absent = Readout::absent("digest", "no chunk has settled yet").unwrap();
        assert_eq!(absent.state, State::Empty);
        assert_eq!(absent.inks(), (Ink::Tertiary, Ink::Secondary));
        // A disabled row's label saturates rather than wrapping: never louder
        // than its own value.
        let disabled = Readout::disabled("pin", "0".to_string());
        let (label, value) = disabled.inks();
        assert!(label.opacity() <= value.opacity());
        assert_eq!(disabled.state, State::Disabled);
    }

    #[test]
    fn a_readout_with_no_value_must_say_why_it_has_none() {
        assert!(Readout::absent("digest", "").is_err());
        assert!(Readout::absent("digest", "  \t ").is_err());
        assert!(Readout::absent("digest", "the streamer has not settled").is_ok());
    }

    #[test]
    fn the_ink_over_a_panel_is_measured_where_it_lands_not_where_it_was_authored() {
        // A 50 % white ink over a white surface is not white-at-50 % — it is
        // invisible, and the paint knows that because it measures the composite.
        let invisible = Ink::Tertiary.paint().contrast_over(white()).unwrap_err();
        match invisible {
            UiRefusal::ContrastBelowFloor { measured, .. } => {
                assert!((measured - 1.0).abs() < 0.01, "{measured}")
            }
            other => panic!("wrong refusal: {other}"),
        }
        // And over chrome it is comfortable.
        assert!(Ink::Tertiary.paint().contrast_over(chrome()).unwrap() > 4.5);
    }

    #[test]
    fn an_empty_panel_needs_a_note_and_is_one_row_tall_not_zero() {
        let bare = Panel::new("refusals", Role::Chrome, "Refusals", Vec::new());
        assert_eq!(
            bare.validate(),
            vec![UiRefusal::EmptyViewWithoutNote { what: "refusals" }]
        );
        // Nothing to draw, and — the point — that is a *refusal*, not a pane.
        assert_eq!(bare.drawn_rows(), 0);
        assert!(!bare.validate().is_empty());
        // It is still laid out with a body: a panel that collapsed to nothing
        // would be the blank pane the rule forbids, wearing zero height.
        assert!(bare.height_dp() > 0.0);

        let noted = bare.clone().with_note("Nothing has been refused this session.").unwrap();
        assert!(noted.validate().is_empty());
        assert_eq!(noted.drawn_rows(), 1);
        assert_eq!(noted.height_dp(), bare.height_dp(), "emptiness does not resize the panel");

        assert!(bare.clone().with_note("\n  ").is_err());
    }

    #[test]
    fn a_panel_is_laid_out_at_the_grid_its_rows_are_inside_it_and_it_has_no_trailing_rule() {
        let palette = Palette::declared();
        let panel = Panel::new(
            "streamer",
            Role::Chrome,
            "Streamer",
            vec![
                Readout::measured("chunks", "605 / 605".to_string()),
                Readout::measured("mesh", "2.9 / 256.0 MB".to_string()),
            ],
        );
        let width = 320.0;
        let elements = layout_panel(&palette, &panel, (16.0, 16.0), width, white()).unwrap();

        let surface = elements.first().unwrap().rect();
        assert_eq!(surface, Rect::new(16.0, 16.0, width, panel.height_dp()));
        assert!(elements[..].iter().all(|e| e.rect().bottom() <= surface.bottom() + 1e-9));
        assert!(elements[..].iter().all(|e| e.rect().right() <= surface.right() + 1e-9));

        // One surface, a title, one rule per row (two here: under the title and
        // between the rows), and two text runs per row.
        let rules = elements.iter().filter(|e| matches!(e, Element::Rule { .. })).count();
        let texts: Vec<&str> = elements.iter().filter_map(|e| e.text()).collect();
        assert_eq!(rules, 2, "a rule under the title and one between two rows");
        assert_eq!(texts, vec!["Streamer", "chunks", "605 / 605", "mesh", "2.9 / 256.0 MB"]);
        // No rule at the very bottom of the panel, which would be a line to nothing.
        let lowest = elements
            .iter()
            .filter(|e| matches!(e, Element::Rule { .. }))
            .map(|e| e.rect().bottom())
            .fold(0.0_f64, f64::max);
        assert!(lowest < surface.bottom() - constant("SPACING_DP"));
        // Every text run sits on the ladder, and the label is never louder than
        // the value beside it.
        assert!(texts.len() % 2 == 1, "title plus one pair per row");
    }

    #[test]
    fn the_layout_refuses_a_surface_role_the_palette_has_not_admitted() {
        // `Canvas` is the standing example of a role named and not admitted; it
        // has to stay unadmitted here or this test stops testing the refusal.
        let palette = Palette::declared();
        let panel = Panel::new(
            "stock",
            Role::Canvas,
            "Stock",
            vec![Readout::measured("measured", "1".to_string())],
        );
        assert_eq!(
            layout_panel(&palette, &panel, (0.0, 0.0), 320.0, white()).unwrap_err(),
            UiRefusal::TokenUnadmitted { role: Role::Canvas }
        );
    }

    #[test]
    fn the_layout_refuses_a_width_that_is_not_a_width() {
        let palette = Palette::declared();
        let panel = Panel::new("x", Role::Chrome, "X", Vec::new())
            .with_note("nothing")
            .unwrap();
        assert!(layout_panel(&palette, &panel, (0.0, 0.0), 0.0, white()).is_err());
        assert!(layout_panel(&palette, &panel, (0.0, 0.0), f64::NAN, white()).is_err());
        assert!(layout_panel(&palette, &panel, (f64::INFINITY, 0.0), 320.0, white()).is_err());
    }

    #[test]
    fn a_panel_carries_its_own_note_into_the_layout_as_a_drawn_row() {
        let palette = Palette::declared();
        let panel = Panel::new("refusals", Role::Chrome, "Refusals", Vec::new())
            .with_note("Nothing has been refused this session.")
            .unwrap();
        let elements = layout_panel(&palette, &panel, (16.0, 16.0), 320.0, white()).unwrap();
        let texts: Vec<&str> = elements.iter().filter_map(|e| e.text()).collect();
        assert_eq!(texts, vec!["Refusals", "Nothing has been refused this session."]);
    }

    #[test]
    fn a_corner_places_a_panel_inside_the_viewport_at_the_edge_margin() {
        let viewport = Rect::new(0.0, 0.0, 1920.0, 1080.0);
        let margin = constant("HUD_MARGIN_DP");
        assert_eq!(Corner::TopStart.origin(viewport, 320.0, 100.0), (margin, margin));
        assert_eq!(
            Corner::TopEnd.origin(viewport, 320.0, 100.0),
            (1920.0 - margin - 320.0, margin)
        );
        assert_eq!(
            Corner::BottomEnd.origin(viewport, 320.0, 100.0),
            (1920.0 - margin - 320.0, 1080.0 - margin - 100.0)
        );
        assert_eq!(
            Corner::BottomStart.origin(viewport, 320.0, 100.0),
            (margin, 1080.0 - margin - 100.0)
        );
    }

    #[test]
    fn the_hud_sits_on_the_spacing_grid_it_claims_to() {
        // Every number the HUD uses for geometry is a multiple of the 8 dp grid
        // or the 1 dp divider, so a panel cannot drift off-grid by half a pixel.
        let grid = constant("SPACING_DP");
        for name in ["ROW_HEIGHT_DP", "HUD_MARGIN_DP", "BAR_HEIGHT_DP", "TITLE_BAR_DP"] {
            let value = constant(name);
            assert_eq!(value % grid, 0.0, "{name} is off the grid");
        }
        assert_eq!(constant("DIVIDER_DP"), 1.0);
        let panel = Panel::new(
            "p",
            Role::Chrome,
            "P",
            vec![Readout::measured("a", "1".to_string()); 3],
        );
        let height = panel.height_dp();
        assert_eq!(height % 1.0, 0.0, "a panel's height is a whole number of dp");
    }

    // -----------------------------------------------------------------------
    // The doctrine carried in: mechanisms, ceilings, bodies, glyphs
    // -----------------------------------------------------------------------

    #[test]
    fn every_admitted_role_states_a_material_and_stays_under_its_ceiling() {
        // `UNIFIED_DESIGN.md` §3.2.1, measured rather than reviewed: a colour
        // comes from a named mechanism, and past its ceiling it has stopped
        // being that material. The measurement is printed so the numbers can be
        // read off a run rather than taken on trust.
        let palette = Palette::declared();
        for role in Role::ALL {
            let rgb = palette.get(role);
            match (rgb, material_of(role)) {
                (Ok(rgb), Some(material)) => {
                    let chroma = rgb.chroma();
                    println!(
                        "{:<10} chroma {chroma:.4}  ceiling {:.3}  ({})",
                        material.name(),
                        material.ceiling(),
                        material.mechanism()
                    );
                    assert!(
                        chroma <= material.ceiling() + 1e-9,
                        "{role:?} carries chroma {chroma:.4}, above the {} ceiling {:.3}: a stronger \
                         colour is a different material, not a stronger one",
                        material.name(),
                        material.ceiling()
                    );
                }
                (Err(_), None) => {}
                (Ok(_), None) => panic!("{role:?} has a value but states no material"),
                (Err(_), Some(material)) => panic!(
                    "{role:?} states {} but has no colour — a mechanism with no value decides nothing",
                    material.name()
                ),
            }
        }
        // The ceiling table is the doctrine's, not this file's: seven families,
        // each with the mechanism it is made by.
        assert_eq!(Material::ALL.len(), 7);
        for material in Material::ALL {
            assert!(material.ceiling() > 0.0 && material.ceiling() <= 0.190, "{}", material.name());
            assert!(!material.mechanism().is_empty());
        }
    }

    #[test]
    fn the_window_body_carries_the_whole_ink_ladder_over_white() {
        // The derivation behind the one palette value this tree added. A window
        // body is a panel-carrying surface, so MD1's 50 % step has to be legible
        // on it over the worst backdrop at the declared opacity — and this is
        // what fixes the value rather than a preference for a dark grey.
        let palette = Palette::declared();
        let body = palette.get(Role::Surface).expect("the body role is admitted");
        let legible = legible_steps(body, constant("PANEL_OPACITY"), white());
        println!("window body {body:?} carries {legible:?} over white");
        assert_eq!(legible, vec![Ink::Full, Ink::Secondary, Ink::Tertiary]);
        // And it is a dark body: a window body appreciably lighter than the
        // chrome that frames it would lie about which surface is nearer the eye,
        // and would fail the ladder above on the next value up the scale.
        let chrome_luminance = palette.get(Role::Chrome).expect("chrome").luminance();
        assert!(
            body.luminance() <= chrome_luminance + 0.004,
            "the body is {:.4} and the chrome is {chrome_luminance:.4}; a lighter body is a \
             different surface, not a stronger one",
            body.luminance()
        );
        // The canvas is still unadmitted, and the MD1 rule that needs it still
        // reports which role blocks it rather than passing vacuously.
        assert_eq!(
            palette.chrome_is_darker_than_canvas().unwrap_err(),
            UiRefusal::TokenUnadmitted { role: Role::Canvas }
        );
    }

    #[test]
    fn the_theme_role_vocabulary_is_closed_and_refuses_an_unknown_name() {
        for (name, role) in Role::THEME_ROLE_NAMES {
            assert_eq!(Role::from_theme_role(name).expect("a declared name"), role, "{name}");
        }
        // The two the alpha package actually writes are in the table.
        assert_eq!(Role::from_theme_role("surface").unwrap(), Role::Surface);
        assert_eq!(Role::from_theme_role("hud").unwrap(), Role::Chrome);
        let err = Role::from_theme_role("brass").unwrap_err();
        assert_eq!(err, UiRefusal::UnknownThemeRole { theme_role: "brass".to_string() });
        assert!(err.to_string().contains("brass"));
    }

    #[test]
    fn every_control_has_one_declared_glyph_at_the_icon_grid() {
        assert_eq!(GLYPHS.len(), GlyphAction::ALL.len());
        for action in GlyphAction::ALL {
            let glyph = glyph_for(action);
            assert_eq!(glyph.action, action);
            assert!(!glyph.text.is_empty(), "{}", action.name());
            assert!(!glyph.source.is_empty(), "{}", action.name());
        }
        // The rule the dirty shell measured and this tree keeps: the glyph sits
        // inside a lane that meets the target floor.
        assert_eq!(glyph_dp(), 24.0);
        assert!(glyph_dp() <= constant("MIN_TARGET_DP"), "a glyph cannot be its own target");
        assert_eq!(bar_height_dp(), constant("MIN_TARGET_DP"), "the bar is one row of targets");
        assert_eq!(title_bar_dp(), bar_height_dp());
    }

    #[test]
    fn a_palette_extension_is_counted_by_name() {
        // The bucket that can grow silently is the one that cannot: a new value
        // with no mechanism in the record is an extension, and this names it.
        let extensions: Vec<Role> = Role::ALL
            .into_iter()
            .filter(|role| {
                matches!(
                    Palette::declared().slot(*role),
                    Slot::Declared { bucket: Bucket::ProjectExtension, .. }
                )
            })
            .collect();
        assert_eq!(
            extensions,
            vec![Role::Surface],
            "a palette extension was added or removed; that is a decision, not a detail"
        );
    }
}
