//! Function declaration parser for MiniC.

use crate::ir::ast::{FunDecl, Type};
use crate::parser::identifiers::identifier;
use crate::parser::statements::statement;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::multispace0,
    combinator::map,
    multi::separated_list0,
    sequence::{delimited, preceded},
    IResult,
};

/// Parse a type name: Int | Float | Bool | Str | Unit
fn type_name(input: &str) -> IResult<&str, Type> {
    preceded(
        multispace0,
        alt((
            map(tag("Int"), |_| Type::Int),
            map(tag("Float"), |_| Type::Float),
            map(tag("Bool"), |_| Type::Bool),
            map(tag("Str"), |_| Type::Str),
            map(tag("Unit"), |_| Type::Unit),
        )),
    )(input)
}

/// Parse a function declaration: `def name(params) -> ReturnType body`.
pub fn fun_decl(input: &str) -> IResult<&str, FunDecl<()>> {
    let (rest, _) = preceded(multispace0, tag("def"))(input)?;
    let (rest, name) = preceded(multispace0, identifier)(rest)?;
    let (rest, params) = delimited(
        preceded(multispace0, tag("(")),
        separated_list0(
            preceded(multispace0, tag(",")),
            preceded(multispace0, identifier),
        ),
        preceded(multispace0, tag(")")),
    )(rest)?;
    let (rest, _) = preceded(multispace0, tag("->"))(rest)?;
    let (rest, return_type) = type_name(rest)?;
    let (rest, body) = preceded(multispace0, statement)(rest)?;
    Ok((
        rest,
        FunDecl {
            name: name.to_string(),
            params: params.iter().map(|s| s.to_string()).collect(),
            return_type,
            body: Box::new(body),
        },
    ))
}
