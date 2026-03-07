//! Type checker for MiniC.
//!
//! Consumes `Program<()>` (unchecked) and returns `Result<Program<Type>, TypeError>`.
//! Fails at the first error.

use crate::ir::ast::{Expr, ExprD, FunDecl, Literal, Program, Statement, StatementD, Type};
use std::collections::HashMap;

/// A type error reported by the type checker.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub message: String,
}

impl TypeError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TypeError {}

/// Type-check a program. Returns `Ok(Program<Type>)` if well-typed, `Err(TypeError)` on first error.
pub fn type_check(program: &Program<()>) -> Result<Program<Type>, TypeError> {
    let mut env: HashMap<String, Type> = HashMap::new();
    let mut functions = Vec::new();
    for f in &program.functions {
        let checked = type_check_fun_decl(f, &mut env)?;
        functions.push(checked);
    }
    env.clear();
    let mut body = Vec::new();
    for s in &program.body {
        let checked = type_check_stmt(s, &mut env)?;
        body.push(checked);
    }
    Ok(Program { functions, body })
}

fn type_check_fun_decl(
    f: &FunDecl<()>,
    env: &mut HashMap<String, Type>,
) -> Result<FunDecl<Type>, TypeError> {
    env.clear();
    for p in &f.params {
        env.insert(p.clone(), Type::Int); // TODO: param types from signature when we have them
    }
    let body = type_check_stmt(&f.body, env)?;
    // TODO: validate body type matches f.return_type
    Ok(FunDecl {
        name: f.name.clone(),
        params: f.params.clone(),
        return_type: f.return_type.clone(),
        body: Box::new(body),
    })
}

fn type_check_stmt(
    s: &StatementD<()>,
    env: &mut HashMap<String, Type>,
) -> Result<StatementD<Type>, TypeError> {
    let stmt = match &s.stmt {
        Statement::Assign { target, value } => {
            let value_checked = type_check_expr_to_typed(value, env)?;
            type_check_assign_target(&target.exp, &value_checked.ty, env)?;
            Statement::Assign {
                target: Box::new(type_check_expr_to_typed(target, env)?),
                value: Box::new(value_checked),
            }
        }
        Statement::Block { seq } => {
            let mut checked = Vec::new();
            for st in seq {
                checked.push(type_check_stmt(st, env)?);
            }
            Statement::Block { seq: checked }
        }
        Statement::Call { name, args } => {
            let args_checked: Result<Vec<_>, _> =
                args.iter().map(|a| type_check_expr_to_typed(a, env)).collect();
            let args_checked = args_checked?;
            Statement::Call {
                name: name.clone(),
                args: args_checked,
            }
        }
        Statement::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond_checked = type_check_expr_to_typed(cond, env)?;
            if cond_checked.ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "if condition must be Bool, got {:?}",
                    cond_checked.ty
                )));
            }
            let then_checked = type_check_stmt(then_branch, env)?;
            let else_checked = else_branch
                .as_ref()
                .map(|e| type_check_stmt(e, env))
                .transpose()?;
            Statement::If {
                cond: Box::new(cond_checked),
                then_branch: Box::new(then_checked),
                else_branch: else_checked.map(Box::new),
            }
        }
        Statement::While { cond, body } => {
            let cond_checked = type_check_expr_to_typed(cond, env)?;
            if cond_checked.ty != Type::Bool {
                return Err(TypeError::new(format!(
                    "while condition must be Bool, got {:?}",
                    cond_checked.ty
                )));
            }
            let body_checked = type_check_stmt(body, env)?;
            Statement::While {
                cond: Box::new(cond_checked),
                body: Box::new(body_checked),
            }
        }
    };
    Ok(StatementD {
        stmt,
        ty: Type::Unit,
    })
}

fn type_check_assign_target(
    target: &Expr<()>,
    value_ty: &Type,
    env: &mut HashMap<String, Type>,
) -> Result<(), TypeError> {
    match target {
        Expr::Ident(name) => {
            env.insert(name.clone(), value_ty.clone());
            Ok(())
        }
        Expr::Index { base, index } => {
            let index_ty = type_check_expr(index, env)?;
            if index_ty != Type::Int {
                return Err(TypeError::new("array index must be Int"));
            }
            let base_ty = type_check_expr(base, env)?;
            if let Type::Array(elem) = &base_ty {
                if **elem != *value_ty {
                    return Err(TypeError::new("assignment type mismatch"));
                }
            } else {
                return Err(TypeError::new("indexed target must be array"));
            }
            Ok(())
        }
        _ => Err(TypeError::new("invalid assignment target")),
    }
}

fn type_check_expr_to_typed(
    e: &ExprD<()>,
    env: &HashMap<String, Type>,
) -> Result<ExprD<Type>, TypeError> {
    let ty = type_check_expr(e, env)?;
    let exp = type_check_expr_inner(&e.exp, env)?;
    Ok(ExprD { exp, ty })
}

