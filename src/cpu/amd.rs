use std::collections::HashMap;
use sysinfo::System;

use super::{get_number_cores, msr::read_msr_on_core};

pub const AMD_MSR_PWR_UNIT: u32 = 0xC0010299;
const AMD_MSR_CORE_ENERGY: u32 = 0xC001029A;
const AMD_MSR_PACKAGE_ENERGY: u32 = 0xC001029B;
const AMD_MSR_HARDWARE_PSTATE_STATUS: u32 = 0xC0010293;

const AMD_ENERGY_UNIT_MASK: u32 = 0x1F00;
const VID_STEP: f64 = 0.00625;

/// Detect the AMD CPU family via CPUID leaf 1.
/// Returns the computed family (BaseFamily + ExtFamily when BaseFamily == 0xF).
fn get_amd_cpu_family() -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        let cpuid = unsafe { std::arch::x86_64::__cpuid(1) };
        let eax = cpuid.eax;
        let base_family = (eax >> 8) & 0xF;
        if base_family == 0xF {
            base_family + ((eax >> 20) & 0xFF)
        } else {
            base_family
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        0x17 // default to Zen 1-4 layout
    }
}

pub fn get_amd_cpu_counter(sys: &mut System, results: &mut HashMap<String, f64>) {
    let nb_core = get_number_cores(sys).unwrap() as u32;
    let family = get_amd_cpu_family();
    let is_zen5 = family >= 0x1A;

    unsafe {
        let core_energy_units: u64 = read_msr_on_core(AMD_MSR_PWR_UNIT, 0).unwrap();
        let energy_unit: u64 = (core_energy_units & AMD_ENERGY_UNIT_MASK as u64) >> 8;
        let energy_unit_d = 0.5f64.powf(energy_unit as f64);

        for core in 0..nb_core {
            let core_energy_raw = read_msr_on_core(AMD_MSR_CORE_ENERGY, core).unwrap() & 0xFFFFFFFF;
            let package_raw = read_msr_on_core(AMD_MSR_PACKAGE_ENERGY, core).unwrap() & 0xFFFFFFFF;

            // MSRC001_0293 — Hardware P-State Status (lower 32 bits only)
            // CurHwPstate [24:22]
            // CurCpuVid   [21:14]
            // CurCpuDfsId [13:8]
            // CurCpuFid   [7:0]   (Zen 1-4, families 17h/19h)
            // CurCpuFid   [11:0]  (Zen 5, family 1Ah)
            let hw_pstate_raw = read_msr_on_core(AMD_MSR_HARDWARE_PSTATE_STATUS, core).unwrap() as u32;

            let cur_hw_pstate = (hw_pstate_raw >> 22) & 0x07;
            let cur_cpu_vid = ((hw_pstate_raw >> 14) & 0xFF) as f64;

            let freq_mhz = if is_zen5 {
                // Zen 5 (family 1Ah): CoreCOF = CpuFid[11:0] * 5
                let cur_cpu_fid = hw_pstate_raw & 0xFFF;
                cur_cpu_fid as f64 * 5.0
            } else {
                // Zen 1-4 (families 17h/19h): CoreCOF = (CpuFid[7:0] / CpuDfsId[13:8]) * 200
                let cur_cpu_fid = (hw_pstate_raw & 0xFF) as f64;
                let cur_cpu_dfs_id = ((hw_pstate_raw >> 8) & 0x3F) as f64;
                if cur_cpu_dfs_id > 0f64 {
                    let ratio = 25f64 * cur_cpu_fid / (12.5 * cur_cpu_dfs_id);
                    ratio * 100f64
                } else {
                    0.0
                }
            };

            // Vcore = 1.550 - (VID * 0.00625)
            let volts = 1.550 - (cur_cpu_vid * VID_STEP);

            results.insert(format!("CORE{}_VOLT (V)", core), volts);
            results.insert(format!("CORE{}_FREQ (MHZ)", core), freq_mhz);
            results.insert(format!("CORE{}_PSTATE", core), cur_hw_pstate as f64);
            results.insert(
                format!("CORE{}_ENERGY (J)", core),
                core_energy_raw as f64 * energy_unit_d,
            );
            results.insert(
                format!("CPU_ENERGY (J)"),
                package_raw as f64 * energy_unit_d,
            );
        }
    }
}
