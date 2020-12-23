// TODO: Support arrays, type aliases, & traits

use cranelift::codegen::ir::types as cranelift_types;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// `()` type
    Unit,

    /// (A, B, C, ...)
    Tuple(Vec<Type<'input>>),
    
    /// Name of a struct, enum, alias, etc.
    User(&'input str),

    /// Unspecified and uninferred type 
    Unknown,
}

impl<'input> Type<'input> {
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

            // TODO: Tuples? Arrays?
            _ => Type::User(type_str),
        }
    }

    pub fn is_unknown(&self) -> bool {
        self == &Type::Unknown
    }

    pub fn ir_type(&self) -> cranelift_types::Type {
        // NOTE: `I` is for `integer` -> sign is not regarded
        match self {
            Type::u8 => cranelift_types::I8,
            Type::u16 => cranelift_types::I16,
            Type::u32 => cranelift_types::I32,
            Type::u64 => cranelift_types::I64,
            Type::u128 => cranelift_types::I128,

            Type::i8 => cranelift_types::I8,
            Type::i16 => cranelift_types::I16,
            Type::i32 => cranelift_types::I32,
            Type::i64 => cranelift_types::I64,
            Type::i128 => cranelift_types::I128,

            Type::f32 => cranelift_types::F32,
            Type::f64 => cranelift_types::F64,

            Type::bool => cranelift_types::B1,

            // TODO: What to do about these?
            Type::Unit => cranelift_types::INVALID,
            Type::Tuple(_) => cranelift_types::INVALID,
            Type::Unknown => cranelift_types::INVALID,

            Type::User(_) => cranelift_types::INVALID,
        }
    }
}