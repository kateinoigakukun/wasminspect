use pretty_env_logger;
use structopt::StructOpt;
use wasminspect_cli;

#[derive(StructOpt)]
struct Opts {
    /// The wasm binary file
    #[structopt(name = "FILE")]
    filepath: Option<String>,
    /// Tells the debugger to read in and execute the debugger commands in given file, after wasm file has been loaded
    #[structopt(short, long)]
    source: Option<String>,
}

fn main() {
    pretty_env_logger::init();
    let opts = Opts::from_args();
    match wasminspect_cli::run_loop(opts.filepath, opts.source) {
        Err(err) => println!("{:?}", err),
        _ => {}
    }
}
