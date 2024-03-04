mod amd;
#[cfg(target_os = "macos")]
mod apple;
mod nvidia;
use std::collections::HashMap;

#[cfg(not(target_os = "macos"))]
use amd::get_amd_gpu_counter;
#[cfg(target_os = "macos")]
use apple::get_apple_gpu_counter;
#[cfg(not(target_os = "macos"))]
use nvidia::get_nvidia_gpu_counter;

pub fn get_gpu_counter(results: &mut HashMap<String, f64>) {
    #[cfg(target_os = "macos")]
    get_apple_gpu_counter(results);
    #[cfg(not(target_os = "macos"))]
    get_nvidia_gpu_counter(results);
    #[cfg(not(target_os = "macos"))]
    get_amd_gpu_counter(results);
}
