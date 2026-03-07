//! Abstract syntax tree for MiniC.
//!
//! The AST is parameterized by a type decoration `Ty`:
//! - `Ty = ()` for unchecked (parser output)
//! - `Ty = Type` for checked (type checker output)
//!
//! See `doc/architecture/ast.md` for the design.

/// MiniC types: scalar, array, function.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Unit,
    Int,
    Float,
    Bool,
    Str,
    Array(Box<Type>),
    Fun(Vec<Type>, Box<Type>),
}

/// A literal value.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}

/// Expression with type decoration.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprD<Ty> {
    pub exp: Expr<Ty>,
    pub ty: Ty,
}

/// An expression: literals, identifiers, and composed operations.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<Ty> {
    Literal(Literal),
    Ident(String),
    /// Unary minus (arithmetic)
    Neg(Box<ExprD<Ty>>),
    Add(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Sub(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Mul(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Div(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Eq(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Ne(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Lt(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Le(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Gt(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Ge(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Not(Box<ExprD<Ty>>),
    And(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    Or(Box<ExprD<Ty>>, Box<ExprD<Ty>>),
    /// Function call: name(args)
    Call {
        name: String,
        args: Vec<ExprD<Ty>>,
    },
    /// Array literal: [ expr, expr, ... ]
    ArrayLit(Vec<ExprD<Ty>>),
    /// Index expression: base[index]
    Index {
        base: Box<ExprD<Ty>>,
        index: Box<ExprD<Ty>>,
    },
}

/// Statement with type decoration.
#[derive(Debug, Clone, PartialEq)]
pub struct StatementD<Ty> {
    pub stmt: Statement<Ty>,
    pub ty: Ty,
}

/// A statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement<Ty> {
    Assign {
        target: Box<ExprD<Ty>>,
        value: Box<ExprD<Ty>>,
    },
    /// Block of statements: `{ stmt ; stmt ; ... }`
    Block {
        seq: Vec<StatementD<Ty>>,
    },
    Call {
        name: String,
        args: Vec<ExprD<Ty>>,
    },
    If {
        cond: Box<ExprD<Ty>>,
        then_branch: Box<StatementD<Ty>>,
        else_branch: Option<Box<StatementD<Ty>>>,
    },
    While {
        cond: Box<ExprD<Ty>>,
        body: Box<StatementD<Ty>>,
    },
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct FunDecl<Ty> {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: Type,
    pub body: Box<StatementD<Ty>>,
}

/// A complete MiniC program: function declarations and main body.
#[derive(Debug, Clone, PartialEq)]
pub struct Program<Ty> {
    pub functions: Vec<FunDecl<Ty>>,
    pub body: Vec<StatementD<Ty>>,
}

// Type synonyms for checked and unchecked phases.
pub type UncheckedExpr = ExprD<()>;
pub type CheckedExpr = ExprD<Type>;
pub type UncheckedStmt = StatementD<()>;
pub type CheckedStmt = StatementD<Type>;
pub type UncheckedFunDecl = FunDecl<()>;
pub type CheckedFunDecl = FunDecl<Type>;
pub type UncheckedProgram = Program<()>;
pub type CheckedProgram = Program<Type>;
