use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Terminal;

use crate::search::{find_next_match, find_prev_match, highlight_line, search_lines};
use crate::theme::Theme;
use crate::render::{self, WikiLink};

#[derive(Clone, Copy, PartialEq)]
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

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    SearchForward,
    SearchBackward,
    GoToLine,
    FileList,
}

pub struct App {
    lines: Vec<Line<'static>>,
    raw_lines: Vec<String>,
    wiki_links: Vec<Vec<WikiLink>>,
    parent_dir: Option<PathBuf>,
    status_message: Option<String>,
    theme: Theme,
    cursor_line: usize,
    cursor_col: u16,
    viewport_height: u16,
    scroll: usize,
    h_scroll: u16,
    max_scroll: usize,
    file_index: usize,
    files: Vec<PathBuf>,
    mode: Mode,
    input_buf: String,
    search_query: String,
    search_results: Vec<usize>,
    search_idx: Option<usize>,
    search_history: Vec<String>,
    search_history_idx: Option<usize>,
    bookmarks: HashMap<char, usize>,
    expecting_bookmark_set: bool,
    expecting_bookmark_jump: bool,
    show_status: bool,
    show_line_numbers: bool,
    wrap_mode: WrapMode,
    follow: bool,
    last_modified: Option<SystemTime>,
    content: String,
    file_history: Vec<PathBuf>,
    raw_mode: bool,
    pending_count: Option<usize>,
}

