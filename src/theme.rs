use std::collections::HashMap;

use ratatui::style::{Color, Style};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
struct StyleDef {
    fg: Option<String>,
    bg: Option<String>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    strikethrough: Option<bool>,
}

#[derive(Deserialize)]
struct ThemeFile {
    colors: Option<HashMap<String, String>>,
    styles: HashMap<String, StyleDef>,
}

/// A named colour theme with text styles for markdown elements.
///
/// Themes are loaded from `.toml` files in either the built-in set or
/// the user's `~/.config/mdr/themes/` directory.
pub struct Theme {
    styles: HashMap<String, StyleDef>,
    colors: HashMap<String, Color>,
}

impl Theme {
    /// Create an empty theme with no styles defined.
    pub fn default() -> Self {
        Self {
            styles: HashMap::new(),
            colors: HashMap::new(),
        }
    }

    /// Built-in dark theme (VS Code–inspired palette).
    pub fn default_dark() -> Self {
        let mut t = Self::default();
        t.set("paragraph", Some("#d4d4d4"), None, false, false, false, false);
        t.set("bold", Some("#ffffff"), None, true, false, false, false);
        t.set("italic", Some("#e6b450"), None, false, true, false, false);
        t.set("strikeout", Some("#808080"), None, false, false, false, false);
        t.set("inline_code", Some("#ce9178"), Some("#2d2d2d"), false, false, false, false);
        t.set("code_block", Some("#d4d4d4"), Some("#1e1e1e"), false, false, false, false);
        t.set("heading1", Some("#f44747"), None, true, false, false, false);
        t.set("heading2", Some("#569cd6"), None, true, false, false, false);
        t.set("heading3", Some("#4ec9b0"), None, true, false, false, false);
        t.set("heading4", Some("#dcdcaa"), None, false, false, false, false);
        t.set("heading5", Some("#9a9a9a"), None, false, false, false, false);
        t.set("heading6", Some("#808080"), None, false, false, false, false);
        t.set("link", Some("#569cd6"), None, false, false, true, false);
        t.set("table", Some("#808080"), None, false, false, false, false);
        t.set("bullet", Some("#569cd6"), None, false, false, false, false);
        t.set("quote_mark", Some("#6a9955"), None, false, false, false, false);
        t.set("horizontal_rule", Some("#404040"), None, false, false, false, false);
        t.set("ellipsis", Some("#404040"), None, false, false, false, false);
        t
    }

    /// Built-in light theme (GitHub–inspired palette).
    pub fn default_light() -> Self {
        let mut t = Self::default();
        t.set("paragraph", Some("#333333"), None, false, false, false, false);
        t.set("bold", Some("#000000"), None, true, false, false, false);
        t.set("italic", Some("#e88d4a"), None, false, true, false, false);
        t.set("strikeout", Some("#999999"), None, false, false, false, false);
        t.set("inline_code", Some("#c7254e"), Some("#f9f2f4"), false, false, false, false);
        t.set("code_block", Some("#333333"), Some("#f5f5f5"), false, false, false, false);
        t.set("heading1", Some("#d73a49"), None, true, false, false, false);
        t.set("heading2", Some("#005cc5"), None, true, false, false, false);
        t.set("heading3", Some("#22863a"), None, true, false, false, false);
        t.set("heading4", Some("#735c0f"), None, false, false, false, false);
        t.set("heading5", Some("#6a737d"), None, false, false, false, false);
        t.set("heading6", Some("#6a737d"), None, false, false, false, false);
        t.set("link", Some("#005cc5"), None, false, false, true, false);
        t.set("table", Some("#959da5"), None, false, false, false, false);
        t.set("bullet", Some("#005cc5"), None, false, false, false, false);
        t.set("quote_mark", Some("#22863a"), None, false, false, false, false);
        t.set("horizontal_rule", Some("#d1d5da"), None, false, false, false, false);
        t.set("ellipsis", Some("#d1d5da"), None, false, false, false, false);
        t
    }

    #[expect(clippy::too_many_arguments)]
    fn set(&mut self, key: &str, fg: Option<&str>, bg: Option<&str>, bold: bool, italic: bool, underline: bool, strikethrough: bool) {
        self.styles.insert(key.to_string(), StyleDef {
            fg: fg.map(String::from),
            bg: bg.map(String::from),
            bold: if bold { Some(true) } else { None },
            italic: if italic { Some(true) } else { None },
            underline: if underline { Some(true) } else { None },
            strikethrough: if strikethrough { Some(true) } else { None },
        });
    }

    /// Load a theme by name. Checks built-in themes first, then `~/.config/mdr/themes/{name}.toml`.
    pub fn load(name: &str) -> Option<Self> {
        if let Some(content) = get_built_in(name) {
            return Self::from_toml(content);
        }
        let dir = dirs::config_dir()?;
        let path = dir.join("mdr").join("themes").join(format!("{}.toml", name));
        let content = std::fs::read_to_string(path).ok()?;
        Self::from_toml(&content)
    }

