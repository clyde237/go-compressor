//! System metrics telemetry engine (CPU and RAM monitoring).

use std::sync::Mutex;
use sysinfo::System;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub used_memory_gb: f32,
    pub total_memory_gb: f32,
}

pub struct MetricsState(pub Mutex<System>);

impl Default for MetricsState {
    fn default() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self(Mutex::new(sys))
    }
}

pub fn get_current_metrics(sys: &mut System) -> SystemMetrics {
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let cpu_usage = sys.global_cpu_usage().clamp(0.0, 100.0);
    let used_memory_gb = (sys.used_memory() as f32 / (1024.0 * 1024.0 * 1024.0)).max(0.0);
    let total_memory_gb = (sys.total_memory() as f32 / (1024.0 * 1024.0 * 1024.0)).max(0.0);

    SystemMetrics {
        cpu_usage,
        used_memory_gb,
        total_memory_gb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_retrieval() {
        let mut sys = System::new();
        let metrics = get_current_metrics(&mut sys);
        assert!(metrics.total_memory_gb >= 0.0);
        assert!(metrics.used_memory_gb >= 0.0);
        assert!(metrics.cpu_usage >= 0.0);
    }
}
