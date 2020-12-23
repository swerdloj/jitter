use std::{collections::HashMap, todo};

use crate::frontend::parse::ast;

use super::types::Type;

///////////////////// Helper Types /////////////////////

/// Stores struct definitions
struct StructDefinition<'input> {
    /// Map of field_name -> (type, byte offset)
    fields: HashMap<&'input str, (Type<'input>, usize)>,
}

/// Stores type sizes and alignments
struct TypeTable<'input> {
    /// Map of field_name -> (size, alignment) in bytes
    data: HashMap<Type<'input>, TypeTableEntry>
}

impl<'input> TypeTable<'input> {
    // TODO: Accept word size here and adjust table accordingly
    fn new() -> Self {
        let mut data = HashMap::new();

        // FIXME: This could be looked up via `match`, but this is more consistent
        // FIXME: Only 64-bit architectures are supported by the below values
        
        data.insert(Type::u8,   TypeTableEntry::new(1, 1));
        data.insert(Type::u16,  TypeTableEntry::new(2, 2));
        data.insert(Type::u32,  TypeTableEntry::new(4, 4));
        data.insert(Type::u64,  TypeTableEntry::new(8, 8));
        data.insert(Type::u128, TypeTableEntry::new(16, 8));

        data.insert(Type::i8,   TypeTableEntry::new(1, 1));
        data.insert(Type::i16,  TypeTableEntry::new(2, 2));
        data.insert(Type::i32,  TypeTableEntry::new(4, 4));
        data.insert(Type::i64,  TypeTableEntry::new(8, 8));
        data.insert(Type::i128, TypeTableEntry::new(16, 8));

        data.insert(Type::f32,  TypeTableEntry::new(4, 4));
        data.insert(Type::f64,  TypeTableEntry::new(8, 8));

        data.insert(Type::bool, TypeTableEntry::new(1, 1));

        data.insert(Type::Unit, TypeTableEntry::new(0, 1));

        Self { data }
    }

    fn insert(&mut self, t: &Type<'input>, entry: TypeTableEntry) -> Result<(), String> {
        match self.data.insert(t.clone(), entry) {
            Some(_) => Err(format!("Type {:?} already exists", t.clone())),
            None => Ok(()),
        }
    }

    fn assert_valid(&mut self, t: &Type<'input>) -> Result<(), String> {
        if self.data.contains_key(t) {
            Ok(())
        } else {
            Err(format!("Type `{:?}` is not valid", t))
        }
    }

    /// Returns alignment of the type in bytes
    fn alignment_of(&self, t: &Type) -> usize {
        self.data.get(t).unwrap().alignment
    }

    /// Returns the size of the type in bytes
    fn size_of(&self, t: &Type) -> usize {
        self.data.get(t).unwrap().size
    }
}

struct TypeTableEntry {
    size: usize,
    alignment: usize,
}

impl TypeTableEntry {
    fn new(size: usize, alignment: usize) -> Self {
        Self { size, alignment }
    }
}

struct FunctionDefinition<'input> {
    // (name, type, mutable)
    parameters: Vec<(&'input str, Type<'input>, bool)>,
    return_type: Type<'input>,
}

struct Scopes<'input> {
    /// Each element represents a nested scope
    scopes: Vec<HashMap<&'input str, VariableData<'input>>>,
    num_scopes: usize,
}

impl<'input> Scopes<'input> {
    fn new() -> Self {
        Self {
            scopes: Vec::new(),
            num_scopes: 0,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.num_scopes += 1;
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
        self.num_scopes -= 1;
    }

    fn add_var_to_scope(&mut self, name: &'input str, mutable: bool, ty: Type<'input>) -> Result<(), String> {
        if let Some(_old) = self.scopes[self.num_scopes-1].insert(name, VariableData::new(mutable, ty)) {
            Err(format!("Variable `{}` is already defined in this scope", name))
        } else {
            Ok(())
        }
    }

    fn get_variable(&mut self, name: &str) -> Result<&VariableData<'input>, String> {
        for scope in &self.scopes {
            if let Some(var) = scope.get(name) {
                return Ok(var);
            }
        }

        Err(format!("No variable `{}` in scope", name))
    }
}

struct VariableData<'input> {
    /// Is the variable mutable
    pub mutable: bool,
    // Is it local or global
    // local: bool,
    /// Type of the variable
    pub ty: Type<'input>,
}

impl<'input> VariableData<'input> {
    fn new(mutable: bool, ty: Type<'input>) -> Self {
        Self { mutable, ty }
    }
}


///////////////////// Main Functionality /////////////////////

pub struct Context<'input> {
    /// Function signatures
    functions: HashMap<&'input str, FunctionDefinition<'input>>,
    /// Struct signatures
    structs: HashMap<&'input str, StructDefinition<'input>>,
    /// Type information
    types: TypeTable<'input>,
    /// Scoped variable information
    scopes: Scopes<'input>,

    /// The validated AST
    pub ast: ast::AST<'input>,
}

impl<'a> Context<'a> {
    /// Creates an empty validation context
    pub fn new(/*ast: ast::AST<'a>*/) -> Self {
        Self {
            functions: HashMap::new(),
            structs: HashMap::new(),
            types: TypeTable::new(),
            scopes: Scopes::new(),
            // Does not allocate any heap memory
            ast: ast::AST::with_capacity(0),
        }
    }

