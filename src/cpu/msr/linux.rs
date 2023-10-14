#![cfg(target_os = "linux")]

use once_cell::sync::OnceCell;
use std::{ffi::CString, sync::Once};
use std::{
    fs::{File, OpenOptions},
};

use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;


pub fn read_msr_on_core(msr: u32, core: u32) -> Result<u64, std::io::Error> {
    let mut file = File::open(format!("/dev/cpu/{}/msr", core))?;

    // Seek to the MSR address
    file.seek(SeekFrom::Start(u64::from(msr)))?;

    // Read the 8-byte MSR value
    let mut value_bytes = [0u8; 8];
    file.read_exact(&mut value_bytes)?;

    let value = u64::from_le_bytes(value_bytes);

    Ok(value)
}
