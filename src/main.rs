/*
MIT License

Copyright (c) 2023 Vincent Hiribarren

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

use std::{
    io::{stdin, BufRead},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use inkwell::{context::Context, values::AnyValue};
use llvm_tuto_kaleidoscope_rust::{codegen::CodeGen, parser::GlobalParser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    without_optim: bool,
    #[arg(short, long)]
    interactive: bool,
    #[arg(short, long, value_name = "FILE")]
    script: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let context = &Context::create();
    let codegen = &mut CodeGen::new(context, !args.without_optim);
    let global_parser = &mut GlobalParser::default();

    if let Some(script_path) = &args.script {
        let file_data = std::fs::read_to_string(script_path)?;
        parse_and_execute(global_parser, codegen, &file_data);
    }
    if args.script.is_none() || args.interactive {
        launch_repl(global_parser, codegen)?;
    }
    Ok(())
}

fn parse_and_execute(global_parser: &mut GlobalParser, codegen: &mut CodeGen, input: &str) {
    match global_parser.parse(input) {
        Ok(ast) => {
            for ast_part in &ast.0 {
                match codegen.visit_top(ast_part) {
                    Ok(ir_value) => println!("{}", ir_value.print_to_string().to_string()),
                    Err(err) => eprintln!("{err}"),
                };
            }
        }
        Err(err) => eprintln!("{err}"),
    };
}

fn launch_repl(global_parser: &mut GlobalParser, codegen: &mut CodeGen) -> Result<()> {
    eprint!("ready> ");
    for line in stdin().lock().lines() {
        let line = line?;
        parse_and_execute(global_parser, codegen, &line);
        eprint!("\nready> ");
    }
    eprintln!("EOF, stopping parsing");
    codegen.print_to_stderr();
    Ok(())
}

#[no_mangle]
pub extern "C" fn hello() -> f64 {
    println!("Bonjour le monde !");
    42.0
}

#[no_mangle]
pub extern "C" fn square(x: f64) -> f64 {
    x * x
}

#[no_mangle]
pub extern "C" fn putchard(x: f64) -> f64 {
    eprint!("{}", x as u8 as char);
    0_f64
}

#[no_mangle]
pub extern "C" fn printd(x: f64) -> f64 {
    eprintln!("{x}");
    0_f64
}

#[used]
static KEEP_FUNCTIONS_PARAM_0: [extern "C" fn() -> f64; 1] = [hello];

#[used]
static KEEP_FUNCTIONS_PARAM_1: [extern "C" fn(f64) -> f64; 3] = [square, putchard, printd];
