#[cfg(target_os = "macos")]
use smc::SMC;

use std::collections::HashMap;

#[cfg(target_os = "macos")]
pub fn get_apple_gpu_counter(results: &mut HashMap<String, f64>) {
    let smc = SMC::new().unwrap();
    for key in [
        // Intel
        // "PCPG", // PCPG format is sp87 which is currently not support by smc
        "PCGM", "PCGC", // other
        "PG0R", "PGPR", "PG0C", "PGPC", "PG0T", "PGPT", "PG0H", "PGPH", "PG0L", "PGPL", "PG0P",
    ] {
        match smc.read_key::<f32>(key.into()) {
            Ok(res) => {
                results.insert("GPU_POWER (Watts)".to_string(), res.into());
            }
            _ => {}
        }
    }
}
