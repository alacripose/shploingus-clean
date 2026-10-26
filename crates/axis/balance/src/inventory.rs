//! **The public visual items this tree declares, and the renderer each one reaches.**
//!
//! A file walk can count code; it cannot tell whether a construct is drawn. The renderer for a
//! panel is a WGSL fragment entry point, the renderer for a line of text is a Rust type from a
//! shaping engine, and a struct that nothing on the draw path names looks identical to one that
//! is on screen every frame. So the inventory is **declared** — one row per item — and every row
//! is then **held to the tree**:
//!
//! | what is checked | against | a failure is |
//! |---|---|---|
//! | the item's site exists and still contains its declaration | the file the row names | [`RefusalKind::FabricatedFeature`] |
//! | a shader renderer's entry point exists in its file | `fn <entry>(` in the WGSL | [`RefusalKind::MissingStage`] |
//! | a Rust renderer's name exists at its site | the file the renderer names | [`RefusalKind::MissingStage`] |
//!
//! **A row with `renderer: None` is the census's finding**, not a row that failed a check: the
//! item exists, its declaration is witnessable, and no draw path reaches it. It is counted in
//! [`Inventory::unrendered`] and printed with its site.
//!
//! # Why this list is complete for the closed sets it names
//!
//! `voxel_ui::Element` is the shell's only element vocabulary, and it is closed at three
//! variants. [`tests::the_element_vocabulary_has_no_unlisted_variant`] reads the enum out of
//! `voxel-ui/src/lib.rs` and asserts that each variant has a row here — so adding a fourth
//! element without adding its row is a failing test rather than an item nobody counted.
//!
//! # What is deliberately one row
//!
//! Four HUD panels, six window contents and every menu are **compositions** of surfaces, rules
//! and text. They are counted as their own public items because they are separate things a
//! person reads, and their renderer is the same one the text row names: the HUD's draw path. A
//! row per emitted rectangle would be a count of a frame, not of the code.

use std::path::Path;

use voxel_model::refusal::RefusalKind;

/// Where an item is drawn: a shader entry point, or a renderer in the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Renderer {
    /// A WGSL entry point: the file, and the function the pipeline names.
    Shader {
        /// The shader's path, relative to the workspace root.
        file: &'static str,
        /// The entry point's name, as `fn <entry>(` appears in that file.
        entry: &'static str,
    },
    /// A Rust renderer that is not WGSL: the file, and the declaration a reader can find there.
    Rust {
        /// The renderer's path, relative to the workspace root.
        site: &'static str,
        /// The declaration that proves the renderer is the one the row means.
        declaration: &'static str,
    },
}

impl Renderer {
    /// The file this renderer is checked in.
    pub const fn site(self) -> &'static str {
        match self {
            Renderer::Shader { file, .. } => file,
            Renderer::Rust { site, .. } => site,
        }
    }

    /// One line a report can print.
    pub fn describe(self) -> String {
        match self {
            Renderer::Shader { file, entry } => format!("{file}:{entry}"),
            Renderer::Rust { site, declaration } => format!("{site}:{declaration}"),
        }
    }
}

/// One public visual item: what it is, where its declaration lives, and where it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    /// The item's name, in the vocabulary a person reads.
    pub name: &'static str,
    /// The file that declares it.
    pub site: &'static str,
    /// The declaration's own text, so the row cannot outlive the thing it names.
    pub declaration: &'static str,
    /// The renderer that draws it, or `None` for an item no draw path reaches.
    pub renderer: Option<Renderer>,
}

impl Item {
    /// Whether a frame draws this item.
    pub const fn rendered(self) -> bool {
        self.renderer.is_some()
    }
}

/// The shell's element vocabulary: the panels shader draws surfaces and rules, glyphs draw text.
const PANELS: Renderer = Renderer::Shader {
    file: "crates/voxel-client/src/panels.wgsl",
    entry: "fs",
};
/// The world's rectangle expansion: one quad record becomes two triangles.
const QUAD: Renderer = Renderer::Shader {
    file: "crates/voxel-client/src/shading.wgsl",
    entry: "vs_quad",
};
/// A body is one record and its own vertex stage.
const BODY: Renderer = Renderer::Shader {
    file: "crates/voxel-client/src/shading.wgsl",
    entry: "vs_body",
};
/// The debug wireframe's own stage.
const WIRE: Renderer = Renderer::Shader {
    file: "crates/voxel-client/src/shading.wgsl",
    entry: "vs_wire",
};
/// The sky's fullscreen triangle.
const SKY: Renderer = Renderer::Shader {
    file: "crates/voxel-client/src/sky.wgsl",
    entry: "vs_main",
};
/// The HUD's text: glyphon shapes and the client's own target draws.
const GLYPHS: Renderer = Renderer::Rust {
    site: "crates/voxel-client/src/hud.rs",
    declaration: "glyphon::TextRenderer",
};
/// The window layer: title bars, panes and their content, one draw path.
const WINDOWS: Renderer = Renderer::Rust {
    site: "crates/voxel-client/src/shell/mount.rs",
    declaration: "pub fn shell_elements(",
};

