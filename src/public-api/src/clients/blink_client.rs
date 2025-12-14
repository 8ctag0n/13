use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct BlinkClient {
    client: Client,
    base_url: String,
}

impl BlinkClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn proxy_get<R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
    ) -> Result<R, reqwest::Error> {
        let url = format!("{}{}", self.base_url, path);
        self.client.get(&url).send().await?.json().await
    }

    pub async fn proxy_post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, reqwest::Error> {
        let url = format!("{}{}", self.base_url, path);
        self.client.post(&url).json(body).send().await?.json().await
    }

    pub async fn health_check(&self) -> Result<bool, reqwest::Error> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}
