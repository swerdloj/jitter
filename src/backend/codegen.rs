use crate::frontend::parse::ast;
use crate::frontend::validate::context::Context as ValidationContext;
use crate::frontend::validate::types::Type as CompilerType;

use cranelift::prelude::*;
use cranelift_module::Module; // for trait functions
use cranelift::codegen::ir::StackSlot;

//////////// CLIF Translation ////////////

// This file simply generates IR -- nothing more

// It might be possible to reuse this module for different targets
// such as generating standalone executables

/// Translates a function and its contents into Cranelift IR
pub struct FunctionTranslator<'input> {
    pointer_type: &'input Type,
    fn_builder: FunctionBuilder<'input>,
    module: &'input mut cranelift_simplejit::SimpleJITModule,
    // Maps variable names to memory locations
    data: super::MemoryMap,
    validation_context: &'input ValidationContext<'input>,
    // Map of already declared functions to their references
    declared_functions: std::collections::HashMap<cranelift_module::FuncId, cranelift::codegen::ir::entities::FuncRef>,
}

impl<'input> FunctionTranslator<'input> {
    pub fn new(pointer_type: &'input Type, fn_builder: FunctionBuilder<'input>, module: &'input mut cranelift_simplejit::SimpleJITModule, validation_context: &'input ValidationContext<'input>) -> Self {
        Self {
            pointer_type,
            fn_builder,
            module,
            data: super::MemoryMap::new(),
            validation_context,
            declared_functions: std::collections::HashMap::new(),
        }
    }

    pub fn translate_function(&mut self, function: &ast::Function, has_return_value: bool) -> Result<(), String> {                        
        // TEMP: debug
        // crate::log!("--Generating function `{}`--", function.prototype.name);
        
        // Create the function's entry block with appropriate function parameters
        let entry_block = self.fn_builder.create_block();
        self.fn_builder.append_block_params_for_function_params(entry_block);
        
        // Emit code within the entry block
        self.fn_builder.switch_to_block(entry_block);
        // No predecessors for entry blocks
        self.fn_builder.seal_block(entry_block);

        // Declare the function's parameters (entry block params)
        for (index, param) in function.prototype.parameters.iter().enumerate() {                        
            let param_address = self.fn_builder.block_params(entry_block)[index];
            
            let var = self.data.create_variable(param.name);
            // Address is passed in to the function rather than actual value
            self.fn_builder.declare_var(var, *self.pointer_type);
            self.fn_builder.def_var(var, param_address);
        }

        // Create a stack pre-allocation for the returned data
        if has_return_value {
            // FIXME: Narrowing cast
            let type_size = self.validation_context.types.size_of(&function.prototype.return_type) as u32;

            let return_slot = self.fn_builder.create_stack_slot(StackSlotData {
                kind: StackSlotKind::StructReturnSlot,
                size: type_size,
                offset: None,
            });

            self.data.register_struct_return_slot(return_slot);
        }
        
        for statement in &function.body.block.item {
            self.translate_statement(statement);
        }

        // FIXME: This doesn't allow users to end functions with `()` or `return;`
        if !has_return_value {
            self.fn_builder.ins().return_(&[]);
        }
               
        self.fn_builder.finalize();
        
        // TEMP: debug (prints function before optimizations)
        // crate::log!("{}", self.fn_builder.display(self.module.isa()));

        Ok(())
    }

    fn translate_statement(&mut self, statement: &ast::Statement) {
        match statement {
            ast::Statement::Let { ident, mutable: _, ty: _, value } => {
                let var = self.data.create_variable(ident);
                self.fn_builder.declare_var(var, *self.pointer_type);
                
                if let Some(assignment) = value {
                    let value_address = self.translate_expression(assignment);
                    self.fn_builder.def_var(var, value_address);
                }
            }
            
            ast::Statement::Assign { lhs, operator: _, expression } => {
                let destination_address = self.translate_expression(lhs);
                let target_address = self.translate_expression(expression);
                
                // Copy the data from target to destination
                let size = self.validation_context.types.size_of(expression.get_type());
                let size_value = self.fn_builder.ins().iconst(*self.pointer_type, size as i64);

                self.fn_builder.call_memcpy(self.module.target_config(), destination_address, target_address, size_value);
            }

            ast::Statement::ImplicitReturn { expression, is_function_return } => {
                // FIXME: Desugar this during validation rather than cloning
                if *is_function_return {
                    self.translate_statement(&ast::Statement::Return {
                        expression: expression.clone(),
                    });
                } else {
                    todo!()
                }
            }

            // TODO: Unit types
            ast::Statement::Return { expression } => {
                let return_data_address = self.translate_expression(expression);
                
                let return_slot = *self.data.get_struct_return_slot();
                let return_slot_address = self.fn_builder.ins().stack_addr(*self.pointer_type, return_slot, 0);

                // Copy the data stored in the address into the proper return slot.
                let size = self.validation_context.types.size_of(expression.get_type()) as i64;
                let size_value = self.fn_builder.ins().iconst(*self.pointer_type, size);
                
                self.fn_builder.call_memcpy(self.module.target_config(), return_slot_address, return_data_address, size_value);

                self.fn_builder.ins().return_(&[return_slot_address]);
            }

            ast::Statement::Expression(expression) => {
                self.translate_expression(expression);
            }
        }
    }

