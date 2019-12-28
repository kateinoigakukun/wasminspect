use clap::{App, Arg};
use wasminspect_core::interpreter::{WasmInstance, WasmValue};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let mut app = App::new("wasminspect")
        .version(VERSION)
        .arg(Arg::with_name("file").help("The wasm binary file"))
        .arg(
            Arg::with_name("start_func")
                .help("The function to start")
                .long("start-func")
                .short("f")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("args")
                .help("The arguments passed to function")
                .long("args")
                .short("a")
                .takes_value(true),
        );
    let matches = match app.get_matches_from_safe_borrow(::std::env::args_os()) {
        Ok(matches) => matches,
        Err(err) => {
            eprintln!("{}", err);
            ::std::process::exit(1);
        }
    };
    let func = matches.value_of("start_func").map(|f| f.to_string());
    let arguments = matches
        .values_of("args")
        .map(|c| c.collect())
        .unwrap_or(vec![]);
    if let Some(path) = matches.value_of("file") {
        let instance = WasmInstance::new(path.to_string());
        match instance.run(
            func,
            arguments
                .iter()
                .map(|s| WasmValue::I32(s.parse().unwrap()))
                .collect(),
        ) {
            Ok(result) => println!("1 + 2 = {:?}", result[0]),
            Err(err) => panic!(err.message()),
        }
    } else {
        eprintln!("error: wasm file is required");
        let _ = app.print_long_help();
        ::std::process::exit(1);
    }
}
