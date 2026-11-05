//! **A webview-style control surface, declared in XML and laid out by Rust.**
//!
//! The map window's controls are not a hand-built panel in `mount.rs`: they are two documents —
//! `<map-toolbar>` (the readout strip across the window's top) and `<map-controls>` (the column in
//! its right gutter) — parsed by [`super::xml::Document::parse`] and laid out by
//! [`render_toolbar`] and
//! [`render`]. The renderer is deliberately **not a browser**: there is no CSS, no script and no
//! box model beyond a column's stack, a row's equal split, a stated height, and the toolbar's
//! fixed-label cells. What it shares with a webview is the property that matters here: **the
//! controls are data**, so a package can add a view without a Rust arm, and a click is an `action`
//! string the shell already knows how to run.
//!
//! # The vocabulary
//!
//! ```xml
//! <map-toolbar>
//!   <title text="WORLD MAP"/>
//!   <fact label="VIEW" field="view"/>
//!   <fact label="CELL" field="cell"/>
//! </map-toolbar>
//!
//! <map-controls>
//!   <label text="VIEW" class="caption"/>
//!   <button text="All" action="plan all"/>
//!   <row><button text="−" action="zoom out"/><button text="+" action="zoom in"/></row>
//!   <spacer height="8"/>
//!   <rule/>
//! </map-controls>
//! ```
//!
//! **The full reference — tags, attributes, facts, actions, geometry and failures — is
//! [`docs/MAP-CONTROLS.md`](../../../../docs/MAP-CONTROLS.md).** The rules this module enforces:
//!
//! * controls: `label` (`text`, `class`: `heading` or `caption`), `button` (`text`, `action`),
//!   `row` (equal-split children), `spacer` (`height`), `rule`;
//! * toolbar: `title` (`text`) and `fact` (`label`, `field`), where `field` is one of [`Facts`]'s
//!   `view`, `cell`, `extent`, `pitch`, `zoom`;
//! * an unknown tag, an unknown fact field, a button without an action, or an unclosed element is
//!   a **refusal naming the text**, never a silently skipped control;
//! * the palette contract is surfaces plus ink: `SurfaceAlt`/`Chrome` for the panel and buttons,
//!   one elevation step for the selected button, and `Ink` for every piece of text and every rule.
//!   The unadmitted roles (`Accent`, `Divider`, `Secondary`) are deliberately not required, because
//!   requiring them is what once turned this renderer into an empty list.

use super::xml::{Document, Node};
use voxel_ui::{Align, Element, Ink, Paint, Palette, Rect, Role};

/// The controls document the map window draws.
pub const MAP_CONTROLS: &str = r#"
<map-controls>
  <label text="VIEW" class="caption"/>
  <button text="All" action="plan all"/>
  <row>
    <button text="Prev" action="plan prev"/>
    <button text="Next" action="plan next"/>
  </row>
  <spacer height="6"/>
  <label text="ZOOM" class="caption"/>
  <row>
    <button text="−" action="zoom out"/>
    <button text="+" action="zoom in"/>
  </row>
  <button text="Reset" action="zoom reset"/>
  <spacer height="6"/>
  <label text="TURN" class="caption"/>
  <row>
    <button text="◀" action="rotate left"/>
    <button text="▶" action="rotate right"/>
  </row>
  <row>
    <button text="Pitch ▲" action="pitch up"/>
    <button text="Pitch ▼" action="pitch down"/>
  </row>
  <button text="Recentre" action="recenter"/>
  <spacer height="6"/>
  <label text="BIOMES" class="caption"/>
  <button text="Ocean" action="plan ocean"/>
  <button text="Mountain" action="plan mountain"/>
  <button text="Forest" action="plan forest"/>
  <button text="Grassland" action="plan grassland"/>
  <button text="Desert" action="plan desert"/>
  <button text="Tundra" action="plan tundra"/>
  <spacer height="6"/>
  <label text="KEYS" class="caption"/>
  <label text="WASD/ARROWS MOVE" class="caption"/>
  <label text="Q/E TURN" class="caption"/>
  <label text="−/+ ZOOM" class="caption"/>
  <label text="HOME CENTRE" class="caption"/>
