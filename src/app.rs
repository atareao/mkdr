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
use crate::render;

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
    theme: Theme,
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
        };

        if let Some(stdin_content) = content_from_stdin {
            app.content = stdin_content;
            app.render_content();
            if start_line > 1 {
                app.scroll = (start_line - 1).min(app.max_scroll);
            }
        } else if !app.files.is_empty() {
            app.load_current_file();
            if start_line > 1 {
                app.scroll = (start_line - 1).min(app.max_scroll);
            }
        }

        app
    }

    fn load_current_file(&mut self) {
        if self.files.is_empty() {
            return;
        }
        let path = &self.files[self.file_index];
        self.content = std::fs::read_to_string(path).unwrap_or_default();
        self.last_modified = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
        self.render_content();
    }

    fn reload_current_file(&mut self) {
        self.load_current_file();
        self.search_results = if self.search_query.is_empty() {
            Vec::new()
        } else {
            search_lines(&self.raw_lines, &self.search_query)
        };
    }

    fn render_content(&mut self) {
        let (styled_lines, raw) = render::render(&self.content, &self.theme);
        self.lines = styled_lines;
        self.raw_lines = raw;
        self.max_scroll = self.lines.len().saturating_sub(1);
        self.scroll = self.scroll.min(self.max_scroll);
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

    fn render_frame(&self, f: &mut ratatui::Frame) {
        let area = f.area();

        let areas = if self.show_status {
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area)
        } else {
            Layout::vertical([Constraint::Min(1)]).split(area)
        };

        let content_area = areas[0];

        let display_lines: Vec<Line<'static>> = if self.mode == Mode::FileList {
            self.build_file_list()
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

        if self.show_status {
            let status = self.build_status_bar();
            f.render_widget(status, areas[1]);
        }
    }

    fn build_status_bar(&self) -> Paragraph<'static> {
        let file_name = self.current_file_name();
        let ln = (self.scroll + 1).min(self.lines.len());
        let total = self.lines.len();
        let pct = if total == 0 {
            0
        } else {
            (self.scroll * 100 / total).min(100)
        };

        let mut parts = vec![
            format!(" {} — Ln {}/{} ({}%) ", file_name, ln, total, pct),
            format!(" wrap:{} ", self.wrap_mode.as_str()),
        ];

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
        if self.expecting_bookmark_set {
            self.expecting_bookmark_set = false;
            if let KeyCode::Char(c) = code
                && c.is_ascii_lowercase()
            {
                self.bookmarks.insert(c, self.scroll);
            }
            return;
        }
        if self.expecting_bookmark_jump {
            self.expecting_bookmark_jump = false;
            if let KeyCode::Char(c) = code
                && c.is_ascii_lowercase()
            {
                if let Some(&pos) = self.bookmarks.get(&c) {
                    self.scroll = pos.min(self.max_scroll);
                }
            }
            return;
        }

        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                std::process::exit(0);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1).min(self.max_scroll);
            }
            KeyCode::Left => {
                if self.wrap_mode == WrapMode::None {
                    self.h_scroll = self.h_scroll.saturating_sub(4);
                }
            }
            KeyCode::Right => {
                if self.wrap_mode == WrapMode::None {
                    self.h_scroll = self.h_scroll.saturating_add(4);
                }
            }
            KeyCode::PageUp | KeyCode::Char('b') => {
                self.scroll = self.scroll.saturating_sub(20);
            }
            KeyCode::PageDown | KeyCode::Char('f') => {
                self.scroll = self.scroll.saturating_add(20).min(self.max_scroll);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.scroll = 0;
                self.h_scroll = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.scroll = self.max_scroll;
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
                if !self.search_results.is_empty() {
                    self.search_idx = find_next_match(&self.search_results, self.search_idx);
                    if let Some(idx) = self.search_idx {
                        self.scroll = idx.min(self.max_scroll);
                    }
                }
            }
            KeyCode::Char('N') => {
                if !self.search_results.is_empty() {
                    self.search_idx = find_prev_match(&self.search_results, self.search_idx);
                    if let Some(idx) = self.search_idx {
                        self.scroll = idx.min(self.max_scroll);
                    }
                }
            }
            KeyCode::Char('r') => {
                if !self.files.is_empty() {
                    self.reload_current_file();
                }
            }
            KeyCode::Char(']') => {
                if self.has_multiple_files() {
                    self.file_index = (self.file_index + 1) % self.files.len();
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
            _ => {}
        }
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
                    self.search_results = search_lines(&self.raw_lines, &query);
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
                    self.search_results = search_lines(&self.raw_lines, &self.search_query);
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
                    self.search_results = search_lines(&self.raw_lines, &self.search_query);
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
                    self.search_results = search_lines(&self.raw_lines, &self.search_query);
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
                    self.search_results = search_lines(&self.raw_lines, &self.search_query);
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
                    self.scroll = (pct_val * self.max_scroll / 100).min(self.max_scroll);
                } else if let Ok(line_num) = trimmed.parse::<usize>() {
                    self.scroll = line_num.saturating_sub(1).min(self.max_scroll);
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