impl App {
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
    ) -> Self {
        let mut app = App {
            lines: Vec::new(),
            raw_lines: Vec::new(),
            wiki_links: Vec::new(),
            parent_dir: None,
            status_message: None,
            cursor_line: 0,
            cursor_col: 0,
            viewport_height: 20,
            theme,
            scroll: 0,
            h_scroll: 0,
            max_scroll: 0,
            file_index: 0,
            files,
            mode: Mode::Normal,
            input_buf: String::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_idx: None,
            search_history: Vec::new(),
            search_history_idx: None,
            bookmarks: HashMap::new(),
            expecting_bookmark_set: false,
            expecting_bookmark_jump: false,
            show_status,
            show_line_numbers: line_numbers,
            wrap_mode,
            follow,
            last_modified: None,
            content: String::new(),
            file_history: Vec::new(),
            raw_mode: false,
            pending_count: None,
        };

        if let Some(stdin_content) = content_from_stdin {
            app.content = stdin_content;
            app.render_content();
            if start_line > 1 {
                let sl = (start_line - 1).min(app.max_scroll);
                app.cursor_line = sl;
                app.scroll = sl;
            }
        } else if !app.files.is_empty() {
            app.load_current_file();
            if start_line > 1 {
                let sl = (start_line - 1).min(app.max_scroll);
                app.cursor_line = sl;
                app.scroll = sl;
            }
        }

        app
    }

    fn load_current_file(&mut self) {
        if self.files.is_empty() {
            return;
        }
        let path = &self.files[self.file_index];
        self.parent_dir = path.parent().map(|p| p.to_path_buf());
        self.content = std::fs::read_to_string(path).unwrap_or_default();
        self.last_modified = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
        self.render_content();
    }

    fn reload_current_file(&mut self) {
        self.load_current_file();
        self.search_results = if self.search_query.is_empty() {
            Vec::new()
        } else {
            self.search_lines_for_mode( &self.search_query)
        };
    }

    fn render_content(&mut self) {
        let (styled_lines, raw, wiki_links) = render::render(&self.content, &self.theme);
        self.lines = styled_lines;
        self.raw_lines = raw;
        self.wiki_links = wiki_links;
        self.max_scroll = self.lines.len().saturating_sub(1);
        self.scroll = self.scroll.min(self.max_scroll);
        self.cursor_line = self.cursor_line.min(self.max_scroll);
    }

    fn search_lines_for_mode(&self, query: &str) -> Vec<usize> {
        if self.raw_mode {
            let lines: Vec<String> = self.content.lines().map(String::from).collect();
            search_lines(&lines, query)
        } else {
            search_lines(&self.raw_lines, query)
        }
    }

    fn line_width(&self, line: usize) -> u16 {
        if self.raw_mode {
            self.content
                .lines()
                .nth(line)
                .map(|l| l.len() as u16)
                .unwrap_or(0)
        } else {
            self.lines
                .get(line)
                .map(|l| l.spans.iter().map(|s| s.content.as_ref().len() as u16).sum())
                .unwrap_or(0)
        }
    }

    fn follow_cursor(&mut self) {
        let vh = self.viewport_height.max(1) as usize;
        // Vertical
        if self.cursor_line < self.scroll {
            self.scroll = self.cursor_line;
        } else if self.cursor_line >= self.scroll + vh {
            self.scroll = self.cursor_line + 1 - vh;
        }
        // Horizontal (only in unwrapped mode)
        if self.wrap_mode == WrapMode::None {
            let lw = self.show_line_numbers as u16;
            let screen_col = self.cursor_col as i16 - self.h_scroll as i16;
            if screen_col < lw as i16 {
                self.h_scroll = self.cursor_col.saturating_sub(lw);
            }
        }
    }

    fn current_file_name(&self) -> &str {
        if !self.files.is_empty() {
            let name = self.files[self.file_index].display().to_string();
            // Leak for 'static — fine since the app runs for the whole session
            Box::leak(name.into_boxed_str())
        } else {
            "<stdin>"
        }
    }

    fn has_multiple_files(&self) -> bool {
        self.files.len() > 1
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        loop {
            if self.follow && !self.files.is_empty() {
                self.check_file_changed();
            }

            terminal.draw(|f| {
                self.render_frame(f);
            })?;

            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match &self.mode {
                    Mode::Normal => self.handle_normal_key(key.code),
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
        if self.files.is_empty() {
            return;
        }
        let path = &self.files[self.file_index];
        if let Ok(modified) = std::fs::metadata(path).and_then(|m| m.modified())
            && self.last_modified != Some(modified)
        {
            self.last_modified = Some(modified);
            self.reload_current_file();
        }
    }

    fn render_frame(&mut self, f: &mut ratatui::Frame) {
        let area = f.area();

        let areas = if self.show_status {
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area)
        } else {
            Layout::vertical([Constraint::Min(1)]).split(area)
        };

        let content_area = areas[0];
        self.viewport_height = content_area.height;

        let display_lines: Vec<Line<'static>> = if self.mode == Mode::FileList {
            self.build_file_list()
        } else if self.raw_mode {
            self.build_raw_lines()
        } else {
            self.lines
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    let line = if !self.search_query.is_empty() {
                        highlight_line(line, &self.search_query)
                    } else {
                        line.clone()
                    };
                    if self.show_line_numbers {
                        prepend_line_number(line, i + 1, self.lines.len())
                    } else {
                        line
                    }
                })
                .collect()
        };

        let has_table = display_lines.iter().any(|l| {
            l.spans.iter().any(|s| {
                s.content.contains('┌')
                    || s.content.contains('│')
                    || s.content.contains('└')
                    || s.content.contains('├')
            })
        });

        let mut paragraph = Paragraph::new(display_lines).scroll((self.scroll as u16, self.h_scroll));
        if has_table {
            // disable wrapping when tables are visible
        } else if self.wrap_mode == WrapMode::Word {
            paragraph = paragraph.wrap(Wrap { trim: false });
        } else if self.wrap_mode == WrapMode::Char {
            paragraph = paragraph.wrap(Wrap { trim: true });
        }
        f.render_widget(paragraph, content_area);

        // Draw cursor
        if self.mode == Mode::Normal || self.mode == Mode::FileList {
            let total_lines = if self.raw_mode {
                self.content.lines().count()
            } else {
                self.lines.len()
            };
            let line_num_width = if self.show_line_numbers {
                total_lines.to_string().len() + 1
            } else {
                0
            } as u16;
            let screen_y = content_area.y + self.cursor_line.saturating_sub(self.scroll) as u16;
            let screen_x = content_area.x + line_num_width + self.cursor_col.saturating_sub(self.h_scroll);
            if screen_y < content_area.bottom() && screen_x < content_area.right() {
                if let Some(cell) = f.buffer_mut().cell_mut((screen_x, screen_y)) {
                    std::mem::swap(&mut cell.fg, &mut cell.bg);
                }
            }
        }

        if self.show_status {
            let status = self.build_status_bar();
            f.render_widget(status, areas[1]);
        }
    }

    fn build_status_bar(&self) -> Paragraph<'static> {
        let file_name = self.current_file_name();
        let total = if self.raw_mode {
            self.content.lines().count()
        } else {
            self.lines.len()
        };
        let ln = (self.cursor_line + 1).min(total);
        let pct = if total == 0 {
            0
        } else {
            (self.cursor_line * 100 / total).min(100)
        };

        let mut parts = vec![
            format!(" {} — Ln {}/{} ({}%) ", file_name, ln, total, pct),
            format!(" wrap:{} ", self.wrap_mode.as_str()),
        ];

        if self.raw_mode {
            parts.push(" RAW ".to_string());
        }

        if self.has_multiple_files() {
            parts.push(format!(
                " file {}/{} ",
                self.file_index + 1,
                self.files.len()
            ));
        }

        if !self.search_query.is_empty() {
            let match_info = match self.search_idx {
                Some(idx) => {
                    if let Some(pos) = self.search_results.iter().position(|&r| r == idx) {
                        format!(" \"{}\" [{}/{}] ", self.search_query, pos + 1, self.search_results.len())
                    } else {
                        format!(" \"{}\" [0/{}] ", self.search_query, self.search_results.len())
                    }
                }
                None => format!(" \"{}\" [0/{}] ", self.search_query, self.search_results.len()),
            };
            parts.push(match_info);
        }

        if self.show_line_numbers {
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
                let prompt = format!(" /{}_ ", self.input_buf);
                left_spans.push(Span::styled(
                    prompt,
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Mode::SearchBackward => {
                let prompt = format!(" ?{}_ ", self.input_buf);
                left_spans.push(Span::styled(
                    prompt,
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Mode::GoToLine => {
                let prompt = format!(" :{}_ ", self.input_buf);
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

    fn handle_normal_key(&mut self, code: KeyCode) {
        self.status_message = None;

        if self.expecting_bookmark_set {
            self.expecting_bookmark_set = false;
            if let KeyCode::Char(c) = code
                && c.is_ascii_lowercase()
            {
                self.bookmarks.insert(c, self.cursor_line);
            }
            return;
        }
        if self.expecting_bookmark_jump {
            self.expecting_bookmark_jump = false;
            if let KeyCode::Char(c) = code
                && c.is_ascii_lowercase()
            {
                if let Some(&pos) = self.bookmarks.get(&c) {
                    self.cursor_line = pos.min(self.max_scroll);
                    self.follow_cursor();
                }
            }
            return;
        }

        match code {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if c == '0' && self.pending_count.is_none() {
                    // standalone 0 → column 0 (vim-like)
                    self.cursor_col = 0;
                } else {
                    let d = c.to_digit(10).unwrap() as usize;
                    self.pending_count = Some(self.pending_count.unwrap_or(0) * 10 + d);
                }
                return;
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                std::process::exit(0);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let count = self.pending_count.take().unwrap_or(1);
                self.cursor_line = self.cursor_line.saturating_sub(count);
                self.follow_cursor();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let count = self.pending_count.take().unwrap_or(1);
                self.cursor_line = self.cursor_line.saturating_add(count).min(self.max_scroll);
                self.follow_cursor();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let count = self.pending_count.take().unwrap_or(1) as u16;
                self.cursor_col = self.cursor_col.saturating_sub(count);
                self.follow_cursor();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let count = self.pending_count.take().unwrap_or(1) as u16;
                let max_col = self.line_width(self.cursor_line);
                self.cursor_col = self.cursor_col.saturating_add(count).min(max_col);
                self.follow_cursor();
            }
            KeyCode::PageUp | KeyCode::Char('b') => {
                let count = self.pending_count.take().unwrap_or(1);
                self.cursor_line = self.cursor_line.saturating_sub(self.viewport_height as usize * count);
                self.follow_cursor();
            }
            KeyCode::PageDown | KeyCode::Char('f') => {
                let count = self.pending_count.take().unwrap_or(1);
                self.cursor_line = self.cursor_line
                    .saturating_add(self.viewport_height as usize * count)
                    .min(self.max_scroll);
                self.follow_cursor();
            }
            KeyCode::Home | KeyCode::Char('g') => {
                let target = self.pending_count.take().map(|c| c.saturating_sub(1)).unwrap_or(0);
                self.cursor_line = target.min(self.max_scroll);
                self.cursor_col = 0;
                self.h_scroll = 0;
                self.scroll = target.min(self.max_scroll);
            }
            KeyCode::End | KeyCode::Char('G') => {
                let target = self.pending_count.take().map(|c| c.saturating_sub(1)).unwrap_or(self.max_scroll);
                self.cursor_line = target.min(self.max_scroll);
                self.cursor_col = 0;
                self.follow_cursor();
            }
            KeyCode::Char('/') => {
                self.mode = Mode::SearchForward;
                self.input_buf.clear();
            }
            KeyCode::Char('?') => {
                self.mode = Mode::SearchBackward;
                self.input_buf.clear();
            }
            KeyCode::Char(':') => {
                self.mode = Mode::GoToLine;
                self.input_buf.clear();
            }
            KeyCode::Char('n') => {
                let count = self.pending_count.take().unwrap_or(1);
                if !self.search_results.is_empty() {
                    for _ in 0..count {
                        self.search_idx = find_next_match(&self.search_results, self.search_idx);
                    }
                    if let Some(idx) = self.search_idx {
                        self.cursor_line = idx.min(self.max_scroll);
                        self.follow_cursor();
                    }
                }
            }
            KeyCode::Char('N') => {
                let count = self.pending_count.take().unwrap_or(1);
                if !self.search_results.is_empty() {
                    for _ in 0..count {
                        self.search_idx = find_prev_match(&self.search_results, self.search_idx);
                    }
                    if let Some(idx) = self.search_idx {
                        self.cursor_line = idx.min(self.max_scroll);
                        self.follow_cursor();
                    }
                }
            }
            KeyCode::Char('r') => {
                self.raw_mode = !self.raw_mode;
                if self.raw_mode {
                    let line_count = self.content.lines().count().saturating_sub(1);
                    self.max_scroll = line_count;
                } else {
                    self.max_scroll = self.lines.len().saturating_sub(1);
                }
                self.search_query.clear();
                self.search_results.clear();
                self.search_idx = None;
                self.cursor_line = self.cursor_line.min(self.max_scroll);
                self.follow_cursor();
            }
            KeyCode::Char('R') => {
                if !self.files.is_empty() {
                    self.reload_current_file();
                }
            }
            KeyCode::Char(']') => {
                if self.has_multiple_files() {
                    self.file_index = (self.file_index + 1) % self.files.len();
                    self.cursor_line = 0;
                    self.cursor_col = 0;
                    self.scroll = 0;
                    self.h_scroll = 0;
                    self.search_idx = None;
                    self.load_current_file();
                }
            }
            KeyCode::Char('[') => {
                if self.has_multiple_files() {
                    self.file_index = if self.file_index == 0 {
                        self.files.len() - 1
                    } else {
                        self.file_index - 1
                    };
                    self.cursor_line = 0;
                    self.cursor_col = 0;
                    self.scroll = 0;
                    self.h_scroll = 0;
                    self.search_idx = None;
                    self.load_current_file();
                }
            }
            KeyCode::Char('m') => {
                self.expecting_bookmark_set = true;
            }
            KeyCode::Char('\'') => {
                self.expecting_bookmark_jump = true;
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
        self.raw_mode = false;
        let prev = match self.file_history.pop() {
            Some(p) => p,
            None => {
                self.status_message = Some("No previous file in history".to_string());
                return;
            }
        };
        if !self.files.is_empty() {
            self.files[self.file_index] = prev;
        } else {
            self.files.push(prev);
            self.file_index = 0;
        }
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll = 0;
        self.h_scroll = 0;
        self.search_idx = None;
        self.search_query.clear();
        self.search_results.clear();
        self.load_current_file();
        self.status_message = None;
    }

    fn follow_wiki_link(&mut self) {
        let links = match self.wiki_links.get(self.cursor_line) {
            Some(l) if !l.is_empty() => l,
            _ => {
                self.status_message = Some("No wiki link on this line".to_string());
                return;
            }
        };
        let link = links.iter()
            .filter(|l| l.col <= self.cursor_col as usize)
            .last()
            .unwrap_or(&links[0]);
        let target_path = if let Some(parent) = &self.parent_dir {
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
        if !self.files.is_empty() {
            self.file_history.push(self.files[self.file_index].clone());
        }
        // Replace current file with the wiki link target
        if !self.files.is_empty() {
            self.files[self.file_index] = target_path;
        } else {
            self.files.push(target_path);
            self.file_index = 0;
        }
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll = 0;
        self.h_scroll = 0;
        self.search_idx = None;
        self.search_query.clear();
        self.search_results.clear();
        self.load_current_file();
    }

    fn handle_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                let query = std::mem::take(&mut self.input_buf);
                let forward = matches!(self.mode, Mode::SearchForward);
                self.search_query = query.clone();
                if !query.is_empty() {
                    if self.search_history.last().map_or(true, |last| last != &query) {
                        self.search_history.push(query.clone());
                        if self.search_history.len() > 50 {
                            self.search_history.remove(0);
                        }
                    }
                    self.search_history_idx = None;
                    self.search_results = self.search_lines_for_mode( &query);
                    self.search_idx = if forward {
                        find_next_match(&self.search_results, Some(self.scroll))
                    } else {
                        find_prev_match(&self.search_results, Some(self.scroll))
                    };
                    if let Some(idx) = self.search_idx {
                        self.scroll = idx.min(self.max_scroll);
                    }
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Up => {
                let idx = self.search_history_idx.get_or_insert(self.search_history.len());
                if *idx > 0 {
                    *idx -= 1;
                    self.input_buf = self.search_history[*idx].clone();
                    self.search_query = self.input_buf.clone();
                    self.search_results = self.search_lines_for_mode( &self.search_query);
                    self.search_idx = None;
                }
            }
            KeyCode::Down => {
                if let Some(idx) = &mut self.search_history_idx {
                    if *idx + 1 < self.search_history.len() {
                        *idx += 1;
                        self.input_buf = self.search_history[*idx].clone();
                    } else {
                        self.search_history_idx = None;
                        self.input_buf.clear();
                    }
                    self.search_query = self.input_buf.clone();
                    self.search_results = self.search_lines_for_mode( &self.search_query);
                    self.search_idx = None;
                }
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.input_buf.clear();
            }
            KeyCode::Backspace => {
                self.input_buf.pop();
                self.search_query = self.input_buf.clone();
                if !self.search_query.is_empty() {
                    self.search_results = self.search_lines_for_mode( &self.search_query);
                    self.search_idx = None;
                } else {
                    self.search_results.clear();
                    self.search_idx = None;
                }
            }
            KeyCode::Char(c) => {
                self.input_buf.push(c);
                self.search_query = self.input_buf.clone();
                if !self.search_query.is_empty() {
                    self.search_results = self.search_lines_for_mode( &self.search_query);
                    self.search_idx = None;
                }
            }
            _ => {}
        }
    }

    fn handle_goto_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.input_buf);
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
                    let line = (pct_val * self.max_scroll / 100).min(self.max_scroll);
                    self.cursor_line = line;
                    self.scroll = line;
                } else if let Ok(line_num) = trimmed.parse::<usize>() {
                    let line = line_num.saturating_sub(1).min(self.max_scroll);
                    self.cursor_line = line;
                    self.scroll = line;
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.input_buf.clear();
            }
            KeyCode::Backspace => {
                self.input_buf.pop();
            }
            KeyCode::Char(c) if !c.is_control() => {
                self.input_buf.push(c);
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
            self.wrap_mode = WrapMode::from_str(&wrap);
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
        for (i, path) in self.files.iter().enumerate() {
            let marker = if i == self.file_index { "▸ " } else { "  " };
            let fg = if i == self.file_index {
                Color::Cyan
            } else {
                Color::White
            };
            let bold = if i == self.file_index {
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
        let para = self.theme.style_for("paragraph");
        let (pfg, pbg, _, _, _, _) = para.unwrap_or((None, None, false, false, false, false));
        let mut para_style = Style::default();
        if let Some(c) = pfg { para_style = para_style.fg(c); }
        if let Some(c) = pbg { para_style = para_style.bg(c); }

        let frontmatter_style = para_style.add_modifier(Modifier::DIM);

        let code_block = self.theme.style_for("code_block");
        let (cfg, cbg, cbold, citalic, _, _) = code_block.unwrap_or((None, None, false, false, false, false));
        let mut code_style = Style::default();
        if let Some(c) = cfg { code_style = code_style.fg(c); }
        if let Some(c) = cbg { code_style = code_style.bg(c); }
        if cbold { code_style = code_style.add_modifier(Modifier::BOLD); }
        if citalic { code_style = code_style.add_modifier(Modifier::ITALIC); }

        let heading_styles: Vec<Style> = (0..6).map(|i| {
            let key = format!("heading{}", i + 1);
            let h = self.theme.style_for(&key);
            let (hfg, hbg, hbold, hitalic, _, _) = h.unwrap_or((pfg, pbg, false, false, false, false));
            let mut s = Style::default();
            if let Some(c) = hfg { s = s.fg(c); }
            if let Some(c) = hbg { s = s.bg(c); }
            if hbold { s = s.add_modifier(Modifier::BOLD); }
            if hitalic { s = s.add_modifier(Modifier::ITALIC); }
            s
        }).collect();

        let content_lines: Vec<&str> = self.content.lines().collect();
        let total_lines = content_lines.len();
        let query_lower = self.search_query.to_lowercase();
        let has_search = !self.search_query.is_empty();

        // Check if content starts with frontmatter
        let first_non_empty = content_lines.iter().find(|l| !l.trim().is_empty()).copied();
        let has_frontmatter = first_non_empty.map_or(false, |l| l.trim() == "---");

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
            } else if let Some(level) = trimmed
                .chars()
                .position(|c| c != '#')
                .filter(|&pos| pos > 0 && pos <= 6 && trimmed.as_bytes().get(pos).map_or(false, |&b| b == b' ' || pos == trimmed.len()))
            {
                style = heading_styles[level - 1];
            } else {
                style = para_style;
            }

            let mut spans = vec![Span::styled((*line).to_string(), style)];

            if has_search && line.to_lowercase().contains(&query_lower) {
                let line_obj = Line::from(spans);
                let highlighted = highlight_line(&line_obj, &self.search_query);
                spans = highlighted.spans;
            }

            let mut line_obj = Line::from(spans);

            if self.show_line_numbers {
                line_obj = prepend_line_number(line_obj, i + 1, total_lines);
            }

            result.push(line_obj);
        }

        result
    }

    fn handle_filelist_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.file_index > 0 {
                    self.file_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.file_index + 1 < self.files.len() {
                    self.file_index += 1;
                }
            }
            KeyCode::Enter => {
                self.scroll = 0;
                self.h_scroll = 0;
                self.search_idx = None;
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
    let num_span = Span::styled(
        num_str,
        Style::default().fg(Color::DarkGray),
    );
    line.spans.insert(0, num_span);
    line
}