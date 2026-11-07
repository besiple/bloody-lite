use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONFIG_FILE: &str = "bloody_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub dpi: u32,
    pub windows_sensitivity: u32,
    pub button4_vk: Option<u16>, // None = Default (Back)
    pub button5_vk: Option<u16>, // None = Default (Forward)
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dpi: 800,
            windows_sensitivity: 10,
            button4_vk: None,
            button5_vk: None,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        if Path::new(CONFIG_FILE).exists() {
            if let Ok(content) = fs::read_to_string(CONFIG_FILE) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        let default_config = Self::default();
        let _ = default_config.save();
        default_config
    }

    pub fn save(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(CONFIG_FILE, json)
    }
}
