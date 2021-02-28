# Methods of Language Extension

## 1. Builtins
### **Custom operators**
Jitter allows for custom binary and unary operators. During the program validation pass, any unknown operators are checked against custom definitions. If found, the expression is transformed into a function call using the operator's associated function like so:

```Rust
// Custom operator defined in Jitter
binary $ {
    fn do_something(lhs: u32, rhs: u32) -> u32 {
        lhs + rhs * 2
    }
}

unary ++ {
    fn add_one(input: u32) -> bool {
        input + 1
    }
}

fn use_operators() {
    // becomes `do_something(1, 2)`
    let x = 1 $ 2;

    // becomes `add_one(x)`
    let y = ++x;
}
```

*Jitter Implementation*
1. Parse `binary` and `unary` AST nodes like any other
2. When parsing expressions, a `parse_expression_custom` rule is given lowest precedence (design decision). If this rule sees any unknown operators, it will treat them as either `UnaryOp::Custom(operator)` or `BinaryOp::Custom(operator)`, then continue parsing the expression as usual
3. During validation pass, register the operator with its corresponding function (such as mapping `$` to `do_something`) and validate the function as usual
4. When validating expressions, upon seeing `UnaryOp::Custom`, simply substitute the `Expression::Unary` with `Expression::FunctionCall`, calling the operator's associated function on the unary operator's right hand side. For `BinaryOp::Custom`, do the same, but call the associated function using `Expression::Binary`'s left- and right-hand sides.


### **Trait-like custom syntax**
A more powerful approach is syntax defined through generic interfaces. The custom operators above are limited by their implementations as functions. A more generic approach should allow for type-sensitive operators (i.e., operator overloading). Borrowing from Rust's trait system, custom syntax may be implemented like the following:
```Rust
// Hypothetical implementation of "syntax via traits" using Rust

// Similar approach to Rust's macro system but more powerful
pub extension Contains<T> {
    pattern:  <$collection:expr> contains <$item:expr>
    becomes:  $collection.contains(&$item)

    fn contains(&self, &item: T) -> bool;
}

// Implementation is done exactly like for traits
impl<T> Contains<T> for Vec<T> {
    fn contains(&self, &item: T) -> bool {
        for x in self {
            if x == item {
                return true;
            }
        }

        return false;
    }
}

// example usage
let x = vec![12, 13];
if x contains 12 {
    println!("Found 12");
}
```
In this example, the `extension` describes what to capture as parsed by the supplied `pattern`. The pattern will capture an expression aliased as `collection`, then the string "contains", then another expression aliased as `item`.  
If the parser sees this pattern, the captured items will be substituted for their respective positions in the `becomes` pattern as signified by the `$`.

Associated functions can be included as well such as `contains`. Implementations, therefor, are identical to Rust's traits.

### **Metaprogramming: code generation**
A JIT-compiled language such as Jitter allows out-of-order code execution/generation. This allows for code to be executed at compile time which generates a string. This string can then be swapped in and treated as code once compilation resumes.

```Rust
// `@meta` signifies a function which runs at compile time and outputs code
@meta
fn make_struct_with_field(name: str, field: str, field_type: str) -> str {
    // Python-style formatted string
    f"struct {name} {
        {field}: {field_type},
    }"
}

fn use_meta_function() {
    // Meta functions are prefixed with `@`
    @make_struct_with_field("Meta", "meta_field", "u32");

    // Create instance of the meta-defined struct 
    let x = Meta {
        meta_field: 7,
    };
}
```

TODO: Mention C# source generators

### **Metaprogramming: macros**
Typical macros as used by Rust. Such macros allow for the parsing of tokens in a user-defined order. Because the host-language's parser is used, macros trade functionality for ease-of-use.
```Rust
// This is the `GetFunction` macro used to obtain Jitter functions from Rust
macro_rules! GetFunction {
    // context::function as fn(ty1, ty2, ..) -> type
    ($context:ident :: $function:ident as fn($($param:ty),*) $(-> $ret:ty)?) => {
        unsafe {
            std::mem::transmute::<
                _,
                fn(
                    $(
                        &$param,
                    )*
                ) $(
                    -> Return<$ret>
                )?
            >
            ($context.get_fn(stringify!($function)))
        }
    };
}

fn use_macro() {
    let jitter_context = ...;

    // Macros are called with the postfix `!` 
    let jitter_fn = GetFunction! {
        jitter_context::function_name as fn(u32) -> u32
    };

    // The above macro invocation expands to:
    let jitter_fn = unsafe {
        std::mem::transmute::<_, fn(&u32,) -> jitter::Return<u32>>(
            jitter_context.get_fn("function_name")
        )
    };
}
```

