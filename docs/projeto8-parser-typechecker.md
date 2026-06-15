# Projeto 8 — Framework de Testes Embutido (Parte 1 e Parte 2)

**Grupo:** Alberto Guevara de Araujo Franca, Davi Gonzaga Guerreiro Barboza,
Fábio Pereira de Miranda, Felipe Torres de Macedo.

Este documento descreve a implementação das duas primeiras etapas do
Projeto 8 — **Framework de Testes Embutido** —, conforme a tabela de
especificação (`docs/09-projects.md`):

| Etapa | Trabalho previsto |
|-------|--------------------|
| **Parser** | `test "nome" { stmts }` como nova declaração de topo; `assert expr;` como novo statement |
| **Type checker** | `assert` exige expressão `bool`; corpo de `test` é verificado como corpo de função sem tipo de retorno; detecção de nomes de teste duplicados |

A terceira etapa (interpretador / CLI `--test`) está fora do escopo deste PR
e será tratada separadamente.

---

## 1. Sintaxe alvo

```c
int add(int a, int b) { return a + b; }

test "soma esta correta" {
    assert add(2, 3) == 5;
    assert add(0, 0) == 0;
}

test "comparacao" {
    int x = 10;
    assert x > 5;
    assert x == 10;
}
```

Um programa MiniC válido agora pode conter, no nível de topo, qualquer
combinação de declarações de função (`fun_decl`) e blocos de teste
(`test_decl`), em qualquer ordem.

---

## 2. Parte 1 — Parser

### 2.1 Novos nós da AST (`src/ir/ast.rs`)

Para representar a nova sintaxe, foram adicionados:

- `Statement::Assert(Box<ExprD<Ty>>)` — representa `assert expr;`. Em tempo
  de execução, o interpretador deve abortar se a expressão avaliar para
  `false` (implementação na Parte 3).

- `TestDecl<Ty>` — uma nova struct paralela a `FunDecl<Ty>`, representando
  um bloco `test "nome" { ... }`:

  ```rust
  pub struct TestDecl<Ty> {
      pub name: String,
      pub body: Box<StatementD<Ty>>,
  }
  ```

- `Program<Ty>` foi estendido com um novo campo `tests: Vec<TestDecl<Ty>>`,
  ficando:

  ```rust
  pub struct Program<Ty> {
      pub functions: Vec<FunDecl<Ty>>,
      pub tests: Vec<TestDecl<Ty>>,
  }
  ```

- Foram adicionados os aliases `UncheckedTestDecl = TestDecl<()>` e
  `CheckedTestDecl = TestDecl<Type>`, seguindo o padrão já usado para
  `FunDecl`/`Program`.

### 2.2 Palavras reservadas (`src/parser/identifiers.rs`)

`assert` e `test` foram adicionadas à lista `RESERVED`, para que não possam
ser usadas como nomes de variáveis, funções ou parâmetros — evitando
ambiguidade na hora de decidir se um identificador inicia um statement
`assert`/declaração `test` ou é apenas um nome comum.

```rust
const RESERVED: &[&str] = &[
    "true", "false", "int", "float", "bool", "str", "void", "return", "assert", "test",
];
```

### 2.3 Novo statement `assert` (`src/parser/statements.rs`)

Foi criado o parser `assert_statement`, que reconhece `assert <expr> ;` e
produz `Statement::Assert`:

```rust
fn assert_statement(input: &str) -> IResult<&str, UncheckedStmt> {
    let (rest, _) = preceded(multispace0, tag("assert"))(input)?;
    let (rest, expr) = preceded(multispace0, expression)(rest)?;
    let (rest, _) = preceded(multispace0, char(';'))(rest)?;
    Ok((rest, wrap(Statement::Assert(Box::new(expr)))))
}
```

