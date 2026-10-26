//! **What a directory holds, counted in files, lines and bytes.**
//!
//! The unit is a [`Measure`], and there is one rule: **the same walk counts both languages**, so
//! a Rust line and a WGSL line are the same kind of thing on both sides of the ratio. A file is
//! counted when its extension matches; lines are newline characters plus a final unterminated
//! line; bytes are the file's own length. Nothing is parsed, normalised or weighted.
//!
//! # What is in the walk, and what is not
//!
//! [`Measure::of_dir`] walks recursively and counts one extension. Each crate is measured over
//! its `src/` only — the code that ships, not the tests that judge it and not the examples that
//! demonstrate it. That boundary is stated here because it changes the number: a crate whose
//! tests are longer than its library is not a crate that is mostly CPU.
//!
//! A directory that cannot be read contributes zero, and the crate is expected to notice through
//! [`Measure::files`] — this module has no refusal channel, because "how many files" is a count
//! and not a verdict. The verdict belongs to [`crate::report`].

use std::fs;
use std::path::Path;

/// One directory's worth of one language: files, lines and bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Measure {
    /// Files whose extension matched.
    pub files: u64,
    /// Lines, counted by newline plus a final unterminated line.
    pub lines: u64,
    /// Bytes on disk.
    pub bytes: u64,
}

impl Measure {
    /// The empty measure: a directory with nothing of this language in it.
    pub const ZERO: Measure = Measure { files: 0, lines: 0, bytes: 0 };

    /// Walk `relative` under `root` and count every file with `extension`.
    ///
    /// **Entries are sorted by path** so the count is the same on every filesystem and a report
    /// is comparable between two runs. A missing directory is [`Measure::ZERO`]; a file that
    /// cannot be read is skipped rather than guessed at.
    pub fn of_dir(root: &Path, relative: &str, extension: &str) -> Measure {
        let mut measure = Measure::ZERO;
        walk(&root.join(relative), extension, &mut measure);
        measure
    }

    /// The sum of two measurements, field by field.
    pub fn add(self, other: Measure) -> Measure {
        Measure {
            files: self.files + other.files,
            lines: self.lines + other.lines,
            bytes: self.bytes + other.bytes,
        }
    }

    /// **The ratio a balance is written as**: this measure's lines per one of `other`'s.
    ///
    /// `None` when the denominator is zero, because `∞` is not a balance and `0` would be a
    /// false statement about a tree with no shaders.
    pub fn per_line_of(self, other: Measure) -> Option<f64> {
        if other.lines == 0 {
            return None;
        }
        Some(self.lines as f64 / other.lines as f64)
    }

    /// Lines per file, or `None` for an empty measure.
    pub fn per_file(self) -> Option<f64> {
        if self.files == 0 {
            return None;
        }
        Some(self.lines as f64 / self.files as f64)
    }

    /// Whether anything at all was counted.
    pub fn is_empty(self) -> bool {
        self.files == 0
    }
}

fn walk(directory: &Path, extension: &str, into: &mut Measure) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    let mut paths: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    for path in paths {
        let is_dir = path.is_dir();
        if is_dir {
            walk(&path, extension, into);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some(extension) {
            continue;
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        into.files += 1;
        into.bytes += bytes.len() as u64;
        if bytes.is_empty() {
            continue;
        }
        let newlines = bytes.iter().filter(|byte| **byte == b'\n').count() as u64;
        into.lines += newlines + u64::from(*bytes.last().expect("non-empty") != b'\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "voxel-axis-balance-measure-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/nested")).expect("a scratch tree");
        root
    }

    #[test]
    fn one_walk_counts_files_lines_and_bytes_and_nothing_else() {
        let root = scratch("counts");
        fs::write(root.join("src/a.rs"), "one\ntwo\nthree\n").expect("a file");
        fs::write(root.join("src/nested/b.rs"), "four\nfive").expect("a nested file");
        fs::write(root.join("src/ignored.wgsl"), "not Rust\n").expect("a shader");
        let rust = Measure::of_dir(&root, "src", "rs");
        assert_eq!(rust.files, 2);
        assert_eq!(rust.lines, 5);
        assert_eq!(rust.bytes, 23);
        let wgsl = Measure::of_dir(&root, "src", "wgsl");
        assert_eq!(wgsl.files, 1);
        assert_eq!(wgsl.lines, 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_directory_is_zero_and_a_ratio_with_no_denominator_is_not_a_number() {
        let root = scratch("missing");
        let absent = Measure::of_dir(&root, "nothing-here", "rs");
        assert_eq!(absent, Measure::ZERO);
        assert!(absent.is_empty());
        assert_eq!(Measure::ZERO.per_file(), None);
        let some = Measure { files: 1, lines: 10, bytes: 100 };
        assert_eq!(some.per_line_of(Measure::ZERO), None);
        assert_eq!(some.per_line_of(Measure { files: 1, lines: 4, bytes: 40 }), Some(2.5));
        let _ = fs::remove_dir_all(&root);
    }
}
