use anyhow::Result;
use std::{
    env,
    io::{self, Write},
    path::Path,
    process,
};

fn main() -> Result<()> {
    let builtins = vec!["echo", "exit", "type", "pwd"];

    loop {
        print!("$ ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();
        let args = input.split_whitespace().collect::<Vec<&str>>();

        if let Some(command) = args.get(0) {
            match *command {
                "exit" => handle_exit(args),
                "echo" => println!("{}", args[1..].join(" ")),
                "type" => handle_type(args, builtins.clone()),
                "pwd" => println!("{}", env::current_dir()?.display()),
                "cd" => handle_cd(args),
                cmd => handle_executable_or_unknown(cmd, &args[1..]),
            }
        }
    }
}

fn handle_exit(args: Vec<&str>) -> ! {
    process::exit(
        args.get(1)
            .and_then(|status| status.parse().ok())
            .unwrap_or(0),
    )
}

fn handle_type(args: Vec<&str>, builtins: Vec<&str>) {
    let cmd = args.get(1).unwrap_or(&"");
    if builtins.contains(&cmd) {
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

fn handle_cd(args: Vec<&str>) {
    let path = args.get(1).unwrap_or(&"");
    if env::set_current_dir(path).is_err() {
        eprintln!("{}: No such file or directory", path);
    }
}
