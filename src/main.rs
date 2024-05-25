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
            Some(cmd) => handle_executable_or_unknown(cmd, &args[1..]),
            None => continue,
        }
    }
}

fn handle_type(cmd: &str) {
    if BUILTINS.contains(&cmd) {
        println!("{} is a shell builtin", cmd);
    } else if let Some(path) = find_command_path(cmd) {
        println!("{} is {}", cmd, path);
    } else {
        println!("{}: not found", cmd);
    }
}

fn find_command_path(cmd: &str) -> Option<String> {
    let path = env::var("PATH").unwrap_or_default();
    let paths: Vec<&str> = path.split(':').collect();
    for path in paths {
        let full_path = Path::new(path).join(cmd);
        if full_path.exists() {
            return Some(full_path.display().to_string());
        }
    }
    None
}

fn handle_executable_or_unknown(cmd: &str, args: &[&str]) {
    if let Some(path) = find_command_path(cmd) {
        let output = process::Command::new(path).args(args).output();
        match output {
            Ok(output) => {
                io::stdout().write_all(&output.stdout).unwrap();
                io::stderr().write_all(&output.stderr).unwrap();
            }
            Err(e) => eprintln!("{}", e),
        }
    } else {
        eprintln!("{}: command not found", cmd);
    }
}
