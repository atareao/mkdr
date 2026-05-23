use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn search_lines(raw_lines: &[String], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let query_lower = query.to_lowercase();
    raw_lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.to_lowercase().contains(&query_lower))
        .map(|(i, _)| i)
        .collect()
}

pub fn find_next_match(results: &[usize], current: Option<usize>) -> Option<usize> {
    if results.is_empty() {
        return None;
    }
    let cur = current.unwrap_or(0);
    match results.binary_search(&cur) {
        Ok(i) => Some(results[(i + 1) % results.len()]),
        Err(i) => {
            if i < results.len() {
                Some(results[i])
            } else {
                Some(results[0])
            }
        }
    }
}

pub fn find_prev_match(results: &[usize], current: Option<usize>) -> Option<usize> {
    if results.is_empty() {
        return None;
    }
    let cur = current.unwrap_or(0);
    match results.binary_search(&cur) {
        Ok(i) => {
            if i == 0 {
                Some(results[results.len() - 1])
            } else {
                Some(results[i - 1])
            }
        }
        Err(i) => {
            if i > 0 {
                Some(results[i - 1])
            } else {
                Some(results[results.len() - 1])
            }
        }
    }
}

pub fn highlight_line(line: &Line<'static>, query: &str) -> Line<'static> {
    if query.is_empty() {
        return line.clone();
    }

    let query_lower = query.to_lowercase();
    let mut new_spans: Vec<Span<'static>> = Vec::new();

    for span in &line.spans {
        let content: &str = span.content.as_ref();
        let content_lower = content.to_lowercase();
        let base_style = span.style;
        let mut start = 0;

        while let Some(pos) = content_lower[start..].find(&query_lower) {
            let abs_pos = start + pos;
            if abs_pos > 0 {
                new_spans
                    .push(Span::styled(content[start..abs_pos].to_string(), base_style));
            }
            let match_end = abs_pos + query.len();
            new_spans.push(Span::styled(
                content[abs_pos..match_end].to_string(),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ));
            start = match_end;
        }
        if start < content.len() {
            new_spans.push(Span::styled(content[start..].to_string(), base_style));
        }
    }

    Line::from(new_spans)
}