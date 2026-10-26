//! **A GPU layer, declared before it draws** — the machine half of M10.1, and the renderer M10.2
//! asks for.
//!
//! # Why a layer is a file rather than a block in the recipe
//!
//! `shploingus.recipe/3` has windows, rows and ballots, and **no layer rows**. Ruling R-64.1 says a
//! document that requires a field the schema does not have is a refusal the build surfaces — not a
//! schema change and not a silent extension. So a layer is a **sibling artifact the package names by
//! id in a `row`**: the recipe grammar is untouched, and a package that declares a layer and no row
//! that names it is refused here rather than mounted as if the layer were furniture.
//!
//! # What a layer may not do
//!
//! `C-33` and `AR15`: the look lives in one shader (`shading.wgsl`) and there is no second copy of
//! it. A layer may add **a pipeline, a bind group, an entry point or a uniform**; it may not write
//! the look. That rule is mechanical here — a `shader` naming `shading.wgsl`, or any path outside the
//! package's own `src/`, is refused **by name**, with the layer's id and the refused path in the same
//! sentence, because a refusal a reader has to decode is a refusal some readers will decode wrongly.
//!
//! # The grammar
//!
//! ```text
//! shploingus.layer/1
//! layer<TAB>id                       (the row id the package's recipe names)
//!     pipeline    world|body|cloud|sky|wire|terrain-lod
//!     shader      src/<id>.wgsl      (the layer's own file; `shading.wgsl` is refused)
//!     entry       fs_<name>|vs_<name>
//!     bind-group  <integer>
//!     uniform     <name> <type> <bytes>   (may be repeated, in declaration order)
//!     lod         <rung>|none        (the terrain rung this layer draws, M10.0.2)
//!     order       <integer>
//!     requires    <layer-id>[,...]|none
//! ```
//!
//! **Every listed key is required**, an unknown key is refused, and a duplicate key is refused. The
//! strictness is the recipe's own: a plausible value that silently overwrote a declared one is the
//! failure this crate exists against.
//!
//! # The two files are one declaration
//!
//! `layers/<id>.layer` is the machine part and `layers/<id>.md` is the record — the same split the
//! patch format already uses. [`Layer::render`] prints **both** as one human-readable block: the
//! layer's fields, then the record's four sections, verbatim. Nothing is written twice, and a record
//! missing `# Rollback` is refused by the section's own name rather than rendered as if it had one.
//!
//! **The render is byte-stable.** It is assembled from ordered vectors and the file's own lines, so
//! two runs over the same folder are the same bytes — which is the acceptance M10.2 asks for, and the
//! property a recipe loses the moment a `HashMap` reaches it.

use std::path::{Path, PathBuf};

use voxel_model::{RefusalKind, RefusalSet};

use crate::patch;

/// The pin. A different grammar is a different file, and an older one is refused rather than read.
pub const HEADER: &str = "shploingus.layer/1";
/// Every version this build accepts. Exactly one, for the recipe's own reason.
pub const ACCEPTED_VERSIONS: &[&str] = &[HEADER];

/// One layer's ceiling. A declaration, not a truncation: a layer larger than this is refused.
pub const MAX_LAYER_BYTES: usize = 64 * 1024;
/// How many uniforms one layer may declare. **Its own ceiling**, because a uniform is a fact a
/// shader reads and a bound nobody declared is a bound the driver chooses.
pub const MAX_UNIFORMS: usize = 32;

/// Every key a layer carries, **in grammar order**. The parser, the missing-key report and the
/// renderer all read this list; writing it out three times is three places to forget a field.
pub const KEYS: [&str; 8] = [
    "pipeline",
    "shader",
    "entry",
    "bind-group",
    "uniform",
    "lod",
    "order",
    "requires",
];

/// The look's own shader. A layer may name it and **may not own it** (`C-33`).
pub const THE_LOOK: &str = "shading.wgsl";

/// The pipeline a layer draws into. **The closed set the client has**, rather than a free string: a
/// package that names a pipeline the renderer does not run is a package that mounts and draws nothing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Pipeline {
    /// The near voxel world's opaque pass.
    World,
    /// The Sun and the Moon's body pass.
    Body,
    /// The cloud cubes.
    Cloud,
    /// The sky and atmosphere background.
    Sky,
    /// The wire and selection overlay.
    Wire,
    /// The terrain rung drawn past the near ring (M10.0.2).
    TerrainLod,
}

impl Pipeline {
    /// Every pipeline the client runs, in the renderer's own order.
    pub const ALL: [Pipeline; 6] = [
        Pipeline::World,
        Pipeline::Body,
        Pipeline::Cloud,
        Pipeline::Sky,
        Pipeline::Wire,
        Pipeline::TerrainLod,
    ];

    /// The pipeline's grammar word — the one a `.layer` writes.
    pub const fn name(self) -> &'static str {
        match self {
            Pipeline::World => "world",
            Pipeline::Body => "body",
            Pipeline::Cloud => "cloud",
            Pipeline::Sky => "sky",
            Pipeline::Wire => "wire",
            Pipeline::TerrainLod => "terrain-lod",
        }
    }

    /// Read a pipeline by its grammar word. `None` is a refusal the caller names.
    pub fn parse(text: &str) -> Option<Pipeline> {
        Pipeline::ALL.into_iter().find(|pipeline| pipeline.name() == text)
    }
}

