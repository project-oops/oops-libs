//! Markdown to egui.
//!
//! # Why a renderer here rather than a crate off the shelf
//!
//! `egui_commonmark` exists and does this. It is also pinned to an egui version, so taking it
//! would mean every future egui bump in this collection waits on a matching release of it - for
//! four projects, to render documents that use about eight markdown features between them.
//! `pulldown-cmark` is the parser underneath most of the ecosystem and depends on nothing that
//! moves; what is written here is the display half only.
//!
//! # Two passes, on purpose
//!
//! Parsing produces a flat [`Block`] list first, and egui draws that. Rendering straight from
//! the event stream means holding layout state across events - am I in a list, how deep, is this
//! cell a header - which is where that kind of code goes wrong. A flat list with an explicit
//! `indent` handles nesting without recursion and can be tested without a UI at all.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// A run of text with one style.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span {
    /// The text itself.
    pub text: String,
    /// Rendered bold.
    pub strong: bool,
    /// Rendered italic.
    pub emphasis: bool,
    /// Rendered as inline code.
    pub code: bool,
    /// Where this points, if it is a link.
    pub link: Option<String>,
}

impl Span {
    /// A plain run of text.
    #[must_use]
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }
}

/// One drawable block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A heading, `level` 1-6.
    Heading {
        /// How large, 1 being largest.
        level: u8,
        /// Its content.
        spans: Vec<Span>,
    },
    /// A run of prose.
    Paragraph {
        /// Its content.
        spans: Vec<Span>,
        /// Nesting depth, in list levels.
        indent: u8,
    },
    /// One item of a list.
    ListItem {
        /// The bullet or number already resolved to text, so drawing needs no counter.
        marker: String,
        /// Its content.
        spans: Vec<Span>,
        /// Nesting depth.
        indent: u8,
    },
    /// A fenced or indented code block.
    Code {
        /// The code, newlines intact.
        text: String,
        /// The info string, when the fence carried one.
        language: Option<String>,
    },
    /// A quoted passage.
    Quote {
        /// Its content.
        spans: Vec<Span>,
    },
    /// One row of a table.
    ///
    /// Rows rather than a table object: these documents use tables as reference material read
    /// top to bottom, and a flat row list draws correctly however ragged the source is.
    TableRow {
        /// The cells, left to right.
        cells: Vec<Vec<Span>>,
        /// Whether this is the header row.
        header: bool,
    },
    /// A horizontal rule.
    Rule,
}

/// Parses markdown into blocks.
///
/// Tables and strikethrough are enabled because these documents use them; everything else is
/// `CommonMark`.
#[must_use]
pub fn parse(source: &str) -> Vec<Block> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let mut walker = Walker::default();
    for event in Parser::new_ext(source, options) {
        walker.event(event);
    }
    walker.blocks
}

/// The state a walk over the event stream has to carry.
///
/// A struct rather than a pile of locals in one long loop. That is this module's whole argument:
/// what goes wrong in an event-driven renderer is layout state nobody can see, and eight mutable
/// locals threaded through a hundred-line match is exactly that state with nowhere to write its
/// name down.
#[derive(Default)]
struct Walker {
    blocks: Vec<Block>,
    /// Inline content accumulated since the last block ended.
    spans: Vec<Span>,
    /// The styles currently open, applied to each new run of text.
    style: Span,
    /// How many list levels deep.
    indent: u8,
    /// One counter per list level, `None` for a bulleted list. A stack, because a numbered list
    /// can contain a bulleted one and then carry on counting.
    counters: Vec<Option<u64>>,
    /// The code block being collected, if inside one.
    code: Option<(String, Option<String>)>,
    /// The heading being collected, if inside one.
    heading: Option<u8>,
    in_quote: bool,
    in_header_row: bool,
    /// Cells of the table row being collected.
    cells: Vec<Vec<Span>>,
}

impl Walker {
    /// Takes the accumulated inline content, leaving the buffer empty.
    fn take(&mut self) -> Vec<Span> {
        std::mem::take(&mut self.spans)
    }

