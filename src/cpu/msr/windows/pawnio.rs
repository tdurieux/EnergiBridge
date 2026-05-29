#![cfg(target_os = "windows")]

use libloading::Library;
use std::ffi::CString;
use std::path::PathBuf;
use std::ptr;
use sysinfo::{CpuExt, System, SystemExt};

// Type aliases matching PawnIOLib.h signatures
// PAWNIOAPI = EXTERN_C HRESULT STDAPICALLTYPE
type PawnioVersion = unsafe extern "system" fn(version: *mut u32) -> i32;
type PawnioOpen = unsafe extern "system" fn(handle: *mut *mut std::ffi::c_void) -> i32;
type PawnioLoad = unsafe extern "system" fn(
    handle: *mut std::ffi::c_void,
    blob: *const u8,
    size: usize,
) -> i32;
type PawnioExecute = unsafe extern "system" fn(
    handle: *mut std::ffi::c_void,
    name: *const i8,
    input: *const u64,
    in_size: usize,
    output: *mut u64,
    out_size: usize,
    return_size: *mut usize,
) -> i32;
type PawnioClose = unsafe extern "system" fn(handle: *mut std::ffi::c_void) -> i32;

const INTEL_MSR_BLOB: &[u8] = include_bytes!("../../../../resources/blobs/IntelMSR.bin");
const AMD_FAMILY17_BLOB: &[u8] = include_bytes!("../../../../resources/blobs/AMDFamily17.bin");

/// HRESULT success check
fn succeeded(hr: i32) -> bool {
    hr >= 0
}

fn select_blob_for_vendor(vendor: &str) -> (&'static str, &'static [u8]) {
    if vendor == "GenuineIntel" {
        ("IntelMSR.bin", INTEL_MSR_BLOB)
    } else if vendor == "AuthenticAMD" {
        ("AMDFamily17.bin", AMD_FAMILY17_BLOB)
    } else {
        eprintln!(
            "[PawnIO] Unknown CPU vendor '{}', using AMD blob as fallback",
            vendor
        );
        ("AMDFamily17.bin", AMD_FAMILY17_BLOB)
    }
}

