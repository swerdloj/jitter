// TODO: Static Jitter context
use jitter::prelude::*;


// TODO: This
#[jitter::link] 
extern {
    fn negate(v: i32) -> i32;
    fn identity(v: u32) -> u32;
}


// TODO: This
#[jitter::export]
fn from_rust() {
    println!("From Rust code");
}


fn hello_from_rust(number: i32) {
    println!("\nHello from Rust! -- {}\n", number);
}

#[derive(Debug)]
#[repr(C)]
struct JitterStruct {
    a: u32,
    b: i8,
}

fn main() {
    let mut jitter = Jitter! {
        ["./tests/rewrite_test.jitter"]
    };

    let test = GetFunction! {
        jitter::test as fn() -> i32
    };

    let params = GetFunction! {
        jitter::params as fn(u32, u32) -> u32
    };

    let structs = GetFunction! {
        jitter::structs as fn(u32, i8) -> i8
    };

    let struct_return = GetFunction! {
        jitter::struct_return as fn(u32, i8) -> JitterStruct
    };

    println!("test() = {}", test().into());
    println!("params(7, 123) = {}", params(&7, &123).into());
    println!("structs(100, 7) = {}", structs(&100, &-70).into());
    println!("struct_return(90, -1) = {:?}", struct_return(&90, &-1).into());
}

fn main2() {
    // TODO: growable environment (REPL-style) / hot-reloading
    
    let mut jitter = Jitter! {
        ["./tests/integration_test.jitter"] <- [hello_from_rust]
    };

    // The above is equivalent to this:

    // let mut jit = JitterContextBuilder::new()
    //     .with_source_path("./tests/integration_test.jitter")
    //     .with_function(jitter::FFI!(hello_from_rust))
    //     .build();

    // TEMP: for testing -- eventually replace with #[jitter::link] usage above
    let negate: fn(i32) -> i32 = unsafe { 
        std::mem::transmute(jitter.get_fn("negate")) 
    };

    let multiply: fn(i32, i32) -> i32 = unsafe { 
        std::mem::transmute(jitter.get_fn("multiply")) 
    };

    let struct_test: fn(u8, u16, u16) -> u16 = unsafe { 
        std::mem::transmute(jitter.get_fn("struct_test")) 
    };

    let specified_literals: fn() -> i8 = unsafe { 
        std::mem::transmute(jitter.get_fn("specified_literals")) 
    };

    let function_calls: fn(u16) -> u16 = unsafe {
        std::mem::transmute(jitter.get_fn("function_calls"))
    };

    // TODO: Return stack allocations
    // #[allow(non_snake_case)]
    // let FFI_test: fn(u8, u16, u16) -> JitterStruct = unsafe { 
    //     std::mem::transmute(jitter.get_fn("FFI_test")) 
    // };

    println!("negate(1234560)      = {:?}", negate(1234560));
    println!("multiply(12, -7)     = {:?}", multiply(12, -7));
    println!("struct_test(1, 2, 3) = {:?}", struct_test(1, 2, 3));
    println!("specified_literals() = {:?}", specified_literals());
    println!("function_calls(5)    = {:?}", function_calls(5));
    // println!("FFI_test(10, 21, 39) = {:?}", FFI_test(10, 29, 31));
}