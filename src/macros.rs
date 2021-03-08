/// Convenience function for instantiating a local Jitter context.  
/// Compiles the given file paths and links the given Rust functions.
///
/// A `where` section can be used to insert lexer callbacks.
///
/// If no functions need to be linked, simply omit the `<- [...]` section.
/// If no lexer replacements are needed, omit the `where [...]` section.
///
/// Usage:
/// ```
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
///     ] where [
///         "pattern1" => "transformation1",
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
        // Optional extension path
        $(
            extensions <- [$extension_path:expr]
        )?
        // Optional lexer callbacks
        $(
            where [ $($input:expr => $output:expr),+    $(,)? ]
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
        // Extension group
        $(
            .with_extension_path($extension_path)
        )?
        // Lexer group
        $(
            $(
                .with_lexer_callback(LexerCallback {
                    string: $input,
                    replacement: $output,
                })
            )+
        )?

        .build()
        .expect("JIT compile")
    };
}

/// Get a function pointer from a Jitter context without worrying about FFI details.
/// 
/// Usage:
/// ```
/// let jitter: JitterContext = ...;
///
/// let jitter_fn = GetFunction! {
///     jitter::function as fn(param, types) -> return_type
/// };
/// ```  
/// The macro will expand to the following code:
/// ```
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

/// Get multiple function pointers from a Jitter context without worrying about FFI details.
/// 
/// Usage:
/// ```
/// let jitter: JitterContext = ...;
///
/// GetFunctions! {
///     jitter_fn1 = jitter::function1 as fn(param, types) -> return_type,
///     jitter_fn2 = jitter::function2 as fn(param, types) -> return_type,
///     ..
/// }
/// ```  
/// The macro will call `GetFunction!` for each item, assigning it to the desired varaible.
#[macro_export]
macro_rules! GetFunctions {
    (
        $(
            $name:ident = 
                $context:ident :: $function:ident as fn($($param:ty),*) $(-> $ret:ty)?
        ),+ $(,)?
    ) => {
        $(
            let $name = 
                GetFunction!($context :: $function as fn($($param),*) $(-> $ret)?);
        )+
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