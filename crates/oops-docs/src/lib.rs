//! Documentation that ships inside the binary.
//!
//! Because the pages ride in the executable, they are always accurate to the build a person is
//! running - there is no version to keep in step and nothing to fetch.
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
//! # Why the registry lives in the consumer and not here
//!
//! `include_str!` resolves relative to the file it is written in, so this crate cannot embed
//! another crate's documents - the paths would be relative to this one. That is a hard
//! constraint rather than a preference, and it sets the boundary cleanly: the viewer is shared,
//! the list of pages is not.
//!
//! # Ship the manual, not the notebook
//!
//! Point [`Doc::new`] at pages written for somebody using the tool. A decision log or a worklog
//! is a development record - useful, in the repository, and not what somebody clicking
//! *documentation* is asking for. They are also large: one of these projects has a decision log
//! approaching a megabyte, and embedding it costs that in every binary.

mod markdown;

pub use markdown::{Block, Span, parse};

use std::collections::HashMap;

/// One page, embedded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Doc {
    /// Stable identifier, used for addressing and for links between pages.
    pub slug: &'static str,
    /// What the reader sees in the list.
    pub name: &'static str,
    /// One line under the name.
    pub blurb: &'static str,
    /// The markdown itself, from `include_str!`.
    pub body: &'static str,
}

impl Doc {
    /// Declares a page. `const` so a registry can be a `const` too.
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

/// Everything wrong with a registry, empty when there is nothing.
///
/// # Why this exists at all
///
/// `include_str!` proves at compile time that a file *exists*. It cannot notice that one was
/// truncated to nothing, that two entries claim the same slug, or that a page has no title -
/// and all three ship silently, because a documentation window showing an empty page looks like
/// a page that has not been written yet.
///
/// Consumers should pin it:
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

/// The reader.
///
/// Holds which page is open and the parsed form of the pages already looked at. Keep one per
/// application and call [`DocsWindow::show`] every frame.
#[derive(Default)]
pub struct DocsWindow {
    open: bool,
    selected: Option<&'static str>,
    // Parsing on every frame would re-parse a megabyte of markdown sixty times a second. Keyed
    // by slug, and never invalidated because the source is baked into the binary.
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
    /// Opens it, on the contents page.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// How wide the list of pages is.
    const NAV_WIDTH: f32 = 190.0;

    /// How much of the window it is drawn on this one takes, before anybody resizes it.
    const SHARE: f32 = 0.9;

    /// The smallest this window is allowed to be.
    ///
    /// Below roughly this, the fixed-width list of pages on the left leaves no usable column
    /// for the page itself, and a reader is resizing a window to see one word per line.
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

    /// Draws it, if it is open. Call once per frame.
    ///
    /// # Why the size is a constraint and not a preference
    ///
    /// `default_size` sets where a window starts and bounds nothing. A window with panels
    /// inside it and no bound sizes itself from its content - and every paragraph here is laid
    /// out with `horizontal_wrapped`, which wraps at `ui.available_width()`.
    ///
    /// Those two together have no fixed point. The text asks how wide it may be, the window
    /// answers *as wide as your content*, so nothing ever wraps: one enormous line per
    /// paragraph, justified across a width far past the frame, painted straight over whatever
    /// the window is sitting on. The frame stays the size it was drawn at, which is why it
    /// looks like a container that has stopped containing.
    ///
    /// So the width is decided before the content is asked, and both directions are bounded to
    /// the screen. A page too wide or too long to fit now scrolls, which is what somebody
    /// reaches for a scrollbar expecting.
    pub fn show(&mut self, ctx: &egui::Context, docs: &[Doc]) {
        if !self.open {
            return;
        }
        let mut open = self.open;
        // **Nine tenths of the window it is drawn on.** A size in pixels is a guess about
        // somebody else's screen; a proportion of the thing it sits inside is not, and it is
        // what a reader means by *a bit smaller than the window*.
        let screen = ctx.screen_rect();
        let most = egui::vec2(
            (screen.width() * Self::SHARE).max(Self::MINIMUM.x),
            (screen.height() * Self::SHARE).max(Self::MINIMUM.y),
        );
        egui::Window::new("documentation")
            .open(&mut open)
            .default_size(most)
            .min_size(Self::MINIMUM)
            // Up to the whole window if somebody drags it there, and no further.
            .max_size(screen.size())
            .default_pos(screen.center())
            .pivot(egui::Align2::CENTER_CENTER)
            .collapsible(false)
            .resizable(true)
            // Kept on screen, so a window dragged half off does not become one whose scrollbar
            // cannot be reached.
            .constrain(true)
            // **The size goes in.** Everything inside lays out to this rather than asking how
            // much room there is - which, in a window that sizes itself from its content, is a
            // question whose answer depends on the answer.
            .show(ctx, |ui| self.contents(ui, docs, most));
        self.open = open;
    }

