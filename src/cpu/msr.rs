use once_cell::sync::OnceCell;
use std::sync::Once;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;

// https://github.com/cs-23-pt-9-01/rapl-interface

pub fn start_rapl() {    
    #[cfg(target_os = "windows")]
    windows::start_rapl_impl();
}

pub unsafe fn read_msr_on_core(msr: u32, core: u32) -> Result<u64, std::io::Error> {
    unsafe {
        #[cfg(target_os = "windows")]
        return windows::read_msr_on_core(msr, core);
        #[cfg(target_os = "linux")]
        return linux::read_msr_on_core(msr, core);
    }
}
