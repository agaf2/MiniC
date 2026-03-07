# MiniC

A minimal C-like language parser and semantic analyzer written in Rust. MiniC parses source code into an abstract syntax tree (AST), type-checks it, and produces a typed AST suitable for interpretation or code generation.

## Quick Start

```bash
cargo build
cargo test
```

## Project Structure

```
src/
├── ir/           # Intermediate representation (AST)
├── parser/       # Parser (nom combinators)
└── semantic/     # Type checker
```

## Documentation

The following documents explain the architecture and key design decisions. **Start here** if you want to understand how MiniC works.

### Architecture

| Document | Description |
|----------|-------------|
| [**AST: Checked vs Unchecked**](doc/architecture/ast.md) | How the AST is parameterized by phase (`ExprD<()>` vs `ExprD<Type>`), type synonyms, and the parser → type checker → interpreter pipeline |
| [**Parser**](doc/architecture/parser.md) | Parser combinators, nom, operator precedence, left-associativity, and array parsing |
| [**Test Architecture**](doc/architecture/tests.md) | How tests are organized, examples of parser/program/type-checker tests, and how to add new tests |

### Design Decisions

| Document | Description |
|----------|-------------|
| [**Type Checker Design**](doc/design/type-checker.md) | Design alternatives (two ASTs vs generic parameter vs `Option<Type>`), int/float coercion rules, and type representation |

### Specifications

Formal specs live under [openspec/specs/](openspec/specs/) and [openspec/changes/](openspec/changes/). The main specs cover:

- [AST](openspec/specs/ast/spec.md)
- [Functions](openspec/specs/functions/spec.md)
- [Arrays](openspec/specs/arrays/spec.md)
- [Parser documentation](openspec/specs/parser-docs/spec.md)

## Contributing

If you want to contribute (e.g. add a feature or fix a bug), start by reading the [Test Architecture](doc/architecture/tests.md) doc. It explains how tests are organized and how to add new ones, with concrete examples for parser, program, and type-checker tests.

## Key Concepts

- **Unchecked AST** (`Program<()>`, `ExprD<()>`) — Parser output; no type information
- **Checked AST** (`Program<Type>`, `ExprD<Type>`) — Type checker output; every node has a `Type`
- **Phase separation** — Downstream phases (interpreter, codegen) accept only checked AST; Rust's type system enforces this

For full details, see [doc/architecture/ast.md](doc/architecture/ast.md).