/// One uniform a layer declares: its name, its WGSL type, and its size in bytes.
///
/// **The size is declared rather than measured here.** This module reads a declaration; laying the
/// buffer out is the client's job, and a size this module inferred from a type string would be a
/// second, unchecked answer to the question the declaration already answers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Uniform {
    /// The shader's binding name.
    pub name: String,
    /// The WGSL type, as the shader spells it (`mat4<f32>`, `u32`, `vec4<f32>`).
    pub ty: String,
    /// The declared size in bytes.
    pub bytes: u32,
}

/// One layer, as declared. [`Layer::parse`] is the only constructor and it checks every field.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Layer {
    /// The layer's id — **and the row id the package's recipe names it by**.
    pub id: String,
    /// The pipeline this layer draws into.
    pub pipeline: Pipeline,
    /// The layer's own shader path, package-relative. Never [`THE_LOOK`].
    pub shader: String,
    /// The shader's entry point.
    pub entry: String,
    /// The bind group the layer's uniforms sit in.
    pub bind_group: u32,
    /// The uniforms, in declaration order.
    pub uniforms: Vec<Uniform>,
    /// The terrain rung this layer draws, when it draws one.
    pub lod: Option<u32>,
    /// Draw order within the pipeline. Lower draws first.
    pub order: u32,
    /// Layers that must be registered before this one, by id.
    pub requires: Vec<String>,
}

/// Record a refusal and keep reading — **a block, not a value**.
///
/// `RefusalSet::refuse` returns `&mut Self` so a chain of checks reads as a list, but that return
/// type makes a `match` arm that refuses incompatible with the arm beside it that assigns a field.
/// The macro is the one-line answer, and the set is passed explicitly so the expansion cannot
/// capture the wrong `errors` in a module that has three.
macro_rules! refuse {
    ($errors:expr, $kind:expr, $detail:expr $(,)?) => {{
        $errors.refuse($kind, $detail);
    }};
}

