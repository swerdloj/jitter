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


/// Builder for creating a `JitterContext`. Enables FFI linking
pub struct JitterContextBuilder<'a> {
    simple_jit_builder: SimpleJITBuilder,
    source_path: &'a str,
}

// TODO: Allow optimization settings to be passed in
// TODO: Accept/determine target ISA
impl<'a> JitterContextBuilder<'a> {
    pub fn new() -> Self {
        let mut settings = settings::builder();
        // TODO: Determine options here
        settings.set("opt_level", "speed_and_size").expect("Optimization");
        
        let isa_builder = isa::lookup(target_lexicon::Triple::host()).expect("isa");
        let isa = isa_builder.finish(settings::Flags::new(settings));

        let simple_jit_builder = SimpleJITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        Self {
            simple_jit_builder,
            source_path: "",
        }
    }

    /// Defines a Rust function called as `alias` with the body `pointer`.
    /// Usage:  
    /// `context.with_function("function_name", function_name as _)`
    pub fn with_function(mut self, alias: &str, pointer: *const u8) -> Self {
        self.simple_jit_builder.symbol(alias, pointer);
        self
    }

    pub fn with_source_path(mut self, path: &'a str) -> Self {
        self.source_path = path;
        self
    }

    // TODO: Compile multiple files instead of just one
    //       also allow context without source (include standard library)
    // pub fn add_source_path...

    // TODO: Return Result
    pub fn build(self) -> JitterContext {
        let mut jit_context = JitterContext::new(self.simple_jit_builder);
        
        if self.source_path != "" {
            let input = &std::fs::read_to_string(self.source_path).expect("Read input");
            let tokens = crate::frontend::lex::Lexer::lex_str(self.source_path, input, true);
            let parser = crate::frontend::parse::Parser::new(self.source_path, tokens);
            let ast = parser.parse_ast();
            let mut validation_context = crate::frontend::validate::context::Context::new();
            validation_context.validate(ast).expect("AST Validation");

            jit_context.translate(validation_context).expect("JIT-compile");
        }

        jit_context
    }
}


/// Contains all information needed to JIT compile and run the generated code
pub struct JitterContext {
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

impl Default for JitterContext {
    #[inline(always)]
    fn default() -> Self {
        JitterContextBuilder::new().build()
    }
}

impl JitterContext {
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
        // Begin by forward-declaring all possible functions
        // TODO: Do same for all constant values
        for (name, definition) in &validation_context.functions.functions {
            self.forward_declare_function(name, definition)?;
        }

        // Translate everything to IR
        // NOTE: Structs define layouts. They do not need translation.
        //       Similarly, ExternBlocks are accounted for as functions
        for function in &validation_context.ast.functions {
            self.generate_function(function, &validation_context)?;
        }
        for trait_ in &validation_context.ast.traits {
            todo!()
        }
        for impl_ in &validation_context.ast.impls {
            todo!()
        }

        // Performs linking
        self.module.finalize_definitions();

        Ok(())
    }

    fn forward_declare_function(&mut self, name: &str, definition: &crate::frontend::validate::FunctionDefinition) -> Result<cranelift_module::FuncId, String> {
        if self.functions.contains_key(name) {
            return Err(format!("Function `{}` was already defined", name));
        }

        let mut signature = self.module.make_signature();

        // All function inputs are the value's stack locations
        for _ in &definition.parameters {
            signature.params.push(AbiParam::new(self.pointer_type));
        }

        // Unit return type -> nothing returned
        if !definition.return_type.is_unit() {
            // Return value is the address of stack pre-allocation
            signature.returns.push(
                AbiParam::special(
                    self.pointer_type, 
                    codegen::ir::ArgumentPurpose::StructReturn
                )
            );
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

    // TODO: Consider moving this into codegen.rs to put all codegen in one place
    fn generate_function(&mut self, function: &ast::Function, validation_context: &ValidationContext) -> Result<(), String> {
        let func_id = self.functions.get(function.prototype.name)
            .ok_or(format!("Attempted to translate an unregistered function: {}", function.prototype.name))?;

        // Define the function parameters (passed in by stack address)
        for _ in &function.prototype.parameters.item {            
            self.fn_context.func.signature.params.push(
                AbiParam::new(self.pointer_type)
            );
        }
        
        let has_return_value = !function.prototype.return_type.is_unit();

        // Returns the stack pre-allocation address
        if has_return_value {
            self.fn_context.func.signature.returns.push(
                AbiParam::special(
                    self.pointer_type, 
                    cranelift::codegen::ir::ArgumentPurpose::StructReturn
                )
            );
        }
        
        let mut function_translator = FunctionTranslator::new(
            &self.pointer_type,
            FunctionBuilder::new(&mut self.fn_context.func, &mut self.fn_builder_context),
            &mut self.module,
            validation_context,
        );
        
        // Generates IR, then finalizes the function, making it ready for the module
        function_translator.translate_function(function, has_return_value)?;

        // Performs constant folding (I'm not sure what else is done elsewhere)
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