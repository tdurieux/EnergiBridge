use serde;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use std::fs;
use std::fs::DirEntry;
use std::time::Instant;

use super::msr::read_msr_on_core;

pub const UJ_TO_J_FACTOR: f64 = 1000000.;

pub const MSR_RAPL_POWER_UNIT: u32 = 0x00000606;
pub const MSR_RAPL_PKG_ENERGY_STAT: u32 = 0x611;

pub const INTEL_MSR_RAPL_PP0: u32 = 0x639;
pub const INTEL_MSR_RAPL_PP1: u32 = 0x641;
pub const INTEL_MSR_RAPL_DRAM: u32 = 0x619;
const INTEL_TIME_UNIT_MASK: u64 = 0xF000;
const INTEL_ENGERY_UNIT_MASK: u32 = 0x1F00;
const INTEL_POWER_UNIT_MASK: u32 = 0x0F;

const INTEL_TIME_UNIT_OFFSET: u32 = 0x10;
const INTEL_ENGERY_UNIT_OFFSET: u32 = 0x08;
const INTEL_POWER_UNIT_OFFSET: u32 = 0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAPLData {
    #[serde(skip_serializing, skip_deserializing)]
    pub path: String,
    pub zone: String,
    pub time_elapsed: f64,
    pub power_j: f64,
    pub watts: f64,
    pub watts_since_last: f64,
    pub start_power: f64,
    pub prev_power: f64,
    pub prev_power_reading: f64,
    pub temp: f64,
}

#[derive(Debug)]
pub struct RAPLZone {
    pub path: String,
    pub name: String,
}

// https://github.com/cs-21-pt-9-01/rapl.rs/blob/master/src/common.rs
pub fn list_rapl() -> Vec<RAPLZone> {
    let base_path = "/sys/devices/virtual/powercap/intel-rapl/";
    let cpus = fs::read_dir(base_path).unwrap();
    let mut zones: Vec<RAPLZone> = vec![];

    for cpu in cpus {
        let pkg = parse_rapl_dir(cpu.unwrap());
        match pkg {
            Some(x) => zones.push(x),
            None => continue,
        }

        let path = &zones.last().unwrap().path;
        let pkg = fs::read_dir(path).unwrap();

        for core in pkg {
            let core_zone = parse_rapl_dir(core.unwrap());
            match core_zone {
                Some(x) => zones.push(x),
                None => continue,
            }
        }
    }

    return zones;
}

fn parse_rapl_dir(item: DirEntry) -> Option<RAPLZone> {
    let cleaned_item_name = item
        .path()
        .display()
        .to_string()
        .split("/")
        .last()
        .unwrap()
        .to_owned();

    if !cleaned_item_name.contains("intel-rapl") {
        return None;
    }

    let item_path = item.path().display().to_string();
    let item_name_data = fs::read(format!("{}/name", item_path))
        .expect(format!("Couldn't read file {}/name", item_path).as_str());
    let item_name = String::from_utf8_lossy(&item_name_data);

    return Some(RAPLZone {
        path: item.path().display().to_string(),
        name: item_name.to_string().replace("\n", ""),
    });
}

pub fn setup_rapl_data() -> Vec<RAPLData> {
    let sys_zones = list_rapl();
    let mut zones: Vec<RAPLData> = vec![];

    for z in sys_zones {
        let start_power = read_power(z.path.to_owned());
        let data = RAPLData {
            path: z.path,
            zone: z.name,
            time_elapsed: 0.,
            power_j: 0.,
            watts: 0.,
            watts_since_last: 0.,
            start_power,
            prev_power: 0.,
            prev_power_reading: start_power,
            temp: 0.,
        };
        zones.push(data);
    }

    return zones;
}

pub fn calculate_power_metrics(
    zone: RAPLData,
    now: Instant,
    start_time: Instant,
    prev_time: Instant,
) -> RAPLData {
    let cur_power_j = read_power(zone.path.to_owned());

    #[allow(unused_assignments)]
    let mut power_j = 0.;
    let mut watts = 0.;
    let mut watts_since_last = 0.;

    let power_limit = read_power_limit(zone.path.to_owned());

    // if RAPL overflow has occurred
    // or if we have done a full RAPL cycle
    if zone.start_power >= cur_power_j || zone.power_j >= power_limit {
        // if our previous reading was pre-overflow, we simply add the new reading
        // otherwise we add the difference
        if zone.prev_power_reading > cur_power_j {
            power_j = (power_limit - zone.prev_power_reading) + cur_power_j + zone.power_j;
        } else {
            power_j = (cur_power_j - zone.prev_power_reading) + zone.power_j;
        }
    } else {
        power_j = cur_power_j - zone.start_power;
    }

    let sample_time = now.duration_since(start_time).as_secs_f64();
    if sample_time > 0. {
        watts = power_j / sample_time;
    }

    if prev_time > start_time {
        watts_since_last = (power_j - zone.power_j) / now.duration_since(prev_time).as_secs_f64();
    }

    return RAPLData {
        path: zone.path,
        zone: zone.zone,
        time_elapsed: start_time.elapsed().as_secs_f64(),
        power_j,
        watts,
        watts_since_last,
        start_power: zone.start_power,
        prev_power: zone.power_j,
        prev_power_reading: cur_power_j,
        temp: 0.,
    };
}

pub fn reading_as_float(reading: &[u8]) -> f64 {
    let mut reading = String::from_utf8(reading.to_vec()).unwrap();
    reading.pop();
    return reading.parse::<f64>().unwrap();
}

pub fn read_power(file_path: String) -> f64 {
    let power = fs::read(format!("{}/energy_uj", file_path.to_owned()))
        .expect(format!("Couldn't read file {}/energy_uj", file_path.to_owned()).as_str());

    return reading_as_float(&power) / UJ_TO_J_FACTOR;
}

pub fn read_power_limit(file_path: String) -> f64 {
    let limit = fs::read(format!("{}/max_energy_range_uj", file_path.to_owned())).expect(
        format!(
            "Couldn't read file {}/max_energy_range_uj",
            file_path.to_owned()
        )
        .as_str(),
    );

    return reading_as_float(&limit) / UJ_TO_J_FACTOR;
}

pub fn get_intel_cpu_cunter(results: &mut HashMap<String, f64>) {
    // let zone = setup_rapl_data();
    let start_time = Instant::now();
    let mut prev_time = Instant::now();
    let mut now = Instant::now();

    //    let mut data =
    //      calculate_power_metrics(zone.get(0).unwrap().to_owned(), now, start_time, prev_time);

    unsafe {
        let core_energy_units: u64 = read_msr_on_core(MSR_RAPL_POWER_UNIT, 0).unwrap();
        let energy_unit: u64 = (core_energy_units & INTEL_TIME_UNIT_MASK) >> 8;
        let energy_unit_d = 0.5f64.powf(energy_unit as f64);

        let pp0 = read_msr_on_core(INTEL_MSR_RAPL_PP0, 0).expect("failed to read PP0");
        let pp1 = read_msr_on_core(INTEL_MSR_RAPL_PP1, 0).expect("failed to read PP1");
        let pkg = read_msr_on_core(MSR_RAPL_PKG_ENERGY_STAT, 0)
            .expect("failed to read RAPL_PKG_ENERGY_STAT");
        let dram = read_msr_on_core(INTEL_MSR_RAPL_DRAM, 0).expect("failed to read DRAM");

        results.insert(
            format!("DRAM_ENERGY (J)"),
            dram as f64 * energy_unit_d,
        );
        results.insert(
            format!("PACAKGE_ENERGY (J)"),
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
