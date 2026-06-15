# Projeto 1 — Tipos Struct (Parte 1 e Parte 2)

**Grupo:** Alberto Guevara de Araujo Franca, Davi Gonzaga Guerreiro Barboza,
Fábio Pereira de Miranda, Felipe Torres de Macedo.

Este documento descreve a implementação das duas primeiras etapas do
Projeto 1 — **Tipos Struct** —, conforme a tabela de especificação
(`docs/09-projects.md`):

| Etapa | Trabalho previsto |
|-------|--------------------|
| **Parser** | `struct Nome { campos }` como nova declaração de topo; literais de struct `Nome { campo: expr, ... }`; acesso a campo `expr.campo` em expressões e como alvo de atribuição |
| **Type checker** | registro global de structs; validação de campos em declarações, literais e atribuições; `Type::Struct` compatível consigo mesmo via `types_compatible` |

A terceira etapa (interpretador — avaliação de `StructLit`/`FieldAccess` e
atribuição de campo em tempo de execução) está fora do escopo deste PR e
será tratada separadamente.

---

## 1. Sintaxe alvo

```c
struct Point {
    int x;
    int y;
}

void main() {
    Point p = Point { x: 0, y: 1 };
    p.x = 42;
    print(p.x);
}
```

---

## 2. Parte 1 — Parser

### 2.1 Novos nós da AST (`src/ir/ast.rs`)

- `Type::Struct(String)` — novo variante de tipo, identificando um struct
  pelo nome (tipo nominal, não estrutural).

- `StructField = (String, Type)` e `StructDef`:

  ```rust
  pub type StructField = (String, Type);

  pub struct StructDef {
      pub name: String,
      pub fields: Vec<StructField>,
  }
  ```

- `Program<Ty>` ganhou o campo `structs: Vec<StructDef>`, ao lado do já
  existente `functions`.

- Dois novos variantes em `Expr<Ty>`:

  ```rust
  /// Literal de struct: `NomeDoStruct { campo: expr, ... }`
  StructLit {
      name: String,
      fields: Vec<(String, ExprD<Ty>)>,
  },
  /// Acesso a campo: `expr.campo`
  FieldAccess {
      base: Box<ExprD<Ty>>,
      field: String,
  },
  ```

### 2.2 Palavra reservada `struct` (`src/parser/identifiers.rs`)

`struct` foi adicionada a `RESERVED`, evitando que seja usada como nome de
variável/função e eliminando ambiguidade na hora de decidir se um item de
topo é uma definição de struct ou uma declaração de função.

### 2.3 Nomes de tipo struct (`src/parser/functions.rs`)

`type_name()` ganhou uma alternativa de **fallback**, tentada por último,
depois de todos os tipos primitivos (`int`, `float`, `bool`, `str`, `void`):

```rust
// Fallback: qualquer identificador não reservado é tratado como nome de
// tipo struct. O type checker valida se o nome corresponde a um struct
// definido.
map(identifier, |s: &str| Type::Struct(s.to_string())),
```

Isso permite que `Point x = ...;` seja parseado como declaração com
`ty = Type::Struct("Point")`, sem que o parser precise conhecer os structs
definidos — essa validação fica para a Parte 2.

### 2.4 Definição de struct no topo do programa (`src/parser/program.rs`)

O programa agora aceita, em qualquer ordem, definições `struct` e
declarações de função:

- `struct_field` reconhece `Tipo identifier ;` (mesmo padrão de terminação
  por `;` usado nas declarações dentro de funções):

  ```rust
  fn struct_field(input: &str) -> IResult<&str, (String, Type)> {
      map(
          tuple((
              preceded(multispace0, type_name),
              preceded(multispace1, identifier),
              preceded(multispace0, char(';')),
          )),
          |(ty, name, _)| (name.to_string(), ty),
      )(input)
  }
  ```

- `struct_def` reconhece `struct Nome { campo* }`:

  ```rust
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
  ```

