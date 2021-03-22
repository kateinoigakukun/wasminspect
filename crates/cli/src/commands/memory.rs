use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use anyhow::{anyhow, Result};

use structopt::StructOpt;

pub struct MemoryCommand {}

impl MemoryCommand {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "read")]
    Read {
        #[structopt(name = "ADDRESS")]
        address: String,
        #[structopt(short, long, default_value = "32")]
        count: u32,
    },
    #[structopt(name = "enable-watch")]
    EnableWatch
}

impl<D: Debugger> Command<D> for MemoryCommand {
    fn name(&self) -> &'static str {
        "memory"
    }

    fn description(&self) -> &'static str {
        "Commands for operating on memory."
    }
    fn run(&self, debugger: &mut D, _context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let opts = Opts::from_iter_safe(args)?;
        match opts {
            Opts::Read { address, count } => {
                let address = if address.starts_with("0x") {
                    let raw = address.trim_start_matches("0x");
                    i64::from_str_radix(raw, 16)?
                } else {
                    i64::from_str_radix(&address, 10)?
                };
                let memory = debugger.memory()?;

                let begin = address as usize;
                let end = begin + (count as usize);
                let chunk_size = 16;
                if memory.len() <= end {
                    return Err(anyhow!("index {} out of range for slice of length {}", end, memory.len()));
                }
                for (offset, bytes) in memory[begin..end].chunks(chunk_size).enumerate() {
                    print!("0x{:>08x}: ", begin + offset * chunk_size);
                    let bytes_str = bytes
                        .iter()
                        .map(|b| format!("{:>02x}", b))
                        .collect::<Vec<String>>();
                    print!("{}", bytes_str.join(" "));
                    println!(" {}", dump_memory_as_str(bytes));
                }
                Ok(())
            }
            Opts::EnableWatch => {
                let mut opts = debugger.get_opts();
                opts.watch_memory = true;
                debugger.set_opts(opts);
                Ok(())
            }
        }
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
