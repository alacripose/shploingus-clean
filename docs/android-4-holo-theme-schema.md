# Android 4.x Holo Theme Schema

**Schema ID:** `android-holo-4x`

**Target:** Android 4.0 Ice Cream Sandwich through Android 4.4 KitKat (API 14–19). The core Holo theme family began in Android 3.0/API 11; `Theme.Holo.Light.DarkActionBar` was introduced in API 14.

**Purpose:** A practical, source-backed design and implementation schema for recreating the native Android Holo visual language in an application, prototype, shell, or component library.

**Evidence convention:**
- **AOSP value** means the value/name is taken from Android Open Source Project framework resources.
- **Platform guidance** means it is documented by Android’s original developer/design material.
- **Schema recommendation** means a practical consistency rule derived for this spec; it should not be mistaken for a mandatory Google-published design token.

---

## 1. Design target and identity

Holo is the dark/light system theme family introduced in Android 3.x and refined/standardized in Android 4.0. Android 4.0 defined three main Holo themes:

| Theme | Intended appearance | Availability |
|---|---|---|
| `@android:style/Theme.Holo` | Dark surfaces, light foregrounds, cyan-blue interactive accent | API 11+ |
| `@android:style/Theme.Holo.Light` | Light gray surfaces, dark foregrounds, cyan-blue interactive accent | API 11+ |
| `@android:style/Theme.Holo.Light.DarkActionBar` | Light content area with a dark action bar | API 14+ |