- Um enum auxiliar `TopLevel` (`Struct` | `Fun`) permite que `many0` colete
  uma sequência heterogênea de itens de topo, na mesma linha do que foi
  feito para `test`/`fun_decl` no Projeto 8:

  ```rust
  enum TopLevel {
      Struct(StructDef),
      Fun(UncheckedFunDecl),
  }

  pub fn program(input: &str) -> IResult<&str, UncheckedProgram> {
      let (rest, items) = many0(preceded(
          multispace0,
          alt((
              map(struct_def, TopLevel::Struct),
              map(fun_decl, TopLevel::Fun),
          )),
      ))(input)?;

      let mut structs = Vec::new();
      let mut functions = Vec::new();
      for item in items {
          match item {
              TopLevel::Struct(s) => structs.push(s),
              TopLevel::Fun(f) => functions.push(f),
          }
      }
      Ok((rest, Program { structs, functions }))
  }
  ```

  `struct_def` é tentado primeiro: como `struct` é palavra reservada, uma
  tentativa que falha não consome nada de forma ambígua, então a ordem aqui
  é apenas organizacional.

### 2.5 Literais de struct em expressões (`src/parser/expressions.rs`)

- `struct_field_init` reconhece um inicializador `campo: expr`:

  ```rust
  fn struct_field_init(input: &str) -> IResult<&str, (String, UncheckedExpr)> {
      map(
          tuple((
              preceded(multispace0, identifier),
              preceded(multispace0, char(':')),
              preceded(multispace0, expression),
          )),
          |(name, _, val)| (name.to_string(), val),
      )(input)
  }
  ```

- `struct_lit` reconhece `NomeDoStruct { campo: expr, ... }`:

  ```rust
  fn struct_lit(input: &str) -> IResult<&str, UncheckedExpr> {
      let (rest, name) = preceded(multispace0, identifier)(input)?;
      // Só "compromete" se um '{' vier a seguir (depois de espaços opcionais).
      let (rest, _) = preceded(multispace0, char('{'))(rest)?;
      let (rest, fields) = separated_list0(
          preceded(multispace0, char(',')),
          struct_field_init,
      )(rest)?;
      let (rest, _) = opt(preceded(multispace0, char(',')))(rest)?;
      let (rest, _) = preceded(multispace0, char('}'))(rest)?;
      Ok((rest, wrap(Expr::StructLit { name: name.to_string(), fields })))
  }
  ```

  **Ponto crítico:** `struct_lit` é colocado **primeiro** na lista de
  alternativas de `atom()`, antes de `parse_call` e `identifier`. Se fosse
  tentado depois, `Point` seria consumido pelo ramo `identifier` e
  `{ x: 0 }` ficaria como entrada inesperada, fazendo o parse falhar. Como
  `struct_lit` só "compromete" depois de ver o `{`, ele falha rapidamente e
  sem consumir entrada quando o identificador não é seguido de `{`,
  permitindo o fallback normal para `identifier`/`parse_call`.

### 2.6 Acesso a campo em expressões e lvalues

