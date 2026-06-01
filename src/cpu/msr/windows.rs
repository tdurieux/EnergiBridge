#![cfg(target_os = "windows")]

use once_cell::sync::OnceCell;
use std::collections::BTreeMap;
use std::{sync::Once};
use thiserror::Error;
use windows::{
    Win32::{
        Foundation::{HANDLE},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::{
            Threading::{GetCurrentProcess, OpenProcessToken},
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

//static RAPL_STOP: AtomicU64 = AtomicU64::new(0);

static RAPL_INIT: Once = Once::new();
static RAPL_DRIVER: OnceCell<PawnIO> = OnceCell::new();
static PHYSICAL_TO_LOGICAL_MAP: OnceCell<Vec<u32>> = OnceCell::new();

fn get_current_physical_core_key(fallback_key: u32) -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        let max_leaf = unsafe { std::arch::x86_64::__cpuid(0) }.eax;
        let topology_leaf = if max_leaf >= 0x1F {
            0x1F
        } else if max_leaf >= 0x0B {
            0x0B
        } else {
            return fallback_key;
        };

        let mut smt_shift: Option<u32> = None;
        for subleaf in 0..8 {
            let topology = unsafe { std::arch::x86_64::__cpuid_count(topology_leaf, subleaf) };
            let level_type = (topology.ecx >> 8) & 0xFF;
            if level_type == 0 {
                break;
            }
            if level_type == 1 {
                smt_shift = Some(topology.eax & 0x1F);
                break;
            }
        }

        let shift = match smt_shift {
            Some(shift) if shift > 0 => shift,
            _ => return fallback_key,
        };

        let topology = unsafe { std::arch::x86_64::__cpuid_count(topology_leaf, 0) };
        topology.edx >> shift
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        fallback_key
    }
}

fn build_physical_to_logical_map() -> Result<Vec<u32>, std::io::Error> {
    let core_ids = core_affinity::get_core_ids().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "failed to enumerate logical cores",
        )
    })?;

    let mut representatives = BTreeMap::<u32, u32>::new();

    for core in core_ids {
        if !core_affinity::set_for_current(core) {
            continue;
        }

        let logical = core.id as u32;
        let physical_key = get_current_physical_core_key(logical);
        representatives.entry(physical_key).or_insert(logical);
    }

    let mapped = representatives.into_values().collect::<Vec<u32>>();
    if mapped.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "failed to map physical cores",
        ));
    }
    Ok(mapped)
}

fn resolve_logical_core(physical_core: u32) -> Result<u32, std::io::Error> {
    let map = PHYSICAL_TO_LOGICAL_MAP.get_or_try_init(build_physical_to_logical_map)?;
    map.get(physical_core as usize).copied().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("physical core index {} out of range", physical_core),
        )
    })
}

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

    let logical_core = resolve_logical_core(core)?;

    if logical_core >= usize::BITS {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("logical core index {} exceeds affinity mask width {}", logical_core, usize::BITS),
        ));
    }

    let mask: usize = 1 << logical_core;
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
