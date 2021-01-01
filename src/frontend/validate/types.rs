// TODO: Support arrays, type aliases, & traits

use cranelift::codegen::ir::types as cranelift_types;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum Type<'input> {
    /// Pointer to a memory location (TODO: Stack only?)  
    /// `&T` or `&mut T`
    Reference {
        ty: Box<Type<'input>>,
        mutable: bool,
    },

    // TODO: Does it makes sense to allow raw pointers in an embedded language?
    //       Seems like grounds for safety (security) issues
    // *const T or *mut T
    // Pointer {
    //     ty: Box<Type<'input>>,
    //     mutable: bool,
    // },

    /// 8-bit unsigned integer
    u8,
    /// 16-bit unsigned integer
    u16,
    /// 32-bit unsigned integer
    u32,
    /// 64-bit unsigned integer
    u64,
    /// 128-bit unsigned integer
    u128,

    /// 8-bit signed integer
    i8,
    /// 16-bit signed integer
    i16,
    /// 32-bit signed integer
    i32,
    /// 64-bit signed integer
    i64,
    /// 128-bit signed integer
    i128,

    /// Architectural word size (unsigned)
    usize,
    /// Architectural word size (signed)
    isize,

    /// IEEE 754 32 bit float as used by C, Rust, and Cranelift
    f32,
    /// IEEE 754 64 bit float as used by C, Rust, and Cranelift
    f64,

    /// 8 bit boolean value:  
    /// - `false` defaults to b00000000
    /// - `true` defaults to  b11111111
    /// - **Anything non-zero is considered `true`** for the purposes of codegen
    ///    - Note, however, that as a user, you cannot create such booleans
    bool,

    /// `()` type
    Unit,

    /// (A, B, C, ...)
    Tuple(Vec<Type<'input>>),

    // [type; length]
    // Array {
    //     ty: Box<Type<'input>>,
    //     length: usize,
    // },
    
    /// Name of a struct, enum, alias, etc.
    User(&'input str),

    /// Unspecified and uninferred type
    Unknown,
}

impl std::fmt::Display for Type<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Type::Reference { ty, mutable } => {
                let mut_str = if *mutable {"mut "} else {""};
                format!("&{}{}", mut_str, ty)
            },
            // TODO: Might not want raw pointers at all
            // Type::Pointer { ty, mutable } => format!("TODO:"),
            Type::u8 => "u8".to_owned(),
            Type::u16 => "u16".to_owned(),
            Type::u32 => "u32".to_owned(),
            Type::u64 => "u64".to_owned(),
            Type::u128 => "u128".to_owned(),
            Type::i8 => "i8".to_owned(),
            Type::i16 => "i16".to_owned(),
            Type::i32 => "i32".to_owned(),
            Type::i64 => "i64".to_owned(),
            Type::i128 => "i128".to_owned(),
            Type::usize => "usize".to_owned(),
            Type::isize => "isize".to_owned(),
            Type::f32 => "f32".to_owned(),
            Type::f64 => "f64".to_owned(),
            Type::bool => "bool".to_owned(),
            Type::Unit => "()".to_owned(),
            Type::Tuple(types) => {
                let mut string = String::from("(");

                for t in types {
                    string.push_str(&format!("{}, ", t));
                }
                // Remove trailing ", "
                string.pop();
                string.pop();
                // Close the first parenthesis
                string.push(')');

                string
            },
            Type::User(t) => String::from(*t),
            Type::Unknown => "!Unknown!".to_owned(),
        };

        write!(f, "{}", string)
    }
}

impl<'input> Type<'input> {
    /// Resolves a type (as text) obtained from lexer/parser to an internal type
    pub fn resolve_builtin(type_str: &str) -> Type {
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

            "usize" => Type::usize,
            "isize" => Type::isize,

            "f32" => Type::f32,
            "f64" => Type::f64,

            "bool" => Type::bool,
            // Tuples, arrays, etc. are handled by `parse::parse_type`
            _ => Type::User(type_str),
        }
    }

    pub fn ir_type(&self, pointer_type: &cranelift_types::Type) -> cranelift_types::Type {
        // NOTE: `I` is for `integer` -> sign is not regarded
        match self {
            // These are all `size` regardless of whether unsigned, reference, or pointer
            Type::usize
            | Type::isize
            | Type::Reference { .. } 
            /*| Type::Pointer { .. } */ => *pointer_type,

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

            Type::bool => cranelift_types::B8,

            // TODO: What to do about these?
            Type::Unit => cranelift_types::INVALID,
            Type::Tuple(_) => cranelift_types::INVALID,
            Type::User(_) => cranelift_types::INVALID,

            Type::Unknown => cranelift_types::INVALID,
        }
    }

    pub fn is_unknown(&self) -> bool {
        self == &Type::Unknown
    }

    /// Used to determine whether explicit stack allocation is needed for the type
    pub fn is_builtin(&self) -> bool {
        match self {
            Type::User(_)
            | Type::Tuple(_) => false,

            _ => true,
        }
    }

    pub fn is_signed_integer(&self) -> bool {
        match self {
            Type::i8 
            | Type::i16 
            | Type::i32 
            | Type::i64 
            | Type::i128 
            | Type::isize => true,

            _ => false,
        }
    }

    pub fn is_integer(&self) -> bool {
        match self {
            Type::u8
            | Type::u16
            | Type::u32
            | Type::u64
            | Type::u128
            | Type::i8
            | Type::i16
            | Type::i32
            | Type::i64
            | Type::i128
            | Type::usize
            | Type::isize => true,

            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        match self {
            Type::f32
            | Type::f64 => true,
            
            _ => false,
        }
    }

    pub fn is_numeric(&self) -> bool {
        match self {
            Type::u8
            | Type::u16
            | Type::u32
            | Type::u64
            | Type::u128
            | Type::i8
            | Type::i16
            | Type::i32
            | Type::i64
            | Type::i128
            | Type::usize
            | Type::isize
            | Type::f32
            | Type::f64 => true,
            
            _ => false,
        }
    }

    pub fn is_reference(&self) -> bool {
        if let Type::Reference {..} = self { true } else { false }
    }

    // This is useful for determining whether an assignment is valid 
    // (variable doesn't need to be mutable if reference is mutable)
    pub fn is_mutable_reference(&self) -> bool {
        if let Type::Reference { mutable: true, ..} = self { true } else { false }
    }
}