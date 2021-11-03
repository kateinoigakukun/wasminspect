//! This file is based on https://github.com/bytecodealliance/wasmtime/blob/v0.30.0/build.rs

use std::env;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(
        env::var_os("OUT_DIR").expect("The OUT_DIR environment variable must be set"),
    );
    let mut out = String::new();
    with_test_module(&mut out, "spec", |out| {
        test_directory(out, "tests/testsuite")?;
        Ok(())
    })?;
    let output = out_dir.join("wast_testsuite_tests.rs");
    fs::write(&output, out)?;
    Ok(())
}

fn test_directory(out: &mut String, path: impl AsRef<Path>) -> Result<usize> {
    let path = path.as_ref();
    let mut dir_entries: Vec<_> = path
        .read_dir()?
        .map(|r| r.expect("reading testsuite directory entry"))
        .filter_map(|dir_entry| {
            let p = dir_entry.path();
            let ext = p.extension()?;
            // Only look at wast files.
            if ext != "wast" {
                return None;
            }
            // Ignore files starting with `.`, which could be editor temporary files
            if p.file_stem()?.to_str()?.starts_with(".") {
                return None;
            }
            Some(p)
        })
        .collect();

    dir_entries.sort();

    for entry in dir_entries.iter() {
        write_testsuite_tests(out, entry)?;
    }

    Ok(dir_entries.len())
}

fn filename_to_testname(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .file_stem()
        .expect("filename should have a stem")
        .to_str()
        .expect("filename should be representable as a string")
        .replace("-", "_")
        .replace("/", "_")
}

fn with_test_module<T>(
    out: &mut String,
    testsuite: &str,
    f: impl FnOnce(&mut String) -> Result<T>,
) -> Result<T> {
    out.push_str("mod ");
    out.push_str(testsuite);
    out.push_str(" {\n");

    let result = f(out)?;

    out.push_str("}\n");
    Ok(result)
}

fn write_testsuite_tests(out: &mut String, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let testname = filename_to_testname(path);

    writeln!(out, "#[test]")?;

    writeln!(out, "fn r#{}() {{", &testname,)?;
    writeln!(out, "    let _ = env_logger::try_init();")?;
    writeln!(
        out,
        "    crate::run_wast(r#\"{}\"#).unwrap();",
        path.display(),
    )?;
    writeln!(out, "}}")?;
    writeln!(out)?;
    Ok(())
}
