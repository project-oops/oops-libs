//! Documentation embedded in the binary, and an egui window to read it in.
//!
//! The pages are compiled in, so they always match the running build.
//!
//! ```ignore
//! use oops_docs::{Doc, DocsWindow};
//!
//! const DOCS: &[Doc] = &[
//!     Doc::new("running", "Running a title", "Loading, and what happens after",
//!              include_str!("../../../docs/features/running.md")),
//! ];
//!
//! // once, in the app
//! let mut docs = DocsWindow::default();
//! // in the menu
//! if ui.button("documentation...").clicked() { docs.open(); }
//! // once per frame
//! docs.show(ctx, DOCS);
//! ```
//!
//! The page list lives in the consumer because `include_str!` resolves relative to the file it
//! is written in (D004). Register user-facing pages, not decision logs or worklogs.

mod markdown;

pub use markdown::{Block, Span, parse};

use std::collections::HashMap;

/// One embedded page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Doc {
    /// Stable identifier, used to address the page and to resolve links to it.
    pub slug: &'static str,
    /// The name shown in the list.
    pub name: &'static str,
    /// One line shown under the name.
    pub blurb: &'static str,
    /// The markdown, from `include_str!`.
    pub body: &'static str,
}

impl Doc {
    /// Declares a page. `const`, so a registry can be a `const`.
    #[must_use]
    pub const fn new(
        slug: &'static str,
        name: &'static str,
        blurb: &'static str,
        body: &'static str,
    ) -> Self {
        Self {
            slug,
            name,
            blurb,
            body,
        }
    }
}

/// Every problem with a registry: repeated or empty slugs, missing names or blurbs, empty
/// pages, and pages without a top-level heading. Empty when sound.
///
/// `include_str!` only proves a file exists. Consumers pin the rest with a test:
///
/// ```ignore
/// #[test]
/// fn the_registry_is_sound() {
///     assert_eq!(oops_docs::check(DOCS), Vec::<String>::new());
/// }
/// ```
#[must_use]
pub fn check(docs: &[Doc]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for (index, doc) in docs.iter().enumerate() {
        if let Some(first) = seen.insert(doc.slug, index) {
            problems.push(format!(
                "'{}' at {index} repeats the slug used at {first}",
                doc.slug
            ));
        }
        if doc.slug.is_empty() {
            problems.push(format!("the page at {index} has no slug"));
        }
        if doc.name.trim().is_empty() {
            problems.push(format!("'{}' has no name", doc.slug));
        }
        if doc.blurb.trim().is_empty() {
            problems.push(format!("'{}' has no blurb", doc.slug));
        }
        if doc.body.trim().is_empty() {
            problems.push(format!("'{}' is empty", doc.slug));
        } else if !doc.body.trim_start().starts_with("# ") {
            problems.push(format!(
                "'{}' does not start with a top-level heading",
                doc.slug
            ));
        }
    }
    problems
}

/// The reader window. Keep one per application and call [`DocsWindow::show`] every frame.
#[derive(Default)]
pub struct DocsWindow {
    open: bool,
    selected: Option<&'static str>,
    /// Parsed pages by slug. Never invalidated: the source is compiled in.
    parsed: HashMap<&'static str, Vec<Block>>,
}

