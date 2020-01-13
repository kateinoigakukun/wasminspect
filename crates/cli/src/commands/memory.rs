use super::command::{self, Command};
use super::debugger::Debugger;
use structopt::StructOpt;

pub struct MemoryCommand {}

impl MemoryCommand {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(StructOpt)]
struct Opts {
    #[structopt(name = "ADDRESS")]
    address: String,
    #[structopt(short, long, default_value = "32")]
    count: u32,
}

impl<D: Debugger> Command<D> for MemoryCommand {
    fn name(&self) -> &'static str {
        "memory"
    }
    fn run(&self, debugger: &mut D, args: Vec<&str>) -> Result<(), command::Error> {
        let opts = match Opts::from_iter_safe(args) {
            Ok(opts) => opts,
            Err(e) => return Err(command::Error::Command(format!("{}", e))),
        };
        let address = if opts.address.starts_with("0x") {
            let raw = opts.address.trim_start_matches("0x");
            i64::from_str_radix(raw, 16).map_err(|e| (command::Error::Command(format!("{}", e))))?
        } else {
            i64::from_str_radix(&opts.address, 10)
                .map_err(|e| (command::Error::Command(format!("{}", e))))?
        };
        let memory = debugger.memory().map_err(command::Error::Command)?;

        let begin = address as usize;
        let end = begin + (opts.count as usize);
        for (offset, bytes) in memory[begin..end].chunks(16).enumerate() {
            print!("0x{:>04x}: ", begin + offset);
            let bytes_str = bytes
                .iter()
                .map(|b| format!("{:>02x}", b))
                .collect::<Vec<String>>();
            print!("{}", bytes_str.join(" "));
            println!(" {}", dump_memory_as_str(bytes));
        }
        Ok(())
    }
}

use std::str;
fn dump_memory_as_str(bytes: &[u8]) -> String {
    let mut v = Vec::new();
    for byte in bytes.into_iter() {
        let byte = *byte;
        let byte = if byte > 0x1f && byte < 0x7f {
            let byte = vec![byte];
            str::from_utf8(&byte).unwrap_or(".").to_string()
        } else {
            ".".to_string()
        };
        v.push(byte)
    }
    v.join("")
}
