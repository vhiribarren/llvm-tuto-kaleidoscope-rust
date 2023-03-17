use std::io::{stdin, BufRead};

use anyhow::Result;
use llvm_tuto_kaleidoscope_rust::parser::generate_ast;

fn main() -> Result<()> {
    launch_repl()
}

fn launch_repl() -> Result<()> {
    eprint!("ready> ");
    for line in stdin().lock().lines() {
        let line = line?;
        dbg!(generate_ast(&line));
        eprint!("\nready> ");
    }
    eprintln!("EOF, stopping parsing");
    Ok(())
}
