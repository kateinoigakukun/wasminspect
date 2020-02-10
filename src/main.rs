use pretty_env_logger;
use structopt::StructOpt;
use wasminspect_cli;

#[derive(StructOpt)]
struct Opts {
    /// The wasm binary file
    #[structopt(name = "FILE")]
    filepath: Option<String>,
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
