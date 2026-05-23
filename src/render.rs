use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::theme::Theme;

const BULLET_CHAR: char = '•';
const QUOTE_CHAR: char = '▐';

pub fn render(content: &str, theme: &Theme) -> (Vec<Line<'static>>, Vec<String>) {
    let renderer = Renderer::new(theme);
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(content, opts);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut raw: Vec<String> = Vec::new();

    renderer.render_doc(parser, &mut lines, &mut raw);

    (lines, raw)
}

#[derive(Default)]
struct ThemeStyle {
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

struct Renderer {
    para: ThemeStyle,
    bold: ThemeStyle,
    italic: ThemeStyle,
    strike: ThemeStyle,
    code: ThemeStyle,
    code_block: ThemeStyle,
    headings: [ThemeStyle; 6],
    link: ThemeStyle,
    table_border: ThemeStyle,
    bullet: Option<Color>,
    quote_mark: Option<Color>,
    rule: Option<Color>,
}

impl Renderer {
    fn new(theme: &Theme) -> Self {
        let para = theme.style_for("paragraph");
        let bold = theme.style_for("bold").or(para);
        let italic = theme.style_for("italic").or(para);
        let strike = theme.style_for("strikeout").or(para);
        let code = theme.style_for("inline_code");
        let code_block = theme.style_for("code_block");
        let link = theme.style_for("link");
        let table_border = theme.style_for("table");
        let bullet = theme.fg_for("bullet");
        let quote_mark = theme.fg_for("quote_mark");
        let rule = theme.fg_for("horizontal_rule");

        let def_fg = para.and_then(|(fg, _, _, _, _, _)| fg);
        let def_bg = para.and_then(|(_, bg, _, _, _, _)| bg);

        Self {
            para: ThemeStyle { fg: def_fg, bg: def_bg, ..Default::default() },
            bold: bold.map(|(fg, bg, b, i, u, s)| ThemeStyle { fg, bg, bold: b, italic: i, underline: u, strikethrough: s })
                .unwrap_or(ThemeStyle { fg: def_fg, bg: def_bg, bold: true, ..Default::default() }),
            italic: italic.map(|(fg, bg, b, i, u, s)| ThemeStyle { fg, bg, bold: b, italic: i, underline: u, strikethrough: s })
                .unwrap_or(ThemeStyle { fg: def_fg, bg: def_bg, italic: true, ..Default::default() }),
            strike: strike.map(|(fg, bg, b, i, u, s)| ThemeStyle { fg, bg, bold: b, italic: i, underline: u, strikethrough: s })
                .unwrap_or(ThemeStyle { fg: def_fg, bg: def_bg, strikethrough: true, ..Default::default() }),
            code: code.map(|(fg, bg, b, i, u, s)| ThemeStyle { fg, bg, bold: b, italic: i, underline: u, strikethrough: s })
                .unwrap_or(ThemeStyle { fg: Some(Color::Yellow), bg: def_bg, ..Default::default() }),
            code_block: code_block.map(|(fg, bg, b, i, u, s)| ThemeStyle { fg, bg, bold: b, italic: i, underline: u, strikethrough: s })
                .unwrap_or(ThemeStyle { fg: def_fg, bg: def_bg, ..Default::default() }),
            headings: std::array::from_fn(|i| {
                let key = format!("heading{}", i + 1);
                theme.style_for(&key).map(|(fg, bg, b, i, u, s)| ThemeStyle { fg, bg, bold: b, italic: i, underline: u, strikethrough: s })
                    .unwrap_or(ThemeStyle { fg: def_fg, bg: def_bg, bold: i < 3, ..Default::default() })
            }),
            link: link.map(|(fg, bg, b, i, u, s)| ThemeStyle { fg, bg, bold: b, italic: i, underline: u, strikethrough: s })
                .unwrap_or(ThemeStyle { fg: Some(Color::Cyan), underline: true, ..Default::default() }),
            table_border: table_border.map(|(fg, bg, _, _, _, _)| ThemeStyle { fg, bg, ..Default::default() })
                .unwrap_or(ThemeStyle { fg: Some(Color::DarkGray), ..Default::default() }),
            bullet,
            quote_mark,
            rule,
        }
    }

