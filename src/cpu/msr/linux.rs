#![cfg(target_os = "linux")]

use once_cell::sync::OnceCell;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};

static PHYSICAL_TO_LOGICAL_MAP: OnceCell<Vec<u32>> = OnceCell::new();

fn parse_cpu_list(input: &str) -> Vec<u32> {
    let mut cpus = Vec::new();

    for part in input.trim().split(',') {
        let token = part.trim();
        if token.is_empty() {
            continue;
        }

        if let Some((start_s, end_s)) = token.split_once('-') {
            let start = start_s.trim().parse::<u32>();
            let end = end_s.trim().parse::<u32>();
            if let (Ok(start), Ok(end)) = (start, end) {
                if start <= end {
                    cpus.extend(start..=end);
                }
            }
        } else if let Ok(cpu) = token.parse::<u32>() {
            cpus.push(cpu);
        }
    }

    cpus.sort_unstable();
    cpus.dedup();
    cpus
}

fn is_cpu_online(cpu: u32) -> bool {
    let path = format!("/sys/devices/system/cpu/cpu{}/online", cpu);
    match fs::read_to_string(path) {
        Ok(value) => value.trim() == "1",
        // cpu0 and some systems don't expose this file for always-online CPUs.
        Err(_) => true,
    }
}

fn build_physical_to_logical_map() -> Result<Vec<u32>, std::io::Error> {
    let cpu_root = "/sys/devices/system/cpu";
    let mut cpu_ids = Vec::<u32>::new();

    for entry in fs::read_dir(cpu_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(id) = name
            .strip_prefix("cpu")
            .and_then(|suffix| suffix.parse::<u32>().ok())
        {
            cpu_ids.push(id);
        }
    }

    cpu_ids.sort_unstable();

    let mut seen_sibling_sets = HashSet::<String>::new();
    let mut mapped = Vec::<u32>::new();

    for cpu in cpu_ids {
        let siblings_path = format!(
            "/sys/devices/system/cpu/cpu{}/topology/thread_siblings_list",
            cpu
        );
        let siblings_raw = fs::read_to_string(siblings_path)?;
        let siblings = parse_cpu_list(&siblings_raw);

        if siblings.is_empty() {
            continue;
        }

        let signature = siblings
            .iter()
            .map(u32::to_string)
            .collect::<Vec<String>>()
            .join(",");

        if !seen_sibling_sets.insert(signature) {
            continue;
        }

        let representative = siblings
            .iter()
            .copied()
            .find(|&logical| is_cpu_online(logical))
            .unwrap_or(siblings[0]);
        mapped.push(representative);
    }

    if mapped.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "failed to map physical cores",
        ));
    }

    Ok(mapped)
}

fn resolve_logical_core(physical_core: u32) -> Result<u32, std::io::Error> {
    let map = PHYSICAL_TO_LOGICAL_MAP.get_or_try_init(build_physical_to_logical_map)?;
    map.get(physical_core as usize).copied().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("physical core index {} out of range", physical_core),
        )
    })
}


pub fn read_msr_on_core(msr: u32, core: u32) -> Result<u64, std::io::Error> {
    let logical_core = resolve_logical_core(core)?;
    let mut file = File::open(format!("/dev/cpu/{}/msr", logical_core))?;

    // Seek to the MSR address
    file.seek(SeekFrom::Start(u64::from(msr)))?;

    // Read the 8-byte MSR value
    let mut value_bytes = [0u8; 8];
    file.read_exact(&mut value_bytes)?;

    let value = u64::from_le_bytes(value_bytes);

    Ok(value)
}
