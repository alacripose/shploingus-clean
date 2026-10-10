//! Milestones and blockers, filed on the chain. Shapes ported from the
//! reference tree's `milestones.rs` / `blockers.rs` (M9); rationale in
//! `docs/LEDGER.md` C-98.

use std::collections::BTreeMap;

use crate::identity::Digest;
use crate::refusal::{Refusal, RefusalKind, RefusalSet};

/// The seven milestone fields, in the plan's own order.
pub const MILESTONE_FIELDS: [&str; 7] = [
    "MISSION",
    "SCOPE",
    "CONSTRAINTS",
    "RESOURCES",
    "EVIDENCE",
    "DELIVERABLE",
    "STOP",
];

/// One milestone: the plan's seven fields, verbatim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Milestone {
    pub id: String,
    pub title: String,
    /// Only the declared seven; absent means the filing never stated it.
    pub fields: BTreeMap<String, String>,
    /// The body below the heading, verbatim — nothing parsed away.
    pub text: String,
}

impl Milestone {
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(String::as_str)
    }

    /// The finish condition. A milestone without one is refused at parse.
    pub fn deliverable(&self) -> Option<&str> {
        self.field("DELIVERABLE")
    }

    pub fn stop(&self) -> Option<&str> {
        self.field("STOP")
    }

    /// Content identity over id and the seven fields in plan order — never map
    /// order, so identity follows the text and not the container.
    pub fn content_identity(&self) -> Digest {
        let mut input = String::from("shploingus/milestone/v1\n");
        input.push_str(&self.id);
        input.push('\n');
        for field in MILESTONE_FIELDS {
            input.push_str(field);
            input.push('=');
            input.push_str(self.fields.get(field).map(String::as_str).unwrap_or(""));
            input.push('\n');
        }
        Digest::of(input.as_bytes())
    }
}

/// One blocker: an unresolved gap with a rule, its consequence, and an owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blocker {
    pub id: String,
    pub title: String,
    /// The `**STOP.**` — required. A blocker with no rule is a note.
    pub rule: String,
    pub stops: Option<String>,
    pub owner: Option<String>,
    pub text: String,
}

impl Blocker {
    pub fn content_identity(&self) -> Digest {
        let mut input = String::from("shploingus/blocker/v1\n");
        input.push_str(&self.id);
        input.push('\n');
        input.push_str(&self.rule);
        input.push('\n');
        input.push_str(self.stops.as_deref().unwrap_or(""));
        input.push('\n');
        Digest::of(input.as_bytes())
    }
}

/// Parse a whole milestone filing: an `ID — title` heading, then `- **FIELD** — value` rows.
pub fn parse_milestone_section(text: &str) -> Result<Milestone, RefusalSet> {
    let (id, title, body) = split_heading(text).ok_or_else(|| malformed(
        "a milestone filing carries no `ID — title` heading on its first line",
    ))?;
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let lines: Vec<&str> = body.lines().collect();
    let mut index = 0usize;
    while index < lines.len() {
        let trimmed = lines[index].trim_start();
        let Some((label, value_at)) = milestone_label(trimmed) else {
            index += 1;
            continue;
        };
        let mut value = String::from(lines[index][value_at..].trim());
        let mut lookahead = index + 1;
        while lookahead < lines.len() {
            let next = lines[lookahead].trim_start();
            if next.is_empty() || milestone_label(next).is_some() {
                break;
            }
            if !value.is_empty() {
                value.push(' ');
            }
            value.push_str(next.trim_start_matches(['-', '–', '—']).trim());
            lookahead += 1;
        }
        if !value.is_empty() {
            fields.entry(label.to_owned()).or_insert(value);
        }
        index = lookahead.max(index + 1);
    }
    if !fields.contains_key("DELIVERABLE") {
        return Err(malformed(format!(
            "the milestone `{id} {title}` states no **DELIVERABLE**, so it has no finish \
             condition and nothing could ever say it is done"
        )));
    }
    Ok(Milestone {
        id: id.to_string(),
        title: title.to_string(),
        fields,
        text: body.trim().to_string(),
    })
}

