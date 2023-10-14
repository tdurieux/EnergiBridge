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

#[derive(Error, Debug)]
pub enum RaplError {
    #[cfg(target_os = "windows")]
    #[error("windows error")]
    Windows(#[from] windows::core::Error),
}

const IOCTL_OLS_READ_MSR: u32 = 0x9C402084;

//static RAPL_STOP: AtomicU64 = AtomicU64::new(0);

static RAPL_INIT: Once = Once::new();
static RAPL_DRIVER: OnceCell<HANDLE> = OnceCell::new();

pub fn start_rapl_impl() {
    // Initialize RAPL driver on first call
    RAPL_INIT.call_once(|| {
        // Check if running as admin due to the driver requirement
        if !is_admin() {
            panic!("not running as admin, this is required for the RAPL driver to work");
        }

        let h_device = open_driver()
            .expect("failed to open driver handle, make sure the driver is installed and running");
        RAPL_DRIVER.get_or_init(|| h_device);
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

fn open_driver() -> Result<HANDLE, RaplError> {
    let driver_name = CString::new("\\\\.\\WinRing0_1_2_0").expect("failed to create driver name");
    Ok(unsafe {
        CreateFileA(
            PCSTR(driver_name.as_ptr() as *const u8), // File path
            GENERIC_READ.0,                           // Access mode (read-only in this example)
            FILE_SHARE_READ,                          // Share mode (0 for exclusive access)
            None,                                     // Security attributes (can be None)
            OPEN_EXISTING,                            // Creation disposition
            FILE_ATTRIBUTE_NORMAL,                    // File attributes (normal for regular files)
            None,                                     // Template file (not used here)
        )
    }?)
}

pub unsafe fn read_msr_on_core(msr: u32, core: u32) -> Result<u64, std::io::Error> {
    // Get the driver handle
    let rapl_driver = *RAPL_DRIVER.get().expect("RAPL driver not initialized");

    // Convert the MSR to a little endian byte array
    let input_data: [u8; 4] = msr.to_le_bytes();

    // Create an empty byte array to store the output
    let output_data: [u8; 8] = [0; 8];
    let mut lp_bytes_returned: u32 = 0;

    // Call the driver to read the MSR
    unsafe {
        DeviceIoControl(
            rapl_driver,
            IOCTL_OLS_READ_MSR,
            Some(input_data.as_ptr() as _),
            input_data.len() as u32,
            Some(output_data.as_ptr() as _),
            output_data.len() as u32,
            Some(&mut lp_bytes_returned as _),
            None,
        )
    }?;

    // TODO: Consider using lp_bytes_returned for error handling or logging it, it is supposed to return 8 bytes on success
    //println!("lp_bytes_returned: {}", lp_bytes_returned);
    Ok(u64::from_le_bytes(output_data))
}