impl std::fmt::Debug for DocsWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocsWindow")
            .field("open", &self.open)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl DocsWindow {
    /// Opens it on the contents page.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Width of the page list.
    const NAV_WIDTH: f32 = 190.0;

    /// Share of the host window it opens at.
    const SHARE: f32 = 0.9;

    /// Smallest size that leaves a usable column for the page beside the list.
    const MINIMUM: egui::Vec2 = egui::Vec2::new(420.0, 260.0);

    /// Opens it at one page.
    pub fn open_at(&mut self, slug: &'static str) {
        self.open = true;
        self.selected = Some(slug);
    }

    /// Whether it is showing.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    /// Draws it, if open. Call once per frame.
    ///
    /// The size is bounded to the host window and passed into the contents. A window that
    /// sized itself from wrapped text would never wrap it, because the text wraps at the
    /// width the window grants.
    pub fn show(&mut self, ctx: &egui::Context, docs: &[Doc]) {
        if !self.open {
            return;
        }
        let mut open = self.open;
        let screen = ctx.screen_rect();
        let most = egui::vec2(
            (screen.width() * Self::SHARE).max(Self::MINIMUM.x),
            (screen.height() * Self::SHARE).max(Self::MINIMUM.y),
        );
        egui::Window::new("documentation")
            .open(&mut open)
            .default_size(most)
            .min_size(Self::MINIMUM)
            .max_size(screen.size())
            .default_pos(screen.center())
            .pivot(egui::Align2::CENTER_CENTER)
            .collapsible(false)
            .resizable(true)
            // Keeps the scrollbar reachable after a drag.
            .constrain(true)
            .show(ctx, |ui| self.contents(ui, docs, most));
        self.open = open;
    }

    /// The list on the left and the page on the right, laid out in explicit sizes no larger
    /// than `most`. Panels are not used: inside a self-sizing window they take whatever width
    /// the content asks for, and the text never wraps.
    fn contents(&mut self, ui: &mut egui::Ui, docs: &[Doc], most: egui::Vec2) {
        let room = egui::vec2(
            ui.available_width().min(most.x),
            ui.available_height().min(most.y),
        );
        ui.set_max_size(room);
        // Floored, so a very small window gives a narrow page rather than a negative width.
        let nav = Self::NAV_WIDTH.min(room.x * 0.4);
        let page = (room.x - nav - 12.0).max(80.0);

        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(nav, room.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_min_size(egui::vec2(nav, room.y));
                    egui::ScrollArea::vertical()
                        .id_salt("docs-nav")
                        .show(ui, |ui| {
                            for doc in docs {
                                let chosen = self.selected == Some(doc.slug);
                                if ui
                                    .selectable_label(chosen, doc.name)
                                    .on_hover_text(doc.blurb)
                                    .clicked()
                                {
                                    self.selected = Some(doc.slug);
                                }
                            }
                        });
                },
            );
            ui.separator();
            ui.allocate_ui_with_layout(
                egui::vec2(page, room.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    // Fixed before any text is laid out; every wrap below reads this width.
                    ui.set_min_size(egui::vec2(page, room.y));
                    ui.set_max_size(egui::vec2(page, room.y));
                    // Both directions: code blocks and tables do not wrap.
                    egui::ScrollArea::both()
                        .id_salt("docs-page")
                        .auto_shrink([false, false])
                        .show(ui, |ui| match self.selected {
                            Some(slug) => self.page(ui, docs, slug),
                            None => Self::index(ui, docs, &mut self.selected),
                        });
                },
            );
        });
    }

    /// The contents page, shown until a page is picked.
    fn index(ui: &mut egui::Ui, docs: &[Doc], selected: &mut Option<&'static str>) {
        ui.heading("Documentation");
        ui.add_space(8.0);
        for doc in docs {
            if ui.link(egui::RichText::new(doc.name).strong()).clicked() {
                *selected = Some(doc.slug);
            }
            ui.label(egui::RichText::new(doc.blurb).weak());
            ui.add_space(6.0);
        }
        if docs.is_empty() {
            ui.label(egui::RichText::new("This build ships no documentation pages.").weak());
        }
    }

    /// One page.
    fn page(&mut self, ui: &mut egui::Ui, docs: &[Doc], slug: &'static str) {
        let Some(doc) = docs.iter().find(|d| d.slug == slug) else {
            ui.label(format!("There is no page called '{slug}'."));
            if ui.button("contents").clicked() {
                self.selected = None;
            }
            return;
        };
        if ui.button("\u{2190} contents").clicked() {
            self.selected = None;
        }
        ui.add_space(4.0);

        let blocks = self.parsed.entry(slug).or_insert_with(|| parse(doc.body));
        let mut follow = None;
        for block in blocks.iter() {
            draw(ui, block, docs, &mut follow);
        }
        if let Some(next) = follow {
            self.selected = Some(next);
        }
    }
}

/// Draws one block.
fn draw(ui: &mut egui::Ui, block: &Block, docs: &[Doc], follow: &mut Option<&'static str>) {
    match block {
        Block::Heading { level, spans } => {
            ui.add_space(if *level <= 2 { 10.0 } else { 6.0 });
            let size = match level {
                1 => 22.0,
                2 => 18.0,
                3 => 16.0,
                _ => 14.0,
            };
            ui.horizontal_wrapped(|ui| {
                for span in spans {
                    ui.label(style(span).size(size).strong());
                }
            });
            ui.add_space(2.0);
        }
        Block::Paragraph { spans, indent } => {
            indented(ui, *indent, |ui| inline(ui, spans, docs, follow));
            ui.add_space(4.0);
        }
        Block::ListItem {
            marker,
            spans,
            indent,
        } => {
            indented(ui, *indent, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(format!("{marker} ")).monospace());
                    render_spans(ui, spans, docs, follow);
                });
            });
        }
        Block::Code { text, language } => {
            ui.add_space(4.0);
            egui::Frame::group(ui.style()).show(ui, |ui| {
                if let Some(language) = language {
                    ui.label(egui::RichText::new(language).weak().small());
                }
                // A read-only text edit, so the code can be selected and copied.
                ui.add(
                    egui::TextEdit::multiline(&mut text.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .code_editor(),
                );
            });
            ui.add_space(4.0);
        }
        Block::Quote { spans } => {
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for span in spans {
                            ui.label(style(span).italics().weak());
                        }
                    });
                });
        }
        Block::TableRow { cells, header } => {
            ui.horizontal_wrapped(|ui| {
                for cell in cells {
                    // Fixed column width: one pass per row, and even columns.
                    ui.allocate_ui_with_layout(
                        egui::vec2(160.0, 0.0),
                        egui::Layout::left_to_right(egui::Align::TOP),
                        |ui| {
                            for span in cell {
                                let text = style(span);
                                ui.label(if *header { text.strong() } else { text });
                            }
                        },
                    );
                }
            });
            if *header {
                ui.separator();
            }
        }
        Block::Rule => {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
        }
    }
}

