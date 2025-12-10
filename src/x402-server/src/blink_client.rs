//! HTTP Client for internal communication with blink-server

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

    /// Proxy a request to blink-server
    pub async fn proxy_post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
        payment_token: Option<&str>,
    ) -> Result<R, reqwest::Error> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.post(&url).json(body);

        if let Some(token) = payment_token {
            request = request.header("X-Payment-Token", token);
        }

        request.send().await?.json().await
    }

    /// Proxy a GET request to blink-server
    pub async fn proxy_get<R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
    ) -> Result<R, reqwest::Error> {
        let url = format!("{}{}", self.base_url, path);
        self.client.get(&url).send().await?.json().await
    }

    /// Health check for blink-server
    pub async fn health_check(&self) -> Result<bool, reqwest::Error> {
        let url = format!("{}/internal/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}
