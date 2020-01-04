use clap::{App, Arg};
use wasminspect_cli;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let mut app = App::new("wasminspect")
        .version(VERSION)
        .arg(Arg::with_name("file").help("The wasm binary file"));
    let matches = match app.get_matches_from_safe_borrow(::std::env::args_os()) {
        Ok(matches) => matches,
        Err(err) => {
            eprintln!("{}", err);
            ::std::process::exit(1);
        }
    };
    match wasminspect_cli::run_loop(matches.value_of("file").map(|s| s.to_string())) {
        Err(err) => eprintln!("{}", err),
        _ => {}
    }
}
