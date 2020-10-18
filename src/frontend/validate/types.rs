// TODO: Support arrays, type aliases, & traits

#[allow(non_camel_case_types)]
pub enum Type<'input> {
    u8,
    u16,
    u32,
    u64,
    u128,

    i8,
    i16,
    i32,
    i64,
    i128,

    f32,
    f64,

    bool,

    Unit,

    // Name of a struct
    Struct(&'input str),
}

// impl<'input> Type<'input> {
    /// Resolves a type (as text) obtained from lexer/parser to an internal type
    pub fn resolve(type_str: &str) -> Type {
        match type_str {
            "u8" => Type::u8,
            "u16" => Type::u16,
            "u32" => Type::u32,
            "u64" => Type::u64,
            "u128" => Type::u128,

            "i8" => Type::i8,
            "i16" => Type::i16,
            "i32" => Type::i32,
            "i64" => Type::i64,
            "i128" => Type::i128,

            "f32" => Type::f32,
            "f64" => Type::f64,

            "bool" => Type::bool,

            "()" => Type::Unit,

            _ => Type::Struct(type_str),
        }
    }
// }
