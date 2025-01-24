use crate::config::InfluxDbConfig;
use crate::models::*;

pub struct InfluxDbWriter {
    enabled: bool,
    client: Option<reqwest::Client>,
    url: String,
    token: String,
    org: String,
    bucket: String,
}

impl InfluxDbWriter {
    pub fn new(config: &InfluxDbConfig) -> Self {
        Self {
            enabled: config.enabled,
            client: if config.enabled { Some(reqwest::Client::new()) } else { None },
            url: config.url.clone(),
            token: config.token.clone(),
            org: config.org.clone(),
            bucket: config.bucket.clone(),
        }
    }

    pub async fn write_cpu(&self, _data: &CpuInfo) -> anyhow::Result<()> {
        if !self.enabled { return Ok(()); }
        Ok(())
    }
}