    fn render_doc(&self, mut parser: Parser<'_>, lines: &mut Vec<Line<'static>>, raw: &mut Vec<String>) {
        let mut list_counters: Vec<usize> = Vec::new();
        let mut in_table = false;
        let mut table_data: TableData = TableData::default();
let mut needs_space = false;
        let mut quote_depth: usize = 0;

        loop {
            match parser.next() {
                Some(Event::Start(tag)) => match tag {
                    Tag::Paragraph => {
                        if needs_space {
                            raw.push(String::new());
                            lines.push(Line::from(vec![]));
                        }
                        let spans = self.collect_inline(&mut parser, &TagEnd::Paragraph, &self.para);
                        raw_line(&spans, raw);
                        lines.push(Line::from(spans));
                        needs_space = true;
                    }
                    Tag::Heading { level, .. } => {
                        if needs_space {
                            raw.push(String::new());
                            lines.push(Line::from(vec![]));
                        }
                        let idx = (level as usize).saturating_sub(1).min(5);
                        let hl = &self.headings[idx];
                        let spans = self.collect_inline(&mut parser, &TagEnd::Heading(level), hl);
                        raw_line(&spans, raw);
                        lines.push(Line::from(spans));
                        needs_space = true;
                    }
Tag::CodeBlock(kind) => {
                        if needs_space {
                            raw.push(String::new());
                            lines.push(Line::from(vec![]));
                        }
                        if let CodeBlockKind::Fenced(ref info) = kind
                            && !info.is_empty()
                        {
                            raw.push(info.to_string());
                            lines.push(Line::from(Span::styled(
                                format!(" {} ", info),
                                Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                            )));
                        }
                        let code = self.collect_code(&mut parser);
                        if let CodeBlockKind::Fenced(info) = kind
                            && !info.is_empty()
                        {
                            for (mut spans, raw_text) in highlight_code(&info, &code, self.code_block.bg) {
                                spans.insert(0, Span::raw("  "));
                                raw.push(format!("  {}", raw_text));
                                lines.push(Line::from(spans));
                            }
                        } else {
                            for line_text in code.lines() {
                                raw.push(format!("  {}", line_text));
                                lines.push(Line::from(Span::styled(
                                    format!("  {}", line_text),
                                    self.code_block.as_style(),
                                )));
                            }
                        }
                        needs_space = true;
                    }
                    Tag::List(start) => {
                        if needs_space {
                            raw.push(String::new());
                            lines.push(Line::from(vec![]));
                            needs_space = false;
                        }
                        list_counters.push(start.unwrap_or(1) as usize);
                    }
                    Tag::Item => {
                        let depth = list_counters.len();
                        if let Some(counter) = list_counters.last_mut() {
                            let bullet = if depth == 1 {
                                format!("{} {} ", BULLET_CHAR, self.bullet_prefix(*counter))
                            } else {
                                format!("  {} {} ", BULLET_CHAR, self.bullet_prefix(*counter))
                            };
                            *counter += 1;
                            let prefix = Span::styled(
                                bullet,
                                Style::default().fg(self.bullet.unwrap_or(Color::DarkGray)),
                            );
                            let mut item_spans = self.collect_inline(&mut parser, &TagEnd::Item, &self.para);
                            item_spans.insert(0, prefix);
                            raw_line(&item_spans, raw);
                            lines.push(Line::from(item_spans));
                        }
                    }
                    Tag::Table(alignments) => {
                        in_table = true;
                        table_data = TableData { alignments, ..Default::default() };
                    }
                    Tag::TableHead => {
                        let row = self.collect_table_row(&mut parser);
                        table_data.headers = row;
                    }
                    Tag::TableRow => {
                        let row = self.collect_table_row(&mut parser);
                        table_data.rows.push(row);
                    }
                    Tag::BlockQuote(_) => {
                        quote_depth += 1;
                        if needs_space {
                            raw.push(String::new());
                            lines.push(Line::from(vec![]));
                        }
                        let spans = self.collect_inline_with_breaks(&mut parser, &TagEnd::BlockQuote(None), &self.para, true);
                        let mut line_groups: Vec<Vec<Span<'static>>> = vec![Vec::new()];
                        for span in spans {
                            if span.content == "\n" {
                                line_groups.push(Vec::new());
                            } else if let Some(last) = line_groups.last_mut() {
                                last.push(span);
                            }
                        }
                        if line_groups.is_empty() {
                            line_groups.push(Vec::new());
                        }
                        let colors = [Color::Rgb(106, 153, 85), Color::Rgb(86, 156, 214), Color::Rgb(212, 71, 71)];
                        let quote_color = self.quote_mark.unwrap_or(colors[(quote_depth - 1) % colors.len()]);
                        let mark_style = Style::default().fg(quote_color);
                        for group in &line_groups {
                            raw_line(group, raw);
                            let mut quoted = vec![Span::styled(format!("{} ", QUOTE_CHAR), mark_style)];
                            quoted.extend(group.iter().cloned());
                            lines.push(Line::from(quoted));
                        }
                        needs_space = true;
                    }
                    Tag::FootnoteDefinition(_) => {
                        let _ = self.skip_to(&mut parser, &TagEnd::FootnoteDefinition);
                    }
                    _ => {}
                },
                Some(Event::End(tag_end)) => match tag_end {
                    TagEnd::List(_) => {
                        list_counters.pop();
                        needs_space = true;
                    }
                    TagEnd::BlockQuote(_) => {
                        quote_depth = quote_depth.saturating_sub(1);
                    }
                    TagEnd::Table if in_table => {
                            if needs_space {
                            raw.push(String::new());
                            lines.push(Line::from(vec![]));
                        }
                            in_table = false;
                            self.render_table(&table_data, lines, raw);
                            needs_space = true;
                        }
                    TagEnd::TableHead | TagEnd::TableRow | TagEnd::TableCell => {}
                    TagEnd::Paragraph => {}
                    _ => {}
                },
                Some(Event::Rule) => {
                    if needs_space {
                            raw.push(String::new());
                            lines.push(Line::from(vec![]));
                        }
                    raw.push(String::new());
                    lines.push(Line::from(Span::styled(
                        "─".repeat(80),
                        Style::default().fg(self.rule.unwrap_or(Color::DarkGray)),
                    )));
                    needs_space = true;
                }
                Some(Event::Html(text)) => {
                    raw.push(text.to_string());
                    lines.push(Line::from(Span::styled(
                        text.to_string(),
                        self.code.as_style(),
                    )));
                }
                Some(Event::SoftBreak) | Some(Event::HardBreak) => {
                    // handled inside collect_inline
                }
                None => break,
                _ => {}
            }
        }
    }

