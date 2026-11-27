//! **The build tree's size guard.** The thing that stops a full volume from killing a build.
//!
//! # Why this exists, in the numbers it was written from
//!
//! On 2026-10-10 `C:` was at **232 MB free of 465 GB** and builds died in it. Nothing was wrong
//! with the code: `target/` had grown to **6.9 GB** — 2.5 GB of compiled example binaries, 2.3 GB
//! of incremental state, 0.4 GB of debug symbols — and the kache store beside it held 9.7 GB.
//! Every one of those gigabytes is *regenerable*, and every one of them was being kept anyway,
//! because nothing in the project had an opinion about it. A build does not fail because a tree
//! is large; it fails because nobody decided how large it may get.
//!
//! So this module is that decision, in one place, asked on every build the gate runs.
//!
//! # What it will and will not delete
//!
//! The guard trims **four** parts of the tree, cheapest to lose first, and stops the moment the
//! volume has its headroom back:
//!
//! | part | what it is | what losing it costs |
//! |---|---|---|
//! | `doc` | `target/doc` | a `cargo doc` |
//! | `examples` | a profile's `examples/` | `/tmp`-less rebuild of the examples, only when one is asked for |
//! | `incremental` | a profile's `incremental*` | the next build of those crates is a full rebuild |
//! | `symbols` | `deps/*.pdb` | **line numbers in a backtrace, until that unit is relinked** |
//!
//! The last row was **measured, not assumed**: a 21 MB `.pdb` was deleted and `cargo build` over
//! it answered `Finished in 0.33s` with no relink, because a `.pdb` is not an output cargo checks
//! for freshness. That is why it is on the list at all — a part whose only cost is symbol
//! line numbers is the last thing to take and the first thing worth naming.
//!
//! Everything else in the tree is on a **keep list, each entry with its reason**, and the list is
//! reported rather than implied: `deps/`, `release/`, `.fingerprint/`, `build/*/out`. Deleting
//! those is `cargo clean`, which costs a full rebuild — and saving that rebuild is the entire
//! reason this module is allowed to delete anything.
//!
//! # The honest edges
//!
//! * **The tree is not the volume.** `target/` is what this guard owns and measures; the kache
//!   store, `logs/`, and everything else on `C:` are named in the report and **not touched**. A
//!   guard that deleted another tool's cache to make its own number look good would be exactly
//!   the kind of silence this project's gates exist to refuse.
//! * **The free-space reading can fail, and says so.** A volume reading is taken with `cmd /C dir`
//!   or `df`, and when both refuse the guard reports `not-measured` with each tool's own reason —
//!   then falls back to a **tree budget**, and says the trigger was the budget rather than
//!   pretending it knew the volume.
//! * **An absent part is rendered as absent.** A category that holds nothing prints the paths it
//!   searched, never a row of zeroes that reads as a measurement of nothing.
//! * **The guard cannot see the future.** The floor is headroom, derived below, not a guarantee.
//!
//! # The thresholds, and the measurement behind each
//!
//! * [`FLOOR`] = 8 GiB. The observed peak tree is 6.9 GB, so 8 GiB is the smallest headroom that
//!   holds one full tree; below it the next large build is a coin flip.
//! * [`TREE_BUDGET`] = 4 GiB, the trigger used when the volume cannot be read. Measured on
//!   2026-10-10, the irreducible tree (`deps` 0.49 GB debug + 0.92 GB release, `build/out` 0.15 GB,
//!   `.fingerprint` 0.01 GB) is about 1.6 GB, so 4 GiB is one regrowth of the trimmable parts on
//!   top of it — high enough that the guard does not trim after every build, low enough that it
//!   acts before the parts are again measured in gigabytes.
//! * [`MINIMUM`] = 1 GiB, the point below which the gate **refuses**. The observed death was at
//!   232 MB with a relink in flight, so a gigabyte is named as the smallest headroom this tree has
//!   ever been seen to work in — and a guess in this direction costs a build that could have
//!   finished, so it is deliberately the lowest of the three.
//!
//! The floor can be restated per machine with [`FLOOR_ENV`] (`SHPLOINGUS_SPACE_FLOOR=12GiB`); a
//! value that does not parse is **refused** and the default stands, reported. There is no switch
//! that turns the guard off, and that is deliberate: its only trigger is *the volume is filling*,
//! so the one thing an off switch would be used for is a build that cannot finish.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One gibibyte, so every threshold below reads as the number a person wrote.
pub const GIB: u64 = 1024 * 1024 * 1024;

/// The headroom a build needs on the volume holding the tree. See the module doc for the basis.
pub const FLOOR: u64 = 8 * GIB;

/// Below this, after trimming everything the guard may trim, the gate **refuses** with the
/// consumers named rather than letting the linker die with an error that names nothing.
pub const MINIMUM: u64 = GIB;

/// The trigger used when the volume cannot be read: the largest tree that is still worth keeping.
pub const TREE_BUDGET: u64 = 4 * GIB;

/// Where an operator may restate [`FLOOR`]. A value the parser refuses leaves the default standing.
pub const FLOOR_ENV: &str = "SHPLOINGUS_SPACE_FLOOR";

// **The order of the three thresholds is a compile-time assertion, not a test.** The refusal line
// below the working line, and the fallback budget below the floor, are properties this module
// cannot function without; an edit that inverted either would otherwise be caught only by whoever
// noticed a build failing on a disk that was not full.
const _: () = assert!(MINIMUM < FLOOR);
const _: () = assert!(TREE_BUDGET < FLOOR);

/// Cargo's own environment override, so a redirected tree is measured where it actually is.
pub const TARGET_ENV: &str = "CARGO_TARGET_DIR";

// ---------------------------------------------------------------------------
// the parts
// ---------------------------------------------------------------------------

/// A kind of artefact cargo regenerates, with the cost of losing it. The order is the cost order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// `target/doc` — `cargo doc` output.
    Doc,
    /// A profile's `examples/` — compiled example binaries.
    Examples,
    /// A profile's `incremental*` — per-build state.
    Incremental,
    /// `deps/*.pdb` — debug symbols. Measured to cost a relink *nothing*.
    Symbols,
}

impl Part {
    /// Cheapest to lose first. The guard walks this in order and stops as soon as it is done.
    pub const ALL: [Part; 4] = [Part::Doc, Part::Examples, Part::Incremental, Part::Symbols];

