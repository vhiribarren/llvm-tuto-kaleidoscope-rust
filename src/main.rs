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
use inkwell::{context::Context, values::AnyValue};
use llvm_tuto_kaleidoscope_rust::{ast::Visitor, codegen::CodeGenVisitor, parser::generate_ast};

fn main() -> Result<()> {
    launch_repl()
}

fn launch_repl() -> Result<()> {
    let context = &Context::create();
    let visitor = &mut CodeGenVisitor::new(context);
    eprint!("ready> ");
    for line in stdin().lock().lines() {
        let line = line?;
        match generate_ast(&line) {
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
