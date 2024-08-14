use std::fs;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_yaml;

#[derive(Serialize, Deserialize, Debug)]
pub struct Template {
    pub toc_prefix: String,
    pub toc_verison: String,
    pub toc_xmlns: String,
    pub toc_meta_content: String,
    pub toc_meta_name: String,
    pub episode_prefix: String,
}

impl Default for Template {
    fn default() -> Self {
        Template {
            toc_prefix: String::new(),
            toc_verison: String::new(),
            toc_xmlns: String::new(),
            toc_meta_content: String::new(),
            toc_meta_name: String::new(),
            episode_prefix: String::new(),
        }
    }
}

impl Template {
    pub fn load(&self) -> Result<Self> {
        let content = fs::read_to_string("config/template.yaml")?;
        Ok(serde_yaml::from_str(&content)?)
    }
}
