use crate::frontend::parse::ast;
use crate::frontend::validate::context::Context as ValidationContext;
use crate::frontend::validate::types::Type as CompilerType;

use cranelift::prelude::*;
use cranelift_module::Module; // for trait functions

use super::MemoryUsage;


//////////// CLIF Translation ////////////

// This file simply generates IR -- nothing more

// It might be possible to reuse this module for different targets
// such as generating standalone executables

/// Used to read from or write to a location in memory
#[derive(Clone)]
struct MemoryLocation {
    pub usage: MemoryUsage,
    pub offset: i32,
}

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
        crate::log!("Generating function `{}`:\n", function.prototype.name);
        
        // Create the function's entry block with appropriate function parameters
        let entry_block = self.fn_builder.create_block();
        self.fn_builder.append_block_params_for_function_params(entry_block);
        
        // Emit code within the entry block
        self.fn_builder.switch_to_block(entry_block);
        // No predecessors for entry blocks
        self.fn_builder.seal_block(entry_block);

        // Declare the function's parameters (entry block params)
        for (index, param) in function.prototype.parameters.iter().enumerate() {                        
            // Address is passed in to the function rather than actual value
            let param_addresss = self.fn_builder.block_params(entry_block)[index];
            self.data.register_variable(param.name, MemoryUsage::Address(param_addresss));
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
            self.translate_statement(statement)?;
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

    /// Writes the given value to the specified location
    fn write_to_memory(&mut self, location: &MemoryLocation, value: Value) {
        match location.usage {
            MemoryUsage::Stack(slot) => {
                self.fn_builder.ins().stack_store(value, slot, location.offset);
            }

            MemoryUsage::Address(address) => {
                self.fn_builder.ins().store(MemFlags::trusted(), value, address, location.offset);
            }
        }
    }

    /// Reads the specified location as the given type
    fn read_from_memory(&mut self, location: &MemoryLocation, target_type: Type) -> Value {
        match location.usage {
            MemoryUsage::Stack(slot) => {
                self.fn_builder.ins().stack_load(target_type, slot, location.offset)
            }

            MemoryUsage::Address(address) => {
                self.fn_builder.ins().load(target_type, MemFlags::trusted(), address, location.offset)
            }
        }
    }

    /// Returns the physical address of the given location
    fn get_location_address(&mut self, location: &MemoryLocation) -> Value {
        match location.usage {
            MemoryUsage::Stack(slot) => {
                self.fn_builder.ins().stack_addr(*self.pointer_type, slot, location.offset)
            }

            MemoryUsage::Address(address) => {
                // TODO: Ensure this is the proper way to get the final address
                self.fn_builder.ins().iadd_imm(address, location.offset as i64)
            },
        }
    }
    
    fn translate_statement(&mut self, statement: &ast::Statement) -> Result<(), String> {
        // NOTE: All types will be known and validated at this point
        match statement {
            ast::Statement::Let { ident, ty, value, .. } => {
                // FIXME: Narrowing cast
                let type_size = self.validation_context.types.size_of(ty) as u32;
                
                // Allocate this variable
                let stack_slot = self.fn_builder.create_stack_slot(StackSlotData {
                    kind: StackSlotKind::ExplicitSlot,
                    size: type_size,
                    offset: None,
                });
                                
                let mut memory_location = MemoryLocation {
                    usage: MemoryUsage::Stack(stack_slot),
                    offset: 0,
                };

                // Fill the variable with data if assigned
                if let Some(assignment) = value {
                    self.translate_expression(assignment, &mut memory_location);
                }

                self.data.register_variable(ident, memory_location.usage);
            }
            
            ast::Statement::Assign { variable, operator, expression } => {
                todo!();
            }

            // Move expression result to StructReturn, then return its address
            ast::Statement::ImplicitReturn { expression, is_function_return } => {
                if *is_function_return {
                    // Get StructReturn slot
                    let return_slot = *self.data.get_struct_return_slot();
                    let mut return_slot_location = MemoryLocation {
                        usage: MemoryUsage::Stack(return_slot),
                        offset: 0,
                    };

                    // Write data to be returned
                    let mut data = return_slot_location.clone();
                    self.translate_expression(expression, &mut data);

                    // FIXME: Copying all the data over is expensive and wasteful
                    //        This is only needed in the case of copying explicit slot data
                    //        to return slot.
                    for byte in 0..self.validation_context.types.size_of(expression.get_type()) {
                        // FIXME: Narrowing cast
                        data.offset = byte as i32;
                        return_slot_location.offset = byte as i32;

                        let value = self.read_from_memory(&data, types::I8);
                        self.write_to_memory(&return_slot_location, value);
                    }

                    // Return the StructReturn address
                    return_slot_location.offset = 0;
                    let return_address = self.get_location_address(&return_slot_location);
                    self.fn_builder.ins().return_(&[return_address]);
                }
            }
            
            // FIXME: This can be represented as an ImplicitReturn with `is_function_return` flag
            ast::Statement::Return { expression } => {
                todo!()
            }
            
            ast::Statement::Expression(expr) => {
                todo!()
            }
        }

        Ok(())
    }
    
    fn translate_expression(&mut self, expression: &ast::Expression, memory_location: &mut MemoryLocation) {
        match expression {
            ast::Expression::Literal { value, ty } => {
                self.translate_expression_literal(value, ty, memory_location);
            }

            ast::Expression::FieldConstructor { ty, fields } => {
                for (field, expression) in fields {
                    let field_offset = self.validation_context.get_field_offset(ty, field).unwrap();
                    
                    memory_location.offset = field_offset;
                    let mut memory_location2 = memory_location.clone();

                    self.translate_expression(expression, &mut memory_location2);
                }
            },

            // "Walks" the offsets to the proper location 
            // Note that subsequent field acceses refer to the same contiguous allocation
            ast::Expression::FieldAccess { base_expr, field, ty: _ } => {
                let field_offset = self.validation_context.get_field_offset(base_expr.get_type(), field)
                    .expect("Get field offset");

                memory_location.offset = field_offset;
                
                self.translate_expression(base_expr, memory_location);
            },

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
                    self.translate_expression(arg, memory_location);
                    values.push(self.get_location_address(memory_location));
                }

                let call = self.fn_builder.ins().call(func_ref, &values);

                // Result is an address -> want data
                let result_address = self.fn_builder.inst_results(call)
                    .get(0)
                    .map(|value| *value);

                // Store result in memory
                if let Some(address) = result_address {
                    memory_location.usage = MemoryUsage::Address(address);
                }
            },

            ast::Expression::BinaryExpression { lhs, op, rhs, ty } => todo!(),
            ast::Expression::UnaryExpression { op, expr, ty } => todo!(),
            ast::Expression::Block(_) => todo!(),

            // Point to the variable's address
            ast::Expression::Ident { name, ty } => {
                let ident_usage = self.data.get_variable_memory(name).clone();
                
                memory_location.usage = ident_usage;
            },
        }
    }

    fn translate_expression_literal(&mut self, literal: &ast::Literal, literal_type: &CompilerType, memory_location: &mut MemoryLocation) {        
        let value = match literal {
            ast::Literal::Integer(integer) => {
                // FIXME: Narrowing cast
                self.fn_builder.ins().iconst(literal_type.ir_type(self.pointer_type), *integer as i64)
            }

            ast::Literal::Float(float) => {
                // if not f32, then f64
                if let CompilerType::f32 = literal_type {
                    self.fn_builder.ins().f32const(*float as f32)
                } else {
                    self.fn_builder.ins().f64const(*float)
                }
            }

            ast::Literal::UnitType => {
                todo!("Assigning unit values")
            }
        };

        // Store the value into the designated location
        self.write_to_memory(&memory_location, value);
    }

    // TODO: Remove this once everything is functional again
    /*
    fn translate_expression_old(&mut self, expression: &ast::Expression) -> Result<Value, String> {
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
    */
}