/// **Every public visual item this build declares**, in the order a report prints them.
pub const INVENTORY: [Item; 20] = [
    Item {
        name: "element-surface",
        site: "crates/voxel-ui/src/lib.rs",
        declaration: "Surface {",
        renderer: Some(PANELS),
    },
    Item {
        name: "element-rule",
        site: "crates/voxel-ui/src/lib.rs",
        declaration: "Rule {",
        renderer: Some(PANELS),
    },
    Item {
        name: "element-text",
        site: "crates/voxel-ui/src/lib.rs",
        declaration: "Text {",
        renderer: Some(GLYPHS),
    },
    Item {
        name: "world-rectangle",
        site: "crates/voxel-client/src/quad.rs",
        declaration: "FACES_BY_INDEX",
        renderer: Some(QUAD),
    },
    Item {
        name: "world-body",
        site: "crates/voxel-client/src/quad.rs",
        declaration: "BODY_KIND",
        renderer: Some(BODY),
    },
    Item {
        name: "world-wireframe",
        site: "crates/voxel-client/src/world.rs",
        declaration: "wire_pipeline",
        renderer: Some(WIRE),
    },
    Item {
        name: "world-sky",
        site: "crates/voxel-client/src/world.rs",
        declaration: "sky_pipeline",
        renderer: Some(SKY),
    },
    Item {
        name: "world-clouds",
        site: "crates/voxel-client/src/cloud_cubes.rs",
        declaration: "pub fn veil_with_side",
        renderer: Some(QUAD),
    },
    Item {
        name: "hud-panels",
        site: "crates/voxel-client/src/hud.rs",
        declaration: "pub fn world_panel",
        renderer: Some(GLYPHS),
    },
    Item {
        name: "console-records",
        site: "crates/voxel-client/src/shell/console.rs",
        declaration: "pub fn visible",
        renderer: Some(WINDOWS),
    },
    Item {
        name: "window-api",
        site: "crates/voxel-client/src/shell/api.rs",
        declaration: "pub struct ApiView",
        renderer: Some(WINDOWS),
    },
    Item {
        name: "window-titles",
        site: "crates/voxel-client/src/shell/titlebar.rs",
        declaration: "pub fn control_lanes",
        renderer: Some(WINDOWS),
    },
    Item {
        name: "window-panes",
        site: "crates/voxel-client/src/shell/pane.rs",
        declaration: "pub fn body",
        renderer: Some(WINDOWS),
    },
    Item {
        name: "window-splitters",
        site: "crates/voxel-client/src/shell/splitter.rs",
        declaration: "pub fn seam_at",
        renderer: Some(WINDOWS),
    },
    Item {
        name: "window-toasts",
        site: "crates/voxel-client/src/shell/notifications.rs",
        declaration: "pub struct ToastQueue",
        renderer: Some(WINDOWS),
    },
    Item {
        name: "window-placeholder",
        site: "crates/voxel-client/src/shell/placeholder.rs",
        declaration: "pub fn offer_lanes",
        renderer: Some(WINDOWS),
    },
    Item {
        name: "window-drop-zone",
        site: "crates/voxel-client/src/shell/drop.rs",
        declaration: "pub enum DropZone",
        renderer: Some(WINDOWS),
    },
    Item {
        name: "debug-lanes",
        site: "crates/voxel-client/src/shell/debug.rs",
        declaration: "pub fn ui_lanes",
        renderer: Some(WINDOWS),
    },
    Item {
        name: "desktop",
        site: "crates/voxel-client/src/shell/desktop.rs",
        declaration: "pub struct Desktop",
        renderer: None,
    },
    Item {
        name: "taskbar",
        site: "crates/voxel-client/src/shell/taskbar.rs",
        declaration: "pub fn task_press",
        renderer: None,
    },
];

/// One row that could not be witnessed, with the fact and the file it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowRefusal {
    /// The item's name.
    pub item: &'static str,
    /// Why the row is refused.
    pub kind: RefusalKind,
    /// The sentence a reader acts on: the file, the declaration and what was there instead.
    pub detail: String,
}

/// The inventory's answer: how many items are rendered, which are not, and which rows lied.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory {
    /// How many rows were read.
    pub total: usize,
    /// How many of them name a renderer that exists.
    pub rendered: usize,
    /// The rows that name no renderer at all — the census's finding.
    pub unrendered: Vec<Item>,
    /// The rows that could not be witnessed, and are therefore in neither count.
    pub refusals: Vec<RowRefusal>,
}

/// **Hold every row to the tree.** A row that names a file or a declaration it does not have is
/// refused; an item with no renderer is counted as unrendered rather than as a refusal.
pub fn check(root: &Path, items: &[Item]) -> Inventory {
    let mut inventory = Inventory { total: items.len(), ..Inventory::default() };
    for item in items {
        let Some(text) = read(root, item.site) else {
            inventory.refusals.push(RowRefusal {
                item: item.name,
                kind: RefusalKind::FabricatedFeature,
                detail: format!(
                    "{} names {} at {}, and the file could not be read",
                    item.name, item.declaration, item.site
                ),
            });
            continue;
        };
        if !text.contains(item.declaration) {
            inventory.refusals.push(RowRefusal {
                item: item.name,
                kind: RefusalKind::FabricatedFeature,
                detail: format!(
                    "{} names {:?} at {}, which no longer contains it",
                    item.name, item.declaration, item.site
                ),
            });
            continue;
        }
        match item.renderer {
            None => inventory.unrendered.push(*item),
            Some(renderer) => match witness(root, renderer) {
                Ok(()) => inventory.rendered += 1,
                Err(detail) => inventory.refusals.push(RowRefusal {
                    item: item.name,
                    kind: RefusalKind::MissingStage,
                    detail,
                }),
            },
        }
    }
    inventory
}

