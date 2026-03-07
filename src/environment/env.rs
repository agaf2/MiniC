use std::collections::HashMap;

use crate::ir::ast::{CheckedFunDecl, Type};

/// Function signature: (param types, return type). Used during type-checking before bodies are checked.
pub type FuncSig = (Vec<Type>, Type);

pub struct Environment<T> {
    bindings: HashMap<String, T>,
    function_signatures: HashMap<String, FuncSig>,
    function_declarations: HashMap<String, CheckedFunDecl>,
}

impl<T> Environment<T> {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            function_signatures: HashMap::new(),
            function_declarations: HashMap::new(),
        }
    }

    pub fn add_binding(&mut self, name: String, value: T) {
        self.bindings.insert(name, value);
    }

    /// Add a function signature (forward declaration). Used during type-checking.
    pub fn add_function_signature(&mut self, name: String, params: Vec<Type>, return_type: Type) {
        self.function_signatures.insert(name, (params, return_type));
    }

    /// Look up a function signature. Returns (param types, return type).
    pub fn lookup_function_signature(&self, name: &str) -> Option<&FuncSig> {
        self.function_signatures.get(name)
    }

    /// Clear variable bindings only. Keeps function signatures. Used when entering a new function.
    pub fn clear_bindings(&mut self) {
        self.bindings.clear();
    }

    pub fn add_function_declaration(&mut self, name: String, function: CheckedFunDecl) {
        self.function_declarations.insert(name, function);
    }

    pub fn lookup(&self, name: &str) -> Option<&T> {
        self.bindings.get(name)
    }

    pub fn lookup_function(&self, name: &str) -> Option<&CheckedFunDecl> {
        self.function_declarations.get(name)
    }
}

impl<T> Default for Environment<T> {
    fn default() -> Self {
        Self::new()
    }
}
