:: USAGE
:: `test` will simply run '/examples/test.rs' in debug mode
:: `test bench` will run '/examples/test.rs' in release mode and time runtime

if "%1" == "bench" (
    cargo run --release --example test --features=benchmark
    goto end
)

cargo run --example test

:end