</map-controls>
"#;

/// The readout strip across the map window's top.
pub const MAP_TOOLBAR: &str = r#"
<map-toolbar>
  <title text="WORLD MAP"/>
  <fact label="VIEW" field="view"/>
  <fact label="CELL" field="cell"/>
  <fact label="GRID" field="extent"/>
  <fact label="PITCH" field="pitch"/>
  <fact label="ZOOM" field="zoom"/>
  <fact label="CURSOR X/Z/Y" field="cursor"/>
</map-toolbar>
"#;

/// The live numbers the toolbar reads. They come from the map frame the window is showing and
/// the mount's zoom, so the toolbar cannot describe a different view from the one drawn.
#[derive(Clone, PartialEq, Debug)]
pub struct Facts {
    /// The selected world feature: `all`, `ocean`, `mountain`, …
    pub view: String,
    /// One map cell, metres.
    pub cell_m: f64,
    /// The frame's declared extent in cells: columns (x) and rows (z).
    pub extent: [u32; 2],
    /// The camera's downward tilt, degrees.
    pub pitch_deg: f64,
    /// The mount's zoom, in doublings.
    pub zoom: i32,
    /// **The world position of the cell under the pointer** — `[x, y, z]` in metres, where `y` is
    /// the ground that cell names. `None` when the pointer is not over the map body.
    pub cursor: Option<[f64; 3]>,
}

impl Facts {
    /// The text for one `<fact field="…"/>`, or a refusal naming it.
    pub fn value(&self, field: &str) -> Result<String, String> {
        match field {
            "view" => Ok(self.view.clone()),
            "cell" => Ok(format!("{} m", self.cell_m)),
            "extent" => Ok(format!("{}×{}", self.extent[0], self.extent[1])),
            "pitch" => Ok(format!("{:.0}°", self.pitch_deg)),
            "zoom" => Ok(self.zoom.to_string()),
            "cursor" => Ok(match self.cursor {
                Some([x, y, z]) => format!("{x:.0}/{z:.0}/{y:.0}"),
                None => "—".to_string(),
            }),
            other => Err(format!(
                "<fact field=\"{other}\"/> is not a fact the toolbar knows; the fields are \
                 view, cell, extent, pitch, zoom and cursor"
            )),
        }
    }
}

/// One pressable rectangle and the action its click runs.
#[derive(Clone, PartialEq, Debug)]
pub struct Control {
    /// Where the button draws, in the same dp space the pointer resolves in.
    pub rect: Rect,
    /// The shell verb the click runs (`plan ocean`, `zoom in`, …).
    pub action: String,
}

/// The drawn controls and the buttons a press resolves against.
#[derive(Clone, PartialEq, Debug)]
pub struct Rendered {
    /// The shell elements, in draw order.
    pub elements: Vec<Element>,
    /// Every button, in document order.
    pub buttons: Vec<Control>,
    /// **How tall the document is, all of it** — every child's own box and gaps, whether or not
    /// this pass drew it. The caller with a rectangle shorter than this is the caller that
    /// scrolls ([`Rendered::content_dp`] minus the rectangle is the furthest useful offset).
    pub content_dp: f64,
}

/// **Lay the document out in `rect`, scrolled down by `skip_dp`.** `active` is the action the
/// current view is showing; a button whose action equals it draws in the accent.
///
/// **The tail of a document taller than its rectangle is reachable, not dropped.** The layout
/// walks the whole document and draws the children that fit **wholly** inside the content box,
/// no matter how far down it is: an element the model cannot clip would otherwise paint a halved
/// button over the toolbar above the gutter. [`Rendered::content_dp`] tells the caller how much
/// there is above the rectangle's bottom, which is what a wheel needs to know before it moves.
pub fn render(
    document: &Document,
    rect: Rect,
    palette: &Palette,
    active: Option<&str>,
    skip_dp: f64,
) -> Result<Rendered, String> {
    if document.root.name != "map-controls" {
        return Err(format!(
            "the root is <{}>; the controls document is <map-controls>",
            document.root.name
        ));
    }
    let surface = paint(palette, Role::SurfaceAlt)?;
    let paints = Paints::of(palette)?;
    let mut out = Rendered {
        elements: vec![Element::Surface {
            rect,
            paint: surface,
        }],
        buttons: Vec::new(),
        content_dp: 0.0,
    };
    out.elements.push(Element::Rule {
        rect: Rect::new(rect.x, rect.y, 1.0, rect.height_dp),
        paint: paints.divider,
    });
    let padding = 8.0;
    let content = Rect::new(
        rect.x + padding,
        rect.y + padding,
        (rect.width_dp - 2.0 * padding).max(1.0),
        (rect.height_dp - 2.0 * padding).max(1.0),
    );
    let mut y = content.y - skip_dp.max(0.0);
    stack(
        &document.root,
        content.x,
        &mut y,
        content.width_dp,
        content.y,
        content.bottom(),
        active,
        &paints,
        &mut out,
    )?;
    out.content_dp = column_dp(&document.root);
    Ok(out)
}

