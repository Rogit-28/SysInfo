use std::sync::Arc;
use crate::config::Settings;
use crate::collectors::CollectorRegistry;
use crate::persistence::InfluxDbWriter;

pub struct AppState {
    pub settings: Arc<Settings>,
    pub collectors: Arc<CollectorRegistry>,
    pub influxdb: Arc<InfluxDbWriter>,
}

impl AppState {
    pub fn new(settings: Arc<Settings>, collectors: Arc<CollectorRegistry>, influxdb: Arc<InfluxDbWriter>) -> Self {
        Self { settings, collectors, influxdb }
    }
}

pub async fn start_server(_state: AppState) -> anyhow::Result<()> {
    Ok(())
}