## 2. Integrated Language Extension
### **Treating the compiler as a library**
Languages such as Jitter are intended for embedded use. This means the Jitter compiler itself is compiled within the host program.

The hosting language can therefore allow various callback functions to be run during Jitter compilation.
```Rust
use jitter::prelude::*;
use jitter::extensions::{Callback, Match};

fn create_context_with_callback() {
    let mut jitter_builder = ...;

    // The lexer will replace `foreach x` with the valid syntax `for _ in x`
    jitter_builder.with_lexer_callback(LexerCallback {
        string: "foreach",
        replacement: "for _ in",
    });

    // The custom parsing rule will be treated as valid
    jitter_context.add_callback(Callback::Parser {
        pattern: "swap {first:expr} {second:expr}",
        does: |matched_items: Vec<Match>| {
            let first = matched_items[0].expect_expr();
            let second = matched_items[1].expect_expr();

            // Swap the expressions
            Vec::from([second, first])
        },
    });
}
```

*Jitter Implementation*  
Lexer Callbacks:
1. Lex the input `string` into tokens
2. Lex the `replacement` into tokens
3. Map the input tokens to the corresponding replacement tokens -- store in `Lexer`
4. When lexing regular input, check each token against the custom rules. If a match is found, substitute the matching tokens with the corresponding tokens (pre-lexed).


### **Preprocessing**
This can be done through traditional text processing. More complex preprocessing can be such as in C/C++, but simple text manipulation provides the simplest means of language extension (adding `$` to the language in this example).

```Rust
// Replaces instances of "$" with a constant
fn preprocess_program(program: String, constant: u32) -> String {
    program.replace("$", &constant.to_string())
}

fn use_preprocessor() {
    let input_program = String::from("let y = $;");

    // Program becomes "let y = 12;"
    let processed_program = preprocess_program(input_program, 12);
}
```

*Jitter Implementation*  
Implemented in the lexer using a state machine:
1. Upon seeing a '#' token, identify the directive as one of:
```C++
#define from_token to_tokens
#include "file_path.jitter"
```
2. For `#include`, simply read the specified file (relative path) and tokenize it using a new lexer. Those tokens are then inserted in-place into the original lexer.
3. For `#define`, lex the `from_token`. If newline is seen next, do not create a `to_tokens`. Otherwise, all following tokens are inserted into `to_tokens`. Once a newline is seen, the tokens are converted into a lexer callback.



## 3. Language Plugins
### **Dynamic libraries**
Dynamic libraries provide a means of loading and calling functions defined outside of the compiled program.

A compiler can use such libraries to allow users to create language plugins.  
In the following Jitter example, the `@plugin(plugin_name)` directive tells the compiler to do the following:

```Rust
@plugin(print)
fn jitter_function() {
    // .. function body
}
```

1. Locate the `print` dynamic library (ending in .dll, .dylib, or .so depending on operating system)
2. Obtain function pointer to the `plugin` function defined as:
```Rust
// Rust program compiled to "print(.dll/.dylib/.so)"

use jitter::plugin::*;

// Jitter expects the `plugin` function to follow this format
#[no_mangle]
fn plugin(mut input: AST) -> PluginResult<AST> {
    // AST manipulation

    if let AST::Function(function) = input {
        // Insert as the first body statement
        function.body.insert(0, 
            format!("print(Entering {});", function.name).parse_statement();
        );

        // Append as final body statement
        function.body.push(
            format!("print(Exiting {});", function.name).parse_statement();
        );
    } else {
        return PluginError("Not a function");
    }

    input
}
```
3. When parsing a Jitter program, the compiler will pass any items marked with `@plugin(plugin_name)` through the corresponding `plugin` function.

This method allows for drag-and-drop language extension. Plugins may be defined in any language capable of generating a compatible dynamic library.

## 4. Total Extension
### **Extension through a new compiler**
A compiler can be developed which accepts the target language with various new mechanisms. For example, `Slang` accepts `HLSL` source code and is capable of compiling HLSL directly. However, the compiler adds additional features such as optional syntax changes, generics, and monomorphization -- all features not found in HLSL on its own.

TODO: Finish this section

### **Transpilation**
TODO: This section

# Uses
## 1. Language Language
The above methods of language extension may be used to create a language able to define itself and other language.  
Using a base language such as Jitter, parsing rules may be defined to change the language entirely.

## 2. Libraries as Language
Rather than creating a library, a language may be defined to implement the desired functionality. For example, a simple domain-specific language may be created to describe neural networks which simplifies the process for users.