use crate::{config::TEMPLATE, Credential, Downloader};
use anyhow::{Ok, Result};
use reqwest::Method;
use scraper::{selectable::Selectable, Html, Selector};

pub struct Episode {
    pub episode_title: String,
    pub content: String,
    pub episode_save_path: String,
    pub order: u32,
}

impl Episode {
    pub fn new() -> Self {
        Episode {
            episode_title: String::new(),
            content: String::new(),
            episode_save_path: String::new(),
            order: 0,
        }
    }

    pub fn episode(&self) -> String {
        format!(
            "{}<head>
  <title>{}</title>
</head><body>
  <h1>{}</h1>{}</body>",
            TEMPLATE.episode_prefix, self.episode_title, self.episode_title, self.content
        )
    }

    pub async fn fetch_esj_episode(
        url: &str,
        credential: Option<&Credential>,
        order: u32,
    ) -> Result<Self> {
        let body = Downloader::new()
            .fetch_esj(Method::GET, &url, credential)
            .await?
            .text()
            .await?;
        let doc = Html::parse_document(&body);

        let episode_title_selector = Selector::parse(r#"div[class="col-xl-9 col-lg-8 p-r-30"]"#)
            .expect("Failed to parse episode title selector");
        let h2_selector = Selector::parse("h2").expect("Failed to parse h2 tag selector");
        let content_block_selector = Selector::parse(r#"div[class="forum-content mt-3"]"#)
            .expect("Failed to parse content blocker selector");
        let episode_title = doc
            .select(&episode_title_selector)
            .flat_map(|episode_title_elem| episode_title_elem.select(&h2_selector))
            .map(|h2_elem| h2_elem.text().collect::<String>())
            .next()
            .unwrap_or_default();

        let content = doc
            .select(&content_block_selector)
            .next()
            .map(|content_block_elem| content_block_elem.html())
            .unwrap_or_else(|| format!("{}: 无文本内容", episode_title));

        let episode_save_path = format!("Text/{}.xhtml", order);
        Ok(Episode {
            episode_title,
            content,
            episode_save_path,
            order,
        })
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Ok;

    use super::*;
    use crate::CONFIG;
    use std::sync::Arc;
    #[tokio::test]
    async fn fuck_episodes() -> Result<()> {
        let esj_credential = Arc::new(Credential {
            esj_key: CONFIG.esj_zone_config.ews_key.clone(),
            esj_token: CONFIG.esj_zone_config.ews_token.clone(),
        });
        let credential = Arc::clone(&esj_credential);
        let _ = Episode::fetch_esj_episode(
            "https://www.esjzone.me/forum/1719148048/225492.html",
            Some(&*credential),
            1,
        )
        .await?;

        Ok(())
    }
}
