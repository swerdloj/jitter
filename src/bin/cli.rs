// TODO: CLI interface for the compiler (isolated JIT code, no embedded env.)

fn main() {
    let input = std::env::args();

    println!("
Usage:
  lang INPUT_PATH --FLAGS

Flags:
  --output OUTPUT_PATH
  --CLIF
...

Run `lang --help FLAG` for more detailed information
");
}