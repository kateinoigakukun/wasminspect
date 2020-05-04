use std::env;
use std::path::Path;
use std::process::Command;

fn default_toolchain_dir() -> String {
    let xcrun_output = Command::new("/usr/bin/xcrun")
        .arg("--find")
        .arg("swiftc")
        .output()
        .expect("failed to execute xcrun");
    let swiftc_path_str = String::from_utf8(xcrun_output.stdout).unwrap();
    let swiftc_path = Path::new(&swiftc_path_str);
    let bin_path = swiftc_path.parent().unwrap();
    let toolchain_path = bin_path.parent().unwrap();
    toolchain_path.to_str().unwrap().to_string()
}

static SWIFT_TOOLCHAIN_DIR: &'static str = "SWIFT_TOOLCHAIN_DIR";
fn main() {
    let toolchain_dir = match env::var(SWIFT_TOOLCHAIN_DIR) {
        Ok(val) => val,
        Err(_) => {
            println!("Environment variable {} not found", SWIFT_TOOLCHAIN_DIR);
            default_toolchain_dir()
        }
    };
    println!("cargo:rustc-link-search=native=/usr/lib/swift");
    println!("cargo:rustc-link-lib=dylib=swiftCore");
}
