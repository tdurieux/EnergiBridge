#[cfg(not(target_os = "macos"))]
mod amd;
#[cfg(target_os = "macos")]
mod apple;
#[cfg(not(target_os = "macos"))]
mod intel;
#[cfg(not(target_os = "macos"))]
pub mod msr;

use std::collections::HashMap;
use sysinfo::{CpuExt, System, SystemExt};

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

#[cfg(not(target_os = "macos"))]
pub fn get_cpu_cunter(results: &mut HashMap<String, f64>) {
    let mut sys = System::new();
    sys.refresh_cpu();

    let vendor = sys.global_cpu_info().vendor_id();
    #[cfg(not(target_os = "macos"))]
    if vendor == "GenuineIntel" {
        intel::get_intel_cpu_cunter(results);
    } else if vendor == "AuthenticAMD" {
        amd::get_amd_cpu_cunter(results);
    }
}

#[cfg(target_os = "macos")]
pub fn get_cpu_cunter(results: &mut HashMap<String, f64>) {
    apple::get_apple_cpu_cunter(results);
}
