use anyhow::{Ok, Result};
use reqwest::{header, Client, Method, Response};

pub struct Downloader {
    pub client: Client,
}

pub struct Credential {
    pub esj_key: String,
    pub esj_token: String,
}

impl Downloader {
    pub fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
            ),
        );
        Downloader {
            client: reqwest::Client::builder()
                .default_headers(headers)
                .cookie_store(true)
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_esj(
        &self,
        method: Method,
        url: &str,
        credential: Option<&Credential>,
    ) -> Result<Response> {
        let mut request = self.client.request(method, url);
        if let Some(credential) = credential {
            request = request.header(
                header::COOKIE,
                format!(
                    "ews_key={};ews_token={};",
                    credential.esj_key, credential.esj_token
                ),
            )
        }
        Ok(request.send().await?)
    }
}
