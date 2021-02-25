use std::collections::{HashMap, HashSet};

use crate::frontend::parse::ast;

use super::types::Type;
use super::*;

///////////////////// Main Validation Functionality /////////////////////

pub struct Context<'input> {
    /// Variable allocation data
    pub allocations: AllocationTable<'input>,
    operators: Vec<(Vec<crate::frontend::lex::Token>, String)>,
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
    /// Used to simplify table lookups
    current_function_name: &'input str,
}

impl<'input> Context<'input> {
    /// Creates an empty validation context
    pub fn new() -> Self {
        Self {
            allocations: AllocationTable::new(),
            operators: Vec::new(),
            functions: FunctionTable::new(),
            structs: HashMap::new(),
            types: TypeTable::new(),
            scopes: Scopes::new(),
            // Does not allocate any heap memory
            ast: ast::AST::placeholder(),

            last_return_type: Type::Unknown,
            current_function_name: "",
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
            // TODO: Want to declare types, then validate them.
            //       That way, structs can reference other types declared later on
            self.register_struct(&struct_)?;
        }
        for extern_block in &ast.externs {
            for prototype in &extern_block.item {
                self.validate_function_prototype(&prototype)?;
                self.functions.forward_declare_function(&prototype, true)?;
            }
        }
        for operator in &ast.operators {
            self.operators.push((operator.pattern.clone(), operator.associated_function.clone()));
        }
        for function in &ast.functions {
            self.validate_function_prototype(&function.prototype)?;
            self.functions.forward_declare_function(&function.prototype, false)?;
        }
        // for constant in &ast.constants {
            // TODO: Declare their typed idents in global scope
        // }
        // for trait_ in &ast.traits {
            // TODO: Build table of traits
        // }
        // for impl_ in &ast.impls {
            // TODO: Register trait implementations
        // }
        
        for function in &mut ast.functions {
            self.current_function_name = function.prototype.name;
            self.validate_function_body(function)?;
        }

        self.ast = ast;

        Ok(())
    }

