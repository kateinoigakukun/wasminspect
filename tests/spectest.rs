extern crate wast_spec;
use std::path::Path;
use wast_spec::WastContext;

include!(concat!(env!("OUT_DIR"), "/wast_testsuite_tests.rs"));

fn run_wast(wast: &str) -> anyhow::Result<()> {
    let wast = Path::new(wast);

    let mut cfg = wasminspect_vm::Config::default();

    cfg.features.simd = feature_found(wast, "simd");
    cfg.features.memory64 = feature_found(wast, "memory64");
    cfg.features.multi_memory = feature_found(wast, "multi-memory");
    cfg.features.component_model = feature_found(wast, "component-model");
    cfg.features.threads = feature_found(wast, "threads");

    let mut context = WastContext::new(cfg);
    match context.run_file(wast) {
        Ok(_) => (),
        Err(err) => panic!("{}", err),
    }
    Ok(())
}

fn feature_found(path: &Path, name: &str) -> bool {
    path.iter().any(|part| match part.to_str() {
        Some(s) => s.contains(name),
        None => false,
    })
}