    /// Emits whatever inline content is pending as a list item at the current depth.
    fn flush_item(&mut self) {
        let spans = self.take();
        if spans.is_empty() {
            return;
        }
        let marker = next_marker(&mut self.counters);
        let indent = self.indent.saturating_sub(1);
        self.blocks.push(Block::ListItem {
            marker,
            spans,
            indent,
        });
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => {
                if let Some((body, _)) = self.code.as_mut() {
                    body.push_str(&text);
                } else {
                    self.spans.push(Span {
                        text: text.to_string(),
                        ..self.style.clone()
                    });
                }
            }
            Event::Code(text) => self.spans.push(Span {
                text: text.to_string(),
                code: true,
                ..self.style.clone()
            }),
            Event::SoftBreak => self.spans.push(Span::plain(" ")),
            Event::HardBreak => self.spans.push(Span::plain("\n")),
            Event::Rule => self.blocks.push(Block::Rule),
            // Html, footnotes, task markers and the rest: not used by these documents, and
            // dropping them reads better than rendering their source.
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => self.heading = Some(heading_level(level)),
            Tag::List(first) => {
                // A *tight* list holds its item text directly, with no paragraph around it, so
                // there is no end-of-paragraph to flush the parent item before a nested list
                // starts. Without this the parent's own text is swallowed into the first child
                // and the parent item is never emitted at all - a numbered item with a nested
                // bullet produced two items instead of three, and the outer numbering then ran
                // short.
                if self.indent > 0 {
                    self.flush_item();
                }
                self.indent = self.indent.saturating_add(1);
                self.counters.push(first);
            }
            Tag::BlockQuote(_) => self.in_quote = true,
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) if !info.is_empty() => Some(info.to_string()),
                    _ => None,
                };
                self.code = Some((String::new(), language));
            }
            Tag::Strong => self.style.strong = true,
            Tag::Emphasis => self.style.emphasis = true,
            Tag::Link { dest_url, .. } => self.style.link = Some(dest_url.to_string()),
            Tag::TableHead => self.in_header_row = true,
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                if let Some(level) = self.heading.take() {
                    let spans = self.take();
                    self.blocks.push(Block::Heading { level, spans });
                }
            }
            TagEnd::Paragraph => {
                let spans = self.take();
                if spans.is_empty() {
                    return;
                }
                let indent = self.indent;
                self.blocks.push(if self.in_quote {
                    Block::Quote { spans }
                } else {
                    Block::Paragraph { spans, indent }
                });
            }
            TagEnd::Item => self.flush_item(),
            TagEnd::List(_) => {
                self.indent = self.indent.saturating_sub(1);
                self.counters.pop();
            }
            TagEnd::BlockQuote(_) => self.in_quote = false,
            TagEnd::CodeBlock => {
                if let Some((text, language)) = self.code.take() {
                    self.blocks.push(Block::Code {
                        // The fence's own trailing newline is not part of the code, and leaving
                        // it draws an empty final line inside the frame.
                        text: text.trim_end_matches('\n').to_owned(),
                        language,
                    });
                }
            }
            TagEnd::Strong => self.style.strong = false,
            TagEnd::Emphasis => self.style.emphasis = false,
            TagEnd::Link => self.style.link = None,
            TagEnd::TableCell => {
                let cell = self.take();
                self.cells.push(cell);
            }
            TagEnd::TableHead => {
                self.push_row(true);
                self.in_header_row = false;
            }
            TagEnd::TableRow => {
                let header = self.in_header_row;
                self.push_row(header);
            }
            _ => {}
        }
    }

    /// Emits the collected cells as one row.
    fn push_row(&mut self, header: bool) {
        let cells = std::mem::take(&mut self.cells);
        self.blocks.push(Block::TableRow { cells, header });
    }
}

/// The bullet or number for the next item at the innermost level.
fn next_marker(counters: &mut [Option<u64>]) -> String {
    match counters.last_mut() {
        Some(Some(n)) => {
            let marker = format!("{n}.");
            *n += 1;
            marker
        }
        _ => "\u{2022}".to_owned(),
    }
}

/// `pulldown-cmark`'s heading level as a number.
const fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(spans: &[Span]) -> String {
        spans.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn a_heading_and_a_paragraph_come_out_separately() {
        let blocks = parse("# Title\n\nSome prose.\n");
        assert_eq!(blocks.len(), 2);
        let Block::Heading { level, spans } = &blocks[0] else {
            panic!("expected a heading, got {:?}", blocks[0])
        };
        assert_eq!(*level, 1);
        assert_eq!(text_of(spans), "Title");
    }

    #[test]
    fn inline_styles_survive_as_flags_rather_than_markers() {
        let blocks = parse("a **bold** and `code` word\n");
        let Block::Paragraph { spans, .. } = &blocks[0] else {
            panic!("expected a paragraph")
        };
        assert!(spans.iter().any(|s| s.strong && s.text == "bold"));
        assert!(spans.iter().any(|s| s.code && s.text == "code"));
        // The asterisks and backticks are gone, not escaped.
        assert!(!text_of(spans).contains('*'));
    }

    #[test]
    fn a_numbered_list_numbers_itself() {
        let blocks = parse("1. one\n2. two\n3. three\n");
        let markers: Vec<&str> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::ListItem { marker, .. } => Some(marker.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(markers, ["1.", "2.", "3."]);
    }

    #[test]
    fn a_nested_list_indents_and_the_outer_one_keeps_counting() {
        let blocks = parse("1. one\n   - inner\n2. two\n");
        let items: Vec<(&str, u8)> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::ListItem { marker, indent, .. } => Some((marker.as_str(), *indent)),
                _ => None,
            })
            .collect();
        // Document order: the parent item, then its child, then the next parent - and `two`
        // is still 2 rather than restarting at 1.
        assert_eq!(items, [("1.", 0), ("\u{2022}", 1), ("2.", 0)]);
    }

    #[test]
    fn a_fenced_block_keeps_its_language_and_loses_its_fence() {
        let blocks = parse("```bash\ncargo test\n```\n");
        let Block::Code { text, language } = &blocks[0] else {
            panic!("expected code, got {:?}", blocks[0])
        };
        assert_eq!(text, "cargo test");
        assert_eq!(language.as_deref(), Some("bash"));
        assert!(
            !text.ends_with('\n'),
            "trailing newline draws an empty line"
        );
    }

    #[test]
    fn a_table_becomes_rows_with_the_header_marked() {
        let blocks = parse("| a | b |\n|---|---|\n| 1 | 2 |\n");
        let rows: Vec<(bool, usize)> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::TableRow { cells, header } => Some((*header, cells.len())),
                _ => None,
            })
            .collect();
        assert_eq!(rows, [(true, 2), (false, 2)]);
    }

    #[test]
    fn a_link_carries_its_destination() {
        let blocks = parse("see [the docs](docs/README.md)\n");
        let Block::Paragraph { spans, .. } = &blocks[0] else {
            panic!("expected a paragraph")
        };
        let link = spans.iter().find(|s| s.link.is_some()).expect("a link");
        assert_eq!(link.text, "the docs");
        assert_eq!(link.link.as_deref(), Some("docs/README.md"));
    }

    #[test]
    fn nothing_at_all_parses_to_nothing() {
        assert!(parse("").is_empty());
        assert!(parse("   \n\n  \n").is_empty());
    }
}