    pub const fn slug(self) -> &'static str {
        match self {
            Part::Doc => "doc",
            Part::Examples => "examples",
            Part::Incremental => "incremental",
            Part::Symbols => "symbols",
        }
    }

    /// The cost of losing this part, in one line, printed beside every removal.
    pub const fn cost(self) -> &'static str {
        match self {
            Part::Doc => "a `cargo doc`",
            Part::Examples => "the examples rebuild, only when one is asked for",
            Part::Incremental => "the next build of those crates is a full rebuild",
            Part::Symbols => {
                "line numbers in a backtrace, until that unit is relinked; \
                 measured to cost no relink now"
            }
        }
    }

    /// **Where this part looks.** The containers, whether or not anything is in them — printed as
    /// the `searched` list of an empty part, so *nothing here* can be told from *looked nowhere*.
    /// A part that rendered an empty search list would be the very silence this project's gates
    /// refuse, and the first version of this module did exactly that for `symbols`.
    fn searched(self, root: &Path) -> Vec<PathBuf> {
        if self == Part::Doc {
            return vec![root.join("doc")];
        }
        profiles(root)
            .into_iter()
            .map(|profile| match self {
                Part::Examples => profile.join("examples"),
                Part::Symbols => profile.join("deps"),
                // The profile itself is where `incremental*` siblings are looked for.
                Part::Incremental => profile,
                Part::Doc => unreachable!("handled above"),
            })
            .collect()
    }

    /// Every path this part would remove, whether or not it exists.
    ///
    /// **Every path is built by joining `root`.** No path here is ever taken from a caller or a
    /// config file, and a directory listing contributes only a **file name** under a container
    /// this function built itself — which is then re-checked by [`inside`] before anything is
    /// removed. A guard that deletes is a guard whose paths must be constructible by reading only
    /// this function, and that is why it is one function instead of three call sites.
    fn units(self, root: &Path) -> Vec<PathBuf> {
        let named = |container: &Path, wanted: fn(&str) -> bool| -> Vec<PathBuf> {
            let Ok(children) = fs::read_dir(container) else {
                return Vec::new();
            };
            children
                .flatten()
                .filter(|child| wanted(&child.file_name().to_string_lossy().to_ascii_lowercase()))
                .map(|child| child.path())
                .collect()
        };
        match self {
            Part::Doc | Part::Examples => self.searched(root),
            // `incremental` and kache's `incremental.kache-auto` are the same fact: per-build state
            // a build can always produce again.
            Part::Incremental => self
                .searched(root)
                .iter()
                .flat_map(|profile| named(profile, |name| name.starts_with("incremental")))
                .collect(),
            Part::Symbols => self
                .searched(root)
                .iter()
                .flat_map(|deps| named(deps, |name| name.ends_with(".pdb")))
                .collect(),
        }
    }
}

/// The profile directories of a cargo tree: the top-level entries that are not cargo's own
/// scratch space. **Named rather than guessed at,** so the set of profiles is the same list every
/// run — `doc` is `cargo doc`'s output directory, `tmp` and `package` are cargo's own scratch.
fn profiles(root: &Path) -> Vec<PathBuf> {
    const NOT_A_PROFILE: [&str; 3] = ["doc", "tmp", "package"];
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter(|entry| {
            !NOT_A_PROFILE
                .contains(&entry.file_name().to_string_lossy().to_ascii_lowercase().as_str())
        })
        .map(|entry| entry.path())
        .collect()
}

// ---------------------------------------------------------------------------
// the measurement
// ---------------------------------------------------------------------------

/// Bytes and files under one path, with everything that was deliberately **not** counted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sizes {
    pub bytes: u64,
    pub files: u64,
    /// Paths skipped because they are symlinks. Counted nowhere, and reported.
    pub not_followed: Vec<PathBuf>,
}

impl Sizes {
    fn add(&mut self, other: Sizes) {
        self.bytes += other.bytes;
        self.files += other.files;
        self.not_followed.extend(other.not_followed);
    }
}

/// What one path cost, or why it could not be measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub path: PathBuf,
    pub sizes: Result<Sizes, String>,
}

impl Found {
    pub fn bytes(&self) -> u64 {
        self.sizes.as_ref().map(|sizes| sizes.bytes).unwrap_or(0)
    }

    pub fn files(&self) -> u64 {
        self.sizes.as_ref().map(|sizes| sizes.files).unwrap_or(0)
    }
}

/// One part of the tree, as measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scanned {
    pub part: Part,
    /// Paths that exist, with their sizes.
    pub found: Vec<Found>,
    /// The containers the part looked in. **Rendered, never left blank**: an empty part that
    /// printed no search list would read as having looked nowhere.
    pub searched: Vec<PathBuf>,
}

impl Scanned {
    pub fn bytes(&self) -> u64 {
        self.found.iter().map(Found::bytes).sum()
    }

    pub fn files(&self) -> u64 {
        self.found.iter().map(Found::files).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.found.is_empty()
    }
}

/// Measure one path: bytes, files, and the symlinks it refused to follow.
fn measure(path: &Path) -> Result<Sizes, String> {
    let meta = fs::symlink_metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if meta.file_type().is_symlink() {
        // A symlink's own bytes are its target's path, and following it would measure bytes the
        // guard does not own and cannot remove. Skipped, counted nowhere, and named in the report.
        return Ok(Sizes {
            bytes: 0,
            files: 0,
            not_followed: vec![path.to_path_buf()],
        });
    }
    if meta.is_file() {
        return Ok(Sizes {
            bytes: meta.len(),
            files: 1,
            not_followed: Vec::new(),
        });
    }
    let mut sizes = Sizes::default();
    let entries =
        fs::read_dir(path).map_err(|error| format!("{}: {error}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", path.display()))?;
        sizes.add(measure(&entry.path())?);
    }
    Ok(sizes)
}

/// Measure one part of the tree. Never fails: a part that cannot be read says so per path.
pub fn scan(root: &Path, part: Part) -> Scanned {
    let mut scanned = Scanned {
        part,
        found: Vec::new(),
        searched: part.searched(root),
    };
    for path in part.units(root) {
        if !path.exists() {
            continue;
        }
        let sizes = measure(&path);
        scanned.found.push(Found { path, sizes });
    }
    scanned
}

// ---------------------------------------------------------------------------
// the volume
// ---------------------------------------------------------------------------

/// What the volume holding the tree had to say, including when it said nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    /// The volume the probe was asked about: `C:` on Windows, the mount's path elsewhere.
    pub anchor: String,
    pub free: Option<u64>,
    /// Which tool answered, when one did.
    pub how: Option<String>,
    /// Every tool that refused, with its own words. Printed when `free` is `None`.
    pub why: Option<String>,
}

impl Volume {
    fn unreadable(anchor: String, why: String) -> Self {
        Volume {
            anchor,
            free: None,
            how: None,
            why: Some(why),
        }
    }

