use anyhow::Result;
use std::{
    io::{self, Write},
    path::PathBuf,
};

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

        let pipeline = match parse_command(&input) {
            Ok(pipe) => pipe,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        if !pipeline.commands.is_empty() {
            for cmd in pipeline.commands {
                cmd.execute()?;
            }
        }
    }
}

fn parse_command(input: &str) -> Result<Pipeline> {
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
                    push_arg(&mut cmd, &mut current_arg);
                }
            }
            '>' => {
                iter.next();
                handle_redirection(&mut iter, &mut cmd, &mut current_arg)?;
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
    loop {
        print!("{}quote> ", if quote == '\'' { "" } else { "d" });
        io::stdout().flush()?;

        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let (chunk, closed) = parse_quoted_string(&mut line.trim().chars().peekable(), quote);
        arg.push_str(&chunk);
        if closed {
            break;
        }
    }
    Ok(())
}

fn handle_redirection(
    iter: &mut std::iter::Peekable<std::str::Chars>,
    cmd: &mut Command,
    current_arg: &mut String,
) -> Result<()> {
    let overwrite = matches!(iter.peek(), Some('>'))
        .then(|| {
            iter.next();
            false
        })
        .unwrap_or(true);
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
                push_arg(cmd, current_arg);
            }
        }
    }
    current_arg.clear();
    Ok(())
}
