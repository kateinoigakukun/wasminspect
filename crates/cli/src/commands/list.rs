use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use super::sourcemap::{LineInfo, ColumnType};
use anyhow::{Result, anyhow};

pub struct ListCommand {}

impl ListCommand {
    pub fn new() -> Self {
        Self {}
    }
}

impl<D: Debugger> Command<D> for ListCommand {
    fn name(&self) -> &'static str {
        "list"
    }

    fn run(&self, debugger: &mut D, context: &CommandContext, args: Vec<&str>) -> Result<()> {
        let (insts, next_index) = debugger.instructions()?;
        let first_inst = insts[0].clone();
        let line_info: LineInfo = match context.sourcemap.find_line_info(first_inst.offset) {
            Some(info) => info,
            None => return Err(anyhow!("Source info not found")),
        };
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        let source = BufReader::new(File::open(line_info.filepath)?);
        for (index, line) in source.lines().enumerate() {
            let out = if Some(index as u64) == line_info.line {
                let mut out = format!("-> {} ", index);
                match line_info.column {
                    ColumnType::Column(col) => {
                        for (col_index, col_char) in line.iter().enumerate() {
                            if col_index as u64 == col {
                                out = format!("{}\x1B[4;34m{}\x1B[0m", out, col_char);
                            } else {
                                out = format!("{}{}", out, col_char);
                            }
                        }
                    }
                    ColumnType::LeftEdge => {
                        out = format!("{}{}", out, line?);
                    }
                }
                out
            } else {
                format!("   {} {}", index, line?)
            };
            println!("{}", out);
        }
        Ok(())
    }
}