/// Runs `body` indented by `indent` list levels.
fn indented(ui: &mut egui::Ui, indent: u8, body: impl FnOnce(&mut egui::Ui)) {
    if indent == 0 {
        body(ui);
        return;
    }
    ui.horizontal(|ui| {
        ui.add_space(f32::from(indent) * 16.0);
        ui.vertical(|ui| body(ui));
    });
}

/// A wrapped run of styled text.
fn inline(ui: &mut egui::Ui, spans: &[Span], docs: &[Doc], follow: &mut Option<&'static str>) {
    ui.horizontal_wrapped(|ui| render_spans(ui, spans, docs, follow));
}

/// The spans, in the current layout.
///
/// Plain text is one label per word, because `horizontal_wrapped` wraps between items. A link
/// to a shipped page opens it here; an `http` link opens the browser.
fn render_spans(
    ui: &mut egui::Ui,
    spans: &[Span],
    docs: &[Doc],
    follow: &mut Option<&'static str>,
) {
    for span in spans {
        match span.link.as_deref() {
            Some(dest) => {
                if ui.link(style(span)).clicked() {
                    if let Some(doc) = resolve(dest, docs) {
                        *follow = Some(doc);
                    } else if dest.starts_with("http") {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(dest));
                    }
                }
            }
            None => {
                for word in span.text.split_inclusive(' ') {
                    if word.trim().is_empty() {
                        continue;
                    }
                    let mut piece = span.clone();
                    word.clone_into(&mut piece.text);
                    ui.label(style(&piece));
                }
            }
        }
    }
}

/// The page a link destination names, matched on file stem, so `../features/running.md`,
/// `running.md#part` and `running` all reach the same page.
fn resolve(dest: &str, docs: &[Doc]) -> Option<&'static str> {
    if dest.starts_with("http") {
        return None;
    }
    let stem = dest
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(dest)
        .split('#')
        .next()
        .unwrap_or(dest)
        .trim_end_matches(".md");
    docs.iter().find(|d| d.slug == stem).map(|d| d.slug)
}

/// One span as egui text.
fn style(span: &Span) -> egui::RichText {
    let mut text = egui::RichText::new(&span.text);
    if span.code {
        text = text.monospace();
    }
    if span.strong {
        text = text.strong();
    }
    if span.emphasis {
        text = text.italics();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &[Doc] = &[
        Doc::new("one", "One", "The first", "# One\n\nBody.\n"),
        Doc::new("two", "Two", "The second", "# Two\n\nBody.\n"),
    ];

    /// A sound registry reports nothing.
    #[test]
    fn a_sound_registry_has_nothing_to_report() {
        assert_eq!(check(GOOD), Vec::<String>::new());
    }

    /// An empty page is reported.
    #[test]
    fn a_truncated_page_is_caught() {
        const DOCS: &[Doc] = &[Doc::new("gone", "Gone", "blurb", "")];
        let problems = check(DOCS);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("empty"), "{problems:?}");
    }

    /// A repeated slug is reported once.
    #[test]
    fn a_repeated_slug_is_caught() {
        const DOCS: &[Doc] = &[
            Doc::new("same", "A", "blurb", "# A\n"),
            Doc::new("same", "B", "blurb", "# B\n"),
        ];
        assert_eq!(check(DOCS).len(), 1);
    }

    /// A page without a top-level heading is reported.
    #[test]
    fn a_page_without_a_title_is_caught() {
        const DOCS: &[Doc] = &[Doc::new("x", "X", "blurb", "Just prose, no heading.\n")];
        let problems = check(DOCS);
        assert!(problems[0].contains("heading"), "{problems:?}");
    }

    /// Every link form resolves to the page; unknown pages and external links do not.
    #[test]
    fn links_between_pages_resolve_however_they_are_written() {
        for form in ["two", "two.md", "../features/two.md", "two.md#a-section"] {
            assert_eq!(resolve(form, GOOD), Some("two"), "{form}");
        }
        assert_eq!(resolve("three.md", GOOD), None);
        assert_eq!(resolve("https://example.com/two.md", GOOD), None);
    }

    /// An empty registry is valid.
    #[test]
    fn an_empty_registry_is_not_an_error() {
        assert_eq!(check(&[]), Vec::<String>::new());
    }
}
