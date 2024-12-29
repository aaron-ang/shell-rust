use anyhow::Result;
use std::{env, fmt::Display, io::Write, path::PathBuf, process};
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

pub struct Command {
    pub name: String,
    pub args: Vec<String>,
    pub out: Box<dyn Write>,
    pub err: Box<dyn Write>,
}

impl Command {
    pub fn execute(mut self) -> Result<()> {
        match Builtin::try_from(self.name.as_str()) {
            Ok(builtin) => match builtin {
                Builtin::Exit => handle_exit(self.args),
                Builtin::Echo => {
                    let arg_str = self.args.join(" ");
                    self.print_out(&arg_str)
                }
                Builtin::Type => self.handle_type(),
                Builtin::Pwd => self.print_out(&env::current_dir()?.display()),
                Builtin::Cd => self.handle_cd(),
            },
            Err(_) => self.run_executable_or_unknown(),
        }
    }

    fn handle_type(&mut self) -> Result<()> {
        if let Some(cmd) = self.args.first() {
            match Builtin::try_from(cmd.as_str()) {
                Ok(_) => self.print_out(&format!("{} is a shell builtin", cmd))?,
                Err(_) => {
                    if let Some(path) = find_command_path(cmd) {
                        self.print_out(&format!("{} is {}", cmd, path.display()))?
                    } else {
                        self.print_out(&format!("{}: not found", cmd))?
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_cd(&mut self) -> Result<()> {
        let path = self.args.get(0).map_or("~", String::as_str);
        let target = if path == "~" {
            env::var("HOME").unwrap_or_else(|_| "/".to_string())
        } else {
            path.to_string()
        };
        if env::set_current_dir(&target).is_err() {
            self.print_err(&format!("cd: {}: No such file or directory", target))?;
        }
        Ok(())
    }

    fn run_executable_or_unknown(&mut self) -> Result<()> {
        if find_command_path(&self.name).is_some() {
            match process::Command::new(&self.name).args(&self.args).output() {
                Ok(output) => {
                    self.out.write_all(&output.stdout)?;
                    self.err.write_all(&output.stderr)?;
                    Ok(())
                }
                Err(e) => self.print_err(&e),
            }
        } else {
            self.print_err(&format!("{}: command not found", self.name))
        }
    }

    fn print_out(&mut self, msg: &dyn Display) -> Result<()> {
        writeln!(self.out, "{msg}")?;
        Ok(())
    }

    fn print_err(&mut self, msg: &dyn Display) -> Result<()> {
        writeln!(self.err, "{msg}")?;
        Ok(())
    }
}

fn find_command_path(cmd: &str) -> Option<PathBuf> {
    env::var("PATH").ok().and_then(|paths| {
        env::split_paths(&paths).find_map(|path| {
            let full = path.join(cmd);
            if full.is_file() {
                Some(full)
            } else {
                None
            }
        })
    })
}

fn handle_exit(args: Vec<String>) -> ! {
    let status = args.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
    process::exit(status);
}
