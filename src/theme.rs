use std::collections::HashMap;

use crossterm::style::{Attribute, Color};
use serde::Deserialize;
use termimad::{CompoundStyle, LineStyle, MadSkin, StyledChar};

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
    pub fn load(name: &str) -> Option<Self> {
        if let Some(content) = get_built_in(name) {
            return Self::from_toml(content);
        }
        let dir = dirs::config_dir()?;
        let path = dir.join("markrender").join("themes").join(format!("{}.toml", name));
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

    pub fn apply_to_skin(&self, skin: &mut MadSkin) {
        for (key, def) in &self.styles {
            match key.as_str() {
                "paragraph" => self.apply_to_line_style(def, &mut skin.paragraph),
                "bold" => self.apply_to_compound(def, &mut skin.bold),
                "italic" => self.apply_to_compound(def, &mut skin.italic),
                "strikeout" => self.apply_to_compound(def, &mut skin.strikeout),
                "inline_code" => self.apply_to_compound(def, &mut skin.inline_code),
                "code_block" => self.apply_to_line_style(def, &mut skin.code_block),
                "heading1" => self.apply_to_line_style(def, &mut skin.headers[0]),
                "heading2" => self.apply_to_line_style(def, &mut skin.headers[1]),
                "heading3" => self.apply_to_line_style(def, &mut skin.headers[2]),
                "heading4" => self.apply_to_line_style(def, &mut skin.headers[3]),
                "heading5" => self.apply_to_line_style(def, &mut skin.headers[4]),
                "heading6" => self.apply_to_line_style(def, &mut skin.headers[5]),
                "table" => self.apply_to_line_style(def, &mut skin.table),
                "ellipsis" => self.apply_to_compound(def, &mut skin.ellipsis),
                "bullet" => self.apply_to_styled_char(def, &mut skin.bullet),
                "quote_mark" => self.apply_to_styled_char(def, &mut skin.quote_mark),
                "horizontal_rule" => self.apply_to_styled_char(def, &mut skin.horizontal_rule),
                _ => {}
            }
        }
    }

    fn resolve_color(&self, spec: &str) -> Option<Color> {
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
            Some(Color::Rgb { r, g, b })
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Color::Rgb { r, g, b })
        } else {
            None
        }
    }

    fn apply_to_compound(&self, def: &StyleDef, style: &mut CompoundStyle) {
        if let Some(ref fg) = def.fg {
            if let Some(c) = self.resolve_color(fg) {
                style.set_fg(c);
            }
        }
        if let Some(ref bg) = def.bg {
            if let Some(c) = self.resolve_color(bg) {
                style.set_bg(c);
            }
        }
        if let Some(v) = def.bold {
            if v {
                style.add_attr(Attribute::Bold);
            } else {
                style.remove_attr(Attribute::Bold);
            }
        }
        if let Some(v) = def.italic {
            if v {
                style.add_attr(Attribute::Italic);
            } else {
                style.remove_attr(Attribute::Italic);
            }
        }
        if let Some(v) = def.underline {
            if v {
                style.add_attr(Attribute::Underlined);
            } else {
                style.remove_attr(Attribute::Underlined);
            }
        }
        if let Some(v) = def.strikethrough {
            if v {
                style.add_attr(Attribute::CrossedOut);
            } else {
                style.remove_attr(Attribute::CrossedOut);
            }
        }
    }

    fn apply_to_line_style(&self, def: &StyleDef, style: &mut LineStyle) {
        self.apply_to_compound(def, &mut style.compound_style);
    }

    fn apply_to_styled_char(&self, def: &StyleDef, ch: &mut StyledChar) {
        if let Some(ref fg) = def.fg {
            if let Some(c) = self.resolve_color(fg) {
                ch.set_fg(c);
            }
        }
        if let Some(ref bg) = def.bg {
            if let Some(c) = self.resolve_color(bg) {
                ch.set_bg(c);
            }
        }
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