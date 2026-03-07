# Type Checker Design

This document describes the design of the MiniC semantic analyzer (type checker), with emphasis on how to achieve Haskell GADT–style guarantees in Rust: an unchecked AST as input and a checked, type-enriched AST as output.

*See also:* [AST Architecture](../architecture/ast.md) (describes the implemented parameterized design), [Parser Architecture](../architecture/parser.md)

---

## 1. Goals

- **Input**: An instance of the current AST (parse output, no type information).
- **Output**: Either a type error or a new AST enriched with type information.
- **Int/float coercion**: Expressions mixing int and float should promote to float (e.g., `1 + 3.14` → float).
- **Type safety**: The type checker should produce a distinct representation that cannot be confused with unchecked data.

---

## 2. Haskell GADT Approach (Reference)

In Haskell, GADTs let you index expressions by their type:

```haskell
data Expr a where
  LitInt  :: Int  -> Expr Int
  LitFloat:: Float -> Expr Float
  Add     :: Expr Int  -> Expr Int  -> Expr Int
  AddMixed:: Expr Int  -> Expr Float -> Expr Float  -- int promoted to float
  ...

typeCheck :: UncheckedExpr -> Either TypeError (Expr a)
```

Properties:

- **Type-level indexing**: A checked `Expr Float` is guaranteed to have type `Float` at compile time.
- **Phase separation**: `UncheckedExpr` and `Expr a` are different types; you cannot mix them.
- **Result semantics**: `typeCheck` returns `Either TypeError (Expr a)`; success yields a typed expression.

Rust does not have GADTs. The rest of this document explores Rust techniques that approximate this behavior.

---

## 3. Rust Techniques

### 3.1 Technique A: Two Parallel AST Types (Recommended)

Define a separate typed AST that mirrors the unchecked one, with a `Type` (or `ty`) field at each node.

```rust
// Current (unchecked) - from parser
pub struct Program { pub functions: Vec<FunDecl>, pub body: Vec<Stmt> }
pub enum Expr { Literal(Literal), Ident(String), Add(Box<Expr>, Box<Expr>), ... }

// New (checked) - output of type checker
pub struct TypedProgram { pub functions: Vec<TypedFunDecl>, pub body: Vec<TypedStmt> }
pub struct TypedExpr {
    pub node: TypedExprNode,
    pub ty: Type,  // Attached after checking
}
pub enum TypedExprNode {
    Literal(Literal),
    Ident(String),
    Add(Box<TypedExpr>, Box<TypedExpr>),
    ...
}
```

**Type checker signature:**

```rust
pub fn type_check(program: &Program) -> Result<TypedProgram, Vec<TypeError>>
```

**Pros:**

- Clear separation: unchecked vs typed AST are different types.
- `TypedProgram` guarantees every node has been checked.
- Straightforward mapping from unchecked to typed structure.
- No runtime overhead for “phase” markers.

**Cons:**

- Some duplication between `Expr` and `TypedExpr` / `TypedExprNode`.
- Can be reduced with macros or code generation if the duplication grows.

---

### 3.2 Technique B: Phantom Types + Newtype Wrappers

Add a type-level “phase” marker so the type system enforces that only unchecked AST is fed to the type checker, and only checked AST is used downstream.

```rust
// Marker types (zero-sized)
pub struct Unchecked;
pub struct Checked;

// Wrapper that carries the phase at the type level
pub struct Ast<P>(Program, PhantomData<P>);

impl Ast<Unchecked> {
    pub fn type_check(self) -> Result<Ast<Checked>, Vec<TypeError>> {
        type_check_program(&self.0).map(|p| Ast(p, PhantomData))
    }
}

// Codegen, interpreter, etc. only accept Ast<Checked>
pub fn compile(ast: Ast<Checked>) -> ... { ... }
```

The inner `Program` can be the same structure in both phases; you either:

- Use `Option<Type>` and rely on convention that it is `Some` when `P = Checked`, or
- Use Technique A (separate `TypedProgram`) inside the wrapper.

**Pros:**

- Compile-time guarantee that `compile` cannot receive unchecked AST.
- Good for API boundaries between phases.

**Cons:**

