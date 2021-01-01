// TODO: Static context for loading functions
// jitter!("./tests/integration_test.jitter");

// TODO: This
#[jitter::link] 
extern {
    fn negate(v: i32) -> i32;
    fn identity(v: u32) -> u32;
}


#[jitter::export]
fn from_rust() {
    println!("From Rust code");
}

#[derive(Debug)]
#[repr(C)]
struct JitterStruct {
    a: u8,
    b: u16,
    c: u16,
}

fn main() {
    // TODO: growable environment (REPL-style) / hot-reloading
    let mut jit = jitter::create_local_context("./tests/integration_test.jitter");

    // TEMP: for testing -- eventually replace with #[jitter::link] usage above
    let negate: fn(i32) -> i32 = unsafe { 
        std::mem::transmute(jit.get_fn("negate")) 
    };

    let multiply: fn(i32, i32) -> i32 = unsafe { 
        std::mem::transmute(jit.get_fn("multiply")) 
    };

    let struct_test: fn(u8, u16, u16) -> u16 = unsafe { 
        std::mem::transmute(jit.get_fn("struct_test")) 
    };

    let specified_literals: fn() -> i8 = unsafe { 
        std::mem::transmute(jit.get_fn("specified_literals")) 
    };

    // TODO: Return stack allocations
    // #[allow(non_snake_case)]
    // let FFI_test: fn(u8, u16, u16) -> JitterStruct = unsafe { 
    //     std::mem::transmute(jit.get_fn("FFI_test")) 
    // };

    println!("negate(1234560) = {:?}", negate(1234560));
    println!("multiply(12, -7) = {:?}", multiply(12, -7));
    println!("struct_test(1, 2, 3) = {:?}", struct_test(1, 2, 3));
    println!("specified_literals() = {:?}", specified_literals());
    // println!("FFI_test(10, 21, 39) = {:?}", FFI_test(10, 29, 31));
}