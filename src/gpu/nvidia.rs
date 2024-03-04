use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::error::NvmlError;
use nvml_wrapper::struct_wrappers::device::*;
use nvml_wrapper::NVML;

use std::collections::HashMap;

pub struct GPUstat {
    id: Result<u32, NvmlError>,
    utilization_rates: Result<Utilization, NvmlError>,
    memory_info: Result<MemoryInfo, NvmlError>,
    // fan_speed: Result<u32, NvmlError>,
    temperature: Result<u32, NvmlError>,
    power: Result<u32, NvmlError>,
}

pub fn read_gpu_stat(device: &nvml_wrapper::Device) -> GPUstat {
    let gpustat = GPUstat {
        id: device.index(),
        utilization_rates: device.utilization_rates(),
        memory_info: device.memory_info(),
        // fan_speed: device.fan_speed(0), // Currently only take one fan, will add more fan readings
        temperature: device.temperature(TemperatureSensor::Gpu),
        power: device.power_usage(),
    };

    return gpustat;
}

pub fn dump_gpu_stat(device: nvml_wrapper::Device, results: &mut HashMap<String, f64>) {
    let gpustat = read_gpu_stat(&device);
    let index = gpustat.id.unwrap();

    match gpustat.utilization_rates {
        Ok(utilization_rates) => {
            results.insert(
                format!("GPU{}_USAGE", index).to_string(),
                utilization_rates.gpu.into(),
            );
        }
        Err(_) => {}
    };

    match gpustat.memory_info {
        Ok(memory_info) => {
            results.insert(
                format!("GPU{}_MEMORY_USED", index).to_string(),
                (memory_info.used / 1024 / 1024) as f64,
            );
            results.insert(
                format!("GPU{}_MEMORY_TOTAL", index).to_string(),
                (memory_info.total / 1024 / 1024) as f64,
            );
        }
        Err(_) => {}
    };

    // let fan_speed = match gpustat.fan_speed {
    //     Ok(fan_speed) => format!("{:>3} % | ", fan_speed),
    //     Err(_err) => "".to_string(),
    // };
    // result.push_str(&fan_speed);

    match gpustat.temperature {
        Ok(temperature) => {
            results.insert(
                format!("GPU{}_TEMPERATURE", index).to_string(),
                temperature.into(),
            );
        }
        Err(_) => {}
    };

    match gpustat.power {
        Ok(power) => {
            let key = format!("GPU{}_POWER (mWatts)", index).to_string();
            results.insert(key, power.into());
        }
        Err(_) => {}
    };
}

pub fn dump_all_gpu_stats(
    nvml: &nvml_wrapper::NVML,
    results: &mut HashMap<String, f64>,
) -> Result<(), nvml_wrapper::error::NvmlErrorWithSource> {
    let device_count = nvml.device_count()?;

    for i in 0..device_count {
        let device = nvml.device_by_index(i)?;
        dump_gpu_stat(device, results);
    }

    return Ok(());
}

pub fn get_nvidia_gpu_counter(results: &mut HashMap<String, f64>) {
    match NVML::init() {
        Ok(nvml) => dump_all_gpu_stats(&nvml, results).unwrap(),
        Err(_) => {
            // nvml not available
        }
    }
}
