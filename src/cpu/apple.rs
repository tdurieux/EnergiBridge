use std::collections::HashMap;

#[cfg(target_os = "macos")]
use smc::SMC;

#[cfg(target_os = "macos")]
pub fn get_apple_cpu_counter(results: &mut HashMap<String, f64>) {
    let smc = SMC::new().unwrap();
    // does not work on M1
    match smc.read_key::<f32>("PCTR".into()) {
        Ok(res) => {
            results.insert("CPU_POWER (Watts)".to_string(), res.into());
        }
        _ => {}
    }
    match smc.read_key::<f32>("PSTR".into()) {
        Ok(res) => {
            results.insert("SYSTEM_POWER (Watts)".to_string(), res.into());
        }
        _ => {}
    }
    let mut i = 0;
    for key in [
        // Intel
        "TC0C", "TC1C", "TC2C", "TC3C", "TC4C", "TC5C", "TC6C", "TC7C", "TC8C", "TC9C",
        // Apple Silicon
        "Tp09", "Tp0T", "Tp01", "Tp05", "Tp0D", "Tp0H", "Tp0L", "Tp0P", "Tp0X", "Tp0b",
        // M2
        "Tp0j", "Tp0r", "Tp0f", "Tp0n",
    ] {
        match smc.temperature(key.into()) {
            Ok(t) if t > 0. => {
                let key: String = format!("CPU_TEMP_{}", i);
                results.insert(key, t.into());
                i += 1;
            }
            _ => {}
        }
    }
}
