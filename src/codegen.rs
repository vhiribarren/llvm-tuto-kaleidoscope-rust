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

use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure, Result};
use inkwell::{
    builder::Builder,
    context::Context,
    execution_engine::JitFunction,
    module::Module,
    passes::PassManager,
    types::BasicMetadataTypeEnum,
    values::{AnyValue, AnyValueEnum, FunctionValue, PointerValue},
    FloatPredicate,
};

use crate::ast::{
    BinaryExprAST, CallExprAST, ExprAST, ForExprAST, FunctionAST, IfExprAST, NumberExprAST,
    PrototypeAST, TopAST, UnaryExprAST, VariableExprAST, ANONYM_FUNCTION,
};

pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    named_values_ctx: HashMap<String, PointerValue<'ctx>>,
    prototypes: HashMap<String, PrototypeAST>,
    builder: Builder<'ctx>,
    modules: Vec<Module<'ctx>>,
    last_pass_manager: PassManager<FunctionValue<'ctx>>,
    with_optim: bool,
}

/// Due to issue or having partial borrow before borrowing the whole structure,
/// a macro is used. The output of generate_and_get_func is not used, otherwise
/// it become difficult to borrow self later.
macro_rules! generate_and_get_func {
    ($codegen:ident, $func_name:expr) => {{
        $codegen.generate_and_get_func($func_name)?;
        $codegen
            .modules
            .last()
            .unwrap()
            .get_function($func_name)
            .ok_or(anyhow!("{} not found in prototype lists", $func_name))
    }};
}

type CodeGenResult<'ctx> = Result<AnyValueEnum<'ctx>>;

impl<'ctx> CodeGen<'ctx> {
    pub fn new(context: &'ctx Context, with_optim: bool) -> Self {
        let (module, pass_manager) = Self::init_new_module(context);
        let modules = vec![module];
        let prototypes = HashMap::new();
        CodeGen {
            context,
            named_values_ctx: HashMap::new(),
            prototypes,
            builder: context.create_builder(),
            last_pass_manager: pass_manager,
            modules,
            with_optim,
        }
    }

    fn init_new_module(context: &Context) -> (Module, PassManager<FunctionValue>) {
        let module = context.create_module("my cool JIT");
        let pass_manager = PassManager::create(&module);
        // Promote allocas to registers.
        pass_manager.add_promote_memory_to_register_pass();
        // Do simple "peephole" optimizations and bit-twiddling optzns.
        pass_manager.add_instruction_combining_pass();
        // Reassociate expressions.
        pass_manager.add_reassociate_pass();
        pass_manager.add_gvn_pass();
        pass_manager.add_cfg_simplification_pass();
        // Promote allocas to registers.
        pass_manager.add_promote_memory_to_register_pass();

        pass_manager.initialize();
        (module, pass_manager)
    }

    fn change_module(&mut self) {
        let (new_module, new_pass) = Self::init_new_module(self.context);
        self.modules.push(new_module);
        self.last_pass_manager = new_pass;
    }

    pub fn print_to_stderr(&self) {
        for module in &self.modules {
            module.print_to_stderr();
        }
    }

    fn generate_and_get_func(&mut self, func_name: &str) -> Result<FunctionValue> {
        if let Some(func_val) = self.modules.last().unwrap().get_function(func_name) {
            Ok(func_val)
        } else {
            let proto_ast = self
                .prototypes
                .get(func_name)
                .ok_or(anyhow!("{func_name} not found in prototype lists"))?
                .clone();
            match self.visit_prototype(&proto_ast)? {
                AnyValueEnum::FunctionValue(func_val) => Ok(func_val),
                _ => bail!("Shoul have been a function value"),
            }
        }
    }