    /// Validates and takes ownership of an AST
    pub fn validate(&mut self, mut ast: ast::AST<'a>) -> Result<(), String> {
        for node in &mut ast {
            match node {
                ast::TopLevel::Function(function) => {
                    self.register_function(&mut function.item)?;
                }
                ast::TopLevel::Struct(struct_) => {
                    self.register_struct(&struct_.item)?;
                }
                ast::TopLevel::ConstDeclaration => {
                    todo!()
                }
                ast::TopLevel::UseStatement => {
                    todo!()
                }
            }
        }

        self.ast = ast;

        Ok(())
    }

    /// Registers and lays out a "repr(C)" struct
    pub fn register_struct(&mut self, struct_: &ast::Struct<'a>) -> Result<(), String> {
        let needed_padding = |offset, alignment| {
            let misalignment = offset % alignment;
            if misalignment > 0 {
                alignment - misalignment
            } else {
                0
            }
        };
        
        // Determine the struct's overall alignment
        let alignment = struct_.fields.item.iter().fold(0, |alignment, x| {
            std::cmp::max(alignment, self.types.alignment_of(&x.item.field_type))
        });

        let mut fields = HashMap::new();
        
        let mut offset = 0;
        // Determine each field's aligned offset
        for field in &struct_.fields.item {
            // Account for any needed padding
            let field_alignment = self.types.alignment_of(&field.item.field_type);
            offset += needed_padding(offset, field_alignment);
            
            // Place field at current offset
            fields.insert(field.item.field_name, (field.item.field_type.clone(), offset));
            
            // Account for the size of the field
            offset += self.types.size_of(&field.item.field_type);
        }
        
        self.structs.insert(
            struct_.name,
            StructDefinition {
                fields,
            }
        ).map(|_already_existing| {
            return Err::<(), String>(format!("Struct `{}` is already defined", struct_.name));
        });

        // Add final padding for the struct's alignment
        let size = offset + needed_padding(offset, alignment);
        self.types.insert(&Type::User(struct_.name), TypeTableEntry::new(size, alignment))?;

        Ok(())
    }

    /// Register a function signature, then validate its contents
    pub fn register_function(&mut self, function: &mut ast::Function<'a>) -> Result<(), String> {
        // Registers a function's name and assigns internal types
        self.functions.insert(
            function.name,
            FunctionDefinition {
                parameters: function.parameters.item.iter().map(|node| {
                        let field_name = node.item.field_name;
                        (field_name, node.item.field_type.clone(), node.item.mutable)
                    }).collect(),
                return_type: function.return_type.clone()
            }
        ).map(|_already_existing| {
            return Err::<(), String>(format!("Function `{}` is already defined", function.name));
        });

        // Create a new scope containing function inputs
        self.scopes.push_scope();
        for param in &function.parameters.item {
            self.types.assert_valid(&param.item.field_type)?;
            self.scopes.add_var_to_scope(param.item.field_name, param.item.mutable, param.item.field_type.clone())?;
        }

        for statement in &mut function.statements.item {
            self.validate_statement(&mut statement.item)?;
        }
        self.scopes.pop_scope();


        Ok(())
    }

    /// Validates a statement & assigns types
    pub fn validate_statement(&mut self, statement: &mut ast::Statement<'a>) -> Result<(), String> {
        match statement {
            // Ensures the variable is not already in scope and has valid types
            ast::Statement::Let { ident, mutable, ty, value } => {
                // TODO: Account for variable scopes

                // Variable is declared and assigned
                if let Some(expr) = value {
                    let expr_type = self.validate_expression(&mut expr.item)?;

                    // Type must be equivalent to the expression type
                    if ty.is_unknown() {
                        *ty = expr_type;
                    } else if ty != &expr_type {
                        return Err(format!("Variable `{}` has type `{:?}`, but is assigned the type `{:?}`", ident, ty, expr_type));
                    }
                // Variable is declared, not assigned
                } else {
                    // Type is not known and cannot be determined at the moment
                    if ty.is_unknown() {
                        // TODO:
                        todo!("Mark variable for being inferred later on");
                    }
                }

                // Type is determined at this point
                // TODO: Account for marked variables (see `todo`)
                self.types.assert_valid(ty)?;
                self.scopes.add_var_to_scope(ident, *mutable, ty.clone())?;
            }

            ast::Statement::Assign { variable, operator, expression } => {
                // todo!()
            }

            ast::Statement::Return { expression } => {
                // todo!()
            }

            ast::Statement::Expression(expr) => {
                self.validate_expression(&mut expr.item)?;
            }
        }

        Ok(())
    }

    /// Validates an expression, determining its type. Returns the type of the expression.
    pub fn validate_expression(&mut self, expression: &mut ast::Expression<'a>) -> Result<Type<'a>, String> {
        match expression {
            ast::Expression::BinaryExpression { lhs, op, rhs, ty } => {
                todo!()
            }

            ast::Expression::UnaryExpression { op, expr, ty } => {
                let expr_type = self.validate_expression(&mut expr.item)?;
                // TODO: this
                // todo!()

                self.types.assert_valid(&expr_type)?;
                // TEMP:
                Ok(expr_type)
            }

            // Recursively determine the type of the expression (and thus all nested expressions)
            ast::Expression::Parenthesized { expr, ty } => {
                *ty = self.validate_expression(&mut expr.item)?;
                Ok(ty.clone())
            }

            ast::Expression::Literal(literal) => {
                // TODO: this
                todo!()
            }

            // Returns the type of the variable
            ast::Expression::Ident(ident) => {
                self.scopes.get_variable(ident).map(|var| var.ty.clone())
            }
        }
    }
}