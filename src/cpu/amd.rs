use std::collections::HashMap;

#[cfg(target_os = "windows")]
use std::arch::asm;
#[cfg(target_os = "windows")]
use x86::msr::{rdmsr, wrmsr, IA32_MSR_TURBO_RATIO_LIMIT};

#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::io::Read;
#[cfg(target_os = "linux")]
use std::io::Seek;
#[cfg(target_os = "linux")]
use std::io::SeekFrom;

use super::get_number_cores;

const AMD_MSR_PWR_UNIT: u32 = 0xC0010299;
const AMD_MSR_CORE_ENERGY: u32 = 0xC001029A;
const AMD_MSR_PACKAGE_ENERGY: u32 = 0xC001029B;
const AMD_MSR_FID: u32 = 0xC0010293;

const AMD_TIME_UNIT_MASK: u32 = 0xF0000;
const AMD_ENERGY_UNIT_MASK: u32 = 0x1F00;
const AMD_POWER_UNIT_MASK: u32 = 0xF;

#[cfg(target_os = "windows")]
fn read_msr_on_core(msr: u32, core: u32) -> u64 {
    // Set the processor affinity to the desired core
    let affinity_mask: u64 = 1 << core;
    unsafe {
        asm!("mov rax, $0" :: "r"(affinity_mask) : "rax" : "volatile");
        asm!("mov rcx, $0" :: "r"(msr) : "rcx" : "volatile");
        asm!("wrmsr" :: : "rdx", "rax", "rcx" : "volatile");
    }

    // Read the MSR
    let value = rdmsr(msr);

    // Restore the original affinity
    let original_affinity: u64 = 0;
    unsafe {
        asm!("mov rax, $0" : : "r"(original_affinity) : "rax" : "volatile");
        asm!("mov rcx, $0" : : "r"(msr) : "rcx" : "volatile");
        asm!("wrmsr" : : : "rdx", "rax", "rcx" : "volatile");
    }

    Ok(value)
}

#[cfg(target_os = "linux")]
fn read_msr_on_core(msr: u32, core: u32) -> Result<u64, std::io::Error> {
    let mut file = File::open(format!("/dev/cpu/{}/msr", core))?;

    // Seek to the MSR address
    file.seek(SeekFrom::Start(u64::from(msr)))?;

    // Read the 8-byte MSR value
    let mut value_bytes = [0u8; 8];
    file.read_exact(&mut value_bytes)?;

    let value = u64::from_le_bytes(value_bytes);

    Ok(value)
}

pub fn get_amd_cpu_cunter(results: &mut HashMap<String, f64>) {
    let nb_core = get_number_cores().unwrap() as u32;

    let core_energy_units: u64 = read_msr_on_core(AMD_MSR_PWR_UNIT, 0).unwrap();
    let energy_unit: u64 = (core_energy_units & AMD_ENERGY_UNIT_MASK as u64) >> 8;
    let energy_unit_d = 0.5f64.powf(energy_unit as f64);

    for core in 0..nb_core {
        let core_energy_raw = read_msr_on_core(AMD_MSR_CORE_ENERGY, core).unwrap();
        let package_raw = read_msr_on_core(AMD_MSR_PACKAGE_ENERGY, core).unwrap();

        let pstate_limits = read_msr_on_core(0xC0010061, core).unwrap();

        let pstate_req = read_msr_on_core(0xC0010062, core).unwrap();
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
            format!("CORE{}_ENERGY (W)", core),
            core_energy_raw as f64 * energy_unit_d,
        );
        results.insert(
            format!("PACAKGE{}_ENERGY (W)", core),
            package_raw as f64 * energy_unit_d,
        );
    }
}
