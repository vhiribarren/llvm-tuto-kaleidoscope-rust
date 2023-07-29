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
use inkwell::{
    context::Context,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
    values::AnyValue,
    OptimizationLevel,
};
use llvm_tuto_kaleidoscope_rust::{codegen::CodeGen, parser::GlobalParser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Parameters {
    /// Disable LLVM code optimisation
    #[arg(long)]
    without_optim: bool,

    /// If a kaleido script is executed, keep a REPL after
    #[arg(short, long)]
    interactive: bool,

    /// Execute a file containing a kaleido script
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Produce an object file
    #[arg(short, long)]
    output_object: Option<PathBuf>,

    /// Mute LLVM code display
    #[arg(short, long)]
    silent: bool,
}

fn main() -> Result<()> {
    let params = &Parameters::parse();
    let context = &Context::create();
    let codegen = CodeGen::new(context, !params.without_optim);
    let global_parser = GlobalParser::default();

    let mut kaleido = Kaleido {
        params,
        codegen,
        global_parser,
    };

    if let Some(script_path) = &params.file {
        let file_data = std::fs::read_to_string(script_path)?;
        kaleido.parse_and_execute(&file_data);
    }
    if params.file.is_none() || params.interactive {
        kaleido.launch_repl()?;
    }
    if params.output_object.is_some() {
        kaleido.produce_object_code();
    }
    Ok(())
}

struct Kaleido<'a> {
    params: &'a Parameters,
    codegen: CodeGen<'a>,
    global_parser: GlobalParser,
}

impl<'ctx> Kaleido<'ctx> {
    fn parse_and_execute(&mut self, input: &str) {
        let ast = match self.global_parser.parse(input) {
            Ok(ast) => ast,
            Err(err) => return eprintln!("{err}"),
        };
        for ast_part in &ast.0 {
            match self.codegen.visit_top(ast_part) {
                Ok(ir_value) => {
                    if !self.params.silent {
                        println!("{}", ir_value.print_to_string().to_string())
                    }
                }
                Err(err) => eprintln!("{err}"),
            };
        }
    }

    fn launch_repl(&mut self) -> Result<()> {
        eprintln!("Ctrl+D Ctrl+D to leave");
        eprint!("ready> ");
        for line in stdin().lock().lines() {
            let line = line?;
            self.parse_and_execute(&line);
            eprint!("\nready> ");
        }
        eprintln!("EOF, stopping parsing");
        self.codegen.print_to_stderr();
        Ok(())
    }

    fn produce_object_code(&self) {
        let Some(ref output) = self.params.output_object else {
            panic!("Cannot produce code if no output file is provided");
        };
        Target::initialize_all(&InitializationConfig {
            asm_parser: true,
            asm_printer: true,
            base: true,
            disassembler: true,
            info: true,
            machine_code: true,
        });
        let cpu = "generic";
        let features = "";
        let level = OptimizationLevel::Default;
        let reloc_mode = RelocMode::Default;
        let code_model = CodeModel::Default;
        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple).unwrap();
        let target_machine = target
            .create_target_machine(&target_triple, cpu, features, level, reloc_mode, code_model)
            .unwrap();
        self.codegen.generate_object_code(&target_machine, output);
    }
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
