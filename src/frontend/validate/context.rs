use std::collections::{HashMap, HashSet};

use crate::frontend::parse::ast;

use super::types::Type;

///////////////////// Helper Types /////////////////////

/// Stores struct definitions
struct StructDefinition<'input> {
    /// Map of field_name -> (type, byte offset)
    fields: HashMap<&'input str, (Type<'input>, usize)>,
}

pub struct TypeTableEntry {
    /// Size of type in bytes
    pub size: usize,
    /// Alignment of type in bytes
    alignment: usize,
    // TODO: Store fields and their offsets here too
    // field_offets: HashMap<?>,
}

impl TypeTableEntry {
    fn new(size: usize, alignment: usize) -> Self {
        Self { size, alignment }
    }
}

/// Stores type sizes and alignments
pub struct TypeTable<'input> {
    /// Map of field_name -> (size, alignment) in bytes
    data: HashMap<Type<'input>, TypeTableEntry>
}

impl<'input> TypeTable<'input> {
    // TODO: Accept word size here and adjust table accordingly
    // TODO: Support `isize` and `usize`
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
            Some(_) => Err(format!("Type {} already exists", t.clone())),
            None => Ok(()),
        }
    }

    fn assert_valid(&self, t: &Type<'input>) -> Result<(), String> {
        match t {
            // Strip away references to check the underlying type
            Type::Reference { ty, .. } => Ok(self.assert_valid(ty)?),

            // Check all contained types
            Type::Tuple(types) => {
                // TODO: All types can be checked (rather than stopping at first error)
                //       Just store all errors, then build an error string
                for ty in types {
                    let result = self.assert_valid(ty);
                    if result.is_err() {
                        return result;
                    }
                }
                Ok(())
            }

            // Base types
            _ => {
                if self.data.contains_key(t) {
                    Ok(())
                } else {
                    Err(format!("Type `{}` is not valid", t))
                }
            }
        }
    }

    /// Returns alignment of the type in bytes
    fn alignment_of(&self, t: &Type) -> usize {
        match t {
            // TODO: Alignment should be same as pointer type
            Type::Reference { ty, .. } => todo!("need pointer type stuff"),
            
            // TODO: Tuples should align same as structs
            Type::Tuple(types) => todo!("tuple alignment"),

            _ => self.data.get(t).expect("alignment_of").alignment,
        }
    }

    /// Returns the size of the type in bytes
    pub fn size_of(&self, t: &Type) -> usize {
        self.data.get(t).unwrap().size
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


pub struct FunctionDefinition<'input> {
    /// Function parameters (field_name, field_type, mutable) in order
    pub parameters: Vec<(&'input str, Type<'input>, bool)>,
    pub return_type: Type<'input>,
    pub is_extern: bool,
    pub is_validated: bool,
}

pub struct FunctionTable<'input> {
    // Map of (name -> data)
    pub functions: HashMap<&'input str, FunctionDefinition<'input>>
}

impl<'input> FunctionTable<'input> {
    fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    // FIXME: A few copies and clones, but nothing bad
    fn forward_declare_function(&mut self, validated_prototype: &ast::FunctionPrototype<'input>, is_extern: bool) -> Result<(), String> {
        if self.functions.contains_key(validated_prototype.name) {
            return Err(format!("Function `{}` already exists", validated_prototype.name));
        }

        let parameters = validated_prototype.parameters.iter().map(|param| {
            (param.field_name, param.field_type.clone(), param.mutable)
        }).collect();

        let definition = FunctionDefinition {
            parameters,
            return_type: validated_prototype.return_type.clone(),
            is_extern,
            is_validated: false,
        };

        self.functions.insert(validated_prototype.name, definition);

        Ok(())
    }

    fn __get_mut(&mut self, name: &str) -> Result<&mut FunctionDefinition<'input>, String> {
        self.functions.get_mut(name)
            .ok_or(format!("Could not find function `{}`", name))
    }

    fn __get(&self, name: &str) -> Result<&FunctionDefinition<'input>, String> {
        self.functions.get(name)
            .ok_or(format!("Could not find function `{}`", name))
    }

    // TODO: This and `get_validated_function_definition` may not ever be used
    //       (this functionality exists in finalized JIT product)
    fn mark_function_validated(&mut self, name: &str) -> Result<(), String> {
        self.__get_mut(name)?
            .is_validated = true;
        Ok(())
    }

    // TODO: Will this ever be used?
    fn get_validated_function_definition(&mut self, name: &str) -> Result<&FunctionDefinition<'input>, String> {
        let function = self.__get(name)?;

        if !function.is_validated {
            // FIXME: This should not be possible
            Err(format!("Function `{}` was not validated", name))
        } else {
            Ok(function)
        }
    }

    /// Returns a `FunctionDefinition` that is not guarenteed to have been
    /// successfully validated
    fn get_unchecked_function_definition(&mut self, name: &str) -> Result<&FunctionDefinition<'input>, String> {
        self.__get(name)
    }
}

