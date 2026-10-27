//! System metrics telemetry engine (CPU and RAM monitoring).

use sysinfo::System;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub used_memory_gb: f32,
    pub total_memory_gb: f32,
}

pub fn get_current_metrics(sys: &mut System) -> SystemMetrics {
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let cpu_usage = sys.global_cpu_usage();
    let used_memory_gb = sys.used_memory() as f32 / (1024.0 * 1024.0 * 1024.0);
    let total_memory_gb = sys.total_memory() as f32 / (1024.0 * 1024.0 * 1024.0);

    SystemMetrics {
        cpu_usage,
        used_memory_gb,
        total_memory_gb,
    }
}
