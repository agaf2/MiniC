# AST Architecture: Checked vs Unchecked

This document explains MiniC's abstract syntax tree design: a single parameterized representation that exists in two phases—**unchecked** (from the parser) and **checked** (from the type checker). The design ensures that downstream phases (interpretation, code generation) can only consume type-checked ASTs.

*See also:* [Parser Architecture](parser.md), [Type Checker Design](../design/type-checker.md), [Test Architecture](tests.md)

---

## 1. Why Two Phases?

A compiler pipeline typically has stages:

1. **Parse** → produce an AST from source
2. **Type-check** → validate types, attach type information
3. **Interpret / Codegen** → execute or emit code

The parser has no type information; it only knows syntax. The type checker validates that expressions and statements are well-typed and attaches a `Type` to each node. Downstream phases need that type information and must not receive unchecked ASTs.

We want **compile-time guarantees**: a function that expects a type-checked program should not accept an unchecked one. Rust's type system can enforce this.

---

## 2. Single AST, Parameterized by Phase

Instead of two separate AST types (`Expr` vs `TypedExpr`), MiniC uses **one parameterized AST**. The type parameter indicates the phase:

- **`Ty = ()`** → unchecked (parser output). The decoration field is `()` (zero-sized).
- **`Ty = Type`** → checked (type checker output). The decoration field holds the inferred type.

Every expression and statement node carries a `ty: Ty` field. When unchecked, it's `()`; when checked, it's `Type`.

```
Parser          →  Program<()>      (unchecked)
Type checker    →  Program<Type>    (checked)
Interpreter     ←  Program<Type>    (only accepts checked)
```

---

## 3. Structure: ExprD and StatementD

Each node that can be type-checked is wrapped in a **decorated** struct:

```rust
struct ExprD<Ty> {
    exp: Expr<Ty>,   // the expression structure
    ty: Ty,          // type decoration: () or Type
}

struct StatementD<Ty> {
    stmt: Statement<Ty>,
    ty: Ty,
}
```

- **`Expr<Ty>`** — the expression variants (Literal, Add, Index, etc.). Subexpressions are `ExprD<Ty>`.
- **`ExprD<Ty>`** — wrapper that adds the `ty` field. This is what the parser and type checker produce.

The same pattern applies to statements, function declarations, and the program root.

---

## 4. Type Synonyms

To make intent clear at call sites, we define type aliases:

| Alias | Definition | Produced by |
|-------|------------|-------------|
| `UncheckedExpr` | `ExprD<()>` | Parser |
| `CheckedExpr` | `ExprD<Type>` | Type checker |
| `UncheckedStmt` | `StatementD<()>` | Parser |
| `CheckedStmt` | `StatementD<Type>` | Type checker |
| `UncheckedProgram` | `Program<()>` | Parser |
| `CheckedProgram` | `Program<Type>` | Type checker |
| `UncheckedFunDecl` | `FunDecl<()>` | Parser |
| `CheckedFunDecl` | `FunDecl<Type>` | Type checker |

Example:

```rust
fn type_check(program: &UncheckedProgram) -> Result<CheckedProgram, TypeError> { ... }
fn interpret(program: &CheckedProgram) -> Value { ... }  // cannot accept UncheckedProgram
```

---

## 5. The Type Parameter Propagates

The type parameter `Ty` is uniform across the entire tree. You cannot have a `CheckedExpr` whose child is an `UncheckedExpr`—the type system forbids it.

```
Expr<Type>  contains  ExprD<Type>  (all checked)
Expr<()>    contains  ExprD<()>    (all unchecked)
```

This matches the Haskell GADT style: once an expression is checked, every subexpression is checked.

---

## 6. Zero Cost for Unchecked

When `Ty = ()`, the `ty` field is `()`, which is zero-sized in Rust. There is no memory overhead for unchecked ASTs.

---

## 7. Pipeline Summary

```
Source  →  Parser  →  Program<()>  →  Type checker  →  Program<Type>  →  Interpreter / Codegen
                    (UncheckedProgram)   (CheckedProgram)
```

- **Parser**: Produces `ExprD<()>`, `StatementD<()>` with `ty: ()` at every node.
- **Type checker**: Consumes `Program<()>`, returns `Result<Program<Type>, TypeError>`. On success, every node has `ty: Type`.
- **Interpreter / Codegen**: Accept only `Program<Type>`; the type system prevents passing unchecked data.

---

## 8. Relation to Haskell / SmartPy

This design is inspired by the **Deco** pattern in SmartPy's Haskell implementation:

```haskell
data Deco s t where
  U :: Deco 'Unchecked t    -- no value
  T :: t -> Deco 'Checked t -- carries value

data ExprD s = ExprD { exp :: Expr s, expType :: Deco s Type }
```

In Haskell, `s` is a kind-level tag (`Unchecked` or `Checked`); `Deco s Type` is either `U` (no type) or `T ty` (has type). Rust achieves the same effect with a generic type parameter: `Ty = ()` vs `Ty = Type`.