Esse parser foi incluído na lista de alternativas de `statement`, posicionado
logo após `return_statement` (mesma categoria de "statement simples
terminado por `;`"):

```rust
alt((
    block_statement,
    if_statement,
    while_statement,
    return_statement,
    assert_statement,   // <-- novo
    decl_statement,
    call_statement,
    assignment,
))
```

Como `assert` agora é palavra reservada, não há ambiguidade com declarações
de variável ou chamadas de função que comecem com identificadores parecidos.

### 2.4 Declaração de topo `test "nome" { ... }` (`src/parser/program.rs`)

O parser de programa foi reescrito para aceitar **dois tipos de item de
topo**: declarações de função (já existentes) e blocos de teste (novos).

- `test_decl` reconhece `test "<nome>" { <statements> }`:

  ```rust
  fn test_decl(input: &str) -> IResult<&str, UncheckedTestDecl> {
      let (rest, _) = preceded(multispace0, tag("test"))(input)?;
      let (rest, name) = preceded(
          multispace0,
          delimited(char('"'), take_while(|c| c != '"'), char('"')),
      )(rest)?;
      let (rest, body) = preceded(
          multispace0,
          map(
              delimited(
                  preceded(multispace0, char('{')),
                  many0(statement),
                  preceded(multispace0, char('}')),
              ),
              |seq| StatementD { stmt: Statement::Block { seq }, ty: () },
          ),
      )(rest)?;
      Ok((rest, TestDecl { name: name.to_string(), body: Box::new(body) }))
  }
  ```

  O corpo do teste é representado internamente como um `Statement::Block`,
  reaproveitando toda a lógica de escopo de blocos já existente no type
  checker e (futuramente) no interpretador.

- Um enum auxiliar `TopItem` (`Fun` | `Test`) permite que `many0` colete uma
  sequência heterogênea de itens de topo:

  ```rust
  enum TopItem {
      Fun(UncheckedFunDecl),
      Test(UncheckedTestDecl),
  }

  fn top_item(input: &str) -> IResult<&str, TopItem> {
      preceded(
          multispace0,
          alt((
              map(test_decl, TopItem::Test),
              map(fun_decl, TopItem::Fun),
          )),
      )(input)
  }
  ```

  `test_decl` é tentado **antes** de `fun_decl` porque `test` é palavra
  reservada — não há risco de um `fun_decl` válido começar com `test`, então
  a ordem aqui é só uma questão de organização.

- `program` agora particiona os itens coletados em `functions` e `tests`:

  ```rust
  pub fn program(input: &str) -> IResult<&str, UncheckedProgram> {
      map(many0(top_item), |items| {
          let mut functions = Vec::new();
          let mut tests = Vec::new();
          for item in items {
              match item {
                  TopItem::Fun(f) => functions.push(f),
                  TopItem::Test(t) => tests.push(t),
              }
          }
          Program { functions, tests }
      })(input)
  }
  ```

Com isso, um arquivo `.minic` pode misturar livremente funções e blocos de
teste, em qualquer ordem, exatamente como no exemplo da seção 1.

---

## 3. Parte 2 — Type Checker (`src/semantic/type_checker.rs`)

### 3.1 `assert` exige `bool`

Foi adicionado um novo ramo em `type_check_stmt` para `Statement::Assert`:
a expressão é verificada normalmente (com `type_check_expr_to_typed`) e seu
tipo precisa ser exatamente `Type::Bool`, caso contrário o type checker
rejeita o programa com uma mensagem explícita:

```rust
Statement::Assert(expr) => {
    let checked = type_check_expr_to_typed(expr, env)?;
    if checked.ty != Type::Bool {
        return Err(TypeError::new(format!(
            "assert requires Bool, got {:?}",
            checked.ty
        )));
    }
    Statement::Assert(Box::new(checked))
}
```

### 3.2 Verificação do corpo de `test "..."`

Foi adicionada a função `type_check_test_decl`, que verifica o corpo de um
bloco `test` exatamente como o corpo de uma função `void` (sem valor de
retorno esperado):

```rust
fn type_check_test_decl(
    t: &UncheckedTestDecl,
    env: &mut Environment<Type>,
    fn_snapshot: &HashMap<String, Type>,
) -> Result<CheckedTestDecl, TypeError> {
    env.restore(fn_snapshot.clone());
    let body = type_check_stmt(&t.body, env, &Type::Unit)?;
    Ok(TestDecl { name: t.name.clone(), body: Box::new(body) })
}
```

- `env.restore(fn_snapshot.clone())` garante que cada teste começa com um
  ambiente "limpo", contendo apenas as assinaturas de funções (stdlib +
  funções do programa) — sem variáveis vazadas de outro teste ou função.
- `expected_return = Type::Unit` reaproveita a mesma regra usada por
  funções `void`: `return;` é permitido, `return <expr>;` não é.
- O resultado é um `CheckedTestDecl`, com o corpo totalmente anotado com
  tipos — assim como `CheckedFunDecl`.

`type_check` foi atualizado para percorrer `program.tests` da mesma forma
que já percorria `program.functions`, produzindo o novo campo
`Program { functions, tests }`:

```rust
let mut tests = Vec::new();
for t in &program.tests {
    let checked = type_check_test_decl(t, &mut env, &fn_snapshot)?;
    tests.push(checked);
}

Ok(Program { functions, tests })
```

### 3.3 `main` deixa de ser obrigatório quando há `test`s

Antes, `type_check` exigia que `main` existisse incondicionalmente. Como o
modo `--test` (Parte 3) **não invoca `main`**, um arquivo composto apenas
por blocos `test` (sem `main`) precisa ser um programa válido. A regra foi
ajustada para:

- Se `main` existir, ela continua precisando ser `void main()` (sem tipo de
  retorno e sem parâmetros) — mesma verificação de antes.
- `main` só é **obrigatória** se o programa não tiver nenhum bloco `test`.

```rust
if let Some(f) = program.functions.iter().find(|f| f.name == "main") {
    if f.return_type != Type::Unit {
        return Err(TypeError::new("main function must return void"));
    }
    if !f.params.is_empty() {
        return Err(TypeError::new("main function must have no parameters"));
    }
}
if program.tests.is_empty() && !program.functions.iter().any(|f| f.name == "main") {
    return Err(TypeError::new("program must have a main function"));
}
```

### 3.4 Nomes de teste duplicados

Para evitar ambiguidade nos relatórios de `--test` (Parte 3), o type checker
detecta nomes de teste repetidos **antes** de verificar qualquer corpo,
usando um `HashSet`:

```rust
let mut seen_tests = std::collections::HashSet::new();
for t in &program.tests {
    if !seen_tests.insert(&t.name) {
        return Err(TypeError::new(format!("duplicate test name: \"{}\"", t.name)));
    }
}
```

---

## 4. Testes adicionados (`tests/type_checker.rs`)

| Teste | O que verifica |
|-------|------------------|
| `test_type_check_assert_bool_ok` | `assert true;` é aceito |
| `test_type_check_assert_non_bool_err` | `assert 42;` é rejeitado (mensagem contém `"Bool"`) |
| `test_type_check_test_block_ok` | `test "t" { assert true; }` é aceito |
| `test_type_check_test_block_no_main_required` | um arquivo só com `test` (sem `main`) passa o type check |
| `test_type_check_duplicate_test_name_err` | dois `test "foo" { ... }` geram erro contendo `"duplicate"` |
| `test_type_check_test_block_bad_assert_type` | `assert 1 + 1;` (tipo `Int`, não `Bool`) dentro de um `test` é rejeitado |

Todos os testes existentes do parser e do type checker continuam passando
sem modificação — as mudanças são estritamente aditivas (novas variantes de
AST e novos ramos de `match`).

---

## 5. Como executar

```bash
# Roda toda a suíte de testes (parser + type checker)
cargo test

# Verifica apenas o type checker
cargo test --test type_checker

# Verifica que um arquivo com test/assert passa pelo parser + type checker
cargo run -- --check tests/fixtures/test_framework.minic
```

---

## 6. Próximos passos (Parte 3 — fora deste PR)

- Interpretador: executar `Statement::Assert`, abortando com `RuntimeError`
  quando a expressão for `false`.
- `interpreter::run_tests`: executar cada `TestDecl`, capturando falhas de
  asserção e imprimindo `PASS`/`FAIL` por teste, com resumo final.
- CLI: nova flag `--test`, que roda `run_tests` em vez de `interpret`
  (sem invocar `main`).