    fn collect_inline(&self, events: &mut Parser<'_>, end_tag: &TagEnd, base: &ThemeStyle) -> Vec<Span<'static>> {
        self.collect_inline_with_breaks(events, end_tag, base, false)
    }

    fn collect_inline_with_breaks(
        &self,
        events: &mut Parser<'_>,
        end_tag: &TagEnd,
        base: &ThemeStyle,
        preserve_breaks: bool,
    ) -> Vec<Span<'static>> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut buf = String::new();

        loop {
            match events.next() {
                Some(Event::Start(tag)) => {
                    flush_buf(&mut buf, &mut spans, base);
                    match tag {
                        Tag::Emphasis => {
                            spans.extend(self.collect_inline_with_breaks(events, &TagEnd::Emphasis, &self.italic, preserve_breaks));
                        }
                        Tag::Strong => {
                            spans.extend(self.collect_inline_with_breaks(events, &TagEnd::Strong, &self.bold, preserve_breaks));
                        }
                        Tag::Strikethrough => {
                            spans.extend(self.collect_inline_with_breaks(events, &TagEnd::Strikethrough, &self.strike, preserve_breaks));
                        }
                        Tag::Link { ref dest_url, .. } => {
                            let mut child = self.collect_inline_with_breaks(events, &TagEnd::Link, &self.link, preserve_breaks);
                            if !dest_url.is_empty() {
                                let url_style = Style::default()
                                    .fg(Color::DarkGray)
                                    .add_modifier(Modifier::DIM);
                                child.push(Span::styled(format!(" ─ {}", dest_url), url_style));
                            }
                            spans.append(&mut child);
                        }
                        Tag::Image { ref dest_url, .. } => {
                            let child = self.collect_inline_with_breaks(events, &TagEnd::Image, &self.para, preserve_breaks);
                            let icon = Span::styled(
                                "🖼 ".to_string(),
                                Style::default().fg(Color::DarkGray),
                            );
                            spans.push(icon);
                            spans.extend(child);
                            if !dest_url.is_empty() {
                                let url_style = Style::default()
                                    .fg(Color::DarkGray)
                                    .add_modifier(Modifier::DIM);
                                spans.push(Span::styled(format!(" ─ {}", dest_url), url_style));
                            }
                        }
                        Tag::CodeBlock(_) | Tag::Paragraph | Tag::Heading { .. } => {
                            let _ = self.skip_to(events, &end_of(&tag));
                        }
                        _ => {}
                    }
                }
                Some(Event::End(tag_end)) if &tag_end == end_tag => {
                    flush_buf(&mut buf, &mut spans, base);
                    break;
                }
                Some(Event::Text(text)) => {
                    buf.push_str(&text);
                }
                Some(Event::Code(text)) => {
                    flush_buf(&mut buf, &mut spans, base);
                    spans.push(Span::styled(text.to_string(), self.code.as_style()));
                }
                Some(Event::SoftBreak) | Some(Event::HardBreak) => {
                    if preserve_breaks {
                        flush_buf(&mut buf, &mut spans, base);
                        spans.push(Span::raw("\n"));
                    } else {
                        buf.push(' ');
                    }
                }
                Some(Event::TaskListMarker(checked)) => {
                    flush_buf(&mut buf, &mut spans, base);
                    let icon = if checked { "☑" } else { "☐" };
                    let color = if checked { Color::Green } else { Color::Red };
                    spans.push(Span::styled(
                        icon.to_string(),
                        Style::default()
                            .fg(color)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                None => break,
                _ => {}
            }
        }

        spans
    }

    fn collect_code(&self, events: &mut Parser<'_>) -> String {
        let mut code = String::new();
        loop {
            match events.next() {
                Some(Event::Text(text)) => code.push_str(&text),
                Some(Event::End(TagEnd::CodeBlock)) => break,
                None => break,
                _ => {}
            }
        }
        code
    }

    fn collect_table_row(&self, events: &mut Parser<'_>) -> Vec<Vec<Span<'static>>> {
        let mut row: Vec<Vec<Span<'static>>> = Vec::new();
        loop {
            match events.next() {
                Some(Event::Start(Tag::TableCell)) => {
                    let spans = self.collect_inline(events, &TagEnd::TableCell, &self.para);
                    row.push(spans);
                }
                Some(Event::End(TagEnd::TableRow)) => break,
                Some(Event::End(TagEnd::TableHead)) => break,
                None => break,
                _ => {}
            }
        }
        row
    }

    fn render_table(
        &self,
        data: &TableData,
        lines: &mut Vec<Line<'static>>,
        raw: &mut Vec<String>,
    ) {
        let b = self.table_border.as_style();
        let num_cols = data
            .headers
            .len()
            .max(data.rows.iter().map(|r| r.len()).max().unwrap_or(0))
            .max(1);

        let mut col_widths = vec![3usize; num_cols];
        for (i, cell) in data.headers.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i].max(cell_width(cell));
            }
        }
        for row in &data.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < num_cols {
                    col_widths[i] = col_widths[i].max(cell_width(cell));
                }
            }
        }

