use std::fs;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_yaml;
#[derive(Serialize, Deserialize, Debug)]
pub struct EsjZoneConfig {
    pub ews_key: String,
    pub ews_token: String,
    pub esj_root_path: String,
    pub esj_output_path: String,
    pub esj_novel_urls: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub esj_zone_config: EsjZoneConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            esj_zone_config: EsjZoneConfig {
                ews_key: String::new(),
                ews_token: String::new(),
                esj_root_path: String::new(),
                esj_output_path: String::new(),
                esj_novel_urls: vec![],
            },
        }
    }
}

impl Config {
    pub fn load(&self) -> Result<Self> {
        let content = fs::read_to_string("config/config.yaml")?;
        Ok(serde_yaml::from_str(&content)?)
    }
}
