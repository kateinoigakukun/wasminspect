use std::io::{self, Write};

pub fn run_loop() {
    let mut raw_cmd = String::new();
    print!("wasminspect> ");
    io::stdout().flush().unwrap();
    while let Ok(len) = io::stdin().read_line(&mut raw_cmd) {
        print!("wasminspect v0.1.0\n");
        print!("wasminspect> ");
        io::stdout().flush().unwrap();
    }
}