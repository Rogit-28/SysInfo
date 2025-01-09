use crate::models::GpuInfo;

pub struct GpuCollector { nvml: Option<nvml_wrapper::Nvml> }

impl GpuCollector {
    pub fn new() -> Self { Self { nvml: nvml_wrapper::Nvml::init().ok() } }
    pub fn collect(&self) -> Vec<GpuInfo> {
        let Some(nvml) = &self.nvml else { return vec![] };
        (0..nvml.device_count().unwrap_or(0)).filter_map(|i| {
            let dev = nvml.device_by_index(i).ok()?;
            Some(GpuInfo { index: i, name: dev.name().unwrap_or_default(), vendor: "NVIDIA".into(), ..Default::default() })
        }).collect()
    }
}
