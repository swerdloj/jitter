// References: 
// https://github.com/bytecodealliance/simplejit-demo/blob/main/src/jit.rs
// https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/simplejit/examples/simplejit-minimal.rs
// https://github.com/CraneStation/kaleidoscope-cranelift

use crate::frontend::parse::ast;
use crate::frontend::validate::context::Context as ValidationContext;
use crate::backend::codegen::FunctionTranslator;

use cranelift::prelude::*;
use cranelift_module::{Module, Linkage, DataContext};
use cranelift_simplejit::{SimpleJITBuilder, SimpleJITModule};

use std::collections::HashMap;


/// Builder for creating a `JITContext`. Enables FFI linking
pub struct JITContextBuilder<'a> {
    simple_jit_builder: SimpleJITBuilder,
    source_path: &'a str,
}

// TODO: Allow optimization settings to be passed in
// TODO: Accept/determine target ISA
impl<'a> JITContextBuilder<'a> {
    pub fn new() -> Self {
        let mut settings = settings::builder();
        // can also do "speed_and_size"
        settings.set("opt_level", "speed").expect("Optimization");
        
        let isa_builder = isa::lookup(target_lexicon::Triple::host()).expect("isa");
        let isa = isa_builder.finish(settings::Flags::new(settings));

        let simple_jit_builder = SimpleJITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        Self {
            simple_jit_builder,
            source_path: "",
        }
    }

    // pub fn with_function(mut self, name: &str, function_address: *const u8) -> Self {
    /// Call this using the `FFI!` macro:  
    /// `builder.with_function(FFI!(function_name))`
    pub fn with_function(mut self, function: (&str, *const u8)) -> Self {
        self.simple_jit_builder.symbol(function.0, function.1);
        self
    }

    pub fn with_source_path(mut self, path: &'a str) -> Self {
        self.source_path = path;
        self
    }

    // TODO: Return Result
    pub fn build(self) -> JITContext {
        let mut jit_context = JITContext::new(self.simple_jit_builder);
        
        if self.source_path != "" {
            let input = &std::fs::read_to_string(self.source_path).expect("Read input");
            let tokens = crate::frontend::lex::Lexer::lex_str(self.source_path, input, true);
            let parser = crate::frontend::parse::Parser::new(self.source_path, tokens);
            let ast = parser.parse_ast();
            let validation_context = crate::frontend::validate::validate_ast(ast).expect("AST Validation");

            jit_context.translate(validation_context).expect("JIT-compile");
        }

        jit_context
    }
}


/// Contains all information needed to JIT compile and run the generated code
pub struct JITContext {
    // FIXME: How is `Context` related to functions?
    fn_builder_context: FunctionBuilderContext,
    fn_context: codegen::Context,
    
    data_context: DataContext,

    module: SimpleJITModule,

    // TODO: Store additional information such as (id, is_defined, param_count, etc.)
    functions: HashMap<String, cranelift_module::FuncId>,

    /// Target architecture's pointer type
    pointer_type: Type,
}

impl Default for JITContext {
    #[inline(always)]
    fn default() -> Self {
        JITContextBuilder::new().build()
    }
}

impl JITContext {
    pub fn new(builder: SimpleJITBuilder) -> Self {
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
        for (name, definition) in &validation_context.functions {
            self.forward_declare_function(name, definition)?;
        }

        for node in &validation_context.ast {
            match node {
                ast::TopLevel::ExternBlock(externs) => {
                    // already declared above -> nothing to do
                }
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

        // Performs linking
        self.module.finalize_definitions();

        // TODO: How to print all of the generated IR?

        Ok(())
    }

    // This will be called from `extern` blocks within jitter
    // fn declare_ffi_fn(&mut self, prototype: &ast::FunctionPrototype) -> Result<(), String> {
    //     if self.functions.contains_key(prototype.name) {
    //         Err(format!("Function `{}` already exists", prototype.name))
    //     } else {
    //         let mut signature = self.module.make_signature();

    //         for x in &prototype.parameters.item {
                
    //         }

    //         let func_id = self.module.declare_function(prototype.name, Linkage::Import, &signature)
    //             .map_err(|e| e.to_string())?;
            
    //         self.functions.insert(prototype.name.to_string(), func_id);

    //         Ok(())
    //     }
    // }

    fn forward_declare_function(&mut self, name: &str, definition: &crate::frontend::validate::context::FunctionDefinition) -> Result<cranelift_module::FuncId, String> {
        if self.functions.contains_key(name) {
            return Err(format!("Function `{}` was already defined", name));
        }

        let mut signature = self.module.make_signature();

        for (_field_name, ty, _mutable) in &definition.parameters {
            signature.params.push(AbiParam::new(ty.ir_type(&self.pointer_type)));
        }

        // Unit return type -> nothing returned
        if !definition.return_type.is_unit() {
            signature.returns.push(AbiParam::new(definition.return_type.ir_type(&self.pointer_type)));
        }

        let linkage = if definition.is_extern {
            Linkage::Import
        } else {
            Linkage::Local
        };

        let func_id = self.module.declare_function(name, linkage, &signature)
            .map_err(|e| e.to_string())?;

        // TODO: Store additional information in `functions`
        self.functions.insert(name.to_string(), func_id);

        Ok(func_id)
    }

    // TODO: Much of this functionality needs to be moved to `codegen.rs`
    //       My goal is to have code generation occur **only** in `codegen.rs`
    //       Nothing else should touch IR generation
    fn generate_function(&mut self, function: &ast::Function, validation_context: &ValidationContext) -> Result<(), String> {
        let func_id = self.functions.get(function.prototype.name)
            .ok_or(format!("Attempted to translate an unregistered function: {}", function.prototype.name))?;
        
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
            pointer_type: &self.pointer_type,
            fn_builder: FunctionBuilder::new(&mut self.fn_context.func, &mut self.fn_builder_context),
            module: &mut self.module,
            data: super::DataMap::new(),
            validation_context,
        };
        
        // Generates IR, then finalizes the function, making it ready for the module
        function_translator.translate_function(function, return_type)?;
        // Performs constant folding (only optimization for now)
        cranelift_preopt::optimize(&mut self.fn_context, self.module.isa()).expect("Optimize");
        
        // Define the function
        self.module
            .define_function(*func_id, &mut self.fn_context, &mut codegen::binemit::NullTrapSink{})
            .map_err(|e| e.to_string())?;

        // Reset the context for the next function
        self.module.clear_context(&mut self.fn_context);

        Ok(())
    }
}