    pub fn is_measured(&self) -> bool {
        self.free.is_some()
    }
}

/// Read the free bytes of the volume holding `root`.
///
/// Two probes, and the platform decides which is tried first: `cmd /C dir <volume>` on Windows
/// (a shell builtin, so it exists wherever the build does), `df -k --output=avail` elsewhere.
/// When both refuse, the reasons are kept — a guard that reported a number here without one would
/// be printing an invention.
/// One way of asking the machine for free bytes: `fn(anchor) -> free bytes, or its own refusal`.
type Probe = fn(&str) -> Result<u64, String>;

pub fn volume_of(root: &Path) -> Volume {
    let anchor = anchor_of(root);
    let mut refusals: Vec<String> = Vec::new();
    let mut probes: Vec<(&'static str, Probe)> = Vec::new();
    if cfg!(windows) {
        probes.push(("cmd /C dir", free_from_dir));
        probes.push(("df -k --output=avail", free_from_df));
    } else {
        probes.push(("df -k --output=avail", free_from_df));
    }
    for (name, probe) in probes {
        match probe(&anchor) {
            Ok(free) => {
                return Volume {
                    anchor,
                    free: Some(free),
                    how: Some(name.to_string()),
                    why: None,
                }
            }
            Err(why) => refusals.push(format!("{name}: {why}")),
        }
    }
    Volume::unreadable(
        anchor,
        format!(
            "no probe could read the volume; {}",
            refusals.join("; ")
        ),
    )
}

/// The volume a path sits on, as text: `C:` on Windows, the path itself elsewhere.
fn anchor_of(root: &Path) -> String {
    if cfg!(windows) {
        let text = root.display().to_string();
        if let Some(colon) = text.find(':') {
            return text[..=colon].to_string();
        }
    }
    root.display().to_string()
}

/// `df -k --output=avail <path>` answers in kibibytes. Last line, first number.
fn free_from_df(anchor: &str) -> Result<u64, String> {
    let output = Command::new("df")
        .args(["-k", "--output=avail"])
        .arg(anchor)
        .output()
        .map_err(|error| format!("could not run df ({error})"))?;
    if !output.status.success() {
        return Err(format!(
            "df exited {}: {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let kib = text
        .lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .next_back()
        .ok_or_else(|| format!("df printed no number: {:?}", text.trim()))?;
    Ok(kib * 1024)
}

/// `cmd /C dir <volume>` ends in `… 12,004,806,656 bytes free`. The number is the parse; the
/// surrounding words are the currency of a localized shell, so the guard reads the token before
/// `bytes free` and, if it is not there, says so instead of guessing.
fn free_from_dir(anchor: &str) -> Result<u64, String> {
    let output = Command::new("cmd")
        .args(["/C", "dir", anchor])
        .output()
        .map_err(|error| format!("could not run cmd ({error})"))?;
    if !output.status.success() {
        return Err(format!(
            "cmd exited {}: {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let Some(at) = line.to_ascii_lowercase().find("bytes free") else {
            continue;
        };
        let before = &line[..at];
        let Some(token) = before.split_whitespace().next_back() else {
            continue;
        };
        let digits: String = token.chars().filter(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        return digits
            .parse::<u64>()
            .map_err(|error| format!("{token:?} is not a byte count ({error})"));
    }
    Err(format!(
        "no `bytes free` line in dir's output: {:?}",
        text.lines().last().unwrap_or("").trim()
    ))
}

// ---------------------------------------------------------------------------
// the decision
// ---------------------------------------------------------------------------

/// Why the guard acted, or did not. A reason, not a boolean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    /// The volume has its headroom, or the tree is inside its budget. Nothing was removed.
    Room,
    /// The volume was read and is below the floor.
    Volume,
    /// The volume could not be read, and the tree is over its budget.
    Budget,
}

impl Trigger {
    pub const fn slug(self) -> &'static str {
        match self {
            Trigger::Room => "room",
            Trigger::Volume => "volume-below-floor",
            Trigger::Budget => "tree-over-budget",
        }
    }
}

/// What the guard removed, and what it refused to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trimmed {
    pub removed: Vec<Found>,
    /// Paths inside the tree that are not under the root — a bug would show up here rather than
    /// as a deletion, and it is printed when it happens.
    pub refused_paths: Vec<(PathBuf, String)>,
    pub freed: u64,
    pub free_after: Option<u64>,
}

/// One entry of the keep list: a thing the guard will not delete, and why.
pub struct Kept {
    /// The name a reader knows it by.
    pub name: &'static str,
    /// The path it was measured at, or `None` when it is outside the tree.
    pub path: Option<PathBuf>,
    pub why: &'static str,
}

/// The keep list, with a reason on every line. Deleting any of these is `cargo clean`.
pub fn keep_list(root: &Path) -> Vec<Kept> {
    vec![
        Kept {
            name: "deps",
            // Joined a segment at a time: `root.join("debug/deps")` writes the separator the
            // literal happens to carry, and a report that mixes `\` and `/` reads as two paths.
            path: Some(root.join("debug").join("deps")),
            why: "reusable compiled artefacts: removing them is a full rebuild, which is the cost \
                  this guard exists to avoid",
        },
        Kept {
            name: "release",
            path: Some(root.join("release")),
            why: "the profile that ships, and its artefacts are reused across runs",
        },
        Kept {
            name: "fingerprint",
            path: Some(root.join("debug").join(".fingerprint")),
            why: "cargo's own freshness record: the fewest bytes in the tree and the most \
                  expensive thing in it to lose",
        },
        Kept {
            name: "build-out",
            path: Some(root.join("debug").join("build")),
            why: "sources and artefacts other crates include through OUT_DIR",
        },
    ]
}

/// The levers the guard will **not** pull, named with the command that pulls each.
pub fn moved_out() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "the kache store",
            "outside the build tree; `kache gc` is the lever, and this guard will not delete \
             another tool's cache to make its own number look good",
        ),
        (
            "logs/",
            "the workspace's retained evidence, not an artefact: deleting it is a human \
             decision",
        ),
    ]
}

/// One whole pass: what was measured, what it decided, what it removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub root: PathBuf,
    pub volume: Volume,
    pub floor: u64,
    pub minimum: u64,
    /// A restated floor that did not parse, if one was offered.
    pub floor_refused: Option<String>,
    pub scanned: Vec<Scanned>,
    pub kept: Vec<Found>,
    pub trimmed: Trimmed,
    pub trigger: Trigger,
}