/// Step 1: Find PawnIO install location from the Windows registry.
/// Reads `InstallLocation` from
/// `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO`.
/// Falls back to `C:\Program Files\PawnIO` if the registry key is not found.
fn find_pawnio_install_location() -> PathBuf {
    use winapi::um::winreg::{RegOpenKeyExA, RegQueryValueExA, HKEY_LOCAL_MACHINE};
    use winapi::um::winnt::{KEY_READ, REG_SZ};

    unsafe {
        let subkey =
            CString::new(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO").unwrap();
        let value_name = CString::new("InstallLocation").unwrap();
        let mut hkey: winapi::shared::minwindef::HKEY = ptr::null_mut();

        let status = RegOpenKeyExA(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        );

        if status == 0 {
            let mut data_type: u32 = 0;
            let mut buffer = [0u8; 512];
            let mut buffer_size: u32 = buffer.len() as u32;

            let status = RegQueryValueExA(
                hkey,
                value_name.as_ptr(),
                ptr::null_mut(),
                &mut data_type,
                buffer.as_mut_ptr(),
                &mut buffer_size,
            );

            // Close the key regardless of query result
            winapi::um::winreg::RegCloseKey(hkey);

            if status == 0 && data_type == REG_SZ {
                let path_str = String::from_utf8_lossy(&buffer[..buffer_size as usize])
                    .trim_end_matches('\0')
                    .to_string();
                if !path_str.is_empty() {


                    // Print only on debug builds to avoid cluttering release output
                    #[cfg(debug_assertions)]
                    println!("[PawnIO] Found install location from registry: {}", path_str);

                    return PathBuf::from(path_str);
                }
            }
        }
    }

    // Fallback path
    let fallback = PathBuf::from(r"C:\Program Files\PawnIO");
    
    #[cfg(debug_assertions)]
    println!(
        "[PawnIO] Registry key not found, using fallback: {}",
        fallback.display()
    );

    fallback
}

/// Holds a loaded PawnIOLib and its resolved function pointers.
pub struct PawnIO {
    _lib: Library,
    pawnio_version: PawnioVersion,
    pawnio_open: PawnioOpen,
    pawnio_load: PawnioLoad,
    pawnio_execute: PawnioExecute,
    pawnio_close: PawnioClose,
    pawnio_module_handle: Option<*mut std::ffi::c_void>,
}

impl PawnIO {
    /// Steps 1-3: Locate PawnIO, load PawnIOLib.dll, and resolve all functions.
    pub fn new(sys: &mut System) -> Result<Self, Box<dyn std::error::Error>> {
        // Step 1: Find install location
        let install_path = find_pawnio_install_location();
        let dll_path = install_path.join("PawnIOLib.dll");

        #[cfg(debug_assertions)]
        println!("[PawnIO] Loading library from: {}", dll_path.display());

        // Step 2: Load PawnIOLib
        let lib = unsafe { Library::new(&dll_path) }
            .map_err(|e| format!("Failed to load PawnIOLib.dll: {}", e))?;

        // Step 3: Resolve functions
        unsafe {
            let fn_version: PawnioVersion = *lib
                .get::<PawnioVersion>(b"pawnio_version")
                .map_err(|e| format!("Failed to resolve pawnio_version: {}", e))?;
            let fn_open: PawnioOpen = *lib
                .get::<PawnioOpen>(b"pawnio_open")
                .map_err(|e| format!("Failed to resolve pawnio_open: {}", e))?;
            let fn_load: PawnioLoad = *lib
                .get::<PawnioLoad>(b"pawnio_load")
                .map_err(|e| format!("Failed to resolve pawnio_load: {}", e))?;
            let fn_execute: PawnioExecute = *lib
                .get::<PawnioExecute>(b"pawnio_execute")
                .map_err(|e| format!("Failed to resolve pawnio_execute: {}", e))?;
            let fn_close: PawnioClose = *lib
                .get::<PawnioClose>(b"pawnio_close")
                .map_err(|e| format!("Failed to resolve pawnio_close: {}", e))?;

            let mut pawn_io = PawnIO {
                _lib: lib,
                pawnio_version: fn_version,
                pawnio_open: fn_open,
                pawnio_load: fn_load,
                pawnio_execute: fn_execute,
                pawnio_close: fn_close,
                pawnio_module_handle: None,
            };

            sys.refresh_cpu();

            let vendor = sys.global_cpu_info().vendor_id();
            let (blob_name, blob) = select_blob_for_vendor(vendor);

            #[cfg(debug_assertions)]
            println!("[PawnIO] Selected embedded blob '{}' ({} bytes)", blob_name, blob.len());

            let mut handle: *mut std::ffi::c_void = ptr::null_mut();

            // Open a new PawnIO module
            let hr = (pawn_io.pawnio_open)(&mut handle);
            if !succeeded(hr) {
                eprintln!("[PawnIO] pawnio_open failed with HRESULT: 0x{:08X}", hr as u32);
                return Err(format!("pawnio_open failed with HRESULT: 0x{:08X}", hr as u32).into());
            }

            #[cfg(debug_assertions)]
            println!("[PawnIO] Handle opened successfully.");

            let hr = (pawn_io.pawnio_load)(handle, blob.as_ptr(), blob.len());
            if !succeeded(hr) {
                eprintln!("[PawnIO] pawnio_load failed with HRESULT: 0x{:08X}", hr as u32);
                // Clean up the handle on failure
                (pawn_io.pawnio_close)(handle);
                return Err(format!("pawnio_load failed with HRESULT: 0x{:08X}", hr as u32).into());
            }

            #[cfg(debug_assertions)]
            println!("[PawnIO] Blob loaded successfully ({} bytes).", blob.len());
            
            pawn_io.pawnio_module_handle = Some(handle);

            Ok(pawn_io)
        }
    }

    /// Get the PawnIOLib version.
    fn version(&self) -> Result<(u8, u8, u8), i32> {
        let mut ver: u32 = 0;
        let hr = unsafe { (self.pawnio_version)(&mut ver) };
        if succeeded(hr) {
            let major = ((ver >> 16) & 0xFF) as u8;
            let minor = ((ver >> 8) & 0xFF) as u8;
            let patch = (ver & 0xFF) as u8;
            Ok((major, minor, patch))
        } else {
            Err(hr)
        }
    }

    /// Step 6: Execute an ioctl on a loaded module.
    pub fn execute(
        &self,
        function_name: &str,
        input: Option<&[u64]>,
        output: Option<&mut [u64]>,
    ) -> Result<usize, i32> {
        let name = CString::new(function_name).expect("Invalid function name");
        let mut return_size: usize = 0;

        let (in_ptr, in_size) = match input {
            Some(buf) => (buf.as_ptr(), buf.len()),
            None => (ptr::null(), 0),
        };

        let (out_ptr, out_size) = match output {
            Some(buf) => (buf.as_mut_ptr(), buf.len()),
            None => (ptr::null_mut(), 0),
        };

        let hr = unsafe {
            (self.pawnio_execute)(
                self.pawnio_module_handle.unwrap(),
                name.as_ptr(),
                in_ptr,
                in_size,
                out_ptr,
                out_size,
                &mut return_size,
            )
        };

        if !succeeded(hr) {
            eprintln!(
                "[PawnIO] pawnio_execute('{}') failed with HRESULT: 0x{:08X}",
                function_name, hr as u32
            );
            return Err(hr);
        }

        #[cfg(debug_assertions)]
        println!(
            "[PawnIO] pawnio_execute('{}') succeeded, return_size = {}",
            function_name, return_size
        );
        Ok(return_size)
    }

    /// Step 7: Close a PawnIO handle.
    pub fn close(&self) -> Result<(), i32> {
        let handle = self.pawnio_module_handle.unwrap();
        let hr = unsafe { (self.pawnio_close)(handle) };
        if !succeeded(hr) {
            eprintln!("[PawnIO] pawnio_close failed with HRESULT: 0x{:08X}", hr as u32);
            return Err(hr);
        }

        #[cfg(debug_assertions)]
        println!("[PawnIO] Handle closed successfully.");
        
        Ok(())
    }
}

unsafe impl Send for PawnIO {}
unsafe impl Sync for PawnIO {}
