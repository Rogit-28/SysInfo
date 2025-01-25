pub mod scheduler;
pub use scheduler::Scheduler;

mod scheduler {
    use std::sync::Arc;
    use crate::collectors::CollectorRegistry;
    use crate::persistence::InfluxDbWriter;
    use crate::config::Settings;

    pub struct Scheduler {
        collectors: Arc<CollectorRegistry>,
        influxdb: Arc<InfluxDbWriter>,
        settings: Arc<Settings>,
    }

    impl Scheduler {
        pub fn new(collectors: Arc<CollectorRegistry>, influxdb: Arc<InfluxDbWriter>, settings: Arc<Settings>) -> Self {
            Self { collectors, influxdb, settings }
        }
        pub async fn run(&self) { }
    }
}
