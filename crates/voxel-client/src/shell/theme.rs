//! **The theme: the coverage a theme must state, mapped onto this tree's vocabulary.**
//!
//! Reference: `docs/portal_simulator.py` — `Theme` 23–44 (the seventeen fields), `THEMES`
//! 46–74 (five named themes), and `apply_theme` 3850–4240 (the selector→field map: which
//! stylesheet rule reads which field). Step 2 of `docs/SHELL-PORT.md`'s map.
//!
//! # What is ported, and what is emphatically not
//!
//! The reference's `Theme` is **seventeen colour strings per theme, five themes** — 85 values.
//! **Not one of them is ported as a value**, and that is the record's rule rather than a
//! preference: a colour that gets drawn is an MD1 rule, an MD1 example, a platform translation
//! or a counted project extension (UI-SPEC §8), and a second palette is a second set of
//! unadmitted roles (`voxel-ui`'s `Theme::ONLY` says exactly this). What *is* ported is the
//! part the record calls the useful half:
//!
//! * **the field list is the coverage a theme must state** — the seventeen things
//!   `apply_theme`'s stylesheet actually reads (desktop, window body, window border, title
//!   active / inactive / text, text, secondary text, button / hover / pressed, accent, panel,
//!   desktop text, taskbar / border);
//! * **the mapping** — each field resolves through this tree's vocabulary: a [`Role`], an
//!   [`Ink`] step, or a role at a declared elevation. Where the record has no admitted value,
//!   the mapping **names the role that blocks it** ([`FieldSource::Unadmitted`]) instead of
//!   filling it.
//!
//! # Why this is a module and not a table in a comment
//!
//! `apply_theme` is the reference's **largest single method** (390 lines) and its job is one
//! sentence: *every surface a theme names is read from the theme and nothing else.* This tree's
//! equivalent is `Palette::get` refusing by name, and the coverage map is what makes the two
//! comparable: [`Theme::coverage`] answers, for every field the reference's stylesheet reads,
//! **which slot of this tree's vocabulary supplies it** — and [`Theme::unadmitted`] lists the
//! fields whose slot has no admitted value yet, each naming its role. That list is the measured
//! debt `theme.rs` owes the rest of the port, not a silence.
//!
//! # The five named themes
//!
//! The reference offers `Classic`, `Modern`, `Traditional`, `Dark`, `Light` (46–74). They are
//! **candidates, not admissions**: `voxel-ui`'s `Theme` has one variant (`Dark`) and the reason
//! is recorded there — a light variant is a second palette, and a second palette is a second
//! set of unadmitted roles. [`Theme::REFERENCE_NAMES`] carries the five names so a reader can
//! see what was considered, and [`Theme::ONLY`] is what exists. **`Light` is the one the record
//! has already ruled against** (MD1 dark theme only, UI-SPEC §8.1's ink ladder is a *white*
//! ladder), and the map says so by name rather than by omission.

use voxel_ui::{Ink, Role, UiRefusal, UiResult};

