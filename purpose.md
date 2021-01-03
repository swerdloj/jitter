# References
- https://github.com/mun-lang/mun
- https://github.com/jonathandturner/rhai
- https://github.com/rune-rs/rune
- https://github.com/pistondevelopers/dyon
- https://www.boringcactus.com/2020/09/16/survey-of-rust-embeddable-scripting-languages.html
- (discussion) https://www.reddit.com/r/rust/comments/jqms89/i_wrote_a_programming_languageinterpreter_as_a/
- https://github.com/nyar-lang/valkyrie-language

# Language Ideas

## Overarching Goals
- Use the language itself as a base (just a Rust-inspired scripting language)
- Continue with the original idea:
  - Jitter is to Rust what Lua is to C/C++
- Hot-reloadable
  - Jitter functions (in .jitter files) should have a hot-reload mechanism when used in Rust projects
  - Allow for designated persistent data (between reloads):
```Rust
// The following variable will initially have its given value
// After any hot-reloads, this field will KEEP its value
@persistent
static mut EXAMPLE_DATA: type = ...;
```
### Secondary/Long-Term Goals
- Custom syntax
  - Not sure how to approach just yet. Base language is needed first
- AST hooks
  - Like custom syntax, but gives users the ability to manipulate the AST directly
- Multiple backends
  - Host-language contexts (primary backend)
  - REPL (with imports, etc.)
  - Standalone compiler (produce executables)

## Language Features
- Simplified enums:
```Rust
// example using `match`
match enum_variable {
    // instead of path::EnumType::Variant
    .Variant {..} => ..,
}
```
- Option unwrapping
  - Similar to `Result`'s `?` operator
  - `panic` instead of returning an error
```Rust
let x: Option<u32> = None;
// Panics if `x` is `None` (equivalent to x.unwrap())
let y: u32 = x!;
```
- Simple threading/async
  - Not sure how viable this is with Cranelift
  - Don't need to worry about being strict like Rust
- No lifetimes
  - Lifetimes would be a bit excessive for a scripting language
  - How to deal with memory though?

## Language Specializations (potential purposes)
## 1. DSLs
- Along with the Rust hooks, create an interface to extend the language via a Rust API
- Plugin example: 
  - 1 - Write a library in Rust/Jitter
  - 2 - Create a Jitter plugin to "add" the library to Jitter
  - 3 - Custom syntax can be used in Jitter which is translated using the plugin
  - 4 - `main` function can be taken over for plugin's purposes
- Use case example:
  - Signed distance fields (SDFs) have primitives and unique operations
  - A Jitter SDF plugin could generate SDFs using custom syntax
  - Syntax would be unique to types defined by the plugin
    - For example `<>` is not an operator, but a plugin could define it for SDF types
    - Would otherwise error for non-plugin types
    - **Operators would depend on types**

**Hypothetical Jitter code examples:**  
SDF generation:
```Rust
use plugin SDF; // defined in Rust

// where `<>` is SDF union
let box_union_sphere = Sphere { radius: 3 } <> Box { length, width, height = 3 };
```
would generate
```Rust
use SDF::*;

let box_union_sphere = union(Sphere{radius: 3, default_fields: ..}, Box{fields: ..});
```
---
Another example: Neural networks
```Rust
use plugin NeuralNetworks;
//                    shape
let network = Input(100, 200) -> Conv2D(..) -> MaxPooling -> Output(1);
```
generates
```Rust
use NeuralNetworks::*;

let network = layer::Input(100*200)
    .add_layer(layer::Conv2D(.., default values))
    .add_layer(layer::MaxPooling)
    .add_layer(layer::Output(1));
```
---
Even simple operators could become plugins such as the `apply` operator from `sdf-lang`:
```Rust
fn add(a: u32, b: u32) -> { a + b }
fn custom(a: T, b: T) -> { a.field.do_something_with(b.field) }

let sum = add <- (1, 2, 3, 4, 5, 6, 9);
// or
let something = custom <- (a, b, c, d);
```
generates
```Rust
let sum = add(1, add(2, add(3, add(4, add(5, add(6, 9))))));
let something = custom(a, custom(b, custom(c, d)));
```
---
**Implementing a Plugin**

Plugins could be created using an API like so:

```Rust
// Rust code

use jitter::plugin::*;

// SDF example
use SDF::library;
let op_union = operator! {
    binary op <> :              // binary or unary
        lhs -> type impl SDF    // type means the operator only applies to that type
        rhs -> type impl SDF    // impl means the type is a trait, not concrete

    becomes:
        code!( library::union(lhs, rhs) ); // This code is generated
};

// `apply` operator example
let op_apply = operator! {
    binary op <- :
        lhs -> (expr,+) type 1  // `type 1` means items are of same, "aliased" type
        rhs -> function type 1  // `function` refers to a function name

    becomes:
        // Add function calls
        for (i, expr) in lhs.iter().enumerate() {
            if i == lhs.len() - 2 { // only 2 items remain
                code!( rhs(expr, lhs[i + 1]) );
                break;
            }
            code!( rhs(expr,  );
        }
        // Add closing perentheses
        for _ in 0..lhs.len() {
            code!( ) );
        }
};
```

## 2. Traits as Syntax
- Like (1.), but operations are built into the language
- Introduce new operators and syntax via traits
  - Similar to meta-programming
  - Think 'extensions' rather than 'plugins'

Example: `contains` trait
```Rust
// Jitter code

// `extension` is like `trait`, but it implements custom syntax rather than functions
pub extension Contains<T> {
    pattern:  <$collection:expr> contains <$item:expr>
    becomes:  $collection.contains(&$item)

    fn contains(&self, &item: T) -> bool;
}

// example impl
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
    print("contains 12");
}

```

## 3. `start` vs `main` -- Custom Engines/Frameworks
- Allow users to write Rust code that passes Jitter-compatible objects to the `start` function
- This allows items such as windows to be passed

Example of a rendering engine using Jitter and `start`
```Rust
use framework renderer

// This is called instead of `main`
@dimensions(800, 600)
fn start(window: Window) {
    window.draw(Circle {...}, 400, 300);
}
```

Alternatively, frameworks could operate as traits for programs. For example:
```Rust
// This could automatically import the needed modules
@framework renderer

// Require functions, their inputs, and their outputs are specified by the framework
fn init(..) {..}

fn update(..) {..}

// no main()
```

Control of the Jitter program would be given to the framework host, and so long as the required functions exist (as specified by the framework), hot reloading would be the primary development mechanism.

This approach lends itself particularly well to Jitter's overarching goals. `@persistent` is especially useful here.

---
**Potential Paper**
- 1 - Libraries as Languages
  - Explore the idea of replacing the typical use case of a library with that of a language (DSL)
- 2 - Syntax as Functions
  - Explore the idea of functions as simply transforming the AST
  - Extending syntax becomes a simple matter