//! **Which gate an act's check command proves: a CPU task or a rendering verification.**
//!
//! The class is read from the command as written; a command that does not place itself is `None`,
//! which the caller refuses rather than defaults.

/// **The two kinds of verification a patch act can carry.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Task {
    /// A gate that exercises the code without drawing a frame.
    Cpu,
    /// A gate that draws, captures or reads back a frame.
    Rendering,
}

impl Task {
    /// Both kinds, in the order `Task::classify` tries them.
    pub const ALL: [Task; 2] = [Task::Rendering, Task::Cpu];

    /// The one word a report prints.
    pub const fn name(self) -> &'static str {
        match self {
            Task::Cpu => "cpu",
            Task::Rendering => "rendering",
        }
    }

    /// **The class of a check command, or `None` when the command does not place itself.**
    ///
    /// A rendering word wins over the verb: `cargo run -p shploingus` launches the client, and a
    /// test named for the frame loop still only runs a test — so the words are matched in the order
    /// they can appear in a command line, not in the order of a guess.
    pub fn classify(check: &str) -> Option<Task> {
        let check = check.trim();
        if check.is_empty() {
            return None;
        }
        // A named measurement is rendering whatever verb reaches it.
        const ARTIFACTS: [&str; 5] = [
            "docs/baselines",
            "capture",
            "frame-cost",
            "ladder-view",
            "screenshot",
        ];
        if ARTIFACTS.iter().any(|word| check.contains(word)) {
            return Some(Task::Rendering);
        }
        let mut words = check.split_whitespace();
        let verb = words.next().unwrap_or("");
        if verb == "cargo" || verb.starts_with("cargo-") {
            // `cargo test -p shploingus` names the binary and still only runs a test.
            let subcommand = words.next().unwrap_or("");
            if subcommand == "run" {
                return Some(if check.contains("shploingus") {
                    Task::Rendering
                } else {
                    Task::Cpu
                });
            }
            return Some(Task::Cpu);
        }
        let cpu = verb == "rustc" || verb == "git" || verb.starts_with("./gradlew");
        if cpu {
            return Some(Task::Cpu);
        }
        None
    }

    /// Every class a command list covers, refusing a command that does not place itself.
    ///
    /// An act that lists two gates of different kinds is both: the record does not say one is the
    /// evidence and the other is decoration.
    pub fn classes<'a>(checks: impl IntoIterator<Item = &'a str>) -> Result<Vec<Task>, String> {
        let mut out: Vec<Task> = Vec::new();
        for check in checks {
            let Some(task) = Task::classify(check) else {
                return Err(format!("unclassified check: {check}"));
            };
            if !out.contains(&task) {
                out.push(task);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_launch_is_rendering_even_though_it_starts_with_cargo() {
        assert_eq!(
            Task::classify("cargo run --release -p shploingus -> exit 0"),
            Some(Task::Rendering)
        );
        assert_eq!(
            Task::classify("cargo test -p voxel-client --lib frame_cost -> failures 0"),
            Some(Task::Cpu),
            "a test whose name mentions the frame still only runs a test"
        );
    }

    #[test]
    fn the_corpus_commands_place_themselves() {
        for check in [
            "cargo test -p voxel-axis-matrix --lib -> failures 0",
            "cargo run --release -p voxel-client --example shadow-lattice -> exit 0",
            "git ls-files crates/axis -> 38 lines",
            "cargo test -p shploingus --bin shploingus a_batch_is_deduped -> failures 0",
        ] {
            let task = Task::classify(check).unwrap_or_else(|| panic!("{check}"));
            assert_eq!(task, Task::Cpu, "{check}");
        }
    }

    #[test]
    fn a_command_that_does_not_place_itself_is_none_rather_than_a_default() {
        assert_eq!(Task::classify(""), None);
        assert_eq!(Task::classify("./run.sh"), None);
        assert_eq!(Task::classify("echo done"), None);
        assert_eq!(
            Task::classes(["cargo test -p voxel-model --lib", "docs/baselines/frame-cost.json"]),
            Ok(vec![Task::Cpu, Task::Rendering])
        );
        assert!(Task::classes(["cargo test -p voxel-model --lib", "echo done"]).is_err());
    }
}
