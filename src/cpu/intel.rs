use serde;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use std::fs;
use std::fs::DirEntry;
use std::time::Instant;

use super::msr::read_msr_on_core;

pub const INTEL_MSR_RAPL_POWER_UNIT: u32 = 0x606;
pub const INTEL_MSR_RAPL_PKG: u32 = 0x611;
pub const INTEL_MSR_RAPL_PP0: u32 = 0x639;
pub const INTEL_MSR_RAPL_PP1: u32 = 0x641;
pub const INTEL_MSR_RAPL_DRAM: u32 = 0x619;

const INTEL_TIME_UNIT_MASK: u64 = 0xF0000; // Bits 19:16
const INTEL_ENGERY_UNIT_MASK: u64 = 0x1F00; // Bits 12:8
const INTEL_POWER_UNIT_MASK: u64 = 0x0F; // Bits 3:0

const INTEL_TIME_UNIT_OFFSET: u32 = 0x10; // Offset 16
const INTEL_ENGERY_UNIT_OFFSET: u32 = 0x08; // Offset 8
const INTEL_POWER_UNIT_OFFSET: u32 = 0; // Offset 0


pub fn get_intel_cpu_counter(results: &mut HashMap<String, f64>) {
    unsafe {
        // --- Read units ---
        // TODO: these values are constant, read them once and store them
        // The MSR only store integer values, but they represent floating point values.
        // The INTEL_MSR_RAPL_POWER_UNIT MSR contains the units for the RAPL MSRs for a specific intel chip.
        // it contains three units, which represent the time, power, and energy increments in the RAPL MSRs.
        let core_energy_units: u64 = read_msr_on_core(INTEL_MSR_RAPL_POWER_UNIT, 0).unwrap();
        
        // First, we extract the individual units using the masks and offsets.
        // Then we convert them to floating point values using the formula 0.5^x.
        // See Section 14.9.1 of the Intel Architectures Software Developer's Manual (Vol 3B) for more information.
        let energy_unit: u64 = (core_energy_units & INTEL_ENGERY_UNIT_MASK) >> INTEL_ENGERY_UNIT_OFFSET;
        let energy_unit_d = 0.5f64.powf(energy_unit as f64);

        // --- Read values ---
        // PP0 = CPU cores energy consumption
        let pp0 = read_msr_on_core(INTEL_MSR_RAPL_PP0, 0).expect("failed to read PP0");
        // PP1 = Integrated GPU energy consumption
        let pp1 = read_msr_on_core(INTEL_MSR_RAPL_PP1, 0).expect("failed to read PP1");
        // PKG = CPU socket energy consumption
        let pkg = read_msr_on_core(INTEL_MSR_RAPL_PKG, 0)
            .expect("failed to read RAPL_PKG_ENERGY_STAT");
        // DRAM = Energy consumed by the DRAM for the chip's memory controller.
        let dram = read_msr_on_core(INTEL_MSR_RAPL_DRAM, 0).expect("failed to read DRAM");
        
        // --- Convert & store ---
        // convert the integer values to floating point values using the energy unit
        // and store them in the results hashmap
        results.insert(
            format!("DRAM_ENERGY (J)"),
            dram as f64 * energy_unit_d,
        );
        results.insert(
            format!("PACKAGE_ENERGY (J)"),
            pkg as f64 * energy_unit_d,
        );
        results.insert(
            format!("PP0_ENERGY (J)"),
            pp0 as f64 * energy_unit_d,
        );
        results.insert(
            format!("PP1_ENERGY (J)"),
            pp1 as f64 * energy_unit_d,
        );
    }
}
