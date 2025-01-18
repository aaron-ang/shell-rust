use std::{
    env, fs,
    io::{self, Write},
};

use anyhow::Result;
use strum::IntoEnumIterator;
use termion::{clear, cursor, raw::RawTerminal};

use crate::command::{Builtin, Pipeline};

pub const BELL: &str = "\x07";

struct CompletionState {
    prefix: String,
    matches: Vec<String>,
}

impl CompletionState {
    fn new(prefix: String, matches: Vec<String>) -> Self {
        Self { prefix, matches }
    }
}

pub struct InputState {
    input: String,
    cursor_pos: usize,
    completion_state: Option<CompletionState>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            completion_state: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    pub fn backspace(&mut self, stdout: &mut RawTerminal<io::Stdout>) -> Result<()> {
        if self.cursor_pos > 0 {
            self.input.remove(self.cursor_pos - 1);
            self.cursor_pos -= 1;
            write!(stdout, "{} {}", cursor::Left(1), cursor::Left(1))?;
        }
        Ok(())
    }

    pub fn move_cursor_left(&mut self, stdout: &mut RawTerminal<io::Stdout>) -> Result<()> {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            write!(stdout, "{}", cursor::Left(1))?;
        }
        Ok(())
    }

    pub fn move_cursor_right(&mut self, stdout: &mut RawTerminal<io::Stdout>) -> Result<()> {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
            write!(stdout, "{}", cursor::Right(1))?;
        }
        Ok(())
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
    }

    pub fn handle_tab(&mut self, stdout: &mut RawTerminal<io::Stdout>) -> Result<()> {
        let input = &self.input[..self.cursor_pos];
        let prefix = input.trim();
        if prefix.is_empty() {
            self.insert_char('\t');
            write!(stdout, "\t")?;
            return Ok(());
        }

        if self
            .completion_state
            .as_ref()
            .map_or(false, |completion| completion.prefix == prefix)
        {
            let matches = &self.completion_state.as_ref().unwrap().matches;
            writeln!(stdout, "\r")?;
            writeln!(stdout, "{}", matches.join("  "))?;
        } else {
            let matches = get_matching_executables(prefix);
            if matches.is_empty() {
                write!(stdout, "{}", BELL)?;
                return Ok(());
            }

            if matches.len() == 1 {
                self.input = matches[0].clone();
                self.input.push(' ');
                self.cursor_pos = self.input.len();
            } else {
                let common_prefix = find_longest_common_prefix(&matches);
                if common_prefix.len() > prefix.len() {
                    self.input = common_prefix;
                    self.cursor_pos = self.input.len();
                } else {
                    write!(stdout, "{}", BELL)?;
                    self.completion_state = Some(CompletionState::new(prefix.to_string(), matches));
                    return Ok(());
                }
            }
        }

        write!(stdout, "\r{}", clear::CurrentLine)?;
        print!("$ {}", self.input);
        Ok(())
    }

    pub fn show_completions(&self, stdout: &mut RawTerminal<io::Stdout>) -> Result<()> {
        let matches = get_matching_executables(&self.input[..self.cursor_pos]);
        if matches.is_empty() {
            write!(stdout, "{}", BELL)?;
            return Ok(());
        }
        writeln!(stdout, "\r")?;
        writeln!(stdout, "{}", matches.join("  "))?;
        write!(stdout, "\r{}", clear::CurrentLine)?;
        print!("$ {}", self.input);
        Ok(())
    }

    pub fn run(self) -> Result<()> {
        match Pipeline::from_input(&self.input) {
            Ok(mut pipeline) => pipeline.execute(),
            Err(e) => {
                eprintln!("{e}");
                Ok(())
            }
        }
    }
}

fn get_matching_executables(prefix: &str) -> Vec<String> {
    let mut matches = Vec::new();
    matches.extend(
        Builtin::iter()
            .map(|b| b.to_string().to_lowercase())
            .filter(|b| b.starts_with(prefix)),
    );
    if let Some(path) = env::var_os("PATH") {
        for dir in env::split_paths(&path) {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.filter_map(Result::ok) {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with(prefix) {
                            matches.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    matches.sort();
    matches.dedup();
    matches
}

fn find_longest_common_prefix(strings: &[String]) -> String {
    let first = &strings[0];
    let last = &strings[strings.len() - 1];

    first
        .chars()
        .zip(last.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a)
        .collect()
}
