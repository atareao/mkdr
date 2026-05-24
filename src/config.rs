use serde::Deserialize;

/// User-level configuration loaded from `~/.config/mdr/config.toml`.
///
/// All fields are optional; missing fields fall back to CLI args or defaults.
#[derive(Deserialize, Default)]
pub struct Config {
    /// Wrap mode override: "none", "word", or "char"
    pub wrap: Option<String>,
    /// Show line numbers in the gutter
    pub line_numbers: Option<bool>,
    /// Theme name (e.g. "onedark", "dracula")
    pub theme: Option<String>,
    /// Show the status bar
    pub show_status: Option<bool>,
}

/// Load configuration from `~/.config/mdr/config.toml`.
///
/// Returns [`Config::default()`] when the file does not exist or cannot be parsed.
pub fn load_config() -> Config {
    let config_dir = dirs::config_dir();
    let config_path = match config_dir {
        Some(mut d) => {
            d.push("mdr");
            let themes_dir = d.join("themes");
            let _ = std::fs::create_dir_all(&themes_dir);
            d.push("config.toml");
            d
        }
        None => return Config::default(),
    };

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Config::default(),
        Err(_) => {
            eprintln!("Warning: could not read config file '{}'", config_path.display());
            return Config::default();
        }
    };

    toml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Warning: could not parse config file '{}': {}", config_path.display(), e);
        Config::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_all_none() {
        let c = Config::default();
        assert!(c.wrap.is_none());
        assert!(c.line_numbers.is_none());
        assert!(c.theme.is_none());
        assert!(c.show_status.is_none());
    }

    #[test]
    fn config_parse_valid_toml() {
        let toml = r#"
wrap = "none"
line_numbers = true
theme = "dracula"
show_status = false
"#;
        let c: Config = toml::from_str(toml).expect("valid toml should parse");
        assert_eq!(c.wrap, Some("none".to_string()));
        assert_eq!(c.line_numbers, Some(true));
        assert_eq!(c.theme, Some("dracula".to_string()));
        assert_eq!(c.show_status, Some(false));
    }

    #[test]
    fn config_parse_empty_toml() {
        let c: Config = toml::from_str("").unwrap_or_default();
        assert!(c.wrap.is_none());
    }

    #[test]
    fn config_parse_partial() {
        let toml = r#"theme = "nord""#;
        let c: Config = toml::from_str(toml).expect("partial toml should parse");
        assert_eq!(c.theme, Some("nord".to_string()));
        assert!(c.wrap.is_none());
    }
}