    /// Registers and lays out a "repr(C)" struct
    pub fn register_struct(&mut self, struct_: &ast::Struct<'input>) -> Result<(), String> {
        let needed_padding = |offset: i32, alignment: i32| {
            let misalignment = offset % alignment;
            if misalignment > 0 {
                alignment - misalignment
            } else {
                0
            }
        };
        
        // Determine the struct's overall alignment
        let alignment = struct_.fields.iter().fold(0, |alignment, x| {
            std::cmp::max(alignment, self.types.alignment_of(&x.ty))
        });

        let mut fields = HashMap::new();
        
        let mut offset = 0_i32;
        // Determine each field's aligned offset
        for field in &struct_.fields.item {
            // Account for any needed padding
            let field_alignment = self.types.alignment_of(&field.ty);
            // FIXME: Narrowing cast
            offset += needed_padding(offset, field_alignment as i32);
            
            // Place field at current offset
            fields.insert(field.name, StructField {
                ty: field.ty.clone(),
                offset,
                is_public: field.is_public,
            });
            
            // Account for the size of the field
            // FIXME: Narrowing cast
            offset += self.types.size_of(&field.ty) as i32;
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
        // FIXME: Narrowing cast
        let size = offset + needed_padding(offset, alignment as i32);
        self.types.insert(&Type::User(struct_.name), TypeTableEntry::new(size as usize, alignment))?;

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
                            .map(|field| field.ty.clone())
                    })
                    .transpose()
                    .unwrap_or(Err(format!("Type `{}` has no field `{}`", ty, field)))
            }

            _ => Err(format!("Type `{}` cannot have any fields (tried accessing field `{}`)", ty, field)),
        }
    }

    /// Returns the byte offset of a field for the given type.  
    /// Note that the type **must be the base type**. References return errors.
    pub fn get_field_offset(&self, ty: &Type<'input>, field: &'input str) -> Result<i32, String> {
        match ty {
            Type::Reference { .. } => Err(format!("Field offsets cannot be obtained from references")),

            Type::Tuple(types) => todo!(),

            Type::User(ident) => {
                // TODO: Errors?
                Ok(self.structs.get(ident).unwrap().fields.get(field).unwrap().offset)
            }

            _ => Err(format!("Tried getting field offset of incompatible type `{}`", ty)),
        }
    }

    pub fn is_field_public(&self, ty: &Type<'input>, field: &'input str) -> Result<bool, String> {        
        match ty {
            // Recursively strip away type wrappers
            Type::Reference { ty: underlying, .. } => {
                self.is_field_public(underlying, field)
            }

            Type::Tuple(types) => todo!(),
            
            Type::User(name) => {
                let struct_ = self.structs.get(name).ok_or(
                    format!("No such type: `{}`", name)
                )?;

                let field = struct_.fields.get(field)
                    .ok_or(format!("Type `{}` has no field `{}`", ty, field))?;

                Ok(field.is_public)
            }

            // TODO: Is this correct?
            _ => unreachable!(), //Err(format!("Type `{}` has no field `{}`", actual_type, field))
        }
    }

    pub fn validate_function_prototype(&self, prototype: &ast::FunctionPrototype) -> Result<(), String> {
        self.types.assert_valid(&prototype.return_type)?;

        for param in &prototype.parameters.item {
            self.types.assert_valid(&param.ty)?;
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
            // NOTE: Function parameters are passed in -> no allocation information needed
            self.scopes.add_var_to_scope(param.name, param.mutable, param.ty.clone(), MemoryUsage::FunctionParam)?;
        }
        
        // Validate the function body
        let _implicit_return_type = self.validate_block(&mut function.body, true)?;

        for (name, data) in self.scopes.pop_scope().variables {
            self.allocations.insert(self.current_function_name, name, data.memory_usage)?;
        }

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
                    let expr_type = self.validate_expression(expression)?;

                    if is_function_body {
                        *is_function_return = true;

                        if let Some(ident) = Self::reduce_expression_to_alias(expression) {
                            self.scopes.signal_return_variable(ident);
                        }
                    }

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
        // NOTE: Don't need to check last return type because function body
        //       can only ever have a single implicit return (the first one found)
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
                // All variables use stack slots by default
                let mut memory_usage = MemoryUsage::StackSlot;
                
                // Variable is declared and assigned
                if let Some(expr) = value {
                    let assigned_type = self.validate_expression(expr)?;

                    // If this assignment simply aliases another variable,
                    // signal that no allocations are needed, as this will use that variable's
                    if let Some(alias) = Self::reduce_expression_to_alias(expr) {
                        memory_usage = MemoryUsage::Alias(alias);
                    }

                    if ty.is_unknown() {
                        // Immediate type inferrence
                        *ty = assigned_type;
                    } else if ty != &assigned_type {
                        // Eplicit type must be equivalent to the expression's type
                        return Err(format!("Variable `{}` has type `{}`, but is assigned the type `{}`", ident, ty, assigned_type));
                    }
                // Variable is declared, not assigned
                } else {
                    // Type is not known and cannot be determined at the moment
                    if ty.is_unknown() {
                        // TODO: Account for unknown types
                        todo!("Mark variable for being inferred later on");
                    }
                }

                self.scopes.add_var_to_scope(ident, *mutable, ty.clone(), memory_usage)?;
            }

            // TODO: aliasing/reducing
            ast::Statement::Assign { lhs, operator, expression } => {
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

                    // TODO: Do this without clones
                    expression.item = ast::Expression::BinaryExpression {
                        lhs: Box::new(lhs.clone()),
                        op: ast::Node::new(new_op, operator.span),
                        rhs: Box::new(expression.clone()),
                        ty: Type::Unknown,
                    };
                }

                let destination_type = self.validate_expression(lhs)?;
                let assigned_type = self.validate_expression(expression)?;

                match &lhs.item {
                    ast::Expression::FieldAccess { base_expr, field, ty } => {
                        // TODO: Assert that the root of base_expr is mutable
                    }

                    ast::Expression::Ident { name, ty } => {
                        let var_data = self.scopes.get_variable_mut(name)?;
                        if !var_data.mutable {
                            return Err(format!("Cannot assign to immutable variable `{}`", name));
                        }
                        if let Some(ident) = Self::reduce_expression_to_alias(expression) {
                            var_data.memory_usage = MemoryUsage::Alias(ident);
                        }
                    }

                    ast::Expression::FunctionCall { name, inputs, ty } => {
                        todo!("assign to function calls if `&mut` returned?");
                    }
                    _ => unreachable!(),
                }
                
                if destination_type != assigned_type {
                    return Err(format!("Tried assigning type `{}` to incompatible type `{}`", &destination_type, &assigned_type));
                }
            }

            ast::Statement::Return { expression } => {
                // Note the type
                let return_type = self.validate_expression(expression)?;

                if self.last_return_type.is_unknown() {
                    self.last_return_type = return_type;
                } else if self.last_return_type != return_type {
                    return Err(format!("Found differing return types: `{}` and `{}`", &return_type, &self.last_return_type));
                }

                // If a stack-allocated variable is being returned,
                // signal that the variable must use the struct-return slot
                if let Some(ident) = Self::reduce_expression_to_alias(expression) {
                    self.scopes.signal_return_variable(ident);
                }
            }

            ast::Statement::ImplicitReturn { expression, .. } => {
                self.validate_expression(expression)?;
            }

            ast::Statement::Expression(expr) => {
                self.validate_expression(expr)?;
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

                match &op.item {
                    ast::BinaryOp::Custom(custom) => {
                        let mut new_expr = None;
                        for pattern in &self.operators {
                            if *custom == pattern.0 {
                                new_expr = Some(ast::Expression::FunctionCall {
                                    name: pattern.1.clone(),
                                    inputs: vec![
                                        *lhs.clone(),
                                        *rhs.clone(),
                                    ],
                                    ty: self.functions.get_unchecked_function_definition(&pattern.1)?.return_type.clone(),
                                })
                            }
                        }

                        if let Some(expr) = new_expr {
                            *expression = expr;
                            self.validate_expression(expression)
                        } else {
                            Err(format!("Binary operator `{:?}` is not defined", custom))
                        }
                    }

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
                
                match &op.item {
                    // TODO: this
                    ast::UnaryOp::Custom(custom) => {
                        let mut new_expr = None;
                        for pattern in &self.operators {
                            if *custom == pattern.0 {
                                new_expr = Some(ast::Expression::FunctionCall {
                                    name: pattern.1.clone(),
                                    inputs: vec![
                                        *expr.clone(),
                                    ],
                                    ty: self.functions.get_unchecked_function_definition(&pattern.1)?.return_type.clone(),
                                })
                            }
                        }

                        if let Some(expr) = new_expr {
                            *expression = expr;
                            self.validate_expression(expression)
                        } else {
                            Err(format!("Unary operator `{:?}` is not defined", custom))
                        }
                    }

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
            ast::Expression::FieldConstructor { ty, fields } => {
                self.types.assert_valid(ty)?;

                // FIXME: To maintain correct field ordering during error printing,
                //        a Vec can be used instead (at the cost of speed)
                let mut required_fields = HashSet::new();
                
                // FIXME: A few hacks to avoid immutable + mutable borrow
                {
                    let struct_definition = self.structs.get(ty.to_string().as_str())
                        .ok_or(format!("No type `{}` compatible with field constructor", ty))?;

                    // Note the required fields
                    for field in struct_definition.fields.keys() {
                        required_fields.insert(*field);
                    }
                }

                // Check each assigned field/value with the expected fields/values
                for (field_name, expr) in fields {
                    // FIXME: Another (not terrible) hack to satisfy borrows
                    let field_type = self.structs.get(ty.to_string().as_str()).unwrap().fields.get(field_name.as_str())
                        .ok_or(format!("Type `{}` has no field `{}`", ty, field_name))?
                        .ty.clone();
                    
                    // Required field is accounted for
                    required_fields.remove(field_name.as_str());

                    let assigned_type = self.validate_expression(expr)?;
                    if assigned_type != field_type {
                        return Err(format!("Field `{}.{}` is of type `{}`, but found type `{}`", ty, field_name, field_type, assigned_type));
                    }
                }

                // Error if any fields are missing
                if required_fields.len() > 0 {
                    // FIXME: Can't use newlines here?
                    let mut error = format!("Constructor for type `{}` is missing fields: ", ty);
                    for missing in required_fields {
                        error.push_str(&format!("`{}`, ", missing));
                    }
                    // Remove trailing ", "
                    error.pop();
                    error.pop();
                    return Err(error);
                }

                Ok(ty.clone())
            }

            // TODO: This needs to be modified later to also support enums and tuples
            ast::Expression::FieldAccess { base_expr, field, ty } => {
                let base_type = self.validate_expression(base_expr)?;
                
                if !self.is_field_public(&base_type, field)? {
                    return Err(format!("Field `{}` of `{}` is private", field, base_type));
                }
                
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
                
                for (name, data) in self.scopes.pop_scope().variables {
                    self.allocations.insert(self.current_function_name, name, data.memory_usage)?;
                }


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

    /// If an expression reduces to an alias, return the alias.  
    /// Returns `None` if the expression does not alias any variables.
    // TODO: Re-enable this once used
    fn reduce_expression_to_alias(validated_expression: &ast::Expression<'input>) -> Option<&'static str> {
        // if let ast::Expression::Ident { name, .. } = validated_expression {
        //     return Some(name);
        // }

        None
    }
}