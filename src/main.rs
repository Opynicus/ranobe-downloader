use std::sync::Arc;

use anyhow::Result;
use book::Book;
use config::CONFIG;
use downloader::Credential;
use once_cell::sync::Lazy;
mod book;
mod config;
mod downloader;
use crate::downloader::Downloader;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    Lazy::force(&CONFIG);
    let esj_credential = Arc::new(Credential {
        esj_key: CONFIG.esj_zone_config.ews_key.clone(),
        esj_token: CONFIG.esj_zone_config.ews_token.clone(),
    });
    for esj_url in &CONFIG.esj_zone_config.esj_novel_urls {
        let credential = Arc::clone(&esj_credential);
        Book::gen_epub(&esj_url, Some(&credential)).await?;
    }
    Ok(())
}