/// **Every paint a control can be drawn with**, from two admitted surface roles and the ink
/// ladder. A struct rather than six parameters keeps the call sites readable and removes the
/// `too_many_arguments` allowance the old signature needed.
struct Paints {
    /// A resting button's surface.
    chrome: Paint,
    /// The selected view's button: chrome raised one elevation step.
    selected: Paint,
    /// Full ink: headings and the selected button's label.
    emphasis: Paint,
    /// Secondary ink: captions and a resting button's label.
    secondary: Paint,
    /// The 12 % rule step: borders and a resting button's underline.
    divider: Paint,
}

impl Paints {
    /// **The whole palette contract in one place.** Two admitted surfaces and the ink ladder;
    /// no unadmitted role can make a control refuse.
    fn of(palette: &Palette) -> Result<Self, String> {
        let chrome = paint(palette, Role::Chrome)?;
        let selected = Paint::opaque(
            palette
                .get(Role::Chrome)
                .and_then(|rgb| rgb.elevated(1))
                .map_err(|refusal| refusal.to_string())?,
        );
        Ok(Self {
            chrome,
            selected,
            emphasis: Ink::Full.paint(),
            secondary: Ink::Secondary.paint(),
            divider: Ink::Divider.paint(),
        })
    }
}

/// **Lay the readout strip out in `rect`**: a title cell on the left and one equal cell per
/// `<fact>`. The toolbar is chrome, not the panel's alternative surface, so it reads as the map
/// window's own header rather than as another control block.
pub fn render_toolbar(
    document: &Document,
    rect: Rect,
    palette: &Palette,
    facts: &Facts,
) -> Result<Rendered, String> {
    if document.root.name != "map-toolbar" {
        return Err(format!(
            "the root is <{}>; the toolbar document is <map-toolbar>",
            document.root.name
        ));
    }
    let paints = Paints::of(palette)?;
    let mut out = Rendered {
        elements: vec![Element::Surface {
            rect,
            paint: paint(palette, Role::Chrome)?,
        }],
        buttons: Vec::new(),
        // The toolbar is a strip: its content is its own height, and there is nothing under it to
        // scroll to. The field belongs to the type because the gutter is a column that does.
        content_dp: rect.height_dp,
    };
    out.elements.push(Element::Rule {
        rect: Rect::new(rect.x, rect.bottom() - 1.0, rect.width_dp, 1.0),
        paint: paints.divider,
    });
    let padding = 10.0;
    let title_width = 96.0;
    // **The cursor's column is two and a half facts wide**, because it is the only fact that
    // carries three numbers: an equal sixth of a narrow map window clipped it to its first value.
    // The rest stay equal, so a value that fitted before still fits.
    let weight = |child: &Node| -> f64 {
        match child.name.as_str() {
            "fact" if child.attr("field") == Some("cursor") => 2.5,
            "fact" => 1.0,
            _ => 0.0,
        }
    };
    let total: f64 = document
        .root
        .children
        .iter()
        .map(weight)
        .sum::<f64>()
        .max(1.0);
    let each = ((rect.width_dp - title_width - 2.0 * padding) / total).max(1.0);
    let mut fact_index = 0usize;
    let mut consumed = 0.0f64;
    for child in &document.root.children {
        match child.name.as_str() {
            "title" => {
                let text = child.attr("text").unwrap_or_default();
                out.elements.push(Element::Text {
                    rect: Rect::new(
                        rect.x + padding,
                        rect.y + (rect.height_dp - 12.0) / 2.0,
                        title_width - padding,
                        12.0,
                    ),
                    text: text.to_string(),
                    paint: paints.emphasis,
                    size_dp: 11.0,
                    align: Align::Start,
                });
            }
            "fact" => {
                let Some(field) = child.attr("field") else {
                    return Err("<fact> has no field".to_string());
                };
                let value = facts.value(field)?;
                let column = weight(child);
                let x = rect.x + title_width + padding + consumed * each;
                let width = (column * each - padding).max(1.0);
                consumed += column;
                if fact_index > 0 {
                    out.elements.push(Element::Rule {
                        rect: Rect::new(
                            x - padding * 0.5,
                            rect.y + 6.0,
                            1.0,
                            rect.height_dp - 12.0,
                        ),
                        paint: paints.divider,
                    });
                }
                out.elements.push(Element::Text {
                    rect: Rect::new(x, rect.y + 5.0, width, 8.0),
                    text: child.attr("label").unwrap_or(field).to_string(),
                    paint: paints.secondary,
                    size_dp: 8.0,
                    align: Align::Start,
                });
                out.elements.push(Element::Text {
                    rect: Rect::new(x, rect.y + 17.0, width, 11.0),
                    text: value,
                    paint: paints.emphasis,
                    size_dp: 11.0,
                    align: Align::Start,
                });
                fact_index += 1;
            }
            other => {
                return Err(format!(
                    "<{other}> is not a toolbar element this renderer knows"
                ))
            }
        }
    }
    Ok(out)
}

