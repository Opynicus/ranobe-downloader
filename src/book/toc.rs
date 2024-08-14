use std::vec;

use anyhow::Result;
use quick_xml::se::to_string;
use serde;
use serde::{Deserialize, Serialize};

use crate::config::TEMPLATE;

use super::*;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "ncx")]
pub struct Ncx {
    #[serde(skip)]
    prefix: String,
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "@xmlns")]
    xmlns: String,
    head: Head,
    doc_title: Text,
    doc_author: Text,
    nav_map: NavMap,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct Head {
    meta: Vec<Meta>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct Meta {
    #[serde(rename = "@content")]
    content: String,
    #[serde(rename = "@name")]
    name: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct NavMap {
    nav_point: Vec<NavPoint>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct NavPoint {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@playOrder")]
    play_order: u32,
    nav_label: Text,
    content: Content,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Text {
    text: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    #[serde(rename = "@src")]
    src: String,
}

impl Text {
    fn new(content: &str) -> Self {
        Text {
            text: content.to_string(),
        }
    }
}

impl Content {
    fn new(content: &str) -> Self {
        Content {
            src: content.to_string(),
        }
    }
}

impl Ncx {
    pub fn new(title: &String, author: &String, episodes: &Vec<Episode>) -> Self {
        let mut nav_points = vec![NavPoint {
            id: "cover".to_string(),
            play_order: 0,
            nav_label: Text::new("封面"),
            content: Content::new("Text/titlepage.xhtml"),
        }];
        for (idx, episode) in episodes.iter().enumerate() {
            nav_points.push(NavPoint {
                id: format!("{}{}", "ep", idx as u32 + 1),
                play_order: idx as u32 + 1,
                nav_label: Text::new(&episode.episode_title),
                content: Content::new(&episode.episode_save_path),
            });
        }
        Ncx {
            prefix: TEMPLATE.toc_prefix.clone(),
            version: TEMPLATE.toc_verison.clone(),
            xmlns: TEMPLATE.toc_xmlns.clone(),
            head: Head {
                meta: vec![Meta {
                    content: TEMPLATE.toc_meta_content.clone(),
                    name: TEMPLATE.toc_meta_name.clone(),
                }],
            },
            doc_title: Text::new(&title),
            doc_author: Text::new(&author),
            nav_map: NavMap {
                nav_point: nav_points,
            },
        }
    }

    pub fn content(&self) -> Result<String> {
        let suffix = to_string(&self)?;
        let content = format!("{}{}", &self.prefix, &suffix);
        Ok(content)
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;

    use super::*;

    #[test]
    fn test_struct() -> Result<()> {
        let episode = Episode {
            episode_title: "设定总和".to_string(),
            content: "cnm".to_string(),
            episode_save_path: "Text/1.xhmtl".to_string(),
            order: 1,
        };
        let episode2 = Episode {
            episode_title: "第一章".to_string(),
            content: "nmsl".to_string(),
            episode_save_path: "Text/2.xhmtl".to_string(),
            order: 2,
        };
        let episodes = vec![episode, episode2];
        let ncx = Ncx::new(&"haha".to_string(), &"fufu".to_string(), &episodes);
        let res = ncx.content().unwrap();
        println!("{:?}", res);
        std::fs::write("./toc.ncx", res)?;
        Ok(())
    }
}
