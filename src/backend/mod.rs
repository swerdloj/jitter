/// Contains functionality for translating an AST into CLIF
mod codegen;
/// Contains the JIT driver
pub mod jit;


use std::collections::HashMap;

use cranelift::prelude::{Value, Variable, EntityRef};
use cranelift::codegen::ir::StackSlot;

/// Simple helper data structure for associating cranelift `Variable`s with `String` names.
/// Note that `String` keys are used to remove dependency from source file
pub struct DataMap {
    /// IR compatible types are allocated as `Variable`s
    variables: HashMap<String, Variable>,
    /// Custom types (e.g.: user-defined) require explicit allocations.  
    /// This is a map of (address -> stack slot)
    stack_slots: HashMap<Value, StackSlot>,
    /// Each variable requires a unique index.
    /// This is automatically incremented each time `create_var()` is called
    index: usize,
}

impl DataMap {
    /// Returns an empty `DataMap`
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            stack_slots: HashMap::new(),
            index: 0,
        }
    }

    /// Stores an IR variable by name
    pub fn create_var(&mut self, name: impl Into<String>) -> Variable {
        let var = Variable::new(self.index);
        self.index += 1;
        
        // TODO: Duplicate checking?
        self.variables.insert(name.into(), var);

        var
    }

    /// Get an IR variable by name
    pub fn get_var(&self, name: &str) -> Result<&Variable, String> {
        self.variables.get(name)
            .ok_or(format!("Variable `{}` does not exist", name))
    }

    /// Stores a `StackSlot` by allocation address
    pub fn register_stack_slot(&mut self, address: Value, slot: StackSlot) {
        self.stack_slots.insert(address, slot);
    }

    /// Get the `StackSlot` at a given address
    pub fn get_stack_slot(&mut self, address: &Value) -> Result<&StackSlot, String> {
        self.stack_slots.get(address)
            .ok_or(format!("Address `{}` not found", address))
    }
}