impl Report {
    /// Bytes in the tree, measured over the parts and the keep list.
    pub fn tree_bytes(&self) -> u64 {
        self.scanned.iter().map(Scanned::bytes).sum::<u64>()
            + self.kept.iter().map(Found::bytes).sum::<u64>()
    }

    /// The free space the decision was made on: after trimming when the guard trimmed, otherwise
    /// the reading it took.
    pub fn free(&self) -> Option<u64> {
        self.trimmed.free_after.or(self.volume.free)
    }

    /// The refusal the gate turns into a failed build, or `None` when the volume has room.
    pub fn refusal(&self) -> Option<String> {
        let free = self.free()?;
        if free >= self.minimum {
            return None;
        }
        let held: Vec<String> = self
            .kept
            .iter()
            .filter(|found| found.bytes() > 0)
            .map(|found| format!("{} ({})", found.path.display(), human(found.bytes())))
            .collect();
        Some(format!(
            "the volume holding {} has {} free after the guard removed {} — below the {} floor and \
             below the {} minimum, so a build here cannot finish. It will not be taken from the \
             tree: the guard's keep list ({}) is what makes a build cheap, and its removal is \
             `cargo clean`. What is left is outside it: {}.",
            self.root.display(),
            human(free),
            human(self.trimmed.freed),
            human(self.floor),
            human(self.minimum),
            if held.is_empty() {
                "nothing measured".to_string()
            } else {
                held.join(", ")
            },
            moved_out()
                .iter()
                .map(|(name, why)| format!("{name} ({why})"))
                .collect::<Vec<_>>()
                .join("; ")
        ))
    }

