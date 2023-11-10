use std::collections::HashMap;
use sysinfo::{System, SystemExt};

pub fn get_memory_usage(sys: &mut System, results: &mut HashMap<String, f64>) {
    sys.refresh_memory();

    results.insert("TOTAL_MEMORY".to_string(), sys.total_memory() as f64);
    results.insert("USED_MEMORY".to_string(), sys.used_memory() as f64);
    results.insert("TOTAL_SWAP".to_string(), sys.total_swap() as f64);
    results.insert("USED_SWAP".to_string(), sys.used_swap() as f64);
}
