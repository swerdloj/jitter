// References: 
// https://github.com/bytecodealliance/simplejit-demo/blob/main/src/jit.rs
// https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/simplejit/examples/simplejit-minimal.rs
// https://github.com/CraneStation/kaleidoscope-cranelift

use crate::frontend::parse::ast;
use crate::frontend::validate::context::Context as ValidationContext;

use cranelift::prelude::*;
use cranelift_module::{Module, Linkage, DataContext};
use cranelift_simplejit::{SimpleJITBuilder, SimpleJITModule};

use std::collections::HashMap;

/// Contains all information needed to JIT compile and run the generated code
pub struct JITContext {
    // FIXME: How is `Context` related to functions?
    fn_builder_context: FunctionBuilderContext,
    fn_context: codegen::Context,
    
    data_context: DataContext,

    module: SimpleJITModule,

    // TEMP: for testing
    functions: HashMap<String, cranelift_module::FuncId>,

    pointer_type: Type,
}

impl JITContext {
    // TODO: Allow optimization settings to be passed in
    // TODO: Accept/determine target ISA
    // TODO: Declare functions (using validation context) first, then translate their bodies
    pub fn new() -> Self {
        let mut settings = settings::builder();
        // can also do "speed_and_size"
        settings.set("opt_level", "speed").expect("Optimization");
        
        let isa_builder = isa::lookup(target_lexicon::Triple::host()).expect("isa");
        let isa = isa_builder.finish(settings::Flags::new(settings));

        let builder = SimpleJITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = SimpleJITModule::new(builder);

        let pointer_type = module.target_config().pointer_type();
        crate::log!("Pointer type is: {}\n", pointer_type);

        Self {
            fn_builder_context: FunctionBuilderContext::new(),
            fn_context: module.make_context(),
            data_context: DataContext::new(),
            module,

            // TEMP: Might want to keep this or make it optional depending on usage
            functions: HashMap::new(),

            pointer_type,
        }
    }

    // TODO: Need a way to verify signature
    pub fn get_fn(&mut self, id: &str) -> *const u8 {
        let func_id = self.functions.get(id).expect("no such function");
        self.module.get_finalized_function(*func_id)
    }

    // NOTE:
    // All code represented by the validation context is assumed to be valid
    pub fn translate(&mut self, validation_context: ValidationContext) -> Result<(), String> {
        // TODO: Use validation context to declare all functions first
        // for function in &validation_context.functions {
        // }


        for node in &validation_context.ast {
            match node {
                ast::TopLevel::Function(function) => {
                    self.generate_function(&function, &validation_context)?;
                }

                ast::TopLevel::Trait(trait_) => {
                    todo!()
                }

                ast::TopLevel::Impl(impl_) => {
                    todo!()
                }

                // Struct informs the compiler of raw data. There is nothing to translate (except impls).
                ast::TopLevel::Struct(_) => {
                    // Nothing to do here
                }

                ast::TopLevel::ConstDeclaration => {
                    todo!()
                }

                ast::TopLevel::UseStatement => {
                    todo!()
                }
            }
        }

        // TODO: How to print all of the generated IR?

        Ok(())
    }

    // TODO: How would structs, etc. work?
    fn generate_function(&mut self, function: &ast::Function, validation_context: &ValidationContext) -> Result<(), String> {
        // TEMP: debug
        crate::log!("Generating function `{}`:\n", function.prototype.name);

        // Define the function parameters
        for parameter in &function.prototype.parameters.item {
            let param_type = parameter.field_type.ir_type(&self.pointer_type);
            
            self.fn_context.func.signature.params.push(
                AbiParam::new(param_type)
            );
        }
        
        // Set return variable
        let return_type = function.prototype.return_type.ir_type(&self.pointer_type);

        if return_type != types::INVALID {
            self.fn_context.func.signature.returns.push(
                AbiParam::new(return_type)
            );
        }
        
        let mut function_translator = FunctionTranslator {
            fn_builder: FunctionBuilder::new(&mut self.fn_context.func, &mut self.fn_builder_context),
            pointer_type: &self.pointer_type,
            variables: super::VarMap::new(),
            validation_context,
        };
        
        // Generates IR, then finalizes the function, making it ready for the module
        function_translator.translate_function(function, return_type)?;
        // Performs constant folding (only optimization for now)
        cranelift_preopt::optimize(&mut self.fn_context, self.module.isa()).expect("Optimize");
        
        // Initial declaration (C-style?)
        let id = self.module
            .declare_function(function.prototype.name, Linkage::Local, &self.fn_context.func.signature)
            .map_err(|e| e.to_string())?;
        
        // Define the function
        self.module
            .define_function(id, &mut self.fn_context, &mut codegen::binemit::NullTrapSink{})
            .map_err(|e| e.to_string())?;

        // Reset the context for the next function
        self.module.clear_context(&mut self.fn_context);

        // FIXME: Is this the correct location? Would probably want to declare all functions at once,
        //        then define them individually, then finalize them all at once
        // Finalizes the function, making it ready for use
        self.module.finalize_definitions();

        // TEMP: for testing
        self.functions.insert(function.prototype.name.to_owned(), id);

        Ok(())
    }
}

