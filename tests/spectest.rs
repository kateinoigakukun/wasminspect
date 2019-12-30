extern crate wast_spec;
use std::path::Path;
use wast_spec::WastContext;

macro_rules! run_wast {
    ($file:expr, $func_name:ident) => {
        #[test]
        fn $func_name() {
            run_spectest($file)
        }
    };
}

fn run_spectest(filename: &str) {
    let testsuite_dir = Path::new(file!()).parent().unwrap().join("testsuite");
    let mut context = WastContext::new();
    let _ = context.run_file(&testsuite_dir.join(filename));
}

run_wast!("br.wast", test_wast_br);
run_wast!("br_if.wast", test_wast_br_if);