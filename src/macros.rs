/// Convenience function for instantiating a local Jitter context  
/// Compiles the given file paths and links the given Rust functions
///
/// If no functions need to be linked, simply omit the `<- [...]` section
///
/// Usage:
/// ```Rust
/// fn some_function(...) {...}
///
/// let jitter_context = Jitter! {
///     [
///         "./path/file1.jitter", 
///         "./path/file2.jitter", 
///         ...
///     ] <- [
///         some_function, 
///         ...
///     ]    
/// };
/// ```
#[macro_export]
macro_rules! Jitter {
    ( 
        // Path group (with optional trailing comma)
        [ $($path:expr),+    $(,)? ]
        // Optional function group
        $(  // Function group body (with optional trailing comma)
            <- [ $($func:ident),+    $(,)? ]
        )?
    ) => {
        JitterContextBuilder::new()

        // Path group
        $(
            .with_source_path($path)
        )+
        // Function group
        $(
            // Function group body
            $(
                .with_function(stringify!($func), $func as *const u8)
            )+
        )?

        .build().unwrap()
    };
}

/// Get a function pointer from a Jitter context without worrying about FFI details.
/// 
/// Usage:
/// ```Rust
/// let jitter: JitterContext = ...;
///
/// let jitter_fn = GetFunction! {
///     jitter::function as fn(param, types) -> return_type
/// };
/// ```  
/// The macro will expand to the following code:
/// ```Rust
/// let jitter_fn: fn(&param, &types) -> Return<return_type> = unsafe {
///     std::mem::transmute(jitter.get_fn("function"))
/// };
/// ```
#[macro_export]
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

        // TODO: Could wrap the above in a closure like so:
        //       Requires naming the closure inputs.
        //       This would need a proc macro, but eliminates FFI types.
        
        // |$(p1: $param,)*| $(-> $ret)? {
        //     func(&p1, ...).into()
        // }
    };
}


/// Convenience macro for optional printing
pub(crate) mod log {
    #[cfg(not(feature = "benchmark"))]
    #[macro_export]
    macro_rules! log {
        ($($e:tt)*) => {
            println!(
                $( $e )*
            );
        };
    }
    
    #[cfg(feature = "benchmark")]
    #[macro_export]
    macro_rules! log {
        ($($e:tt)*) => {};
    }
}