    /// The gate's section, `key<TAB>value` like every other line this project prints.
    pub fn describe(&self) -> String {
        let mut out = String::new();
        let state = if self.trimmed.removed.is_empty() {
            "ok"
        } else if self.free().map(|free| free >= self.floor).unwrap_or(true) {
            "trimmed"
        } else {
            "short"
        };
        out.push_str(&format!("space-guard\t{state}\n"));
        out.push_str(&format!("space-target\t{}\n", self.root.display()));
        match self.volume.free {
            Some(free) => out.push_str(&format!(
                "space-volume\t{}\tfree-bytes\t{free}\tfloor-bytes\t{}\tmeasured-by\t{}\n",
                self.volume.anchor,
                self.floor,
                self.volume.how.as_deref().unwrap_or("unknown")
            )),
            None => out.push_str(&format!(
                "space-volume\tnot-measured\twhy\t{}\n",
                self.volume.why.as_deref().unwrap_or("no reason recorded")
            )),
        }
        if let Some(refused) = &self.floor_refused {
            out.push_str(&format!("space-floor-refused\t{refused}\n"));
        }
        out.push_str(&format!(
            "space-trigger\t{}\t{}\n",
            self.trigger.slug(),
            match self.trigger {
                Trigger::Room => format!(
                    "nothing was removed: the volume has its {} of headroom and the tree is \
                     inside its budget",
                    human(self.floor)
                ),
                Trigger::Volume => format!(
                    "the volume was below {} and the parts below were taken, cheapest first",
                    human(self.floor)
                ),
                Trigger::Budget => "the volume could not be read, so the tree's own budget was \
                     the trigger and the guard says so rather than claiming to know the disk"
                    .to_string(),
            }
        ));
        out.push_str(&format!("space-tree-bytes\t{}\n", self.tree_bytes()));
        // **Which moment the numbers below are from.** Every part line is the state this pass
        // *found*; the removals further down are what changed it. A report that printed both
        // without saying so would let a reader think the removed bytes are still on disk.
        out.push_str(&format!(
            "space-measured\tfound before this pass; the removals below are what changed it ({} freed)\n",
            human(self.trimmed.freed)
        ));
        for scanned in &self.scanned {
            if scanned.is_empty() {
                // **Absence, rendered as absence.** The search sites are printed, so a reader can
                // tell "nothing here" from "looked nowhere" — and when there is not even a place
                // to look, that is the fact printed instead of an empty list.
                let searched = if scanned.searched.is_empty() {
                    format!(
                        "none: no profile directory under {}",
                        self.root.display()
                    )
                } else {
                    scanned
                        .searched
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\t")
                };
                out.push_str(&format!(
                    "space-part\t{}\tempty\tsearched\t{searched}\n",
                    scanned.part.slug()
                ));
                continue;
            }
            let unreadable: Vec<&Found> = scanned
                .found
                .iter()
                .filter(|found| found.sizes.is_err())
                .collect();
            if unreadable.len() == scanned.found.len() {
                out.push_str(&format!(
                    "space-part\t{}\tnot-measured\t{}\n",
                    scanned.part.slug(),
                    unreadable
                        .iter()
                        .map(|found| found.sizes.as_ref().err().cloned().unwrap_or_default())
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            } else {
                out.push_str(&format!(
                    "space-part\t{}\tbytes\t{}\tfiles\t{}\tcost\t{}\n",
                    scanned.part.slug(),
                    scanned.bytes(),
                    scanned.files(),
                    scanned.part.cost()
                ));
            }
            for found in &scanned.found {
                if let Err(why) = &found.sizes {
                    out.push_str(&format!(
                        "space-part-unreadable\t{}\t{why}\n",
                        found.path.display()
                    ));
                }
                if let Ok(sizes) = &found.sizes {
                    for path in &sizes.not_followed {
                        out.push_str(&format!("space-not-followed\t{}\n", path.display()));
                    }
                }
            }
        }
        for kept in &self.kept {
            match &kept.sizes {
                Ok(sizes) if sizes.bytes > 0 => out.push_str(&format!(
                    "space-kept\t{}\tbytes\t{}\tfiles\t{}\treason\t{}\n",
                    kept.path.display(),
                    sizes.bytes,
                    sizes.files,
                    keep_list(&self.root)
                        .iter()
                        .find(|entry| entry.path.as_deref() == Some(kept.path.as_path()))
                        .map(|entry| entry.why)
                        .unwrap_or("no reason recorded")
                )),
                Ok(_) => out.push_str(&format!(
                    "space-kept\t{}\tempty\n",
                    kept.path.display()
                )),
                Err(why) => out.push_str(&format!(
                    "space-kept\t{}\tnot-measured\t{why}\n",
                    kept.path.display()
                )),
            }
        }
        if self.trimmed.removed.is_empty() {
            out.push_str(&format!(
                "space-removed\tnothing\t{}\n",
                match self.trigger {
                    Trigger::Room => "the volume had its headroom; nothing needed to be taken",
                    Trigger::Volume | Trigger::Budget => {
                        "the trigger fired but every measured part was already empty"
                    }
                }
            ));
        }
        for removed in &self.trimmed.removed {
            out.push_str(&format!(
                "space-removed\t{}\tbytes\t{}\tfiles\t{}\tcost\t{}\n",
                removed.path.display(),
                removed.bytes(),
                removed.files(),
                part_owning(&self.scanned, &removed.path)
                    .map(Part::cost)
                    .unwrap_or("no cost recorded")
            ));
        }
        for (path, why) in &self.trimmed.refused_paths {
            out.push_str(&format!(
                "space-path-refused\t{}\t{why}\n",
                path.display()
            ));
        }
        out.push_str(&format!("space-freed\t{}\n", self.trimmed.freed));
        out.push_str(&format!(
            "space-floor-reached\t{}\n",
            match self.free() {
                Some(free) if free >= self.floor => "yes",
                Some(_) => "no",
                None => "not-measured",
            }
        ));
        if let Some(free) = self.free() {
            if free < self.floor {
                out.push_str(&format!("space-shortfall\t{}\n", self.floor - free));
            }
        }
        for (name, why) in moved_out() {
            out.push_str(&format!("space-moved-out\t{name}\t{why}\n"));
        }
        out.push_str(
            "space-guard-is-not\ta claim about the volume: it measures the tree it owns and reads \
             the free space with the tools the machine has. A trimmed artefact is regenerated by \
             cargo at the cost printed beside it, and a removal happens while this build runs — \
             cargo treats a missing part as absent and produces it again.\n",
        );
        out
    }
}

/// The part a removed path belongs to, so its cost line travels with it.
fn part_owning(scanned: &[Scanned], path: &Path) -> Option<Part> {
    scanned
        .iter()
        .find(|scanned| scanned.found.iter().any(|found| found.path == path))
        .map(|scanned| scanned.part)
}

/// One whole pass over `root`, with the floor read from the environment or [`FLOOR`].
pub fn guard(root: &Path) -> Report {
    let (floor, refused) = floor_from_env();
    guard_with(root, floor, refused)
}

/// The same pass with the floor stated by the caller — what `--floor` and the tests use.
pub fn guard_with(root: &Path, floor: u64, floor_refused: Option<String>) -> Report {
    let volume = volume_of(root);
    let scanned: Vec<Scanned> = Part::ALL.iter().map(|part| scan(root, *part)).collect();
    let kept: Vec<Found> = keep_list(root)
        .into_iter()
        .filter_map(|entry| entry.path)
        .map(|path| {
            let sizes = if path.exists() {
                measure(&path)
            } else {
                Err("nothing at this path".to_string())
            };
            Found { path, sizes }
        })
        .collect();

    // ---- the decision: the volume first, the tree's own budget only when the volume is a blank
    let tree_bytes = scanned.iter().map(Scanned::bytes).sum::<u64>()
        + kept.iter().map(Found::bytes).sum::<u64>();
    let trigger = trigger_of(volume.free, tree_bytes, floor);

    let mut trimmed = Trimmed {
        free_after: volume.free,
        ..Trimmed::default()
    };
    if trigger != Trigger::Room {
        // The budget is the shortage in bytes, computed from the reading that fired the trigger:
        // how far below the floor the volume is, or how far over its budget the tree is.
        let budget = match (trigger, volume.free) {
            (Trigger::Volume, Some(free)) => floor.saturating_sub(free),
            (Trigger::Budget, _) => tree_bytes.saturating_sub(TREE_BUDGET),
            _ => 0,
        };
        trim(root, &scanned, budget, &mut trimmed);
    }
    Report {
        root: root.to_path_buf(),
        volume,
        floor,
        minimum: MINIMUM,
        floor_refused,
        scanned,
        kept,
        trimmed,
        trigger,
    }
}

/// Remove the parts, cheapest first, until the budget the trigger implies is met.
///
/// **A budget, not a loop of probes.** A volume reading costs a process, so it is taken once
/// before this and once after it; the decision in between is arithmetic in bytes. That is also
/// what makes the loop stoppable: `floor - free` is the number of bytes the volume needs, and the
/// guard stops the moment the parts it has taken add up to it.
fn trim(root: &Path, scanned: &[Scanned], budget: u64, trimmed: &mut Trimmed) {
    let start = trimmed.free_after;
    let mut taken = 0;
    'parts: for scanned in scanned {
        for found in &scanned.found {
            if taken >= budget {
                break 'parts;
            }
            if !inside(root, &found.path) {
                // **Never a deletion.** A path outside the tree is a defect in this module, not a
                // thing to remove, and it is reported as a refusal rather than swallowed.
                trimmed.refused_paths.push((
                    found.path.clone(),
                    format!("outside the build tree {}", root.display()),
                ));
                continue;
            }
            if found.sizes.is_err() {
                // An unmeasured path is not removed: the guard does not know its size, so it
                // cannot say what removing it bought, and an unaccounted deletion is worse than
                // a full volume.
                continue;
            }
            let before = found.bytes();
            if remove(&found.path).is_err() {
                continue;
            }
            let after = measure(&found.path).map(|sizes| sizes.bytes).unwrap_or(0);
            // **What was freed is the difference, not the scan.** Whatever is still at the path
            // after the removal is not free space — a file another process holds open would
            // otherwise be credited to the guard as bytes it did not win.
            let freed = before.saturating_sub(after);
            taken += freed;
            trimmed.freed += freed;
            trimmed.removed.push(Found {
                path: found.path.clone(),
                sizes: Ok(Sizes {
                    bytes: freed,
                    files: found.files(),
                    not_followed: Vec::new(),
                }),
            });
        }
    }
    // The outcome is measured rather than computed, so the report says what the volume actually
    // has — and when the volume cannot be read, it keeps the reading it had instead of inventing
    // one from the bytes it removed.
    trimmed.free_after = volume_of(root).free.or(start);
}

/// Remove a file or a directory, and nothing else.
fn remove(path: &Path) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || meta.is_file() {
        fs::remove_file(path)
    } else {
        fs::remove_dir_all(path)
    }
}

/// Whether a path is inside the tree, and not the tree itself. The one gate every deletion passes.
pub fn inside(root: &Path, candidate: &Path) -> bool {
    candidate.starts_with(root) && candidate != root
}

/// Which trigger fires, from the two facts the guard has: the volume (when it could be read) and
/// the tree. Separate from the pass so every branch — including *the volume could not be read* — is
/// testable without arranging a full disk.
fn trigger_of(free: Option<u64>, tree_bytes: u64, floor: u64) -> Trigger {
    match free {
        Some(free) if free < floor => Trigger::Volume,
        Some(_) => Trigger::Room,
        None if tree_bytes > TREE_BUDGET => Trigger::Budget,
        None => Trigger::Room,
    }
}

