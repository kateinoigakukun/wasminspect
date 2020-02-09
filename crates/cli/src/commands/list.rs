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

    fn description(&self) -> &'static str {
        "List relevant source code."
    }

    fn run(&self, debugger: &mut D, context: &CommandContext, _args: Vec<&str>) -> Result<()> {
        let line_info = next_line_info(debugger, &context.sourcemap)?;
        display_source(line_info)
    }
}

pub fn next_line_info<D: Debugger>(
    debugger: &D,
    sourcemap: &Box<dyn SourceMap>,
) -> Result<LineInfo> {
    let (insts, next_index) = debugger.instructions()?;
    match sourcemap.find_line_info(insts[next_index].offset) {
        Some(info) => Ok(info),
        None => Err(anyhow!("Source info not found")),
    }
}

pub fn display_source(line_info: LineInfo) -> Result<()> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    let source = BufReader::new(File::open(line_info.filepath)?);
    // In case compiler can't determine source code location. Page 151.
    if line_info.line == Some(0) {
        return Ok(())
    }
    let range = line_info.line.map(|l| {
        if l < 20 {
            (0..(l + 20))
        } else {
            (l - 20)..(l + 20)
        }
    });
    for (index, line) in source.lines().enumerate() {
        // line_info.line begin with 1
        let index = index + 1;
        let line = line?;

        let should_display = range.as_ref().map(|r| r.contains(&(index as u64)));
        if !(should_display.unwrap_or(true)) {
            continue;
        }
        let out = if Some(index as u64) == line_info.line {
            let mut out = format!("-> {: <4} ", index);
            match line_info.column {
                ColumnType::Column(col) => {
                    for (col_index, col_char) in line.chars().enumerate() {
                        if (col_index + 1) as u64 == col {
                            out = format!("{}\x1B[4m{}\x1B[0m", out, col_char);
                        } else {
                            out = format!("{}{}", out, col_char);
                        }
                    }
                }
                ColumnType::LeftEdge => {
                    out = format!("{}{}", out, line);
                }
            }
            out
        } else {
            format!("   {: <4} {}", index, line)
        };
        println!("{}", out);
    }
    Ok(())
}
