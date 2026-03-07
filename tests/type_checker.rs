//! Integration tests for the MiniC type checker.

use nom::combinator::all_consuming;
use mini_c::ir::ast::Type;
use mini_c::parser::program;
use mini_c::semantic::type_check;

fn parse_and_type_check(src: &str) -> Result<mini_c::ir::ast::Program<Type>, mini_c::semantic::TypeError> {
    let (_, prog) = all_consuming(program)(src).map_err(|_| {
        mini_c::semantic::TypeError {
            message: "parse failed".to_string(),
        }
    })?;
    type_check(&prog)
}

#[test]
fn test_type_check_simple_assign() {
    let result = parse_and_type_check("x = 1");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_int_float_coercion() {
    let result = parse_and_type_check("x = 1 + 3.14");
    assert!(result.is_ok());
    let prog = result.unwrap();
    if let mini_c::ir::ast::Statement::Assign { ref value, .. } = prog.body[0].stmt {
        assert_eq!(value.ty, Type::Float);
    } else {
        panic!("expected Assign");
    }
}

#[test]
fn test_type_check_undeclared_var() {
    let result = parse_and_type_check("x = y");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("undeclared"));
}

#[test]
fn test_type_check_bool_condition() {
    let result = parse_and_type_check("if true then x = 1");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_array_literal() {
    let result = parse_and_type_check("x = [1, 2, 3]");
    assert!(result.is_ok());
}

#[test]
fn test_type_check_array_index() {
    let result = parse_and_type_check("arr = [1, 2]\nx = arr[0]");
    assert!(result.is_ok());
}
