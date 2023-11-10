use std::collections::HashMap;
use sysinfo::{System, SystemExt, ProcessExt, Pid};

pub fn get_process_usage(sys: &mut System, pid: u32, results: &mut HashMap<String, f64>) {
    let p = sys.process(Pid::from(pid as usize));
    match p {
        Some(p) => {
            results.insert("PROCESS_MEMORY".to_string(), p.memory() as f64);
            results.insert("PROCESS_VIRTUAL_MEMORY".to_string(), p.virtual_memory() as f64);
            results.insert("PROCESS_CPU_USAGE".to_string(), p.cpu_usage() as f64);
        }
        None => {}
    }
}