        // Top border
        lines.push(self.table_border_line(&col_widths, "┌", "┬", "┐", &b));
        raw.push(String::new());

        // Header
        if !data.headers.is_empty() {
            self.push_table_row(&data.headers, &col_widths, lines, raw, &b, &data.alignments, true);
            lines.push(self.table_border_line(&col_widths, "├", "┼", "┤", &b));
            raw.push(String::new());
        }

        // Data rows
        for row in &data.rows {
            self.push_table_row(row, &col_widths, lines, raw, &b, &data.alignments, false);
        }

        // Bottom border
        lines.push(self.table_border_line(&col_widths, "└", "┴", "┘", &b));
        raw.push(String::new());
    }

    fn table_border_line(
        &self,
        widths: &[usize],
        left: &str,
        sep: &str,
        right: &str,
        style: &Style,
    ) -> Line<'static> {
        let mut spans = vec![Span::styled(left.to_string(), *style)];
        for (i, w) in widths.iter().enumerate() {
            spans.push(Span::styled("─".repeat(w + 2), *style));
            if i < widths.len() - 1 {
                spans.push(Span::styled(sep.to_string(), *style));
            }
        }
        spans.push(Span::styled(right.to_string(), *style));
        Line::from(spans)
    }

    fn push_table_row(
        &self,
        cells: &[Vec<Span<'static>>],
        widths: &[usize],
        lines: &mut Vec<Line<'static>>,
        raw: &mut Vec<String>,
        style: &Style,
        alignments: &[Alignment],
        is_header: bool,
    ) {
        let mut spans = vec![Span::styled("│".to_string(), *style)];
        let mut raw_text = String::new();
        for (i, cell_spans) in cells.iter().enumerate() {
            let w = widths.get(i).copied().unwrap_or(3);
            let cell_width = cell_width(cell_spans);
            let pad = w.saturating_sub(cell_width);
            let align = alignments.get(i).copied().unwrap_or(Alignment::None);

            let left_pad = match align {
                Alignment::Right => pad,
                Alignment::Center => pad / 2,
                _ => 0,
            };
            let right_pad = match align {
                Alignment::Right => 0,
                Alignment::Center => pad - pad / 2,
                _ => pad,
            };

            if left_pad > 0 {
                spans.push(Span::styled(" ".repeat(left_pad), Style::default()));
            }
            for s in cell_spans {
                let span = if is_header {
                    let style = s.style.clone().add_modifier(Modifier::BOLD);
                    Span::styled(s.content.clone(), style)
                } else {
                    s.clone()
                };
                spans.push(span);
            }
            if right_pad > 0 {
                spans.push(Span::styled(" ".repeat(right_pad), Style::default()));
            }
            spans.push(Span::styled("  │".to_string(), *style));
            raw_text.push(' ');
            for _ in 0..left_pad {
                raw_text.push(' ');
            }
            for s in cell_spans {
                raw_text.push_str(s.content.as_ref());
            }
            raw_text.push_str(&" ".repeat(right_pad + 2));
        }
        raw.push(raw_text);
        lines.push(Line::from(spans));
    }

    fn bullet_prefix(&self, counter: usize) -> String {
        if counter == 0 {
            String::new()
        } else {
            counter.to_string()
        }
    }

    fn skip_to(&self, events: &mut Parser<'_>, tag_end: &TagEnd) -> String {
        let mut content = String::new();
        loop {
            match events.next() {
                Some(Event::End(t)) if &t == tag_end => break,
                Some(Event::Text(t)) => content.push_str(&t),
                Some(Event::Start(t)) => {
                    let _ = self.skip_to(events, &end_of(&t));
                }
                None => break,
                _ => {}
            }
        }
        content
    }
}

