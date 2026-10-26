//! **The segments: seven systems, and the sentence that says what each one is.**
//!
//! A calibration matrix needs a row key, and the key cannot be a crate name: a crate is how the tree
//! is *built*, while a segment is how the game is *verified*. Three crates can be one segment (the
//! world is `voxel-model`, the shapers and the mesher together) and one crate can hold two segments
//! (the client carries both the shell and the pattern system), so keying by crate would make the
//! matrix's *totality* a claim about Cargo rather than about the game.
//!
//! The seven are the user's, named on 2026-10-21: *"allowing for shell, ui, game, world, and all
//! other systems to be consumed and verified"*. `pattern`, `axis` and `tooling` are the three the
//! game already has and the sentence leaves room for — a pattern is drawn, an axis is read, and
//! tooling is what files, checks and serves. **The list is closed on purpose**, like the work verb
//! list: a segment grows by decision, not by a string a caller invents at a call site.
//!
//! # Why the pin is a `const` and not a test
//!
//! [`SEGMENTS_ARE_TOTAL`] is `[(); SEGMENT_COUNT] = [(); ALL.len()]`. A variant added without an
//! entry in [`Segment::ALL`] does not compile, and a count edited without a variant does not either.
//! The value is meaningless — its **type** is the assertion, the same trick the census uses for its
//! crate count and `EventKind` uses for its kinds.

/// One system the game is verified in. **Closed**, and the key of every cell in the matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Segment {
    /// The console: windows, panes, focus, and the mount that replaces a running arrangement.
    Shell,
    /// The declared interface: panels, lanes, and the elements a body is drawn from.
    Ui,
    /// The binary and its frame loop: what a player runs, ticks and sees.
    Game,
    /// The cubic world and everything that shapes it: fields, materials, the mesher.
    World,
    /// The drawn pattern system: the surface a body wears before anything is shaped on it.
    Pattern,
    /// The axis layer itself: the ladders, the readings, this matrix, the civilians.
    Axis,
    /// What files, checks and serves: the package, the MCP surface, and the von model behind them.
    Tooling,
}

/// How many segments this build knows. **Written down rather than derived**, and pinned by
/// [`SEGMENTS_ARE_TOTAL`].
pub const SEGMENT_COUNT: usize = 7;

impl Segment {
    /// Every segment, in the order a report prints them: the shell a player sees first, then the
    /// interface inside it, then the game, the world it stands in, the pattern it wears, the axis
    /// that reads it, and the tooling that files the reading.
    pub const ALL: [Segment; SEGMENT_COUNT] = [
        Segment::Shell,
        Segment::Ui,
        Segment::Game,
        Segment::World,
        Segment::Pattern,
        Segment::Axis,
        Segment::Tooling,
    ];

    /// The recorded name. Lowercase, one word, and the word the recipe line's header carries.
    pub const fn name(self) -> &'static str {
        match self {
            Segment::Shell => "shell",
            Segment::Ui => "ui",
            Segment::Game => "game",
            Segment::World => "world",
            Segment::Pattern => "pattern",
            Segment::Axis => "axis",
            Segment::Tooling => "tooling",
        }
    }

    /// What this segment is, in one sentence a reader can check against the tree.
    ///
    /// **A segment with no sentence is a label with no subject**, so [`crate::Matrix::build`] refuses
    /// one — the same rule the census's `means` field enforces on a crate row.
    pub const fn means(self) -> &'static str {
        match self {
            Segment::Shell => {
                "the console: mounted windows, their panes and lanes, focus and the frame loop \
                 that draws them (crates/voxel-client/src/shell)"
            }
            Segment::Ui => {
                "the declared interface: panels, content kinds, and the elements a body is drawn \
                 from (crates/voxel-ui)"
            }
            Segment::Game => {
                "the binary and what a player does with it: commands, the tick, the apidoc window \
                 and the build gate (crates/shploingus)"
            }
            Segment::World => {
                "the cubic world and what shapes it: field laws, materials, the lattice and the \
                 mesher (crates/voxel-model, voxel-shapers, voxel-mesh)"
            }
            Segment::Pattern => {
                "the drawn pattern system: the surface a body wears before anything is shaped on it \
                 (crates/voxel-client/src/pattern.rs)"
            }
            Segment::Axis => {
                "the axis layer: the ladders, the readings, the civilian systems and this matrix \
                 (crates/axis)"
            }
            Segment::Tooling => {
                "what files, checks and serves: the package format, the MCP surface and the von \
                 model behind them (crates/voxel-package, voxel-mcp, voxel-von)"
            }
        }
    }

    /// Parse a recorded name. **Unknown is refused, never defaulted** — a cell for a segment nobody
    /// declared is a reading about a system that does not exist.
    pub fn parse(word: &str) -> Result<Self, crate::MatrixRefusal> {
        let word = word.trim();
        Self::ALL
            .into_iter()
            .find(|segment| segment.name() == word)
            .ok_or_else(|| crate::MatrixRefusal::UnknownSegment {
                word: word.to_string(),
            })
    }
}

impl std::fmt::Display for Segment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// The table is as wide as the segment set. **Compile-checked, not tested.** `pub` and deliberately:
/// a private constant whose *type* is the check is dead code, and dead code is what a tidy-up
/// deletes.
pub const SEGMENTS_ARE_TOTAL: [(); SEGMENT_COUNT] = [(); Segment::ALL.len()];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_segment_round_trips_through_its_recorded_name() {
        for segment in Segment::ALL {
            let parsed = Segment::parse(segment.name()).expect("a declared name parses");
            assert_eq!(parsed, segment);
            assert_eq!(Segment::parse(&format!("  {} \n", segment.name())).unwrap(), segment);
        }
    }

    #[test]
    fn the_names_are_distinct_and_lowercase() {
        let mut names: Vec<&str> = Segment::ALL.iter().map(|segment| segment.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), SEGMENT_COUNT, "two segments share a name");
        for name in names {
            assert_eq!(name, name.to_lowercase(), "{name} is not a recorded word");
            assert!(!name.contains(' '), "{name} is two words");
        }
    }

    #[test]
    fn an_unknown_segment_is_refused_and_the_word_is_quoted() {
        let refused = Segment::parse("renderer").expect_err("renderer is not a segment");
        assert!(refused.detail().contains("\"renderer\""), "got {refused}");
        assert_eq!(
            refused.kind(),
            voxel_model::refusal::RefusalKind::UndeclaredName
        );
    }

    #[test]
    fn every_segment_says_what_it_is() {
        for segment in Segment::ALL {
            let means = segment.means();
            assert!(
                means.len() > 40,
                "{} has no sentence: {means:?}",
                segment.name()
            );
            assert!(
                means.contains('('),
                "{} names no crate or path to check against",
                segment.name()
            );
        }
    }

    #[test]
    fn the_table_is_as_wide_as_the_count() {
        // The pin is the compile-time check; this is the same fact stated where a reader looks.
        assert_eq!(Segment::ALL.len(), SEGMENT_COUNT);
        assert_eq!(SEGMENTS_ARE_TOTAL.len(), SEGMENT_COUNT);
    }
}