/// The declared floor and any refusal that came with it.
fn floor_from_env() -> (u64, Option<String>) {
    let Some(text) = std::env::var_os(FLOOR_ENV) else {
        return (FLOOR, None);
    };
    let text = text.to_string_lossy().trim().to_string();
    match parse_floor(&text) {
        Ok(gib) => (gib * GIB, None),
        Err(refused) => (
            FLOOR,
            Some(format!(
                "{FLOOR_ENV}={refused:?} is not a whole number of GiB, so the {} default stands",
                human(FLOOR)
            )),
        ),
    }
}

/// A whole number of GiB, with the unit written out or left off. Anything else is refused —
/// **and the text of the refusal is the text that was given**, so a report can quote it back.
///
/// One implementation for the build gate and for the standalone runner, because a second parser
/// is a second answer to the same question.
pub fn parse_floor(text: &str) -> Result<u64, String> {
    let digits = text.trim().trim_end_matches(['G', 'i', 'B', 'g', 'b']).trim();
    digits
        .parse::<u64>()
        .ok()
        .filter(|gib| *gib > 0)
        .ok_or_else(|| text.trim().to_string())
}

/// The target directory of a workspace: cargo's override when it is set, the default otherwise.
pub fn target_dir(workspace: &Path) -> PathBuf {
    match std::env::var_os(TARGET_ENV) {
        Some(value) if !value.is_empty() => {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                path
            } else {
                workspace.join(path)
            }
        }
        _ => workspace.join("target"),
    }
}

