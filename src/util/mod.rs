use crate::cpu::{get_cpu_counter, get_cpu_usage};
use crate::gpu::get_gpu_counter;
use crate::memory::get_memory_usage;
use itertools::Itertools;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::process::{exit, Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;

pub(crate) fn collect(
    sys: &mut System,
    collect_gpu: bool,
    pid: u32,
    results: &mut HashMap<String, f64>,
) {
    get_memory_usage(sys, results);
    get_cpu_usage(sys, results);
    get_cpu_counter(sys, results);
    if collect_gpu {
        get_gpu_counter(results);
    }
    // get_process_usage(sys, pid, results);
}

pub(crate) fn print_results(
    time: SystemTime,
    results: &mut HashMap<String, f64>,
    sep: &str,
    output: &mut dyn Write,
) {
    output
        .write_all(
            format!(
                "{}{}{}",
                time.elapsed().unwrap().as_millis(),
                sep,
                time.duration_since(UNIX_EPOCH).unwrap().as_millis()
            )
            .as_bytes(),
        )
        .expect("Failed to write results");
    for key in results.keys().sorted() {
        output
            .write_all(format!("{}{}", sep, results[key]).as_bytes())
            .expect("Failed to write results");
    }
    output.write_all(b"\n").expect("Failed to write results");
}

pub(crate) fn print_header(results: &HashMap<String, f64>, sep: &str, output: &mut dyn Write) {
    output
        .write_all(format!("Delta{}Time", sep).as_bytes())
        .expect("Failed to write header");
    for key in results.keys().sorted() {
        output
            .write_all(format!("{}{}", sep, key).as_bytes())
            .expect("Failed to write header");
    }
    output.write_all(b"\n").expect("Failed to write header");
}

pub(crate) fn execute_command(
    command: Vec<String>,
    output: Option<String>,
) -> std::io::Result<Child> {
    if command.is_empty() {
        exit(1);
    }
    let mut cmd = Command::new(&command[0]);
    for arg in command.iter().skip(1) {
        cmd.arg(arg);
    }
    if output.is_some() {
        cmd.stdout(Stdio::from(File::create(output.unwrap())?));
    }

    cmd.spawn()
}

pub fn process_summary(
    summary: bool,
    results: &mut HashMap<String, f64>,
    previous_time: &mut SystemTime,
    previous_results: &mut HashMap<String, f64>,
) -> f64 {
    if !summary {
        return 0f64;
    }

    if results.contains_key("CPU_POWER (Watts)") {
        let energy = results["CPU_POWER (Watts)"];
        return energy * (previous_time.elapsed().unwrap().as_millis() as f64 / 1000f64);
    } else if results.contains_key("SYSTEM_POWER (Watts)") {
        let energy = results["SYSTEM_POWER (Watts)"];
        return energy * (previous_time.elapsed().unwrap().as_millis() as f64 / 1000f64);
    } else if results.contains_key("CPU_ENERGY (J)") {
        let energy = results["CPU_ENERGY (J)"];
        let old_energy = previous_results["CPU_ENERGY (J)"];
        return energy - old_energy;
    } else if results.contains_key("PACKAGE_ENERGY (J)") {
        let energy = results["PACKAGE_ENERGY (J)"];
        let old_energy = previous_results["PACKAGE_ENERGY (J)"];
        return energy - old_energy;
    }

    0f64
}