    fn translate_expression(&mut self, expression: &ast::Expression) -> Value {
        match expression {
            ast::Expression::Ident { name, ty: _ } => {
                let var = self.data.get_variable(name);
                self.fn_builder.use_var(var)
            }

            ast::Expression::FieldAccess { base_expr, field, ty: _ } => {
                let base_address = self.translate_expression(base_expr);
                let field_offset = self.validation_context.get_field_offset(base_expr.get_type(), field).unwrap();
                // return the address of the desired field
                self.fn_builder.ins().iadd_imm(base_address, field_offset as i64)
            }

            ast::Expression::FieldConstructor { ty, fields } 
                => self.translate_field_constructor(ty, fields),

            ast::Expression::Literal { value, ty } 
                => self.translate_expression_literal(value, ty),

            ast::Expression::FunctionCall { name, inputs, ty } 
                => self.translate_expression_function_call(name, inputs, ty),

            ast::Expression::BinaryExpression { lhs, op, rhs, ty } => {
                todo!()
            }

            ast::Expression::UnaryExpression { op, expr, ty } => {
                todo!()
            }

            ast::Expression::Block(_) => {
                todo!()
            }
        }
    }

    fn translate_expression_function_call(&mut self, name: &str, inputs: &Vec<ast::Node<ast::Expression>>, ty: &CompilerType) -> Value {
        let func_id = if let cranelift_module::FuncOrDataId::Func(id) = self.module.declarations().get_name(name).unwrap() {
            id
        } else {
            unreachable!()
        };
        
        // If a function has already been declared, don't declare it again
        // If it is new, save the reference for future use
        let func_ref = if !self.declared_functions.contains_key(&func_id) {
            let func_ref = self.module.declare_func_in_func(func_id, &mut self.fn_builder.func);
            self.declared_functions.insert(func_id, func_ref);
            func_ref
        } else {
            *self.declared_functions.get(&func_id).unwrap()
        };

        let mut passed_params = Vec::new();
        for input in inputs {
            passed_params.push(
                self.translate_expression(input)
            );
        }

        let call = self.fn_builder.ins().call(func_ref, &passed_params);

        // Cranelift allows multiple returns, but Jitter only allows one
        let maybe_multiple_return = self.fn_builder.inst_results(call);

        // Either one value is returned or none
        if let Some(return_address) = maybe_multiple_return.get(0).map(|v| *v) {
            // FIXME: Copy shouldn't be needed, but I don't know what else is wrong
            //        that causes function calls as arguments to other functions
            //        resulting in garbage being passed instead.
            //
            //        Should simply be able to pass along the returned address (`return_address` in this case)
            let size = self.validation_context.types.size_of(ty) as u32;
            let size_value = self.fn_builder.ins().iconst(*self.pointer_type, size as i64);
            let slot = self.create_explicit_stack_allocation(size);
            let slot_address = self.fn_builder.ins().stack_addr(*self.pointer_type, slot, 0);
            self.fn_builder.call_memcpy(self.module.target_config(), slot_address, return_address, size_value);
            slot_address
        } else {
            // If nothing is returned, just return an arbitrary value.
            // Assignments to unit types will ignore this anyway.
            Value::new(0)
        }
    }

    fn translate_field_constructor(&mut self, ty: &CompilerType, fields: &std::collections::HashMap<&str, ast::Node<ast::Expression>>) -> Value {
        // 1. Allocate memory for the object
        // FIXME: Narrowing cast
        let size = self.validation_context.types.size_of(ty) as u32;
        let slot = self.create_explicit_stack_allocation(size);
        let slot_address = self.fn_builder.ins().stack_addr(*self.pointer_type, slot, 0);

        // TODO: Is there any way to write to a given address
        //       rather than needing to copy like this?

        for (field, expression) in fields {
            // 2.1 Obtain the address containing the corresponding field's data
            let field_value_address = self.translate_expression(expression);
            let field_offset = self.validation_context.get_field_offset(ty, field).unwrap() as i64;
            let destination_address = self.fn_builder.ins().iadd_imm(slot_address, field_offset);
            
            let field_size = self.validation_context.types.size_of(expression.get_type()) as i64;
            let field_size_value = self.fn_builder.ins().iconst(*self.pointer_type, field_size);

            // 2.2 Copy that data's bytes into the assigned slot
            self.fn_builder.call_memcpy(self.module.target_config(), destination_address, field_value_address, field_size_value);
        }

        // 3. Return the address of the newly instantiated object
        self.fn_builder.ins().stack_addr(*self.pointer_type, slot, 0)
    }

    // Allocate the data on the stack, fill it, and return the address
    fn translate_expression_literal(&mut self, literal: &ast::Literal, ty: &CompilerType) -> Value {
        // FIXME: Narrowing casts
        let value = match literal {
            ast::Literal::Integer(integer) => {
                self.fn_builder.ins().iconst(ty.ir_type(self.pointer_type), *integer as i64)
            }
            
            ast::Literal::Float(float) => {
                // If not f32, then f64
                if let CompilerType::f32 = ty {
                    self.fn_builder.ins().f32const(*float as f32)
                } else {
                    self.fn_builder.ins().f64const(*float)
                }
            }
            
            ast::Literal::UnitType => {
                todo!()
            }
        };

        let size = self.validation_context.types.size_of(ty) as u32;
        let allocation = self.create_explicit_stack_allocation(size);

        self.fn_builder.ins().stack_store(value, allocation, 0);
        self.fn_builder.ins().stack_addr(*self.pointer_type, allocation, 0)      
    }

    fn create_explicit_stack_allocation(&mut self, size: u32) -> StackSlot {
        self.fn_builder.create_stack_slot(StackSlotData {
            kind: StackSlotKind::ExplicitSlot,
            size,
            offset: None,
        })
    }
}