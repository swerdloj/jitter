use crate::frontend::parse::ast;
use crate::frontend::validate::context::Context as ValidationContext;
use crate::frontend::validate::types::Type as CompilerType;

use cranelift::prelude::*;
use cranelift_module::Module; // for trait functions

//////////// CLIF Translation ////////////

// This file simply generates IR -- nothing more

// It might be possible to reuse this module for different targets
// such as generating standalone executables

// Translates a function and its contents into IR
pub struct FunctionTranslator<'input> {
    pub pointer_type: &'input Type,
    pub fn_builder: FunctionBuilder<'input>,
    pub module: &'input mut cranelift_simplejit::SimpleJITModule,
    // Maps `Variable`s to names and `StackSlot`s to addresses
    pub data: super::DataMap,
    pub validation_context: &'input ValidationContext<'input>,
}

impl<'input> FunctionTranslator<'input> {
    pub fn new(pointer_type: &'input Type, fn_builder: FunctionBuilder<'input>, module: &'input mut cranelift_simplejit::SimpleJITModule, validation_context: &'input ValidationContext<'input>) -> Self {
        Self {
            pointer_type,
            fn_builder,
            module,
            data: super::DataMap::new(),
            validation_context,
        }
    }

    pub fn translate_function(&mut self, function: &ast::Function, return_type: Type) -> Result<(), String> {                        
        // TEMP: debug
        // crate::log!("Generating function `{}`:\n", function.prototype.name);
        
        // Create the function's entry block with appropriate function parameters
        let entry_block = self.fn_builder.create_block();
        self.fn_builder.append_block_params_for_function_params(entry_block);
        
        // Emit code within the entry block
        self.fn_builder.switch_to_block(entry_block);
        // No predecessors for entry blocks
        self.fn_builder.seal_block(entry_block);

        // Declare the function's parameters (entry block params)
        for (index, param_node) in function.prototype.parameters.iter().enumerate() {            
            let param_type = param_node.field_type.ir_type(&self.pointer_type);
            
            let var = self.data.create_var(param_node.field_name.to_owned());
            
            // Decalre the parameter and its type
            self.fn_builder.declare_var(var, param_type);
            // Define the parameter with the values passed when calling the function
            self.fn_builder.def_var(var, self.fn_builder.block_params(entry_block)[index]);
        }
    
        // TODO: If function returns a user type, need to allocate
        //       a `StructReturnSlot`, then store the type data into that
        //       `StackSlot`
        //       Note that a special return value is also needed in this case
        
        for statement in &function.body.block.item {
            self.translate_statement(statement)?;
        }

        // No return type -> just return nothing at end of function
        // TODO: Ensure that explicit `return ()`s don't break anything
        //       with this extra return being inserted
        if return_type.is_invalid() {
            self.fn_builder.ins().return_(&[]);
        }
               
        self.fn_builder.finalize();
        
        // TEMP: debug (prints function before optimizations)
        // crate::log!("{}", self.fn_builder.display(self.module.isa()));

        Ok(())
    }
    
    fn translate_statement(&mut self, statement: &ast::Statement) -> Result<(), String> {
        // NOTE: All types will be known and validated at this point
        match statement {
            // Create a new variable and assign it if an expression is given
            ast::Statement::Let { ident, ty, value, .. } => {
                let var = self.data.create_var(*ident);

                // Unit type is declared as invalid. 
                // This mean variables assigned type `()` can still be referenced
                // (perhaps for traits?), but cannot be assigned illegally
                self.fn_builder.declare_var(var,ty.ir_type(&self.pointer_type));
                
                if let Some(expr) = value {
                    let assigned_value = self.translate_expression(expr)?;
                    // Unit type has no actual representation (zero-sized)
                    if !ty.is_unit() {
                        self.fn_builder.def_var(var, assigned_value);
                    }
                }
            }
            
            // Assign a value to a variable
            ast::Statement::Assign { variable, operator, expression } => {
                let expr_value = self.translate_expression(expression)?;
                let var = self.data.get_var(variable)?;
                
                match operator.item {
                    ast::AssignmentOp::Assign => {
                        self.fn_builder.def_var(*var, expr_value);
                    }

                    // Transformed during validation
                    _ => unreachable!(),
                }
            }

            ast::Statement::ImplicitReturn { expression, is_function_return } => {
                let expr_value = self.translate_expression(expression)?;
                
                if *is_function_return {
                    self.fn_builder.ins().return_(&[expr_value]);
                } else {
                    todo!()
                }
            }
            
            // FIXME: This can be represented as an ImplicitReturn with `is_function_return` flag
            ast::Statement::Return { expression } => {
                let return_value = self.translate_expression(expression)?;
                self.fn_builder.ins().return_(&[return_value]);
            }
            
            ast::Statement::Expression(expr) => {
                self.translate_expression(&expr)?;
            }
        }

        Ok(())
    }
    