/// Parse a whole blocker filing: a `B-n — title` heading, then `**Label.** value` rows.
pub fn parse_blocker_section(text: &str) -> Result<Blocker, RefusalSet> {
    let (id, title, body) = split_heading(text).ok_or_else(|| malformed(
        "a blocker filing carries no `ID — title` heading on its first line",
    ))?;
    let mut fields: BTreeMap<BlockerField, String> = BTreeMap::new();
    let lines: Vec<&str> = body.lines().collect();
    let mut index = 0usize;
    while index < lines.len() {
        let trimmed = lines[index].trim_start();
        let Some((label, value_at)) = blocker_label(trimmed) else {
            index += 1;
            continue;
        };
        let mut value = String::from(lines[index][value_at..].trim());
        let mut lookahead = index + 1;
        while lookahead < lines.len() {
            let next = lines[lookahead].trim_start();
            if next.is_empty() || blocker_label(next).is_some() {
                break;
            }
            if !value.is_empty() {
                value.push(' ');
            }
            value.push_str(next);
            lookahead += 1;
        }
        if !value.is_empty() {
            fields.entry(classify_blocker_label(label)).or_insert(value);
        }
        index = lookahead.max(index + 1);
    }
    let Some(rule) = fields.remove(&BlockerField::Rule) else {
        return Err(malformed(format!(
            "the blocker `{id} {title}` carries no `**STOP.**` rule, so there is nothing to obey. \
             A provisional rule is never invented for it"
        )));
    };
    Ok(Blocker {
        id: id.to_string(),
        title: title.to_string(),
        rule,
        stops: fields.remove(&BlockerField::Stops),
        owner: fields.remove(&BlockerField::Owner),
        text: body.trim().to_string(),
    })
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum BlockerField {
    Rule,
    Stops,
    Owner,
    /// A label this module does not model. Kept in `text`, never dropped.
    Other,
}

/// First line `ID — title`; leading `#`, whitespace and `[MST]`/`[BLK]` markers stripped.
fn split_heading(text: &str) -> Option<(&str, &str, &str)> {
    let mut rest = text.trim_start();
    loop {
        let stripped = rest
            .trim_start_matches('#')
            .trim_start()
            .trim_start_matches('[')
            .trim_start();
        if stripped.len() == rest.len() {
            break;
        }
        rest = stripped;
    }
    if let Some(end) = rest.find(']') {
        let marker = &rest[..end];
        if marker.eq_ignore_ascii_case("mst") || marker.eq_ignore_ascii_case("blk") {
            rest = rest[end + 1..].trim_start();
        }
    }
    let newline = rest.find('\n').unwrap_or(rest.len());
    let heading = rest[..newline].trim();
    let body = rest.get(newline + 1..).unwrap_or("");
    let sep = heading.find(" — ")?;
    let id = heading[..sep].trim();
    let title = heading[sep + " — ".len()..].trim();
    (!id.is_empty() && !title.is_empty()).then_some((id, title, body))
}

/// A `**FIELD**` run from the seven, and the offset its value starts at.
/// Preceded by a bullet; separator (`— - – :`) after the run is skipped.
fn milestone_label(line: &str) -> Option<(&str, usize)> {
    let opening = line.find("**")?;
    let rest = &line[opening + 2..];
    let close = rest.find("**")?;
    let bolded = &rest[..close];
    if !MILESTONE_FIELDS.contains(&bolded) {
        return None;
    }
    let run_end = line[opening + 4 + close..]
        .bytes()
        .position(|byte| byte != b'*')
        .map(|offset| opening + 4 + close + offset)
        .unwrap_or(line.len());
    let after = &line[run_end..];
    let value_at = after
        .char_indices()
        .find(|(_, character)| {
            !character.is_whitespace() && *character != '—' && *character != '-' && *character != '–' && *character != ':'
        })
        .map(|(offset, _)| run_end + offset)
        .unwrap_or(line.len());
    Some((bolded, value_at))
}

/// `**Label.**` opening the line: bold, period inside the bold, ≤ 4 words.
fn blocker_label(line: &str) -> Option<(&str, usize)> {
    let rest = line.strip_prefix("**")?;
    let close = rest.find("**")?;
    let bolded = &rest[..close];
    if !bolded.ends_with('.') {
        return None;
    }
    if bolded.trim_end_matches('.').split_whitespace().count() > 4 {
        return None;
    }
    let marker_end = line[close + 2..]
        .bytes()
        .position(|byte| byte != b'*')
        .map(|offset| close + 2 + offset)
        .unwrap_or(line.len());
    Some((bolded, marker_end))
}

/// Meaning of a blocker label. Period trimmed first; `Stops.` would not match
/// `stops` otherwise.
fn classify_blocker_label(label: &str) -> BlockerField {
    let lowered = label.trim_end_matches('.').to_ascii_lowercase();
    if lowered == "stop" {
        BlockerField::Rule
    } else if lowered.starts_with("owner") {
        BlockerField::Owner
    } else if lowered.contains("stop") || lowered.contains("block") {
        BlockerField::Stops
    } else {
        BlockerField::Other
    }
}

fn malformed(detail: impl Into<String>) -> RefusalSet {
    let mut refusals = RefusalSet::new();
    refusals.push(Refusal::owned(RefusalKind::MalformedRecord, detail));
    refusals
}

#[cfg(test)]
mod tests {
    use super::*;

    const MILESTONE: &str = "\
M14 — FIRST-WORLD: the three-package demo

- **MISSION** — The milestone session-rows already records: a flat scoped world.
- **SCOPE** — the demo world, the three packages.
- **CONSTRAINTS** — the person plays; the agent logs.
- **RESOURCES** — session-rows.jsonl.
- **EVIDENCE** — the three packages load and are shown.
- **DELIVERABLE** — The demo, played by the user.
- **STOP** — Never test the client invisibly.
";

    const BLOCKER: &str = "\
B-236 — Streaming is unwired, not broken

**STOP.** Do not build a new streamer before wiring the existing one.

**Stops.** L12's M19 cannot extend until the horizon test can fail.

**Owner.** A lane.
";

    #[test]
    fn a_milestone_reads_all_seven_fields_verbatim() {
        let milestone = parse_milestone_section(MILESTONE).expect("the filing parses");
        assert_eq!(milestone.id, "M14");
        assert_eq!(milestone.title, "FIRST-WORLD: the three-package demo");
        assert_eq!(milestone.fields.len(), 7);
        assert_eq!(
            milestone.deliverable(),
            Some("The demo, played by the user.")
        );
        assert_eq!(
            milestone.field("MISSION"),
            Some("The milestone session-rows already records: a flat scoped world.")
        );
    }

    #[test]
    fn a_milestone_with_no_deliverable_is_refused_by_name() {
        let body = "M1 — no finish\n\n- **MISSION** — does a thing.\n- **STOP** — stops.\n";
        let refused = parse_milestone_section(body).expect_err("no deliverable, no milestone");
        assert_eq!(refused.all()[0].kind(), RefusalKind::MalformedRecord);
        assert!(refused.all()[0].detail().contains("DELIVERABLE"));
    }

    #[test]
    fn a_milestone_filed_under_the_marker_parses_to_the_same_milestone() {
        let marked = MILESTONE.replace("M14 —", "[MST] M14 —");
        let plain = parse_milestone_section(MILESTONE).expect("parses");
        let from_marker = parse_milestone_section(&marked).expect("parses");
        assert_eq!(plain, from_marker);
    }

    #[test]
    fn a_blocker_reads_its_rule_and_its_two_optional_fields() {
        let blocker = parse_blocker_section(BLOCKER).expect("the filing parses");
        assert_eq!(blocker.id, "B-236");
        assert_eq!(blocker.rule, "Do not build a new streamer before wiring the existing one.");
        assert!(blocker.stops.expect("a stops field").contains("M19"));
        assert_eq!(blocker.owner.as_deref(), Some("A lane."));
    }

    #[test]
    fn a_blocker_with_no_rule_is_refused_rather_than_given_a_provisional_one() {
        let body = "B-1 — a note\n\n**Stops.** nothing yet.\n";
        let refused = parse_blocker_section(body).expect_err("no rule, no blocker");
        assert_eq!(refused.all()[0].kind(), RefusalKind::MalformedRecord);
        assert!(refused.all()[0].detail().contains("STOP"));
    }

    #[test]
    fn the_consequence_is_recognised_under_any_of_its_measured_spellings() {
        for label in ["Stops", "What it stops", "What it blocks", "STOPS"] {
            let body = format!("B-2 — spelling\n\n**STOP.** obey.\n\n**{label}.** the consequence.\n");
            let blocker = parse_blocker_section(&body).expect(label);
            assert_eq!(blocker.stops.as_deref(), Some("the consequence."), "{label}");
        }
    }

    #[test]
    fn a_bold_paragraph_is_not_a_label() {
        // `**STOP.**` requires the bold to open the line and close on a label;
        // a paragraph that merely opens with bold is not one.
        let body = "B-3 — not a label\n\n**STOP.** obey.\n\n**A very long bolded sentence that keeps going.** swallows nothing.\n";
        let blocker = parse_blocker_section(body).expect("parses");
        assert_eq!(blocker.rule, "obey.");
    }

    #[test]
    fn identity_moves_when_a_field_changes_and_not_with_map_order() {
        let first = parse_milestone_section(MILESTONE).expect("parses");
        let changed = parse_milestone_section(&MILESTONE.replace("played by the user", "played twice"))
            .expect("parses");
        assert_ne!(first.content_identity(), changed.content_identity());
        let same = parse_milestone_section(MILESTONE).expect("parses");
        assert_eq!(first.content_identity(), same.content_identity());
    }
}