/// **One child's own height and the gap under it**, dp — *the* place the column's arithmetic
/// lives, so the layout that draws the document and the measurement of how tall it is cannot
/// drift apart. A tag this renderer does not know is a zero here and a refusal in [`stack`], where
/// the name is available to print.
fn child_box(node: &Node) -> (f64, f64) {
    match node.name.as_str() {
        "row" | "button" => (24.0, 2.0),
        "label" => (
            if node.attr("class") == Some("heading") {
                18.0
            } else {
                14.0
            },
            4.0,
        ),
        "spacer" => (
            node.attr("height")
                .and_then(|value| value.parse().ok())
                .unwrap_or(6.0),
            0.0,
        ),
        "rule" => (1.0, 4.0),
        _ => (0.0, 0.0),
    }
}

/// **How tall a node's column is**, dp, over the whole document — every child and every gap,
/// whether or not any rectangle has room to draw it. This is what a caller with a shorter
/// rectangle scrolls to the end of ([`Rendered::content_dp`]).
fn column_dp(node: &Node) -> f64 {
    node.children
        .iter()
        .map(|child| {
            let (height, gap) = child_box(child);
            height + gap
        })
        .sum()
}

/// Lay a node's children out as a column, drawing what fits.
///
/// **A child above `top` is walked and not drawn**, and a child that would cross `bottom` ends the
/// walk: only what fits wholly inside `top..bottom` becomes elements. A straddling button is drawn
/// whole or not at all, because an element this model has no scissor for would paint over the
/// toolbar above the gutter. The **document's own height** is [`column_dp`]'s, not this walk's:
/// what did not fit is still there, one scroll away.
#[allow(clippy::too_many_arguments)]
fn stack(
    node: &Node,
    x: f64,
    y: &mut f64,
    width: f64,
    top: f64,
    bottom: f64,
    active: Option<&str>,
    paints: &Paints,
    out: &mut Rendered,
) -> Result<(), String> {
    for child in &node.children {
        if *y >= bottom {
            break;
        }
        let (height, gap) = child_box(child);
        if *y + height > bottom {
            break;
        }
        // **Scrolled past, or straddling the top edge**: nothing is drawn for it.
        let visible = *y >= top;
        match child.name.as_str() {
            "row" => {
                let count = child.children.len().max(1) as f64;
                let gap = 4.0;
                let each = ((width - gap * (count - 1.0)) / count).max(1.0);
                if visible {
                    for (index, button) in child.children.iter().enumerate() {
                        let at = x + index as f64 * (each + gap);
                        widget(button, Rect::new(at, *y, each, height), active, paints, out)?;
                    }
                }
            }
            "button" => {
                if visible {
                    widget(child, Rect::new(x, *y, width, height), active, paints, out)?;
                }
            }
            "label" => {
                if visible {
                    let text = child.attr("text").unwrap_or_default();
                    let ink = if child.attr("class") == Some("heading") {
                        paints.emphasis
                    } else {
                        paints.secondary
                    };
                    out.elements.push(Element::Text {
                        rect: Rect::new(x, *y, width, height),
                        text: text.to_string(),
                        paint: ink,
                        size_dp: if height > 16.0 { 12.0 } else { 9.0 },
                        align: Align::Start,
                    });
                }
            }
            "spacer" => {}
            "rule" => {
                if visible {
                    out.elements.push(Element::Rule {
                        rect: Rect::new(x, *y, width, 1.0),
                        paint: paints.divider,
                    });
                }
            }
            other => return Err(format!("<{other}> is not a control this renderer knows")),
        }
        *y += height + gap;
    }
    Ok(())
}

