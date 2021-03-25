use env_logger;
use std::io::Read;
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

fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("warn"));

    let opts = Opts::from_args();
    let buffer = match opts.filepath {
        Some(filepath) => {
            let mut buffer = Vec::new();
            let mut f = std::fs::File::open(filepath)?;
            f.read_to_end(&mut buffer)?;
            Some(buffer)
        }
        None => None,
    };
    Ok(match wasminspect_debugger::run_loop(buffer, opts.source) {
        Err(err) => println!("{:?}", err),
        _ => {}
    })
}