#[derive(Default)]
struct TableData {
    headers: Vec<Vec<Span<'static>>>,
    rows: Vec<Vec<Vec<Span<'static>>>>,
    alignments: Vec<Alignment>,
}

impl ThemeStyle {
    fn as_style(&self) -> Style {
        let mut s = Style::default();
        if let Some(c) = self.fg {
            s = s.fg(c);
        }
        if let Some(c) = self.bg {
            s = s.bg(c);
        }
        let mut mods = Modifier::empty();
        if self.bold {
            mods |= Modifier::BOLD;
        }
        if self.italic {
            mods |= Modifier::ITALIC;
        }
        if self.underline {
            mods |= Modifier::UNDERLINED;
        }
        if self.strikethrough {
            mods |= Modifier::CROSSED_OUT;
        }
        s.add_modifier(mods)
    }
}

fn highlight_code(language: &str, code: &str, bg: Option<Color>) -> Vec<(Vec<Span<'static>>, String)> {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    static THEME: OnceLock<syntect::highlighting::Theme> = OnceLock::new();

    let syntax_set = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let theme = THEME.get_or_init(|| {
        let ts = ThemeSet::load_defaults();
        ts.themes["base16-ocean.dark"].clone()
    });

    let syntax = syntax_set
        .find_syntax_by_token(language)
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut result = Vec::new();

    for line in LinesWithEndings::from(code) {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        if trimmed.is_empty() && line.ends_with('\n') {
            result.push((vec![Span::raw("\n")], String::new()));
            continue;
        }
        let highlighted = highlighter.highlight_line(trimmed, syntax_set).unwrap();

        let mut spans = Vec::new();
        let mut raw_text = String::new();

        for (style, text) in &highlighted {
            let mut s = Style::default()
                .fg(Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b));
            if let Some(bg_color) = bg {
                s = s.bg(bg_color);
            }
            if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                s = s.add_modifier(Modifier::BOLD);
            }
            if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                s = s.add_modifier(Modifier::ITALIC);
            }
            if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
                s = s.add_modifier(Modifier::UNDERLINED);
            }
            spans.push(Span::styled(text.to_string(), s));
            raw_text.push_str(text);
        }

        result.push((spans, raw_text));
    }

    result
}

