use std::vec;

use anyhow::Result;
use quick_xml::se::to_string;
use serde;
use serde::{Deserialize, Serialize};


use super::Book;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "package")]
pub struct Opf {
    #[serde(skip)]
    prefix: String,
    #[serde(rename = "@xmlns")]
    xmlns: String,
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "@unique-identifier")]
    unique_identifier: String,
    meta_data: MetaData,
    manifest: Manifest,
    spine: Spine,
    guide: Guide,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaData {
    #[serde(rename = "@xmlns:opf")]
    xmlns_opf: String,
    #[serde(rename = "@xmlns:dc")]
    xmlns_dc: String,
    #[serde(rename = "@xmlns:dcterms")]
    xmlns_dcterms: String,
    #[serde(rename = "@xmlns:xsi")]
    xmlns_xsi: String,
    #[serde(rename = "@xmlns:calibre")]
    xmlns_calibre: String,
    #[serde(rename = "dc:title")]
    dc_title: String,
    #[serde(rename = "dc:creator")]
    dc_creator: String,
    meta: Meta,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@content")]
    content: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    item: Vec<Item>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@media-type")]
    media_type: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spine {
    #[serde(rename = "@toc")]
    toc: String,
    itemref: Vec<Itemref>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Itemref {
    #[serde(rename = "@idref")]
    idref: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Guide {
    reference: Reference,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    #[serde(rename = "@type")]
    r#type: String,
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@title")]
    title: String,
}

impl Opf {
    pub fn new(book: &Book) -> Self {
        let meta_data = MetaData {
            xmlns_opf: "http://www.idpf.org/2007/opf".to_string(),
            xmlns_dc: "http://purl.org/dc/elements/1.1/".to_string(),
            xmlns_dcterms: "http://purl.org/dc/terms/".to_string(),
            xmlns_xsi: "http://www.w3.org/2001/XMLSchema-instance".to_string(),
            xmlns_calibre: "http://calibre.kovidgoyal.net/2009/metadata".to_string(),
            dc_title: book.title.clone(),
            dc_creator: book.author.clone(),
            meta: Meta {
                name: "cover".to_string(),
                content: "cover.jpg".to_string(),
            },
        };
        let mut item = vec![];
        let mut itemref = vec![];
        item.push(Item {
            id: "titlepage.xhtml".to_string(),
            href: "Text/titlepage.xhtml".to_string(),
            media_type: "application/xhtml+xml".to_string(),
        });
        itemref.push(Itemref {
            idref: "titlepage.xhtml".to_string(),
        });
        book.episodes.iter().for_each(|episode| {
            item.push(Item {
                id: format!("{}.xhtml", &episode.order),
                href: episode.episode_save_path.clone(),
                media_type: "application/xhtml+xml".to_string(),
            });
            itemref.push(Itemref {
                idref: format!("{}.xhtml", &episode.order),
            })
        });
        book.illustration_urls
            .values()
            .enumerate()
            .for_each(|(idx, name)| {
                item.push(Item {
                    id: format!("added{}", idx as u32),
                    href: format!("Images/{}", name),
                    media_type: "image/jpeg".to_string(),
                })
            });
        item.push(Item {
            id: "ncx".to_string(),
            href: "toc.ncx".to_string(),
            media_type: "application/x-dtbncx+xml".to_string(),
        });
        let manifest = Manifest { item };
        let spine = Spine {
            toc: "ncx".to_string(),
            itemref,
        };
        let guide = Guide {
            reference: Reference {
                r#type: "cover".to_string(),
                href: "titlepage.xhtml".to_string(),
                title: "Cover".to_string(),
            },
        };
        Opf {
            prefix: "<?xml version=\"1.0\" encoding=\"UTF-8\"?>".to_string(),
            xmlns: "http://www.idpf.org/2007/opf".to_string(),
            version: "2.0".to_string(),
            unique_identifier: "uuid_id".to_string(),
            meta_data,
            manifest,
            spine,
            guide,
        }
    }

    pub fn content(&self) -> Result<String> {
        let suffix = to_string(&self)?;
        let content = format!("{}{}", &self.prefix, &suffix);
        Ok(content)
    }
}