- Em `primary()` (`src/parser/expressions.rs`), o laço que acumula
  sufixos `[ expr ]` (index) agora também tenta um sufixo `.campo`,
  produzindo `Expr::FieldAccess { base, field }`. Os dois sufixos podem
  ser encadeados (`m[0].x`, `p.inner.x`, etc. — a restrição de "apenas um
  nível" é imposta no type checker/atribuição, não no parser).

- Em `lvalue()` (`src/parser/statements.rs`), usado para o lado esquerdo
  de atribuições, o mesmo laço foi estendido da mesma forma, permitindo
  `p.x = 42;` ser parseado como
  `Assign { target: FieldAccess { base: Ident("p"), field: "x" }, value: ... }`.

---

## 3. Parte 2 — Type Checker (`src/semantic/type_checker.rs`)

### 3.1 Registro global de structs

No início de `type_check`, é construído um `HashMap<String, StructDef>` a
partir de `program.structs`, validando:

- **Definições duplicadas** — dois `struct` com o mesmo nome são rejeitados.
- **Campos `void`** — um campo não pode ter tipo `void`.
- **Tipos de campo desconhecidos** — se um campo referencia outro struct
  (`Type::Struct(sname)`), `sname` precisa já estar registrado.

```rust
let mut struct_defs: HashMap<String, StructDef> = HashMap::new();
for sd in &program.structs {
    if struct_defs.contains_key(&sd.name) {
        return Err(TypeError::new(format!("duplicate struct definition: {}", sd.name)));
    }
    for (fname, fty) in &sd.fields {
        if fty == &Type::Unit {
            return Err(TypeError::new(format!(
                "field '{}' in struct '{}' cannot have type void", fname, sd.name
            )));
        }
        if let Type::Struct(sname) = fty {
            if !struct_defs.contains_key(sname) {
                return Err(TypeError::new(format!(
                    "unknown struct type '{}' in field '{}' of struct '{}'",
                    sname, fname, sd.name
                )));
            }
        }
    }
    struct_defs.insert(sd.name.clone(), sd.clone());
}
```

Esse registro é **global e imutável**: construído uma única vez e passado
por referência (`&HashMap<String, StructDef>`) a todas as funções
auxiliares do type checker (`type_check_fun_decl`, `type_check_stmt`,
`type_check_expr`, `type_check_expr_inner`, `type_check_expr_to_typed`,
`type_check_assign_target`). Diferente do `Environment<Type>`, ele **não**
é snapshot/restaurado — não há escopo para definições de struct.

O `CheckedProgram` retornado preserva os structs:

```rust
Ok(Program { structs: program.structs.clone(), functions })
```

### 3.2 Declaração de variável com tipo struct

Em `Statement::Decl`, se o tipo declarado for `Type::Struct(name)`, o type
checker verifica que `name` está no registro:

```rust
if let Type::Struct(sname) = ty {
    if !struct_defs.contains_key(sname) {
        return Err(TypeError::new(format!("unknown struct type '{}'", sname)));
    }
}
```

A compatibilidade do valor inicializador com o tipo declarado segue o
mesmo `types_compatible` já usado para tipos primitivos (ver 3.5).

### 3.3 Literal de struct (`Expr::StructLit`)

Em `type_check_expr`, um `StructLit { name, fields }` é verificado assim:

1. `name` precisa existir no registro de structs.
2. O **número de campos** do literal precisa ser igual ao número de campos
   da definição.
3. Para cada campo do literal, o **nome do campo** precisa existir na
   definição, e o **tipo do valor** precisa ser compatível (via
   `types_compatible`, então `int` → `float` é aceito) com o tipo declarado
   do campo.
4. O resultado é `Type::Struct(name)`.

```rust
Expr::StructLit { name, fields } => {
    let sd = struct_defs.get(name)
        .ok_or_else(|| TypeError::new(format!("unknown struct type '{}'", name)))?;
    if fields.len() != sd.fields.len() {
        return Err(TypeError::new(format!(
            "struct '{}' has {} fields but literal provides {}",
            name, sd.fields.len(), fields.len()
        )));
    }
    for (fname, fexpr) in fields {
        let expected_ty = sd.fields.iter()
            .find(|(n, _)| n == fname)
            .map(|(_, t)| t)
            .ok_or_else(|| TypeError::new(format!("unknown field '{}' on struct '{}'", fname, name)))?;
        let actual_ty = type_check_expr(fexpr, env, struct_defs)?;
        if !types_compatible(&actual_ty, expected_ty) {
            return Err(TypeError::new(format!(
                "field '{}': expected {:?}, got {:?}", fname, expected_ty, actual_ty
            )));
        }
    }
    Ok(Type::Struct(name.clone()))
}
```

`type_check_expr_inner` apenas propaga o `StructLit`, type-checando
recursivamente o valor de cada campo (mantendo a mesma estrutura, agora com
`Ty = Type`).

### 3.4 Leitura de campo (`Expr::FieldAccess`)

```rust
Expr::FieldAccess { base, field } => {
    let base_ty = type_check_expr(base, env, struct_defs)?;
    match base_ty {
        Type::Struct(sname) => {
            let sd = struct_defs.get(&sname)
                .ok_or_else(|| TypeError::new(format!("unknown struct type '{}'", sname)))?;
            sd.fields.iter()
                .find(|(n, _)| n == field)
                .map(|(_, t)| t.clone())
                .ok_or_else(|| TypeError::new(format!("struct '{}' has no field '{}'", sname, field)))
        }
        other => Err(TypeError::new(format!("field access on non-struct type {:?}", other))),
    }
}
```

- A base precisa ter tipo `Type::Struct(sname)`.
- `sname` precisa estar no registro (sempre verdade se o valor foi
  construído via `StructLit` checado, mas a verificação é redundante e
  segura).
- O campo precisa existir na definição; o tipo retornado é o tipo
  declarado do campo.

### 3.5 Atribuição a campo (`p.x = 42;`)

`type_check_assign_target` ganhou um novo ramo para `Expr::FieldAccess`:

```rust
Expr::FieldAccess { base, field } => {
    // Só permite base Ident — sem atribuição em struct aninhado.
    if !matches!(base.exp, Expr::Ident(_)) {
        return Err(TypeError::new(
            "field assignment only supported on simple variables, not expressions",
        ));
    }
    let base_ty = type_check_expr(base, env, struct_defs)?;
    match base_ty {
        Type::Struct(sname) => {
            let sd = struct_defs.get(&sname)
                .ok_or_else(|| TypeError::new(format!("unknown struct type '{}'", sname)))?;
            let field_ty = sd.fields.iter()
                .find(|(n, _)| n == field)
                .map(|(_, t)| t)
                .ok_or_else(|| TypeError::new(format!("struct '{}' has no field '{}'", sname, field)))?;
            if !types_compatible(value_ty, field_ty) {
                return Err(TypeError::new(format!(
                    "field '{}': expected {:?}, got {:?}", field, field_ty, value_ty
                )));
            }
            Ok(())
        }
        other => Err(TypeError::new(format!("field assignment on non-struct type {:?}", other))),
    }
}
```

**Restrição deliberada:** apenas `identificador.campo = valor` é permitido.
`a.b.c = valor` (atribuição em struct aninhado) é rejeitado explicitamente
pelo type checker — essa limitação é compartilhada com o interpretador
(Parte 3), que só sabe atualizar um struct armazenado diretamente em uma
variável do ambiente.

### 3.6 `types_compatible` para structs

```rust
(Type::Struct(a), Type::Struct(b)) => a == b,
```

Tipos struct são **nominais**: `Type::Struct("Point")` só é compatível com
`Type::Struct("Point")`. Não há coerção entre structs diferentes, mesmo que
tenham os mesmos campos.

### 3.7 Encanamento (`struct_defs` propagado por todo o checker)

Todas as funções auxiliares que antes recebiam apenas
`env: &Environment<Type>` agora também recebem
`struct_defs: &HashMap<String, StructDef>`:

- `type_check_fun_decl`
- `type_check_stmt`
- `type_check_expr`
- `type_check_expr_inner`
- `type_check_expr_to_typed`
- `check_call` (via `type_check_expr_to_typed`, que já recebe `struct_defs`)
- `type_check_assign_target`

Essa é uma mudança puramente mecânica — cada chamada interna passa
`struct_defs` adiante — mas necessária para que a validação de structs
esteja disponível em qualquer ponto da árvore (uma declaração `Point p = ...`
pode estar dentro de um `if` dentro de um `while`, por exemplo).

---

## 4. Testes adicionados (`tests/type_checker.rs`)

| Teste | O que verifica |
|-------|------------------|
| declaração + literal de struct + leitura de campo | `Point p = Point { x: 0, y: 1 }; int x = p.x;` é aceito |
| atribuição de campo | `p.x = 42;` é aceito quando `p` é `Point` e `42` é compatível com `int` |
| tipo struct desconhecido em declaração | `Foo f = ...;` sem `struct Foo` definido é rejeitado |
| nome de campo errado no literal | `Point { z: 1 }` é rejeitado (`z` não existe em `Point`) |
| tipo de campo errado no literal | `Point { x: true, y: 1 }` é rejeitado (`x` é `int`) |
| acesso a campo em não-struct | `int x = 1; x.y` é rejeitado |
| definição de struct duplicada | dois `struct Point { ... }` geram erro |

Os testes existentes do parser e do type checker continuam passando sem
modificação — `Type::Struct`, `StructDef`, `StructLit` e `FieldAccess` são
adições puramente novas à AST.

---

## 5. Como executar

```bash
# Roda toda a suíte de testes (parser + type checker)
cargo test

# Verifica apenas o type checker
cargo test --test type_checker

# Verifica que um programa com structs passa pelo parser + type checker
cargo run -- --check tests/fixtures/struct_basic.minic
```

---

## 6. Próximos passos (Parte 3 — fora deste PR)

- `Value::Struct { name, fields: HashMap<String, Value> }` no interpretador.
- `eval_expr`: avaliar `Expr::StructLit` (construir o `Value::Struct`) e
  `Expr::FieldAccess` (ler um campo de um `Value::Struct`).
- `exec_stmt::assign_lvalue`: suportar `Expr::FieldAccess` com base
  `Ident`, clonando o struct do ambiente, atualizando o campo e regravando
  com `env.set`.
