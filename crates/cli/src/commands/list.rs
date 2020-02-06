use super::command::{Command, CommandContext};
use super::debugger::Debugger;
use super::sourcemap::{ColumnType, LineInfo, SourceMap};
use anyhow::{anyhow, Result};

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

    fn run(&self, debugger: &mut D, context: &CommandContext, _args: Vec<&str>) -> Result<()> {
        let (insts, next_index) = debugger.instructions()?;
        let current_index = if next_index == 0 { 0 } else { next_index - 1 };
        let first_inst = insts[current_index].clone();
        display_source(first_inst.offset, &context.sourcemap)
    }
}

pub fn display_source(offset: usize, sourcemap: &Box<dyn SourceMap>) -> Result<()> {
    let line_info: LineInfo = match sourcemap.find_line_info(offset) {
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