    fn translate_expression(&mut self, expression: &ast::Expression) -> Result<Value, String> {
        let value = match expression {
            ast::Expression::BinaryExpression { lhs, op, rhs, ty } => {
                let l_value = self.translate_expression(lhs)?;
                let r_value = self.translate_expression(rhs)?;

                match op.item {
                    ast::BinaryOp::Add => {
                        if ty.is_integer() {
                            self.fn_builder.ins().iadd(l_value, r_value)
                        } else if ty.is_float() {
                            self.fn_builder.ins().fadd(l_value, r_value)
                        } else {
                            unreachable!();
                        }
                    }

                    ast::BinaryOp::Subtract => {
                        if ty.is_integer() {
                            self.fn_builder.ins().isub(l_value, r_value)
                        } else if ty.is_float() {
                            self.fn_builder.ins().fsub(l_value, r_value)
                        } else {
                            unreachable!();
                        }
                    }

                    ast::BinaryOp::Multiply => {
                        if ty.is_integer() {
                            self.fn_builder.ins().imul(l_value, r_value)
                        } else if ty.is_float() {
                            self.fn_builder.ins().fmul(l_value, r_value)
                        } else {
                            unreachable!();
                        }
                    }

                    ast::BinaryOp::Divide => {
                        todo!()
                    }
                }
            }

            ast::Expression::UnaryExpression { op, expr, ty } => {
                let value = self.translate_expression(expr)?;

                match op.item {
                    ast::UnaryOp::Negate => {
                        if ty.is_signed_integer() {
                            self.fn_builder.ins().ineg(value)
                        } else if ty.is_float() {
                            self.fn_builder.ins().fneg(value)
                        } else {
                            unreachable!();
                        }
                    }

                    ast::UnaryOp::Not => {
                        self.fn_builder.ins().bnot(value)
                    }
                }
            }

            // Any type created by constructor is allocated on the stack
            ast::Expression::FieldConstructor { ty, fields } => {
                let type_size = self.validation_context.types.size_of(ty) as u32;
                
                // Allocate the type on the stack and get its address
                let stack_slot = self.fn_builder.create_stack_slot(StackSlotData {
                    kind: StackSlotKind::ExplicitSlot,
                    size: type_size,
                    offset: None,
                });
                
                // FIXME: This appears to be the wrong approach to getting stack addresses
                //        Is this address what should be returned for custom types?
                let stack_address = self.fn_builder.ins().stack_addr(
                    *self.pointer_type, 
                    stack_slot, 
                    0
                );

                // TODO: Do not allocate extra slot for nested types
                for (field, expr) in fields {
                    let field_value = self.translate_expression(expr)?;
                    let field_offset = self.validation_context.get_field_offset(ty, field)? as i32;
                    self.fn_builder.ins().stack_store(field_value, stack_slot, field_offset);
                }

                self.data.register_stack_slot(stack_address, stack_slot);

                stack_address
            }

            ast::Expression::FieldAccess { base_expr, field, ty  } => {
                let stack_address = self.translate_expression(base_expr)?;
                let stack_slot = self.data.get_stack_slot(&stack_address)?;
                let base_type = base_expr.get_type();
                let offset = self.validation_context.get_field_offset(base_type, field)?;

                // FIXME: Narrowing cast
                self.fn_builder.ins().stack_load(
                    ty.ir_type(&self.pointer_type), 
                    *stack_slot, 
                    offset as i32,
                )
            }

            // FIXME: Same functions are being declared multiple times.
            //        Should only ever declare a function once
            ast::Expression::FunctionCall { name, inputs, ty } => {
                let func_id = if let cranelift_module::FuncOrDataId::Func(id) = self.module.declarations().get_name(name).expect("get_function_id_for_call") {
                    id
                } else {
                    // NOTE: The given AST is assumed to be valid
                    unreachable!();
                    // return Err(format!("Not a function: {}", name));
                };
            
                // FIXME: This is only needed once per referenced function 
                //        (declares same function multiple times in some cases)
                let func_ref = self.module.declare_func_in_func(func_id, &mut self.fn_builder.func);

                // Obtain argument values
                let mut values = Vec::new();
                for arg in inputs {
                    values.push(self.translate_expression(arg)?);
                }

                let call = self.fn_builder.ins().call(func_ref, &values);

                let result = self.fn_builder.inst_results(call);

                // FIXME: Handle the multiple return API better
                if result.len() == 1 {
                    result[0]
                } else if result.len() == 0 {
                    // FIXME: Ideally, I would return an INVALID value
                    Value::from_u32(0)
                } else {
                    // Multiple returns not suppported by jitter
                    unreachable!()
                }
            }

            ast::Expression::Block(block) => {
                todo!()
            }

            ast::Expression::Literal { value, ty } => {
                match value {
                    ast::Literal::Integer(integer) => {
                        // FIXME: Narrowing cast
                        self.fn_builder.ins().iconst(ty.ir_type(self.pointer_type), *integer as i64)
                    }
                    ast::Literal::Float(float) => {
                        match ty {
                            // FIXME: Narrowing cast
                            CompilerType::f32 => self.fn_builder.ins().f32const(*float as f32),
                            
                            CompilerType::f64 => self.fn_builder.ins().f64const(*float),
                            
                            _ => unreachable!(),
                        }
                    }
                    ast::Literal::UnitType => {
                        // Return arbitrary value (unused)
                        Value::from_u32(0)
                    }
                }                
            }
            
            // Get IR reference to the variable
            ast::Expression::Ident { name, ty } => {
                let var = self.data.get_var(name)?;
                self.fn_builder.use_var(*var)
            }
        };

        Ok(value)
    }   
}