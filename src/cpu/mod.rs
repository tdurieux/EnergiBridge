#[cfg(not(target_os = "macos"))]
mod amd;
#[cfg(target_os = "macos")]
mod apple;
#[cfg(not(target_os = "macos"))]
mod intel;

use std::collections::HashMap;
use sysinfo::{CpuExt, System, SystemExt};

#[cfg(target_os = "macos")]
use apple::get_apple_cpu_cunter;
#[cfg(not(target_os = "macos"))]
use intel::get_intel_cpu_cunter;

pub fn get_number_cores() -> Option<usize> {
    let sys = System::new();

    return sys.physical_core_count();
}

pub fn get_cpu_usage(results: &mut HashMap<String, f64>) {
    let mut sys = System::new();
    sys.refresh_cpu();

    for (i, cpu) in sys.cpus().iter().enumerate() {
        let key: String = format!("CPU_USAGE_{i}");
        results.insert(key, cpu.cpu_usage().into());
        let key: String = format!("CPU_FREQUENCY_{i}");
        results.insert(key, cpu.frequency() as f64);
    }
}

pub fn get_cpu_cunter(results: &mut HashMap<String, f64>) {
    #[cfg(target_os = "macos")]
    get_apple_cpu_cunter(results);
    #[cfg(not(target_os = "macos"))]
    amd::get_amd_cpu_cunter(results);
    #[cfg(not(target_os = "macos"))]
    get_intel_cpu_cunter(results);
}
