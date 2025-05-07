use std::{
    env,
    fmt::Display,
    io::{self, Write},
    path::PathBuf,
    process,
};

use anyhow::Result;
use strum::{Display, EnumIter, EnumString};

#[derive(EnumIter, EnumString, Display)]
#[strum(ascii_case_insensitive)]
pub enum Builtin {
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
    fn new() -> Self {
        Self {
            name: String::new(),
            args: Vec::new(),
            out: Box::new(io::stdout()),
            err: Box::new(io::stderr()),
        }
    }

    pub fn execute(&mut self) -> Result<()> {
        match Builtin::try_from(self.name.as_str()) {
            Ok(builtin) => match builtin {
                Builtin::Exit => handle_exit(&self.args),
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
        let target = match self.args.first().map(String::as_str) {
            Some("~") | None => env::var("HOME").unwrap_or_else(|_| "/".to_string()),
            Some(path) => path.to_string(),
        };
        if env::set_current_dir(&target).is_err() {
            self.print_err(&format!("cd: {}: No such file or directory", target))?;
        }
        Ok(())
    }

    fn handle_redirection(
        &mut self,
        iter: &mut std::iter::Peekable<std::str::Chars>,
        current_arg: &mut String,
    ) -> Result<()> {
        let overwrite = !matches!(iter.peek(), Some('>'));
        if !overwrite {
            iter.next();
        }

        let path_str: String = iter.by_ref().skip_while(|c| c.is_whitespace()).collect();
        let file = create_file(&path_str, overwrite)?;

        match current_arg.as_str() {
            "2" => self.err = file,
            "1" => self.out = file,
            _ => {
                self.out = file;
                if !current_arg.is_empty() {
                    self.push_arg(current_arg);
                }
            }
        }
        current_arg.clear();
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

    fn push_arg(&mut self, current_arg: &mut String) {
        if self.name.is_empty() {
            self.name = current_arg.clone();
        } else {
            self.args.push(current_arg.clone());
        }
        current_arg.clear();
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

fn create_file(path: &str, overwrite: bool) -> Result<Box<dyn Write>> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true)
        .create(true)
        .truncate(overwrite)
        .append(!overwrite);
    Ok(Box::new(opts.open(path)?))
}

fn find_command_path(cmd: &str) -> Option<PathBuf> {
    env::var("PATH").ok().and_then(|path_str| {
        env::split_paths(&path_str).find_map(|path| {
            let full_path = path.join(cmd);
            if full_path.is_file() {
                Some(full_path)
            } else {
                None
            }
        })
    })
}

fn handle_exit(args: &[String]) -> ! {
    let status = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    process::exit(status);
}

pub struct Pipeline {
    commands: Vec<Command>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn execute(&mut self) -> Result<()> {
        for cmd in &mut self.commands {
            cmd.execute()?;
        }
        Ok(())
    }

    pub fn from_input(input: &str) -> Result<Self> {
        let mut pipeline = Pipeline::new();
        let mut iter = input.trim().chars().peekable();
        let mut cmd = Command::new();
        let mut current_arg = String::new();

        while let Some(&ch) = iter.peek() {
            match ch {
                '\'' | '"' => {
                    iter.next();
                    let (mut quoted, closed) = parse_quoted_string(&mut iter, ch);
                    if !closed {
                        collect_additional_quoted(&mut quoted, ch)?;
                    }
                    current_arg.push_str(&quoted);
                }
                '\\' => {
                    iter.next();
                    if let Some(escaped) = iter.next() {
                        current_arg.push(escaped);
                    }
                }
                ' ' => {
                    iter.next();
                    if !current_arg.is_empty() {
                        cmd.push_arg(&mut current_arg);
                    }
                }
                '>' => {
                    iter.next();
                    cmd.handle_redirection(&mut iter, &mut current_arg)?;
                }
                '|' => todo!("Implement pipes"),
                _ => {
                    current_arg.push(iter.next().unwrap());
                }
            }
        }

        if !current_arg.is_empty() {
            cmd.push_arg(&mut current_arg);
        }

        pipeline.commands.push(cmd);
        Ok(pipeline)
    }
}

fn parse_quoted_string(
    iter: &mut std::iter::Peekable<std::str::Chars>,
    quote: char,
) -> (String, bool) {
    let mut chunk = String::new();
    while let Some(mut ch) = iter.next() {
        if ch == quote {
            return (chunk, true);
        }
        if ch == '\\' && quote == '"' {
            if let Some(next_ch) = iter.next() {
                if !matches!(next_ch, '$' | '`' | '"' | '\\' | '\n') {
                    chunk.push('\\');
                }
                ch = next_ch;
            }
        }
        chunk.push(ch);
    }
    (chunk, false)
}

fn collect_additional_quoted(arg: &mut String, quote: char) -> Result<()> {
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    loop {
        print!("{}quote> ", if quote == '\'' { "" } else { "d" });
        stdout.flush()?;

        let mut line = String::new();
        stdin.read_line(&mut line)?;
        let (chunk, closed) = parse_quoted_string(&mut line.trim().chars().peekable(), quote);
        arg.push_str(&chunk);
        if closed {
            break;
        }
    }
    Ok(())
}
