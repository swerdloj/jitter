# Jitter: **J**ust-**I**n-**T**ime **R**ust
Jitter is a just-in-time (JIT) compiled scripting language designed with Rust interoperability in mind.

## Disclaimer
Jitter evolved into a research project investigating the ideas of:
1. Treating a compiler as a library
2. Language extension mechanisms

Therefore, Jitter is a research language and not intended for everyday use. **Many features are non-existant or broken**. However, Jitter may be further evolved into a functional language in the future.

Because the focus of Jitter is language extension mechanisms, arithmetic operations are not yet implemented.

The research/white paper associated with Jitter is currently in progress.

---

## How does Jitter compare to Rust?
### *Syntax*
Much of Jitter's syntax comes directly from Rust. This includes everything from pattern matching to keywords.

Ideally, Jitter would improve the syntax wherever possible such as allowing `.Variant` instead of Rust's `path::EnumType::Variant`

### *Why not use Rust instead?*
Rust is rather strict and for good reason. These reasons, however, do not necessarily apply to scripting languages.

Thus, Jitter does not aim to be fast, safe, or low-level. Instead, it seeks to promote *good* design patterns while being easy to work with and providing functionality not found in traditional languages.

---

## Motivations
Jitter's primary goal is to be an embedded scripting language that is fast to compile, fast at runtime, and all-around easy to use.

Jitter aims to be to Rust what Lua is to C/C++.  
Additionally, Jitter is fully compatible with C and C++.

---

## Future Plans
See `ideas.md`

---

## Compilation
Jitter compiles to machine code using Cranelift. The compilation process is structured as follows:

**Text Input -> Lexer (+ preprocessing) -> Parser (+ macro expansion) -> (Type Checker & Transformer) -> IR Code Generator -> IR Compiler**

Respective input transformations:  
**String -> [Tokens] -> AST -> (Typed AST + Contextual Tables) -> CLIF -> Machine Code**

### *Some implementation details*

**Lexer**:  
The lexer is straight-forward apart from keywords. Keyword lexing is done using a DFA which should have been generated through a macro instead.

**Parser**:  
The parser is a recursive descent parser.
The advantage of a recursive descent parser is the ability to prioritize rules.  
For example, given the rule `A -> B | C`, priority can be given to `B` which can help eliminate some ambiguity.

**AST**:  
I chose to represent the AST as a struct like so:
```Rust
struct AST {
  functions: Vec<Function>,
  structs: Vec<Struct>,
  // other top-levels and contextual types
  ...
}
```
The advantage of this representation to an `enum`/tree-based approach is the ability to "lookup" top-level items. For example, functions can all be forward declared by simply iterating over `AST.functions` without needing AST traversal.

---

## FFI and Rust Interop
All data types in Jitter align with Rust's `#[repr(C)]`. Because all primitive types also align with Rust's, interop is simple.

Jitter provides a simple context object for working with Rust.  
Note that Jitter is meant to be embedded within a Rust project.

Hooking Rust functions into a global Jitter context is done like so:
```Rust
use jitter::prelude::*;

#[repr(C)]
struct Data {...}

fn some_function(data: Data) {...}
fn another(...) {...}

fn main() {
    let jitter = Jitter! {
        // files to load             functions to export from Rust
        ["./path/to/file.jitter"] <- [some_function, another]
    };

    // Obtain a reference to a Jitter function
    // `GetFunctions` can get multiple functions at once (see `macros.rs`)
    let jitter_main = GetFunction!{
      jitter::main as fn()
    };

    // Runs the main function
    jitter_main();
}
```

Calling Rust functions from Jitter:
```Rust
// Mirror the Rust (or any C-like language) struct
struct Data {...}

// Note that the names of these function must match the source name
extern {
    fn some_function(data: Data);
    fn another();
}

fn main() {
    let data = Data {...};
    // Call into Rust code
    some_function(data);
}
```

---

## Future Goals of Jitter
*A future version of Jitter would have the following features*


- No raw pointers
  - Since Jitter runs in-memory and source code can change on-the-fly, raw pointers may pose security risks
    - References work exactly the same as in Rust, meaning pointers aren't really needed
- No lifetimes
  - Just allocate to the heap for persistent/unsized data
  - Store references like C/C++, but without explicit lifetimes
- Persistent data
  - Since Jitter is an embedded scripting language, you may want to update a program without resetting its state
  - This is done using `@persistent` with static variables:
```Rust
@persistent
static mut var: Type = Type::new();
```
`var` will be initialized when Jitter first identifies the static identifier as being persistent. This occurs the first time your program is run.

From then on, any hot-reloads with the same variable `var` will load the previous contents of `var`.
- `box`
  - `let x = box 123` is equivalent to Rust's `let x = Box::new(123)`
  - The result of the expression right of `box` will be allocated on the heap
  - When `x` exits scope, the heap allocation will be freed
- No `unsafe`
  - Without raw pointers or lifetimes, this isn't needed
  - You can still break things by operating on bits/memory
  - `static mut` variables are treated like any other mutable variables
- Strings
  - `String` works the same as in Rust, utilizing resizable heap allocations
  - `str` (**not** `&str`) is a **fixed-length** string
  - `str` is nothing more than an array of characters which may be stored as constant data
    - The `str` type is therefore **always immutable**
  - `String`s can be formatted similarly to Python
    - A formatted string is heap-allocated because the required space is unknown
  - A reference to a `String` is a `str`
```Rust
let a: str = "This is a str";

let number = 12;
// equivalent to Rust's `format!("This is a formatted String: {}", number);`
let b: String = f"This is a formatted String: {number}";
let c: str = f"{number}".as_str();
let d: str = &b;
```