use std::collections::HashMap;

use cranelift::codegen::ir::{types, Type};

// TODO: How to deal with user-defined types? Tuples? Arrays? Enums? Strings?
//       They would all resolve to these base types, but how to associate them?
pub struct TypeMap {
    type_map: HashMap<String, Type>,
}

// TODO: This should work on frontend::validate::types::Type, not strings
//       This should be done via `impl Into<ir::Type> for Type`
impl TypeMap {
    pub fn new() -> Self {
        let mut type_map = HashMap::new();
        // NOTE: `I` is for integer, not signed. The sign is not determined here.
        type_map.insert("i8".to_owned(), types::I8);
        type_map.insert("i32".to_owned(), types::I32);
        type_map.insert("i64".to_owned(), types::I64);
        type_map.insert("i128".to_owned(), types::I128);

        type_map.insert("u8".to_owned(), types::I8);
        type_map.insert("u32".to_owned(), types::I32);
        type_map.insert("u64".to_owned(), types::I64);
        type_map.insert("u128".to_owned(), types::I128);

        type_map.insert("f32".to_owned(), types::F32);
        type_map.insert("f64".to_owned(), types::F64);

        type_map.insert("bool".to_owned(), types::B1);

        // FIXME: Is this correct for void type? Should wrap `Type` in an enum to account for this?
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