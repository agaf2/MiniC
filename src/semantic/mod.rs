//! Semantic analysis for MiniC: type checking.

pub mod type_checker;

pub use type_checker::{type_check, TypeError};