///////////////////// Main Functionality /////////////////////

pub struct Context<'input> {
    /// Function signatures
    pub functions: FunctionTable<'input>,
    /// Struct signatures
    structs: HashMap<&'input str, StructDefinition<'input>>,
    /// Type information
    pub types: TypeTable<'input>,
    /// Scoped variable information
    scopes: Scopes<'input>,

    /// The validated AST
    pub ast: ast::AST<'input>,

    /// Used to validate function bodies using `Statement::Return`s
    last_return_type: Type<'input>,
}

impl<'input> Context<'input> {
    /// Creates an empty validation context
    pub fn new() -> Self {
        Self {
            functions: FunctionTable::new(),
            structs: HashMap::new(),
            types: TypeTable::new(),
            scopes: Scopes::new(),
            // Does not allocate any heap memory
            ast: ast::AST::placeholder(),

            last_return_type: Type::Unknown,
        }
    }

    /// Validates and takes ownership of an AST
    pub fn validate(&mut self, mut ast: ast::AST<'input>) -> Result<(), String> {
        // Registration pass (gathers contextual information)    
        // NOTE: Order matters here
        // for use_ in &ast.uses {
            // TODO: Build symbol/alias table
            //       Must be done first (to avoid collisions and to reference used items)
        // }
        for struct_ in &ast.structs {
            self.register_struct(&struct_)?;
        }
        for extern_block in &ast.externs {
            for prototype in &extern_block.item {
                self.validate_function_prototype(&prototype)?;
                self.functions.forward_declare_function(&prototype, true)?;
            }
        }
        for function in &ast.functions {
            self.validate_function_prototype(&function.prototype)?;
            self.functions.forward_declare_function(&function.prototype, false)?;
        }
        // for constant in &ast.constants {
            // TODO: Declare constant in global scope
        // }
        // for trait_ in &ast.traits {
            // TODO: Build table of traits
        // }
        // for impl_ in &ast.impls {
            // TODO: Register trait implementations
        // }
        

        // Validation pass
        // TODO:
        // for use_ in &ast.uses {
        // }
        // TODO:
        // for constant in &ast.constants {
        // }
        for function in &mut ast.functions {
            self.validate_function_body(function)?;
        }
        // TODO:
        // for trait_ in &ast.traits {
        // }
        // TODO:
        // for impl_ in &ast.impls {
        // }

        self.ast = ast;

        Ok(())
    }

