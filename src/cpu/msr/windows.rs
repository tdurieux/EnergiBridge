#![cfg(target_os = "windows")]

use once_cell::sync::OnceCell;
use std::{ffi::CString, sync::Once};
use std::{
    fs::{File, OpenOptions},
};
use thiserror::Error;
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{GENERIC_READ, HANDLE},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        Storage::FileSystem::{CreateFileA, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, OPEN_EXISTING},
        System::{
            Threading::{GetCurrentProcess, OpenProcessToken},
            IO::DeviceIoControl,
        },
    },
};

use crate::cpu::msr::windows::pawnio::PawnIO;

mod pawnio;

#[derive(Error, Debug)]
pub enum RaplError {
    #[cfg(target_os = "windows")]
    #[error("windows error")]
    Windows(#[from] windows::core::Error),
}

const IOCTL_OLS_READ_MSR: u32 = 0x9C402084;

//static RAPL_STOP: AtomicU64 = AtomicU64::new(0);

static RAPL_INIT: Once = Once::new();
static RAPL_DRIVER: OnceCell<PawnIO> = OnceCell::new();

pub fn start_rapl_impl(mut sys: &mut sysinfo::System) {
    // Initialize RAPL driver on first call
    RAPL_INIT.call_once(|| {
        // Check if running as admin due to the driver requirement
        if !is_admin() {
            panic!("not running as admin, this is required for the RAPL driver to work");
        }

        let pawn_io = PawnIO::new(&mut sys).expect("Failed to initialize PawnIO for RAPL driver");
        RAPL_DRIVER.get_or_init(|| pawn_io);
    });
}

// check if running as admin using the windows crate
fn is_admin() -> bool {
    let mut h_token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut h_token as _) }.unwrap();

    let mut token_elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let token_elevation_ptr = &mut token_elevation as *mut TOKEN_ELEVATION;
    let mut cb_size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

    unsafe {
        GetTokenInformation(
            h_token,
            TokenElevation,
            Some(token_elevation_ptr as _),
            cb_size,
            &mut cb_size as _,
        )
        .unwrap();
    }

    token_elevation.TokenIsElevated != 0
}

pub fn read_msr_on_core(msr: u32, core: u32) -> Result<u64, std::io::Error> {
    // Get the driver handle
    let pawn_io_driver = RAPL_DRIVER.get().expect("RAPL driver not initialized");

    // Pin the current thread to the target core before reading the MSR.
    // PawnIO's ioctl_read_msr executes RDMSR on whichever core the calling
    // thread is scheduled on, so we must set affinity first
    let prev_affinity = set_thread_affinity_to_core(core)?;

    let input = [msr as u64];
    let mut output = [0u64; 1];

    let result = match pawn_io_driver.execute("ioctl_read_msr", Some(&input), Some(&mut output)) {
        Ok(_) => Ok(output[0]),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, format!("PawnIO error with HRESULT: {}", e))),
    };

    // Restore the original thread affinity
    restore_thread_affinity(prev_affinity);

    result
}

/// Set the current thread's affinity to a single logical core.
/// Returns the previous affinity mask so it can be restored.
fn set_thread_affinity_to_core(core: u32) -> Result<usize, std::io::Error> {
    use windows::Win32::System::Threading::{GetCurrentThread, SetThreadAffinityMask};

    let mask: usize = 1 << core;
    let prev = unsafe { SetThreadAffinityMask(GetCurrentThread(), mask) };
    if prev == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(prev)
}

/// Restore the thread affinity to a previously saved mask.
fn restore_thread_affinity(mask: usize) {
    use windows::Win32::System::Threading::{GetCurrentThread, SetThreadAffinityMask};

    unsafe {
        SetThreadAffinityMask(GetCurrentThread(), mask);
    }
}

pub fn close_rapl() {
    // RAPL_STOP.store(1, Ordering::SeqCst);
    let pawn_io_driver = RAPL_DRIVER.get().expect("RAPL driver not initialized");
    
    pawn_io_driver.close().expect("Failed to close PawnIO driver handle");
}