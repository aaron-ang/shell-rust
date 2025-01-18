use std::{
    env, fs,
    io::{self, Write},
};

use anyhow::Result;
use strum::IntoEnumIterator;
use termion::{clear, cursor, raw::RawTerminal};

use crate::command::{Builtin, Pipeline};

pub struct InputState {
    input: String,
    cursor_pos: usize,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
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

    pub fn autocomplete(&mut self) -> Result<()> {
        let input = &self.input[..self.cursor_pos];
        let prefix = input.trim();
        let matches = get_matching_executables(prefix);
        let mut stdout = io::stdout();

        if matches.is_empty() {
            write!(stdout, "\x07")?; // Ring bell for no matches
            return Ok(());
        }

        write!(stdout, "\r{}", clear::CurrentLine)?;
        self.input = matches[0].clone();
        if matches.len() == 1 {
            self.input.push(' ');
        }
        self.cursor_pos = self.input.len();
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