impl Layer {
    /// **The only constructor that checks.** A wrong header, an unknown or duplicate key, a missing
    /// key, a shader outside the package, or `shading.wgsl` refuses here, with every problem named
    /// in one pass rather than one per round-trip.
    pub fn parse(source: &str) -> Result<Layer, RefusalSet> {
        let mut errors = RefusalSet::new();
        if source.len() > MAX_LAYER_BYTES {
            refuse!(errors, 
                RefusalKind::MalformedRecord,
                format!(
                    "layer declaration has {} bytes; the ceiling is {MAX_LAYER_BYTES}",
                    source.len()
                ),
            );
            return Err(errors);
        }

        let all: Vec<&str> = source.lines().collect();
        let header = all.first().copied().unwrap_or_default();
        if !ACCEPTED_VERSIONS.contains(&header) {
            refuse!(errors, 
                RefusalKind::MalformedRecord,
                format!(
                    "unknown layer schema {header:?}; this build accepts {ACCEPTED_VERSIONS:?}. A \
                     recipe header is not a layer, and reading one as the other would be a \
                     plausible substitute for a declared grammar"
                ),
            );
            return Err(errors);
        }

        // The `layer<TAB>id` row, exactly as a window block declares its id.
        let Some(declaration) = all.get(1).copied() else {
            refuse!(errors, 
                RefusalKind::MalformedRecord,
                "the layer declaration ends at the header; a layer needs a `layer<TAB>id` row",
            );
            return Err(errors);
        };
        let fields: Vec<&str> = declaration.split('\t').collect();
        let id = match fields.as_slice() {
            ["layer", id] if row_identifier(id) => id.to_string(),
            _ => {
                refuse!(errors, 
                    RefusalKind::MalformedRecord,
                    format!(
                        "expected `layer<TAB>id` after the header, found {declaration:?}; the id is \
                         the row id the package's recipe names, so it is the record's id shape \
                         (`LOD-0001`), never prose"
                    ),
                );
                String::new()
            }
        };

        let mut pipeline: Option<Pipeline> = None;
        let mut shader: Option<String> = None;
        let mut entry: Option<String> = None;
        let mut bind_group: Option<u32> = None;
        let mut uniforms: Vec<Uniform> = Vec::new();
        let mut lod: Option<Option<u32>> = None;
        let mut order: Option<u32> = None;
        let mut requires: Option<Vec<String>> = None;

        for line in all.iter().skip(2) {
            if line.trim().is_empty() || is_comment(line) {
                continue;
            }
            // **Indentation is presentational.** A key line is written `\tkey\tvalue` so the block
            // reads as one under its `layer<TAB>id` header; the tab that indents it is not a field.
            let fields: Vec<&str> = line.trim_start_matches('\t').split('\t').collect();
            let key = fields[0];
            let values: Vec<&str> = fields.iter().skip(1).map(|value| value.trim()).collect();
            match key {
                "pipeline" => {
                    if pipeline.is_some() {
                        refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?} declares `pipeline` twice"),
                        );
                    }
                    match values.as_slice() {
                        [word] => match Pipeline::parse(word) {
                            Some(parsed) => pipeline = Some(parsed),
                            None => refuse!(errors, 
                                RefusalKind::InvalidFieldSpec,
                                format!(
                                    "layer {id:?} names pipeline {word:?}, which this build does \
                                     not run; the pipelines are {:?}",
                                    Pipeline::ALL.map(Pipeline::name)
                                ),
                            ),
                        },
                        _ => refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?}: `pipeline` takes one word"),
                        ),
                    }
                }
                "shader" => {
                    if shader.is_some() {
                        refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?} declares `shader` twice"),
                        );
                    }
                    match values.as_slice() {
                        [path] => match owned_shader(path) {
                            Ok(()) => shader = Some((*path).to_string()),
                            Err(refusal) => {
                                errors.push(refusal);
                            }
                        },
                        _ => refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?}: `shader` takes one package-relative path"),
                        ),
                    }
                }
                "entry" => {
                    if entry.is_some() {
                        refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?} declares `entry` twice"),
                        );
                    }
                    match values.as_slice() {
                        [name] if entry_identifier(name) => entry = Some((*name).to_string()),
                        [name] => refuse!(errors, 
                            RefusalKind::InvalidFieldSpec,
                            format!(
                                "layer {id:?} names entry {name:?}; an entry is `fs_<name>` or \
                                 `vs_<name>`, and nothing else is a stage this build runs"
                            ),
                        ),
                        _ => refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?}: `entry` takes one name"),
                        ),
                    }
                }
                "bind-group" => {
                    if bind_group.is_some() {
                        refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?} declares `bind-group` twice"),
                        );
                    }
                    match values.as_slice() {
                        [number] => match number.parse::<u32>() {
                            Ok(parsed) if parsed.to_string() == *number => {
                                bind_group = Some(parsed)
                            }
                            _ => refuse!(errors, 
                                RefusalKind::InvalidFieldSpec,
                                format!("layer {id:?} binds group {number:?}, which is not an index"),
                            ),
                        },
                        _ => refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?}: `bind-group` takes one integer"),
                        ),
                    }
                }
                "uniform" => match values.as_slice() {
                    [name, ty, bytes] if !name.is_empty() && !ty.is_empty() => {
                        let bytes: Option<u32> = bytes.parse().ok();
                        match bytes {
                            Some(bytes) if uniforms.len() < MAX_UNIFORMS => {
                                if uniforms.iter().any(|uniform| uniform.name == *name) {
                                    refuse!(errors, 
                                        RefusalKind::MalformedRecord,
                                        format!("layer {id:?} declares uniform {name:?} twice"),
                                    );
                                } else {
                                    uniforms.push(Uniform {
                                        name: (*name).to_string(),
                                        ty: (*ty).to_string(),
                                        bytes,
                                    });
                                }
                            }
                            Some(_) => refuse!(errors, 
                                RefusalKind::MalformedRecord,
                                format!(
                                    "layer {id:?} declares more than {MAX_UNIFORMS} uniforms, and \
                                     a uniform nobody bounds is a bound the driver chooses"
                                ),
                            ),
                            None => refuse!(errors, 
                                RefusalKind::InvalidFieldSpec,
                                format!(
                                    "layer {id:?}: uniform {name:?} declares {bytes:?} bytes, and \
                                     a size is a whole number"
                                ),
                            ),
                        }
                    }
                    _ => refuse!(errors, 
                        RefusalKind::MalformedRecord,
                        format!(
                            "layer {id:?}: a uniform is `uniform<TAB>name<TAB>type<TAB>bytes`"
                        ),
                    ),
                },
                "lod" => {
                    if lod.is_some() {
                        refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?} declares `lod` twice"),
                        );
                    }
                    match values.as_slice() {
                        ["none"] => lod = Some(None),
                        [number] => match number.parse::<u32>() {
                            Ok(parsed) if parsed.to_string() == *number => {
                                lod = Some(Some(parsed))
                            }
                            _ => refuse!(errors, 
                                RefusalKind::InvalidFieldSpec,
                                format!(
                                    "layer {id:?} declares lod {number:?}; a rung is a whole number \
                                     or `none`"
                                ),
                            ),
                        },
                        _ => refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?}: `lod` takes one rung or `none`"),
                        ),
                    }
                }
                "order" => {
                    if order.is_some() {
                        refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?} declares `order` twice"),
                        );
                    }
                    match values.as_slice() {
                        [number] => match number.parse::<u32>() {
                            Ok(parsed) if parsed.to_string() == *number => order = Some(parsed),
                            _ => refuse!(errors, 
                                RefusalKind::InvalidFieldSpec,
                                format!("layer {id:?} draws at order {number:?}, which is not an \
                                         index"),
                            ),
                        },
                        _ => refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?}: `order` takes one integer"),
                        ),
                    }
                }
                "requires" => {
                    if requires.is_some() {
                        refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?} declares `requires` twice"),
                        );
                    }
                    match values.as_slice() {
                        ["none"] => requires = Some(Vec::new()),
                        [list] => {
                            let mut named: Vec<String> = Vec::new();
                            let mut refused = false;
                            for dependency in list.split(',') {
                                let dependency = dependency.trim();
                                if !row_identifier(dependency) {
                                    refused = true;
                                    refuse!(errors, 
                                        RefusalKind::InvalidFieldSpec,
                                        format!(
                                            "layer {id:?} requires {dependency:?}, which is not a \
                                             layer id"
                                        ),
                                    );
                                } else {
                                    named.push(dependency.to_string());
                                }
                            }
                            if !refused {
                                requires = Some(named);
                            }
                        }
                        _ => refuse!(errors, 
                            RefusalKind::MalformedRecord,
                            format!("layer {id:?}: `requires` takes ids or `none`"),
                        ),
                    }
                }
                _ => refuse!(errors, 
                    RefusalKind::UnadmittedFieldSpec,
                    format!(
                        "layer {id:?} declares key {key:?}, which is not in the grammar; the keys \
                         are {KEYS:?}"
                    ),
                ),
            }
        }

        let missing: Vec<&str> = [
            (pipeline.is_none(), "pipeline"),
            (shader.is_none(), "shader"),
            (entry.is_none(), "entry"),
            (bind_group.is_none(), "bind-group"),
            (lod.is_none(), "lod"),
            (order.is_none(), "order"),
            (requires.is_none(), "requires"),
        ]
        .into_iter()
        .filter_map(|(absent, key)| absent.then_some(key))
        .collect();
        for key in missing {
            refuse!(errors, 
                RefusalKind::MalformedRecord,
                format!("layer {id:?} declares no `{key}`, and every key is required"),
            );
        }

        errors.into_result(Layer {
            id,
            pipeline: pipeline.unwrap_or(Pipeline::World),
            shader: shader.unwrap_or_default(),
            entry: entry.unwrap_or_default(),
            bind_group: bind_group.unwrap_or(0),
            uniforms,
            lod: lod.flatten(),
            order: order.unwrap_or(0),
            requires: requires.unwrap_or_default(),
        })
    }

    /// The layer as one line: `LOD-0001 · terrain-lod · fs_horizon · lod 0 · order 10`.
    pub fn describe(&self) -> String {
        format!(
            "{} · {} · {} · lod {} · order {}",
            self.id,
            self.pipeline.name(),
            self.entry,
            self.lod
                .map_or_else(|| "none".to_string(), |rung| rung.to_string()),
            self.order
        )
    }

    /// **The human-readable block, from the two files and nothing else** (M10.2).
    ///
    /// The first line is the plan's own: `api-layer · <id> · pipeline <p> · entry <e> · uniforms <…>`.
    /// The fenced block repeats every field in grammar order, so a reader can check the declaration
    /// against the file; the record's four sections follow verbatim, so a reader never has to open
    /// the `.md` to find out why the layer exists.
    ///
    /// A record missing one of [`patch::SECTIONS`], or carrying an empty one, is refused by name.
    pub fn render(&self, record: &str) -> Result<String, RefusalSet> {
        let mut errors = RefusalSet::new();
        let mut sections: Vec<(&'static str, String)> = Vec::new();
        for heading in patch::SECTIONS {
            match section_of(record, heading) {
                Some(body) if !body.trim().is_empty() => sections.push((heading, body)),
                Some(_) => {
                    refuse!(errors, 
                        RefusalKind::MalformedRecord,
                        format!(
                            "layer {}'s record has an empty `{heading}` section, and a heading with \
                             nothing under it is a heading that was not written",
                            self.id
                        ),
                    );
                }
                None => {
                    refuse!(errors, 
                        RefusalKind::MalformedRecord,
                        format!(
                            "layer {}'s record declares no `{heading}` section; every record carries \
                             all four",
                            self.id
                        ),
                    );
                }
            }
        }
        errors.into_result(self.render_with(&sections))
    }

    fn render_with(&self, sections: &[(&'static str, String)]) -> String {
        let uniforms = if self.uniforms.is_empty() {
            "none".to_string()
        } else {
            self.uniforms
                .iter()
                .map(|uniform| format!("{}:{}@{}", uniform.name, uniform.ty, uniform.bytes))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let requires = if self.requires.is_empty() {
            "none".to_string()
        } else {
            self.requires.join(",")
        };
        let lod = self
            .lod
            .map_or_else(|| "none".to_string(), |rung| rung.to_string());

        let mut out = format!(
            "api-layer · {} · pipeline {} · entry {} · uniforms {}\n\n```api-layer\n",
            self.id,
            self.pipeline.name(),
            self.entry,
            uniforms
        );
        out.push_str(&format!("id          {}\n", self.id));
        out.push_str(&format!("pipeline    {}\n", self.pipeline.name()));
        out.push_str(&format!("shader      {}\n", self.shader));
        out.push_str(&format!("entry       {}\n", self.entry));
        out.push_str(&format!("bind-group  {}\n", self.bind_group));
        for uniform in &self.uniforms {
            out.push_str(&format!(
                "uniform     {} {} {}\n",
                uniform.name, uniform.ty, uniform.bytes
            ));
        }
        out.push_str(&format!("lod         {lod}\n"));
        out.push_str(&format!("order       {}\n", self.order));
        out.push_str(&format!("requires    {requires}\n"));
        out.push_str("```\n");
        for (heading, body) in sections {
            out.push('\n');
            out.push_str(heading);
            out.push('\n');
            out.push_str(body.trim_end());
            out.push('\n');
        }
        out
    }
}

/// A layer and the record that explains it, as a folder holds them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Declared {
    /// The parsed layer.
    pub layer: Layer,
    /// The record's text, exactly as `layers/<id>.md` holds it.
    pub record: String,
    /// The `.layer` file this was read from.
    pub file: PathBuf,
}

/// A display package as a folder holds it: the recipe, and every layer with its record.
///
/// **The recipe is the same object the shell mounts**, parsed by [`crate::recipe::Recipe::parse`]
/// rather than by a second reader here — a folder whose recipe and layers were read by two different
/// grammars would be two answers to what the package is.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Package {
    /// The folder this was read from.
    pub folder: PathBuf,
    /// The package's own recipe (`shploingus.recipe/3`).
    pub recipe: crate::recipe::Recipe,
    /// The layers the recipe names, each with its record.
    pub layers: Vec<Declared>,
}

/// **Every layer in a display package, with its record**, checked against the package's recipe.
///
/// Three refusals are the point of reading a folder rather than a file:
///
/// * a `.layer` with no `layers/<id>.md` is refused by path — the machine part without the record is
///   half a declaration;
/// * a declared layer the package's `package.shspec` names by **no row** is refused, because a layer
///   nothing names is a layer no reader can find and no gate can check (M10.1 item 3);
/// * a declared shader with no file under the package's `src/` is refused, because a layer that names
///   a shader it does not carry is a pass that draws nothing.
pub fn read(folder: &Path) -> Result<Package, RefusalSet> {
    let mut errors = RefusalSet::new();
    let layers_dir = folder.join("layers");
    let source = folder.join("package.shspec");
    let recipe = match std::fs::read_to_string(&source) {
        Ok(text) => match crate::recipe::Recipe::parse(text.as_bytes()) {
            Ok(recipe) => Some(recipe),
            Err(found) => {
                for refusal in found.all() {
                    refuse!(errors, 
                        refusal.kind(),
                        format!("{}: {}", source.display(), refusal.detail()),
                    );
                }
                None
            }
        },
        Err(why) => {
            refuse!(errors, 
                RefusalKind::MalformedRecord,
                format!(
                    "{} could not be read ({why}); a display package's source is the recipe it \
                     mounts",
                    source.display()
                ),
            );
            None
        }
    };
    // **No fake recipe.** A package whose recipe did not parse has nothing to name its layers
    // against, and continuing would print a render for a package that cannot be mounted.
    let Some(recipe) = recipe else {
        return Err(errors);
    };

    let mut files: Vec<PathBuf> = match std::fs::read_dir(&layers_dir) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("layer"))
            .collect(),
        // **A package with no layer is a real state**, not a missing file: the shell draws through
        // the renderer's own pipelines and declares none. An *unreadable* `layers/` is still a
        // refusal — absence and a permission error are different facts, and reading one as the
        // other would make a package look empty when nothing was measured.
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(why) => {
            refuse!(errors, 
                RefusalKind::MalformedRecord,
                format!("{} could not be read ({why})", layers_dir.display()),
            );
            return Err(errors);
        }
    };
    files.sort();

    let mut declared = Vec::new();
    for file in files {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let text = match std::fs::read_to_string(&file) {
            Ok(text) => text,
            Err(why) => {
                refuse!(errors, 
                    RefusalKind::MalformedRecord,
                    format!("{} could not be read ({why})", file.display()),
                );
                continue;
            }
        };
        let layer = match Layer::parse(&text) {
            Ok(layer) => layer,
            Err(found) => {
                for refusal in found.all() {
                    refuse!(errors, 
                        refusal.kind(),
                        format!("{}: {}", file.display(), refusal.detail()),
                    );
                }
                continue;
            }
        };
        // The file names the id, and the id names the file — two answers to one question must not
        // disagree, so a `LOD-0002.layer` holding `layer LOD-0001` is refused by both names.
        let stem = name.trim_end_matches(".layer");
        if stem != layer.id {
            refuse!(errors, 
                RefusalKind::MalformedRecord,
                format!(
                    "{} declares layer {:?}: a file names its own layer, and a layer named twice is \
                     a layer a reader will look for in the wrong place",
                    file.display(),
                    layer.id
                ),
            );
        }
        if !recipe.rows.iter().any(|row| row.id == layer.id) {
            refuse!(errors, 
                RefusalKind::UnadmittedFieldSpec,
                format!(
                    "{} declares layer {} but {} names it by no row; a layer the recipe cannot \
                     name is a layer the package does not carry",
                    file.display(),
                    layer.id,
                    source.display()
                ),
            );
        }
        // **The shader is part of the declaration.** A layer naming `src/x.wgsl` the folder does not
        // hold is a pass that draws nothing, and the parse's own rule (a file under `src/`) would
        // read as satisfied while the file is absent.
        let shader_path = folder.join(&layer.shader);
        match std::fs::metadata(&shader_path) {
            Ok(metadata) if metadata.is_file() && metadata.len() > 0 => {}
            Ok(metadata) => refuse!(errors, 
                RefusalKind::MalformedRecord,
                format!(
                    "{} declares shader {} and {} holds {} bytes; a layer that names an empty \
                     shader is a pass that draws nothing",
                    file.display(),
                    layer.shader,
                    shader_path.display(),
                    metadata.len()
                ),
            ),
            Err(why) => refuse!(errors, 
                RefusalKind::MalformedRecord,
                format!(
                    "{} declares shader {} and {} could not be read ({why}); a layer draws with a \
                     file the package carries and with nothing else",
                    file.display(),
                    layer.shader,
                    shader_path.display()
                ),
            ),
        }
        let record_path = layers_dir.join(format!("{}.md", layer.id));
        let record = match std::fs::read_to_string(&record_path) {
            Ok(record) => record,
            Err(why) => {
                refuse!(errors, 
                    RefusalKind::MalformedRecord,
                    format!(
                        "{} declares layer {} and {} could not be read ({why}); the machine part \
                         without the record is half a declaration",
                        file.display(),
                        layer.id,
                        record_path.display()
                    ),
                );
                continue;
            }
        };
        if let Err(found) = layer.render(&record) {
            for refusal in found.all() {
                refuse!(errors, 
                    refusal.kind(),
                    format!("{}: {}", record_path.display(), refusal.detail()),
                );
            }
            continue;
        }
        declared.push(Declared { layer, record, file });
    }
    errors.into_result(Package {
        folder: folder.to_path_buf(),
        recipe,
        layers: declared,
    })
}

