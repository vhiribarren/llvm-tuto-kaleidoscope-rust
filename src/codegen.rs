use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure, Result};
use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    types::BasicMetadataTypeEnum,
    values::{AnyValue, AnyValueEnum},
};

use crate::ast::{
    BinaryExprAST, CallExprAST, ExprAST, FunctionAST, NumberExprAST, PrototypeAST, TopAST,
    VariableExprAST, Visitor,
};

pub struct CodeGenVisitor<'ctx> {
    context: &'ctx Context,
    named_values_ctx: HashMap<String, AnyValueEnum<'ctx>>,
    builder: Builder<'ctx>,
    pub module: Module<'ctx>,
}

impl<'ctx> CodeGenVisitor<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        CodeGenVisitor {
            context,
            named_values_ctx: HashMap::new(),
            builder: context.create_builder(),
            module: context.create_module("main"),
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
    fn visit_call_expr(&mut self, call_elem: &CallExprAST) -> Self::Result {
        let func_name = &call_elem.callee;
        let func = self
            .module
            .get_function(func_name)
            .ok_or(anyhow!("Function {func_name} not found."))?;
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
    fn visit_expr(&mut self, expr_elem: &ExprAST) -> Self::Result {
        match expr_elem {
            ExprAST::NumberExpr(num_elem) => self.visit_number_expr(num_elem),
            ExprAST::VariableExpr(var_elem) => self.visit_variable_expr(var_elem),
            ExprAST::BinaryExpr(bin_elem) => self.visit_binary_expr(bin_elem),
            ExprAST::CallExpr(call_elem) => self.visit_call_expr(call_elem),
        }
    }
    fn visit_function(&mut self, func_elem: &FunctionAST) -> Self::Result {
        let func = match self.module.get_function(&func_elem.proto.name) {
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
                func.verify(false);
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
    fn visit_number_expr(&mut self, num_elem: &NumberExprAST) -> Self::Result {
        let f64_type = self.context.f64_type();
        Ok(AnyValueEnum::FloatValue(f64_type.const_float(num_elem.val)))
    }
    fn visit_prototype(&mut self, proto_elem: &PrototypeAST) -> Self::Result {
        let f64_type: BasicMetadataTypeEnum = self.context.f64_type().into();
        let param_types = vec![f64_type; proto_elem.args.len()];
        let func_type = self
            .context
            .f64_type()
            .fn_type(param_types.as_slice(), false);
        let func = self.module.add_function(
            &proto_elem.name,
            func_type,
            Some(inkwell::module::Linkage::External),
        );
        func.get_params().iter().enumerate().for_each(|(idx, arg)| {
            arg.set_name(&proto_elem.args[idx]);
        });
        func.count_params();
        Ok(AnyValueEnum::FunctionValue(func))
    }
    fn visit_top(&mut self, top_elem: &TopAST) -> Self::Result {
        match top_elem {
            TopAST::Function(func_elem) => self.visit_function(func_elem),
            TopAST::Prototype(proto_elem) => self.visit_prototype(proto_elem),
        }
    }
    fn visit_variable_expr(&mut self, var_elem: &VariableExprAST) -> Self::Result {
        Ok(*self
            .named_values_ctx
            .get(&var_elem.name)
            .ok_or(anyhow!("Variable not found"))?)
    }
}
