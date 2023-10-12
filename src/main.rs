mod cpu;
mod gpu;
mod memory;
mod process;

use clap::Parser;

use std::collections::HashMap;
use std::fs::File;
use std::io::{stdout, Write};
use std::process::Command;
use std::process::{exit, Child};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cpu::{get_cpu_cunter, get_cpu_usage};
use gpu::get_gpu_cunter;
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
    #[arg(short, long, default_value_t = 100)]
    interval: u32,

    // enable to measure the GPU power consumption
    #[arg(short, long, default_value_t = false)]
    gpu: bool,

    // the command to execute
    command: Vec<String>,
}

fn main() {
    // EXAMPLE https://gist.github.com/carstein/6f4a4fdf04ec002d5494a11d2cf525c7
    let args = Args::parse();
    let interval = Duration::from_millis(args.interval.into());
    let sep = args.separator.as_str();

    if args.command.is_empty() {
        eprintln!("Usage: {} <command>", "EnergiBridge");
        exit(1);
    }

    let mut results = HashMap::new();
    let mut output = match args.output {
        Some(ref path) => {
            Box::new(File::create(path).expect("Failed to open output file")) as Box<dyn Write>
        }
        None => Box::new(stdout()) as Box<dyn Write>,
    };

    let cmd = execute_command(args.command);

    match cmd {
        Ok(mut child) => {
            collect(child.id(), &mut results);
            print_header(&results, sep, &mut output);
            let mut previous_time = SystemTime::now();
            loop {
                let time_before = SystemTime::now();
                print_results(previous_time, &mut results, sep, &mut output);
                previous_time = SystemTime::now();
                collect(child.id(), &mut results);

                match child.try_wait() {
                    Ok(Some(status)) => {
                        print_results(previous_time, &mut results, sep, &mut output);
                        exit(status.code().unwrap());
                    }
                    Ok(None) => {
                        sleep(interval - time_before.elapsed().unwrap());
                    }
                    Err(e) => println!("Error waiting: {}", e),
                }
            }
        }
        Err(_) => {
            eprintln!("Failed to execute command.");
            exit(1);
        }
    }
}

fn execute_command(command: Vec<String>) -> std::io::Result<Child> {
    if command.is_empty() {
        exit(1);
    }
    let mut cmd = Command::new(&command[0]);
    for arg in command.iter().skip(1) {
        cmd.arg(arg);
    }

    return cmd.spawn();
}

fn collect(pid: u32, results: &mut HashMap<String, f64>) {
    get_memory_usage(results);
    get_cpu_usage(results);
    get_cpu_cunter(results);
    get_gpu_cunter(results);
    // get_process_usage(pid, results);
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
    for (_, value) in results.iter() {
        output
            .write_all(format!("{}{}", sep, value).as_bytes())
            .expect("Failed to write results");
    }
    output.write_all(b"\n").expect("Failed to write results");
}

fn print_header(results: &HashMap<String, f64>, sep: &str, output: &mut dyn Write) {
    output
        .write_all(format!("Delta{}Time", sep).as_bytes())
        .expect("Failed to write header");
    for (key, _value) in results.iter() {
        output
            .write_all(format!("{}{}", sep, key).as_bytes())
            .expect("Failed to write header");
    }
    output.write_all(b"\n").expect("Failed to write header");
}
