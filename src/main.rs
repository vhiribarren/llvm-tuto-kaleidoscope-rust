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

use std::io::{stdin, BufRead};

use anyhow::Result;
use clap::Parser;
use inkwell::{context::Context, values::AnyValue};
use llvm_tuto_kaleidoscope_rust::{ast::Visitor, codegen::CodeGenVisitor, parser::GlobalParser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    without_optim: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    launch_repl(&args)
}

fn launch_repl(args: &Args) -> Result<()> {
    let context = &Context::create();
    let global_parser = &mut GlobalParser::default();
    let visitor = &mut CodeGenVisitor::new(context, !args.without_optim);
    eprint!("ready> ");
    for line in stdin().lock().lines() {
        let line = line?;
        match global_parser.parse(&line) {
            Ok(ast) => {
                for ast_part in &ast.0 {
                    match visitor.visit_top(ast_part) {
                        Ok(ir_value) => println!("{}", ir_value.print_to_string().to_string()),
                        Err(err) => eprintln!("{err}"),
                    };
                }
            }
            Err(err) => eprintln!("{err}"),
        };
        eprint!("\nready> ");
    }
    eprintln!("EOF, stopping parsing");
    visitor.print_to_stderr();
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

#[used]
static KEEP_FUNCTIONS_PARAM_0: [extern "C" fn() -> f64; 1] = [hello];

#[used]
static KEEP_FUNCTIONS_PARAM_1: [extern "C" fn(f64) -> f64; 1] = [square];
