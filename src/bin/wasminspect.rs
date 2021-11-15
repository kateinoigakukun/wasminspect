use env_logger;
use std::io::Read;
use structopt::StructOpt;
use wasminspect_debugger::{self, ModuleInput};
use anyhow::anyhow;


fn parse_map_dirs(s: &str) -> anyhow::Result<(String, String)> {
    let parts: Vec<&str> = s.split("::").collect();
    if parts.len() != 2 {
        return Err(anyhow!("must contain exactly one double colon ('::')"));
    }
    Ok((parts[0].into(), parts[1].into()))
}


#[derive(StructOpt)]
struct Opts {
    /// The wasm binary file
    #[structopt(name = "FILE")]
    filepath: Option<String>,
    /// Tells the debugger to read in and execute the debugger commands in given file, after wasm file has been loaded
    #[structopt(short, long)]
    source: Option<String>,
    /// Grant access to a guest directory mapped as a host directory
    #[structopt(long = "mapdir", number_of_values = 1, value_name = "GUEST_DIR::HOST_DIR", parse(try_from_str = parse_map_dirs))]
    map_dirs: Vec<(String, String)>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("warn"));

    let opts = Opts::from_args();
    let module_input = match opts.filepath {
        Some(filepath) => {
            let mut buffer = Vec::new();
            let filepath = std::path::Path::new(&filepath);
            let basename = filepath
                .file_name()
                .expect("invalid file path")
                .to_str()
                .expect("invalid file name encoding")
                .to_string();
            let mut f = std::fs::File::open(filepath)?;
            f.read_to_end(&mut buffer)?;
            Some(ModuleInput {
                bytes: buffer,
                basename,
            })
        }
        None => None,
    };
    Ok(
        match wasminspect_debugger::run_loop(module_input, opts.source, opts.map_dirs) {
            Err(err) => println!("{:?}", err),
            _ => {}
        },
    )
}
