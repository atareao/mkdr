use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn parse_ansi_line(s: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut style = Style::default();
    let mut buf = String::new();
    let mut chars = s.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch == '\x1b' {
            chars.next();
            if chars.next() == Some('[') {
                if !buf.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut buf), style));
                }

                let mut params = String::new();
                loop {
                    match chars.next() {
                        Some('m') => break,
                        Some(c) => params.push(c),
                        None => break,
                    }
                }

                style = apply_ansi_seq(&params, style);
                continue;
            }
            buf.push('\x1b');
        } else {
            buf.push(chars.next().unwrap());
        }
    }

    if !buf.is_empty() {
        spans.push(Span::styled(buf, style));
    }

    Line::from(spans)
}

pub fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch == '\x1b' {
            chars.next();
            if chars.next() == Some('[') {
                while let Some(c) = chars.next() {
                    if c == 'm' {
                        break;
                    }
                }
            }
        } else {
            result.push(chars.next().unwrap());
        }
    }
    result
}

fn apply_ansi_seq(params: &str, mut style: Style) -> Style {
    if params.is_empty() || params == "0" {
        return Style::default();
    }

    let parts: Vec<&str> = params.split(';').collect();
    let mut i = 0;

    while i < parts.len() {
        match parts[i] {
            "0" => style = Style::default(),
            "1" => style = style.add_modifier(Modifier::BOLD),
            "2" => style = style.add_modifier(Modifier::DIM),
            "3" => style = style.add_modifier(Modifier::ITALIC),
            "4" => style = style.add_modifier(Modifier::UNDERLINED),
            "22" => style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            "23" => style = style.remove_modifier(Modifier::ITALIC),
            "24" => style = style.remove_modifier(Modifier::UNDERLINED),
            "38" if i + 2 < parts.len() && parts[i + 1] == "5" => {
                if let Ok(n) = parts[i + 2].parse() {
                    style = style.fg(Color::Indexed(n));
                }
                i += 2;
            }
            "48" if i + 2 < parts.len() && parts[i + 1] == "5" => {
                if let Ok(n) = parts[i + 2].parse() {
                    style = style.bg(Color::Indexed(n));
                }
                i += 2;
            }
            "39" => style = style.fg(Color::Reset),
            "49" => style = style.bg(Color::Reset),
            s => {
                if let Some(c) = fg_color(s) {
                    style = style.fg(c);
                } else if let Some(c) = bg_color(s) {
                    style = style.bg(c);
                }
            }
        }
        i += 1;
    }

    style
}

fn fg_color(s: &str) -> Option<Color> {
    match s {
        "30" => Some(Color::Black),
        "31" => Some(Color::Red),
        "32" => Some(Color::Green),
        "33" => Some(Color::Yellow),
        "34" => Some(Color::Blue),
        "35" => Some(Color::Magenta),
        "36" => Some(Color::Cyan),
        "37" => Some(Color::Gray),
        "90" => Some(Color::DarkGray),
        "91" => Some(Color::LightRed),
        "92" => Some(Color::LightGreen),
        "93" => Some(Color::LightYellow),
        "94" => Some(Color::LightBlue),
        "95" => Some(Color::LightMagenta),
        "96" => Some(Color::LightCyan),
        "97" => Some(Color::White),
        _ => None,
    }
}

fn bg_color(s: &str) -> Option<Color> {
    match s {
        "40" => Some(Color::Black),
        "41" => Some(Color::Red),
        "42" => Some(Color::Green),
        "43" => Some(Color::Yellow),
        "44" => Some(Color::Blue),
        "45" => Some(Color::Magenta),
        "46" => Some(Color::Cyan),
        "47" => Some(Color::Gray),
        "100" => Some(Color::DarkGray),
        "101" => Some(Color::LightRed),
        "102" => Some(Color::LightGreen),
        "103" => Some(Color::LightYellow),
        "104" => Some(Color::LightBlue),
        "105" => Some(Color::LightMagenta),
        "106" => Some(Color::LightCyan),
        "107" => Some(Color::White),
        _ => None,
    }
}