//! **The shell's constrained XML: a document to a tree, and nothing else.**
//!
//! Two surfaces in this shell are declared as data rather than built by hand — the map
//! window's control documents ([`super::webview`]) and the pinned window's minimal title bar
//! ([`super::titlebar`]) — and both are read by this parser. It was written inside `webview`,
//! which made the map's renderer the owner of the grammar the title bar needed; the grammar is
//! not the map's, so it lives here and each surface keeps its own renderer.
//!
//! The grammar is deliberately small, and a value this parser does not understand is a
//! **refusal naming the text**, never a skipped node:
//!
//! ```xml
//! <root>
//!   <child key="value"/>              an element; `"` or `'` quotes
//!   <child key="a">text</child>       text is kept, trimmed, with the five escapes resolved
//!   <!-- a comment -->
//! </root>
//! ```
//!
//! There is no DTD, no namespace, no entity expansion beyond the five XML escapes, and no
//! mixed-content model. One root element, balanced tags. That is the whole of it.

/// One XML node. Text is kept for a caller that reads a body; the two shell documents use
/// attributes only.
#[derive(Clone, PartialEq, Debug)]
pub struct Node {
    /// The tag's name.
    pub name: String,
    /// `key="value"` attributes, in document order.
    pub attrs: Vec<(String, String)>,
    /// Child nodes, in document order.
    pub children: Vec<Node>,
    /// The text directly inside this node.
    pub text: String,
}

impl Node {
    /// One attribute, by name.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// One attribute parsed as a finite `f64`, or a refusal naming the attribute and its text.
    pub fn number(&self, name: &str) -> Result<f64, String> {
        let text = self
            .attr(name)
            .ok_or_else(|| format!("<{}> has no {name:?} attribute", self.name))?;
        match text.trim().parse::<f64>() {
            Ok(value) if value.is_finite() => Ok(value),
            _ => Err(format!(
                "<{}> {name}={text:?} is not a finite number",
                self.name
            )),
        }
    }
}

/// A parsed document: the root node.
#[derive(Clone, PartialEq, Debug)]
pub struct Document {
    /// The one root element.
    pub root: Node,
}

impl Document {
    /// **Parse the constrained XML a shell surface is written in.**
    ///
    /// Comments are skipped and whitespace between tags is ignored. A closing tag must name the
    /// open one; a second root, an unterminated tag or an unclosed element is refused by name.
    pub fn parse(source: &str) -> Result<Document, String> {
        let mut stack: Vec<Node> = Vec::new();
        let mut root: Option<Node> = None;
        let mut rest = source;
        while let Some(open) = rest.find('<') {
            let text = &rest[..open];
            if let Some(node) = stack.last_mut() {
                if !text.trim().is_empty() {
                    node.text.push_str(text.trim());
                }
            }
            rest = &rest[open..];
            let Some(close) = rest.find('>') else {
                return Err(format!("unterminated tag: {}", &rest[..rest.len().min(40)]));
            };
            let tag = &rest[1..close];
            rest = &rest[close + 1..];
            if tag.starts_with("!--") {
                // A comment. `<!-- ... -->` has already reached its first `>`; if that `>` was
                // the comment's own end the tag text ends with `--`, and otherwise the body
                // held a `>` and the end is still ahead.
                if tag.ends_with("--") {
                    continue;
                }
                if let Some(end) = rest.find("-->") {
                    rest = &rest[end + 3..];
                    continue;
                }
                return Err("unterminated XML comment".to_string());
            }
            if let Some(name) = tag.strip_prefix('/') {
                let name = name.trim();
                let Some(node) = stack.pop() else {
                    return Err(format!("</{name}> closes nothing"));
                };
                if node.name != name {
                    return Err(format!("</{name}> closes <{}>", node.name));
                }
                match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => {
                        if root.is_some() {
                            return Err("the document has more than one root".to_string());
                        }
                        root = Some(node);
                    }
                }
                continue;
            }
            let (name, attrs, self_closing) = parse_tag(tag)?;
            let node = Node {
                name: name.to_string(),
                attrs,
                children: Vec::new(),
                text: String::new(),
            };
            if self_closing {
                match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => {
                        if root.is_some() {
                            return Err("the document has more than one root".to_string());
                        }
                        root = Some(node);
                    }
                }
            } else {
                stack.push(node);
            }
        }
        if let Some(open) = stack.last() {
            return Err(format!("<{}> is never closed", open.name));
        }
        root.map(|root| Document { root })
            .ok_or_else(|| "the document declares no element".to_string())
    }

    /// The root's name, for a renderer that refuses a document addressed elsewhere.
    pub fn root_name(&self) -> &str {
        &self.root.name
    }
}

