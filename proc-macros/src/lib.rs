use proc_macro::TokenStream;

/*

TODO:

IMPORTS:
Jitter function from Rust
```
#[jitter::link]
extern "Jitter" { 
    fn multiply(a: i32, b: i32) -> i32; 
}
```

would generate:


```
fn multiply(a: i32, b: i32) -> i32 {
    static __jitter_multiply: fn(i32, i32) -> i32 = std::mem::transmute(static_jit_context.get("multiply"));
    unsafe {
        __jitter__multiply(a, b)
    }
}
```

--------------------------------

EXPORTS:
Rust functions from Jitter
```
#[jitter::export]
fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

```

would generate:

```
#[no_mangle]
extern "C" fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

// and (somehow) call the following code:
unsafe {
    jitter::global_context.link_fn(
        FFI_FunctionDefinition {
            fn_pointer: multiply as *const i8,
            name: "multiply",
            parameters: vec![("a", "i32"), ("b", "i32")],
            returns: "i32"
        }
    );
}
```


or similar

*/


/// Generates Rust-callable functions from an `extern "Jitter"` block
///
/// Note that the functions and their types must be equivalent to their Jitter implementations
#[proc_macro_attribute]
pub fn link(attr: TokenStream, item: TokenStream) -> TokenStream {
    // TODO: This
    
    TokenStream::new()
}

/// Exposes Rust functions to Jitter
///
/// Note that all types used must also be exposed to Jitter.
#[proc_macro_attribute]
pub fn export(attr: TokenStream, item: TokenStream) -> TokenStream {
    // TODO: This
    
    TokenStream::new()
}