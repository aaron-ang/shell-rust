use std::{
    io::{self, Write},
    path::PathBuf,
};

use anyhow::Result;

mod command;
use command::Command;

struct Pipeline {
    commands: Vec<Command>,
    // background: bool,
}

fn main() -> Result<()> {
    loop {
        print!("$ ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let pipeline = match parse_cmd(&input) {
            Ok(cmd_args) => cmd_args,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        if pipeline.commands.is_empty() {
            continue;
        }

        for cmd in pipeline.commands {
            cmd.execute()?;
        }
    }
}

fn parse_cmd(input: &str) -> Result<Pipeline> {
    let mut pipeline = Pipeline { commands: vec![] };
    let mut iter = input.trim().chars().peekable();

    let mut cmd = Command {
        name: String::new(),
        args: vec![],
        out: Box::new(io::stdout()),
        err: Box::new(io::stderr()),
    };
    let mut current_arg = String::new();

    while let Some(&ch) = iter.peek() {
        match ch {
            '\'' | '"' => {
                let quote = iter.next().unwrap();
                let (mut str_arg, mut found_closing) = parse_quotes(&mut iter, quote);
                while !found_closing {
                    print!("{}quote> ", if quote == '\'' { "" } else { "d" });
                    io::stdout().flush()?;
                    let mut line = String::new();
                    io::stdin().read_line(&mut line)?;
                    let (new_arg, new_found_closing) =
                        parse_quotes(&mut line.trim().chars().peekable(), quote);
                    str_arg.push_str(&new_arg);
                    found_closing = new_found_closing;
                }
                current_arg.push_str(&str_arg);
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
                    push_arg(&mut cmd, &mut current_arg);
                }
            }
            '>' => {
                iter.next();
                let overwrite = match iter.peek() {
                    Some('>') => {
                        iter.next();
                        false
                    }
                    _ => true,
                };

                let path_str: String = iter.by_ref().skip_while(|c| c.is_whitespace()).collect();
                let path = PathBuf::from(path_str);

                let file: Box<dyn Write> = {
                    let mut opts = std::fs::OpenOptions::new();
                    opts.write(true)
                        .create(true)
                        .truncate(overwrite)
                        .append(!overwrite);
                    Box::new(opts.open(&path)?)
                };

                match current_arg.as_str() {
                    "2" => cmd.err = file,
                    "1" => cmd.out = file,
                    _ => {
                        cmd.out = file;
                        if !current_arg.is_empty() {
                            push_arg(&mut cmd, &mut current_arg);
                        }
                    }
                }

                current_arg.clear();
            }
            '|' => todo!("Implement pipes"),
            _ => {
                current_arg.push(iter.next().unwrap());
            }
        }
    }

    if !current_arg.is_empty() {
        push_arg(&mut cmd, &mut current_arg);
    }
    pipeline.commands.push(cmd);
    Ok(pipeline)
}

fn push_arg(cmd: &mut Command, current_arg: &mut String) {
    if cmd.name.is_empty() {
        cmd.name = current_arg.clone();
    } else {
        cmd.args.push(current_arg.clone());
    }
    current_arg.clear();
}

fn parse_quotes(iter: &mut std::iter::Peekable<std::str::Chars>, quote: char) -> (String, bool) {
    let mut current_arg = String::new();
    let mut found_closing = false;

    while let Some(mut ch) = iter.next() {
        if ch == quote {
            found_closing = true;
            break;
        }
        if ch == '\\' && quote == '"' {
            if let Some(next_ch) = iter.next() {
                if !matches!(next_ch, '$' | '`' | '"' | '\\' | '\n') {
                    current_arg.push('\\');
                }
                ch = next_ch;
            }
        }
        current_arg.push(ch);
    }

    (current_arg, found_closing)
}
