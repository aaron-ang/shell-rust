use std::{fs, path::PathBuf};

use anyhow::{anyhow, Result};
use os_pipe::pipe;

use crate::command::Command;
use crate::history::History;
use crate::token::{tokenize, RedirectType, Token};

pub struct Pipeline {
    commands: Vec<Command>,
    history: History,
}

impl Pipeline {
    pub fn new(input: &str, history: History) -> Result<Self> {
        let mut pipeline = Self {
            commands: Vec::new(),
            history,
        };
        let tokens = tokenize(input)?;
        pipeline.parse_tokens(tokens)?;
        Ok(pipeline)
    }

    fn parse_tokens(&mut self, tokens: Vec<Token>) -> Result<()> {
        let mut cmd = Command::new(self.history.clone());
        for token in tokens {
            match token {
                Token::Arg(arg) => cmd.push_arg(&arg),
                Token::Pipe => {
                    if cmd.is_empty() {
                        return Err(anyhow!("Empty command before pipe"));
                    }
                    self.commands.push(cmd);
                    cmd = Command::new(self.history.clone());
                }
                Token::Redirect {
                    type_,
                    path,
                    append,
                } => {
                    let redirect_file = create_file(&path, append)?;
                    match type_ {
                        RedirectType::Stdout => cmd.set_output(redirect_file),
                        RedirectType::Stderr => cmd.set_err(redirect_file),
                        RedirectType::Both => {
                            let err = redirect_file.try_clone()?;
                            cmd.set_output(redirect_file);
                            cmd.set_err(err);
                        }
                    }
                }
            }
        }
        if !cmd.is_empty() {
            self.commands.push(cmd);
        }
        Ok(())
    }

    pub fn execute(&mut self) -> Result<()> {
        if self.commands.is_empty() {
            return Ok(());
        }
        if self.commands.len() == 1 {
            return self.commands[0].execute();
        }

        let last_idx = self.commands.len() - 1;
        let mut children = Vec::new();
        let mut prev_pipe = None;

        for (i, cmd) in self.commands.iter_mut().enumerate() {
            let is_last = i == last_idx;
            // Prepare a pipe for this stage if not last
            let (next_reader, next_writer) = if !is_last {
                let (r, w) = pipe()?;
                (Some(r), Some(w))
            } else {
                (None, None)
            };
            if cmd.is_builtin() {
                // Builtin: execute directly, redirect stdout if needed
                if let Some(out) = next_writer {
                    // temporarily swap output to pipe
                    cmd.execute_to_output(out)?;
                } else {
                    // last builtin
                    cmd.execute()?;
                }
            } else {
                // External: spawn child process
                let mut p = cmd.new_process();
                if let Some(reader) = prev_pipe.take() {
                    p.stdin(reader);
                }
                if let Some(out) = next_writer {
                    p.stdout(out);
                }
                children.push(p.spawn()?);
            }
            prev_pipe = next_reader;
        }

        for mut child in children {
            child.wait()?;
        }

        Ok(())
    }
}

fn create_file(path: &PathBuf, append: bool) -> Result<fs::File> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true)
        .create(true)
        .truncate(!append)
        .append(append);
    Ok(opts.open(path)?)
}