    /// The window's inside: a list on the left, the page on the right.
    ///
    /// # Why this does not use panels
    ///
    /// It did: a `SidePanel` and a `CentralPanel`, shown inside the window. A panel takes the
    /// space it is given and asks for however much it wants; a window that is sizing itself
    /// gives however much its content asks for. Neither commits to a number, and the paragraphs
    /// below wrap at `ui.available_width()` - so nothing ever wrapped, and the text was laid
    /// out across a width the frame had no idea about and painted straight over the window it
    /// was supposed to be inside.
    ///
    /// Constraining the window did not fix it, because the panels were never reading the
    /// constraint. Replacing them with explicit allocations did not fix it either, because the
    /// number they were allocated from was `ui.available_size()` - and inside a window that is
    /// sizing itself, that is not a measurement, it is the same open question wearing a
    /// different hat. It came back enormous, the columns were allocated enormous, and the window
    /// grew to fit them: wider than the application it was floating over.
    ///
    /// So the ceiling arrives from outside, worked out from the window this is drawn on before
    /// anything here is asked anything. `most` is that ceiling; the room actually used is
    /// whichever of it and the current size is smaller, so dragging the window narrower still
    /// narrows the text.
    fn contents(&mut self, ui: &mut egui::Ui, docs: &[Doc], most: egui::Vec2) {
        let room = egui::vec2(
            ui.available_width().min(most.x),
            ui.available_height().min(most.y),
        );
        ui.set_max_size(room);
        // The list of pages is a fixed column; the page gets the rest, less the separator. Both
        // are floored, so a window dragged very small produces a narrow page rather than a
        // negative width and a panic.
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
                    // **The number every wrap below reads.** Set before a single word is laid
                    // out, so `available_width` answers with this rather than with a question.
                    ui.set_min_size(egui::vec2(page, room.y));
                    ui.set_max_size(egui::vec2(page, room.y));
                    // **Both directions.** Wrapping handles prose, but a code block is one long
                    // line by nature and a table has the width it has. Vertical-only scrolling
                    // left those with nowhere to go but outwards.
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

    /// The contents page, shown until something is picked.
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
            // Says which of the two it is. "No documentation" alone leaves a reader unsure
            // whether the window is broken or the pages were never written.
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

        let blocks = self
            .parsed
            .entry(slug)
            .or_insert_with(|| markdown::parse(doc.body));
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
            // Sizes rather than egui's `heading`, so the six levels stay distinguishable.
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
                // Selectable: a reader who wants a command wants to copy it, and a label they
                // cannot select is a command they have to retype.
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
                    // A fixed column width rather than a measured one: measuring means two
                    // passes over every row, and these tables are reference material where an
                    // even column reads better than a tight one.
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

/// Runs the body of a block at a nesting depth.
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

/// The spans themselves, without opening a layout of their own.
fn render_spans(
    ui: &mut egui::Ui,
    spans: &[Span],
    docs: &[Doc],
    follow: &mut Option<&'static str>,
) {
    // egui lays out horizontal_wrapped by item, so one label per word keeps the wrap points
    // where a reader expects them. A single label per span wraps only between spans, which puts
    // a whole sentence on the next line.
    for span in spans {
        match span.link.as_deref() {
            Some(target) => {
                if ui.link(style(span)).clicked() {
                    // A link to another shipped page opens it here rather than in a browser -
                    // the whole point of embedding them is that they work with no network.
                    if let Some(doc) = resolve(target, docs) {
                        *follow = Some(doc);
                    } else if target.starts_with("http") {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(target));
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

/// The page a link points at, when it points at one of these.
///
/// Matches on the file stem, so `../features/running.md`, `running.md` and `running` all reach
/// the same page - which is what a document written for a repository browser will contain.
fn resolve(target: &str, docs: &[Doc]) -> Option<&'static str> {
    if target.starts_with("http") {
        return None;
    }
    let stem = target
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(target)
        .split('#')
        .next()
        .unwrap_or(target)
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

    #[test]
    fn a_sound_registry_has_nothing_to_report() {
        assert_eq!(check(GOOD), Vec::<String>::new());
    }

    #[test]
    fn a_truncated_page_is_caught() {
        // The failure `include_str!` cannot see: the file exists and is empty.
        const DOCS: &[Doc] = &[Doc::new("gone", "Gone", "blurb", "")];
        let problems = check(DOCS);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("empty"), "{problems:?}");
    }

    #[test]
    fn a_repeated_slug_is_caught() {
        const DOCS: &[Doc] = &[
            Doc::new("same", "A", "blurb", "# A\n"),
            Doc::new("same", "B", "blurb", "# B\n"),
        ];
        assert_eq!(check(DOCS).len(), 1);
    }

    #[test]
    fn a_page_without_a_title_is_caught() {
        const DOCS: &[Doc] = &[Doc::new("x", "X", "blurb", "Just prose, no heading.\n")];
        let problems = check(DOCS);
        assert!(problems[0].contains("heading"), "{problems:?}");
    }

    #[test]
    fn links_between_pages_resolve_however_they_are_written() {
        // All three forms appear in documents written to be read in a repository browser.
        for form in ["two", "two.md", "../features/two.md", "two.md#a-section"] {
            assert_eq!(resolve(form, GOOD), Some("two"), "{form}");
        }
        assert_eq!(resolve("three.md", GOOD), None);
        // An external link is the browser's problem, not a missing page.
        assert_eq!(resolve("https://example.com/two.md", GOOD), None);
    }

    #[test]
    fn an_empty_registry_is_not_an_error() {
        // A tool may ship the viewer before it ships pages; that is a state, not a fault.
        assert_eq!(check(&[]), Vec::<String>::new());
    }
}
