use std::{
    io::{self, Write},
    iter, mem,
    path::PathBuf,
    str,
};

use anyhow::{anyhow, Result};

pub enum Token {
    Arg(String),
    Pipe,
    Redirect(Redirect),
}

pub enum Redirect {
    Stdout { path: PathBuf, append: bool },
    Stderr { path: PathBuf, append: bool },
    Both { path: PathBuf, append: bool },
}

pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut input_char_iter = input.trim().chars().peekable();
    let mut tokens = Vec::new();
    let mut cur_token = String::new();

    while let Some(&ch) = input_char_iter.peek() {
        match ch {
            '\'' | '"' => {
                input_char_iter.next();
                let (mut quoted, closed) = parse_quoted_string(&mut input_char_iter, ch);
                if !closed {
                    collect_additional_quoted(&mut quoted, ch)?;
                }
                cur_token.push_str(&quoted);
            }
            '\\' => {
                input_char_iter.next();
                if let Some(escaped) = input_char_iter.next() {
                    cur_token.push(escaped);
                }
            }
            ' ' => {
                input_char_iter.next();
                if !cur_token.is_empty() {
                    tokens.push(Token::Arg(mem::take(&mut cur_token)));
                }
            }
            '>' => {
                input_char_iter.next();
                let redirect_token = handle_redirection(&mut input_char_iter, &cur_token)?;
                tokens.push(redirect_token);
                cur_token.clear();
            }
            '|' => {
                input_char_iter.next();
                if !cur_token.is_empty() {
                    tokens.push(Token::Arg(mem::take(&mut cur_token)));
                }
                tokens.push(Token::Pipe);
                if input_char_iter.peek().is_none() {
                    cur_token.push_str(&collect_next_arg()?);
                }
            }
            _ => cur_token.push(input_char_iter.next().unwrap()),
        }
    }

    if !cur_token.is_empty() {
        tokens.push(Token::Arg(cur_token));
    }

    Ok(tokens)
}

fn parse_quoted_string(iter: &mut impl Iterator<Item = char>, quote: char) -> (String, bool) {
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
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let prefix = if quote == '\'' { "d" } else { "" };
    let mut line = String::new();
    loop {
        print!("{prefix}quote> ");
        stdout.flush()?;
        stdin.read_line(&mut line)?;
        let (chunk, closed) = parse_quoted_string(&mut line.trim().chars().peekable(), quote);
        arg.push_str(&chunk);
        if closed {
            break;
        }
        line.clear();
    }
    Ok(())
}

fn collect_next_arg() -> Result<String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut line = String::new();
    loop {
        print!("> ");
        stdout.flush()?;
        stdin.read_line(&mut line)?;
        let arg = line.trim();
        if !arg.is_empty() {
            return Ok(arg.to_string());
        } else {
            line.clear();
        }
    }
}

fn handle_redirection(iter: &mut iter::Peekable<str::Chars>, fd_arg: &str) -> Result<Token> {
    // Check if we have >> (append) or just > (overwrite)
    let append = matches!(iter.peek(), Some('>'));
    if append {
        iter.next(); // consume second '>'
    }
    let path: PathBuf = iter
        .by_ref()
        .skip_while(|c| c.is_whitespace())
        .collect::<String>()
        .into();
    if path.as_os_str().is_empty() {
        return Err(anyhow!("No file specified for redirection"));
    }
    let redirect = match fd_arg {
        "2" => Redirect::Stderr { path, append }, // stderr
        "&" => Redirect::Both { path, append },   // both stdout and stderr
        _ => Redirect::Stdout { path, append },
    };
    Ok(Token::Redirect(redirect))
}
