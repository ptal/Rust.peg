# Learn Oak

This section is devoted to introduce smoothly the different PEG combinators through a tutorial presenting `Calc`: a small language with arithmetic expressions and variable bindings. If you want to test the code while reading this tutorial, a skeleton project is available in the chapter [Getting Started](getting-started.md). This tutorial is split into several sections:

* [What is parsing?](learn-oak.html#what-is-parsing?)
* [Syntactic atoms of `Calc`](learn-oak.html#syntactic-atoms-of-calc)
* [Generated code and runtime](learn-oak.html#generated-code-and-runtime)
* [Semantic action](learn-oak.html#semantic-action)
* [Choice combinator](learn-oak.html#choice-combinator)
* [Sequence combinator](learn-oak.html#sequence-combinator)
* [Operator precedence](learn-oak.html#operator-precedence)
* [Syntactic predicates](learn-oak.html#syntactic-predicates)
* [Spacing](learn-oak.html#spacing)
* [Identifier and keyword](learn-oak.html#identifier-and-keyword)
* [Operator associativity](learn-oak.html#operator-associativity)
* [Conclusion](learn-oak.html#conclusion)
* [Exercises](learn-oak.html#exercises)

Before diving into the details, we present a program written in `Calc`:

```rust
let a = 10 - 2 in
let b = a / 2 in
b^2 + 2 * (1 - a)
```

It declares two local variables `a` and `b` initialized with arithmetic expressions and usable within the scope of the let-binding, which is everything after the `in`.
Let-bindings can be composed in cascade but must terminate with an arithmetic expression, such as `b^2 + 2 * (1 - a)` in our example.

### What is parsing?

A parser is a bridge between meaning-less sequence of characters and structured representation of data.
It tries to give meaning to a sequence of raw characters by constructing an *Abstract Syntax Tree* (AST) that will be processed by subsequent compilation phases.
We expect a parser to transform `7 - 1` into a structure such as `Subtraction(i32, i32)`.
As a side note, you should avoid to compute the actual result of `7 - 1` in the parsing step, it works for simple language but it tends to entangle syntactic and semantic analysis for more realistic grammar.
Invalid programs such as `let a = 8 in a * b` should be correctly parsed while the semantic analysis will be responsible for detecting that `b` is undeclared.

This tutorial will not cover the semantic analysis part and we will only describe the grammar used for parsing `Calc`.
Our parser will thus produce an AST but without evaluating the expression.

### Syntactic atoms of `Calc`

When it comes to elaborating a grammar, we usually start by identifying atoms of the language, e.g., syntactic constructions that can not be divided into smaller ones.
These atoms are called *tokens* and are often processed during a *lexical analysis* happening before the parsing.
Oak is based on _Parsing Expression Grammar_ (PEG) and works directly on a stream of characters instead of a stream of tokens.
An advantage is to have a unique and coherent grammar syntax which is helpful for composing grammars that do not necessarily expect the same set of tokens.
Before you continue reading, try to find out what are the atoms of `Calc`.

The keywords `let` and `in`, the binding operator `=`, parenthesis `()` and arithmetic operators `+`, `-`, `*`, `/`, `^` form the *unvalued atoms* of the language.
`Calc` has two *valued atoms* which are identifiers and numbers.
Unvalued atoms give a shape to the AST but they do not carry any specific data retrieved from the stream of characters.
The following grammar parses the atoms of `Calc`:

```rust
oak! {
  #![show_api]

  let_kw = "let"
  in_kw = "in"
  bind_op = "="
  add_op = "+"
  sub_op = "-"
  mul_op = "*"
  div_op = "/"
  exp_op = "^"
  lparen = "("
  rparen = ")"

  identifier = ["a-zA-Z0-9_"]+
  number = ["0-9"]+
}
```

A grammar is introduced with the macro `oak!`.
A grammar is a set of rules of the form `<name> = <expr>` where `<name>` is the rule name and `<expr>` a parsing expression.

The rules describing keywords and operators use *string literals* expressions of the form `"<literal>"`, it expects the input to match exactly the sequence of characters given.

Identifiers and numbers are recognized with *character classes* where a class is a single character or a character range.
A range `r` has the form `<char>-<char>` inside a set `["r1r2..rN"]`.
Since `-` is used to denote a range, it must be placed before or after all the ranges such as in `["-a-z"]` to be recognized as an accepted character.
Character classes will succeed and "eat" *one* character if it is present in the set, so `b`, `8`, `_` are all accepted by `["a-zA-Z0-9_"]` but `é`, `-` or `]` are not.

For both string literals and character classes, any Unicode characters are interpreted following the same requirements as [string literals](https://doc.rust-lang.org/reference/tokens.html#string-literals) in the Rust specification.
The only other parsing expression consuming a character is the expression `.` (a simple dot), it consumes any character and can only fail if we reached the end of input.

The remaining parsing expressions are combinators, they must be composed with sub-expressions.
Identifiers and numbers are sequences of one or more characters and we use the combinator `e+` to repeat `e` while it succeeds. For example `identifier` matches "x_1" from the input "x_1 x_2" by successively applying `["a-zA-Z0-9_"]` to the input; it parses `x`, `_` and `1` and then fails on the space character.
As it read one or more character, the expression `["a-zA-Z0-9_"]` succeeds, although the match is partial, and `identifier` returns the remaining input " x_2" and the data read.
A requirement of `e+` is that `e` must be repeated *at least once*.
The `e*` expression does not impose this constraint and allows `e` to be repeated *zero or more times*.
The last combinator in this category is `e?`, it consumes `e` *zero or one time*.
The combinators `e*`, `e+` and `e?` will consume as much input as they can and are said to be *greedy operators*.

### Generated code and runtime

Before explaining the other combinators, we get a glimpse at the generated code and how to use it.
Oak will generate two functions per rule, a *recognizer* and a *parser*.
A recognizer only matches the input against a specific rule and does not build any value from it.
A parser matches and builds the corresponding AST, possibly with the help of user-specific functions called *semantic actions*.
For example, the functions `parse_identifier` and `recognize_identifier` will be generated for the rule `identifier`.

We can observe the generated code of the macro using `rustfmt` and `cargo expand`:

```sh
rustup component add rustfmt
cargo install cargo-expand
```

By typing `cargo expand`, we obtain the following generated code for the `Calc` grammar:

```rust
// `ParseState` and `CharStream` should be prefixed by `oak_runtime::`.
// It is removed from this snippet for clarity.

#[inline]
pub fn recognize_identifier<'a>(mut state: oak_runtime::ParseState<Stream<'a>, ()>)
  -> oak_runtime::ParseState<Stream<'a>, ()> { /* ... */ }

#[inline]
pub fn parse_identifier<'a>(mut state: oak_runtime::ParseState<Stream<'a>, ()>)
  -> oak_runtime::ParseState<Stream<'a>, Vec<char>> { /* ... */ }

// ...
// Rest of the output truncated for the tutorial.
```

You can also retreive the code of a single function by typing `cargo expand parse_identifier`.

We can already use these functions in our main:

```rust
fn main() {
  let let_kw = "let";
  let state = recognize_let_kw(let_kw.into_state());
  assert!(state.is_successful());

  let ten = "10";
  let state = parse_number(ten.into_state());
  assert_eq!(state.unwrap_data(), vec!['1', '0']);
}
```

Before continuing, you should know that a [documentation of the runtime](https://docs.rs/oak_runtime/) is available, however be aware that it also contains functions and structures used by the generated code that you will probably not need.

The main runtime data structure is [`ParseState<S, T>`](https://docs.rs/oak_runtime/latest/oak_runtime/parse_state/struct.ParseState.html) where `S` is the type of the underlying data being parsed, called the _stream_, and `T` the type of the AST being constructed.
`T` is always equal to `()` for recognizer functions as they do not build an AST, as shown in the type signature of `recognize_identifier` above.
As for `parse_identifier`, it returns an AST of type `Vec<char>` which is the expected AST type produced by this rule.
A stream must implement several operations given in the module [`stream`](https://docs.rs/oak_runtime/latest/oak_runtime/stream/index.html).
You can find a list of all types implementing `Stream` in the [implementors list of `Stream`](https://docs.rs/oak_runtime/latest/oak_runtime/stream/trait.Stream.html), and it is also possible to implement `Stream` for your own type.
A parse state is created by calling `into_state` on a type implementing the trait `Stream`.
For example, the trait `IntoState` is implemented for the type `&'a str` and we can directly pass the result of `into_state()` to the parsing function, as in:

```rust
recognize_let_kw(let_kw.into_state())
```

A state indicates if the parsing was successful, partial or erroneous.
It carries information about which item was expected next and the AST built from the data read.
Convenient functions such as `unwrap_data()` or `is_successful()` are available directly from [`ParseState`](https://docs.rs/oak_runtime/latest/oak_runtime/parse_state/struct.ParseState.html).
A more complete function is `into_result()` which transforms the state into a type [ParseResult](https://docs.rs/oak_runtime/latest/oak_runtime/parse_state/enum.ParseResult.html) that can be pattern matched.
Here a full example:

```rust
fn analyse_state(state: ParseState<StrStream, Vec<char>>) {
  use oak_runtime::parse_state::ParseResult::*;
  match state.into_result() {
    Success(data) => println!("Full match: {:?}", data),
    Partial(data, expectation) => {
      println!("Partial match: {:?} because: {:?}", data, expectation);
    }
    Failure(expectation) => {
      println!("Failure: {:?}", expectation);
    }
  }
}

fn main() {
  analyse_state(parse_number("10".into_state())); // complete
  analyse_state(parse_number("10a".into_state())); // partial
  analyse_state(parse_number("a".into_state())); // erroneous
}

// Result:

// Full match: ['1', '0']
// Partial match: ['1', '0'] because: 1:3: unexpected `a`, expecting `["0-9"]`.
// Failure: 1:1: unexpected `a`, expecting `["0-9"]`.
```

The function `analyse_state` shows how to examine the result of a state, however if you just need to debug the result, `ParseResult` implements `Debug` so you can use the more generic `println("{:?}", state.into_result())` statement to obtain a similar result.

Congratz, you are now able to efficiently use the code generated by Oak!

### Semantic action

As you probably noticed, the rule `number` produces a value of type `Vec<char>` which is not a usable representation of a number.
We must transform this value into a better type such as `u32`.
To achieve this goal, we use a *semantic action* which gives meaning to the characters read.
A semantic action takes the value produced by an expression as arguments, and builds a type more suited for further processing.
The grammar becomes:

```rust
use std::str::FromStr;

oak! {
  // ... previous rules truncated.

  identifier = ["a-zA-Z0-9_"]+ > to_string
  number = ["0-9"]+ > to_number

  fn to_string(raw_text: Vec<char>) -> String {
    raw_text.into_iter().collect()
  }

  fn to_number(raw_text: Vec<char>) -> u32 {
    u32::from_str(&*to_string(raw_text)).unwrap()
  }
}
```

The combinator `e > f` expects a parsing expression on the left and a function name on the right, it works like a "reverse function call operator" in the sense that `f` is called with the result value of `e`.
For instance, `["a-zA-Z0-9_"]+ > to_string` is equivalent to calling `to_string` on the AST produced by the left-hand side expression `["a-zA-Z0-9_"]+`.
If the semantic action is a Rust function provided inside the procedural macro, then Oak will figure out the return type of the function automatically.
Note that almost any Rust code is accepted inside the macro.
Equivalently, we can declare `to_string` outside of the procedural macro as follows:

```rust
use std::str::FromStr;

fn to_string(raw_text: Vec<char>) -> String {
  raw_text.into_iter().collect()
}

oak! {
  // ... previous rules truncated.

  identifier: String = ["a-zA-Z0-9_"]+ > to_string

  // or equivalently:
  identifier = (["a-zA-Z0-9_"]+ > to_string):String
}
```

In that case, we must annotate the rule `identifier`, or the expression, with its type `String`.

Oak gives a type to any parsing expression to help you constructing your AST more easily.
Next chapters explain how Oak gives a type to expressions and how you can help Oak to infer better types.
For the moment, when you want to know the type of an expression, just creates a rule `r = e`, and check the type of the rule using `cargo expand parse_r`.
Note that a tuple type such as `(T, U)` is automatically unpacked into two function arguments, so we expect a semantic action function to be of type `f(T, U)` and not `f((T, U))`.

Finally, note that semantic actions have the property of not being called inside recognizers since they do not build an AST.

### Choice combinator

We can now build another part of our language: a simple arithmetic calculator where operands can be numbers, variables or a parenthesized expression. We extend the grammar with a `factor` rule:

```rust
use self::Expression::*;

pub type PExpr = Box<Expression>;

pub enum Expression {
  Variable(String),
  Number(u32)
}

oak! {
  // ... previous rules and code truncated.

  factor: PExpr
    = number > box Number
    / identifier > box Variable
}
```

A new combinator has appeared!
An operand can be a `number` _or_ an `identifier` (for variables); these two possibilities are expressed with the *choice combinator* of the form `e1 / e2 / ... / eN`.
It tries the expression `e1` and if it fails, it restarts with `e2`, etc.
It fails if the last expression `eN` fails.
An important point is that *order matters*, hence the grammar is unambiguous, for each input, only one parse tree is possible.
It's worth mentioning that this prioritized choice can lead to unexpected, but however easy to detect, wrong behaviour.
For example, if you consider `identifier / number` which reverses the order of the factors, `number` will never be reached because `identifier` accepts a super-set of the language recognized by `number`.
Choice combinators naturally map to an enumeration type in Rust, in our example we defined the `Expression` enumeration.

There are two new features demonstrated in the semantic action `box Number` and `box Variable`.
Firstly, Oak allows us to call the variant of an enumeration as a semantic action, for instance, `number > Number` will construct the value `Number(x)` where `x` is the AST value returned by `number`.
Secondly, to avoid creating helper functions, we allow the box syntax to encapsulate the result in a `Box` type.
Here, `number > box Number` is similar to the following, more verbose, code:

```rust
factor
  = number > number_expr
  / identifier > box Variable

fn number_expr(value: u32) -> PExpr {
  Box::new(Number(value))
}
```

### Sequence combinator

Now, we have all the necessary pieces to parse our first arithmetic expression.
We start with `+` and `-` because they have the same precedence, next we will add `*` and `/`.
The sequence combinator is required to parse a sequence of two or more expressions and is denoted as `e1 e2 ... eN`.
If `e1` succeeds, then `e2` is called and so on until `eN` succeeds.
It fails if any `e` fails, this is the main difference with the choice combinator which fails only if the last expression fails.
Let's give a look to the new grammar:

```rust
use self::Expression::*;
use self::BinOp::*;

pub type PExpr = Box<Expression>;

pub enum Expression {
  Variable(String),
  Number(u32),
  BinaryExpr(BinOp, PExpr, PExpr)
}

pub enum BinOp {
  Add, Sub
}

oak! {
  // ... previous rules and code truncated.

  expression
    = factor (term_op factor)* > fold_left

  term_op: BinOp
    = add_op > Add
    / sub_op > Sub

  fn fold_left(head: PExpr, rest: Vec<(BinOp, PExpr)>) -> PExpr {
    rest.into_iter().fold(head,
      |accu, (op, expr)| Box::new(BinaryExpr(op, accu, expr)))
  }
}
```

Parsing rules for arithmetic expression are usually written with *left recursion* which would give us a rule such as:

```rust
expression
  = factor
  / expression term_op factor
```

PEGs are closer to the generated code than context-free language specifications, for example the choice combinator is prioritized, which is similar to nested *if-then-else* statements in hand-written recursive descent parser.
This is why left recursion leads to infinite loops (and eventually to stack overflow) in PEG implementation while it is nicely handled in other parser generators.
Therefore, Oak does not support left recursion so the grammar above will generate a compilation error.
Instead, we write the `expression` rule without left recursion which is made possible with the repetition combinator `e*` expression instead of recursive rules.

Due to the lack of left recursion, the resulting AST is flatten into a type `(PExpr, Vec<(BinOp, PExpr)>)` which is not convenient to manipulate during subsequent compilation phases.
A problem with this representation is that operator associativity is not directly encoded inside the AST, which is error-prone because the subsequent phases in the compiler must be aware of associativity.
This is why we use the function `fold_left` to create a binary tree from this list.

### Operator precedence

Generally, a programming language has multiple operators that do not share the same precedence.
It is the case for a simple arithmetic expression where `*` and `/` take precedence over `+` and `-`.
We show the grammar for `Calc` basic arithmetic expressions and then expose how to write such rules in the general case.

```rust
pub enum BinOp {
  Add, Sub, Mul, Div
}

oak! {
  // ... previous rules and code truncated.

  expression
    = term (term_op term)* > fold_left

  term
    = factor (factor_op factor)* > fold_left

  factor_op: BinOp
    = mul_op > Mul
    / div_op > Div
}
```

We added support for multiplication and division with the `term` rule separating factors by `*` or `/`.
Note that we re-use the same function `fold_left` for transforming the expression list into a binary tree.
We show how precedence is encoded into these rules by computing step by step the parsing of the `Calc` expression `8-2/2`.

1. We enter `expression` and directly call `term` which in turn call `factor`.
2. We enter `factor` and try the rule `number` which succeeds. `factor` returns `Number(8)`.
3. We go back in `term` and try `(factor_op factor)*` but `factor_op` does not match `-` so `e*` produces an empty `Vec` and `fold_left` returns the first and unchanged value `Number(8)`.
4. We go back in `expression` and try `(term_op term)*`, `term_op` matches `-` and returns `Sub`.
5. We re-enter `term` and since the remaining input is `2/2`, it exactly matches the expression `factor factor_op factor` and returns `BinaryExpr(Div, Number(2), Number(2))`.
6. We go back in `expression` and build the final expression `BinaryExpr(Sub, Number(8), BinaryExpr(Div, Number(2), Number(2)))`.

This expression respects the precedence of arithmetic operators.
A general technique to build a PEG supporting any level of precedence is to nest rules in the invert order of precedence.
For example in `Calc`, numbers and variables have the highest precedence; note that this is always the case for atoms.
Addition and subtraction have the lowest precedence and it implies that, for `e1+e2`, both sub-expressions will first be considered to be terms or factors before trying to parse them as expressions.
We suggest that you first group operators by precedence levels and than write the expression rules:

```rust
operators_lvl_1 = "+" / "-"
operators_lvl_2 = "*" / "/"
// ...
operators_lvl_n = "-" // unary minus operator

expr_lvl_1 = expr_lvl_2 (operators_lvl_1 expr_lvl_2)*
expr_lvl_2 = expr_lvl_3 (operators_lvl_2 expr_lvl_3)*
// ...
expr_lvl_n = operators_lvl_n atom
```

You can freely adapt this template for any level of precedence in your grammar and add the corresponding semantic actions.

### Syntactic predicates

Our grammar already parse simple arithmetic expression, we now improve the rule for identifiers.
For the moment, `98a` is a valid identifier because we stated that identifiers are parsed with `["a-zA-Z0-9_"]+`.
Similarly to classic programming languages, we would like to forbid a digit to start an identifier.
We can achieve that with the combinators we have already seen:

```rust
oak! {
  // ... previous rules and code truncated.

  identifier = ["a-zA-Z_"] ["a-zA-Z0-9_"]* > to_string_2

  fn to_string_2(head: char, mut raw_text: Vec<char>) -> String {
    raw_text.push(head);
    to_string(raw_text)
  }
}
```

It works but seems redundant and does not expressed very well the intention of the grammar writer.
Moreover, it is not clear at a first sight that `"0-9"` is missing in the first character class.
Further, the value produced is split into a pair which is less friendly to use than a single vector.
Thanks to the syntactic predicate `!`, we can indicate that a parsing sequence must not start with a digit:

```rust
oak! {
  // ... previous rules and code truncated.

  identifier = !digit ["a-zA-Z0-9_"]+ > to_string
  number = digit+ > to_number
  digit = ["0-9"]

  fn to_string(raw_text: Vec<char>) -> String {
    raw_text.into_iter().collect()
  }
```

The syntactic predicate `!e` succeeds if `e` fails, and in any case, *it does not consume input*.
Its dual combinator is `&e` which succeeds if `e` succeeds and is a short-cut for `!!e`.
It can be thought as a `if` statement which executes the next combinator only if the condition `!e` or `e` is true.
It is very useful to look-ahead in the buffer without consuming it.
For example, we can use the expression `!.` to check that we are at the end of file, remember that `.` succeeds if it consumes any single character.
It is useful to forbid partial matching directly in the grammar specification instead of consulting the result value.

### Spacing

Spacing is traditionally processed by a lexer, executed before the parsing phase, which transforms a character stream into a token stream where blank characters are removed.
As said before, PEG works directly on the character stream so we must manage spaces ourselves.
The following grammar is equipped with spacing.

```rust
oak! {
  // ... previous rules and code truncated.

  program = spacing expression

  identifier = !digit ["a-zA-Z0-9_"]+ spacing > to_string
  number = digit+ spacing > to_number

  spacing = [" \n\r\t"]*:(^)

  let_kw = "let" spacing
  in_kw = "in" spacing
  bind_op = "=" spacing
  add_op = "+" spacing
  sub_op = "-" spacing
  mul_op = "*" spacing
  div_op = "/" spacing
  exp_op = "^" spacing
  lparen = "(" spacing
  rparen = ")" spacing
}
```

The idea is to make sure that blank characters are consumed before the parsing of an atom (such as `"let"` or `["a-zA-Z0-9_"]`).
Since only atoms can consume the stream, we need to surround them with the `spacing` rule such as in `spacing "let" spacing`.
However, for two atoms `a1 a2`, the `spacing` rule will be called twice between `a1` and `a2`.
We can do better with a new rule `program` that first call `spacing` and then `expression`.
It guarantees that the very first blank characters will be consumed.
Therefore, atoms only need to consume trailing blank characters.

In `spacing`, the expression `[" \n\t"]*` has type `Vec<char>`, but we do not really care about this value.
This is why Oak proposes the *invisible type* annotation combinator `e:(^)` to indicate that we do not care about the value of an expression `e` and should be "invisible" in the AST.
Oak will automatically propagate `(^)` to the calling site, for example, tuple like `((^), char)` are automatically reduced to `char`.
There is much more to say about types and it is not part of PEG itself, so we discuss about it in the [typing expression](typing-expression.md) chapter.

### Identifier and keyword

Now we have a grammar for arithmetic expressions.
We continue by adding the let-in construction for declaring new variables.
It has the form `let <ident> = <expression> in <expression>` and is parsed by the following grammar.

```rust
oak! {
  // ... previous rules and code truncated.

  factor: PExpr
    = number > box Number
    / identifier > box Variable
    / let_expr > box LetIn
    / lparen expression rparen

  let_expr = let_kw let_binding in_kw expression
  let_binding = identifier bind_op expression
}
```

There is no new concept in this grammar, we have already seen all the combinators used.
However it does not work as expected for programs containing let-in expressions.
For example, it partially matches `let x = 1 in x` and the data returned is `Variable("let")`.
It does not work because `identifier` is parsed before `let_expr` in `factor`, so `"let"` is recognized as a valid identifier.
There is clearly some overlapping between the language accepted by identifiers and keywords.
It does not help to inverse the order of both rules because the identifier would successfully parse the `"let"` keyword if the `let_expr` rule fails.

This is a problem specific to PEG due to its combined lexical and parsing analysis.
Disambiguation is usually done by the lexer with an ad-hoc keyword table; if a string is present in the table, the corresponding token is returned, otherwise it is considered as an identifier.
In PEG, we encode this difference directly in the rules with syntactic predicates as follows:


```rust
grammar! calc {
  // ... previous rules and code truncated.

  identifier = !digit !keyword ident_char+ spacing > to_string
  ident_char = ["a-zA-Z0-9_"]

  kw_tail = !ident_char spacing

  keyword = let_kw / in_kw
  let_kw = "let" kw_tail
  in_kw = "in" kw_tail
}
```

We ensure that a keyword rule never accept the beginning of a valid identifier, and conversely, we forbid an identifier to be a valid keyword.
The first is done with `kw_tail` which prevents a valid identifier character (`ident_char`) to follow a keyword.
It must be appended to every keyword or, more generally, to atom recognizing a subset of the identifiers.
Instead of the keyword table used in a lexer, we use the rule `keyword` accepting every keyword of the language and we explicitly prevent an identifier to start with a keyword (see `!keyword`).

### Operator associativity

For now, `Calc` only contains left-associative operators and the corresponding AST is built with the `fold_left` function. It is pretty simple to transform an operator separated-list of expression to its right-associative version if we use a `fold_right` function. We extend the `Calc` grammar with the exponent operator `e1 ^ e2` which is right-associative and takes precedence over `term` expressions.

```rust
pub enum BinOp {
  Add, Sub, Mul, Div, Exp
}

oak! {
  // ... previous rules and code truncated.

  term
    = exponent (factor_op exponent)* > fold_left

  exponent
    = (factor exponent_op)* factor > fold_right

  exponent_op: BinOp = exp_op > Exp

  fn fold_right(front: Vec<(PExpr, BinOp)>, last: PExpr) -> PExpr {
    front.into_iter().rev().fold(last,
      |accu, (expr, op)| Box::new(BinaryExpr(op, expr, accu)))
  }
```

A simple trick for right-folding is to reverse the list and to left fold with the accumulator being the last element. It would be correct to write the rule `exponent` as `factor (exponent_op factor)*` but since we need the last element for right-folding, we would do unnecessary work in the semantic action. Therefore, it is better to directly write the rule in an adapted way for right-folding.

To summarize, operator associativity is managed by the semantic actions and not directly in the parsing expressions. Generic left and right folding functions can be used to create a binary tree for expressions with left or right associative operators.

### Conclusion

That's it! We built a complete grammar for a small language encompassing arithmetic expressions and variable bindings. This tutorial should have covered most of the useful techniques to write your own grammar. The full grammar and usage examples of the `Calc` language are available in the [next chapter](full-calc-grammar.md). If you want to use the most of Oak capabilities, please read-on and learn how Oak gives types to parsing expressions!

### Exercises

* Extend the grammar to support negative numbers.
* Extend the grammar to support declaration and function call.
