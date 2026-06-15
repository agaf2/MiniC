//! Top-level program parser for MiniC.
//!
//! # Overview
//!
//! Exposes one public function:
//!
//! * [`program`] — parses a complete MiniC program as a sequence of zero or
//!   more struct definitions, function declarations, and `test` blocks,
//!   returning an [`UncheckedProgram`].
//!
//! A valid MiniC program contains only struct definitions, function
//! declarations, and `test` blocks at the top level. Struct definitions use
//! the `struct` keyword; function declarations start with a return type. The
//! type checker then verifies that a `main` function exists (required in
//! `--run` mode) and that all struct names are valid.
//!
//! # Design Decisions
//!
//! ## Interleaved structs, functions, and tests
//!
//! The grammar allows struct definitions, function declarations, and `test`
//! blocks to appear in any order at the top level. Each iteration of the
//! `many0` loop tries `struct_def` first (because `struct` is a reserved
//! keyword not valid as a type name, so a failed attempt is unambiguous),
//! then `test_decl` (because `test` is similarly unambiguous), then falls
//! back to `fun_decl`. Results are separated into `structs`, `functions`, and
//! `tests` vecs.

use crate::ir::ast::{Program, StructDef, TestDecl, UncheckedFunDecl, UncheckedProgram, UncheckedTestDecl};
use crate::parser::functions::{fun_decl, type_name};
use crate::parser::identifiers::identifier;
use crate::parser::statements::statement;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, multispace0, multispace1},
    combinator::map,
    multi::many0,
    sequence::{delimited, preceded, tuple},
    IResult,
};

/// Parse a single struct field: `Type identifier ;`
fn struct_field(input: &str) -> IResult<&str, (String, crate::ir::ast::Type)> {
    map(
        tuple((
            preceded(multispace0, type_name),
            preceded(multispace1, identifier),
            preceded(multispace0, char(';')),
        )),
        |(ty, name, _)| (name.to_string(), ty),
    )(input)
}

/// Parse a struct definition: `struct Name { field* }`
pub fn struct_def(input: &str) -> IResult<&str, StructDef> {
    let (rest, _) = preceded(multispace0, tag("struct"))(input)?;
    let (rest, name) = preceded(multispace1, identifier)(rest)?;
    let (rest, fields) = delimited(
        preceded(multispace0, char('{')),
        many0(struct_field),
        preceded(multispace0, char('}')),
    )(rest)?;
    Ok((rest, StructDef { name: name.to_string(), fields }))
}

/// Parse a named test block: `test "name" { stmts }`.
fn test_decl(input: &str) -> IResult<&str, UncheckedTestDecl> {
    let (rest, _) = preceded(multispace0, tag("test"))(input)?;
    let (rest, name) = preceded(
        multispace0,
        delimited(char('"'), nom::bytes::complete::take_while(|c| c != '"'), char('"')),
    )(rest)?;
    let (rest, body) = preceded(
        multispace0,
        map(
            delimited(
                preceded(multispace0, char('{')),
                many0(statement),
                preceded(multispace0, char('}')),
            ),
            |seq| crate::ir::ast::StatementD {
                stmt: crate::ir::ast::Statement::Block { seq },
                ty: (),
            },
        ),
    )(rest)?;
    Ok((
        rest,
        TestDecl {
            name: name.to_string(),
            body: Box::new(body),
        },
    ))
}

enum TopLevel {
    Struct(StructDef),
    Fun(UncheckedFunDecl),
    Test(UncheckedTestDecl),
}

/// Parse a complete MiniC program: zero or more struct definitions, function
/// declarations, and test blocks.
/// Execution starts at the `main` function (validated by the type checker).
pub fn program(input: &str) -> IResult<&str, UncheckedProgram> {
    let (rest, items) = many0(preceded(
        multispace0,
        alt((
            map(struct_def, TopLevel::Struct),
            map(test_decl, TopLevel::Test),
            map(fun_decl, TopLevel::Fun),
        )),
    ))(input)?;

    let mut structs = Vec::new();
    let mut functions = Vec::new();
    let mut tests = Vec::new();
    for item in items {
        match item {
            TopLevel::Struct(s) => structs.push(s),
            TopLevel::Fun(f) => functions.push(f),
            TopLevel::Test(t) => tests.push(t),
        }
    }
    Ok((rest, Program { structs, functions, tests }))
}