- Phantom alone does not attach type information; you still need a typed representation (Technique A) or `Option<Type>`.

**Combination:** Use Technique A for the actual data and Technique B for API boundaries:

```rust
pub struct CheckedProgram(TypedProgram);
impl CheckedProgram {
    pub fn inner(&self) -> &TypedProgram { &self.0 }
}
```

---

### 3.3 Technique C: Single AST with Optional Type Annotation

Keep one `Expr` and add `Option<Type>` (or similar) to each node:

```rust
pub struct Expr {
    pub node: ExprNode,
    pub ty: Option<Type>,  // None = unchecked, Some = checked
}
```

**Pros:**

- No duplication.
- Simple to implement.

**Cons:**

- No static guarantee that `ty` is `Some` when “checked”; easy to use unchecked data by mistake.
- `Option` is checked at runtime, not at compile time.

---

### 3.4 Technique D: Generic AST with Type Parameter

Parameterize the AST by what is stored at each node:

```rust
pub enum Expr<A> {
    Literal(Literal, A),
    Ident(String, A),
    Add(Box<Expr<A>>, Box<Expr<A>>, A),
    ...
}

// Unchecked: A = ()
type UncheckedExpr = Expr<()>;

// Checked: A = Type
type TypedExpr = Expr<Type>;
```

**Pros:**

- Single definition; `A` varies by phase.
- Can enforce that `A = Type` when checked.

**Cons:**

- Every constructor gets an extra `A` parameter; `()` for unchecked feels redundant.
- More complex pattern matching and construction.

---

## 4. Recommendation

**Use Technique A (two parallel AST types) as the core design**, optionally combined with Technique B for API boundaries:

1. **`ir::ast`** – Keep the current unchecked AST (parser output).
2. **`ir::typed_ast`** – New module with `TypedProgram`, `TypedExpr`, `TypedStmt`, etc., each carrying a `Type`.
3. **`semantic::type_checker`** – `fn type_check(program: &Program) -> Result<TypedProgram, Vec<TypeError>>`.
4. **Optional**: Wrap `TypedProgram` in `CheckedProgram` (newtype) so later passes (codegen, interpreter) cannot accidentally receive unchecked AST.

This gives:

- Clear phase separation (unchecked vs typed).
- A result type analogous to Haskell’s `Either TypeError (Expr a)`.
- Room for int/float coercion and richer type rules without changing the core idea.

---

## 5. Int/Float Coercion Rules

When both operands are numeric, promote to the “wider” type:

| Left   | Right | Result |
|--------|-------|--------|
| `Int`  | `Int` | `Int`  |
| `Int`  | `Float`| `Float`|
| `Float`| `Int` | `Float`|
| `Float`| `Float`| `Float`|

**Operators:** `+`, `-`, `*`, `/` — apply the table above.

**Unary minus:** `Neg(e)` — if `e : Int` then `Int`; if `e : Float` then `Float`.

**Relational operators** (`==`, `!=`, `<`, `<=`, `>`, `>=`): same coercion for operands; result type is `Bool`.

**Boolean operators** (`and`, `or`, `!`): operands must be `Bool`; result is `Bool`.

**Array literals:** `[e1, e2, ...]` — all elements must have the same type (after coercion); result is `Array(elem_ty)`.

**Index:** `base[i]` — `base` must be `Array(t)`; `i` must be `Int`; result is `t`.

---

## 6. Type Representation

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Array(Box<Type>),
    // Future: Function(Vec<Type>, Box<Type>)
}
```

---

## 7. Summary

| Technique | Phase separation | Type attachment | Compile-time safety | Complexity |
|-----------|------------------|-----------------|---------------------|------------|
| A: Two ASTs | ✓ | Explicit in typed AST | ✓ | Medium |
| B: Phantom wrappers | ✓ | Via A or Option | ✓ (API) | Low (add-on) |
| C: Option&lt;Type&gt; | ✗ | Runtime check | ✗ | Low |
| D: Generic Expr&lt;A&gt; | ✓ | Via type param | ✓ | High |

**Recommended:** Technique A, with optional Technique B for phase boundaries. This mirrors the Haskell pattern of `UncheckedExpr -> Either TypeError (Expr a)` while staying idiomatic in Rust.
