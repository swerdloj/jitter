// TODO: CLI interface for the compiler (isolated JIT code, no embedded env.)

fn main() {
    let input: Vec<String> = std::env::args().collect();

    println!("
Usage:
  jitter INPUT_PATH --FLAGS       Run a jitter file
  jitter                          Open jitter REPL session

Flags:
  --help FLAG                     Display a flag's help information
  --output OUTPUT_PATH            Specify file output path
  --CLIF                          Output Cranelift IR to a file
  --AST                           Output the jitter AST to a file
...

Run 'jitter --help FLAG' for more detailed information
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