use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::SystemTime;



use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::render::{self, LinkItem, WikiLink};
use crate::search::{find_next_match, find_prev_match, highlight_line, search_lines};
use crate::theme::Theme;

/// How text wrapping behaves in the TUI viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WrapMode {
    None,
    Word,
    Char,
}

impl WrapMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "none" => WrapMode::None,
            "char" => WrapMode::Char,
            _ => WrapMode::Word,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            WrapMode::None => "none",
            WrapMode::Word => "word",
            WrapMode::Char => "char",
        }
    }
}

/// Application mode — determines how keyboard input is interpreted.
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    SearchForward,
    SearchBackward,
    GoToLine,
    FileList,
}

struct ImagePlacement {
    path: PathBuf,
    line: usize,
    disp_cols: u32,
    disp_rows: u32,
}

/// TUI markdown viewer state and event handling.
pub struct App {
    /// Rendered markdown content
    rendered: RenderedContent,
    /// Scrolling, cursor, and display mode
    view: ViewState,
    /// Search input, results, and history
    search: SearchState,
    /// Open files and navigation history
    file_state: FileState,
    /// Line bookmarks
    bm: BookmarkState,
    /// Current input mode
    mode: Mode,
    /// Transient status message shown in the status bar
    status_message: Option<String>,
    /// Active colour theme
    theme: Theme,
    /// Disable all colours and styles
    no_colour: bool,
    /// Exit immediately on any error
    fail: bool,
    /// Pending numeric prefix count (vim-style `3j` etc.)
    pending_count: Option<usize>,
    /// Image link placements for Kitty-protocol inline rendering
    image_placements: Vec<ImagePlacement>,
    /// Whether terminal supports Kitty graphics protocol (lazily checked)
    kitty_supported: bool,
    /// Whether inline image rendering is enabled
    images_enabled: bool,
    /// Cache of remote image URL → local temp file path
    image_download_cache: HashMap<String, PathBuf>,
}

impl Drop for App {
    fn drop(&mut self) {
        for (_url, path) in self.image_download_cache.drain() {
            let _ = std::fs::remove_file(&path);
        }
    }
}

struct RenderedContent {
    lines: Vec<Line<'static>>,
    raw_lines: Vec<String>,
    wiki_links: Vec<Vec<WikiLink>>,
    links: Vec<Vec<LinkItem>>,
    content: String,
    content_lower_lines: Vec<String>,
    display_cache: Option<Vec<Line<'static>>>,
}

struct ViewState {
    cursor_line: usize,
    cursor_col: u16,
    viewport_height: u16,
    scroll: usize,
    h_scroll: u16,
    max_scroll: usize,
    show_status: bool,
    show_line_numbers: bool,
    wrap_mode: WrapMode,
    raw_mode: bool,
    max_columns: Option<u16>,
}

struct SearchState {
    input_buf: String,
    search_query: String,
    search_results: Vec<usize>,
    search_idx: Option<usize>,
    search_history: Vec<String>,
    search_history_idx: Option<usize>,
}

struct FileState {
    files: Vec<PathBuf>,
    file_index: usize,
    file_history: Vec<PathBuf>,
    parent_dir: Option<PathBuf>,
    file_name: String,
    last_modified: Option<SystemTime>,
    follow: bool,
}

struct BookmarkState {
    bookmarks: HashMap<char, usize>,
    expecting_bookmark_set: bool,
    expecting_bookmark_jump: bool,
}