/// **Every field the reference's `Theme` states** (23–44), in its own declaration order.
///
/// This is the coverage a theme must answer for: each variant names one of the seventeen fields
/// `apply_theme`'s stylesheet reads. The names are the reference's field names, because the
/// point of the list is to be checkable against the reference.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    /// The desktop's own ground (3500: `QColor(self.portal.theme.desktop)`).
    Desktop,
    /// The taskbar's surface (`QFrame#taskbar`, 3905).
    Taskbar,
    /// The taskbar's top border (`border-top: 1px solid`, 3907).
    TaskbarBorder,
    /// A window's body (`QFrame#portalWindow`, 3862).
    Window,
    /// A window's border (`border: 1px solid`, 3863).
    WindowBorder,
    /// The title bar of a focused window (`QFrame#titlebar[focused="true"]`, 3920–3922).
    TitleActive,
    /// The title bar of an unfocused window (`QFrame#titlebar`, 3917–3919).
    TitleInactive,
    /// The window title's text (`QLabel#windowTitle`, 3924–3927).
    TitleText,
    /// The general text colour (`QWidget`, 3854).
    Text,
    /// Secondary text — the pane controls' own ink (`QPushButton#pane…`, 4000–4008).
    SecondaryText,
    /// A button's resting surface (`QPushButton`, 3899).
    Button,
    /// A button under the pointer (`QPushButton:hover`, 3901).
    ButtonHover,
    /// A button under a press (`QPushButton:pressed`, 3903).
    ButtonPressed,
    /// The accent — the splitter handle's hover, the placeholder's mark, the drop preview's
    /// border and label, the edge triggers (3970–3974, 4105–4109, 4060–4069, 4045–4053).
    Accent,
    /// A panel's surface (`t.panel`).
    Panel,
    /// The desktop's own text (`QListWidget#desktopExplorer`, 3868).
    DesktopText,
    /// The pane placeholder's text (`QLabel#placeholderText`).
    PlaceholderText,
}

impl Field {
    /// All seventeen, in the reference's declaration order (23–44).
    pub const ALL: [Field; 17] = [
        Field::Desktop,
        Field::Taskbar,
        Field::TaskbarBorder,
        Field::Window,
        Field::WindowBorder,
        Field::TitleActive,
        Field::TitleInactive,
        Field::TitleText,
        Field::Text,
        Field::SecondaryText,
        Field::Button,
        Field::ButtonHover,
        Field::ButtonPressed,
        Field::Accent,
        Field::Panel,
        Field::DesktopText,
        Field::PlaceholderText,
    ];

    /// The reference's own field name, for checking the list against `Theme` (23–44).
    pub const fn name(self) -> &'static str {
        match self {
            Field::Desktop => "desktop",
            Field::Taskbar => "taskbar",
            Field::TaskbarBorder => "taskbar_border",
            Field::Window => "window",
            Field::WindowBorder => "window_border",
            Field::TitleActive => "title_active",
            Field::TitleInactive => "title_inactive",
            Field::TitleText => "title_text",
            Field::Text => "text",
            Field::SecondaryText => "secondary_text",
            Field::Button => "button",
            Field::ButtonHover => "button_hover",
            Field::ButtonPressed => "button_pressed",
            Field::Accent => "accent",
            Field::Panel => "panel",
            Field::DesktopText => "desktop_text",
            Field::PlaceholderText => "placeholder_text",
        }
    }
}

/// **Where a field's value comes from in this tree's vocabulary** — the mapping `apply_theme`
/// performs with stylesheet text, stated as data.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldSource {
    /// A declared [`Role`], read through `Palette::get` (and refused by name when unadmitted).
    Role(Role),
    /// An **ink step** — the white ladder, not a colour: the field's job is a text weight or a
    /// line, and MD1 says those are ink rather than a second hue.
    Ink(Ink),
    /// A role at a **declared elevation** (`Role` + `ELEV_STEP × level`, C-55 §55.1): the
    /// field's job is a raised state of another surface, and the interaction language is the
    /// same tint.
    Elevated(Role, u8),
    /// **The rendered world is this field.** The shell floats over the world, and the world is
    /// the background — the user's ruling, 2026-10-03: *"the desktop background over the game
    /// would be dumb."* So nothing is drawn and no palette value is owed; `Role::Canvas` stays
    /// unadmitted **by design** rather than by debt.
    World,
    /// **No admitted value**, naming the role that blocks it — the refusal `Palette::get`
    /// would carry, made visible before the draw rather than at it.
    Unadmitted(Role),
}

/// This tree's theme: one variant, and the reason is `voxel-ui`'s (a second palette is a second
/// set of unadmitted roles). The name is kept in the record's spelling.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Theme;

impl Theme {
    /// The one theme.
    pub const ONLY: Theme = Theme;