    fn create_entry_block_alloca<'c>(
        ctx: &'c Context,
        func: &FunctionValue,
        var_name: &str,
    ) -> Result<PointerValue<'c>> {
        let local_builder = ctx.create_builder();
        let first_block = func
            .get_first_basic_block()
            .ok_or(anyhow!("No first basic block"))?;
        match first_block.get_first_instruction() {
            Some(first_inst) => local_builder.position_before(&first_inst),
            None => local_builder.position_at_end(first_block),
        }
        Ok(local_builder.build_alloca(ctx.f64_type(), var_name))
    }

    fn visit_unary_expr(&mut self, unary_elem: &UnaryExprAST) -> CodeGenResult<'ctx> {
        let operand_value = self.visit_expr(&unary_elem.operand)?.into_float_value();
        let func_name = PrototypeAST::gen_unary_func_name(unary_elem.opcode);
        let func = generate_and_get_func!(self, &func_name)?;
        let result = self
            .builder
            .build_call(func, &[operand_value.into()], "unop")
            .try_as_basic_value()
            .left()
            .ok_or(anyhow!("Error when calling function"))?
            .into_float_value();
        Ok(AnyValueEnum::FloatValue(result))
    }

    fn visit_binary_expr(&mut self, bin_elem: &BinaryExprAST) -> CodeGenResult<'ctx> {
        let l = self.visit_expr(&bin_elem.lhs)?.into_float_value();
        let r = self.visit_expr(&bin_elem.rhs)?.into_float_value();
        let result = match bin_elem.op {
            '+' => self.builder.build_float_add(l, r, "addtmp"),
            '-' => self.builder.build_float_sub(l, r, "subtmp"),
            '*' => self.builder.build_float_mul(l, r, "multmp"),
            '<' => {
                let comp =
                    self.builder
                        .build_float_compare(inkwell::FloatPredicate::ULT, l, r, "cmttmp");
                self.builder
                    .build_unsigned_int_to_float(comp, self.context.f64_type(), "booltmp")
            }
            other => {
                let func_name = PrototypeAST::gen_binary_func_name(other);
                let func = generate_and_get_func!(self, &func_name)?;
                self.builder
                    .build_call(func, &[l.into(), r.into()], "binop")
                    .try_as_basic_value()
                    .left()
                    .ok_or(anyhow!("Error when calling function"))?
                    .into_float_value()
            }
        };
        Ok(AnyValueEnum::FloatValue(result))
    }

    fn visit_expr(&mut self, expr_elem: &ExprAST) -> CodeGenResult<'ctx> {
        match expr_elem {
            ExprAST::NumberExpr(num_elem) => self.visit_number_expr(num_elem),
            ExprAST::VariableExpr(var_elem) => self.visit_variable_expr(var_elem),
            ExprAST::UnaryExpr(unary_elem) => self.visit_unary_expr(unary_elem),
            ExprAST::BinaryExpr(bin_elem) => self.visit_binary_expr(bin_elem),
            ExprAST::CallExpr(call_elem) => self.visit_call_expr(call_elem),
            ExprAST::IfExpr(if_elem) => self.visit_if_expr(if_elem),
            ExprAST::ForExpr(for_elem) => self.visit_for_expr(for_elem),
        }
    }

    fn visit_if_expr(&mut self, if_elem: &IfExprAST) -> CodeGenResult<'ctx> {
        let cond_value = self.visit_expr(&if_elem.condition)?;
        let current_func = self
            .builder
            .get_insert_block()
            .ok_or(anyhow!("No block"))?
            .get_parent()
            .ok_or(anyhow!("No parent"))?;
        let comparison = self.builder.build_float_compare(
            inkwell::FloatPredicate::ONE,
            cond_value.into_float_value(),
            self.context.f64_type().const_float(0.0),
            "ifcond",
        );
        let then_block = self.context.append_basic_block(current_func, "then");
        let else_block = self.context.append_basic_block(current_func, "else");
        let merge_block = self.context.append_basic_block(current_func, "ifcont");
        self.builder
            .build_conditional_branch(comparison, then_block, else_block);
        // Then block
        self.builder.position_at_end(then_block);
        let then_value = self.visit_expr(&if_elem.then_block)?.into_float_value();
        self.builder.build_unconditional_branch(merge_block);
        let phi_then_block = self
            .builder
            .get_insert_block()
            .ok_or(anyhow!("Could not find block"))?;
        // Else block
        self.builder.position_at_end(else_block);
        let else_value = self.visit_expr(&if_elem.else_block)?.into_float_value();
        self.builder.build_unconditional_branch(merge_block);
        let phi_else_block = self
            .builder
            .get_insert_block()
            .ok_or(anyhow!("Could not find block"))?;
        // Merge block
        self.builder.position_at_end(merge_block);
        let phi_node = self.builder.build_phi(self.context.f64_type(), "iftmp");
        phi_node.add_incoming(&[(&then_value, phi_then_block), (&else_value, phi_else_block)]);
        Ok(AnyValueEnum::FloatValue(
            phi_node.as_basic_value().into_float_value(),
        ))
    }

    fn visit_number_expr(&mut self, num_elem: &NumberExprAST) -> CodeGenResult<'ctx> {
        let f64_type = self.context.f64_type();
        Ok(AnyValueEnum::FloatValue(f64_type.const_float(num_elem.val)))
    }

    fn visit_variable_expr(&mut self, var_elem: &VariableExprAST) -> CodeGenResult<'ctx> {
        let alloca_pointer = self
            .named_values_ctx
            .get(&var_elem.name)
            .ok_or(anyhow!("Unknown variable name"))?;
        let loaded_val =
            self.builder
                .build_load(self.context.f64_type(), *alloca_pointer, &var_elem.name);
        Ok(loaded_val.as_any_value_enum())
    }

    fn visit_call_expr(&mut self, call_elem: &CallExprAST) -> CodeGenResult<'ctx> {
        let func_name = &call_elem.callee;
        let func = generate_and_get_func!(self, func_name)?;
        ensure!(
            func.count_params() == call_elem.args.len() as u32,
            "Bad parameter number"
        );
        let mut arg_values = vec![];
        for expr_elem in &call_elem.args {
            arg_values.push(self.visit_expr(expr_elem)?.into_float_value().into());
        }
        Ok(AnyValueEnum::FloatValue(
            self.builder
                .build_call(func, arg_values.as_slice(), "calltmp")
                .try_as_basic_value()
                .left()
                .ok_or(anyhow!("Error when calling function"))?
                .into_float_value(),
        ))
    }

    fn visit_prototype(&mut self, proto_elem: &PrototypeAST) -> CodeGenResult<'ctx> {
        let f64_type: BasicMetadataTypeEnum = self.context.f64_type().into();
        let param_types = vec![f64_type; proto_elem.args.len()];
        let func_name = &proto_elem.name;
        let func_type = self
            .context
            .f64_type()
            .fn_type(param_types.as_slice(), false);
        let func = self.modules.last().unwrap().add_function(
            func_name,
            func_type,
            Some(inkwell::module::Linkage::External),
        );
        func.get_params().iter().enumerate().for_each(|(idx, arg)| {
            arg.set_name(&proto_elem.args[idx]);
        });
        Ok(AnyValueEnum::FunctionValue(func))
    }

    fn visit_function(&mut self, func_elem: &FunctionAST) -> CodeGenResult<'ctx> {
        let proto_elem = &func_elem.proto;
        let func_name = &proto_elem.name;
        let insert_result = self
            .prototypes
            .insert(proto_elem.name.to_string(), proto_elem.clone());
        if insert_result.is_some() && !func_elem.is_top_function() {
            bail!("Prototype {func_name} already exists");
        }
        let func = generate_and_get_func!(self, func_name)?;
        ensure!(!func.is_null(), "Function cannot be redefined");
        let basic_block = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(basic_block);
        self.named_values_ctx.clear();
        for (idx, arg) in func.get_param_iter().enumerate() {
            let alloca_inst = Self::create_entry_block_alloca(
                self.context,
                &func,
                arg.as_any_value_enum()
                    .into_float_value()
                    .get_name()
                    .to_str()?,
            )?;
            self.builder.build_store(alloca_inst, arg);
            self.named_values_ctx
                .insert(func_elem.proto.args[idx].clone(), alloca_inst);
        }
        match self.visit_expr(&func_elem.body) {
            Ok(ret_val) => {
                self.builder.build_return(Some(&ret_val.into_float_value()));
                if !func.verify(false) {
                    bail!("Verify function detected an issue");
                }
                if self.with_optim {
                    self.last_pass_manager.run_on(&func);
                }
                Ok(AnyValueEnum::FunctionValue(func))
            }
            error => {
                unsafe {
                    func.delete();
                }
                error
            }
        }
    }

    pub fn visit_top(&mut self, top_elem: &TopAST) -> CodeGenResult<'ctx> {
        match top_elem {
            TopAST::Function(func_elem) => {
                self.change_module();
                let func = self.visit_function(func_elem)?;
                if func_elem.is_top_function() {
                    let execution_engine = self
                        .modules
                        .last()
                        .unwrap()
                        .create_jit_execution_engine(inkwell::OptimizationLevel::None)
                        .unwrap();
                    self.modules
                        .iter()
                        .take(self.modules.len() - 1)
                        .for_each(|m| {
                            execution_engine.add_module(m).unwrap();
                        });
                    unsafe {
                        if let Ok(top_func) = execution_engine.get_function(ANONYM_FUNCTION) {
                            let result =
                                (top_func as JitFunction<unsafe extern "C" fn() -> f64>).call();
                            println!("\nEvaluated to: {result}\n");
                            for module in &self.modules {
                                execution_engine.remove_module(module).unwrap();
                            }
                        }
                    }
                }
                Ok(func)
            }
            TopAST::Prototype(proto_elem) => {
                self.prototypes
                    .insert(proto_elem.name.to_string(), proto_elem.clone());
                self.visit_prototype(proto_elem)
            }
        }
    }

    fn visit_for_expr(&mut self, for_elem: &ForExprAST) -> CodeGenResult<'ctx> {
        // Get the current block of the enclosing function
        let pre_header_block = self
            .builder
            .get_insert_block()
            .ok_or(anyhow!("Block not found"))?;
        let enclosing_func = pre_header_block
            .get_parent()
            .ok_or(anyhow!("Block is not owned by a function"))?;

        let var_name = &for_elem.var_name;
        let alloca = Self::create_entry_block_alloca(self.context, &enclosing_func, var_name)?;
        let start_val = self.visit_expr(&for_elem.var_start)?;
        self.builder
            .build_store(alloca, start_val.into_float_value());

        let loop_block = self.context.append_basic_block(enclosing_func, "loop");
        // We jump from the entry block to the loop block
        // since we cannot directly have a phi instruction
        self.builder.build_unconditional_branch(loop_block);
        // Preparing the content of the loop block
        self.builder.position_at_end(loop_block);

        // We update the variable table so that the variable can be used inside the loop block by other instruction
        // Previous variable with a same name is shadowed, but we save its state to restore it at the end.
        let old_var_val = self.named_values_ctx.insert(var_name.into(), alloca);
        // Generating the body of the loop
        self.visit_expr(&for_elem.body)?;
        // Time to increment the for variable ; if not step value, default to 1
        let step_val = match &for_elem.step {
            Some(step) => self.visit_expr(step)?,
            None => self.context.f64_type().const_float(1.0).into(),
        };

        // Evaluating condition
        let end_cond = self.visit_expr(&for_elem.var_end)?;
        let end_comp_cmp = self.builder.build_float_compare(
            FloatPredicate::ONE,
            end_cond.into_float_value(),
            self.context.f64_type().const_float(0.0),
            "loopcond",
        );

        let cur_var = self
            .builder
            .build_load(self.context.f64_type(), alloca, var_name);
        let next_var = self.builder.build_float_add(
            cur_var.into_float_value(),
            step_val.into_float_value(),
            "nextvar",
        );
        self.builder.build_store(alloca, next_var);

        // Preparing the block after the for block
        let after_block = self.context.append_basic_block(enclosing_func, "afterloop");
        self.builder
            .build_conditional_branch(end_comp_cmp, loop_block, after_block);
        self.builder.position_at_end(after_block);
        // This is the end, restoring shadowed variable if one was existing
        if let Some(val) = old_var_val {
            self.named_values_ctx.insert(var_name.into(), val);
        } else {
            self.named_values_ctx.remove(var_name);
        }
        Ok(self.context.f64_type().const_zero().into())
    }
}
