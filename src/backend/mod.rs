/* 
TODO:

    1. Define functions in the language
    2. JIT the functions and store code
    3. Interpret inputs such that those functions can be called

    4. Get structs working

    5. Get struct impls working

    6. Allow use of Rust-defined functions
    7. Function args & returns should be valid

       Rust Code:
           #[language_link]
           fn name(u32, u32) -> u32 {..}
    
       Language Code:
           ...
           let x = name(1, 2);

       Need to ensure types are compatible and signatures can be understood

*/