    /// Registers and lays out a "repr(C)" struct
    pub fn register_struct(&mut self, struct_: &ast::Struct<'input>) -> Result<(), String> {
        let needed_padding = |offset, alignment| {
            let misalignment = offset % alignment;
            if misalignment > 0 {
                alignment - misalignment
            } else {
                0
            }
        };
        
        // Determine the struct's overall alignment
        let alignment = struct_.fields.iter().fold(0, |alignment, x| {
            std::cmp::max(alignment, self.types.alignment_of(&x.field_type))
        });

        let mut fields = HashMap::new();
        
        let mut offset = 0;
        // Determine each field's aligned offset
        for field in &struct_.fields.item {
            // Account for any needed padding
            let field_alignment = self.types.alignment_of(&field.field_type);
            offset += needed_padding(offset, field_alignment);
            
            // Place field at current offset
            fields.insert(field.field_name, (field.field_type.clone(), offset));
            
            // Account for the size of the field
            offset += self.types.size_of(&field.field_type);
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

    /// Returns the type of a field from a struct, enum, or tuple.  
    /// For referenced types, the underlying type will be used.
    pub fn get_field_type(&self, ty: &Type<'input>, field: &'input str) -> Result<Type<'input>, String> {
        match ty {
            // Peel away the references
            Type::Reference { ty: underlying_type, .. } => {
                self.get_field_type(underlying_type, field)
            }

            Type::Tuple(types) => todo!(),

            // Check whether the type exists.
            // If it does, check whether the field exists.
            // If it does, return that field's type
            Type::User(ident) => {
                self.structs.get(ident)
                    .ok_or(format!("Type `{}` does not exist", ty))
                    .map(|struct_def| {
                        struct_def.fields.get(field)
                            .map(|(field_type, _size)| field_type.clone())
                    })
                    .transpose()
                    .unwrap_or(Err(format!("Type `{}` has no field `{}`", ty, field)))
            }

            _ => Err(format!("Type `{}` cannot have any fields (tried accessing field `{}`)", ty, field)),
        }
    }

    /// Returns the byte offset of a field for the given type.  
    /// Note that the type **must be the base type**. References return errors.
    pub fn get_field_offset(&self, ty: &Type<'input>, field: &'input str) -> Result<usize, String> {
        match ty {
            Type::Reference { .. } => Err(format!("Field offsets cannot be obtained from references")),

            Type::Tuple(types) => todo!(),

            Type::User(ident) => {
                // TODO: Errors?
                Ok(self.structs.get(ident).unwrap().fields.get(field).unwrap().1)
            }

            _ => Err(format!("Tried getting field offset of incompatible type `{}`", ty)),
        }
    }

    pub fn validate_function_prototype(&self, prototype: &ast::FunctionPrototype) -> Result<(), String> {
        self.types.assert_valid(&prototype.return_type)?;

        for param in &prototype.parameters.item {
            self.types.assert_valid(&param.field_type)?;
        }

        Ok(())
    }

    // TODO: Handle `self` parameter -- needs context of `impl`
    //       `Self` type must be handled similarly
    // NOTE: The function's parameters are valid at this point
    pub fn validate_function_body(&mut self, function: &mut ast::Function<'input>) -> Result<(), String> {        
        // Create a new scope containing the function's parameters
        self.scopes.push_scope();
        for param in &function.prototype.parameters.item {
            self.scopes.add_var_to_scope(param.field_name, param.mutable, param.field_type.clone())?;
        }
        
        // Validate the function body
        let _implicit_return_type = self.validate_block(&mut function.body, true)?;
        self.scopes.pop_scope();

        if self.last_return_type == function.prototype.return_type {
            // Reset for the next function
            self.last_return_type = Type::Unknown;
            
            // The function is confirmed valid at this point
            self.functions.mark_function_validated(function.prototype.name)?;

            Ok(())
        } else {
            Err(format!("Expected function `{}` to have return type `{}` but found `{}`", &function.prototype.name, &function.prototype.return_type, &self.last_return_type))
        }
    }

    /// Validates a block expression/function body.  
    /// Returns the block's type.
    pub fn validate_block(&mut self, block: &mut ast::BlockExpression<'input>, is_function_body: bool) -> Result<Type<'input>, String> {
        let mut block_type = Type::Unknown;

        for statement in &mut block.block.item {
            match &mut statement.item {
                // ImplcitReturn is just a special expression
                ast::Statement::ImplicitReturn { expression, is_function_return } => {
                    if is_function_body {
                        *is_function_return = true;
                    }


                    let expr_type = self.validate_expression(expression)?;
                    self.types.assert_valid(&expr_type)?;

                    if block_type.is_unknown() {
                        block_type = expr_type;
                    } else if block_type != expr_type {
                        return Err(format!("Differing return types. Expected `{}` but found `{}`", &block_type, &expr_type));
                    }
                }

                _ => self.validate_statement(statement)?,
            }
        }

        // No specified type -> Unit
        if block_type.is_unknown() {
            block_type = Type::Unit;
        }

        // Implicit return is used in place of explicit return
        if is_function_body && self.last_return_type.is_unknown() {
            self.last_return_type = block_type.clone();
        }

        block.ty = block_type.clone();

        Ok(block_type)
    }

    /// Validates a statement & assigns types
    pub fn validate_statement(&mut self, statement: &mut ast::Statement<'input>) -> Result<(), String> {
        match statement {
            // Ensures the variable is not already in scope and has valid types
            ast::Statement::Let { ident, mutable, ty, value } => {
                // TODO: Account for variable scopes

                // Variable is declared and assigned
                if let Some(expr) = value {
                    let expr_type = self.validate_expression(expr)?;

                    // Type must be equivalent to the expression type
                    if ty.is_unknown() {
                        *ty = expr_type;
                    } else if ty != &expr_type {
                        return Err(format!("Variable `{}` has type `{}`, but is assigned the type `{}`", ident, ty, expr_type));
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
                // Desugar op-assignments
                if operator.item != ast::AssignmentOp::Assign {
                    let new_op = match operator.item {
                        ast::AssignmentOp::Assign => unreachable!(),
                        ast::AssignmentOp::AddAssign => ast::BinaryOp::Add,
                        ast::AssignmentOp::SubtractAssign => ast::BinaryOp::Subtract,
                        ast::AssignmentOp::MultiplyAssign => ast::BinaryOp::Multiply,
                        ast::AssignmentOp::DivideAssign => ast::BinaryOp::Divide,
                    };

                    operator.item = ast::AssignmentOp::Assign;

                    expression.item = ast::Expression::BinaryExpression {
                        lhs: Box::new(ast::Node::new(ast::Expression::Ident {
                            name: variable,
                            ty: Type::Unknown,
                        }, expression.span)),
                        op: ast::Node::new(new_op, operator.span),
                        rhs: Box::new(expression.clone()),
                        ty: Type::Unknown,
                    }
                }

                let var_data = self.scopes.get_variable(variable)?;
                if !var_data.mutable {
                    return Err(format!("Cannot assign to immutable variable `{}`", variable));
                }

                self.validate_expression(expression)?;
            }

            ast::Statement::Return { expression } => {
                // Note the type
                let return_type = self.validate_expression(expression)?;

                if self.last_return_type.is_unknown() {
                    self.last_return_type = return_type;
                } else if self.last_return_type != return_type {
                    return Err(format!("Found differing return types: `{}` and `{}`", &return_type, &self.last_return_type));
                }
            }

            ast::Statement::Expression(expr) => {
                self.validate_expression(expr)?;
            }

            
            ast::Statement::ImplicitReturn { expression, .. } => {
                self.validate_expression(expression)?;
            }
        }

        Ok(())
    }

    // TODO: Make sure `ty` is assigned wherever needed
    /// Validates an expression, determining its type. Returns the type of the expression.
    pub fn validate_expression(&mut self, expression: &mut ast::Expression<'input>) -> Result<Type<'input>, String> {
        match expression {
            ast::Expression::BinaryExpression { lhs, op, rhs, ty } => {
                let l_type = self.validate_expression(lhs)?;
                let r_type = self.validate_expression(rhs)?;
                self.types.assert_valid(&l_type)?;
                self.types.assert_valid(&r_type)?;

                match op.item {
                    ast::BinaryOp::Add => {
                        // Primitive numeric types can be multiplied together
                        if l_type.is_numeric() && (r_type == l_type) {
                            // l/r_type is arbitrary here
                            *ty = r_type;
                            Ok(l_type)
                        } else {
                            todo!("Convert `+` to `std::ops::add(LType, RType)` call")
                        }
                    }

                    ast::BinaryOp::Subtract => {
                        todo!()
                    }

                    ast::BinaryOp::Multiply => {
                        // Primitive numeric types can be multiplied together
                        if l_type.is_numeric() && (r_type == l_type) {
                            // l/r_type is arbitrary here
                            *ty = r_type;
                            Ok(l_type)
                        } else {
                            todo!("Convert `*` to `std::ops::multiply(LType, RType)` call")
                        }
                    }

                    ast::BinaryOp::Divide => {
                        todo!()
                    }
                }
            }

            ast::Expression::UnaryExpression { op, expr, ty } => {
                let expr_type = self.validate_expression(expr)?;
                self.types.assert_valid(&expr_type)?;
                
                // TODO: this
                match op.item {
                    ast::UnaryOp::Negate => {
                        if expr_type.is_signed_integer() || expr_type.is_float() {
                            *ty = expr_type.clone();
                            Ok(expr_type)
                        } else {
                            todo!("Convert `-` to `std::ops::negate(T)` call");
                        }
                    }

                    ast::UnaryOp::Not => {
                        // If not boolean, then must convert to `std::op`
                        if expr_type == Type::bool {
                            Ok(expr_type)
                        } else {
                            todo!("Convert `!` to `std::ops::not(T)` call");
                        }
                    }
                }
            }

            // Ensure that all fields are filled and that valid types are used
            ast::Expression::FieldConstructor { type_name, fields } => {
                let target_type = Type::User(type_name);
                self.types.assert_valid(&target_type)?;

                // FIXME: To maintain correct field ordering during error printing,
                //        a Vec can be used instead (at the cost of speed)
                let mut required_fields = HashSet::new();
                
                // FIXME: A few hacks to avoid immutable + mutable borrow
                {
                    let struct_definition = self.structs.get(type_name)
                        .ok_or(format!("No type `{}` compatible with field constructor", type_name))?;

                    // Note the required fields
                    for field in struct_definition.fields.keys() {
                        required_fields.insert(*field);
                    }
                }

                // Check each assigned field/value with the expected fields/values
                for (field_name, expr) in fields {
                    // FIXME: Another (not terrible) hack to satisfy borrows
                    let (field_type, _) = self.structs.get(type_name).unwrap().fields.get(field_name)
                        .ok_or(format!("Type `{}` has no field `{}`", type_name, field_name))?.clone();
                    
                    // Required field is accounted for
                    required_fields.remove(field_name);

                    let assigned_type = self.validate_expression(expr)?;
                    if assigned_type != field_type {
                        return Err(format!("Field `{}.{}` is of type `{}`, but found type `{}`", type_name, field_name, field_type, assigned_type));
                    }
                }

                // Error if any fields are missing
                if required_fields.len() > 0 {
                    // FIXME: Can't use newlines here?
                    let mut error = format!("Constructor for type `{}` is missing fields: ", type_name);
                    for missing in required_fields {
                        error.push_str(&format!("`{}`, ", missing));
                    }
                    // Remove trailing ", "
                    error.pop();
                    error.pop();
                    return Err(error);
                }

                Ok(target_type)
            }

            // TODO: This needs to be modified later to also support enums and tuples
            ast::Expression::FieldAccess { base_expr, field, ty } => {
                let base_type = self.validate_expression(base_expr)?;
                
                let field_type = self.get_field_type(&base_type, field)?;
                *ty = field_type.clone();

                Ok(field_type)
            }

            ast::Expression::FunctionCall { name, inputs, ty } => {
                // Avoids requiring iter_mut() with zip()
                // Avoids mutable + immutable borrow of self
                let mut input_types = Vec::new();
                for input_expr in inputs.iter_mut() {
                    input_types.push(
                        self.validate_expression(input_expr)?
                    );
                }

                let definition = self.functions.get_unchecked_function_definition(name)?;

                if definition.parameters.len() != inputs.len() {
                    return Err(format!("Function `{}` accepts {} parameters, but {} were passed", name, definition.parameters.len(), inputs.len()));
                }

                // Note that the evaluation order here is the same as the input order
                for (i, (given_type, (param_name, param_type, _mutable))) in input_types.iter().zip(definition.parameters.iter()).enumerate() {
                    if given_type != param_type {
                        return Err(format!("Parameter #{} (`{}`) of call to `{}` has type `{}`, but found type `{}`", i, param_name, name, param_type, given_type));
                    }
                }

                *ty = definition.return_type.clone();

                Ok(definition.return_type.clone())
            }

            ast::Expression::Block(block) => {
                self.scopes.push_scope();
                let expr_type = self.validate_block(block, false)?;
                self.scopes.pop_scope();

                Ok(expr_type)
            }

            ast::Expression::Literal { value, ty } => {
                // TODO: Is this correct? 
                //       What about when the type isn't known?
                Ok(ty.clone())
            }

            // Returns the type of the variable
            ast::Expression::Ident { name, ty } => {
                let ident_type = self.scopes.get_variable(name).map(|var| var.ty.clone())?;
                *ty = ident_type.clone();

                Ok(ident_type)
            }
        }
    }

    // TODO: Could this be moved to an `impl ast::Expression` function?
    /// Returns the type of a **validated** expression
    pub fn get_expression_type(&self, expression: &ast::Expression<'input>) -> Result<Type<'input>, String> {
        let expr_type = match expression {
            ast::Expression::BinaryExpression { ty, .. } => ty.clone(),
            ast::Expression::UnaryExpression { ty, .. } => ty.clone(), 
            ast::Expression::FieldConstructor { type_name, .. } => Type::User(type_name),
            ast::Expression::FieldAccess { ty, .. } => ty.clone(),
            ast::Expression::FunctionCall { ty, .. } => ty.clone(),
            ast::Expression::Block(block) => block.ty.clone(),
            ast::Expression::Literal { ty, .. } => ty.clone(),
            ast::Expression::Ident { ty, .. } => ty.clone(),
        };

        if expr_type.is_unknown() {
            Err(format!("Cannot get expression type of non-validated expression"))
        } else {
            Ok(expr_type)
        }
    }
}