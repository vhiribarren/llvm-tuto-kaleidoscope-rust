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
                    let ir = visitor.visit_top(ast_part)?.print_to_string().to_string();
                    println!("{ir}");
                }
            }
            Err(err) => eprintln!("{err}"),
        };
        eprint!("\nready> ");
    }
    eprintln!("EOF, stopping parsing");
    visitor.module.print_to_stderr();
    Ok(())
}
