use std::{
    env, fs,
    io::{self, Write},
};

use anyhow::Result;
use strum::IntoEnumIterator;
use termion::{
    clear, cursor,
    event::Key,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
};

use crate::command::{Builtin, Pipeline};

const BELL: &str = "\x07";

struct Completion {
    prefix: String,
    matches: Vec<String>,
}

impl Completion {
    fn new(prefix: String, matches: Vec<String>) -> Self {
        Self { prefix, matches }
    }
}

pub struct Terminal {
    input: String,
    cursor_pos: usize,
    stdout: RawTerminal<io::Stdout>,
    completion: Option<Completion>,
}

impl Terminal {
    pub fn new() -> Result<Self> {
        let term = Self {
            input: String::new(),
            cursor_pos: 0,
            stdout: io::stdout().into_raw_mode()?,
            completion: None,
        };
        Ok(term)
    }

    pub fn start(&mut self) -> Result<()> {
        loop {
            self.draw_input()?;
            match self.process_input() {
                Ok(should_execute) => {
                    if should_execute {
                        self.run()?;
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
            self.reset();
        }
    }

    fn reset(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.completion = None;
    }

    fn draw_input(&mut self) -> Result<()> {
        write!(self.stdout, "\r{}", clear::CurrentLine)?;
        write!(self.stdout, "$ {}", self.input)?;
        self.stdout.flush()?;
        Ok(())
    }

    fn process_input(&mut self) -> Result<bool> {
        for key in io::stdin().keys().filter_map(Result::ok) {
            match key {
                Key::Char('\n') => {
                    writeln!(self.stdout, "\r")?;
                    return Ok(!self.input.is_empty());
                }
                Key::Char('\t') => self.handle_tab()?,
                Key::Ctrl('c') => {
                    writeln!(self.stdout, "\r")?;
                    return Ok(false);
                }
                Key::Ctrl('d') => {
                    if self.input.is_empty() {
                        self.stdout.suspend_raw_mode()?;
                        println!();
                        std::process::exit(0);
                    }
                    self.show_completions()?;
                }
                Key::Backspace => self.backspace()?,
                Key::Left => self.move_cursor_left()?,
                Key::Right => self.move_cursor_right()?,
                Key::Char(c) => self.insert_char(c)?,
                _ => (),
            };
            self.stdout.flush()?;
        }
        Ok(true)
    }

    fn backspace(&mut self) -> Result<()> {
        if self.cursor_pos > 0 {
            self.input.remove(self.cursor_pos - 1);
            self.cursor_pos -= 1;
            // Erase the character to the left of the cursor
            write!(self.stdout, "{} {}", cursor::Left(1), cursor::Left(1))?;
        }
        Ok(())
    }

    fn move_cursor_left(&mut self) -> Result<()> {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            write!(self.stdout, "{}", cursor::Left(1))?;
        }
        Ok(())
    }

    fn move_cursor_right(&mut self) -> Result<()> {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
            write!(self.stdout, "{}", cursor::Right(1))?;
        }
        Ok(())
    }

    fn insert_char(&mut self, c: char) -> Result<()> {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
        write!(self.stdout, "{}", c)?;
        Ok(())
    }

    fn handle_tab(&mut self) -> Result<()> {
        let input = &self.input[..self.cursor_pos];
        let prefix = input.trim();
        if prefix.is_empty() {
            return self.insert_char('\t');
        }

        // Get matches for completion:
        // - Reuse existing matches if we have completion state with same prefix
        // - Otherwise find new matches for the current prefix
        let matches = match &self.completion {
            Some(state) if state.prefix == prefix => state.matches.clone(),
            _ => find_matching_executables(prefix),
        };

        match matches.len() {
            0 => write!(self.stdout, "{}", BELL)?,
            // Single match: complete with the match and add a space
            1 => {
                let mut completed = matches[0].clone();
                completed.push(' ');
                self.update_input(completed)?;
            }
            // Multiple matches: try partial completion or show options
            _ => {
                let common_prefix = longest_common_prefix(&matches);
                // If common prefix is longer than current prefix, use it for partial completion
                if common_prefix.len() > prefix.len() {
                    self.update_input(common_prefix)?;
                } else {
                    // Show all matches
                    self.completion = Some(Completion::new(prefix.to_string(), matches.clone()));
                    write!(self.stdout, "{}", BELL)?;
                    self.display_matches(&matches)?;
                }
            }
        }

        Ok(())
    }

    fn update_input(&mut self, new_input: String) -> Result<()> {
        self.input = new_input;
        self.cursor_pos = self.input.len();
        self.draw_input()
    }

    fn display_matches(&mut self, matches: &[String]) -> Result<()> {
        writeln!(self.stdout, "\r")?;
        writeln!(self.stdout, "{}", matches.join("  "))?;
        self.draw_input()
    }

    fn show_completions(&mut self) -> Result<()> {
        let matches = find_matching_executables(&self.input[..self.cursor_pos]);
        if matches.is_empty() {
            write!(self.stdout, "{}", BELL)?;
            return Ok(());
        }
        self.display_matches(&matches)
    }

    fn run(&self) -> Result<()> {
        self.stdout.suspend_raw_mode()?;
        match Pipeline::from_input(&self.input) {
            Ok(mut pipeline) => pipeline.execute()?,
            Err(e) => {
                eprintln!("{e}");
            }
        };
        self.stdout.activate_raw_mode()?;
        Ok(())
    }
}

fn find_matching_executables(prefix: &str) -> Vec<String> {
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

fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    assert!(strings.is_sorted());
    let first = &strings[0];
    let last = &strings[strings.len() - 1];
    first
        .chars()
        .zip(last.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a)
        .collect()
}
