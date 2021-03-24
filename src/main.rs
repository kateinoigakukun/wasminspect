use env_logger;
use structopt::StructOpt;
use wasminspect_debugger;

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
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("warn"));

    let opts = Opts::from_args();
    match wasminspect_debugger::run_loop(opts.filepath, opts.source) {
        Err(err) => println!("{:?}", err),
        _ => {}
    }
}
