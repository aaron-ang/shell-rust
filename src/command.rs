use std::{
    env,
    fmt::Display,
    io::{self, Write},
    mem,
    path::PathBuf,
    process, str,
};

use anyhow::Result;

use strum::{Display, EnumIter, EnumString};

#[derive(EnumIter, EnumString, Display)]
#[strum(ascii_case_insensitive)]
pub enum Builtin {
    Cd,
    Exit,
    Echo,
    Pwd,
    Type,
}

pub struct Command {
    name: String,
    args: Vec<String>,
    output: Box<dyn Write>,
    err: Box<dyn Write>,
}

impl Command {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            args: Vec::new(),
            output: Box::new(io::stdout()),
            err: Box::new(io::stderr()),
        }
    }

    pub fn push_arg(&mut self, current_arg: &str) {
        if self.name.is_empty() {
            self.name = current_arg.to_string();
        } else {
            self.args.push(current_arg.to_string());
        }
    }

    pub fn set_output(&mut self, output: impl Write + 'static) {
        self.output = Box::new(output);
    }

    pub fn set_err(&mut self, err: impl Write + 'static) {
        self.err = Box::new(err);
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }

    pub fn is_builtin(&self) -> bool {
        Builtin::try_from(self.name.as_str()).is_ok()
    }

    pub fn new_process(&self) -> process::Command {
        let mut cmd = process::Command::new(&self.name);
        cmd.args(&self.args);
        cmd
    }

    pub fn execute(&mut self) -> Result<()> {
        match Builtin::try_from(self.name.as_str()) {
            Ok(builtin) => match builtin {
                Builtin::Exit => self.handle_exit(),
                Builtin::Echo => {
                    let arg_str = self.args.join(" ");
                    self.print_out(arg_str)
                }
                Builtin::Type => self.handle_type(),
                Builtin::Pwd => self.print_out(env::current_dir()?.display()),
                Builtin::Cd => self.handle_cd(),
            },
            Err(_) => self.execute_external_command(),
        }
    }

    pub fn execute_to_output(&mut self, out: impl Write + 'static) -> Result<()> {
        let orig_out = mem::replace(&mut self.output, Box::new(out));
        self.execute()?;
        self.output = orig_out;
        Ok(())
    }

    fn handle_exit(&self) -> ! {
        let status = self
            .args
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();
        process::exit(status);
    }

    fn handle_type(&mut self) -> Result<()> {
        if let Some(cmd) = self.args.first() {
            match Builtin::try_from(cmd.as_str()) {
                Ok(_) => self.print_out(format!("{} is a shell builtin", cmd))?,
                Err(_) => {
                    if let Some(path) = Self::full_path(cmd) {
                        self.print_out(format!("{} is {}", cmd, path.display()))?
                    } else {
                        self.print_out(format!("{}: not found", cmd))?
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_cd(&mut self) -> Result<()> {
        let target = match self.args.first().map(String::as_str) {
            Some("~") | None => env::var("HOME").unwrap_or_else(|_| "/".to_string()),
            Some(path) => path.to_string(),
        };
        if env::set_current_dir(&target).is_err() {
            self.print_err(format!("cd: {}: No such file or directory", target))?;
        }
        Ok(())
    }

    fn execute_external_command(&mut self) -> Result<()> {
        if self.exists() {
            let mut process = process::Command::new(&self.name);
            match process.args(&self.args).output() {
                Ok(output) => {
                    self.output.write_all(&output.stdout)?;
                    self.err.write_all(&output.stderr)?;
                    Ok(())
                }
                Err(e) => self.print_err(e),
            }
        } else {
            self.print_err(format!("{}: command not found", self.name))
        }
    }

    fn full_path(cmd: &str) -> Option<PathBuf> {
        env::var("PATH").ok().and_then(|path_str| {
            env::split_paths(&path_str).find_map(|path| {
                let full_path = path.join(cmd);
                full_path.is_file().then_some(full_path)
            })
        })
    }

    fn exists(&self) -> bool {
        Self::full_path(&self.name).is_some()
    }

    fn print_out(&mut self, msg: impl Display) -> Result<()> {
        writeln!(self.output, "{msg}")?;
        Ok(())
    }

    fn print_err(&mut self, msg: impl Display) -> Result<()> {
        writeln!(self.err, "{msg}")?;
        Ok(())
    }
}
