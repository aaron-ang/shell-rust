use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process,
};

use anyhow::Result;
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

        let (cmd, args) = match parse_args(&input) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        match Builtin::try_from(cmd) {
            Ok(builtin) => match builtin {
                Builtin::Exit => handle_exit(args),
                Builtin::Echo => println!("{}", args.join(" ")),
                Builtin::Type => handle_type(args),
                Builtin::Pwd => println!("{}", env::current_dir()?.display()),
                Builtin::Cd => handle_cd(args),
            },
            Err(_) => handle_executable_or_unknown(cmd, &args),
        }
    }
}

fn parse_args(input: &str) -> Result<(&str, Vec<&str>)> {
    let input = input.trim();
    let (cmd, rest) = input.split_once(' ').unwrap_or((input, ""));
    let mut iter = rest.trim().chars().peekable();
    let mut args: Vec<&str> = vec![];
    let mut current_arg = String::new();

    while let Some(c) = iter.peek() {
        match c {
            '\'' | '"' => {
                let quote = iter.next().unwrap();
                let mut found_closing = false;

                while let Some(ch) = iter.next() {
                    if ch == quote {
                        found_closing = true;
                        break;
                    }
                    current_arg.push(ch);
                }

                if !found_closing {
                    anyhow::bail!("No closing quote found");
                }

                args.push(Box::leak(current_arg.clone().into_boxed_str()));
                current_arg.clear();
            }
            ' ' => {
                iter.next();
                if !current_arg.is_empty() {
                    args.push(Box::leak(current_arg.clone().into_boxed_str()));
                    current_arg.clear();
                }
            }
            _ => {
                current_arg.push(iter.next().unwrap());
            }
        }
    }

    if !current_arg.is_empty() {
        args.push(Box::leak(current_arg.into_boxed_str()));
    }

    Ok((cmd, args))
}

fn handle_exit(args: Vec<&str>) -> ! {
    let status = args
        .get(0)
        .and_then(|status| status.parse().ok())
        .unwrap_or(0);
    process::exit(status);
}

fn handle_type(args: Vec<&str>) {
    if let Some(&cmd) = args.get(0) {
        match Builtin::try_from(cmd) {
            Ok(_) => println!("{} is a shell builtin", cmd),
            Err(_) => {
                if let Some(path) = find_command_path(cmd) {
                    println!("{} is {}", cmd, path.display());
                } else {
                    println!("{}: not found", cmd);
                }
            }
        }
    }
}

fn find_command_path(cmd: &str) -> Option<PathBuf> {
    let path_env = env::var("PATH").unwrap_or_default();
    for path in env::split_paths(&path_env) {
        let full_path = path.join(cmd);
        if full_path.is_file() {
            return Some(full_path);
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
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    } else {
        eprintln!("{}: command not found", cmd);
    }
}

fn handle_cd(args: Vec<&str>) {
    let path = args.get(0).unwrap_or(&"");
    let new_path = if path.is_empty() || *path == "~" {
        env::var("HOME").unwrap_or_default()
    } else {
        path.to_string()
    };
    if env::set_current_dir(&new_path).is_err() {
        eprintln!("{}: No such file or directory", new_path);
    }
}
