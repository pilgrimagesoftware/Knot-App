//! The text a reply renders, worked out from its Markdown without laying it
//! out, so find counts matches where it paints them (#251).
//!
//! A reply's highlights index the text its `TextViewState` renders, which
//! exists only for replies laid out near the viewport. Counting matches in
//! the Markdown source instead put the two out of step whenever a match fell
//! inside syntax the reader never sees - `see [docs](https://x/docs) and
//! docs` counts three "docs" in the source and paints two, so the second
//! counted match painted the third. Counting over this text keeps the *n*th
//! counted match the *n*th painted one.
//!
//! The rules are `gpui-base`'s for its rendered text (`RenderedText`): marks
//! and escapes resolved, heading and list markers left out, a link as its
//! text and not its address, images and HTML as nothing, a soft line break
//! as a space, each block - a quote and a table included - ending in a
//! newline, and table cells joined by a space. The tests hold this
//! against a real `TextViewState`, so a toolkit change that moves the rules
//! fails here rather than painting the wrong match.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use markdown::ParseOptions;
use markdown::mdast::Node;

/// The text `source` renders as. Markdown that fails to parse - which the
/// GFM parser does not do for ordinary text - falls back to the source.
pub(crate) fn plain_text(source: &str) -> String {
    // GFM without math: `gpui-base` parses math too, but turns math no
    // plugin claims back into prose, so `$5 and $10` renders as written.
    let Ok(root) = markdown::to_mdast(source, &ParseOptions::gfm())
    else {
        return source.to_owned();
    };
    let mut out = String::new();
    push_block(&root, &mut out);
    out
}

fn push_block(node: &Node, out: &mut String) {
    let start = out.len();
    match node {
        // A list and its items add nothing of their own; a quote ends with a
        // newline after its last block, as every block does.
        Node::Root(_) | Node::List(_) | Node::ListItem(_) => {
            for child in node.children().into_iter().flatten() {
                push_block(child, out);
            }
            return;
        }
        Node::Blockquote(_) => {
            for child in node.children().into_iter().flatten() {
                push_block(child, out);
            }
        }
        Node::Paragraph(_) | Node::Heading(_) => push_inlines(node, out),
        Node::Code(code) => out.push_str(&code.value),
        Node::Math(math) => out.push_str(&math.value),
        Node::Table(table) => {
            for row in &table.children {
                for (index, cell) in row.children().into_iter().flatten().enumerate() {
                    if index > 0 {
                        out.push(' ');
                    }
                    push_inlines(cell, out);
                }
                out.push('\n');
            }
        }
        _ => {}
    }
    if out.len() > start {
        out.push('\n');
    }
}

fn push_inlines(node: &Node, out: &mut String) {
    for child in node.children().into_iter().flatten() {
        match child {
            // A line break inside a paragraph is a space; only a hard break
            // (two trailing spaces, or a backslash) ends a line.
            Node::Text(text) => {
                out.extend(text.value.chars().map(|c| if c == '\n' { ' ' } else { c }))
            }
            Node::InlineCode(code) => out.push_str(&code.value),
            Node::InlineMath(math) => out.push_str(&math.value),
            Node::Break(_) => out.push('\n'),
            Node::Image(_) | Node::ImageReference(_) | Node::Html(_) => {}
            other => push_inlines(other, out),
        }
    }
}

/// [`plain_text`] per message, worked out again only when the message's
/// text changes - while a reply streams, that is the last one alone.
#[derive(Default)]
pub(crate) struct PlainTexts(HashMap<usize, (u64, String)>);

impl PlainTexts {
    /// The rendered text of message `index`, whose Markdown is `source`.
    pub(crate) fn get(&mut self, index: usize, source: &str) -> &str {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let fingerprint = hasher.finish();
        let entry = self.0
                        .entry(index)
                        .or_insert_with(|| (fingerprint.wrapping_add(1), String::new()));
        if entry.0 != fingerprint {
            *entry = (fingerprint, plain_text(source));
        }
        &entry.1
    }

    /// Forgets messages at or past `len`: a reloaded conversation can bring
    /// a different message back at an index this still remembers.
    pub(crate) fn truncate(&mut self, len: usize) {
        self.0.retain(|&index, _| index < len);
    }
}

#[cfg(test)]
mod tests;