    /// List all built-in theme names.
    pub fn list_names() -> Vec<&'static str> {
        vec![
            "ayu_dark",
            "ayu_light",
            "ayu_mirage",
            "catppuccin_mocha",
            "dracula",
            "gruvbox_dark",
            "nord",
            "onedark",
            "solarized_light",
            "tokyonight",
        ]
    }

    fn from_toml(content: &str) -> Option<Self> {
        let tf: ThemeFile = toml::from_str(content).ok()?;
        let mut resolved_colors = HashMap::new();
        if let Some(colors) = tf.colors {
            for (name, hex) in colors {
                if let Some(c) = Self::parse_hex(&hex) {
                    resolved_colors.insert(name, c);
                }
            }
        }
        Some(Theme {
            styles: tf.styles,
            colors: resolved_colors,
        })
    }

    /// Resolve a colour spec (hex like `#ff8800` or a named colour from the theme's `[colors]` table).
    pub fn resolve_color(&self, spec: &str) -> Option<Color> {
        if let Some(c) = self.colors.get(spec) {
            return Some(*c);
        }
        Self::parse_hex(spec)
    }

    fn parse_hex(hex: &str) -> Option<Color> {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Color::Rgb(r, g, b))
        } else {
            None
        }
    }

    /// Get style properties for a named element (e.g. `"paragraph"`, `"heading1"`, `"link"`).
    ///
    /// Returns `(fg, bg, bold, italic, underline, strikethrough)`.
    #[expect(clippy::type_complexity)]
    pub fn style_for(&self, key: &str) -> Option<(Option<Color>, Option<Color>, bool, bool, bool, bool)> {
        let def = self.styles.get(key)?;
        let fg = def.fg.as_ref().and_then(|c| self.resolve_color(c));
        let bg = def.bg.as_ref().and_then(|c| self.resolve_color(c));
        Some((fg, bg, def.bold.unwrap_or(false), def.italic.unwrap_or(false), def.underline.unwrap_or(false), def.strikethrough.unwrap_or(false)))
    }

    /// Resolve a style entry to a ratatui `Style`, or `None` if the key is not found.
    pub fn style_as_style(&self, key: &str) -> Option<Style> {
        let (fg, bg, bold, italic, underline, strikethrough) = self.style_for(key)?;
        let mut s = Style::default();
        if let Some(c) = fg { s = s.fg(c); }
        if let Some(c) = bg { s = s.bg(c); }
        if bold { s = s.add_modifier(ratatui::style::Modifier::BOLD); }
        if italic { s = s.add_modifier(ratatui::style::Modifier::ITALIC); }
        if underline { s = s.add_modifier(ratatui::style::Modifier::UNDERLINED); }
        if strikethrough { s = s.add_modifier(ratatui::style::Modifier::CROSSED_OUT); }
        Some(s)
    }

    /// Get the foreground colour for a named element.
    pub fn fg_for(&self, key: &str) -> Option<Color> {
        let def = self.styles.get(key)?;
        def.fg.as_ref().and_then(|c| self.resolve_color(c))
    }
}

fn get_built_in(name: &str) -> Option<&'static str> {
    match name {
        "onedark" => Some(include_str!("../themes/onedark.toml")),
        "catppuccin_mocha" => Some(include_str!("../themes/catppuccin_mocha.toml")),
        "dracula" => Some(include_str!("../themes/dracula.toml")),
        "gruvbox_dark" => Some(include_str!("../themes/gruvbox_dark.toml")),
        "nord" => Some(include_str!("../themes/nord.toml")),
        "solarized_light" => Some(include_str!("../themes/solarized_light.toml")),
        "tokyonight" => Some(include_str!("../themes/tokyonight.toml")),
        "ayu_dark" => Some(include_str!("../themes/ayu_dark.toml")),
        "ayu_mirage" => Some(include_str!("../themes/ayu_mirage.toml")),
        "ayu_light" => Some(include_str!("../themes/ayu_light.toml")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dark_has_paragraph_style() {
        let t = Theme::default_dark();
        assert!(t.style_for("paragraph").is_some());
    }

    #[test]
    fn default_dark_has_all_headings() {
        let t = Theme::default_dark();
        for i in 1..=6 {
            let key = format!("heading{}", i);
            assert!(t.style_for(&key).is_some(), "missing {key}");
        }
    }

    #[test]
    fn default_light_has_paragraph_style() {
        let t = Theme::default_light();
        assert!(t.style_for("paragraph").is_some());
    }

    #[test]
    fn list_names_returns_expected_count() {
        let names = Theme::list_names();
        assert_eq!(names.len(), 10);
    }

    #[test]
    fn resolve_color_hex_6digit() {
        let t = Theme::default();
        let c = t.resolve_color("#ff8800");
        assert_eq!(c, Some(Color::Rgb(255, 136, 0)));
    }

    #[test]
    fn resolve_color_hex_3digit() {
        let t = Theme::default();
        let c = t.resolve_color("#f80");
        assert_eq!(c, Some(Color::Rgb(255, 136, 0)));
    }

    #[test]
    fn resolve_color_hex_without_hash() {
        let t = Theme::default();
        let c = t.resolve_color("ff8800");
        assert_eq!(c, Some(Color::Rgb(255, 136, 0)));
    }

    #[test]
    fn resolve_color_invalid_returns_none() {
        let t = Theme::default();
        assert!(t.resolve_color("not a color").is_none());
    }

    #[test]
    fn resolve_color_named_in_empty_theme_parses_as_hex() {
        let t = Theme::default();
        // Without a [colors] table, any name is tried as hex
        assert!(t.resolve_color("abcxyz").is_none());
    }

    #[test]
    fn fg_for_known_key() {
        let t = Theme::default_dark();
        let fg = t.fg_for("paragraph");
        assert!(fg.is_some(), "paragraph should have a foreground color");
    }

    #[test]
    fn fg_for_unknown_key_returns_none() {
        let t = Theme::default_dark();
        assert!(t.fg_for("nonexistent_key").is_none());
    }

    #[test]
    fn style_for_unknown_key_returns_none() {
        let t = Theme::default();
        assert!(t.style_for("nothing").is_none());
    }
}