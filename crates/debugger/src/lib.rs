mod commands;
mod debugger;
mod dwarf;
mod process;

use std::{cell::RefCell, rc::Rc};

pub use commands::command::CommandContext;
pub use commands::command::CommandResult;
pub use commands::debugger::{Debugger, RunResult};
pub use debugger::MainDebugger;
pub use linefeed;
pub use process::Interactive;
pub use process::Process;

use anyhow::{anyhow, Result};
use commands::command;
use log::warn;

pub fn try_load_dwarf(
    buffer: &Vec<u8>,
    context: &mut commands::command::CommandContext,
) -> Result<()> {
    use dwarf::transform_dwarf;
    let debug_info = transform_dwarf(&buffer)?;
    context.sourcemap = Box::new(debug_info.sourcemap);
    context.subroutine = Box::new(debug_info.subroutine);
    Ok(())
}

struct ConsolePrinter {}
impl commands::debugger::OutputPrinter for ConsolePrinter {
    fn println(&self, output: &str) {
        println!("{}", output);
    }
    fn eprintln(&self, output: &str) {
        eprintln!("{}", output);
    }
}

pub struct ModuleInput {
    pub bytes: Vec<u8>,
    pub basename: String,
}

pub fn start_debugger<'a>(
    module_input: Option<ModuleInput>,
) -> Result<(
    process::Process<debugger::MainDebugger>,
    command::CommandContext,
)> {
    let mut debugger = debugger::MainDebugger::new()?;
    let mut context = commands::command::CommandContext {
        sourcemap: Box::new(commands::sourcemap::EmptySourceMap::new()),
        subroutine: Box::new(commands::subroutine::EmptySubroutineMap::new()),
        printer: Box::new(ConsolePrinter {}),
    };

    if let Some(ref module_input) = module_input {
        debugger.load_main_module(&module_input.bytes, module_input.basename.clone())?;
        match try_load_dwarf(&module_input.bytes, &mut context) {
            Ok(_) => (),
            Err(err) => {
                warn!("Failed to load dwarf info: {}", err);
            }
        }
    }
    let process = process::Process::new(
        debugger,
        vec![
            Box::new(commands::thread::ThreadCommand::new()),
            Box::new(commands::list::ListCommand::new()),
            Box::new(commands::memory::MemoryCommand::new()),
            Box::new(commands::stack::StackCommand::new()),
            Box::new(commands::breakpoint::BreakpointCommand::new()),
            Box::new(commands::disassemble::DisassembleCommand::new()),
            Box::new(commands::expression::ExpressionCommand::new()),
            Box::new(commands::global::GlobalCommand::new()),
            Box::new(commands::local::LocalCommand::new()),
            Box::new(commands::frame::FrameCommand::new()),
            Box::new(commands::settings::SettingsCommand::new()),
            Box::new(commands::process::ProcessCommand::new()),
        ],
        vec![
            Box::new(commands::run::RunCommand::new()),
            Box::new(commands::backtrace::BacktraceCommand::new()),
        ],
    )?;
    Ok((process, context))
}

pub fn run_loop(module_input: Option<ModuleInput>, init_source: Option<String>) -> Result<()> {
    let (mut process, context) = start_debugger(module_input)?;

    {
        let is_default = init_source.is_none();
        let lines = match {
            let init_source = init_source.unwrap_or("~/.wasminspect_init".to_string());
            use std::fs::File;
            use std::io::{BufRead, BufReader};
            File::open(init_source).map(|file| BufReader::new(file).lines())
        } {
            Ok(lines) => lines.map(|l| l.unwrap()).collect::<Vec<String>>(),
            Err(err) => {
                if is_default {
                    vec![]
                } else {
                    return Err(anyhow!("{}", err));
                }
            }
        };
        for line in lines {
            process.dispatch_command(&line, &context)?;
        }
    }
    let mut interactive = Interactive::new_with_loading_history()?;
    let process = Rc::new(RefCell::new(process));
    loop {
        match interactive.run_loop(&context, process.clone())? {
            CommandResult::ProcessFinish(_) => {}
            CommandResult::Exit => break,
        }
    }
    Ok(())
}
