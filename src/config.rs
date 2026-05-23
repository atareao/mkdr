use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    pub wrap: Option<String>,
    pub line_numbers: Option<bool>,
    pub theme: Option<String>,
    pub show_status: Option<bool>,
}

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
        Err(_) => return Config::default(),
    };

    toml::from_str(&content).unwrap_or_default()
}