/// Draw one button, or refuse the tag.
#[allow(clippy::too_many_arguments)]
fn widget(
    node: &Node,
    rect: Rect,
    active: Option<&str>,
    paints: &Paints,
    out: &mut Rendered,
) -> Result<(), String> {
    let Some(action) = node.attr("action") else {
        return Err(format!("<{}> has no action", node.name));
    };
    let text = node.attr("text").unwrap_or(action);
    let selected = active == Some(action);
    out.elements.push(Element::Surface {
        rect,
        paint: if selected {
            paints.selected
        } else {
            paints.chrome
        },
    });
    out.elements.push(Element::Rule {
        rect: Rect::new(rect.x, rect.bottom() - 1.0, rect.width_dp, 1.0),
        paint: if selected {
            paints.emphasis
        } else {
            paints.divider
        },
    });
    out.elements.push(Element::Text {
        rect: Rect::new(
            rect.x + 6.0,
            rect.y + 4.0,
            rect.width_dp - 12.0,
            rect.height_dp - 8.0,
        ),
        text: text.to_string(),
        paint: if selected {
            paints.emphasis
        } else {
            paints.secondary
        },
        size_dp: 11.0,
        align: Align::Start,
    });
    out.buttons.push(Control {
        rect,
        action: action.to_string(),
    });
    Ok(())
}