//////////// CLIF Translation ////////////

// Translates a function and its contents into IR
struct FunctionTranslator<'a> {
    fn_builder: FunctionBuilder<'a>,
    pointer_type: &'a Type,
    // Maps `Variable`s with names
    variables: super::VarMap,
    validation_context: &'a ValidationContext<'a>,
}

impl FunctionTranslator<'_> {
    fn translate_function(&mut self, function: &ast::Function, return_type: Type) -> Result<(), String> {                
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
            
            let var = self.variables.create_var(param_node.field_name.to_owned());
            
            // Decalre the parameter and its type
            self.fn_builder.declare_var(var, param_type);
            // Define the parameter with the values passed when calling the function
            self.fn_builder.def_var(var, self.fn_builder.block_params(entry_block)[index]);
        }
        
        // Declare the function's return Variable
        let return_var = self.variables.create_var("return".to_owned());
        self.fn_builder.declare_var(return_var, return_type);
        
        for statement in &function.body.block.item {
            self.translate_statement(statement)?;
        }
        
        // Return nothing if function has no return type
        if return_type == types::INVALID {
            self.fn_builder.ins().return_(&[]);
        }
        
        self.fn_builder.finalize();
        
        // TEMP: debug
        crate::log!("{}", self.fn_builder.display(None));
        
        Ok(())
    }

    fn allocate_explicit_stack_data(&mut self, bytes: u32) -> codegen::ir::StackSlot {
        self.fn_builder.create_stack_slot(
            StackSlotData::new(
                StackSlotKind::ExplicitSlot, 
                bytes
            )
        )
    }
    
    fn translate_statement(&mut self, statement: &ast::Statement) -> Result<(), String> {
        // NOTE: All types will be known and validated at this point
        match statement {
            // TODO: Utilize knowledge of mutability
            // Create a new variable and assign it if an expression is given
            ast::Statement::Let { ident, mutable, ty, value } => {
                let var = self.variables.create_var(*ident);
                
                // Allocate stack slot if type is not an IR type
                if ty.is_builtin() {
                    self.fn_builder.declare_var(
                        var,
                        ty.ir_type(&self.pointer_type)
                    );

                    if let Some(expr) = value {
                        let assigned_value = self.translate_expression(expr)?;
                        self.fn_builder.def_var(var, assigned_value);
                    }
                } else {
                    let bytes_needed = self.validation_context.types.size_of(ty);
                    // FIXME: Casting usize to u32 here is (potentially) bad
                    let stack_slot = self.allocate_explicit_stack_data(bytes_needed as u32);
                    self.variables.register_stack_slot(*ident, stack_slot);
                    
                    // TODO: Fill stack slot with data
                    if let Some(expr) = value {
                        todo!()
                    }
                }
            }
            
            // Assign a value to a variable
            ast::Statement::Assign { variable, operator, expression } => {
                let expr_value = self.translate_expression(expression)?;
                let var = self.variables.get_var(variable)?;
                
                match operator.item {
                    ast::AssignmentOp::Assign => {
                        self.fn_builder.def_var(*var, expr_value);
                    }
                    ast::AssignmentOp::AddAssign => {
                        todo!();
                    }
                    ast::AssignmentOp::SubtractAssign => {
                        todo!();
                    }
                    ast::AssignmentOp::MultiplyAssign => {
                        todo!();
                    }
                    ast::AssignmentOp::DivideAssign => {
                        todo!();
                    }
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

            ast::Expression::FieldConstructor { type_name, fields } => {
                todo!()
            }

            ast::Expression::FieldAccess { base_expr, field, ty  } => {
                todo!()
            }

            ast::Expression::FunctionCall { function, inputs } => {
                todo!()
            }

            ast::Expression::Block(block) => {
                todo!()
            }

            // Parentheses only matter during parsing
            ast::Expression::Parenthesized (expr) => {
                self.translate_expression(expr)?
            }

            ast::Expression::Literal(literal) => {
                match literal {
                    ast::Literal::Integer(_) => {}
                    ast::Literal::Float(_) => {}
                    ast::Literal::UnitType => {}
                }

                todo!()
            }
            
            // Get IR reference to the variable
            ast::Expression::Ident(ident) => {
                let var = self.variables.get_var(ident)?;
                self.fn_builder.use_var(*var)
            }
        };

        Ok(value)
    }   
}