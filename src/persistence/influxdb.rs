use crate::config::InfluxDbConfig;

pub struct InfluxDbWriter { enabled: bool }

impl InfluxDbWriter {
    pub fn new(_config: &InfluxDbConfig) -> Self { Self { enabled: false } }
}
