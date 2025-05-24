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
use crate::history::History;
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

enum ProcessResult {
    Continue(bool),
    Exit,
}

pub struct Terminal {
    /// Input buffer for the command line
    input: Vec<char>,
    /// Current position of the cursor in the input string
    cursor_pos: usize,
    /// Raw terminal output for direct terminal manipulation
    stdout: RawTerminal<io::Stdout>,
    /// History of previously entered commands
    history: History,
    /// Current index in the command history
    history_index: usize,
    /// Last command entered before navigating history
    last_input: String,
    /// Tab completion state
    completion: Option<Completion>,
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.history.save();
    }
}

impl Terminal {
    pub fn new() -> Result<Self> {
        let history = History::open();
        let history_len = history.len();
        let term = Self {
            input: Vec::new(),
            cursor_pos: 0,
            stdout: io::stdout().into_raw_mode()?,
            history,
            history_index: history_len,
            last_input: String::new(),
            completion: None,
        };
        Ok(term)
    }

    pub fn start(&mut self) -> Result<()> {
        loop {
            self.draw_input()?;
            match self.process_input() {
                Ok(ProcessResult::Continue(should_execute)) => {
                    if should_execute {
                        self.run()?;
                    }
                }
                Ok(ProcessResult::Exit) => {
                    println!();
                    return Ok(());
                }
                Err(e) => eprintln!("{e}"),
            }
            self.reset_input();
        }
    }

    fn reset_input(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
    }

    fn draw_input(&mut self) -> Result<()> {
        write!(self.stdout, "\r{}$ ", clear::CurrentLine)?;
        for c in &self.input {
            write!(self.stdout, "{}", c)?;
        }
        self.stdout.flush()?;
        Ok(())
    }

