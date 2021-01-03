:: USAGE
:: `test` will simply run '/examples/test.rs' in debug mode
:: `test bench` will run '/tests/benchmark.rs' in release mode and time compilation stages

@echo off

if "%1" == "bench" (
    cargo test benchmark --release --features=benchmark -q -- --nocapture
    goto end
)

cargo run --example test

:end