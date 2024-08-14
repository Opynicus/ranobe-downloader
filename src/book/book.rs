use super::Opf;
use super::{toc::Ncx, Episode};
use crate::config::TEMPLATE;
use crate::CONFIG;
use crate::{Credential, Downloader};
use anyhow::Context;
use anyhow::Result;
use md5::{Digest, Md5};
use reqwest::Method;
use scraper::selectable::Selectable;
use scraper::{Html, Selector};
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::vec;
use std::{collections::HashMap, path::Path};
use tracing::{debug, error, info};
use walkdir::{DirEntry, WalkDir};
use zip::{result::ZipError, write::SimpleFileOptions};
pub struct Book {
    pub title: String,
    pub author: String,
    pub episodes: Vec<Episode>,
    pub save_path: PathBuf,
    pub illustration_urls: HashMap<String, String>,
    pub with_cover: bool,
}
// TODO: 引入信号量控制并发数
impl Book {
    pub fn new() -> Self {
        // let save_path = Path::new(&CONFIG.esj_zone_config.esj_root_path).join(&title);
        Book {
            title: String::new(),
            author: String::new(),
            episodes: vec![],
            save_path: PathBuf::new(),
            illustration_urls: HashMap::new(),
            with_cover: false,
        }
    }

    async fn init_dir(&self) -> Result<()> {
        if let Err(_) = tokio::fs::metadata(&CONFIG.esj_zone_config.esj_root_path).await {
            info!("创建小说生成目录");
            tokio::fs::create_dir(&CONFIG.esj_zone_config.esj_root_path).await?;
        }
        if let Err(_) = tokio::fs::metadata(&CONFIG.esj_zone_config.esj_output_path).await {
            info!("创建输出目录");
            tokio::fs::create_dir(&CONFIG.esj_zone_config.esj_output_path).await?;
        }
        if let Ok(_) = tokio::fs::metadata(&self.save_path).await {
            info!("《{}》目录已存在，删除", self.title);
            tokio::fs::remove_dir_all(&self.save_path).await?;
        }
        tokio::fs::create_dir(&self.save_path).await?;
        let meta_inf_path = Path::new(&self.save_path).join("META-INF");
        let oebps_path = Path::new(&self.save_path).join("OEBPS");
        let mimetype_path = Path::new(&self.save_path).join("mimetype");
        let create_meta_inf_path_task =
            tokio::spawn(async move { tokio::fs::create_dir(meta_inf_path).await });
        let create_oebps_path_task =
            tokio::spawn(async move { tokio::fs::create_dir(oebps_path).await });
        let create_mimetype_task = tokio::spawn(async move {
            let _ = tokio::fs::write(mimetype_path, "application/epub+zip".as_bytes()).await;
        });
        let _ = tokio::join!(
            create_meta_inf_path_task,
            create_oebps_path_task,
            create_mimetype_task
        );

        let container_path = Path::new(&self.save_path)
            .join("META-INF")
            .join("container.xml");
        let opf: String = Opf::new(&self).content()?;
        let opf_path = Path::new(&self.save_path).join("OEBPS").join("content.opf");
        let toc = Ncx::new(&self.title, &self.author, &self.episodes).content()?;
        let toc_path = Path::new(&self.save_path).join("OEBPS").join("toc.ncx");
        let fonts_path = Path::new(&self.save_path).join("OEBPS").join("FONTS");
        let image_path = Path::new(&self.save_path).join("OEBPS").join("Images");
        let style_path = Path::new(&self.save_path).join("OEBPS").join("STYLES");
        let text_path = Path::new(&self.save_path).join("OEBPS").join("Text");

        let create_mimetype_task = tokio::spawn(async move {
            tokio::fs::copy("./template/container.xml", container_path).await
        });

        let create_toc_task = tokio::spawn(async move { tokio::fs::write(toc_path, toc).await });

        let create_opf_task = tokio::spawn(async move { tokio::fs::write(opf_path, opf).await });

        let create_fonts_path_task =
            tokio::spawn(async move { tokio::fs::create_dir(fonts_path).await });
        let create_images_path_task =
            tokio::spawn(async move { tokio::fs::create_dir(image_path).await });
        let create_style_path_task =
            tokio::spawn(async move { tokio::fs::create_dir(style_path).await });
        let create_text_path_task =
            tokio::spawn(async move { tokio::fs::create_dir(text_path).await });

        let _ = tokio::join!(
            create_mimetype_task,
            create_toc_task,
            create_opf_task,
            create_fonts_path_task,
            create_images_path_task,
            create_style_path_task,
            create_text_path_task
        );
        info!(
            "《{}》初始化完成， 路径为: {}",
            self.title,
            self.save_path.to_str().unwrap()
        );
        Ok(())
    }