impl App {
    /// Create a new `App` with the given files and display options.
    ///
    /// Automatically loads the first file (or stdin content) on construction.
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        files: Vec<PathBuf>,
        follow: bool,
        wrap_mode: WrapMode,
        line_numbers: bool,
        show_status: bool,
        theme: Theme,
        start_line: usize,
        content_from_stdin: Option<String>,
        columns: Option<u16>,
        fail: bool,
        images: bool,
    ) -> Self {
        let no_colour = theme.is_plain();
        let mut app = App {
            rendered: RenderedContent {
                lines: Vec::new(),
                raw_lines: Vec::new(),
                wiki_links: Vec::new(),
                links: Vec::new(),
                content: String::new(),
                content_lower_lines: Vec::new(),
                display_cache: None,
            },
            view: ViewState {
                cursor_line: 0,
                cursor_col: 0,
                viewport_height: 20,
                scroll: 0,
                h_scroll: 0,
                max_scroll: 0,
                show_status,
                show_line_numbers: line_numbers,
                wrap_mode,
                raw_mode: false,
                max_columns: columns,
            },
            search: SearchState {
                input_buf: String::new(),
                search_query: String::new(),
                search_results: Vec::new(),
                search_idx: None,
                search_history: Vec::new(),
                search_history_idx: None,
            },
            file_state: FileState {
                files,
                file_index: 0,
                file_history: Vec::new(),
                parent_dir: None,
                file_name: String::new(),
                last_modified: None,
                follow,
            },
            bm: BookmarkState {
                bookmarks: HashMap::new(),
                expecting_bookmark_set: false,
                expecting_bookmark_jump: false,
            },
            mode: Mode::Normal,
            status_message: None,
            theme,
            no_colour,
            fail,
            pending_count: None,
            image_placements: Vec::new(),
            kitty_supported: false,
            images_enabled: images,
            image_download_cache: HashMap::new(),
        };

        if let Some(stdin_content) = content_from_stdin {
            app.rendered.content = stdin_content;
            app.render_content();
            if start_line > 1 {
                let sl = (start_line - 1).min(app.view.max_scroll);
                app.view.cursor_line = sl;
                app.view.scroll = sl;
            }
        } else if !app.file_state.files.is_empty() {
            app.load_current_file();
            if start_line > 1 {
                let sl = (start_line - 1).min(app.view.max_scroll);
                app.view.cursor_line = sl;
                app.view.scroll = sl;
            }
        }

        app
    }

    fn load_current_file(&mut self) {
        if self.file_state.files.is_empty() {
            return;
        }
        let path = &self.file_state.files[self.file_state.file_index];
        self.file_state.parent_dir = path.parent().map(|p| p.to_path_buf());
        self.rendered.content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                if self.fail {
                    eprintln!("Error: could not read '{}': {}", path.display(), e);
                    std::process::exit(1);
                }
                self.status_message = Some(format!("Error reading file: {}", e));
                String::new()
            }
        };
        self.file_state.last_modified =
            std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
        self.refresh_file_name();
        self.render_content();
    }

    fn reload_current_file(&mut self) {
        self.load_current_file();
        self.search.search_results = if self.search.search_query.is_empty() {
            Vec::new()
        } else {
            self.search_lines_for_mode(&self.search.search_query)
        };
    }

    fn render_content(&mut self) {
        let (styled_lines, raw, wiki_links, links) = if self.no_colour {
            render::render_with_options(&self.rendered.content, &self.theme, true)
        } else {
            render::render(&self.rendered.content, &self.theme)
        };
        self.rendered.lines = styled_lines;
        self.rendered.raw_lines = raw;
        self.rendered.wiki_links = wiki_links;
        self.rendered.links = links;

        // Record image links for Kitty-protocol inline rendering
        if self.images_enabled && !self.no_colour {
            self.expand_inline_images();
        }

        self.rendered.content_lower_lines = self
            .rendered
            .content
            .lines()
            .map(str::to_lowercase)
            .collect();
        self.view.max_scroll = self.rendered.lines.len().saturating_sub(1);
        self.view.scroll = self.view.scroll.min(self.view.max_scroll);
        self.view.cursor_line = self.view.cursor_line.min(self.view.max_scroll);
        self.invalidate_display();
    }

    fn invalidate_display(&mut self) {
        self.rendered.display_cache = None;
    }

    fn search_lines_for_mode(&self, query: &str) -> Vec<usize> {
        if self.view.raw_mode {
            let lines: Vec<String> = self.rendered.content.lines().map(String::from).collect();
            search_lines(&lines, query)
        } else {
            search_lines(&self.rendered.raw_lines, query)
        }
    }

    fn line_width(&self, line: usize) -> u16 {
        if self.view.raw_mode {
            self.rendered
                .content
                .lines()
                .nth(line)
                .map(|l| l.len() as u16)
                .unwrap_or(0)
        } else {
            self.rendered
                .lines
                .get(line)
                .map(|l| {
                    l.spans
                        .iter()
                        .map(|s| s.content.as_ref().len() as u16)
                        .sum()
                })
                .unwrap_or(0)
        }
    }

    fn follow_cursor(&mut self) {
        let vh = self.view.viewport_height.max(1) as usize;
        // Vertical
        if self.view.cursor_line < self.view.scroll {
            self.view.scroll = self.view.cursor_line;
        } else if self.view.cursor_line >= self.view.scroll + vh {
            self.view.scroll = self.view.cursor_line + 1 - vh;
        }
        // Horizontal (only in unwrapped mode)
        if self.view.wrap_mode == WrapMode::None {
            let lw = self.view.show_line_numbers as u16;
            let screen_col = self.view.cursor_col as i16 - self.view.h_scroll as i16;
            if screen_col < lw as i16 {
                self.view.h_scroll = self.view.cursor_col.saturating_sub(lw);
            }
        }
    }

    fn current_file_name(&self) -> &str {
        if !self.file_state.files.is_empty() {
            &self.file_state.file_name
        } else {
            "<stdin>"
        }
    }

    fn refresh_file_name(&mut self) {
        self.file_state.file_name = if !self.file_state.files.is_empty() {
            self.file_state.files[self.file_state.file_index]
                .display()
                .to_string()
        } else {
            String::new()
        };
    }

    fn has_multiple_files(&self) -> bool {
        self.file_state.files.len() > 1
    }

    /// Run the main event loop, drawing frames and handling keyboard input.
    ///
    /// Returns when the user quits (via `q`/`Esc`) or an I/O error occurs.
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        // Detect Kitty graphics protocol support (only if images are enabled)
        if self.images_enabled {
            self.kitty_supported = match viuer::get_kitty_support() {
                viuer::KittySupport::Local | viuer::KittySupport::Remote => true,
                viuer::KittySupport::None => false,
            };
            if self.kitty_supported {
                let _ = write!(io::stdout(), "\x1b_Ga=d\x1b\\");
                let _ = io::stdout().flush();
            }
        }

        loop {
            if self.file_state.follow && !self.file_state.files.is_empty() {
                self.check_file_changed();
            }

            terminal.draw(|f| {
                self.render_frame(f);
            })?;

            // After the frame is drawn and flushed, place inline Kitty images
            if self.kitty_supported && self.images_enabled {
                self.render_inline_images_kitty();
            }

            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match &self.mode {
                    Mode::Normal => self.handle_normal_key(&key),
                    Mode::SearchForward | Mode::SearchBackward => {
                        self.handle_search_key(key.code);
                    }
                    Mode::GoToLine => self.handle_goto_key(key.code),
                    Mode::FileList => self.handle_filelist_key(key.code),
                }
            }
        }
    }

    fn check_file_changed(&mut self) {
        if self.file_state.files.is_empty() {
            return;
        }
        let path = &self.file_state.files[self.file_state.file_index];
        if let Ok(modified) = std::fs::metadata(path).and_then(|m| m.modified())
            && self.file_state.last_modified != Some(modified)
        {
            self.file_state.last_modified = Some(modified);
            self.reload_current_file();
        }
    }

    fn render_frame(&mut self, f: &mut ratatui::Frame) {
        let area = f.area();

        let areas = if self.view.show_status {
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area)
        } else {
            Layout::vertical([Constraint::Min(1)]).split(area)
        };

        let content_area = areas[0];
        self.view.viewport_height = content_area.height;

        let content_area = if let Some(max_cols) = self.view.max_columns {
            let w = content_area.width.min(max_cols);
            let x = content_area.x + (content_area.width.saturating_sub(w)) / 2;
            Rect { x, width: w, ..content_area }
        } else {
            content_area
        };

        let display_lines: Vec<Line<'static>> = if self.mode == Mode::FileList {
            self.build_file_list()
        } else if self.view.raw_mode {
            self.build_raw_lines()
        } else {
            if self.rendered.display_cache.is_none() {
                let lines = self.build_styled_lines();
                self.rendered.display_cache = Some(lines);
            }
            self.rendered.display_cache.as_ref().unwrap().clone()
        };

        let has_table = display_lines.iter().any(|l| {
            l.spans.iter().any(|s| {
                s.content.contains('┌')
                    || s.content.contains('│')
                    || s.content.contains('└')
                    || s.content.contains('├')
            })
        });

        let mut paragraph =
            Paragraph::new(display_lines).scroll((self.view.scroll as u16, self.view.h_scroll));
        if has_table {
            // disable wrapping when tables are visible
        } else if self.view.wrap_mode == WrapMode::Word {
            paragraph = paragraph.wrap(Wrap { trim: false });
        } else if self.view.wrap_mode == WrapMode::Char {
            paragraph = paragraph.wrap(Wrap { trim: true });
        }
        f.render_widget(paragraph, content_area);

        // Draw cursor
        if self.mode == Mode::Normal || self.mode == Mode::FileList {
            let total_lines = if self.view.raw_mode {
                self.rendered.content.lines().count()
            } else {
                self.rendered.lines.len()
            };
            let line_num_width = if self.view.show_line_numbers {
                total_lines.to_string().len() + 1
            } else {
                0
            } as u16;
            let screen_y =
                content_area.y + self.view.cursor_line.saturating_sub(self.view.scroll) as u16;
            let screen_x = content_area.x
                + line_num_width
                + self.view.cursor_col.saturating_sub(self.view.h_scroll);
            if screen_y < content_area.bottom()
                && screen_x < content_area.right()
                && let Some(cell) = f.buffer_mut().cell_mut((screen_x, screen_y))
            {
                std::mem::swap(&mut cell.fg, &mut cell.bg);
            }
        }

        if self.view.show_status {
            let status = self.build_status_bar();
            f.render_widget(status, areas[1]);
        }
    }

    fn build_styled_lines(&self) -> Vec<Line<'static>> {
        self.rendered
            .lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line = if !self.search.search_query.is_empty() {
                    highlight_line(line, &self.search.search_query)
                } else {
                    line.clone()
                };
                if self.view.show_line_numbers {
                    prepend_line_number(line, i + 1, self.rendered.lines.len())
                } else {
                    line
                }
            })
            .collect()
    }

    #[expect(clippy::manual_checked_ops)]
    fn build_status_bar(&self) -> Paragraph<'static> {
        let file_name = self.current_file_name();
        let total = if self.view.raw_mode {
            self.rendered.content.lines().count()
        } else {
            self.rendered.lines.len()
        };
        let ln = (self.view.cursor_line + 1).min(total);
        let pct = if total == 0 {
            0
        } else {
            (self.view.cursor_line * 100 / total).min(100)
        };

        let mut parts = vec![
            format!(" {} — Ln {}/{} ({}%) ", file_name, ln, total, pct),
            format!(" wrap:{} ", self.view.wrap_mode.as_str()),
        ];

        if self.view.raw_mode {
            parts.push(" RAW ".to_string());
        }

        if self.has_multiple_files() {
            parts.push(format!(
                " file {}/{} ",
                self.file_state.file_index + 1,
                self.file_state.files.len()
            ));
        }

        if !self.search.search_query.is_empty() {
            let match_info = match self.search.search_idx {
                Some(idx) => {
                    if let Some(pos) = self.search.search_results.iter().position(|&r| r == idx) {
                        format!(
                            " \"{}\" [{}/{}] ",
                            self.search.search_query,
                            pos + 1,
                            self.search.search_results.len()
                        )
                    } else {
                        format!(
                            " \"{}\" [0/{}] ",
                            self.search.search_query,
                            self.search.search_results.len()
                        )
                    }
                }
                None => format!(
                    " \"{}\" [0/{}] ",
                    self.search.search_query,
                    self.search.search_results.len()
                ),
            };
            parts.push(match_info);
        }

        if self.view.show_line_numbers {
            parts.push(" # ".to_string());
        }

        let status_text = parts.concat();

        let mut left_spans = vec![Span::styled(
            status_text,
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )];

        if let Some(msg) = &self.status_message {
            left_spans.push(Span::styled(
                format!(" ⚠ {} ", msg),
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        match &self.mode {
            Mode::SearchForward => {
                let prompt = format!(" /{}_ ", self.search.input_buf);
                left_spans.push(Span::styled(
                    prompt,
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Mode::SearchBackward => {
                let prompt = format!(" ?{}_ ", self.search.input_buf);
                left_spans.push(Span::styled(
                    prompt,
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Mode::GoToLine => {
                let prompt = format!(" :{}_ ", self.search.input_buf);
                left_spans.push(Span::styled(
                    prompt,
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Mode::FileList => {}
            Mode::Normal => {}
        }

        Paragraph::new(Line::from(left_spans))
    }

    fn handle_normal_key(&mut self, key: &KeyEvent) {
        let code = key.code;
        self.status_message = None;

        if self.bm.expecting_bookmark_set {
            self.bm.expecting_bookmark_set = false;
            if let KeyCode::Char(c) = code
                && c.is_ascii_lowercase()
            {
                self.bm.bookmarks.insert(c, self.view.cursor_line);
            }
            return;
        }
        if self.bm.expecting_bookmark_jump {
            self.bm.expecting_bookmark_jump = false;
            if let KeyCode::Char(c) = code
                && c.is_ascii_lowercase()
                && let Some(&pos) = self.bm.bookmarks.get(&c)
            {
                self.view.cursor_line = pos.min(self.view.max_scroll);
                self.follow_cursor();
            }
            return;
        }

        match code {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if c == '0' && self.pending_count.is_none() {
                    self.view.cursor_col = 0;
                } else {
                    let d = c.to_digit(10).expect("char is ascii digit") as usize;
                    self.pending_count = Some(self.pending_count.unwrap_or(0) * 10 + d);
                }
                return;
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                std::process::exit(0);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let count = self.pending_count.take().unwrap_or(1);
                self.view.cursor_line = self.view.cursor_line.saturating_sub(count);
                self.follow_cursor();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let count = self.pending_count.take().unwrap_or(1);
                self.view.cursor_line = self
                    .view
                    .cursor_line
                    .saturating_add(count)
                    .min(self.view.max_scroll);
                self.follow_cursor();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let count = self.pending_count.take().unwrap_or(1) as u16;
                self.view.cursor_col = self.view.cursor_col.saturating_sub(count);
                self.follow_cursor();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let count = self.pending_count.take().unwrap_or(1) as u16;
                let max_col = self.line_width(self.view.cursor_line);
                self.view.cursor_col = self.view.cursor_col.saturating_add(count).min(max_col);
                self.follow_cursor();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let count = self.pending_count.take().unwrap_or(1);
                let half = (self.view.viewport_height as usize / 2).max(1);
                self.view.cursor_line = self.view.cursor_line.saturating_sub(half * count);
                self.follow_cursor();
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let count = self.pending_count.take().unwrap_or(1);
                let half = (self.view.viewport_height as usize / 2).max(1);
                self.view.cursor_line = self
                    .view
                    .cursor_line
                    .saturating_add(half * count)
                    .min(self.view.max_scroll);
                self.follow_cursor();
            }
            KeyCode::PageUp | KeyCode::Char('b') => {
                let count = self.pending_count.take().unwrap_or(1);
                self.view.cursor_line = self
                    .view
                    .cursor_line
                    .saturating_sub(self.view.viewport_height as usize * count);
                self.follow_cursor();
            }
            KeyCode::PageDown | KeyCode::Char('f') => {
                let count = self.pending_count.take().unwrap_or(1);
                self.view.cursor_line = self
                    .view
                    .cursor_line
                    .saturating_add(self.view.viewport_height as usize * count)
                    .min(self.view.max_scroll);
                self.follow_cursor();
            }
            KeyCode::Home | KeyCode::Char('g') => {
                let target = self
                    .pending_count
                    .take()
                    .map(|c| c.saturating_sub(1))
                    .unwrap_or(0);
                self.view.cursor_line = target.min(self.view.max_scroll);
                self.view.cursor_col = 0;
                self.view.h_scroll = 0;
                self.view.scroll = target.min(self.view.max_scroll);
            }
            KeyCode::End | KeyCode::Char('G') => {
                let target = self
                    .pending_count
                    .take()
                    .map(|c| c.saturating_sub(1))
                    .unwrap_or(self.view.max_scroll);
                self.view.cursor_line = target.min(self.view.max_scroll);
                self.view.cursor_col = 0;
                self.follow_cursor();
            }
            KeyCode::Char('/') => {
                self.mode = Mode::SearchForward;
                self.search.input_buf.clear();
            }
            KeyCode::Char('?') => {
                self.mode = Mode::SearchBackward;
                self.search.input_buf.clear();
            }
            KeyCode::Char(':') => {
                self.mode = Mode::GoToLine;
                self.search.input_buf.clear();
            }
            KeyCode::Char('n') => {
                let count = self.pending_count.take().unwrap_or(1);
                if !self.search.search_results.is_empty() {
                    for _ in 0..count {
                        self.search.search_idx =
                            find_next_match(&self.search.search_results, self.search.search_idx);
                    }
                    if let Some(idx) = self.search.search_idx {
                        self.view.cursor_line = idx.min(self.view.max_scroll);
                        self.follow_cursor();
                    }
                }
            }
            KeyCode::Char('N') => {
                let count = self.pending_count.take().unwrap_or(1);
                if !self.search.search_results.is_empty() {
                    for _ in 0..count {
                        self.search.search_idx =
                            find_prev_match(&self.search.search_results, self.search.search_idx);
                    }
                    if let Some(idx) = self.search.search_idx {
                        self.view.cursor_line = idx.min(self.view.max_scroll);
                        self.follow_cursor();
                    }
                }
            }
            KeyCode::Char('r') => {
                self.view.raw_mode = !self.view.raw_mode;
                if self.view.raw_mode {
                    let line_count = self.rendered.content.lines().count().saturating_sub(1);
                    self.view.max_scroll = line_count;
                } else {
                    self.view.max_scroll = self.rendered.lines.len().saturating_sub(1);
                }
                self.search.search_query.clear();
                self.search.search_results.clear();
                self.search.search_idx = None;
                self.view.cursor_line = self.view.cursor_line.min(self.view.max_scroll);
                self.follow_cursor();
            }
            KeyCode::Char('R') if !self.file_state.files.is_empty() => {
                self.reload_current_file();
            }
            KeyCode::Char(']') if self.has_multiple_files() => {
                self.file_state.file_index =
                    (self.file_state.file_index + 1) % self.file_state.files.len();
                self.view.cursor_line = 0;
                self.view.cursor_col = 0;
                self.view.scroll = 0;
                self.view.h_scroll = 0;
                self.search.search_idx = None;
                self.load_current_file();
            }
            KeyCode::Char('[') if self.has_multiple_files() => {
                self.file_state.file_index = if self.file_state.file_index == 0 {
                    self.file_state.files.len() - 1
                } else {
                    self.file_state.file_index - 1
                };
                self.view.cursor_line = 0;
                self.view.cursor_col = 0;
                self.view.scroll = 0;
                self.view.h_scroll = 0;
                self.search.search_idx = None;
                self.load_current_file();
            }
            KeyCode::Char('m') => {
                self.bm.expecting_bookmark_set = true;
            }
            KeyCode::Char('\'') => {
                self.bm.expecting_bookmark_jump = true;
            }
            KeyCode::Enter => {
                self.follow_wiki_link();
            }
            KeyCode::Backspace => {
                self.navigate_back();
            }
            _ => {}
        }
        self.pending_count = None;
    }

    fn navigate_back(&mut self) {
        self.view.raw_mode = false;
        let prev = match self.file_state.file_history.pop() {
            Some(p) => p,
            None => {
                self.status_message = Some("No previous file in history".to_string());
                return;
            }
        };
        if !self.file_state.files.is_empty() {
            self.file_state.files[self.file_state.file_index] = prev;
        } else {
            self.file_state.files.push(prev);
            self.file_state.file_index = 0;
        }
        self.view.cursor_line = 0;
        self.view.cursor_col = 0;
        self.view.scroll = 0;
        self.view.h_scroll = 0;
        self.search.search_idx = None;
        self.search.search_query.clear();
        self.search.search_results.clear();
        self.load_current_file();
        self.status_message = None;
    }

/// Resolve a local image URL to an absolute path.
    fn resolve_image_path(&self, url: &str) -> Option<std::path::PathBuf> {
        if url.starts_with("http://") || url.starts_with("https://") {
            return None;
        }
        let path = std::path::Path::new(url);
        if path.is_absolute() {
            if path.exists() { Some(path.to_path_buf()) } else { None }
        } else if let Some(parent) = &self.file_state.parent_dir {
            let candidate = parent.join(url);
            if candidate.exists() { Some(candidate) } else { None }
        } else {
            if path.exists() { Some(path.to_path_buf()) } else { None }
        }
    }

    /// Download a remote image to a temp file and cache it.
    fn fetch_remote_image(&mut self, url: &str) -> Option<std::path::PathBuf> {
        if self.image_download_cache.contains_key(url) {
            return self.image_download_cache.get(url).cloned();
        }

        let response = ureq::get(url).call().ok()?;

        let mut body = response.into_body();
        let bytes: Vec<u8> = body.read_to_vec().ok()?;

        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();
        let ext = std::path::Path::new(url)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let temp_dir = std::env::temp_dir().join("mkdr");
        let _ = std::fs::create_dir_all(&temp_dir);
        let path = temp_dir.join(format!("img-{hash:x}.{ext}"));

        if std::fs::write(&path, &bytes).is_err() {
            return None;
        }

        self.image_download_cache
            .insert(url.to_string(), path.clone());
        Some(path)
    }

    /// Render inline images using the Kitty graphics protocol.
    /// Called after each frame draw — places pixel-perfect images at the correct
    /// cell positions by first moving the cursor, then printing with viuer.
    fn render_inline_images_kitty(&mut self) {
        if self.image_placements.is_empty() {
            return;
        }

        // Clear all previous Kitty placements from the screen
        let _ = write!(io::stdout(), "\x1b_Ga=d\x1b\\");

        for placement in &self.image_placements {
            if placement.line < self.view.scroll {
                continue;
            }
            let screen_row = placement.line - self.view.scroll;
            if screen_row >= self.view.viewport_height as usize {
                continue;
            }

            let img = match image::ImageReader::open(&placement.path) {
                Ok(r) => match r.decode() {
                    Ok(i) => i,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            // Move cursor to cell (1, screen_row+1) then let viuer print there
            let _ = write!(io::stdout(), "\x1b[{};{}H", screen_row + 1, 1);
            let config = viuer::Config {
                width: Some(placement.disp_cols),
                height: Some(placement.disp_rows),
                x: 0,
                y: 0,
                absolute_offset: false,
                use_kitty: true,
                ..Default::default()
            };
            let _ = viuer::print(&img, &config);
        }
        let _ = io::stdout().flush();
    }

    /// Find image links in rendered output, record placements, and insert blank
    /// placeholder lines so the image's height is reserved in the text layout.
    fn expand_inline_images(&mut self) {
        self.image_placements.clear();
        if self.view.raw_mode {
            return;
        }

        let term_rows = crossterm::terminal::size()
            .map(|s| s.1 as u32)
            .unwrap_or(24);
        let bound_h = (term_rows / 2).clamp(4, 30);

        let cols = crossterm::terminal::size()
            .map(|s| s.0 as u32)
            .unwrap_or(80);
        let bound_w = cols.saturating_sub(1).max(10);

        // Collect image link URLs and their line numbers from rendered links
        let mut image_urls: Vec<(usize, String)> = Vec::new();
        for (li, items) in self.rendered.links.iter().enumerate().rev() {
            for item in items {
                if item.kind == crate::render::LinkKind::Image {
                    image_urls.push((li, item.url.clone()));
                    break;
                }
            }
        }

        // Collect placements in reverse order so splicing doesn't invalidate indices
        let mut placements: Vec<(usize, Vec<Line<'static>>)> = Vec::new();

        for (li, url) in &image_urls {
            if let Some(path) = self.resolve_image_path(url)
                .or_else(|| self.fetch_remote_image(url))
            {
                if let Ok(r) = image::ImageReader::open(&path) {
                    if let Ok(img) = r.decode() {
                        let (iw, ih) = (img.width(), img.height());

                        let eff_bound_h = bound_h * 2;
                        let scale = (bound_w as f32 / iw.max(1) as f32)
                            .min(eff_bound_h as f32 / ih.max(1) as f32);
                        let disp_w = (iw as f32 * scale).round() as u32;
                        let disp_h_px = (ih as f32 * scale).round() as u32;
                        let disp_rows = (disp_h_px + 1).saturating_div(2).max(1);

                        self.image_placements.push(ImagePlacement {
                            path,
                            line: *li,
                            disp_cols: disp_w,
                            disp_rows,
                        });

                        let blank = std::iter::repeat(Line::from(Span::raw(" ")))
                            .take(disp_rows as usize);
                        placements.push((*li, blank.collect()));
                    }
                }
            }
        }

        // Splice blanks in reverse order (already sorted by .rev() above)
        for (li, blanks) in placements {
            let count = blanks.len();
            self.rendered.lines.splice(li..=li, blanks);
            self.rendered.raw_lines.splice(
                li..=li,
                std::iter::repeat(String::new()).take(count),
            );
            self.rendered.wiki_links.splice(
                li..=li,
                std::iter::repeat(Vec::new()).take(count),
            );
            self.rendered.links.splice(
                li..=li,
                std::iter::repeat(Vec::new()).take(count),
            );
        }
    }

    fn follow_wiki_link(&mut self) {
        let line = self.view.cursor_line;

        // Check for external links (web / image) on this line
        if let Some(item) = self.rendered.links.get(line).and_then(|items| {
            items
                .iter()
                .rfind(|i| i.col <= self.view.cursor_col as usize)
        }) {
            match item.kind {
                crate::render::LinkKind::Web => {
                    if open::that(&item.url).is_ok() {
                        self.status_message = Some("Opened in browser".to_string());
                    } else {
                        self.status_message = Some(format!("Failed to open: {}", item.url));
                    }
                    return;
                }
                crate::render::LinkKind::Image => {
                    // Images are rendered inline via Kitty protocol.
                    // Enter to open full-size in system viewer.
                    if open::that(&item.url).is_ok() {
                        self.status_message = Some("Opened in image viewer".to_string());
                    } else {
                        self.status_message = Some(format!("Failed to open: {}", item.url));
                    }
                    return;
                }
            }
        }

        // Fall back to wiki link navigation
        let links = match self.rendered.wiki_links.get(line) {
            Some(l) if !l.is_empty() => l,
            _ => {
                self.status_message = Some("No link on this line".to_string());
                return;
            }
        };
        let link = links
            .iter()
            .rfind(|l| l.col <= self.view.cursor_col as usize)
            .unwrap_or(&links[0]);
        let target_path = if let Some(parent) = &self.file_state.parent_dir {
            let p = parent.join(&link.target);
            if p.exists() {
                p
            } else if p.extension().is_none() {
                let with_md = parent.join(format!("{}.md", &link.target));
                if with_md.exists() { with_md } else { p }
            } else {
                p
            }
        } else {
            let p = PathBuf::from(&link.target);
            if p.exists() {
                p
            } else if p.extension().is_none() {
                let with_md = PathBuf::from(format!("{}.md", &link.target));
                if with_md.exists() { with_md } else { p }
            } else {
                p
            }
        };
        if !target_path.exists() {
            self.status_message = Some(format!("File not found: {}", target_path.display()));
            return;
        }
        // Save current file to history before navigating
        if !self.file_state.files.is_empty() {
            self.file_state
                .file_history
                .push(self.file_state.files[self.file_state.file_index].clone());
        }
        // Replace current file with the wiki link target
        if !self.file_state.files.is_empty() {
            self.file_state.files[self.file_state.file_index] = target_path;
        } else {
            self.file_state.files.push(target_path);
            self.file_state.file_index = 0;
        }
        self.view.cursor_line = 0;
        self.view.cursor_col = 0;
        self.view.scroll = 0;
        self.view.h_scroll = 0;
        self.search.search_idx = None;
        self.search.search_query.clear();
        self.search.search_results.clear();
        self.load_current_file();
    }

    fn handle_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                let query = std::mem::take(&mut self.search.input_buf);
                let forward = matches!(self.mode, Mode::SearchForward);
                self.search.search_query = query.clone();
                self.invalidate_display();
                if !query.is_empty() {
                    if self.search.search_history.last() != Some(&query) {
                        self.search.search_history.push(query.clone());
                        if self.search.search_history.len() > 50 {
                            self.search.search_history.remove(0);
                        }
                    }
                    self.search.search_history_idx = None;
                    self.search.search_results = self.search_lines_for_mode(&query);
                    self.search.search_idx = if forward {
                        find_next_match(&self.search.search_results, Some(self.view.scroll))
                    } else {
                        find_prev_match(&self.search.search_results, Some(self.view.scroll))
                    };
                    if let Some(idx) = self.search.search_idx {
                        self.view.scroll = idx.min(self.view.max_scroll);
                    }
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Up => {
                let idx = self
                    .search
                    .search_history_idx
                    .get_or_insert(self.search.search_history.len());
                if *idx > 0 {
                    *idx -= 1;
                    self.search.input_buf = self.search.search_history[*idx].clone();
                    self.search.search_query = self.search.input_buf.clone();
                    self.invalidate_display();
                    self.search.search_results =
                        self.search_lines_for_mode(&self.search.search_query);
                    self.search.search_idx = None;
                }
            }
            KeyCode::Down => {
                if let Some(idx) = &mut self.search.search_history_idx {
                    if *idx + 1 < self.search.search_history.len() {
                        *idx += 1;
                        self.search.input_buf = self.search.search_history[*idx].clone();
                    } else {
                        self.search.search_history_idx = None;
                        self.search.input_buf.clear();
                    }
                    self.search.search_query = self.search.input_buf.clone();
                    self.invalidate_display();
                    self.search.search_results =
                        self.search_lines_for_mode(&self.search.search_query);
                    self.search.search_idx = None;
                }
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.search.input_buf.clear();
            }
            KeyCode::Backspace => {
                self.search.input_buf.pop();
                self.search.search_query = self.search.input_buf.clone();
                self.invalidate_display();
                if !self.search.search_query.is_empty() {
                    self.search.search_results =
                        self.search_lines_for_mode(&self.search.search_query);
                    self.search.search_idx = None;
                } else {
                    self.search.search_results.clear();
                    self.search.search_idx = None;
                }
            }
            KeyCode::Char(c) => {
                self.search.input_buf.push(c);
                self.search.search_query = self.search.input_buf.clone();
                self.invalidate_display();
                if !self.search.search_query.is_empty() {
                    self.search.search_results =
                        self.search_lines_for_mode(&self.search.search_query);
                    self.search.search_idx = None;
                }
            }
            _ => {}
        }
    }

    fn handle_goto_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.search.input_buf);
                let trimmed = input.trim();
                if trimmed.starts_with("theme ") {
                    let name = trimmed.trim_start_matches("theme ").trim();
                    self.switch_theme(name);
                } else if trimmed == "reload" {
                    self.reload_config();
                } else if trimmed == "files" && self.has_multiple_files() {
                    self.mode = Mode::FileList;
                    return;
                } else if let Some(pct) = trimmed.strip_suffix('%')
                    && let Ok(pct_val) = pct.parse::<usize>()
                {
                    let line = (pct_val * self.view.max_scroll / 100).min(self.view.max_scroll);
                    self.view.cursor_line = line;
                    self.view.scroll = line;
                } else if let Ok(line_num) = trimmed.parse::<usize>() {
                    let line = line_num.saturating_sub(1).min(self.view.max_scroll);
                    self.view.cursor_line = line;
                    self.view.scroll = line;
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.search.input_buf.clear();
            }
            KeyCode::Backspace => {
                self.search.input_buf.pop();
            }
            KeyCode::Char(c) if !c.is_control() => {
                self.search.input_buf.push(c);
            }
            _ => {}
        }
    }

    fn switch_theme(&mut self, name: &str) {
        let theme = match name {
            "dark" => Theme::default_dark(),
            "light" => Theme::default_light(),
            name => Theme::load(name).unwrap_or_else(|| {
                eprintln!("Warning: theme '{}' not found, using default dark", name);
                Theme::default_dark()
            }),
        };
        self.theme = theme;
        self.render_content();
    }

    fn reload_config(&mut self) {
        let config = crate::config::load_config();
        if let Some(name) = config.theme {
            self.switch_theme(&name);
        } else {
            self.render_content();
        }
        if let Some(wrap) = config.wrap {
            self.view.wrap_mode = WrapMode::from_str(&wrap);
        }
    }

    fn build_file_list(&self) -> Vec<Line<'static>> {
        let mut result = Vec::new();
        result.push(Line::from(Span::styled(
            " Files ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )));
        result.push(Line::from(Span::styled(
            "───",
            Style::default().fg(Color::DarkGray),
        )));
        for (i, path) in self.file_state.files.iter().enumerate() {
            let marker = if i == self.file_state.file_index {
                "▸ "
            } else {
                "  "
            };
            let fg = if i == self.file_state.file_index {
                Color::Cyan
            } else {
                Color::White
            };
            let bold = if i == self.file_state.file_index {
                Modifier::BOLD
            } else {
                Modifier::empty()
            };
            result.push(Line::from(Span::styled(
                format!("{}{}", marker, path.display()),
                Style::default().fg(fg).add_modifier(bold),
            )));
        }
        result
    }

    fn build_raw_lines(&self) -> Vec<Line<'static>> {
        let para_style = self.theme.style_as_style("paragraph").unwrap_or_default();
        let frontmatter_style = para_style.add_modifier(Modifier::DIM);
        let code_style = self.theme.style_as_style("code_block").unwrap_or_default();
        let heading_styles: Vec<Style> = (0..6)
            .map(|i| {
                let key = format!("heading{}", i + 1);
                self.theme.style_as_style(&key).unwrap_or(para_style)
            })
            .collect();

        let content_lines: Vec<&str> = self.rendered.content.lines().collect();
        let total_lines = content_lines.len();
        let query_lower = self.search.search_query.to_lowercase();
        let has_search = !self.search.search_query.is_empty();

        // Check if content starts with frontmatter
        let first_non_empty = content_lines.iter().find(|l| !l.trim().is_empty()).copied();
        let has_frontmatter = first_non_empty.is_some_and(|l| l.trim() == "---");

        let mut in_frontmatter = false;
        let mut in_fence = false;
        let mut result: Vec<Line<'static>> = Vec::with_capacity(total_lines);

        for (i, line) in content_lines.iter().enumerate() {
            let trimmed = line.trim_start();
            let style: Style;

            if has_frontmatter && !in_frontmatter && trimmed == "---" {
                in_frontmatter = true;
                style = frontmatter_style;
            } else if in_frontmatter && trimmed == "---" {
                in_frontmatter = false;
                style = frontmatter_style;
            } else if in_frontmatter {
                style = frontmatter_style;
            } else if trimmed.starts_with("```") {
                in_fence = !in_fence;
                style = code_style;
            } else if in_fence {
                style = code_style;
            } else if let Some(level) = trimmed.chars().position(|c| c != '#').filter(|&pos| {
                pos > 0
                    && pos <= 6
                    && trimmed
                        .as_bytes()
                        .get(pos)
                        .is_some_and(|&b| b == b' ' || pos == trimmed.len())
            }) {
                style = heading_styles[level - 1];
            } else {
                style = para_style;
            }

            let mut spans = vec![Span::styled((*line).to_string(), style)];

            if has_search && self.rendered.content_lower_lines[i].contains(&query_lower) {
                let line_obj = Line::from(spans);
                let highlighted = highlight_line(&line_obj, &self.search.search_query);
                spans = highlighted.spans;
            }

            let mut line_obj = Line::from(spans);

            if self.view.show_line_numbers {
                line_obj = prepend_line_number(line_obj, i + 1, total_lines);
            }

            result.push(line_obj);
        }

        result
    }

    fn handle_filelist_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up | KeyCode::Char('k') if self.file_state.file_index > 0 => {
                self.file_state.file_index -= 1;
            }
            KeyCode::Down | KeyCode::Char('j')
                if self.file_state.file_index + 1 < self.file_state.files.len() =>
            {
                self.file_state.file_index += 1;
            }
            KeyCode::Enter => {
                self.view.scroll = 0;
                self.view.h_scroll = 0;
                self.search.search_idx = None;
                self.load_current_file();
                self.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }
}

fn prepend_line_number(mut line: Line<'static>, num: usize, total: usize) -> Line<'static> {
    let digits = total.to_string().len();
    let num_str = format!("{:>width$} ", num, width = digits);
    let num_span = Span::styled(num_str, Style::default().fg(Color::DarkGray));
    line.spans.insert(0, num_span);
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_mode_from_str_word() {
        assert_eq!(WrapMode::from_str("word"), WrapMode::Word);
    }

    #[test]
    fn wrap_mode_from_str_none() {
        assert_eq!(WrapMode::from_str("none"), WrapMode::None);
    }

    #[test]
    fn wrap_mode_from_str_char() {
        assert_eq!(WrapMode::from_str("char"), WrapMode::Char);
    }

    #[test]
    fn wrap_mode_from_str_unknown_defaults_to_word() {
        assert_eq!(WrapMode::from_str("whatever"), WrapMode::Word);
    }

    #[test]
    fn wrap_mode_as_str_roundtrip() {
        for mode in &[WrapMode::Word, WrapMode::None, WrapMode::Char] {
            assert_eq!(WrapMode::from_str(mode.as_str()), *mode);
        }
    }

    #[test]
    fn prepend_line_number_adds_span() {
        let line = Line::from(Span::raw("content"));
        let result = prepend_line_number(line, 5, 100);
        assert_eq!(result.spans.len(), 2);
        assert!(result.spans[0].content.contains('5'));
    }

    #[test]
    fn prepend_line_number_pads_width() {
        let line = Line::from(Span::raw("x"));
        let result = prepend_line_number(line, 7, 1000);
        let num_text = result.spans[0].content.as_ref();
        assert_eq!(num_text.len(), 5); // 4 digits + space
        assert_eq!(num_text, "   7 ");
    }
}
