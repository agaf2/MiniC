# Test Architecture

This document describes how tests are organized in MiniC and how to add new ones. See also: [AST Architecture](ast.md), [Parser Architecture](parser.md).

---

## 1. Overview

MiniC uses **integration tests** in the `tests/` directory. All tests use only the public API of the `mini_c` library. There are no `#[cfg(test)]` blocks inside source modules—tests live entirely in `tests/`.

```
tests/
├── parser.rs        # Parser unit-style tests (literals, expressions, statements, etc.)
├── program.rs       # Full-program parsing from fixture files
├── type_checker.rs  # Semantic (type-checking) tests
└── fixtures/        # MiniC source files for program tests
    ├── empty.minic
    ├── statements_only.minic
    ├── function_single.minic
    ├── function_with_block.minic
    ├── full_program.minic
    └── invalid_syntax.minic
```

Run all tests with `cargo test`.

---

## 2. Test Files

### 2.1 `tests/parser.rs` — Parser Tests

Tests individual parser functions (literals, identifiers, expressions, statements, functions) using **inline strings**. Each test focuses on one construct.

**Pattern:** Call a parser, assert on the result. Parser functions return `IResult<&str, T>`: `Ok((remaining_input, value))` or `Err(...)`.

#### Example: Literals

```rust
#[test]
fn test_integer_positive() {
    assert_eq!(integer_literal("42"), Ok(("", 42)));
    assert_eq!(integer_literal("0"), Ok(("", 0)));
}

#[test]
fn test_integer_reject() {
    assert!(integer_literal("abc").is_err());
    assert!(integer_literal("12.34").is_err());
}
```

#### Example: Expressions (AST structure)

For expressions and statements, the parser returns `ExprD<()>` or `StatementD<()>`. Use `.map(|(r, e)| (r, e.exp))` to compare the inner `Expr` if you don't care about the `ty: ()` field:

```rust
#[test]
fn test_primary_literal() {
    assert_eq!(
        expression("42").map(|(r, e)| (r, e.exp)),
        Ok(("", Expr::Literal(Literal::Int(42))))
    );
    assert_eq!(
        expression("x").map(|(r, e)| (r, e.exp)),
        Ok(("", Expr::Ident("x".to_string())))
    );
}
```

#### Example: Precedence and structure

Use `match` or `matches!` to inspect the AST structure:

```rust
#[test]
fn test_precedence_arithmetic() {
    let result = expression("1 + 2 * 3").unwrap().1.exp;
    match &result {
        Expr::Add(l, r) => {
            assert_eq!(l.exp, Expr::Literal(Literal::Int(1)));
            match &r.exp {
                Expr::Mul(m, n) => {
                    assert_eq!(m.exp, Expr::Literal(Literal::Int(2)));
                    assert_eq!(n.exp, Expr::Literal(Literal::Int(3)));
                }
                _ => panic!("expected Mul"),
            }
        }
        _ => panic!("expected Add"),
    }
}
```

#### Example: Invalid input

```rust
#[test]
fn test_invalid_unbalanced_paren() {
    assert!(expression("(1 + 2").is_err());
    assert!(all_consuming(expression)("1 + 2)").is_err());
}
```

Use `all_consuming(parser)` when the entire input must be consumed; otherwise the parser may succeed and leave trailing input.

---

### 2.2 `tests/program.rs` — Full-Program Tests

Tests parsing **complete MiniC programs** from fixture files in `tests/fixtures/`. Use this when you want to test the full pipeline (many functions + body) or multi-line programs.

**Pattern:** Read a `.minic` file, parse with `all_consuming(program)`, assert on the resulting `Program`.

#### Helper

```rust
fn fixtures_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn parse_program_file(name: &str) -> Result<Program<()>, ...> {
    let path = fixtures_dir().join(name);
    let src = std::fs::read_to_string(&path).expect("fixture file should exist");
    let src = src.trim();
    let parse_result = all_consuming(program)(src);
    match parse_result {
        Ok((_, prog)) => Ok(prog),
        Err(e) => Err(e.map_input(String::from)),
    }
}
```

`env!("CARGO_MANIFEST_DIR")` ensures the path works regardless of the current working directory.

#### Example: Assert on program structure

```rust
#[test]
fn test_parse_function_with_block() {
    let prog = parse_program_file("function_with_block.minic")
        .expect("function with block should parse");
    assert_eq!(prog.functions.len(), 1);
    assert_eq!(prog.functions[0].name, "add");
    assert_eq!(prog.functions[0].params, vec!["x", "y"]);
    assert!(matches!(prog.functions[0].body.stmt, Statement::Block { ref seq } if seq.len() == 2));
}
```

#### Example: Expect parse failure

```rust
#[test]
fn test_parse_invalid_syntax_fails() {
    let result = parse_program_file("invalid_syntax.minic");
    assert!(result.is_err(), "invalid syntax should fail to parse");
}
```

---

### 2.3 `tests/type_checker.rs` — Semantic Tests

Tests the type checker: parse a program, then run `type_check`. Use **inline strings** for short programs.

**Pattern:** Parse + type-check in one helper, assert on success/failure or on the resulting typed AST.

#### Helper

```rust
fn parse_and_type_check(src: &str) -> Result<Program<Type>, TypeError> {
    let (_, prog) = all_consuming(program)(src).map_err(|_| TypeError {
        message: "parse failed".to_string(),
    })?;
    type_check(&prog)
}
```

#### Example: Success and type inspection

```rust
#[test]
fn test_type_check_int_float_coercion() {
    let result = parse_and_type_check("x = 1 + 3.14");
    assert!(result.is_ok());
    let prog = result.unwrap();
    if let Statement::Assign { ref value, .. } = prog.body[0].stmt {
        assert_eq!(value.ty, Type::Float);
    } else {
        panic!("expected Assign");
    }
}
```

#### Example: Expect type error

```rust
#[test]
fn test_type_check_undeclared_var() {
    let result = parse_and_type_check("x = y");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("undeclared"));
}
```

---

## 3. Adding New Tests

### Adding a parser test

1. Open `tests/parser.rs`.
2. Find the section for the construct (e.g. `// --- Expressions ---`).
3. Add a `#[test] fn test_<name>() { ... }` function.
4. Use the same patterns: `assert_eq!` for expected output, `assert!(...is_err())` for invalid input.

### Adding a program fixture test

1. Create a new `.minic` file in `tests/fixtures/` (e.g. `my_feature.minic`).
2. Open `tests/program.rs`.
3. Add a test that calls `parse_program_file("my_feature.minic")` and asserts on the result.

### Adding a type checker test

1. Open `tests/type_checker.rs`.
2. Add a test that calls `parse_and_type_check("...")`.
3. Assert `is_ok()` or `is_err()` and, if needed, inspect the typed AST or error message.

---

## 4. Naming Conventions

- **Test functions:** `test_<construct>_<scenario>` (e.g. `test_integer_positive`, `test_if_with_else`).
- **Fixtures:** Descriptive names with `.minic` extension (e.g. `function_with_block.minic`).

---

## 5. Summary

| File          | What it tests        | Input style      |
|---------------|----------------------|------------------|
| `parser.rs`   | Individual parsers   | Inline strings   |
| `program.rs`  | Full program parse  | Fixture files    |
| `type_checker.rs` | Parse + type-check | Inline strings   |

- Use **inline strings** for short, focused tests (parser, type checker).
- Use **fixture files** for multi-line programs or when you want to share a program across tests.
- All tests are integration tests: they use only `mini_c`'s public API.
