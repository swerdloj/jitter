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

    println!("negate(1234560) = {:?}", negate(1234560));
    println!("multiply(12, -7) = {:?}", multiply(12, -7));
}