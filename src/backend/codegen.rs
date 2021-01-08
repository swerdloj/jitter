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
    pub pointer_type: &'input Type,
    pub fn_builder: FunctionBuilder<'input>,
    pub module: &'input mut cranelift_simplejit::SimpleJITModule,
    // Maps variable names to memory locations
    pub data: super::MemoryMap,
    pub validation_context: &'input ValidationContext<'input>,
}

impl<'input> FunctionTranslator<'input> {
    pub fn new(pointer_type: &'input Type, fn_builder: FunctionBuilder<'input>, module: &'input mut cranelift_simplejit::SimpleJITModule, validation_context: &'input ValidationContext<'input>) -> Self {
        Self {
            pointer_type,
            fn_builder,
            module,
            data: super::MemoryMap::new(),
            validation_context,
        }
    }

    pub fn translate_function(&mut self, function: &ast::Function, has_return_value: bool) -> Result<(), String> {                        
        // TEMP: debug
        crate::log!("--Generating function `{}`--", function.prototype.name);
        
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
        crate::log!("{}", self.fn_builder.display(self.module.isa()));

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
            
            ast::Statement::Assign { variable, operator, expression } => {
                todo!()
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
                let return_address = self.translate_expression(expression);
                // TODO: Copy the data stored in the address into the proper return slot.
                // TODO: Is Cranelift doing this for me? The code works as-is
                self.fn_builder.ins().return_(&[return_address]);
            }

            ast::Statement::Expression(_) => {
                todo!()
            }
        }
    }

    fn translate_expression(&mut self, expression: &ast::Expression) -> Value {
        match expression {
            ast::Expression::Ident { name, ty } => {
                let var = self.data.get_variable(name);
                self.fn_builder.use_var(var)
            }

            ast::Expression::FieldAccess { base_expr, field, ty } => {
                let base_address = self.translate_expression(base_expr);
                let field_offset = self.validation_context.get_field_offset(base_expr.get_type(), field).unwrap();

                // return the address of the desired field
                self.fn_builder.ins().iadd_imm(base_address, field_offset as i64)
            }

            ast::Expression::FieldConstructor { ty, fields } => {
                // 1. Allocate memory for the object
                // FIXME: Narrowing cast
                let size = self.validation_context.types.size_of(ty) as u32;
                let slot = self.create_explicit_stack_allocation(size);

                // TODO: Is there any way to write to a given address
                //       rather than needing to copy like this?

                for (field, expression) in fields {
                    // 2.1 Obtain the address containing the corresponding field's data
                    let field_value_address = self.translate_expression(expression);
                    let field_offset = self.validation_context.get_field_offset(ty, field).unwrap() as i32;
                    
                    // 2.2 Copy that data's bytes into the assigned slot
                    for byte in 0..self.validation_context.types.size_of(expression.get_type()) {
                        let offset = byte as i32 + field_offset;

                        let value = self.fn_builder.ins().load(types::I8, MemFlags::trusted(), field_value_address, byte as i32);
                        self.fn_builder.ins().stack_store(value, slot, offset as i32);
                    }
                }

                // 3. Return the address of the newly instantiated object
                self.fn_builder.ins().stack_addr(*self.pointer_type, slot, 0)
            }

            ast::Expression::Literal { value, ty } => 
                self.translate_expression_literal(value, ty),

            ast::Expression::BinaryExpression { lhs, op, rhs, ty } => {
                todo!()
            }
            ast::Expression::UnaryExpression { op, expr, ty } => {
                todo!()
            }
            ast::Expression::FunctionCall { name, inputs, ty } => {
                todo!()
            }
            ast::Expression::Block(_) => {
                todo!()
            }
        }
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