**Use the named Holo theme explicitly** when matching the historical platform appearance. Android 4.0's `Theme.DeviceDefault` was allowed to reflect a device manufacturer's design; it is not a reliable synonym for the fixed Holo look. See [Holo Everywhere](https://android-developers.googleblog.com/2012/01/holo-everywhere.html).

### Identity principles

1. Dark or pale-gray structural surfaces, with a restrained cyan-blue accent.
2. High-contrast, clean sans-serif typography using Roboto for the Ice Cream Sandwich-era look.
3. Compact top-level navigation and actions through the Action Bar; contextual actions should appear when relevant.
4. Native-looking controls: rectangular/edge-defined fields and buttons, Holo checkboxes/radio buttons, seek bars, spinners, progress indicators, dialogs, and selection feedback.
5. Subtle state animation and visible pressed/focused/selected feedback. Do not substitute Material Design's later visual vocabulary (floating action buttons, elevated rounded cards, Material ripple, or modern bottom navigation) if the goal is faithful Android 4.x Holo.

## 2. Color tokens

The framework source defines the colors below. Hex values use `#AARRGGBB` when eight digits are shown; opaque equivalents are also listed for convenience.

### 2.1 Core palette

| Token | AOSP value | Use |
|---|---:|---|
| `surface.dark` / `background_holo_dark` | `#FF000000` (`#000000`) | Main dark-theme window background |
| `surface.light` / `background_holo_light` | `#FFF3F3F3` (`#F3F3F3`) | Main light-theme window background |
| `text.primary.onDark` / `bright_foreground_holo_dark` | `#FFF3F3F3` (`#F3F3F3`) | Primary foreground on dark surface; references the light background color |
| `text.primary.onLight` / `bright_foreground_holo_light` | `#FF000000` (`#000000`) | Primary foreground on light surface; references the dark background color |
| `text.secondary.onDark` / `dim_foreground_holo_dark` | `#BEBEBE` | Secondary/dim text on dark surface |
| `text.secondary.onLight` / `dim_foreground_holo_light` | `#323232` | Secondary/dim text on light surface |
| `text.disabled.onDark` / `bright_foreground_disabled_holo_dark` | `#FF4C4C4C` (`#4C4C4C`) | Disabled bright foreground in dark theme |
| `text.disabled.onLight` / `bright_foreground_disabled_holo_light` | `#FFB2B2B2` (`#B2B2B2`) | Disabled bright foreground in light theme |
| `text.hint` / `hint_foreground_holo_*` | `#808080` | Hint/placeholder foreground token |
| `accent.primary` / `holo_blue_light` | `#FF33B5E5` (`#33B5E5`) | Signature Holo cyan-blue accent; active controls and selection accents |
| `accent.blue.dark` / `holo_blue_dark` | `#FF0099CC` (`#0099CC`) | Deeper Holo blue; use where the control/state calls for the darker shade |
| `accent.green.light` / `holo_green_light` | `#FF99CC00` (`#99CC00`) | Holo green accent/status color |
| `accent.green.dark` / `holo_green_dark` | `#FF669900` (`#669900`) | Deeper green accent/status color |
| `status.red.light` / `holo_red_light` | `#FFFF4444` (`#FF4444`) | Error/destructive state color |
| `status.red.dark` / `holo_red_dark` | `#FFCC0000` (`#CC0000`) | Deeper error/destructive state color |
| `accent.orange.light` / `holo_orange_light` | `#FFFFBB33` (`#FFBB33`) | Warning/highlight accent |
| `accent.orange.dark` / `holo_orange_dark` | `#FFFF8800` (`#FF8800`) | Deeper orange accent |
| `accent.purple` / `holo_purple` | `#FFAA66CC` (`#AA66CC`) | Secondary expressive accent; use sparingly |
| `selection.textHighlight` / `highlighted_text_holo_*` | `#6633B5E5` | Translucent blue selection/highlight background |
| `text.link.onDark` / `link_text_holo_dark` | `#FF5C5CFF` (`#5C5CFF`) | Legacy Holo dark-theme link token |
| `text.link.onLight` / `link_text_holo_light` | `#FF0000EE` (`#0000EE`) | Legacy Holo light-theme link token |

**Canonical accent rule:** Use `#33B5E5` as the first-choice Holo accent when custom controls need a signature Holo blue. Do not recolor every element blue: surfaces and text remain neutral, and green/orange/red are reserved for meaning or state.

### 2.2 Color state rules

| State | Rule |
|---|---|
| Normal | Use the normal control/text color for the active theme. |
| Pressed | Prefer the framework Holo drawable/state list or its theme attribute. Do not imitate a Material ripple. |
| Focused | Show a clear Holo focus treatment, including for keyboard/trackball navigation; focus must not depend on color alone. |
| Selected / activated | Use the Holo selection/activation treatment consistently across a list or control group. |
| Disabled | Use the relevant disabled framework color/alpha treatment; preserve readable labels and distinguish disabled controls from active controls. |
| Error | Use Holo red plus clear text or an icon. Do not communicate the error using color alone. |
| Success | Holo green may be used as a status cue, not as a replacement for ordinary body text. |

Keep theme-specific text colors separate: dark-theme widgets sit on black/dark surfaces and light-theme widgets sit on `#F3F3F3`. Do not apply a single foreground color to both variants.

## 3. Typography schema

**Typeface:** Roboto is the defining Ice Cream Sandwich-era type family. Use Roboto rather than a modern rounded geometric font when visual fidelity matters. Android framework font fallback remains acceptable on systems where Roboto is not available.

The platform's base text appearance resources define these common size levels:

| Token | Reference size | Typical purpose |
|---|---:|---|
| `type.body` | `16sp` | Default body and control text |
| `type.large` | `22sp` | Large heading / emphasized text |
| `type.medium` | `18sp` | Medium emphasis / dialog title-scale text |
| `type.small` | `14sp` | Secondary labels, metadata, subtitles |
| `type.actionBarTitle` | `18` framework resource units | Historical Action Bar title metric; the AOSP dimension resource names this as `18dp` |
| `type.actionBarSubtitle` | `14` framework resource units | Historical Action Bar subtitle metric; the AOSP dimension resource names this as `14dp` |

For application-authored text, use **sp** so user font scaling works. The Action Bar entries above document the original framework resource definitions rather than recommending dp for new text styles.

### Typography rules

- Use a clear hierarchy based on size, weight, and spacing; avoid oversized marketing-style headings in ordinary utility screens.
- Favor normal-weight text for body content. Use bold sparingly for hierarchy and immediate emphasis.
- Avoid all-caps for long strings. Keep button/action labels short and direct.
- Maintain readable line length and line spacing; never clip text when font scaling is enabled.
- Do not use accent blue for long paragraphs or large blocks of copy.

## 4. Layout and dimensions

These are implementation baselines, not a universal grid guarantee for every Android 4.x layout.

| Token | Baseline | Evidence / use |
|---|---:|---|
| `layout.actionBar.height` | `48dp` | AOSP framework default Action Bar height |
| `layout.actionBar.iconVerticalPadding` | `8dp` | AOSP framework vertical padding around Action Bar icons |
| `layout.dialog.buttonBar.height` | `48dp` | AOSP framework dialog button-bar height |
| `layout.preference.sidePadding` | `8dp` | AOSP preference item side padding |
| `layout.preference.innerPadding` | `8dp` | AOSP preference item inner padding |
| `layout.schema.baseSpacing` | `8dp` | **Schema recommendation** for a practical spacing rhythm |
| `layout.schema.screenInset` | `16dp` | **Schema recommendation** for custom app content where the framework does not supply insets |

### Units

- Use **dp** for layout geometry, padding, and touch-target bounds.
- Use **sp** for app-authored text sizes.
- Use physical pixels only for bitmap asset dimensions or APIs that explicitly require pixels.
- Verify phone portrait, phone landscape, and tablet widths. Holo is a phone-and-tablet-era design system; do not hard-code a phone-only canvas if the application should adapt.
- Prefer Android's theme attributes and framework preferred dimensions when available rather than duplicating guessed sizes.

## 5. Component schema

### 5.1 App chrome and Action Bar

- Use the platform Action Bar as the primary top-level app chrome.
- Place the app title and the most important actions there; move less-frequent actions into the overflow menu when appropriate.
- Use action icons with the correct contrast for the Action Bar background. The original Android Design downloads include Holo Light/Holo Dark action-bar icon variants at density-specific sizes.
- Use the system's home/up affordance for hierarchical navigation; distinguish **Up** (within the app hierarchy) from **Back** (history/navigation behavior).
- If tabs are needed, use the period-appropriate Action Bar tab pattern; do not replace it with a modern bottom navigation bar for a faithful Holo recreation.
- `Theme.Holo.Light.DarkActionBar` is the canonical mixed variant: light content with a dark Action Bar.

### 5.2 Buttons

- Prefer the native `Widget.Holo` button style or an equivalent state-list drawable.
- Preserve Holo's crisp, compact, lightly dimensional/outlined treatment. Avoid pill-shaped geometry, large floating shadows, and Material filled-button styling.
- Every action has normal, pressed, focused, and disabled states.
- Button labels should describe the action (for example, “Save”, “Cancel”, “Refresh”).
- Destructive actions should have an explicit label and confirmation when the consequence is not easily reversible.

### 5.3 Text fields and search

- Use Holo EditText/search widgets or custom equivalents that preserve the native field outline/underline, selection handles, hint treatment, and focus indication.
- Keep hint text visually secondary to entered text.
- Show validation text near the relevant field and avoid color-only error indication.
- Ensure selection, cursor, and focus remain visible on both dark and light variants.

### 5.4 Checkboxes, radio buttons, switches, seek bars, and progress

- Use native Holo state drawables wherever possible; they already encode the expected sizing and interaction states.
- Active/checked/dragged states use the Holo control accent (typically `#33B5E5`).
- Keep labels associated with the control and make the entire sensible row/label area clickable where possible.
- For sliders/seek bars, show the current value when meaningful; do not rely on thumb position alone for numeric precision.
- Indeterminate progress should be visibly different from determinate progress; report progress in text when completion time or amount matters.

### 5.5 Lists and preference screens

- Use a clear row hierarchy: primary line first, secondary explanation or status below it, and a trailing control only when appropriate.
- Use native Holo list selectors/dividers and theme-provided preferred item metrics rather than drawing arbitrary separators.
- Make selected, activated, pressed, and focused states distinct enough to understand.
- For settings pages, follow the Android preference pattern: concise title, optional explanatory summary, and a control or disclosure affordance.
- Avoid wrapping every item in a modern rounded “card”; native Holo commonly presents content as rows on a shared screen surface.

### 5.6 Dialogs, menus, and popups

- Use platform Holo alert dialogs and contextual menus where possible.
- Keep a dialog focused on one decision or short task. Use concise title/body/action copy.
- Keep dialog actions in the expected button bar and provide a clear cancellation path when appropriate.
- Drop-downs, overflow menus, and contextual action modes should use theme-consistent dark/light backgrounds, separators, and text contrast.

### 5.7 Icons

- Prefer original Android 4.x action-bar iconography or simple custom icons that match its visual language.
- Use monochrome silhouettes appropriate to the action-bar background; do not mix dark icons onto a dark bar or light icons onto a light bar.
- Keep icons semantically recognizable and avoid decorative icons that compete with labels.
- Use density-specific raster assets for the original platform era, or correctly scaled vector assets when the implementation platform supports them. Do not enlarge a small raster icon and expect clean edges.

## 6. Interaction and motion

Every interactive element should support the following applicable states:

```text
normal
pressed
focused
selected_or_activated
checked
disabled
error_or_invalid
loading_or_progress
```

- Provide immediate, subtle response to touch and clear focus for keyboard, trackball, or other non-touch input.
- Prefer framework-provided selectors and drawable state lists to hand-built one-off effects.
- Animations should explain the change in state or navigation. Keep them brief and avoid motion that delays access to the next action.
- Holo feedback is not Material ripple. Do not add ripple animations to claim historical fidelity.
- Respect system animation settings where the implementation platform makes them available.

## 7. Implementation: native Android XML

### 7.1 Select the historical theme explicitly

For a native Android 4.x application, use the framework theme directly:

```xml
<application
    android:theme="@android:style/Theme.Holo"
    ... >
</application>
```

For the light variant:

```xml
<application
    android:theme="@android:style/Theme.Holo.Light"
    ... >
</application>
```

For light content with the dark Action Bar (API 14+):

```xml
<application
    android:theme="@android:style/Theme.Holo.Light.DarkActionBar"
    ... >
</application>
```

### 7.2 Customize incrementally

Prefer deriving app-specific styles from framework Holo styles and changing only the necessary values. The original Android framework guidance explicitly identifies `Widget.Holo` styles as stable parents for incremental customization on compatible Android versions.

Example theme inheritance:

```xml
<resources>
    <style name="AppTheme" parent="@android:style/Theme.Holo">
        <!-- Add only intentional app-specific overrides here. -->
    </style>

    <style name="AppActionButton" parent="@android:style/Widget.Holo.Button">
        <!-- Add only necessary widget-level overrides here. -->
    </style>
</resources>
```

Do not copy a modern Material theme into an Android 4.x project and assume that it reproduces Holo. Also avoid setting attributes introduced after API 19 directly in resources loaded on Android 4.x unless version-qualified resources or a compatibility library handles them. In particular, do not treat modern Material `colorPrimary`/`colorAccent` tokens as the original Holo contract.

### 7.3 Theme-aware dimensions and selectors

When custom widgets must match the system, prefer framework-provided theme attributes for selectable backgrounds, preferred list-item dimensions, padding, and text appearances. The 2012 Holo guidance gives examples such as `?android:attr/selectableItemBackground`, `?android:attr/listPreferredItemHeight`, `?android:attr/listPreferredItemPaddingLeft`, and `?android:attr/textAppearanceListItem`.

## 8. Custom UI / non-Android renderers

If implementing Holo in a custom shell, game UI, desktop UI, or a web renderer instead of native Android widgets, map each native concept to a named token rather than copying colors ad hoc.

```yaml
schema: android-holo-4x
profile: dark # dark | light | light-dark-action-bar
font_family: Roboto
units:
  layout: dp-equivalent
  text: sp-equivalent
colors:
  accent: "#33B5E5"
  accent_dark: "#0099CC"
  surface_dark: "#000000"
  surface_light: "#F3F3F3"
  text_primary_on_dark: "#F3F3F3"
  text_primary_on_light: "#000000"
  text_secondary_on_dark: "#BEBEBE"
  text_secondary_on_light: "#323232"
  hint: "#808080"
  success: "#99CC00"
  warning: "#FFBB33"
  error: "#FF4444"
metrics:
  action_bar_height: 48
  action_bar_icon_vertical_padding: 8
  dialog_button_bar_height: 48
  base_spacing: 8 # schema recommendation
  screen_inset: 16 # schema recommendation
text_sizes:
  body: 16
  small: 14
  medium: 18
  large: 22
states:
  - normal
  - pressed
  - focused
  - selected_or_activated
  - checked
  - disabled
  - error_or_invalid
```

This YAML is a **portable schema proposal**, not an Android resource file and not a verbatim Google data format. Native Android resource values and theme inheritance remain authoritative for native implementations.

## 9. Fidelity checklist

A recreation is Holo-faithful when all applicable items pass:

- [ ] Uses the intended dark, light, or light-content/dark-action-bar profile consistently.
- [ ] Uses the Holo cyan-blue `#33B5E5` as the primary active accent.
- [ ] Uses the native black (`#000000`) or light gray (`#F3F3F3`) surface, not modern Material gray surfaces by default.
- [ ] Uses Roboto or a visually compatible sans-serif, with a clear, restrained text hierarchy.
- [ ] Uses Action Bar-era top-level navigation/actions rather than later Material navigation conventions.
- [ ] Buttons, input fields, checkboxes, radio buttons, switches, progress controls, lists, and dialogs share a consistent Holo state language.
- [ ] Pressed/focused/selected/disabled states are implemented and distinguishable.
- [ ] Icons are appropriate for their background and do not mix competing visual styles.
- [ ] Layout uses dp-equivalent geometry and scalable text; no important content clips under text scaling.
- [ ] Modern Material features (FABs, elevated rounded cards, ripple, Material bottom navigation) have not been introduced unintentionally.
- [ ] Theme customization derives from native Holo widget/theme resources where available instead of replacing every widget from scratch.

## 10. Source references

The sources below are intended to be directly checkable. The AOSP files are primary implementation evidence; Android's archived design/developer guidance explains the intended family and usage.

1. **Android Developers Blog — “Holo Everywhere” (3 January 2012).** Defines the Holo family, its cross-device compatibility intent, explicit theme selection, `DeviceDefault` distinction, theme metrics, and theme-aware widget examples.  
   https://android-developers.googleblog.com/2012/01/holo-everywhere.html

2. **Android Developers — Android 4.0 / Ice Cream Sandwich highlights.** Describes the refined unified phone/tablet UI, visibility of common actions, interaction feedback, and the typeface update.  
   https://developer.android.com/about/versions/android-4.0-highlights

3. **Android Open Source Project — `colors_holo.xml`.** Primary source for Holo palette names and exact color values, including `holo_blue_light`, `holo_blue_dark`, Holo green/red/orange/purple, and Holo surfaces.  
   https://android.googlesource.com/platform/frameworks/base/+/6d2c0e5/core/res/res/values/colors_holo.xml

4. **Android Open Source Project — `colors.xml` Holo entries.** Primary source for Holo foreground, disabled, hint, link, selection, and surface colors.  
   https://android.googlesource.com/platform/frameworks/base/+/e79b55482eb3f26d6d5b56dce40682dd68826f8c/core/res/res/values/colors.xml

5. **Android Open Source Project — `themes_holo.xml`.** Framework theme declarations, theme attributes, palette wiring, and widget/style inheritance.  
   https://android.googlesource.com/platform/frameworks/base/+/1adf4002e7517c9981df0d1320e18b38a86978f8/core/res/res/values/themes_holo.xml

6. **Android Open Source Project — `styles_holo.xml`.** Holo widget and text-appearance definitions.  
   https://android.googlesource.com/platform/frameworks/base/+/master/core/res/res/values/styles_holo.xml

7. **Android Open Source Project — `dimens.xml`.** Framework metrics including the default Action Bar height, Action Bar icon padding, title/subtitle metrics, dialog button-bar height, and preference padding.  
   https://android.googlesource.com/platform/frameworks/base/+/HEAD/core/res/res/values/dimens.xml

8. **Android Design downloads (archived Android Design material).** Notes that the Ice Cream Sandwich design stencil/source package includes UI typography, colors, interactive controls, and phone/tablet outlines, plus Holo Light/Dark Action Bar icon assets and color swatches.  
   https://www.appframework.org/android_sdk_docs/docs/design/downloads/index.html

9. **Android Developers Blog — “Android Design V2: Now with stencils” (29 February 2012).** Historical confirmation of official ICS design stencils and source files for icons and controls.  
   https://android-developers.googleblog.com/2012/02/android-design-v2-now-with-stencils.html

### Attribution and interpretation note

Android framework source files are published under the Apache License 2.0. This Markdown document is a new specification assembled from the cited references; the portable YAML block, layout recommendations, and fidelity checklist are schema guidance, not official Google-authored standards. For strict pixel-perfect recreation, compare against an Android 4.0–4.4 emulator and the matching framework resources for the target API level.
