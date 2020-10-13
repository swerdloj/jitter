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

    // TEMP: For testing
    functions: HashMap<String, usize>,
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

            functions: HashMap::new(),
        }
    }

    pub fn get_fn(&mut self, id: &str) -> *const u8 {
        let func_id = self.functions.get(id).expect("no such function");
        self.module.get_finalized_function(cranelift_module::FuncId::new(*func_id))
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
        self.translate_function(function)?;
        
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
        self.functions.insert(function.name.to_owned(), id.as_u32() as usize);

        Ok(())
    }
}

// Translation functions
impl JITContext {
    fn translate_function(&mut self, function: &ast::Function) -> Result<(), String> {
        // TEMP: debug
        println!("Translating `{}`\n", function.name);
        
        // Define the function parameters
        for parameter in &function.parameters.item {
            let param_type = self.type_map.get(parameter.item.field_type)?;
            
            self.fn_context.func.signature.params.push(
                AbiParam::new(*param_type)
            );
        }

        let return_type = *self.type_map.get(function.return_type)?;

        if return_type != types::INVALID {
            self.fn_context.func.signature.returns.push(
                AbiParam::new(return_type)
            );
        }

        let mut builder = FunctionBuilder::new(&mut self.fn_context.func, &mut self.fn_builder_context);

        // Create the function's entry block with appropriate function parameters
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);

        // Emit code within the entry block
        builder.switch_to_block(entry_block);
        // No predecessors for entry blocks
        builder.seal_block(entry_block);

        // TODO: Create a data structure to handle this grouping
        let mut variable_map = HashMap::new();
        let mut variable_index = 0;

        // Declare the function's parameters (entry block params)
        for (index, param_node) in function.parameters.item.iter().enumerate() {            
            let param_type = self.type_map.get(param_node.item.field_type)?;
            
            let var = Variable::new(variable_index);
            // FIXME: duplicate name checking? how would shadowing affect this?
            variable_map.insert(param_node.item.field_name.to_owned(), var);

            // Decalre the parameter and its type
            builder.declare_var(var, *param_type);
            // Define the parameter with the values passed when calling the function
            builder.def_var(var, builder.block_params(entry_block)[index]);
            
            variable_index += 1;
        }

        // Declare the function's return value
        let return_var = Variable::new(variable_index);
        // NOTE: because `return` is a keyword, it cannot be duplicated
        variable_map.insert("return".to_owned(), return_var);
        builder.declare_var(return_var, return_type);
        // TODO: Do variables need to be manually initialized to zero?
        // builder.def_var(return_var, 0);
        variable_index += 1;


        for statement in &function.statements.item {
            // TODO: Fill the function with ir code here
        }

        // Specify and use the return variable
        let return_value = builder.use_var(return_var);
        builder.ins().return_(&[return_value]);

        builder.finalize();


        // TEMP: debug
        let output = builder.display(None);
        print!("{}:\n{}\n", function.name, output);

        Ok(())
    }

    fn translate_statement(&mut self, statement: ast::Statement) -> Result<(), String> {
        match statement {
            ast::Statement::Let { ident, mutable, type_, value } => {

            }

            ast::Statement::Assign { variable, operator, expression } => {

            }

            ast::Statement::Return { expression } => {

            }

            ast::Statement::Expression(_) => {

            }
        }
        
        todo!()
    }

    fn translate_expression(&mut self) -> Result<(), String> {
        todo!()
    }
}