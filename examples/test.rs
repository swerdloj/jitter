use jitter::prelude::*;


fn hello_from_rust(number: &i32) {
    println!("\nHello from Rust! -- {}\n", number);
}

fn print_i32(n: &i32) {
    println!("i32: {}", n);
}

fn print_u32(n: &u32) {
    println!("u32: {}", n);
}

#[derive(Debug)]
#[repr(C)]
struct JitterStruct {
    a: u32,
    b: i32,
}

fn main() {
    let jitter = Jitter! {
        ["./tests/rewrite_test.jitter"] <- [print_i32, print_u32, hello_from_rust]
        extensions <- ["./examples"]
        where 
        [
            // Informs the lexer to replace left side with right side
            "func" => "fn",
            "hello_there" => "hello_from_rust(123_u32);"
        ]
    };

    // NOTE: The above `where` clause expands to this:
    // jitter_builder.with_lexer_callback(LexerCallback {
    //     string: "func",
    //     replacement: "fn",
    // });
    // jitter_builder.with_lexer_callback(LexerCallback {
    //     string: "hello_there",
    //     replacement: "hello_from_rust(123_u32);",
    // });


    // Can get single function
    let function_call1 = GetFunction! {
        jitter::function_call1 as fn() -> i32
    };

    let function_call2 = GetFunction! {
        jitter::function_call2 as fn() -> JitterStruct
    };


    // Sets the identifier to `GetFunction!(rhs)` 
    GetFunctions! {
        struct_return = jitter::struct_return       as fn(u32, i32) -> JitterStruct,
        structs       = jitter::structs             as fn(u32, i32) -> i32,
        params        = jitter::params              as fn(u32, u32) -> u32,
        test          = jitter::test                as fn() -> i32,
        ffi           = jitter::FFI                 as fn(u32),
        callback      = jitter::custom_lex_callback as fn(),
        ops           = jitter::custom_operators    as fn(),
        preprocessing = jitter::preprocessing       as fn(),
        meta          = jitter::meta_usage          as fn(),
    }

    ffi(&9);
    println!("test() = {}", test().into());
    println!("params(7, 123) = {}", params(&7, &123).into());
    println!("structs(100, -70) = {}", structs(&100, &-70).into());
    println!("struct_return(90, -1) = {:?}", struct_return(&90, &-1).into());
    println!("function_call1() = {}", function_call1().into());
    println!("function_call2() = {:?}", function_call2().into());
    callback();
    println!("--preprocessing()--");
    preprocessing();
    println!("--operators()--");
    ops();
    meta();
}