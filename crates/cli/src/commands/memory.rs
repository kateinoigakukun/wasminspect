use super::command::{self, Command, Interface};
use super::debugger::Debugger;
use clap::{App, Arg};
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
}

impl<D: Debugger> Command<D> for MemoryCommand {
    fn name(&self) -> &'static str {
        "memory"
    }
    fn run(&self, debugger: &mut D, interface: &Interface, args: Vec<&str>) -> Result<(), command::Error> {
        let opts = match Opts::from_iter_safe(args) {
            Ok(opts) => opts,
            Err(e) => return Err(command::Error::Command(format!("{}", e))),
        };
        match opts {
            Opts::Read { address, count } => {
                let address = if address.starts_with("0x") {
                    let raw = address.trim_start_matches("0x");
                    i64::from_str_radix(raw, 16)
                        .map_err(|e| (command::Error::Command(format!("{}", e))))?
                } else {
                    i64::from_str_radix(&address, 10)
                        .map_err(|e| (command::Error::Command(format!("{}", e))))?
                };
                let memory = debugger.memory().map_err(command::Error::Command)?;

                let begin = address as usize;
                let end = begin + (count as usize);
                let chunk_size = 16;
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
