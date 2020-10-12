// TODO: Consider a trait implemented by all AST types which generates cranelift
//       code

// Reference: https://github.com/bytecodealliance/simplejit-demo/blob/main/src/jit.rs

use crate::frontend::parse::ast;

use cranelift::prelude::*;
use cranelift_module::{Module, Linkage, DataContext};
use cranelift_simplejit::{SimpleJITBackend, SimpleJITBuilder};

/// Contains all information needed to JIT compile and run the generated code
pub struct JITContext {
    type_map: super::types::TypeMap,

    // FIXME: How is `Context` related to functions?
    fn_builder_context: FunctionBuilderContext,
    fn_context: codegen::Context,
    
    data_context: DataContext,

    module: Module<SimpleJITBackend>,
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
        }
    }

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

        todo!()
    }

    // TODO: How would structs, etc. work?
    fn generate_function(&mut self, function: &ast::Function) -> Result<*const u8, String> {
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

        // Obtain pointer to the generated function
        let code = self.module.get_finalized_function(id);

        Ok(code)
    }
}

// Translation functions
impl JITContext {
    fn translate_function(&mut self, function: &ast::Function) -> Result<(), String> {
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

        // TODO: Declare variables here. Can nodes declare their own variables,
        //       or do they all need to be declared first?

        // TODO: Fill the function with ir code here

        // TODO: Obtain and set the return variable 

        // TODO: Finalize the builder

        todo!()
    }
}