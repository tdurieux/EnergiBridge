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

pub fn get_number_cores(sys: &mut System) -> Option<usize> {
    return sys.physical_core_count();
}

pub fn get_cpu_usage(sys: &mut System, results: &mut HashMap<String, f64>) {
    sys.refresh_cpu();

    for (i, cpu) in sys.cpus().iter().enumerate() {
        let key: String = format!("CPU_USAGE_{i}");
        results.insert(key, cpu.cpu_usage().into());
        let key: String = format!("CPU_FREQUENCY_{i}");
        results.insert(key, cpu.frequency() as f64);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_cpu_counter(sys: &mut System, results: &mut HashMap<String, f64>) {
    sys.refresh_cpu();

    let vendor = sys.global_cpu_info().vendor_id();
    #[cfg(not(target_os = "macos"))]
    if vendor == "GenuineIntel" {
        intel::get_intel_cpu_counter(results);
    } else if vendor == "AuthenticAMD" {
        amd::get_amd_cpu_counter(sys, results);
    }
}

#[cfg(target_os = "macos")]
pub fn get_cpu_counter(sys: &mut System, results: &mut HashMap<String, f64>) {
    apple::get_apple_cpu_counter(results);
}