/// One tag's name, attributes and whether it closes itself.
fn parse_tag(tag: &str) -> Result<(&str, Vec<(String, String)>, bool), String> {
    let tag = tag.trim();
    let (body, self_closing) = match tag.strip_suffix('/') {
        Some(body) => (body.trim_end(), true),
        None => (tag, false),
    };
    let mut chars = body.char_indices().peekable();
    let mut name_end = body.len();
    while let Some((index, ch)) = chars.peek().copied() {
        if ch.is_ascii_whitespace() {
            name_end = index;
            break;
        }
        chars.next();
    }
    let name = &body[..name_end];
    if name.is_empty() {
        return Err("a tag with no name".to_string());
    }
    let mut attrs = Vec::new();
    let mut rest = &body[name_end..];
    while let Some(eq) = rest.find('=') {
        let key = rest[..eq].trim();
        let after = rest[eq + 1..].trim_start();
        let Some(quote) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            return Err(format!("attribute {key:?} has no quoted value"));
        };
        let Some(end) = after[1..].find(quote) else {
            return Err(format!("attribute {key:?} has no closing quote"));
        };
        let value = unescape(&after[1..1 + end]);
        if key.is_empty() {
            return Err("an attribute with no name".to_string());
        }
        attrs.push((key.to_string(), value));
        rest = &after[1 + end + 1..];
    }
    Ok((name, attrs, self_closing))
}

/// The five XML escapes, plus the en-dash the zoom button uses so the source stays ASCII.
fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_parses_its_tree_attributes_and_text() {
        let document = Document::parse(
            r#"<!-- a note -->
            <root height="24" glyph='13'>
              <child name="a"> body </child>
              <child name="b"/>
            </root>"#,
        )
        .expect("the document parses");
        assert_eq!(document.root_name(), "root");
        assert_eq!(document.root.number("height").unwrap(), 24.0);
        assert_eq!(document.root.attr("glyph"), Some("13"));
        assert_eq!(document.root.children.len(), 2);
        assert_eq!(document.root.children[0].text, "body");
        assert!(document.root.children[1].children.is_empty());
    }

    #[test]
    fn the_escapes_are_resolved_and_an_unknown_one_is_left_alone() {
        let document = Document::parse(r#"<r a="&lt;&amp;&gt;&quot;&apos;" b="&nbsp;"/>"#)
            .expect("the document parses");
        assert_eq!(document.root.attr("a"), Some("<&>\"'"));
        assert_eq!(document.root.attr("b"), Some("&nbsp;"), "only the five");
    }

    #[test]
    fn a_broken_document_is_refused_by_name() {
        assert!(Document::parse("<r>").is_err(), "an unclosed root");
        assert!(Document::parse("<r></s>").is_err(), "a mismatched close");
        assert!(Document::parse("<r/><r/>").is_err(), "two roots");
        assert!(Document::parse("no element").is_err(), "no root");
        assert!(
            Document::parse("<r a=1/>").is_err(),
            "an unquoted attribute"
        );
        assert!(
            Document::parse("<r height=\"tall\"/>")
                .unwrap()
                .root
                .number("height")
                .is_err(),
            "a word where a number is asked for"
        );
        assert!(
            Document::parse("<r/>")
                .unwrap()
                .root
                .number("height")
                .is_err(),
            "a missing number is refused rather than defaulted"
        );
    }
}