fn paint(palette: &Palette, role: Role) -> Result<Paint, String> {
    palette
        .get(role)
        .map(|rgb| Paint { rgb, alpha: 1.0 })
        .map_err(|refusal| refusal.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_document_parses_and_every_button_has_an_action() {
        let document = Document::parse(MAP_CONTROLS).expect("the controls parse");
        let rendered = render(
            &document,
            Rect::new(0.0, 0.0, 136.0, 440.0),
            &Palette::declared(),
            None,
            0.0,
        )
        .expect("the controls render");
        assert!(rendered.buttons.len() >= 10, "{:?}", rendered.buttons);
        assert!(rendered
            .buttons
            .iter()
            .any(|button| button.action == "plan ocean"));
        assert!(rendered
            .buttons
            .iter()
            .all(|button| button.rect.x >= 0.0 && button.rect.width_dp > 0.0));
        // **And the document's own height, which is what a caller with a shorter rectangle
        // scrolls.** The column is taller than a 440 dp gutter — that is why the mount has a
        // scroll — so the number is the whole document's and not the part that fitted.
        assert!(rendered.content_dp > 440.0, "{:?}", rendered.content_dp);
        // What was drawn, though, is inside the rectangle: content never leaks out of its box.
        for button in &rendered.buttons {
            assert!(
                button.rect.y >= 0.0 && button.rect.bottom() <= 440.0,
                "{:?} escapes the gutter",
                button.rect
            );
        }
    }

    /// **A document taller than its rectangle is scrolled, not truncated.** The tail of the
    /// gutter's column is reachable by asking for a taller document with an offset, and what is
    /// drawn is what fits wholly inside the rectangle — never a halved button over the toolbar.
    #[test]
    fn a_taller_document_is_drawn_from_the_offset_and_reaches_its_own_end() {
        let document = Document::parse(MAP_CONTROLS).expect("the controls parse");
        let rect = Rect::new(0.0, 0.0, 136.0, 200.0);
        let top = render(&document, rect, &Palette::declared(), None, 0.0).expect("renders");
        let most = (top.content_dp - (rect.height_dp - 16.0)).max(0.0);
        assert!(
            most > 0.0,
            "the document fits after all: {:?}",
            top.content_dp
        );
        let bottom = render(&document, rect, &Palette::declared(), None, most).expect("renders");
        // **The document's own last button is the one the end shows**, read out of the document so a
        // control appended tomorrow is covered without this test naming it: the two passes between
        // them hold every action the document declares, which is what makes a scroll a way to reach
        // a control rather than a way to hide one.
        let mut declared = Vec::new();
        walk(&document.root, &mut declared);
        assert!(
            bottom
                .buttons
                .iter()
                .any(|button| Some(&button.action) == declared.last()),
            "the document's last control is not reachable at its own end: {:?}",
            bottom.buttons
        );
        for button in &bottom.buttons {
            assert!(
                button.rect.y >= rect.y && button.rect.bottom() <= rect.bottom(),
                "a scrolled button escaped the gutter: {:?}",
                button.rect
            );
        }
        let mut reached: Vec<String> = top.buttons.iter().map(|b| b.action.clone()).collect();
        // **Every offset the wheel can reach**, a row at a time, and the union of what they draw is
        // the document: a control is reachable when some scroll shows it, which is the claim a
        // scrolling column makes. A row step is the mount's own ([`MAP_CONTROLS_ROW_DP`]).
        let mut offset = 0.0;
        while offset < most {
            offset = (offset + 26.0).min(most);
            let pass =
                render(&document, rect, &Palette::declared(), None, offset).expect("renders");
            reached.extend(pass.buttons.iter().map(|b| b.action.clone()));
        }
        fn walk(node: &Node, out: &mut Vec<String>) {
            if node.name == "button" {
                if let Some(action) = node.attr("action") {
                    out.push(action.to_string());
                }
            }
            for child in &node.children {
                walk(child, out);
            }
        }
        for action in &declared {
            assert!(
                reached.contains(action),
                "{action} is declared and never drawn"
            );
        }
        // **The two passes are not the same pass**: the offset moved the document, so the end draws
        // ground the head does not — the claim that a scroll scrolls, rather than the head twice.
        assert_ne!(
            top.buttons.iter().map(|b| &b.action).collect::<Vec<_>>(),
            bottom.buttons.iter().map(|b| &b.action).collect::<Vec<_>>(),
            "the scrolled pass drew exactly the head's own controls"
        );
    }

    #[test]
    fn the_active_view_is_one_elevation_and_full_ink() {
        let document = Document::parse(MAP_CONTROLS).expect("the controls parse");
        let plain = render(
            &document,
            Rect::new(0.0, 0.0, 136.0, 440.0),
            &Palette::declared(),
            None,
            0.0,
        )
        .expect("renders");
        let selected = render(
            &document,
            Rect::new(0.0, 0.0, 136.0, 440.0),
            &Palette::declared(),
            Some("plan ocean"),
            0.0,
        )
        .expect("renders");
        let count = |rendered: &Rendered| {
            rendered
                .elements
                .iter()
                .filter(|element| matches!(element, Element::Surface { .. }))
                .count()
        };
        assert_eq!(count(&plain), count(&selected));
        assert_ne!(
            plain
                .elements
                .iter()
                .filter_map(element_paint)
                .collect::<Vec<_>>(),
            selected
                .elements
                .iter()
                .filter_map(element_paint)
                .collect::<Vec<_>>()
        );
        let full = Ink::Full.paint();
        let full_labels = |rendered: &Rendered| {
            rendered
                .elements
                .iter()
                .filter(|element| match element {
                    Element::Text { paint, .. } => *paint == full,
                    _ => false,
                })
                .count()
        };
        assert!(
            full_labels(&selected) > full_labels(&plain),
            "the selected view carries one more full-ink label than the resting one"
        );
    }

    #[test]
    fn the_declared_palette_draws_every_control_over_an_opaque_surface() {
        // **The bug this test would have caught.** The renderer used to require `Role::Accent`,
        // `Role::Divider` and `Role::Secondary`, all deliberately unadmitted; every call with
        // `Palette::declared()` failed and the mount turned that failure into an empty list.
        let palette = Palette::declared();
        assert!(
            palette.unadmitted().contains(&Role::Accent),
            "the accent is this tree's measured debt, not a value to invent"
        );
        let rect = Rect::new(800.0, 329.0, 143.0, 398.0);
        let document = Document::parse(MAP_CONTROLS).expect("the controls parse");
        let rendered =
            render(&document, rect, &palette, None, 0.0).expect("the declared palette draws");
        assert!(rendered.buttons.len() >= 10, "{:?}", rendered.buttons);
        assert!(
            rendered.elements.first()
                == Some(&Element::Surface {
                    rect,
                    paint: palette
                        .get(Role::SurfaceAlt)
                        .map(voxel_ui::Paint::opaque)
                        .expect("surfaceAlt"),
                }),
            "the panel's first element covers the whole gutter"
        );
        assert!(
            !rendered.elements.iter().any(
                |element| matches!(element, Element::Text { text, .. } if text == "WORLD MAP")
            ),
            "the title belongs to the toolbar, not to the gutter"
        );
        for button in &rendered.buttons {
            assert!(button.rect.x >= rect.x && button.rect.y >= rect.y);
            assert!(button.rect.right() <= rect.right() && button.rect.bottom() <= rect.bottom());
        }
    }

    #[test]
    fn the_toolbar_reads_the_live_facts_and_refuses_an_unknown_field() {
        let palette = Palette::declared();
        let rect = Rect::new(329.0, 329.0, 480.0, 40.0);
        let document = Document::parse(MAP_TOOLBAR).expect("the toolbar parses");
        let facts = Facts {
            view: "mountain".to_string(),
            cell_m: 32.0,
            extent: [28, 32],
            pitch_deg: -24.0,
            zoom: 1,
            cursor: Some([64.0, 57.0, -12.0]),
        };
        let rendered =
            render_toolbar(&document, rect, &palette, &facts).expect("the toolbar draws");
        let texts: Vec<&str> = rendered.elements.iter().filter_map(Element::text).collect();
        for expected in [
            "WORLD MAP",
            "mountain",
            "32 m",
            "28×32",
            "-24°",
            "1",
            "64/-12/57",
        ] {
            assert!(
                texts.contains(&expected),
                "{expected:?} is missing from {texts:?}"
            );
        }
        for element in &rendered.elements {
            let at = element.rect();
            assert!(
                at.x >= rect.x
                    && at.y >= rect.y
                    && at.right() <= rect.right() + 0.01
                    && at.bottom() <= rect.bottom() + 0.01,
                "{element:?} escapes the toolbar"
            );
        }

        let unknown =
            Document::parse("<map-toolbar><fact label=\"X\" field=\"volcano\"/></map-toolbar>")
                .expect("parses");
        let refusal = render_toolbar(&unknown, rect, &palette, &facts)
            .expect_err("an unknown fact field is refused");
        assert!(refusal.contains("volcano"), "{refusal}");
    }

    fn element_paint(element: &Element) -> Option<Paint> {
        match element {
            Element::Surface { paint, .. } | Element::Rule { paint, .. } => Some(*paint),
            Element::Text { paint, .. } => Some(*paint),
        }
    }

    #[test]
    fn an_unknown_tag_or_unclosed_element_is_refused_by_name() {
        let bad =
            Document::parse("<map-controls><toast text=\"x\"/></map-controls>").expect("parses");
        let refusal = render(
            &bad,
            Rect::new(0.0, 0.0, 136.0, 200.0),
            &Palette::declared(),
            None,
            0.0,
        )
        .expect_err("an unknown control is refused");
        assert!(refusal.contains("toast"), "{refusal}");
        let refusal = Document::parse("<map-controls>").expect_err("an unclosed root");
        assert!(refusal.contains("never closed"), "{refusal}");
    }
}