fn type_check_expr_inner(
    e: &Expr<()>,
    env: &HashMap<String, Type>,
) -> Result<Expr<Type>, TypeError> {
    match e {
        Expr::Literal(l) => Ok(Expr::Literal(l.clone())),
        Expr::Ident(name) => Ok(Expr::Ident(name.clone())),
        Expr::Neg(inner) => {
            let i = type_check_expr_to_typed(inner, env)?;
            Ok(Expr::Neg(Box::new(i)))
        }
        Expr::Add(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Add(Box::new(lt), Box::new(rt)))
        }
        Expr::Sub(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Sub(Box::new(lt), Box::new(rt)))
        }
        Expr::Mul(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Mul(Box::new(lt), Box::new(rt)))
        }
        Expr::Div(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Div(Box::new(lt), Box::new(rt)))
        }
        Expr::Eq(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Eq(Box::new(lt), Box::new(rt)))
        }
        Expr::Ne(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Ne(Box::new(lt), Box::new(rt)))
        }
        Expr::Lt(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Lt(Box::new(lt), Box::new(rt)))
        }
        Expr::Le(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Le(Box::new(lt), Box::new(rt)))
        }
        Expr::Gt(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Gt(Box::new(lt), Box::new(rt)))
        }
        Expr::Ge(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Ge(Box::new(lt), Box::new(rt)))
        }
        Expr::Not(inner) => {
            let i = type_check_expr_to_typed(inner, env)?;
            Ok(Expr::Not(Box::new(i)))
        }
        Expr::And(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::And(Box::new(lt), Box::new(rt)))
        }
        Expr::Or(l, r) => {
            let lt = type_check_expr_to_typed(l, env)?;
            let rt = type_check_expr_to_typed(r, env)?;
            Ok(Expr::Or(Box::new(lt), Box::new(rt)))
        }
        Expr::Call { name, args } => {
            let args_checked: Result<Vec<_>, _> =
                args.iter().map(|a| type_check_expr_to_typed(a, env)).collect();
            Ok(Expr::Call {
                name: name.clone(),
                args: args_checked?,
            })
        }
        Expr::ArrayLit(elems) => {
            let elems_checked: Result<Vec<_>, _> =
                elems.iter().map(|e| type_check_expr_to_typed(e, env)).collect();
            Ok(Expr::ArrayLit(elems_checked?))
        }
        Expr::Index { base, index } => {
            let base_checked = type_check_expr_to_typed(base, env)?;
            let index_checked = type_check_expr_to_typed(index, env)?;
            Ok(Expr::Index {
                base: Box::new(base_checked),
                index: Box::new(index_checked),
            })
        }
    }
}

fn type_check_expr(e: &ExprD<()>, env: &HashMap<String, Type>) -> Result<Type, TypeError> {
    match &e.exp {
        Expr::Literal(l) => Ok(literal_type(l)),
        Expr::Ident(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| TypeError::new(format!("undeclared variable: {}", name))),
        Expr::Neg(inner) => {
            let ty = type_check_expr(inner, env)?;
            if matches!(ty, Type::Int | Type::Float) {
                Ok(ty)
            } else {
                Err(TypeError::new("unary minus requires Int or Float"))
            }
        }
        Expr::Add(l, r) | Expr::Sub(l, r) | Expr::Mul(l, r) | Expr::Div(l, r) => {
            let lt = type_check_expr(l, env)?;
            let rt = type_check_expr(r, env)?;
            numeric_binop_result(&lt, &rt)
        }
        Expr::Eq(l, r) | Expr::Ne(l, r) | Expr::Lt(l, r) | Expr::Le(l, r) | Expr::Gt(l, r) | Expr::Ge(l, r) => {
            let _lt = type_check_expr(l, env)?;
            let _rt = type_check_expr(r, env)?;
            Ok(Type::Bool)
        }
        Expr::Not(inner) => {
            let ty = type_check_expr(inner, env)?;
            if ty == Type::Bool {
                Ok(Type::Bool)
            } else {
                Err(TypeError::new("not requires Bool operand"))
            }
        }
        Expr::And(l, r) | Expr::Or(l, r) => {
            let lt = type_check_expr(l, env)?;
            let rt = type_check_expr(r, env)?;
            if lt == Type::Bool && rt == Type::Bool {
                Ok(Type::Bool)
            } else {
                Err(TypeError::new("and/or require Bool operands"))
            }
        }
        Expr::Call { name: _, args } => {
            for a in args {
                type_check_expr(a, env)?;
            }
            Ok(Type::Unit) // TODO: function return type
        }
        Expr::ArrayLit(elems) => {
            if elems.is_empty() {
                return Err(TypeError::new("empty array literal needs type annotation"));
            }
            let first = type_check_expr(&elems[0], env)?;
            for e in elems.iter().skip(1) {
                let ty = type_check_expr(e, env)?;
                if !types_compatible(&first, &ty) {
                    return Err(TypeError::new("array elements must have same type"));
                }
            }
            Ok(Type::Array(Box::new(first)))
        }
        Expr::Index { base, index } => {
            let index_ty = type_check_expr(index, env)?;
            if index_ty != Type::Int {
                return Err(TypeError::new("array index must be Int"));
            }
            let base_ty = type_check_expr(base, env)?;
            if let Type::Array(elem) = base_ty {
                Ok(*elem)
            } else {
                Err(TypeError::new("indexed expression must be array"))
            }
        }
    }
}

fn literal_type(l: &Literal) -> Type {
    match l {
        Literal::Int(_) => Type::Int,
        Literal::Float(_) => Type::Float,
        Literal::Str(_) => Type::Str,
        Literal::Bool(_) => Type::Bool,
    }
}

fn numeric_binop_result(l: &Type, r: &Type) -> Result<Type, TypeError> {
    match (l, r) {
        (Type::Int, Type::Int) => Ok(Type::Int),
        (Type::Int, Type::Float) | (Type::Float, Type::Int) | (Type::Float, Type::Float) => {
            Ok(Type::Float)
        }
        _ => Err(TypeError::new("arithmetic operands must be Int or Float")),
    }
}

fn types_compatible(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Int, Type::Int) | (Type::Float, Type::Float) | (Type::Bool, Type::Bool) | (Type::Str, Type::Str) => true,
        (Type::Int, Type::Float) | (Type::Float, Type::Int) => true,
        (Type::Array(a), Type::Array(b)) => types_compatible(a, b),
        _ => false,
    }
}
