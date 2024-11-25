use anyhow::Result;
use std::{
    env,
    io::{self, Write},
    process,
};
use strum::EnumString;

#[derive(EnumString)]
#[strum(ascii_case_insensitive)]
enum Builtin {
    Exit,
    Echo,
    Type,
    Pwd,
    Cd,
}

fn main() -> Result<()> {
    loop {
        print!("$ ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();
        let args = input.split_whitespace().collect::<Vec<&str>>();

        if let Some(&command) = args.get(0) {
            match Builtin::try_from(command) {
                Ok(builtin) => match builtin {
                    Builtin::Exit => handle_exit(args),
                    Builtin::Echo => println!("{}", args[1..].join(" ")),
                    Builtin::Type => handle_type(args),
                    Builtin::Pwd => println!("{}", env::current_dir()?.display()),
                    Builtin::Cd => handle_cd(args),
                },
                Err(_) => handle_executable_or_unknown(command, &args[1..]),
            }
        }
    }
}

fn handle_exit(args: Vec<&str>) -> ! {
    let status = args
        .get(1)
        .and_then(|status| status.parse().ok())
        .unwrap_or(0);
    process::exit(status);
}

fn handle_type(args: Vec<&str>) {
    if let Some(&cmd) = args.get(1) {
        match Builtin::try_from(cmd) {
            Ok(_) => println!("{} is a shell builtin", cmd),
            Err(_) => {
                if let Some(path) = find_command_path(cmd) {
                    println!("{} is {}", cmd, path);
                } else {
                    println!("{}: not found", cmd);
                }
            }
        }
    }
}

fn find_command_path(cmd: &str) -> Option<String> {
    let path_env = env::var("PATH").unwrap_or_default();
    for path in env::split_paths(&path_env) {
        let full_path = path.join(cmd);
        if full_path.exists() {
            return Some(full_path.display().to_string());
        }
    }
    None
}

fn handle_executable_or_unknown(cmd: &str, args: &[&str]) {
    if let Some(path) = find_command_path(cmd) {
        match process::Command::new(path).args(args).output() {
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
    let new_path = if path.is_empty() || *path == "~" {
        env::var("HOME").unwrap_or_default()
    } else {
        path.to_string()
    };
    if env::set_current_dir(&new_path).is_err() {
        eprintln!("{}: No such file or directory", new_path);
    }
}
