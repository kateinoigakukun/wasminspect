use super::commands::debugger;

pub struct MainDebugger {}

impl MainDebugger {
    pub fn new() -> Self {
        Self {}
    }
}

impl debugger::Debugger for MainDebugger {}
