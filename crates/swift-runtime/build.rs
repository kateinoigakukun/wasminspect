use std::env;

static SWIFT_RUNTIME_LIB_DIR: &'static str = "SWIFT_RUNTIME_LIB_DIR";

fn main() {
    let runtime_lib_dir = match env::var(SWIFT_RUNTIME_LIB_DIR) {
        Ok(val) => val,
        Err(_) => {
            println!("Environment variable {} not found", SWIFT_RUNTIME_LIB_DIR);
            "/usr/lib/swift".to_string()
        }
    };
    println!("cargo:rerun-if-env-changed={}", SWIFT_RUNTIME_LIB_DIR);
    println!("cargo:rustc-link-search=native={}", runtime_lib_dir);
    println!("cargo:rustc-link-lib=dylib=swiftCore");
}
