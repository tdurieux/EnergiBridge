use std::collections::HashMap;
use sysinfo::{System};

use super::{get_number_cores, msr::read_msr_on_core};

pub const AMD_MSR_PWR_UNIT: u32 = 0xC0010299;
const AMD_MSR_CORE_ENERGY: u32 = 0xC001029A;
const AMD_MSR_PACKAGE_ENERGY: u32 = 0xC001029B;
const AMD_MSR_FID: u32 = 0xC0010293;

const AMD_ENERGY_UNIT_MASK: u32 = 0x1F00;

pub fn get_amd_cpu_counter(sys: &mut System, results: &mut HashMap<String, f64>) {
    #[cfg(target_os = "linux")]
    let nb_core = get_number_cores(sys).unwrap() as u32;
    #[cfg(target_os = "windows")]
    let nb_core = 1;

    unsafe {
        let core_energy_units: u64 = read_msr_on_core(AMD_MSR_PWR_UNIT, 0).unwrap();
        let energy_unit: u64 = (core_energy_units & AMD_ENERGY_UNIT_MASK as u64) >> 8;
        let energy_unit_d = 0.5f64.powf(energy_unit as f64);

        for core in 0..nb_core {
            let core_energy_raw = read_msr_on_core(AMD_MSR_CORE_ENERGY, core).unwrap();
            let package_raw = read_msr_on_core(AMD_MSR_PACKAGE_ENERGY, core).unwrap();
            let pstate = read_msr_on_core(0xC0010063, core).unwrap();

            let fid = read_msr_on_core(AMD_MSR_FID, core).unwrap();

            let Did = ((fid >> 8) & 0x3F) as f64;
            let Fid = (fid & 0xFF) as f64;
            let Vid = ((fid >> 14) & 0xff) as f64;

            let ratio = 25f64 * Fid / (12.5 * (Did));
            let freq_mhz = ratio * 100f64;
            let volts = 1.55 - (Vid) * 0.00625;

            results.insert(format!("CORE{}_VOLT (V)", core), volts);
            results.insert(format!("CORE{}_FREQ (MHZ)", core), freq_mhz);
            results.insert(format!("CORE{}_PSTATE", core), (pstate & 0x07) as f64);
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
