use proc_macro::TokenStream;

/*

TODO:

Import Jitter function to Rust
```
#[jitter::link]
extern "Jitter" { 
    fn multiply(a: i32, b: i32) -> i32; 
}
```

would generate the following:


```

fn multiply(a: i32, b: i32) -> i32 {
    static __jitter__multiply: fn(i32, i32) -> i32 = std::mem::transmute(static_jit_context.get("multiply"));
    unsafe {
        __jitter__multiply(a, b)
    }
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