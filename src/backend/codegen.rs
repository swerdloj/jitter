// TODO: Consider a trait implemented by all AST types which generates cranelift
//       code

// References: 
// https://github.com/bytecodealliance/simplejit-demo/blob/main/src/jit.rs
// https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/simplejit/examples/simplejit-minimal.rs

use crate::frontend::parse::ast;

use cranelift::prelude::*;
use cranelift_module::{Module, Linkage, DataContext};
use cranelift_simplejit::{SimpleJITBackend, SimpleJITBuilder};

use std::collections::HashMap;

/// Contains all information needed to JIT compile and run the generated code
pub struct JITContext {
    type_map: super::types::TypeMap,

    // FIXME: How is `Context` related to functions?
    fn_builder_context: FunctionBuilderContext,
    fn_context: codegen::Context,
    
    data_context: DataContext,

    module: Module<SimpleJITBackend>,

    // TEMP: for testing
    functions: HashMap<String, cranelift_module::FuncId>,
}

// High-level functionality (lower-level functionality in second impl block)
impl JITContext {
    pub fn new() -> Self {
        let builder = SimpleJITBuilder::new(cranelift_module::default_libcall_names());
        let module = Module::new(builder);

        Self {
            type_map: super::types::TypeMap::new(),
            fn_builder_context: FunctionBuilderContext::new(),
            fn_context: module.make_context(),
            data_context: DataContext::new(),
            module,

            // TEMP:
            functions: HashMap::new(),
        }
    }

    // TODO: Need a way to verify signature
    pub fn get_fn(&mut self, id: &str) -> *const u8 {
        let func_id = self.functions.get(id).expect("no such function");
        self.module.get_finalized_function(*func_id)
    }

    // TODO: Utilize the validation context
    pub fn translate(&mut self, ast: ast::AST) -> Result<(), String> {
        for node in ast {
            match node {
                ast::TopLevel::Function(function) => {
                    self.generate_function(&function.item)?;
                }

                ast::TopLevel::Struct(_) => {
                    todo!()
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
    fn generate_function(&mut self, function: &ast::Function) -> Result<(), String> {
        // TEMP: debug
        println!("Generating `{}`\n", function.name);

        // Define the function parameters
        for parameter in &function.parameters.item {
            let param_type = self.type_map.get(parameter.item.field_type)?;
            
            self.fn_context.func.signature.params.push(
                AbiParam::new(*param_type)
            );
        }
        
        // Set return variable
        let return_type = *self.type_map.get(function.return_type)?;

        if return_type != types::INVALID {
            self.fn_context.func.signature.returns.push(
                AbiParam::new(return_type)
            );
        }
        
        let mut function_translator = FunctionTranslator {
            fn_builder: FunctionBuilder::new(&mut self.fn_context.func, &mut self.fn_builder_context),
            type_map: &mut self.type_map,
            variables: super::VarMap::new(),
        };
        
        // Generates IR, then finalizes the function, making it ready for the module
        function_translator.translate_function(function, return_type)?;
        

        // Initial declaration (C-style?)
        let id = self.module
            .declare_function(function.name, Linkage::Local, &self.fn_context.func.signature)
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
        self.functions.insert(function.name.to_owned(), id);

        Ok(())
    }
}

//// CLIF Translation ////

// Translates a function and its contents into IR
struct FunctionTranslator<'a> {
    fn_builder: FunctionBuilder<'a>,
    type_map: &'a mut super::types::TypeMap,
    // Maps `Variable`s with names
    variables: super::VarMap,
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
        for (index, param_node) in function.parameters.item.iter().enumerate() {            
            let param_type = self.type_map.get(param_node.item.field_type)?;
            
            let var = self.variables.create_var(param_node.item.field_name.to_owned());
            
            // Decalre the parameter and its type
            self.fn_builder.declare_var(var, *param_type);
            // Define the parameter with the values passed when calling the function
            self.fn_builder.def_var(var, self.fn_builder.block_params(entry_block)[index]);
        }
        
        // Declare the function's return Variable
        let return_var = self.variables.create_var("return".to_owned());
        self.fn_builder.declare_var(return_var, return_type);
        
        for statement in &function.statements.item {
            self.translate_statement(&statement.item)?;
        }
        
        // TODO: This should be associated with `Statement::Return`
        // Specify and use the return variable (if the function returns anything)
        if return_type != types::INVALID {
            let return_value = self.fn_builder.use_var(return_var);
            self.fn_builder.ins().return_(&[return_value]);
        } else {
            self.fn_builder.ins().return_(&[]);
        }
        
        self.fn_builder.finalize();
        
        
        // TEMP: debug
        print!("{}:\n{}\n", function.name, self.fn_builder.display(None));
        
        Ok(())
    }
    
    fn translate_statement(&mut self, statement: &ast::Statement) -> Result<(), String> {
        // NOTE: All types will be known and validated at this point
        match statement {
            // TODO: Utilize knowledge of mutability
            // Create a new variable and assign it if an expression is given
            ast::Statement::Let { ident, mutable, type_, value } => {
                let var = self.variables.create_var(ident.to_string());
                self.fn_builder.declare_var(
                    var, 
                    *self.type_map.get(type_.expect("Type should be known by now"))?
                );
                
                if let Some(value) = value {
                    let assigned_value = self.translate_expression(&value.item)?;
                    self.fn_builder.def_var(var, assigned_value);
                }
            }
            
            // Assign a value to a variable
            ast::Statement::Assign { variable, operator, expression } => {
                todo!()
            }
            
            ast::Statement::Return { expression } => {
                todo!()
            }
            
            ast::Statement::Expression(expr) => {
                self.translate_expression(&expr.item)?;
            }
        }

        Ok(())
    }
    
    fn translate_expression(&mut self, expression: &ast::Expression) -> Result<Value, String> {
        let value = match expression {
            ast::Expression::BinaryExpression { lhs, op, rhs } => {
                todo!()
            }

            ast::Expression::UnaryExpression { op, expr } => {
                todo!()
            }

            ast::Expression::Parenthesized(expr) => {
                todo!()
            }

            ast::Expression::Literal(literal) => {
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