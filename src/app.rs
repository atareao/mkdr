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
use termimad::MadSkin;

use crate::ansi::{parse_ansi_line, strip_ansi};
use crate::search::{find_next_match, find_prev_match, highlight_line, search_lines};

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

pub enum Mode {
    Normal,
    SearchForward,
    SearchBackward,
    GoToLine,
}

pub struct App {
    lines: Vec<Line<'static>>,
    raw_lines: Vec<String>,
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
    show_status: bool,
    show_line_numbers: bool,
    wrap_mode: WrapMode,
    skin: MadSkin,
    follow: bool,
    last_modified: Option<SystemTime>,
    content: String,
    input_mode_prompt: String,
}

impl App {
    pub fn new(
        files: Vec<PathBuf>,
        follow: bool,
        wrap_mode: WrapMode,
        line_numbers: bool,
        show_status: bool,
        skin: MadSkin,
        start_line: usize,
        content_from_stdin: Option<String>,
    ) -> Self {
        let mut app = App {
            lines: Vec::new(),
            raw_lines: Vec::new(),
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
            show_status,
            show_line_numbers: line_numbers,
            wrap_mode,
            skin,
            follow,
            last_modified: None,
            content: String::new(),
            input_mode_prompt: String::new(),
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
        let styled = self.skin.text(&self.content, None);
        let styled_text = format!("{}", styled);

        self.raw_lines = styled_text.lines().map(|s| strip_ansi(s)).collect();
        self.lines = styled_text
            .lines()
            .map(|s| parse_ansi_line(s))
            .collect();
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

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match &self.mode {
                        Mode::Normal => self.handle_normal_key(key.code),
                        Mode::SearchForward | Mode::SearchBackward => {
                            self.handle_search_key(key.code);
                        }
                        Mode::GoToLine => self.handle_goto_key(key.code),
                    }
                }
            }
        }
    }

    fn check_file_changed(&mut self) {
        if self.files.is_empty() {
            return;
        }
        let path = &self.files[self.file_index];
        if let Ok(modified) = std::fs::metadata(path).and_then(|m| m.modified()) {
            if self.last_modified.map_or(true, |lm| lm != modified) {
                self.last_modified = Some(modified);
                self.reload_current_file();
            }
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

        let display_lines: Vec<Line<'static>> = self
            .lines
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
            .collect();

        let mut paragraph = Paragraph::new(display_lines).scroll((self.scroll as u16, self.h_scroll));
        if self.wrap_mode == WrapMode::Word {
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
            Mode::Normal => {}
        }

        Paragraph::new(Line::from(left_spans))
    }

    fn handle_normal_key(&mut self, code: KeyCode) {
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
                self.input_mode_prompt = "/".to_string();
            }
            KeyCode::Char('?') => {
                self.mode = Mode::SearchBackward;
                self.input_buf.clear();
                self.input_mode_prompt = "?".to_string();
            }
            KeyCode::Char(':') => {
                self.mode = Mode::GoToLine;
                self.input_buf.clear();
                self.input_mode_prompt = ":".to_string();
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
                if let Ok(line_num) = self.input_buf.parse::<usize>() {
                    let target = line_num.saturating_sub(1).min(self.max_scroll);
                    self.scroll = target;
                }
                self.mode = Mode::Normal;
                self.input_buf.clear();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.input_buf.clear();
            }
            KeyCode::Backspace => {
                self.input_buf.pop();
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.input_buf.push(c);
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