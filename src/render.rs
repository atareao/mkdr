use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

const BULLET_CHAR: char = '•';
const QUOTE_CHAR: char = '▐';

pub fn render(content: &str, theme: &Theme) -> (Vec<Line<'static>>, Vec<String>) {
    let renderer = Renderer::new(theme);
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
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

        loop {
            match parser.next() {
                Some(Event::Start(tag)) => match tag {
                    Tag::Paragraph => {
                        let spans = self.collect_inline(&mut parser, &TagEnd::Paragraph, &self.para);
                        raw_line(&spans, raw);
                        lines.push(Line::from(spans));
                    }
                    Tag::Heading { level, .. } => {
                        let idx = (level as usize).saturating_sub(1).min(5);
                        let hl = &self.headings[idx];
                        let spans = self.collect_inline(&mut parser, &TagEnd::Heading(level), hl);
                        raw_line(&spans, raw);
                        lines.push(Line::from(spans));
                    }
                    Tag::CodeBlock(kind) => {
                        if let CodeBlockKind::Fenced(info) = kind
                            && !info.is_empty()
                        {
                            raw.push(info.to_string());
                            lines.push(Line::from(Span::styled(
                                format!(" {} ", info),
                                Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                            )));
                        }
                        let code = self.collect_code(&mut parser);
                        for line_text in code.lines() {
                            raw.push(line_text.to_string());
                            lines.push(Line::from(Span::styled(
                                line_text.to_string(),
                                self.code_block.as_style(),
                            )));
                        }
                    }
                    Tag::List(start) => {
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
                    Tag::Table(_alignments) => {
                        in_table = true;
                        table_data = TableData::default();
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
                        let mark = Span::styled(
                            format!("{} ", QUOTE_CHAR),
                            Style::default().fg(self.quote_mark.unwrap_or(Color::DarkGray)),
                        );
                        let spans = self.collect_inline(&mut parser, &TagEnd::BlockQuote(None), &self.para);
                        raw_line(&spans, raw);
                        let mut quoted = vec![mark];
                        quoted.extend(spans);
                        lines.push(Line::from(quoted));
                    }
                    Tag::FootnoteDefinition(_) => {
                        let _ = self.skip_to(&mut parser, &TagEnd::FootnoteDefinition);
                    }
                    _ => {}
                },
                Some(Event::End(tag_end)) => match tag_end {
                    TagEnd::List(_) => {
                        list_counters.pop();
                    }
                    TagEnd::Table => {
                        if in_table {
                            in_table = false;
                            self.render_table(&table_data, lines, raw);
                        }
                    }
                    TagEnd::TableHead | TagEnd::TableRow | TagEnd::TableCell => {}
                    TagEnd::Paragraph => {}
                    _ => {}
                },
                Some(Event::Rule) => {
                    raw.push(String::new());
                    lines.push(Line::from(Span::styled(
                        "─".repeat(80),
                        Style::default().fg(self.rule.unwrap_or(Color::DarkGray)),
                    )));
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

    fn collect_inline(
        &self,
        events: &mut Parser<'_>,
        end_tag: &TagEnd,
        base: &ThemeStyle,
    ) -> Vec<Span<'static>> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut buf = String::new();

        loop {
            match events.next() {
                Some(Event::Start(tag)) => {
                    flush_buf(&mut buf, &mut spans, base);
                    match tag {
                        Tag::Emphasis => {
                            spans.extend(self.collect_inline(events, &TagEnd::Emphasis, &self.italic));
                        }
                        Tag::Strong => {
                            spans.extend(self.collect_inline(events, &TagEnd::Strong, &self.bold));
                        }
                        Tag::Strikethrough => {
                            spans.extend(self.collect_inline(events, &TagEnd::Strikethrough, &self.strike));
                        }
                        Tag::Link { .. } => {
                            let mut child = self.collect_inline(events, &TagEnd::Link, &self.link);
                            spans.append(&mut child);
                        }
                        Tag::CodeBlock(_) | Tag::Paragraph | Tag::Heading { .. } => {
                            // nested block inside inline (shouldn't happen, but just in case)
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
                    buf.push(' ');
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
            self.push_table_row(&data.headers, &col_widths, lines, raw, &b);
            lines.push(self.table_border_line(&col_widths, "├", "┼", "┤", &b));
            raw.push(String::new());
        }

        // Data rows
        for row in &data.rows {
            self.push_table_row(row, &col_widths, lines, raw, &b);
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
    ) {
        let mut spans = vec![Span::styled("│".to_string(), *style)];
        let mut raw_text = String::new();
        for (i, cell_spans) in cells.iter().enumerate() {
            let w = widths.get(i).copied().unwrap_or(3);
            let cell_width = cell_width(cell_spans);
            let pad = w.saturating_sub(cell_width);

            spans.extend(cell_spans.iter().cloned());
            // padding after content
            if pad > 0 {
                spans.push(Span::styled(" ".repeat(pad), Style::default()));
            }
            spans.push(Span::styled(" │".to_string(), *style));
            raw_text.push(' ');
            for s in cell_spans {
                raw_text.push_str(s.content.as_ref());
            }
            raw_text.push_str(&" ".repeat(pad + 1));
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
        assert_eq!(raw.len(), 3);
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
    fn renders_inline_code() {
        let theme = Theme::default_dark();
        let (lines, raw) = render("Use `code` here", &theme);
        assert_eq!(raw[0], "Use code here");
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
    fn renders_unordered_list() {
        let theme = Theme::default_dark();
        let (lines, raw) = render("- item1\n- item2\n", &theme);
        assert_eq!(raw.len(), 2);
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
}