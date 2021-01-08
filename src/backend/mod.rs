/// Contains functionality for translating an AST into CLIF
mod codegen;
/// Contains the JIT driver
pub mod jit;


use std::collections::HashMap;

use cranelift::prelude::{Value, Variable, EntityRef};
use cranelift::codegen::ir::StackSlot;


/// Maps variables to their in-memory representations
pub struct MemoryMap {
    /// Map of (variable name -> cranelift variable index)
    variables: HashMap<String, Variable>,
    index: usize,

    /// Special StructReturnSlot. If a function returns a value, it must be stored here.
    struct_return_slot: Option<StackSlot>,
}

impl MemoryMap {
    /// Returns an empty `DataMap`
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            index: 0,
            struct_return_slot: None,
        }
    }

    // TODO: Overwrite check?
    pub fn register_struct_return_slot(&mut self, slot: StackSlot) {
        self.struct_return_slot = Some(slot);
    }

    pub fn get_struct_return_slot(&self) -> &StackSlot {
        // TODO: Error check?
        self.struct_return_slot.as_ref().expect("get_struct_return_slot")
    }

    pub fn create_variable(&mut self, name: &str) -> Variable {
        let variable = Variable::new(self.index);
        self.index += 1;

        // TODO: Duplicate checking?
        self.variables.insert(name.into(), variable);

        variable
    }

    pub fn get_variable(&self, name: &str) -> Variable {
        // TODO: Error check?
        *self.variables.get(name).expect("get_variable")
    }
}