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

use crate::command::Builtin;
use crate::pipeline::Pipeline;

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
    input: String,                   // Current user input string being edited
    cursor_pos: usize,               // Current position of the cursor within the input string
    stdout: RawTerminal<io::Stdout>, // Raw terminal output for direct terminal manipulation
    history: Vec<String>,            // Collection of previously entered commands
    history_index: usize,            // Current index when navigating through command history
    last_input: String,              // User input before history navigation
    completion: Option<Completion>,  // Tab completion state
}

impl Terminal {
    pub fn new() -> Result<Self> {
        let term = Self {
            input: String::new(),
            cursor_pos: 0,
            stdout: io::stdout().into_raw_mode()?,
            history: Vec::new(),
            history_index: 0,
            last_input: String::new(),
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
                    eprintln!("{e}");
                }
            }
            self.reset_input();
        }
    }

    fn reset_input(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
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
                    self.append_history();
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
                Key::Up => self.get_previous_command()?,
                Key::Down => self.get_next_command()?,
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
        } else {
            write!(self.stdout, "{}", BELL)?;
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

    fn append_history(&mut self) {
        // Don't add empty commands or duplicates of the last command
        let command = &self.input;
        if command.is_empty() || (self.history.last().map_or(false, |last| last == command)) {
            return;
        }
        self.history.push(command.to_string());
        self.history_index = self.history.len();
    }

    fn get_previous_command(&mut self) -> Result<()> {
        // Can't go back if we're at the beginning of history or history is empty
        if self.history.is_empty() || self.history_index == 0 {
            write!(self.stdout, "{}", BELL)?;
            return Ok(());
        }
        // Save current input before moving to previous command
        if self.history_index == self.history.len() {
            self.last_input = self.input.clone();
        } else {
            self.history[self.history_index] = self.input.clone();
        }
        // Move to previous command
        self.history_index -= 1;
        self.input = self.history[self.history_index].clone();
        self.cursor_pos = self.input.len();
        self.draw_input()
    }

    fn get_next_command(&mut self) -> Result<()> {
        // Check if we're already at or beyond the end of history
        if self.history_index >= self.history.len() {
            write!(self.stdout, "{}", BELL)?;
            return Ok(());
        }
        // Save current input to the history
        self.history[self.history_index] = self.input.clone();
        self.history_index += 1;
        // Set input: either from stored_input (if at end) or from history
        if self.history_index == self.history.len() {
            self.input = self.last_input.clone();
        } else {
            self.input = self.history[self.history_index].clone();
        }
        // Update cursor position and redraw
        self.cursor_pos = self.input.len();
        self.draw_input()
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
    let first = &strings[0];
    let last = &strings[strings.len() - 1];
    first
        .chars()
        .zip(last.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a)
        .collect()
}
