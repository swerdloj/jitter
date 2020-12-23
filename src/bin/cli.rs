use std::todo;

// TODO: CLI interface for the compiler (isolated JIT code, no embedded env.)

fn main() {
    let input: Vec<String> = std::env::args().collect();

    println!("
Usage:
  lang INPUT_PATH --FLAGS

Flags:
  --output OUTPUT_PATH
  --CLIF
...

Run 'lang --help FLAG' for more detailed information
");

    let mut i = 0;
    while i < input.len() {
        match input[i].as_str() {
            // TODO: this
            "--help" => {
                i += 1;

                // TODO: Help messages
            }

            _ => {
                todo!("Unrecognized")
            }
        }

        i += 1;
    }
}