use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Find all line indices in `raw_lines` that contain `query` (case-insensitive).
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

/// Find the next match after `current` in a sorted `results` list, wrapping around.
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

/// Find the previous match before `current` in a sorted `results` list, wrapping around.
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

/// Highlight occurrences of `query` in `line` by applying an inverted style to matches.
///
/// With `no_colour` set, the matches are still located and split into their own
/// spans, but no highlight style is applied: the split spans keep the original
/// span style, so the frame stays free of styling.
pub fn highlight_line(line: &Line<'static>, query: &str, no_colour: bool) -> Line<'static> {
    if query.is_empty() {
        return line.clone();
    }

    let query_lower = query.to_lowercase();
    let mut new_spans: Vec<Span<'static>> = Vec::new();

    for span in &line.spans {
        let content: &str = span.content.as_ref();
        let content_lower = content.to_lowercase();
        let base_style = span.style;
        let match_style = if no_colour {
            base_style
        } else {
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        };
        let mut start = 0;

        while let Some(pos) = content_lower[start..].find(&query_lower) {
            let abs_pos = start + pos;
            if abs_pos > 0 {
                new_spans.push(Span::styled(
                    content[start..abs_pos].to_string(),
                    base_style,
                ));
            }
            let match_end = abs_pos + query.len();
            new_spans.push(Span::styled(
                content[abs_pos..match_end].to_string(),
                match_style,
            ));
            start = match_end;
        }
        if start < content.len() {
            new_spans.push(Span::styled(content[start..].to_string(), base_style));
        }
    }

    Line::from(new_spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_lines_basic() {
        let lines = vec![
            "hello world".to_string(),
            "goodbye".to_string(),
            "HELLO again".to_string(),
        ];
        let results = search_lines(&lines, "hello");
        assert_eq!(results, vec![0, 2]);
    }

    #[test]
    fn test_search_lines_empty_query() {
        let lines = vec!["hello".to_string()];
        assert!(search_lines(&lines, "").is_empty());
    }

    #[test]
    fn test_search_lines_no_match() {
        let lines = vec!["hello".to_string()];
        assert!(search_lines(&lines, "zzz").is_empty());
    }

    #[test]
    fn test_find_next_match_forward() {
        let results = vec![1, 3, 5];
        assert_eq!(find_next_match(&results, Some(0)), Some(1));
        assert_eq!(find_next_match(&results, Some(1)), Some(3));
        assert_eq!(find_next_match(&results, Some(4)), Some(5));
        assert_eq!(find_next_match(&results, Some(5)), Some(1)); // wraps
    }

    #[test]
    fn test_find_next_match_empty() {
        assert_eq!(find_next_match(&[], Some(0)), None);
    }

    #[test]
    fn test_find_prev_match_backward() {
        let results = vec![1, 3, 5];
        assert_eq!(find_prev_match(&results, Some(0)), Some(5)); // wraps
        assert_eq!(find_prev_match(&results, Some(3)), Some(1));
        assert_eq!(find_prev_match(&results, Some(4)), Some(3));
        assert_eq!(find_prev_match(&results, Some(6)), Some(5));
    }

    #[test]
    fn test_find_prev_match_empty() {
        assert_eq!(find_prev_match(&[], Some(0)), None);
    }

    #[test]
    fn test_highlight_line_basic() {
        let line = Line::from(Span::raw("hello world"));
        let highlighted = highlight_line(&line, "world", false);
        let text: String = highlighted
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(text, "hello world");
        assert!(highlighted.spans.len() >= 2); // "hello " + "world"
    }

    #[test]
    fn test_highlight_line_multiple_matches() {
        let line = Line::from(Span::raw("foo bar foo"));
        let highlighted = highlight_line(&line, "foo", false);
        let text: String = highlighted
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(text, "foo bar foo");
    }

    #[test]
    fn test_highlight_line_empty_query() {
        let line = Line::from(Span::raw("hello"));
        let highlighted = highlight_line(&line, "", false);
        assert_eq!(highlighted.spans.len(), 1);
    }

    #[test]
    fn test_highlight_line_respects_no_colour_policy() {
        let line = Line::from(Span::raw("foo bar foo"));

        // With the policy active the matches are still located and split out,
        // but no highlight style is applied.
        let plain = highlight_line(&line, "foo", true);
        let plain_text: String = plain.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(plain_text, "foo bar foo");
        assert_eq!(
            plain.spans.len(),
            3,
            "matches should still split the line: {:?}",
            plain.spans
        );
        for span in &plain.spans {
            assert!(
                span.style.fg.is_none()
                    && span.style.bg.is_none()
                    && span.style.add_modifier.is_empty(),
                "no-colour highlight should not style anything: {:?}",
                span.style
            );
        }

        // Without the policy the matches keep the inverted highlight.
        let styled = highlight_line(&line, "foo", false);
        let styled_text: String = styled.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(styled_text, "foo bar foo");
        assert!(
            styled.spans.iter().any(|s| s.style.bg == Some(Color::Blue)),
            "without the policy the match should keep its highlight: {:?}",
            styled.spans
        );
    }
}
