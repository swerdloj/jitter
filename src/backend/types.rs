use std::collections::HashMap;

use cranelift::codegen::ir::{types, Type};

// TODO: How to deal with user-defined types? Tuples? Arrays? Enums?
pub struct TypeMap {
    type_map: HashMap<String, Type>,
}

impl TypeMap {
    pub fn new() -> Self {
        let mut type_map = HashMap::new();
        type_map.insert("i32".to_owned(), types::I32);
        type_map.insert("u32".to_owned(), types::I32);
        // FIXME: Is this correct for void type?
        type_map.insert("()".to_owned(), types::INVALID);

        Self {
            type_map,
        }
    }

    pub fn get(&self, t: &str) -> Result<&Type, String> {
        self.type_map.get(t)
            .ok_or(format!("Type not defined: `{}`", t))
    }
}