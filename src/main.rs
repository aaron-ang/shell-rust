use anyhow::Result;
use std::io::{self, Write};
use std::process;

fn main() -> Result<()> {
    loop {
        print!("$ ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();
        let args: Vec<&str> = input.split_whitespace().collect();

        match args.get(0) {
            Some(&"exit") => {
                assert_eq!(args.len(), 2);
                process::exit(args[1].parse::<i32>()?);
            }
            Some(&"echo") => println!("{}", args[1..].join(" ")),
            Some(command) => println!("{}: command not found", command),
            None => continue,
        }
    }
}