fn witness(root: &Path, renderer: Renderer) -> Result<(), String> {
    match renderer {
        Renderer::Shader { file, entry } => {
            let text = read(root, file)
                .ok_or_else(|| format!("the shader {file} could not be read, so {entry} was not seen"))?;
            let needle = format!("fn {entry}(");
            if text.contains(&needle) {
                Ok(())
            } else {
                Err(format!("{file} declares no {needle}, so the pipeline has no stage"))
            }
        }
        Renderer::Rust { site, declaration } => {
            let text = read(root, site)
                .ok_or_else(|| format!("the renderer {site} could not be read, so nothing was seen"))?;
            if text.contains(declaration) {
                Ok(())
            } else {
                Err(format!("{site} no longer declares {declaration:?}, so the renderer is gone"))
            }
        }
    }
}

fn read(root: &Path, relative: &str) -> Option<String> {
    std::fs::read_to_string(root.join(relative)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "voxel-axis-balance-inventory-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("a scratch tree");
        root
    }

    fn real_root() -> std::path::PathBuf {
        voxel_package::features::workspace_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("this crate sits inside the workspace")
    }

    #[test]
    fn the_element_vocabulary_has_no_unlisted_variant() {
        let source = std::fs::read_to_string(real_root().join("crates/voxel-ui/src/lib.rs"))
            .expect("the element vocabulary");
        let body = source
            .split("pub enum Element {")
            .nth(1)
            .expect("the enum body");
        let variants: Vec<&str> = body
            .lines()
            .map(str::trim)
            .take_while(|line| !line.starts_with('}'))
            .filter_map(|line| line.split([' ', '{']).next())
            .filter(|word| word.chars().next().is_some_and(char::is_uppercase))
            .collect();
        assert_eq!(variants, ["Surface", "Rule", "Text"], "the vocabulary moved");
        for variant in variants {
            let listed = INVENTORY.iter().any(|item| {
                item.site == "crates/voxel-ui/src/lib.rs"
                    && item.declaration == format!("{variant} {{")
            });
            assert!(listed, "{variant} has no row in the inventory");
        }
    }

    #[test]
    fn every_row_is_witnessed_and_the_unrendered_count_is_the_one_this_build_states() {
        let inventory = check(&real_root(), &INVENTORY);
        assert_eq!(inventory.total, INVENTORY.len());
        assert!(
            inventory.refusals.is_empty(),
            "the inventory was refused: {:#?}",
            inventory.refusals
        );
        assert_eq!(
            inventory.rendered + inventory.unrendered.len(),
            inventory.total,
            "a row is rendered or unrendered, never both and never neither"
        );
        let names: Vec<&str> = inventory.unrendered.iter().map(|item| item.name).collect();
        assert_eq!(
            names,
            ["desktop", "taskbar"],
            "the unrendered items are the ones no draw path reaches"
        );
    }

    #[test]
    fn a_row_that_names_a_declaration_it_does_not_have_is_refused() {
        let root = scratch("fabricated");
        std::fs::write(root.join("src/lib.rs"), "pub struct Nothing;\n").expect("a file");
        let item = Item {
            name: "invented",
            site: "src/lib.rs",
            declaration: "pub fn drawn",
            renderer: None,
        };
        let inventory = check(&root, &[item]);
        assert_eq!(inventory.rendered, 0);
        assert!(inventory.unrendered.is_empty(), "a refused row is not counted");
        assert_eq!(inventory.refusals.len(), 1);
        assert_eq!(inventory.refusals[0].kind, RefusalKind::FabricatedFeature);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_renderer_whose_stage_is_gone_is_a_missing_stage_and_not_a_pass() {
        let root = scratch("missing-stage");
        std::fs::write(root.join("src/lib.rs"), "pub fn drawn() {}\n").expect("a file");
        std::fs::write(root.join("src/s.wgsl"), "@fragment\nfn other() {}\n").expect("a shader");
        let item = Item {
            name: "item",
            site: "src/lib.rs",
            declaration: "pub fn drawn",
            renderer: Some(Renderer::Shader { file: "src/s.wgsl", entry: "fs" }),
        };
        let inventory = check(&root, &[item]);
        assert_eq!(inventory.rendered, 0);
        assert_eq!(inventory.refusals.len(), 1);
        assert_eq!(inventory.refusals[0].kind, RefusalKind::MissingStage);
        assert!(inventory.refusals[0].detail.contains("no fn fs("));
        let _ = std::fs::remove_dir_all(&root);
    }
}
