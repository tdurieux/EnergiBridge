mod cpu;
mod gpu;
mod memory;
mod process;

use clap::Parser;

use itertools::Itertools;
use std::collections::HashMap;
use std::fs::File;
use std::io::{stdout, Write};
use std::process::{exit, Child};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use sysinfo::{CpuExt, ProcessExt, RefreshKind, System, SystemExt};

use cpu::{get_cpu_counter, get_cpu_usage};
use gpu::get_gpu_counter;
use memory::get_memory_usage;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Where to save the output of power measurements
    #[arg(short, long)]
    output: Option<String>,

    #[arg(short, long, default_value = ",")]
    separator: String,

    // Where to save the output of the command
    #[arg(short, long, required = false)]
    command_output: Option<String>,

    /// Duration of the interval between two measurements in micoseconds
    #[arg(short, long, default_value_t = 200)]
    interval: u32,

    /// Define the maximum duration of the execution of the command in seconds, set to 0 to disable
    #[arg(short, long, default_value_t = 0)]
    max_execution: u32,

    // enable to measure the GPU power consumption
    #[arg(short, long, default_value_t = false)]
    gpu: bool,

    // print the summary of the energy consumption
    #[arg(long, default_value_t = false)]
    summary: bool,

    // the command to execute
    #[clap(trailing_var_arg = true)]
    command: Vec<String>,
}

fn main() {
    // EXAMPLE https://gist.github.com/carstein/6f4a4fdf04ec002d5494a11d2cf525c7
    let args = Args::parse();
    let interval = Duration::from_millis(args.interval.into());
    let sep = args.separator.as_str();
    let collect_gpu = args.gpu;
    // Create an atomic flag to indicate when to stop the execution loop
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    if args.command.is_empty() {
        eprintln!("Usage: {} <command>", "EnergiBridge");
        exit(1);
    }

    if interval < System::MINIMUM_CPU_UPDATE_INTERVAL {
        eprintln!(
            "[WARNING] Interval must be at least {}ms to accurately measure CPU usage.",
            System::MINIMUM_CPU_UPDATE_INTERVAL.as_millis()
        );
    }
    
    // Set up the Ctrl+C handler
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, stopping...");
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    #[cfg(not(target_os = "macos"))]
    cpu::msr::start_rapl();

    let mut sys = System::new_all();
    sys.refresh_all();
    std::thread::sleep(System::MINIMUM_CPU_UPDATE_INTERVAL);
    let mut results: HashMap<String, f64> = HashMap::new();
    collect(&mut sys, collect_gpu, 0, &mut results);

    let mut output = match args.output {
        Some(ref path) => {
            Box::new(File::create(path).expect("Failed to open output file")) as Box<dyn Write>
        }
        None => Box::new(stdout()) as Box<dyn Write>,
    };

    let cmd = execute_command(args.command, args.command_output);

    match cmd {
        Ok(mut child) => {
            let start_time = Instant::now();

            collect(&mut sys, collect_gpu, child.id(), &mut results);
            print_header(&results, sep, &mut output);
            let mut previous_time = SystemTime::now();
            let mut energy_array: f64 = 0 as f64;
            let mut previous_results = results.clone();
            let exit_code = loop {
                if args.max_execution > 0
                    && start_time.elapsed().as_secs() >= args.max_execution as u64
                {
                    // kill the process if it is still running
                    child.kill().expect("Failed to kill child");
                    break 0;
                }
                let time_before = SystemTime::now();
                print_results(previous_time, &mut results, sep, &mut output);

                if args.summary {
                    if results.contains_key("CPU_POWER (Watts)") {
                        let energy = results["CPU_POWER (Watts)"];
                        energy_array += energy
                            * (previous_time.elapsed().unwrap().as_millis() as f64 / 1000 as f64);
                    } else if results.contains_key("SYSTEM_POWER (Watts)") {
                        let energy = results["SYSTEM_POWER (Watts)"];
                        energy_array += energy
                            * (previous_time.elapsed().unwrap().as_millis() as f64 / 1000 as f64);
                    } else if results.contains_key("CPU_ENERGY (J)") {
                        let energy = results["CPU_ENERGY (J)"];
                        let old_energy = previous_results["CPU_ENERGY (J)"];
                        energy_array += energy - old_energy;
                    } else if results.contains_key("PACKAGE_ENERGY (J)") {
                        let energy = results["PACKAGE_ENERGY (J)"];
                        let old_energy = previous_results["PACKAGE_ENERGY (J)"];
                        energy_array += energy - old_energy;
                    }
                }
                previous_time = SystemTime::now();
                previous_results = results.clone();
                collect(&mut sys, collect_gpu, child.id(), &mut results);

                if !running.load(Ordering::SeqCst) {
                    // EnergiBridge received ctrlc
                    child.kill().expect("Failed to kill child");
                    break 1;
                }
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // print_results(previous_time, &mut results, sep, &mut output);
                        break status.code().unwrap();
                    }
                    Ok(None) => {
                        sleep(interval - time_before.elapsed().unwrap());
                    }
                    Err(e) => println!("Error waiting: {}", e),
                }
            };

            print_results(previous_time, &mut results, sep, &mut output);
            if energy_array > 0.0 && args.summary {
                println!(
                    "Energy consumption in joules: {} for {} sec of execution.",
                    energy_array,
                    start_time.elapsed().as_secs_f32()
                );
            }

            exit(exit_code);
        }
        Err(err) => {
            eprintln!("Failed to execute command: {}", err);
            exit(1);
        }
    }
}

fn execute_command(command: Vec<String>, output: Option<String>) -> std::io::Result<Child> {
    if command.is_empty() {
        exit(1);
    }
    let mut cmd = Command::new(&command[0]);
    for arg in command.iter().skip(1) {
        cmd.arg(arg);
    }
    if output.is_some() {
        cmd.stdout(Stdio::from(File::create(output.unwrap()).unwrap()));
    }

    return cmd.spawn();
}

fn collect(sys: &mut System, collect_gpu: bool, pid: u32, results: &mut HashMap<String, f64>) {
    get_memory_usage(sys, results);
    get_cpu_usage(sys, results);
    get_cpu_counter(sys, results);
    if collect_gpu {
        get_gpu_counter(results);
    }
    // get_process_usage(sys, pid, results);
}

fn print_results(
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

fn print_header(results: &HashMap<String, f64>, sep: &str, output: &mut dyn Write) {
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
