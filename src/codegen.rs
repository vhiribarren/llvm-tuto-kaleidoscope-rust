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
    values::{AnyValue, AnyValueEnum, FunctionValue},
};

use crate::ast::{
    BinaryExprAST, CallExprAST, ExprAST, FunctionAST, IfExprAST, NumberExprAST, PrototypeAST,
    TopAST, VariableExprAST, Visitor, ANONYM_FUNCTION,
};

pub struct CodeGenVisitor<'ctx> {
    context: &'ctx Context,
    named_values_ctx: HashMap<String, AnyValueEnum<'ctx>>,
    prototypes: HashMap<String, PrototypeAST>,
    builder: Builder<'ctx>,
    modules: Vec<Module<'ctx>>,
    last_pass_manager: PassManager<FunctionValue<'ctx>>,
}

impl<'ctx> CodeGenVisitor<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        let (module, pass_manager) = Self::init_new_module(context);
        let modules = vec![module];
        let prototypes = HashMap::new();
        CodeGenVisitor {
            context,
            named_values_ctx: HashMap::new(),
            prototypes,
            builder: context.create_builder(),
            last_pass_manager: pass_manager,
            modules,
        }
    }

    fn init_new_module(context: &Context) -> (Module, PassManager<FunctionValue>) {
        let module = context.create_module("my cool JIT");
        let pass_manager = PassManager::create(&module);
        pass_manager.add_instruction_combining_pass();
        pass_manager.add_reassociate_pass();
        pass_manager.add_gvn_pass();
        pass_manager.add_cfg_simplification_pass();
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
}

impl<'ctx> Visitor for CodeGenVisitor<'ctx> {
    type Result = Result<AnyValueEnum<'ctx>>;

    fn visit_binary_expr(&mut self, bin_elem: &BinaryExprAST) -> Self::Result {
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
            other => bail!("Unknown operator: {other}"),
        };
        Ok(AnyValueEnum::FloatValue(result))
    }

    fn visit_expr(&mut self, expr_elem: &ExprAST) -> Self::Result {
        match expr_elem {
            ExprAST::NumberExpr(num_elem) => self.visit_number_expr(num_elem),
            ExprAST::VariableExpr(var_elem) => self.visit_variable_expr(var_elem),
            ExprAST::BinaryExpr(bin_elem) => self.visit_binary_expr(bin_elem),
            ExprAST::CallExpr(call_elem) => self.visit_call_expr(call_elem),
            ExprAST::IfExpr(if_elem) => self.visit_if_expr(if_elem),
        }
    }

    fn visit_if_expr(&mut self, if_elem: &IfExprAST) -> Self::Result {
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

    fn visit_number_expr(&mut self, num_elem: &NumberExprAST) -> Self::Result {
        let f64_type = self.context.f64_type();
        Ok(AnyValueEnum::FloatValue(f64_type.const_float(num_elem.val)))
    }

    fn visit_variable_expr(&mut self, var_elem: &VariableExprAST) -> Self::Result {
        Ok(*self
            .named_values_ctx
            .get(&var_elem.name)
            .ok_or(anyhow!("Variable not found"))?)
    }

    fn visit_call_expr(&mut self, call_elem: &CallExprAST) -> Self::Result {
        let func_name = &call_elem.callee;
        let func = if let Some(func_val) = self.modules.last().unwrap().get_function(func_name) {
            func_val
        } else {
            let proto_ast = self
                .prototypes
                .get(func_name)
                .ok_or(anyhow!("{func_name} not found in prototype lists"))?
                .clone();
            match self.visit_prototype(&proto_ast)? {
                AnyValueEnum::FunctionValue(func_val) => func_val,
                _ => bail!("Shoul have been a function value"),
            }
        };

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

    fn visit_prototype(&mut self, proto_elem: &PrototypeAST) -> Self::Result {
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

    fn visit_function(&mut self, func_elem: &FunctionAST) -> Self::Result {
        let proto_elem = &func_elem.proto;
        let func_name = &proto_elem.name;
        if !func_elem.is_top_function() {
            let insert_result = self
                .prototypes
                .insert(proto_elem.name.to_string(), proto_elem.clone());
            if insert_result.is_some() {
                bail!("Prototype {func_name} already exists");
            }
        }
        let func = match self
            .modules
            .last()
            .unwrap()
            .get_function(&func_elem.proto.name)
        {
            Some(func) => func,
            None => self
                .visit_prototype(&func_elem.proto)?
                .into_function_value(),
        };
        ensure!(!func.is_null(), "Function cannot be redefined");
        let basic_block = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(basic_block);
        self.named_values_ctx.clear();
        for (idx, arg) in func.get_param_iter().enumerate() {
            self.named_values_ctx
                .insert(func_elem.proto.args[idx].clone(), arg.as_any_value_enum());
        }
        match self.visit_expr(&func_elem.body) {
            Ok(ret_val) => {
                self.builder.build_return(Some(&ret_val.into_float_value()));
                if !func.verify(false) {
                    bail!("Verify function detected an issue");
                }
                self.last_pass_manager.run_on(&func);
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

    fn visit_top(&mut self, top_elem: &TopAST) -> Self::Result {
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
}
