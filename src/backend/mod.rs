/// Contains functionality for translating an AST into CLIF
mod codegen;
/// Contains the JIT driver
pub mod jit;


use std::collections::HashMap;

use cranelift::prelude::Value;
use cranelift::codegen::ir::StackSlot;


/// Stores the location of a variable
#[derive(Clone)]
pub enum MemoryUsage {
    Stack(StackSlot),
    Address(Value),
}

/// Maps variables to their in-memory representations
pub struct MemoryMap {
    /// Map of (variable -> location)
    variables: HashMap<String, MemoryUsage>,

    /// Special StructReturnSlot. If a function returns a value, it must be stored here.
    struct_return_slot: Option<StackSlot>,
}

impl MemoryMap {
    /// Returns an empty `DataMap`
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            struct_return_slot: None,
        }
    }

    // TODO: Overwrite check?
    pub fn register_struct_return_slot(&mut self, slot: StackSlot) {
        self.struct_return_slot = Some(slot);
    }

    // TODO: Error check?
    pub fn get_struct_return_slot(&self) -> &StackSlot {
        self.struct_return_slot.as_ref().expect("get struct return slot")    }

    /// Associates a variable with a StackSlot at the specified address
    pub fn register_variable(&mut self, name: &str, usage: MemoryUsage) {
        // TODO: Duplicate checking?
        self.variables.insert(name.into(), usage);
    }
    
    pub fn get_variable_memory(&mut self, name: &str) -> &MemoryUsage {
        // TODO: Duplicate checking?
        self.variables.get(name).expect("get variable location")
    }
}