    fn process_input(&mut self) -> Result<ProcessResult> {
        for key in io::stdin().keys().filter_map(Result::ok) {
            match key {
                Key::Char('\n') => {
                    writeln!(self.stdout, "\r")?;
                    self.append_history();
                    return Ok(ProcessResult::Continue(!self.input.is_empty()));
                }
                Key::Char('\t') => self.handle_tab()?,
                Key::Ctrl('c') => {
                    writeln!(self.stdout, "\r")?;
                    return Ok(ProcessResult::Continue(false));
                }
                Key::Ctrl('d') => {
                    if self.input.is_empty() {
                        self.stdout.suspend_raw_mode()?;
                        return Ok(ProcessResult::Exit);
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
        Ok(ProcessResult::Continue(true))
    }

    fn backspace(&mut self) -> Result<()> {
        if self.cursor_pos > 0 {
            self.input.remove(self.cursor_pos - 1);
            self.cursor_pos -= 1;
            // Erase the character to the left of the cursor
            write!(self.stdout, "{} {}", cursor::Left(1), cursor::Left(1))?;
        } else {
            write!(self.stdout, "{BELL}")?;
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
        let command = self.input.iter().collect::<String>();
        self.history.add(command);
        self.history_index = self.history.len();
    }

    fn get_previous_command(&mut self) -> Result<()> {
        // Can't go back if we're at the beginning of history or history is empty
        let history_len = self.history.len();
        if history_len == 0 || self.history_index == 0 {
            write!(self.stdout, "{BELL}")?;
            return Ok(());
        }
        // Save current input before moving to previous command
        let input = self.input.iter().collect::<String>();
        if self.history_index == history_len {
            self.last_input = input;
        } else {
            self.history.set(self.history_index, input);
        }
        // Move to previous command
        self.history_index -= 1;
        if let Some(cmd) = self.history.get(self.history_index) {
            self.input = cmd.chars().collect();
            self.cursor_pos = self.input.len();
        }
        self.draw_input()
    }

    fn get_next_command(&mut self) -> Result<()> {
        // Check if we're already at or beyond the end of history
        let history_len = self.history.len();
        if self.history_index >= history_len {
            write!(self.stdout, "{BELL}")?;
            return Ok(());
        }
        // Save current input to the history
        self.history
            .set(self.history_index, self.input.iter().collect());
        self.history_index += 1;
        // Set input: either from stored_input (if at end) or from history
        if self.history_index == history_len {
            self.input = self.last_input.chars().collect();
        } else if let Some(cmd) = self.history.get(self.history_index) {
            self.input = cmd.chars().collect();
        }
        // Update cursor position and redraw
        self.cursor_pos = self.input.len();
        self.draw_input()
    }

    fn insert_char(&mut self, c: char) -> Result<()> {
        self.input.insert(self.cursor_pos, c);
        let suffix = &self.input[self.cursor_pos + 1..];
        write!(self.stdout, "{c}")?;
        for c in suffix {
            write!(self.stdout, "{}", c)?;
        }
        // Move the cursor back so it sits just after the inserted char
        if !suffix.is_empty() {
            write!(self.stdout, "{}", cursor::Left(suffix.len() as u16))?;
        }
        self.cursor_pos += 1;
        Ok(())
    }

    fn handle_tab(&mut self) -> Result<()> {
        let prefix = self.input[..self.cursor_pos].iter().collect::<String>();
        let matches = match &self.completion {
            Some(state) if state.prefix == prefix => state.matches.clone(),
            _ => {
                if prefix == "." || prefix == ".." {
                    vec![prefix.clone() + "/"]
                } else {
                    self.get_completions(&prefix)
                }
            }
        };
        match matches.len() {
            0 => write!(self.stdout, "{BELL}")?,
            1 => {
                let completion = matches[0].clone();
                self.update_input(&(completion + " "))?;
            }
            _ => {
                let common_prefix = longest_common_prefix(&matches);
                if common_prefix.len() > prefix.len() {
                    self.update_input(&common_prefix)?;
                } else {
                    // Show all matches
                    self.completion = Some(Completion::new(prefix.to_string(), matches.clone()));
                    write!(self.stdout, "{BELL}")?;
                    self.display_matches(&matches)?;
                }
            }
        }
        Ok(())
    }

    fn get_completions(&self, prefix: &str) -> Vec<String> {
        let last_token = prefix.split(" ").last().unwrap_or("");
        // Complete as files if there's a path separator or it's not the first word
        if last_token.contains('/') || prefix.contains(' ') {
            find_matching_files(last_token)
        } else {
            find_matching_executables(last_token)
        }
    }

    fn update_input(&mut self, completion: &str) -> Result<()> {
        // Find the start of the current token being completed
        let token_start = self.input[..self.cursor_pos]
            .iter()
            .rposition(|c| c.is_whitespace())
            .map_or(0, |pos| pos + 1);
        // For file paths, we only want to replace after the last slash
        let insertion_start = self.input[token_start..self.cursor_pos]
            .iter()
            .rposition(|&c| c == '/')
            .map_or(token_start, |pos| token_start + pos + 1);
        // Create the replacement text, adding a space for completed words
        let completion_chars: Vec<char> = completion.chars().collect();
        // Replace the partial text with the completion
        self.input
            .splice(insertion_start..self.cursor_pos, completion_chars.clone());
        self.cursor_pos = insertion_start + completion_chars.len();
        // Redraw the input with the completion
        self.draw_input()
    }

    fn display_matches(&mut self, matches: &[String]) -> Result<()> {
        writeln!(self.stdout, "\r")?;
        writeln!(self.stdout, "{}", matches.join("  "))?;
        self.draw_input()
    }

    fn show_completions(&mut self) -> Result<()> {
        let prefix = self.input[..self.cursor_pos].iter().collect::<String>();
        let matches = find_matching_executables(&prefix);
        if matches.is_empty() {
            write!(self.stdout, "{BELL}")?;
            return Ok(());
        }
        self.display_matches(&matches)
    }

    fn run(&self) -> Result<()> {
        self.stdout.suspend_raw_mode()?;
        let input = self.input.iter().collect::<String>();
        match Pipeline::new(&input, self.history.clone()) {
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
    // Builtin commands
    matches.extend(
        Builtin::iter()
            .map(|b| b.to_string().to_lowercase())
            .filter(|b| b.starts_with(prefix)),
    );
    // Executables in PATH
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

fn find_matching_files(prefix: &str) -> Vec<String> {
    if prefix == "." || prefix == ".." {
        return vec![format!("{}/", prefix)];
    }

    let mut matches = Vec::new();
    let (dir, stem) = if let Some(pos) = prefix.rfind('/') {
        prefix.split_at(pos + 1)
    } else {
        (".", prefix)
    };
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(stem) {
                    let mut candidate = name.to_string();
                    // Add trailing slash if it's a directory
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        candidate.push('/');
                    }
                    matches.push(candidate);
                }
            }
        }
    }
    matches.sort();
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
