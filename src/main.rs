use anyhow::Result;
use std::io::{self, Write};
use std::process;

fn main() -> Result<()> {
    loop {
        print!("$ ");
        io::stdout().flush()?;

        // Wait for user input
        let stdin = io::stdin();
        let mut input = String::new();
        stdin.read_line(&mut input)?;

        input = input.trim().into();
        let tokens = tokenize(&input);
        match tokens[..] {
            ["exit", status] => process::exit(status.parse::<i32>()?),
            _ => {
                println!("{}: command not found", tokens[0]);
            }
        }
    }
}

fn tokenize(input: &String) -> Vec<&str> {
    input.split_whitespace().collect()
}