fn flush_buf(buf: &mut String, spans: &mut Vec<Span<'static>>, base: &ThemeStyle) {
    if !buf.is_empty() {
        spans.push(Span::styled(std::mem::take(buf), base.as_style()));
    }
}

fn raw_line(spans: &[Span<'static>], raw: &mut Vec<String>) {
    let mut text = String::new();
    for s in spans {
        text.push_str(s.content.as_ref());
    }
    raw.push(text);
}

fn cell_width(spans: &[Span<'static>]) -> usize {
    spans.iter().map(|s| s.content.as_ref().width()).sum()
}

fn end_of(tag: &Tag) -> TagEnd {
    match tag {
        Tag::Paragraph => TagEnd::Paragraph,
        Tag::Heading { level, .. } => TagEnd::Heading(*level),
        Tag::CodeBlock(_) => TagEnd::CodeBlock,
        Tag::List(_) => TagEnd::List(false),
        Tag::Item => TagEnd::Item,
        Tag::FootnoteDefinition(_) => TagEnd::FootnoteDefinition,
        Tag::Table(_) => TagEnd::Table,
        Tag::TableHead => TagEnd::TableHead,
        Tag::TableRow => TagEnd::TableRow,
        Tag::TableCell => TagEnd::TableCell,
        Tag::BlockQuote(_) => TagEnd::BlockQuote(None),
        Tag::Emphasis => TagEnd::Emphasis,
        Tag::Strong => TagEnd::Strong,
        Tag::Strikethrough => TagEnd::Strikethrough,
        Tag::Link { .. } => TagEnd::Link,
        Tag::Image { .. } => TagEnd::Image,
        _ => TagEnd::Paragraph,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn renders_paragraph() {
        let theme = Theme::default_dark();
        let (lines, raw) = render("Hello world", &theme);
        assert_eq!(lines.len(), 1);
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0], "Hello world");
    }

    #[test]
    fn renders_bold_and_italic() {
        let theme = Theme::default_dark();
        let (lines, raw) = render("**bold** and *italic*", &theme);
        assert_eq!(raw[0], "bold and italic");
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_headings() {
        let theme = Theme::default_dark();
        let (lines, raw) = render("# H1\n## H2\n### H3", &theme);
        assert_eq!(raw.len(), 5);
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_link() {
        let theme = Theme::default_dark();
        let (lines, raw) = render("Click [here](https://example.com)", &theme);
        assert!(raw[0].contains("Click here"));
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_reference_link() {
        let theme = Theme::default_dark();
        let md = "Click [here][ref]\n\n[ref]: https://example.com";
        let (lines, raw) = render(md, &theme);
        assert!(raw[0].contains("Click here"));
        let rendered: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(rendered.contains("example.com"), "URL should appear in: {rendered}");
    }

    #[test]
    fn renders_nested_list() {
        let theme = Theme::default_dark();
        let md = "- outer\n  - inner\n- outer2\n";
        let (lines, _) = render(md, &theme);
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_blockquote_nested() {
        let theme = Theme::default_dark();
        let md = "> first\n>> second\n>>> third\n";
        let (lines, _) = render(md, &theme);
        assert!(lines.len() >= 1, "should have at least one line");
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_ordered_list() {
        let theme = Theme::default_dark();
        let (lines, _) = render("1. one\n2. two\n", &theme);
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_task_list() {
        let theme = Theme::default_dark();
        let md = "- [x] done\n- [ ] todo\n";
        let (lines, _) = render(md, &theme);
        let text: String = lines[0].spans.iter().filter_map(|s| {
            if s.content.contains('☑') || s.content.contains('☐') { Some(s.content.as_ref().to_string()) } else { None }
        }).collect();
        assert!(!text.is_empty(), "should contain task markers: got {lines:?}");
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_image() {
        let theme = Theme::default_dark();
        let (lines, _) = render("![alt](img.png)", &theme);
        assert_eq!(lines.len(), 1);
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("img.png"), "URL should appear: {text}");
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_code_block() {
        let theme = Theme::default_dark();
        let (lines, raw) = render("```rust\nfn main() {}\n```\n", &theme);
        assert!(raw.iter().any(|l| l.contains("fn main()")));
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_table() {
        let theme = Theme::default_dark();
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let (lines, _) = render(md, &theme);
        assert!(lines.len() >= 3);
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_table_alignment() {
        let theme = Theme::default_dark();
        let md = "| Left | Center | Right |\n|:-----|:------:|------:|\n| a | b | c |\n";
        let (lines, _) = render(md, &theme);

        let text: String = lines[3].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(
            text,
            "│a     │  b     │    c  │",
            "data row alignment: got '{text}'"
        );

        let border: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(
            border,
            "┌──────┬────────┬───────┐",
            "top border: got '{border}'"
        );
    }

    #[test]
    fn renders_complex_table() {
        let theme = Theme::default_dark();
        let md = "| Name  | Age | City    |\n|-------|-----|---------|\n| Alice | 30  | Madrid  |\n| Bob   | 25  | París   |\n| Carol | 35  | Roma    |\n";
        let (lines, _) = render(md, &theme);
        assert!(lines.len() >= 5, "should have border + header + border + rows + border, got {} lines", lines.len());
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_unordered_list() {
        let theme = Theme::default_dark();
        let (lines, _) = render("- item1\n- item2\n", &theme);
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_inline_html() {
        let theme = Theme::default_dark();
        let (lines, raw) = render("<div>hello</div>", &theme);
        assert_eq!(lines.len(), 1);
        assert!(raw[0].contains("hello"));
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_horizontal_rule() {
        let theme = Theme::default_dark();
        let (lines, _) = render("---\n", &theme);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn renders_strikethrough() {
        let theme = Theme::default_dark();
        let (lines, raw) = render("Hello ~~world~~", &theme);
        assert!(raw[0].contains("world"));
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_mixed_content_with_spacing() {
        let theme = Theme::default_dark();
        let md = "# Title\n\nHello world.\n\n> A quote.\n\n- item\n\n```\ncode\n```\n";
        let (lines, _) = render(md, &theme);
        // Title, blank, para, blank, quote, blank, list, blank, code
        assert!(lines.len() > 3, "should have spacing between sections: {} lines", lines.len());
        insta::assert_debug_snapshot!(lines);
    }

    #[test]
    fn renders_inline_code_bg() {
        let theme = Theme::default_dark();
        let (lines, _) = render("Use `code` here", &theme);
        let code_span = &lines[0].spans[1];
        assert!(code_span.style.bg.is_some(), "inline code should have bg color: {code_span:?}");
        insta::assert_debug_snapshot!(lines);
    }
}