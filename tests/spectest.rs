extern crate wast_spec;
use std::path::Path;
use wast_spec::WastContext;

fn run_spectest(filename: &str) {
    let testsuite_dir = Path::new(file!()).parent().unwrap().join("testsuite");
    let mut context = WastContext::new();
    context.run_file(&testsuite_dir.join(filename));
}

#[test]
fn run_spectests() {
    run_spectest("br.wast");
}