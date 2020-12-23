pub mod codegen;

/* 
TODO:

    1. Define functions in the language
    2. JIT the functions and store code
    3. Interpret inputs such that those functions can be called
        - This should support REPL-style addition of functions

    4. Get structs working

    5. Get struct impls working

    6. Allow use of Rust-defined functions
    7. Function args & returns should be valid

       Rust Code:
           #[language_link]
           fn name(a: u32, b: u32) -> u32 {..}
    
       Language Code:
           extern "Rust" fn name(a: u32, b: u32) -> u32;
           ...
           let x = name(1, 2);

       Need to ensure types are compatible and signatures can be understood

*/

use std::collections::HashMap;

use cranelift::prelude::*;

/// Simple helper data structure for associating cranelift `Variable`s with
/// `String` names
pub struct VarMap {
    variables: HashMap<String, Variable>,
    /// Each variable requires a unique index
    index: usize,
}

impl VarMap {
    pub fn new() -> Self {
        Self{
            variables: HashMap::new(),
            index: 0,
        }
    }

    pub fn create_var(&mut self, name: String) -> Variable {
        let var = Variable::new(self.index);
        self.index += 1;
        
        // TODO: Duplicate checking?
        self.variables.insert(name, var);

        var
    }

    pub fn get_var(&self, name: &str) -> Result<&Variable, String> {
        self.variables.get(name)
            .ok_or("Variable `{}` does not exist".to_owned())
    }
}