    async fn save_episodes(&self) -> Result<()> {
        info!("开始保存小说《{}》章节", self.title);
        let title_page_path = Path::new(&self.save_path)
            .join("OEBPS")
            .join("Text")
            .join("titlepage.xhtml");

        let create_episode_tasks: Vec<_> = self
            .episodes
            .iter()
            .map(|episode| {
                debug!("开始保存《{}》- {}", self.title, episode.episode_title);
                let episode_path = Path::new(&self.save_path)
                    .join("OEBPS")
                    .join("Text")
                    .join(format!("{}{}", episode.order, ".xhtml"));

                let content = self.illustration_urls.iter().fold(
                    episode.episode(),
                    |acc, (url, illustration_name)| {
                        acc.replace(url, &format!("../Images/{}", illustration_name))
                    },
                );
                tokio::fs::write(episode_path, content)
            })
            .collect();

        let create_title_page_task = tokio::spawn(async move {
            tokio::fs::copy("./template/titlepage.xhtml", title_page_path).await
        });
        let _ = create_title_page_task.await?;
        for create_episode_task in create_episode_tasks {
            create_episode_task.await?;
        }
        info!("《{}》全章节保存完毕", self.title);
        Ok(())
    }

    fn update_illustration_urls(&mut self) {
        let content_selector = Selector::parse(r#"div[class="forum-content mt-3"]"#)
            .expect("Failed to parse content selector");
        let img_selector = Selector::parse("img").expect("Failed to parse image selector");
        self.episodes.iter().for_each(|episode| {
            let doc = Html::parse_document(&episode.content);
            doc.select(&content_selector)
                .flat_map(|content| content.select(&img_selector))
                .for_each(|img| {
                    if let Some(illustration_url) = img.value().attr("src") {
                        let res = {
                            let mut hasher = Md5::new();
                            hasher.update(illustration_url);
                            hex::encode(hasher.finalize()) + ".jpg"
                        };
                        self.illustration_urls
                            .entry(illustration_url.to_string())
                            .or_insert(res);
                    }
                });
        })
    }

    async fn fetch_book(&mut self, url: &str, credential: Option<&Credential>) -> Result<()> {
        let body = Downloader::new()
            .fetch_esj(Method::GET, url, credential)
            .await?
            .text()
            .await?;

        let doc = Html::parse_document(&body);
        let title_selector = Selector::parse(r#"h2[class="p-t-10 text-normal"]"#)
            .expect("Failed to parse title selector");
        let author_selector = Selector::parse(r#"ul[class="list-unstyled mb-2 book-detail"]"#)
            .expect("Failed to parse title selector");
        let episode_list_selector = Selector::parse(r#"div[id="chapterList"]"#)
            .expect("Failed to parse episode list selector");
        let a_selector = Selector::parse("a").expect("Failed to parse a tag selector");
        let cover_selector = Selector::parse(r#"div[class="product-gallery text-center mb-3"]"#)
            .expect("Failed to parse cover selector");

        if let Some(title) = doc.select(&title_selector).next() {
            self.title = title.text().collect::<String>();
        }

        if let Some(cover_url) = doc
            .select(&cover_selector)
            .flat_map(|a_elem| a_elem.select(&a_selector))
            .next()
        {
            let cover_url = cover_url
                .value()
                .attr("href")
                .expect("no href tag")
                .to_string();
            self.illustration_urls
                .entry(cover_url)
                .or_insert("cover.jpg".to_string());
            self.with_cover = true;
        }

        if let Some(author) = doc
            .select(&author_selector)
            .flat_map(|ul_elem| ul_elem.select(&a_selector))
            .next()
        {
            self.author = author.text().collect::<String>();
        }

        self.save_path = Path::new(&CONFIG.esj_zone_config.esj_root_path).join(&self.title);
        let episode_list: Vec<String> = doc
            .select(&episode_list_selector)
            .flat_map(|div_elem| div_elem.select(&a_selector))
            .filter_map(|a_elem| a_elem.value().attr("href").map(|url| url.to_string()))
            .collect();

        let esj_credential = Arc::new(Credential {
            esj_key: CONFIG.esj_zone_config.ews_key.clone(),
            esj_token: CONFIG.esj_zone_config.ews_token.clone(),
        });
        let fetch_esj_episode_tasks: Vec<_> = episode_list
            .into_iter()
            .enumerate()
            .map(|(idx, episode_url)| {
                let credential = Arc::clone(&esj_credential);
                tokio::spawn(async move {
                    Episode::fetch_esj_episode(&episode_url, Some(&credential), idx as u32 + 1)
                        .await
                })
            })
            .collect();
        for fetch_esj_episode_task in fetch_esj_episode_tasks {
            let episode = fetch_esj_episode_task.await?;
            self.episodes
                .push(episode.expect("unable to fetch episode"));
        }
        Ok(())
    }

    async fn download_illustration(
        title: String,
        url: String,
        illustration_path: &PathBuf,
    ) -> Result<()> {
        let downloader = Downloader::new();
        info!(
            "正在下载《{}》中插画：{}",
            title,
            illustration_path.to_str().unwrap()
        );
        let content = downloader
            .fetch_esj(Method::GET, &url, None)
            .await?
            .bytes()
            .await?;
        let _ = tokio::fs::write(illustration_path, content).await?;
        info!(
            "下载《{}》中插画：{}完成",
            title,
            illustration_path.to_str().unwrap()
        );
        Ok(())
    }

    async fn save_illustration(&self) -> Result<()> {
        let base_path = Path::new(&self.save_path).join("OEBPS").join("Images");
        let save_tasks: Vec<_> = self
            .illustration_urls
            .iter()
            .map(|(url, illustration_name)| {
                let url = url.clone();
                let title = self.title.clone();
                let illustration_path = base_path.join(illustration_name);
                tokio::spawn(async move {
                    Book::download_illustration(title, url, &illustration_path).await
                })
            })
            .collect();
        if !self.with_cover {
            info!("《{}》封面不存在，使用默认封面替代", self.title);
            let cover_task = tokio::spawn(async move {
                tokio::fs::copy("./template/default_cover.jpg", base_path.join("cover.jpg")).await
            });
            let _ = cover_task.await?;
        }
        info!("开始下载《{}》插画", self.title);
        for save_task in save_tasks {
            // 这样写方便对每个task单独处理？
            let _ = save_task.await?;
        }
        info!("下载《{}》插画完成", self.title);
        Ok(())
    }

    fn make_epub(&self) -> Result<()> {
        info!("开始《{}》epub文件打包", self.title);
        let src_dir = Path::new(&CONFIG.esj_zone_config.esj_root_path).join(&self.title);
        let dst_file =
            Path::new(&CONFIG.esj_zone_config.esj_output_path).join(format!("{}.epub", self.title));
        if !Path::new(&src_dir).is_dir() {
            return Err(ZipError::FileNotFound.into());
        }

        let dst_path = Path::new(&dst_file);

        let mut epub_writer = zip::ZipWriter::new(std::fs::File::create(dst_path).unwrap());
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        let mut buffer = Vec::new();
        for entry in WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.strip_prefix(&src_dir).unwrap();
            let epub_inner_path = path
                .strip_prefix(&src_dir)
                .unwrap()
                .to_str()
                .map(str::to_owned)
                .with_context(|| format!("{name:?} Is a Non UTF-8 Path"))?;

            match path.is_file() {
                true => {
                    epub_writer.start_file(epub_inner_path, options)?;
                    let mut f = std::fs::File::open(path)?;
                    f.read_to_end(&mut buffer)?;
                    epub_writer.write_all(&buffer)?;
                    buffer.clear();
                }
                false => {
                    if !name.as_os_str().is_empty() {
                        epub_writer.add_directory(epub_inner_path, options)?;
                    }
                }
            }
        }
        epub_writer.finish()?;
        info!("《{}》打包完成", self.title);
        Ok(())
    }

    async fn create_epub(&mut self, url: &String, credential: Option<&Credential>) -> Result<()> {
        let _ = self.fetch_book(&url, credential).await?;
        self.update_illustration_urls();
        let _ = self.init_dir().await?;
        let _ = self.save_episodes().await?;
        let _ = self.save_illustration().await?;
        self.make_epub()?;
        Ok(())
    }

    pub async fn gen_epub(url: &String, credential: Option<&Credential>) -> Result<()> {
        let mut book = Book::new();
        let _ = book.create_epub(url, credential).await?;
        Ok(())
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
        let episode = Episode {
            episode_title: "设定总和".to_string(),
            content: "cnm".to_string(),
            episode_save_path: "./Text/1.xhtml".to_string(),
            order: 1,
        };
        let episode2 = Episode {
            episode_title: "第一章".to_string(),
            content: "nmsl".to_string(),
            episode_save_path: "./Text/2.xhtml".to_string(),
            order: 1,
        };
        let episodes = vec![episode, episode2];
        let mut book = Book {
            title: "下北泽秘闻".to_string(),
            author: "野兽先生".to_string(),
            episodes: episodes,
            save_path: Path::new("./esjNovelGen").join("下北泽秘闻"),
            illustration_urls: HashMap::new(),
            with_cover: false,
        };
        let esj_credential = Arc::new(Credential {
            esj_key: CONFIG.esj_zone_config.ews_key.clone(),
            esj_token: CONFIG.esj_zone_config.ews_token.clone(),
        });
        let credential = Arc::clone(&esj_credential);
        let _ = book
            .fetch_book(
                "https://www.esjzone.me/detail/1719148048.html",
                Some(&*credential),
            )
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn fuck_illustration_urls() -> Result<()> {
        let episode = Episode {
            episode_title: "设定总和".to_string(),
            content: "cnm".to_string(),
            episode_save_path: "./Text/1.xhtml".to_string(),
            order: 1,
        };
        let esj_credential = Arc::new(Credential {
            esj_key: CONFIG.esj_zone_config.ews_key.clone(),
            esj_token: CONFIG.esj_zone_config.ews_token.clone(),
        });
        let credential = Arc::clone(&esj_credential);

        let _ = Episode::fetch_esj_episode(
            "https://www.esjzone.me/forum/1696518058/180636.html",
            Some(&*credential),
            1,
        )
        .await?;
        let episodes = vec![episode];
        let mut book = Book {
            title: "下北泽秘闻".to_string(),
            author: "野兽先生".to_string(),
            episodes: episodes,
            save_path: Path::new("./esjNovelGen").join("下北泽秘闻"),
            illustration_urls: HashMap::new(),
            with_cover: false,
        };
        book.update_illustration_urls();
        info!("{:?}", &book.illustration_urls);
        book.save_illustration().await?;
        Ok(())
    }

    #[tokio::test]
    async fn fuck_save_episodes() -> Result<()> {
        let esj_credential = Arc::new(Credential {
            esj_key: CONFIG.esj_zone_config.ews_key.clone(),
            esj_token: CONFIG.esj_zone_config.ews_token.clone(),
        });
        let credential = Arc::clone(&esj_credential);
        let mut book = Book::new();
        let _ = book
            .fetch_book(
                // "https://www.esjzone.me/detail/1696518058.html",
                "https://www.esjzone.cc/detail/1718674070.html",
                Some(&credential),
            )
            .await?;
        book.update_illustration_urls();
        let _ = book.init_dir().await?;
        let _ = book.save_episodes().await?;
        let _ = book.save_illustration().await?;
        book.make_epub()?;
        // info!("{:?}", &book.illustration_urls);
        // book.save_illustration().await?;
        Ok(())
    }

    #[tokio::test]
    async fn fuck_init_dir() -> Result<()> {
        let episode = Episode {
            episode_title: "设定总和".to_string(),
            content: "cnm".to_string(),
            episode_save_path: "./Text/1.xhtml".to_string(),
            order: 1,
        };
        let esj_credential = Arc::new(Credential {
            esj_key: CONFIG.esj_zone_config.ews_key.clone(),
            esj_token: CONFIG.esj_zone_config.ews_token.clone(),
        });
        let credential = Arc::clone(&esj_credential);

        let _ = Episode::fetch_esj_episode(
            "https://www.esjzone.me/forum/1696518058/180636.html",
            Some(&*credential),
            1,
        )
        .await?;
        let episodes = vec![episode];
        let book = Book {
            title: "下北泽秘闻".to_string(),
            author: "野兽先生".to_string(),
            episodes: episodes,
            save_path: Path::new("./esjNovelGen").join("下北泽秘闻"),
            illustration_urls: HashMap::new(),
            with_cover: false,
        };
        book.init_dir().await?;
        Ok(())
    }
}
