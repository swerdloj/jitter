use proc_macro::TokenStream;

/*

TODO:

```
#[jitter::link]
extern "jitter" { 
    fn multiply(a: i32, b: i32) -> i32; 
}
```

would generate the following:


```
static __jitter__multiply: fn(i32, i32) -> i32 = std::mem::transmute(static_jit_context.get("multiply"));

fn multiply(a: i32, b: i32) -> i32 {
    unsafe {
        __jitter__multiply(a, b)
    }
}
```


or similar




fn jitter_test(x: u32) -> u32 {
    println!("{} -- hello", x);
    12
}

static static_example: fn(u32) -> u32 = unsafe { std::mem::transmute(jitter_test as *const u8) };

fn test(x: u32) -> u32 {
    static_example(x)
}

*/


/// Generates Rust-callable functions from an `extern "jitter"` block
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