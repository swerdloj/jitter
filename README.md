# Jitter: **J**ust-**I**n-**T**ime **R**ust
Jitter is a just-in-time (JIT) compiled scripting language designed with Rust interoperability in mind.

Please note that Jitter is a personal project and many ideas used/stated in this project are **highly opinionated**.

---

## How does Jitter compare to Rust?
### *Syntax*
Much of Jitter's syntax comes directly from Rust. This includes everything from pattern matching to keywords.

I improved the syntax wherever possible such as allowing `.Variant` instead of Rust's `path::EnumType::Variant`

### *Why not just use Rust?*
I created Jitter according to my personal language preferences, and Rust happens to be nearly perfect for my needs. But there is a rather large difference between a systems programming language (Rust) and a scripting language.

Most notably, Rust is rather strict and for good reason. These reasons, however, do not necessarily apply to scripting languages.

Thus, Jitter does not aim to be fast, safe, or low-level. Instead, it seeks to promote *good* (in my opinion) design patterns while being easy to work with and providing functionality not found in traditional languages.

---

## Motivations
I like to use Python as my antithesis of a good language.  

My problem with Python is that it is built on layers of hacks, variable declarations are invisible, it lacks strong typing (and its static typing support is essentially cosmetic), and indendation-based scopes drive me crazy. I won't go into my opinions on inheritance.

Furthermore, embedded languages such as Lua suffer from the exact same problems on top of being extremely slow.

So, I made Jitter: Jitter is to Rust what Lua is to C/C++.  
Additionally, Jitter is fully compatible with C and C++.

I went with a scripting language for the sake of being able to write embedded scripts without constantly needing to recompile an entire project. The simple FFI described below makes hooking Jitter into Rust feel exactly like it should: like writing Rust.

The JIT compiler is also extremely fast (thanks to Cranelift) and produces good machine code (although LLVM's is faster).

---

## Future Plans
I have plans for Jitter beyond the scope of a scripting language. I won't detail those plans here, but they involve "main hijacking" and custom syntax to create languages which are frameworks in themselves.

A scripting language is perfect for this, as typical Rust problems can be removed entirely from end-user experience (lifetimes, etc.)

---

## Compilation
Jitter compiles to machine code using Cranelift. The compilation process is structured as follows:

**Textual Input -> Lexer -> Parser -> (Type Checker & Transformer) -> IR Code Generator -> IR Compiler**

Respective input transformations:  
**String -> [Token] -> AST -> (Typed AST + Context Tables) -> CLIF -> Machine Code**

### *Some implementation details*

**Lexer**:  
The lexer is straight-forward apart from keywords. Keyword lexing is done using a hand-rolled DFA which should have been through a macro. At least it's fast.

**Parser**:  
The parser is a hand-written recursive descent parser. I have user LALRPOP in the past, but I found it difficult to reason about the program with.  
The advantage of a recursive descent parser is the ability to prioritize rules.  
For example, given the rule `A -> B | C`, priority can be given to `B` which can help eliminate some ambiguity.

**AST**:  
I chose to represent the AST as a struct like so:
```Rust
// simplified
struct AST {
  functions: Vec<Function>,
  structs: Vec<Struct>,
  ...
}
```
The advantage of this representation to an `enum`/tree-based approach is the ability to "lookup" top-level items. For example, functions can all be forward declared by simply iterating over `AST.functions`. No AST traversal needed.

**General frontend notes**:  
The entire frontend pipeline operates on strings taken from the input (`&'input str`). The only redundant allocations needed are a handful of `Type` clones which are rather cheap.  
Once compilation is finished, the input and all frontend data structures are dropped simultaneously.

### *Cranelift*
The use of Cranelift offers the following advantages:
- Rust-native API
  - I wanted to learn and use LLVM for this project, but I had trouble finding usable resources (especially regarding Rust)
  - The Cranelift API is quite easy to work with, as it is written in Rust
- 1:1 mapping of data types
  - Cranelift data types are the same as Rust's: `u8`, `i128`, `f32`, etc. work as expected
- No pointers
  - In Cranelift, pointers are just integers. These are easy to maintain
- Cranelift IR can be used for:
  - JIT compilation
  - Ahead-of-Time compilation
  - IR Interpreter  
  - CLIF -> LLIR for maximum optimization
- Simple
  - At the cost of fancy optimizations like LLVM, Cranelift is very simple (and understandable)
  - Dispite the lack of documentation and examples, I found it (somewhat) easy to get started with

---

## FFI and Rust Interop
All data types in Jitter align with Rust's `#[repr(C)]`. Because all primitive types also align with Rust's, interop is incredibly simple.

Jitter provides a simple context object for working with Rust.  
Note that Jitter is meant to be embedded within a Rust project. While it can be used alone, FFI support without a Rust host is harder.

Hooking Rust functions into a global Jitter context is done like so:
```Rust
use jitter::prelude::*;

#[repr(C)]
struct Data {...}

// Uses a global context to register this function
#[jitter::link]
fn do_something(data: Data) {
    ...    
}

fn main() {
    // Runs the main function
    jitter::run("./path/to/file.jitter");
}
```
Or, using a local context:
```Rust
// Note that no macros are needed when using local contexts
fn some_function(data: Data) {...}
fn another(...) {...}

fn main() {
    let jitter = Jitter! {
        // files to load             functions to export from Rust
        ["./path/to/file.jitter"] <- [some_function, another]
    };

    // Runs the main function
    jitter.run_main();
}
```

Calling FFI function in Jitter:
```Rust
// Mirror the Rust (or any C-like language) struct
struct Data {...}

extern {
    // Note that the name of this function must match the source name
    fn do_something(data: Data);
}

// for next example
fn from_jitter(data: Data) -> u32 {...}

fn main() {
    let data = Data {...};
    do_something(data);
}
```
It's that easy!

Calling Jitter functions from Rust:
```Rust
// This macro allows this type to be passed to jitter
#[jitter::data]
struct Data {...}

fn main() {
    let data = Data {...};

    // using local context
    let result = jitter.call("from_jitter", &[data]);

    // using global context
    let result = jitter::call("from_jitter", &[data]);
}
```
Arguments are passed as an array. The above would be equivalent to `from_jitter(data)`.

---

## Programming in Jitter
If you know Rust, you know Jitter.

Jitter can even compile a decent subset of Rust code. The differences are as follows:
- No raw pointers
  - Since Jitter runs in-memory and source code can change on-the-fly, I didn't think this would be very safe (security-wise)
  - References work exactly the same as in Rust, so pointers aren't really needed anyway
- No lifetimes
  - Just allocate to the heap for persistent/unsized data
  - You can store references like C/C++, but such lifetimes are then in your hands
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
  - I'm not yet sure how I want to approach manual heap allocations
- No `unsafe`
  - Without raw pointers or lifetimes, this isn't needed
  - You can still break things by operating on bits/memory
  - `static mut` variables are treated like any other mutable variables
- Strings
  - I find the different string types to be useful, so I kept them with minor changes
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