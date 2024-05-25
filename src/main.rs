use anyhow::Result;
use std::{
    env,
    io::{self, Write},
    path::Path,
    process,
};

const BUILTINS: [&str; 3] = ["echo", "exit", "type"];

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
            Some(&"type") => {
                assert_eq!(args.len(), 2);
                handle_type(args[1]);
            }
            Some(command) => println!("{}: command not found", command),
            None => continue,
        }
    }
}

fn handle_type(cmd: &str) {
    if BUILTINS.contains(&cmd) {
        println!("{} is a shell builtin", cmd);
    } else {
        let path = env::var("PATH").unwrap();
        let paths: Vec<&str> = path.split(':').collect();
        for path in paths {
            let full_path = Path::new(path).join(cmd);
            if full_path.exists() {
                println!("{} is {}", cmd, full_path.display());
                return;
            }
        }
        println!("{}: not found", cmd);
    }
}
