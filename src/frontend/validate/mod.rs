pub mod context;
pub mod types;

///////////////////// Validation Helpers /////////////////////

use std::collections::HashMap;
use crate::frontend::validate::types::Type;
use crate::frontend::parse::ast;

///////////////////// TYPES /////////////////////

// NOTE: Offsets are i32 for Cranelift

/// Stores struct definitions
struct StructDefinition {
    /// Map of field_name -> (type, byte offset)
    fields: HashMap<String, StructField>,
}

pub struct StructField {
    pub ty: Type,
    pub offset: i32,
    pub is_public: bool,
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
pub struct TypeTable {
    /// Map of field_name -> (size, alignment) in bytes
    data: HashMap<Type, TypeTableEntry>
}

impl TypeTable {
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

    fn insert(&mut self, t: &Type, entry: TypeTableEntry) -> Result<(), String> {
        match self.data.insert(t.clone(), entry) {
            Some(_) => Err(format!("Type {} already exists", t.clone())),
            None => Ok(()),
        }
    }

    fn assert_valid(&self, t: &Type) -> Result<(), String> {
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


///////////////////// SCOPES + VARIABLES /////////////////////

#[derive(Debug)]
pub enum MemoryUsage {
    /// The variable is new -> requires allocation  
    /// e.g.: `let x: u32 = 7;`
    StackSlot,
    
    /// The variable is a struct being returned
    /// e.g.: `return Type {...};`
    StructReturn,

    /// Aliases an existing variable -> use its allocation  
    /// e.g.: `let x: u32 = y;`
    Alias(String),

    /// The variable is allocated elsewhere before being passed as a param  
    /// e.g.: `function(12, x);`
    FunctionParam,

    // TODO: References an existing variable -> ??
    // e.g.: `let x: &u32 = &y;`
    // Borrow(&'input str),

    // TODO: Aliases a field of an existing variable -> ??
    // e.g.: `let x: u32 = y.a;`
    // FieldAlias(),
}

pub struct AllocationTable {
    // Map of ((function_name, variable name) -> variable's usage)
    pub allocations: HashMap<(String, String), MemoryUsage>,
}

impl AllocationTable {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
        }
    }

    pub fn insert(&mut self, function: String, variable: String, usage: MemoryUsage) -> Result<(), String> {
        if let Some(_existing) = self.allocations.insert((function.clone(), variable.clone()), usage) {
            return Err(format!("Variable {} is already defined in function {}", variable, function));
        }

        Ok(())
    }

    pub fn get_usage(&mut self, function: &str, variable: &str) -> &MemoryUsage {
        // NOTE: This should always be valid
        self.allocations.get(&(function.to_owned(), variable.to_owned())).expect("get_usage")
    }
}

struct VariableData {
    /// Type of the variable
    pub ty: Type,
    /// What allocation this variable needs
    pub memory_usage: MemoryUsage,
    /// Is the variable mutable
    pub mutable: bool,
}

impl VariableData {
    fn new(ty: Type, memory_usage: MemoryUsage, mutable: bool) -> Self {
        Self { ty, memory_usage, mutable }
    }
}

struct Scope {
    /// **This scope's** map of (variable name -> data)
    variables: HashMap<String, VariableData>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    fn get_var_data(&self, var: &str) -> &VariableData {
        // NOTE: This operation should always succeed
        self.variables.get(var).expect("get_var_data")
    }

    fn get_var_data_mut(&mut self, var: &str) -> &mut VariableData {
        // NOTE: This operation should always succeed
        self.variables.get_mut(var).expect("get_var_data_mut")
    }

