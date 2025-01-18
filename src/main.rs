use std::io::{self, Write};

use anyhow::Result;
use termion::{clear, event::Key, input::TermRead, raw::IntoRawMode};

mod command;
mod state;
use state::InputState;

fn main() -> Result<()> {
    let mut stdout = io::stdout().into_raw_mode()?;

    'outer: loop {
        stdout.activate_raw_mode()?;

        write!(stdout, "\r{}", clear::CurrentLine)?;
        print!("$ ");
        stdout.flush()?;

        let mut input = InputState::new();

        for key in io::stdin().keys().filter_map(Result::ok) {
            match key {
                Key::Char('\n') => {
                    writeln!(stdout, "\r")?;
                    break;
                }
                Key::Char('\t') => input.handle_tab(&mut stdout)?,
                Key::Ctrl('c') => {
                    writeln!(stdout)?;
                    continue 'outer;
                }
                Key::Ctrl('d') => {
                    if input.is_empty() {
                        return Ok(());
                    } else {
                        input.show_completions(&mut stdout)?;
                    }
                }
                Key::Backspace => input.backspace(&mut stdout)?,
                Key::Left => input.move_cursor_left(&mut stdout)?,
                Key::Right => input.move_cursor_right(&mut stdout)?,
                Key::Char(c) => {
                    input.insert_char(c);
                    write!(stdout, "{}", c)?;
                }
                _ => {}
            }
            stdout.flush()?;
        }

        stdout.suspend_raw_mode()?;

        input.run()?;
    }
}