    /// The reference's five names (46–74), carried so a reader sees what was considered.
    ///
    /// **Not admitted.** `Light` is the one the record has already ruled against: MD1 is a dark
    /// theme here and the ink ladder is a **white** ladder (UI-SPEC §8.1), so a light ground
    /// would put the whole ladder below the contrast floor. The others are candidate palettes
    /// whose values this tree has not derived — and a value is admitted by a rule or by a
    /// counted extension, never by being pretty.
    pub const REFERENCE_NAMES: [&'static str; 5] =
        ["Classic", "Modern", "Traditional", "Dark", "Light"];

    /// **The coverage map**: what supplies each of the reference's seventeen fields, in the
    /// reference's own order.
    ///
    /// Read it as the answer to *"the reference's stylesheet reads this field — what draws
    /// it here?"*. A [`FieldSource::Role`] is a value the palette already admits or refuses;
    /// [`FieldSource::Ink`] is the ladder; [`FieldSource::Elevated`] is a role at a declared
    /// level; [`FieldSource::Unadmitted`] is the debt, naming its role.
    pub const fn coverage(field: Field) -> FieldSource {
        match field {
            // The desktop's ground is **the world itself**. The reference fills this with
            // `theme.desktop` (3494–3507); the user ruled the background out — *"the desktop
            // background over the game would be dumb"* — so the shell draws no ground and the
            // field resolves to nothing rather than to an unadmitted role.
            Field::Desktop => FieldSource::World,
            // The taskbar is the upper bar (D-2), and the bar is chrome — the same role the
            // derived shell already draws it on (`mount.rs::bar_elements`).
            Field::Taskbar => FieldSource::Role(Role::Chrome),
            // A border is a line: the divider step, the same 1 dp rule the window's own
            // outline uses.
            Field::TaskbarBorder => FieldSource::Ink(Ink::Divider),
            // A window's body: the declared `theme-role`, which for every shipped window is
            // `surface` (the one body value the tree derived — `voxel-ui`'s
            // `the_window_body_carries_the_whole_ink_ladder_over_white`).
            Field::Window => FieldSource::Role(Role::Surface),
            // A window's border: 1 dp, the divider step (`window.rs::BORDER_DP`), brightened
            // one step when the window is focused — the reference's `focused` property
            // (3917–3922) as the tree's focus language.
            Field::WindowBorder => FieldSource::Ink(Ink::Divider),
            // The reference's `title_active` is a *background*, and this tree's title bar is
            // chrome elevated by the declared window level — `titlebar::elements` reads
            // `Role::Chrome` and `spec.elevation`. The field maps to the chrome role; which
            // elevation is the declaration's (C-109), not this table's.
            Field::TitleActive => FieldSource::Elevated(Role::Chrome, 0),
            // The unfocused bar is the same chrome: the reference changes its *colour*, and
            // this tree changes its **ink** (the title dims one step) — so the mapping is the
            // same role, and the difference is stated here rather than silently equal.
            Field::TitleInactive => FieldSource::Elevated(Role::Chrome, 0),
            // The window title: full ink when focused, one step down when not — the tree's
            // focus language, drawn by `titlebar.rs`.
            Field::TitleText => FieldSource::Ink(Ink::Full),
            // General text: full ink. The ladder *is* the text vocabulary — MD1's rule is that
            // secondary text is white at 70 %, never a tinted grey.
            Field::Text => FieldSource::Ink(Ink::Full),
            // Secondary text: the 70 % step, exactly as the pane controls already draw.
            Field::SecondaryText => FieldSource::Ink(Ink::Secondary),
            // A button's resting surface: chrome, at the window's declared level.
            Field::Button => FieldSource::Elevated(Role::Chrome, 0),
            // Hover is **one elevation step** — the tree's ruled interaction language
            // (UI-SPEC §8.1), not the reference's `button_hover` colour.
            Field::ButtonHover => FieldSource::Elevated(Role::Chrome, 1),
            // Pressed is two steps, for the same reason.
            Field::ButtonPressed => FieldSource::Elevated(Role::Chrome, 2),
            // **The accent is the debt this module's map makes countable.** The reference draws
            // it in four places — the splitter handle's hover, the placeholder's mark, the drop
            // preview's border and its label pill, the edge triggers — and every one of those
            // sites in this port currently falls back to the ink ladder *and says so in its own
            // module*. `Role::Accent` has no admitted value; this is the row that names it.
            Field::Accent => FieldSource::Unadmitted(Role::Accent),
            // A panel's surface: the alternative surface, the record's second declared value
            // (`#332F45`), which the menu and the placeholder already draw on.
            Field::Panel => FieldSource::Role(Role::SurfaceAlt),
            // The desktop's text: full ink. The reference's `desktop_text` colours the icon
            // labels and the wallpaper caption (3868, 3506–3507). **D-3 — whether the desktop
            // carries its own icons — is still open**, so the field maps to the ink a label
            // would use: the map stays complete whichever way the ruling goes, and nothing is
            // drawn from it today.
            Field::DesktopText => FieldSource::Ink(Ink::Full),
            // The placeholder's text: secondary ink — the note under the mark, drawn by
            // `placeholder.rs` on the `SurfaceAlt` frame.
            Field::PlaceholderText => FieldSource::Ink(Ink::Secondary),
        }
    }

    /// The value for a field, when this tree draws one — **or the refusal, by name**.
    ///
    /// A [`FieldSource::Role`] or [`FieldSource::Elevated`] asks `Palette::get`, so an
    /// unadmitted role refuses exactly as it does everywhere else; an [`FieldSource::Ink`] step
    /// is the ladder, drawn as white at its own alpha; an [`FieldSource::Unadmitted`] is the
    /// refusal its own map row states.
    ///
    /// **`Ok(None)` is a field this port does not draw** — [`FieldSource::World`] — and it is a
    /// state rather than a failure, which is the distinction the map's own variant carries. It
    /// is deliberately not folded into the refusal: a debt and a ruling are different facts.
    pub fn resolve(field: Field, palette: &voxel_ui::Palette) -> UiResult<Option<voxel_ui::Paint>> {
        match Self::coverage(field) {
            FieldSource::Role(role) => palette.get(role).map(|rgb| Some(voxel_ui::Paint::opaque(rgb))),
            FieldSource::Elevated(role, level) => palette
                .get(role)?
                .elevated(level)
                .map(|rgb| Some(voxel_ui::Paint::opaque(rgb))),
            FieldSource::Ink(ink) => Ok(Some(ink.paint())),
            FieldSource::World => Ok(None),
            FieldSource::Unadmitted(role) => Err(UiRefusal::TokenUnadmitted { role }),
        }
    }

    /// The fields this tree's palette **cannot draw yet**, each with the role that blocks it —
    /// the measured debt, in the reference's field order.
    pub fn unadmitted() -> Vec<(Field, Role)> {
        Field::ALL
            .into_iter()
            .filter_map(|field| match Self::coverage(field) {
                FieldSource::Unadmitted(role) => Some((field, role)),
                _ => None,
            })
            .collect()
    }

    /// The fields that resolve, and how — the other half of [`Theme::unadmitted`].
    pub fn admitted() -> Vec<Field> {
        Field::ALL
            .into_iter()
            .filter(|field| !matches!(Self::coverage(*field), FieldSource::Unadmitted(_)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_ui::Palette;

    #[test]
    fn the_coverage_is_the_reference_s_own_field_list_in_its_own_order() {
        // The reference's `Theme` (23–44) declares exactly these seventeen, in this order.
        let names: Vec<&str> = Field::ALL.iter().map(|field| field.name()).collect();
        assert_eq!(
            names,
            vec![
                "desktop",
                "taskbar",
                "taskbar_border",
                "window",
                "window_border",
                "title_active",
                "title_inactive",
                "title_text",
                "text",
                "secondary_text",
                "button",
                "button_hover",
                "button_pressed",
                "accent",
                "panel",
                "desktop_text",
                "placeholder_text",
            ]
        );
    }

    #[test]
    fn every_field_states_a_source_and_the_debt_is_exactly_the_accent() {
        // **The map's point, measured.** Six roles are unadmitted in `voxel-ui`; the one field
        // that actually needs one is the accent, and it is named rather than filled. The
        // desktop's ground is not debt: the world is the background (the user's ruling).
        let debt = Theme::unadmitted();
        assert_eq!(
            debt,
            vec![(Field::Accent, Role::Accent)],
            "a new unadmitted field is a decision, not a detail"
        );
        assert_eq!(Theme::admitted().len(), Field::ALL.len() - debt.len());
    }

    #[test]
    fn the_desktop_s_ground_is_the_world_and_resolves_to_nothing() {
        // The user's ruling, in the map: no wallpaper is drawn, so the field is `World` and
        // `resolve` answers `Ok(None)` — not a refusal, because nothing is owed.
        assert_eq!(Theme::coverage(Field::Desktop), FieldSource::World);
        let palette = Palette::declared();
        assert_eq!(Theme::resolve(Field::Desktop, &palette).expect("not a refusal"), None);
    }

    #[test]
    fn an_unadmitted_field_refuses_by_name_the_same_way_the_palette_does() {
        let palette = Palette::declared();
        assert_eq!(
            Theme::resolve(Field::Accent, &palette).unwrap_err(),
            UiRefusal::TokenUnadmitted { role: Role::Accent }
        );
    }

    #[test]
    fn the_admitted_fields_resolve_through_the_same_palette_the_rest_of_the_shell_uses() {
        let palette = Palette::declared();
        let value = |field| {
            Theme::resolve(field, &palette)
                .expect("admitted")
                .expect("drawn")
                .rgb
        };
        // The taskbar is chrome; a panel is the alternative surface; a window is the body.
        assert_eq!(value(Field::Taskbar), palette.get(Role::Chrome).expect("chrome"));
        assert_eq!(value(Field::Panel), palette.get(Role::SurfaceAlt).expect("surfaceAlt"));
        assert_eq!(value(Field::Window), palette.get(Role::Surface).expect("surface"));
        // Hover and pressed are the interaction language: one and two elevation steps.
        let resting = value(Field::Button);
        assert_eq!(
            value(Field::ButtonHover),
            resting.elevated(1).expect("one step")
        );
        assert_eq!(
            value(Field::ButtonPressed),
            resting.elevated(2).expect("two steps")
        );
        // An ink field is the white ladder at its own step, not a colour.
        assert_eq!(
            Theme::resolve(Field::Text, &palette).expect("admitted"),
            Some(Ink::Full.paint())
        );
    }

    #[test]
    fn the_five_reference_themes_are_carried_as_candidates_and_not_admitted() {
        assert_eq!(Theme::REFERENCE_NAMES.len(), 5);
        assert_eq!(Theme::REFERENCE_NAMES[3], "Dark", "the one the record admits is in the list");
        // `voxel-ui`'s `Theme` is the one palette, and this module does not add a second.
        assert_eq!(voxel_ui::Theme::ONLY, voxel_ui::Theme::Dark);
    }

    #[test]
    fn no_field_carries_a_colour_literal() {
        // The whole module's claim, as a test: the coverage names roles, ink steps and
        // elevations — never a colour. A `FieldSource` that could hold an `Rgb` would be the
        // second palette this tree refuses, so the type cannot spell one.
        for field in Field::ALL {
            match Theme::coverage(field) {
                FieldSource::Role(_)
                | FieldSource::Ink(_)
                | FieldSource::Elevated(_, _)
                | FieldSource::World => {}
                FieldSource::Unadmitted(role) => {
                    assert!(
                        Palette::declared().get(role).is_err(),
                        "{} is named unadmitted but the palette admits it",
                        field.name()
                    );
                }
            }
        }
    }
}
