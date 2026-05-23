use std::collections::HashMap;

use ratatui::style::Color;
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

pub struct Theme {
    styles: HashMap<String, StyleDef>,
    colors: HashMap<String, Color>,
}

impl Theme {
    pub fn default() -> Self {
        Self {
            styles: HashMap::new(),
            colors: HashMap::new(),
        }
    }

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

    pub fn load(name: &str) -> Option<Self> {
        if let Some(content) = get_built_in(name) {
            return Self::from_toml(content);
        }
        let dir = dirs::config_dir()?;
        let path = dir.join("mdr").join("themes").join(format!("{}.toml", name));
        let content = std::fs::read_to_string(path).ok()?;
        Self::from_toml(&content)
    }

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

    #[expect(clippy::type_complexity)]
    pub fn style_for(&self, key: &str) -> Option<(Option<Color>, Option<Color>, bool, bool, bool, bool)> {
        let def = self.styles.get(key)?;
        let fg = def.fg.as_ref().and_then(|c| self.resolve_color(c));
        let bg = def.bg.as_ref().and_then(|c| self.resolve_color(c));
        Some((fg, bg, def.bold.unwrap_or(false), def.italic.unwrap_or(false), def.underline.unwrap_or(false), def.strikethrough.unwrap_or(false)))
    }

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