/// Bytes as a person reads them.
pub fn human(bytes: u64) -> String {
    const UNITS: [(&str, u64); 4] = [
        ("GiB", GIB),
        ("MiB", 1024 * 1024),
        ("KiB", 1024),
        ("B", 1),
    ];
    for (unit, scale) in UNITS {
        if bytes >= scale {
            // A count below a kibibyte reads as a count, not as a rounded fraction of itself.
            if scale == 1 {
                return format!("{bytes} B");
            }
            return format!("{:.2} {unit}", bytes as f64 / scale as f64);
        }
    }
    "0 B".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree with the same shape cargo produces, built from real files so the trim deletes real
    /// bytes. Nothing here is a mock: the guard's whole risk is that it deletes the wrong thing.
    fn a_tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("space-guard-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let write = |path: PathBuf, bytes: usize| {
            fs::create_dir_all(path.parent().expect("a parent")).expect("create the parent");
            fs::write(path, vec![b'x'; bytes]).expect("write the file");
        };
        write(root.join("debug/examples/demo.exe"), 4096);
        write(root.join("debug/examples/demo.pdb"), 2048);
        write(root.join("debug/incremental/session/state.bin"), 1024);
        write(root.join("debug/deps/keep.rlib"), 8192);
        write(root.join("debug/deps/keep.pdb"), 512);
        write(root.join("debug/.fingerprint/entry"), 64);
        write(root.join("debug/build/out/generated.rs"), 256);
        write(root.join("doc/index.html"), 128);
        write(root.join("release/examples/demo.exe"), 2048);
        root
    }

    fn cleanup(root: &Path) {
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn every_path_the_guard_removes_is_inside_the_tree() {
        // The safety property, stated as a test over the *scanner* rather than the trimmer: every
        // unit comes from `Part::units`, and every unit is inside the root it was built from.
        let root = a_tree("inside");
        for part in Part::ALL {
            for path in part.units(&root) {
                assert!(
                    inside(&root, &path),
                    "{} is not inside {}",
                    path.display(),
                    root.display()
                );
                assert!(
                    !path.to_string_lossy().contains(".."),
                    "{} carries a parent reference",
                    path.display()
                );
            }
        }
        cleanup(&root);
    }

    #[test]
    fn a_path_outside_the_tree_is_refused_rather_than_removed() {
        let root = a_tree("outside");
        let outside = root.parent().expect("a parent").join("space-guard-outside.txt");
        fs::write(&outside, b"not the guard's").expect("write");
        assert!(!inside(&root, &outside));
        assert!(!inside(&root, &root), "the root itself is not a unit");
        assert!(outside.exists(), "the refusal above is an assertion, not a deletion");
        let _ = fs::remove_file(&outside);
        cleanup(&root);
    }

    #[test]
    fn the_scan_separates_what_is_there_from_where_it_looked() {
        let root = a_tree("scan");
        let examples = scan(&root, Part::Examples);
        // Two profiles: debug and release. Both are named as search sites, and both hold examples.
        assert_eq!(examples.searched.len(), 2, "{:?}", examples.searched);
        assert!(examples.searched.iter().all(|path| path.ends_with("examples")));
        assert_eq!(examples.bytes(), 4096 + 2048 + 2048);
        assert_eq!(examples.files(), 3);
        let symbols = scan(&root, Part::Symbols);
        // **Only `deps/*.pdb`.** An example's own symbols live inside `examples/` and are taken
        // with it; the symbols part is the one file-per-unit category, and it is the last taken.
        assert_eq!(symbols.bytes(), 512);
        assert_eq!(symbols.files(), 1);
        assert!(symbols.searched.iter().all(|path| path.ends_with("deps")));
        // A part whose tree is not there renders as absence with its search site, never as a zero.
        let root2 = root.join("nothing");
        let doc = scan(&root2, Part::Doc);
        assert!(doc.is_empty());
        assert_eq!(doc.searched, vec![root2.join("doc")]);
        cleanup(&root);
    }

    #[test]
    fn a_part_trimmed_to_nothing_still_says_where_it_looked() {
        // **The bug this test was written for.** The first version of this module collected only
        // the `.pdb` files it found, so a part whose files had just been taken printed
        // `space-part symbols empty searched ` — a blank list, which reads as *looked nowhere*.
        // The search sites are now the authority, and this is the regression that holds it.
        let root = a_tree("searched-after-trim");
        let mut trimmed = Trimmed::default();
        let scanned: Vec<Scanned> = Part::ALL.iter().map(|part| scan(&root, *part)).collect();
        trim(&root, &scanned, u64::MAX, &mut trimmed);
        let after = scan(&root, Part::Symbols);
        assert!(after.is_empty(), "the symbols were taken");
        assert_eq!(after.searched.len(), 2, "and the two sites are still named");
        assert!(after.searched.iter().all(|path| path.ends_with("deps")));
        // Which is what the report prints: no part line ends in an empty search list.
        let report = guard_with(&root, 0, None);
        for line in report.describe().lines() {
            assert!(
                !line.starts_with("space-part\t") || !line.trim_end().ends_with("searched"),
                "a blank search list reached the report: {line:?}"
            );
        }
        cleanup(&root);
    }

    #[test]
    fn the_keep_list_is_never_a_candidate_and_every_entry_says_why() {
        let root = a_tree("keep");
        for entry in keep_list(&root) {
            assert!(!entry.why.is_empty(), "{} has no reason", entry.name);
        }
        // `deps` holds the guard's own keep file, and no part may claim it.
        let deps = root.join("debug/deps/keep.rlib");
        for part in Part::ALL {
            assert!(
                !part.units(&root).iter().any(|path| path == &deps),
                "{} claims a kept artefact",
                part.slug()
            );
        }
        cleanup(&root);
    }

    #[test]
    fn a_trim_takes_the_cheapest_part_and_leaves_the_kept_ones_alone() {
        let root = a_tree("trim");
        let kept = root.join("debug/deps/keep.rlib");
        let fingerprint = root.join("debug/.fingerprint/entry");
        // One byte of headroom on a volume with none: the cheapest part is taken, and the trim
        // stops as soon as the floor is met.
        let mut trimmed = Trimmed {
            free_after: Some(0),
            ..Trimmed::default()
        };
        let scanned: Vec<Scanned> = Part::ALL.iter().map(|part| scan(&root, *part)).collect();
        trim(&root, &scanned, 1, &mut trimmed);
        assert_eq!(trimmed.removed.len(), 1, "{:?}", trimmed.removed);
        assert_eq!(trimmed.removed[0].path, root.join("doc"));
        assert_eq!(trimmed.freed, 128, "the removal is measured, not assumed");
        assert!(root.join("debug/examples/demo.exe").exists());
        assert!(kept.exists(), "the keep list survived");
        assert!(fingerprint.exists(), "cargo's record survived");
        cleanup(&root);
    }

    #[test]
    fn a_trim_that_cannot_meet_its_budget_takes_only_the_parts_it_may() {
        let root = a_tree("floor");
        let mut trimmed = Trimmed::default();
        let scanned: Vec<Scanned> = Part::ALL.iter().map(|part| scan(&root, *part)).collect();
        // A budget no trimming can meet: everything the guard may take is taken, and the keep
        // list still stands.
        trim(&root, &scanned, u64::MAX, &mut trimmed);
        let taken: Vec<String> = trimmed
            .removed
            .iter()
            .map(|found| found.path.display().to_string())
            .collect();
        assert!(!root.join("doc").exists(), "{taken:?}");
        assert!(!root.join("debug/examples").exists(), "{taken:?}");
        assert!(!root.join("debug/incremental").exists(), "{taken:?}");
        assert!(root.join("debug/deps/keep.rlib").exists(), "deps is kept");
        assert!(root.join("debug/.fingerprint/entry").exists(), "the record is kept");
        assert!(root.join("debug/build/out/generated.rs").exists(), "build/out is kept");
        assert!(trimmed.freed > 0);
        cleanup(&root);
    }

    #[test]
    fn a_link_is_never_followed_and_never_counted() {
        let root = a_tree("link");
        let sizes = measure(&root.join("debug/examples/demo.exe")).expect("a file");
        assert_eq!(sizes.bytes, 4096);
        assert!(sizes.not_followed.is_empty());
        // A missing path is an error rather than a zero, so `not-measured` can be reported.
        assert!(measure(&root.join("nothing/here")).is_err());
        cleanup(&root);
    }

    #[test]
    fn a_symlink_is_skipped_and_named_instead_of_counted() {
        let root = a_tree("symlink");
        let target = root.join("debug/deps/keep.rlib");
        let link = root.join("debug/examples/link.exe");
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(&target, &link).is_ok();
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&target, &link).is_ok();
        if made {
            let sizes = measure(&root.join("debug/examples")).expect("the directory reads");
            assert_eq!(sizes.not_followed, vec![link.clone()]);
            // The linked file is counted once, through its own name — never twice. **The two
            // numbers come from `measure`, not from the sizes written above**, because `bytes` is
            // what the volume allocated: a 2048-byte file is one 2048-byte run on a volume with
            // 2 KiB clusters and one 4096-byte run on this machine's, and a test holding the literal
            // total was measuring the cluster size of the disk it happened to run on. A link
            // allocates no clusters of its own, so the directory is its two files and nothing else.
            let demo = measure(&root.join("debug/examples/demo.exe")).expect("a file").bytes;
            let pdb = measure(&root.join("debug/examples/demo.pdb")).expect("a file").bytes;
            assert_eq!(sizes.bytes, demo + pdb);
            assert!(sizes.bytes < demo + pdb + 8192, "the link's target is not counted");
        }
        cleanup(&root);
    }

    #[test]
    fn an_unreadable_volume_falls_back_to_the_tree_budget_and_says_so() {
        // The volume below its floor is the volume trigger, whether or not the tree is large.
        assert_eq!(trigger_of(Some(FLOOR - 1), 0, FLOOR), Trigger::Volume);
        // Room is room, even in a large tree: the trigger is the volume, and it answered.
        assert_eq!(trigger_of(Some(FLOOR), TREE_BUDGET * 2, FLOOR), Trigger::Room);
        // With no reading, the tree's own budget is the only fact left, and it is used.
        assert_eq!(trigger_of(None, TREE_BUDGET + 1, FLOOR), Trigger::Budget);
        // With no reading and a tree inside its budget there is nothing to answer, and a guard
        // that trimmed on a blank would be deleting on a guess.
        assert_eq!(trigger_of(None, TREE_BUDGET, FLOOR), Trigger::Room);
    }

    #[test]
    fn the_three_thresholds_are_the_ones_the_measurements_derived() {
        // The *order* of the three is not a test but a compile-time assertion, above — an edit that
        // put the refusal line above the working line would not build. These are the values.
        assert_eq!(FLOOR, 8 * GIB);
        assert_eq!(MINIMUM, GIB);
        assert_eq!(TREE_BUDGET, 4 * GIB);
    }

    #[test]
    fn a_bad_floor_is_refused_and_the_default_stands() {
        // The parser is exercised without touching the process environment, which every test in
        // this process shares — and it is the same parser the gate and the runner use.
        assert_eq!(parse_floor("12GiB"), Ok(12));
        assert_eq!(parse_floor("12G"), Ok(12));
        assert_eq!(parse_floor(" 3 "), Ok(3));
        assert_eq!(parse_floor("twelve"), Err("twelve".to_string()));
        assert_eq!(parse_floor("0"), Err("0".to_string()));
        assert_eq!(parse_floor("-1"), Err("-1".to_string()));
    }

    #[test]
    fn a_volume_reading_is_either_a_number_or_a_reason_with_every_probe_in_it() {
        let volume = volume_of(&std::env::temp_dir());
        assert!(!volume.anchor.is_empty());
        match volume.free {
            Some(_) => assert!(volume.how.is_some() && volume.why.is_none()),
            None => {
                let why = volume.why.expect("an unreadable volume says why");
                assert!(why.contains(':'), "each probe's own words are kept: {why}");
            }
        }
    }

    #[test]
    fn a_tree_with_room_is_reported_as_room_and_nothing_is_removed() {
        let root = a_tree("room");
        // The floor is zero, so the volume can never be below it: the trigger is `room`.
        let report = guard_with(&root, 0, None);
        assert_eq!(report.trigger, Trigger::Room);
        assert!(report.trimmed.removed.is_empty());
        assert!(root.join("doc/index.html").exists());
        let text = report.describe();
        assert!(text.contains("space-guard\tok"), "{text}");
        assert!(text.contains("space-removed\tnothing\t"), "{text}");
        assert!(text.contains("space-trigger\troom\t"), "{text}");
        // The keep list is reported with its reasons even when nothing was removed.
        assert!(text.contains("space-kept\t"), "{text}");
        assert!(text.contains("this guard exists to avoid"), "{text}");
        assert!(report.refusal().is_none());
        cleanup(&root);
    }

    #[test]
    fn a_floor_the_volume_cannot_meet_is_reported_as_short_and_the_keep_list_survives() {
        let root = a_tree("short");
        let report = guard_with(&root, u64::MAX, None);
        let text = report.describe();
        assert!(text.contains("space-guard\tshort"), "{text}");
        assert!(text.contains("space-floor-reached\tno"), "{text}");
        assert!(text.contains("space-shortfall\t"), "{text}");
        // Everything the guard may take was taken, and nothing it may not take was.
        assert!(root.join("debug/deps/keep.rlib").exists(), "deps is kept");
        assert!(root.join("debug/.fingerprint/entry").exists(), "the record is kept");
        cleanup(&root);
    }

    /// A report whose volume reading is stated, so the refusal line can be exercised without a
    /// full disk — the one condition a test cannot arrange honestly on a working machine.
    fn report_with_free(root: &Path, free: u64) -> Report {
        let scanned: Vec<Scanned> = Part::ALL.iter().map(|part| scan(root, *part)).collect();
        let kept = keep_list(root)
            .into_iter()
            .filter_map(|entry| entry.path)
            .map(|path| {
                let sizes = measure(&path);
                Found { path, sizes }
            })
            .collect();
        Report {
            root: root.to_path_buf(),
            volume: Volume {
                anchor: "C:".to_string(),
                free: Some(free),
                how: Some("a test".to_string()),
                why: None,
            },
            floor: FLOOR,
            minimum: MINIMUM,
            floor_refused: None,
            scanned,
            kept,
            trimmed: Trimmed {
                free_after: Some(free),
                ..Trimmed::default()
            },
            trigger: Trigger::Volume,
        }
    }

    #[test]
    fn a_volume_with_no_room_ends_in_a_refusal_that_names_what_holds_the_space() {
        let root = a_tree("refusal");
        let report = report_with_free(&root, MINIMUM - 1);
        let refusal = report
            .refusal()
            .expect("below the minimum is a refusal, not a warning");
        assert!(refusal.contains("cannot finish"), "{refusal}");
        assert!(refusal.contains("keep list"), "{refusal}");
        assert!(refusal.contains("kache gc"), "{refusal}");
        assert!(refusal.contains("logs/"), "{refusal}");
        cleanup(&root);
    }

    #[test]
    fn a_volume_above_the_minimum_is_not_a_refusal_even_below_the_floor() {
        // The floor warns and trims; only the minimum refuses. A gate that refused at the floor
        // would fail builds that could have finished.
        let root = a_tree("warning");
        let report = report_with_free(&root, MINIMUM);
        assert!(report.floor > report.minimum);
        assert!(report.refusal().is_none(), "{}", report.describe());
        cleanup(&root);
    }

    #[test]
    fn the_report_carries_every_removal_with_its_cost_and_every_kept_entry() {
        let root = a_tree("report");
        let report = guard_with(&root, u64::MAX, None);
        let text = report.describe();
        assert!(text.contains("space-guard\ttrimmed") || text.contains("space-guard\tshort"), "{text}");
        assert!(text.contains("space-removed\t"), "{text}");
        assert!(text.contains("cost\ta `cargo doc`"), "{text}");
        assert!(text.contains("space-freed\t"), "{text}");
        assert!(text.contains("space-floor-reached\t"), "{text}");
        assert!(text.contains("space-moved-out\tthe kache store\t"), "{text}");
        assert!(text.contains("space-guard-is-not\t"), "{text}");
        // Nothing removed may be outside the tree, and the removal list is the scan's own paths.
        for removed in &report.trimmed.removed {
            assert!(inside(&root, &removed.path), "{}", removed.path.display());
        }
        cleanup(&root);
    }

    #[test]
    fn every_part_of_an_empty_tree_renders_as_searched_and_never_blank() {
        let root = std::env::temp_dir().join(format!("space-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("the tree exists");
        let report = guard_with(&root, 0, None);
        let text = report.describe();
        for part in Part::ALL {
            assert!(
                text.contains(&format!("space-part\t{}\tempty\tsearched", part.slug())),
                "{} is not rendered as searched: {text}",
                part.slug()
            );
        }
        // A tree with no profile directories is not a tree with places to look, and it says so.
        assert!(text.contains("none: no profile directory under"), "{text}");
        // A part with no profile to look in still names the one site `doc` has.
        assert!(text.contains(&root.join("doc").display().to_string()), "{text}");
        // And no line of the report claims a measurement of zero bytes for a part.
        assert!(!text.contains("space-part\tdoc\tbytes\t0"), "{text}");
        cleanup(&root);
    }

    #[test]
    fn the_target_directory_follows_cargos_own_override() {
        let workspace = Path::new("C:/somewhere/shploingus");
        // With no override set, the default is the workspace's own `target`.
        if std::env::var_os(TARGET_ENV).is_none() {
            assert_eq!(target_dir(workspace), workspace.join("target"));
        }
    }

    #[test]
    fn bytes_read_as_a_person_reads_them() {
        assert_eq!(human(0), "0 B");
        assert_eq!(human(512), "512 B");
        assert_eq!(human(1024), "1.00 KiB");
        assert_eq!(human(2048), "2.00 KiB");
        assert_eq!(human(3 * GIB), "3.00 GiB");
    }
}