/// **A package rendered as one human-readable text** — the command's whole body, so the bin is thin
/// and a test can ask the same question the user asks.
///
/// The acceptance M10.2 states is *"names every layer, every window and every row"*, so the header
/// is the recipe's own: its name and version, every window id, every home (tab, bar, filed, pin),
/// and every row with the record's words. Then one heading and one block per layer.
///
/// Two runs over the same folder are byte-identical; a refusal is the caller's to print, with every
/// problem named.
pub fn render_package(folder: &Path) -> Result<String, RefusalSet> {
    let package = read(folder)?;
    let recipe = &package.recipe;
    let mut out = format!("# {}\n", display_path(&package.folder));
    out.push_str(&format!("package     {} {}\n", recipe.name, recipe.version));
    if recipe.windows.is_empty() {
        out.push_str("windows     none\n");
    } else {
        out.push_str(&format!(
            "windows     {}\n",
            recipe
                .windows
                .iter()
                .map(|window| window.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out.push_str(&format!("tabs        {}\n", homes(&recipe.tabs)));
    out.push_str(&format!("bar         {}\n", homes(&recipe.bar)));
    out.push_str(&format!(
        "filed       {}\n",
        if recipe.filed.is_empty() {
            "none".to_string()
        } else {
            recipe
                .filed
                .iter()
                .map(|(window, tab)| format!("{window}->{tab}"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));
    out.push_str(&format!("pins        {}\n", homes(&recipe.pinned)));
    if recipe.rows.is_empty() {
        out.push_str("rows        none\n");
    } else {
        out.push_str(&format!("rows        {}\n", recipe.rows.len()));
        for row in &recipe.rows {
            out.push_str(&format!("row         {}  {}\n", row.id, row.text));
        }
    }
    if package.layers.is_empty() {
        out.push_str("layers      none declared\n");
    } else {
        out.push_str(&format!("layers      {}\n", package.layers.len()));
    }
    // **The frames are read with the recipe this render already parsed**, so the count and the
    // blocks below are the same join the mount checks — a frame the recipe cannot name refuses here
    // exactly as it refuses at mount, rather than being printed as if it were mountable.
    let frames = crate::cellframe::read(folder, recipe)?;
    if frames.frames.is_empty() {
        out.push_str("frames      none declared\n");
    } else {
        out.push_str(&format!("frames      {}\n", frames.frames.len()));
        for entry in &frames.frames {
            out.push_str(&format!("frame       {}  {}\n", entry.frame.id, entry.frame.describe()));
        }
    }
    for entry in package.layers {
        out.push_str(&format!("\n## {}\n\n", display_path(&entry.file)));
        match entry.layer.render(&entry.record) {
            Ok(rendered) => out.push_str(&rendered),
            // `read` already rendered once; a second failure here would be a different answer to
            // the same question, which this module does not have.
            Err(found) => return Err(found),
        }
    }
    for entry in frames.frames {
        out.push_str(&format!("\n## {}\n\n", display_path(&entry.file)));
        match entry.frame.render(&entry.record) {
            Ok(rendered) => out.push_str(&rendered),
            Err(found) => return Err(found),
        }
    }
    Ok(out)
}

/// A list of ids as a line prints them, or `none` — **absence spelled as a word**, never an empty
/// value that reads as a missing field.
fn homes(ids: &[String]) -> String {
    if ids.is_empty() {
        "none".to_string()
    } else {
        ids.join(", ")
    }
}

/// A path as a recipe writes it: `/` separators, so a render does not change with the platform it
/// was made on. A recipe that reads differently on Windows is not byte-stable in the sense M10.2
/// means, and the baseline record is read by whoever comes next, not by the machine that wrote it.
fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The text under one heading, up to the next heading of the same kind — or `None`.
fn section_of(record: &str, heading: &str) -> Option<String> {
    let lines: Vec<&str> = record.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.trim_end() == heading)?;
    let mut body = Vec::new();
    for line in lines.iter().skip(start + 1) {
        if patch::SECTIONS.contains(&line.trim_end()) {
            break;
        }
        body.push(*line);
    }
    Some(body.join("\n"))
}

/// The record's id shape, **the recipe's own**: the layer's id is the row id a package names.
///
/// **Shared with [`crate::cellframe`] rather than copied.** A frame is named by a row exactly as a
/// layer is, so the two artifacts must agree about what an id is; a second predicate here would be
/// two answers to one question, and the first id one reader accepted would be the other's refusal.
pub(crate) fn row_identifier(text: &str) -> bool {
    if text.is_empty() || text.len() > 64 {
        return false;
    }
    let Some((kind, number)) = text.rsplit_once('-') else {
        return letters_then_digits(text);
    };
    !kind.is_empty()
        && kind.len() <= 32
        && kind.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_uppercase())
        && !number.is_empty()
        && number.len() <= 8
        && number.bytes().all(|b| b.is_ascii_digit())
}

fn letters_then_digits(text: &str) -> bool {
    let split = text.find(|c: char| c.is_ascii_digit()).unwrap_or(0);
    split > 0
        && split <= 32
        && text[..split].bytes().all(|b| b.is_ascii_alphabetic())
        && !text[split..].is_empty()
        && text[split..].bytes().all(|b| b.is_ascii_digit())
}

/// A shader entry point: `fs_` or `vs_`, then a canonical name.
fn entry_identifier(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("fs_").or_else(|| text.strip_prefix("vs_")) else {
        return false;
    };
    !rest.is_empty()
        && rest.len() <= 64
        && rest
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && !rest.starts_with(|c: char| c.is_ascii_digit())
}

/// **The rule `C-33` states, made mechanical**: a layer owns a file under `src/` and may not own the
/// look, may not reach outside the package, and may not name a path that is not WGSL.
fn owned_shader(path: &str) -> Result<(), voxel_model::Refusal> {
    if path == THE_LOOK || path.ends_with(&format!("/{THE_LOOK}")) {
        return Err(voxel_model::Refusal::owned(
            RefusalKind::UnadmittedFieldSpec,
            format!(
                "layer names shader {path:?}, and {THE_LOOK} is the look: a layer may add a \
                 pipeline, a bind group, an entry point or a uniform, and it may not write the one \
                 shader every window shares (C-33, AR15)"
            ),
        ));
    }
    if !path.starts_with("src/") {
        return Err(voxel_model::Refusal::owned(
            RefusalKind::UnadmittedFieldSpec,
            format!(
                "layer names shader {path:?}, which is not under the package's own `src/`; a layer \
                 draws with a file the package carries and with nothing else"
            ),
        ));
    }
    if path.contains("..") || path.contains('\\') {
        return Err(voxel_model::Refusal::owned(
            RefusalKind::UnadmittedFieldSpec,
            format!(
                "layer names shader {path:?}, which is not a package-relative path: `..` and `\\` \
                 are how a declaration reaches a file the package does not own"
            ),
        ));
    }
    if !path.ends_with(".wgsl") {
        return Err(voxel_model::Refusal::owned(
            RefusalKind::InvalidFieldSpec,
            format!(
                "layer names shader {path:?}, and a shader is a `.wgsl` file this build can read"
            ),
        ));
    }
    Ok(())
}

/// A whole-line comment: a line whose first non-tab character is `#`. Skipped whole, so it can never
/// contribute a key.
fn is_comment(line: &str) -> bool {
    line.trim_start_matches('\t')
        .trim_start()
        .starts_with('#')
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
shploingus.layer/1
layer\tLOD-0001
\tpipeline\tterrain-lod
\tshader\tsrc/LOD-0001.wgsl
\tentry\tfs_horizon
\tbind-group\t1
\tuniform\tview_proj\tmat4<f32>\t64
\tuniform\trung\tu32\t4
\tlod\t0
\torder\t10
\trequires\tnone
";

    const RECORD: &str = "\
# Change
Draw the far ring from the rung's own record.

# Acceptance
The horizon sweep draws the rung with no hole and no double geometry.

# Evidence
docs/baselines/mount-v2/layer-recipes-2026-10-24.txt

# Rollback
Delete the layer and its row; the near ring is untouched.
";

    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("the crate sits two levels under the workspace")
            .to_path_buf()
    }

    fn display_packages() -> PathBuf {
        workspace().join("display-packages")
    }

    #[test]
    fn the_fixture_layer_parses_and_names_every_field() {
        let layer = Layer::parse(FIXTURE).expect("the fixture parses");
        assert_eq!(layer.id, "LOD-0001");
        assert_eq!(layer.pipeline, Pipeline::TerrainLod);
        assert_eq!(layer.shader, "src/LOD-0001.wgsl");
        assert_eq!(layer.entry, "fs_horizon");
        assert_eq!(layer.bind_group, 1);
        assert_eq!(layer.uniforms.len(), 2);
        assert_eq!(layer.uniforms[0].ty, "mat4<f32>");
        assert_eq!(layer.lod, Some(0));
        assert_eq!(layer.order, 10);
        assert!(layer.requires.is_empty());
    }

    #[test]
    fn a_layer_that_writes_the_look_is_refused_by_name() {
        let source = FIXTURE.replace("src/LOD-0001.wgsl", "shading.wgsl");
        let found = Layer::parse(&source).expect_err("the look is not a layer's to write");
        let text = RefusalSet::to_string(&found);
        assert!(text.contains("shading.wgsl"), "{text}");
        assert!(text.contains("C-33"), "{text}");
        assert!(found
            .all()
            .iter()
            .any(|refusal| refusal.kind() == RefusalKind::UnadmittedFieldSpec));
    }

    #[test]
    fn a_shader_outside_the_package_is_refused() {
        for path in ["../src/x.wgsl", "vendor/x.wgsl", "src/../shading.wgsl"] {
            let source = FIXTURE.replace("src/LOD-0001.wgsl", path);
            assert!(
                Layer::parse(&source).is_err(),
                "{path:?} is outside the package and must be refused"
            );
        }
    }

    #[test]
    fn an_unknown_key_is_refused_and_named() {
        let source = format!("{FIXTURE}\tcolour\tteal\n");
        let text = RefusalSet::to_string(&Layer::parse(&source).expect_err("unknown key"));
        assert!(text.contains("colour"), "{text}");
    }

    #[test]
    fn a_duplicate_key_is_refused() {
        let source = format!("{FIXTURE}\torder\t20\n");
        let text = RefusalSet::to_string(&Layer::parse(&source).expect_err("duplicate key"));
        assert!(text.contains("twice"), "{text}");
    }

    #[test]
    fn a_missing_key_is_refused_with_its_name() {
        let source = FIXTURE.replace("\torder\t10\n", "");
        let text = RefusalSet::to_string(&Layer::parse(&source).expect_err("missing key"));
        assert!(text.contains("`order`"), "{text}");
    }

    #[test]
    fn a_recipe_header_is_not_a_layer() {
        let text = RefusalSet::to_string(
            &Layer::parse("shploingus.recipe/3\npackage\tx\tv0\n").expect_err("wrong header"),
        );
        assert!(text.contains("shploingus.layer/1"), "{text}");
    }

    #[test]
    fn a_render_carries_the_block_and_all_four_sections() {
        let layer = Layer::parse(FIXTURE).expect("fixture");
        let rendered = layer.render(RECORD).expect("the record is complete");
        assert!(
            rendered.starts_with("api-layer · LOD-0001 · pipeline terrain-lod · entry fs_horizon"),
            "{rendered}"
        );
        assert!(rendered.contains("```api-layer"), "{rendered}");
        assert!(rendered.contains("uniform     view_proj mat4<f32> 64"), "{rendered}");
        for heading in patch::SECTIONS {
            assert!(rendered.contains(heading), "missing {heading}: {rendered}");
        }
    }

    #[test]
    fn a_record_missing_a_section_is_refused_by_name() {
        let without = RECORD.replace("# Rollback\nDelete the layer and its row; the near ring is untouched.\n", "");
        let layer = Layer::parse(FIXTURE).expect("fixture");
        let text = RefusalSet::to_string(&layer.render(&without).expect_err("no rollback"));
        assert!(text.contains("# Rollback"), "{text}");
    }

    #[test]
    fn the_render_is_byte_stable_across_two_reads_and_names_the_whole_package() {
        let folder = display_packages().join("horizon-lod");
        let once = render_package(&folder).expect("first read");
        let twice = render_package(&folder).expect("second read");
        assert_eq!(once, twice);
        assert!(once.contains("api-layer · LOD-0001"), "{once}");
        // M10.2's acceptance names every layer, every window and every row — so the header is
        // checked for each of the three, not just for the layer block.
        assert!(once.contains("package     horizon-lod v0"), "{once}");
        assert!(once.contains("windows     none"), "{once}");
        assert!(once.contains("row         LOD-0001"), "{once}");
    }

    #[test]
    fn every_display_package_reads_with_every_layer_named_and_recorded() {
        let root = display_packages();
        let entries = std::fs::read_dir(&root).expect("display-packages/ is on disk");
        let mut packages = 0;
        for entry in entries.flatten() {
            let folder = entry.path();
            if !folder.is_dir() || !folder.join("package.shspec").is_file() {
                continue;
            }
            packages += 1;
            render_package(&folder).unwrap_or_else(|found| {
                panic!("{}: {}", folder.display(), RefusalSet::to_string(&found))
            });
        }
        assert!(packages > 0, "no display package was measured, so nothing was checked");
    }
}
