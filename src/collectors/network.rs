use crate::models::NetworkInfo;
use sysinfo::Networks;

pub struct NetworkCollector { networks: Networks }

impl NetworkCollector {
    pub fn new() -> Self { Self { networks: Networks::new_with_refreshed_list() } }
    pub fn collect(&mut self) -> Vec<NetworkInfo> {
        self.networks.refresh(true);
        self.networks.iter().map(|(name, data)| NetworkInfo {
            name: name.clone(), mac: Some(data.mac_address().to_string()),
            bytes_sent: data.total_transmitted(), bytes_received: data.total_received(),
            packets_sent: data.total_packets_transmitted(), packets_received: data.total_packets_received(),
            rx_errors: data.total_errors_on_received(), tx_errors: data.total_errors_on_transmitted(),
            ..Default::default()
        }).collect()
    }
}
