use std::collections::HashMap;
use sysinfo::{Pid, ProcessExt, System, SystemExt};

pub fn get_process_usage(pid: u32, results: &mut HashMap<String, f64>) {
    let sys = System::new();
    let p = sys.process(Pid::from(pid as usize));
    match p {
        Some(p) => {
            results.insert("MEMORY".to_string(), p.memory() as f64);
            results.insert("VIRTUAL_MEMORY".to_string(), p.virtual_memory() as f64);
            results.insert("CPU_USAGE".to_string(), p.cpu_usage() as f64);
        }
        None => {}
    }
}