    fn insert_var_data(&mut self, name: String, var: VariableData) {
        // NOTE: This operation should never overwrite existing
        self.variables.insert(name, var);
    }
}

/// Uses alias analysis to determine stack slot allocations and struct return slot usage
struct Scopes {
    /// Each element represents a subsequently nested scope
    scopes: Vec<Scope>,
    /// Map of (variable name -> its scope)
    all_variables: HashMap<String, usize>,
    num_scopes: usize,
}

impl Scopes {
    fn new() -> Self {
        Self {
            scopes: Vec::new(),
            all_variables: HashMap::new(),
            num_scopes: 0,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
        self.num_scopes += 1;
    }

    fn pop_scope(&mut self) -> Scope {
        // NOTE: These operations should always succeed
        let removed_scope = self.scopes.pop().expect("pop_scope");
        for key in removed_scope.variables.keys() {
            self.all_variables.remove(key);
        }

        self.num_scopes -= 1;

        removed_scope
    }

    fn current_index(&self) -> usize {
        self.num_scopes - 1
    }

    fn current_scope(&mut self) -> &mut Scope {
        let i = self.current_index();
        &mut self.scopes[i]
    }

    // TODO: Field aliasing
    // TODO: Handle shadowing
    fn add_var_to_scope(&mut self, name: String, mutable: bool, ty: Type, memory_usage: MemoryUsage) -> Result<(), String> {
        // if name exists already
        if let Some(scope_index) = self.all_variables.insert(name.clone(), self.current_index()) {
            // Name exists in the current scope
            if scope_index == self.current_index() {
                return Err(format!("Variable `{}` is already defined in this scope", name));
            } else {
                // TODO: This
                todo!("Nested scope shadowing")
            }
        }
        
        self.current_scope().insert_var_data(name, VariableData::new(ty, memory_usage, mutable));

        Ok(())
    }

    // TODO: Handle shadowing
    fn get_variable(&self, name: &str) -> Result<&VariableData, String> {
        if let Some(&index) = self.all_variables.get(name) {
            return Ok(self.scopes[index].get_var_data(name));
        }

        Err(format!("No variable `{}` in scope", name))
    }

    fn get_variable_mut(&mut self, name: &str) -> Result<&mut VariableData, String> {
        if let Some(&index) = self.all_variables.get(name) {
            return Ok(self.scopes[index].get_var_data_mut(name));
        }

        Err(format!("No variable `{}` in scope", name))
    }

    // NOTE: Program is valid at this point. No safety checks needed
    /// Uses aliases to convert the return variable's generic allocation to struct-return allocation
    /// Target variable is always in the current scope.
    fn signal_return_variable(&mut self, mut target: String) {
        let mut current;

        // Traverse the alias graph to find the true variable being returned.
        loop {
            current = self.current_scope().get_var_data_mut(&target);
            
            match &current.memory_usage {
                // keep looking for root
                MemoryUsage::Alias(next) => target = next.clone(),

                // TODO: I don't know if this is correct
                // returning what was input -> use it instead of an allocation
                MemoryUsage::FunctionParam => {
                    current.memory_usage = MemoryUsage::Alias(target);
                    break;
                }

                // Found the root
                MemoryUsage::StackSlot
                | MemoryUsage::StructReturn => {
                    current.memory_usage = MemoryUsage::StructReturn;
                    break;
                }
            }
        }
    }
}


///////////////////// FUNCTIONS /////////////////////


pub struct FunctionDefinition {
    /// Function parameters (field_name, field_type, mutable) in order
    pub parameters: Vec<(String, Type, bool)>,
    pub return_type: Type,
    pub is_extern: bool,
    pub is_validated: bool,
}

pub struct FunctionTable {
    // Map of (name -> data)
    pub functions: HashMap<String, FunctionDefinition>
}

impl FunctionTable {
    fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    // FIXME: A few copies and clones, but nothing bad
    fn forward_declare_function(&mut self, validated_prototype: &ast::FunctionPrototype, is_extern: bool) -> Result<(), String> {
        if self.functions.contains_key(&validated_prototype.name) {
            return Err(format!("Function `{}` already exists", validated_prototype.name));
        }

        let parameters = validated_prototype.parameters.iter().map(|param| {
            (param.name.clone(), param.ty.clone(), param.mutable)
        }).collect();

        let definition = FunctionDefinition {
            parameters,
            return_type: validated_prototype.return_type.clone(),
            is_extern,
            is_validated: false,
        };

        self.functions.insert(validated_prototype.name.clone(), definition);

        Ok(())
    }

    fn __get_mut(&mut self, name: &str) -> Result<&mut FunctionDefinition, String> {
        self.functions.get_mut(name)
            .ok_or(format!("Could not find function `{}`", name))
    }

    fn __get(&self, name: &str) -> Result<&FunctionDefinition, String> {
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
    // fn get_validated_function_definition(&mut self, name: &str) -> Result<&FunctionDefinition<'input>, String> {
    //     let function = self.__get(name)?;

    //     if !function.is_validated {
    //         // FIXME: This should not be possible
    //         Err(format!("Function `{}` was not validated", name))
    //     } else {
    //         Ok(function)
    //     }
    // }

    /// Returns a `FunctionDefinition` that is not guarenteed to have been
    /// successfully validated
    fn get_unchecked_function_definition(&mut self, name: &str) -> Result<&FunctionDefinition, String> {
        self.__get(name)
    }
}