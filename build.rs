use sysinfo::{CpuExt, System, SystemExt};

fn main() {
    if std::env::var("TARGET").unwrap().contains("-apple") {
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }

    let sys = System::new_all();
    let cpu = sys.cpus().first().expect("failed getting CPU").vendor_id();
    match cpu {
        "GenuineIntel" => println!("cargo:rustc-cfg=intel"),
        "AuthenticAMD" => println!("cargo:rustc-cfg=amd"),
        "Apple" => println!("cargo:rustc-cfg=apple"),
        _ => {
            panic!("unknown CPU detected: {}", cpu);
        }
    };
}
