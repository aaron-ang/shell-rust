use std::io::{self, Write};

use anyhow::Result;
use termion::{event::Key, input::TermRead, raw::IntoRawMode};

mod command;
mod state;
use state::InputState;

fn main() -> Result<()> {
    let mut stdout = io::stdout().into_raw_mode()?;

    'outer: loop {
        stdout.activate_raw_mode()?;
        let mut input = InputState::new();
        input.redraw_prompt(&mut stdout)?;
        stdout.flush()?;

        for key in io::stdin().keys().filter_map(Result::ok) {
            let result = match key {
                Key::Char('\n') => {
                    writeln!(stdout, "\r")?;
                    break;
                }
                Key::Char('\t') => input.handle_tab(&mut stdout),
                Key::Ctrl('c') => {
                    writeln!(stdout)?;
                    continue 'outer;
                }
                Key::Ctrl('d') => {
                    if input.is_empty() {
                        return Ok(());
                    }
                    input.show_completions(&mut stdout)
                }
                Key::Backspace => input.backspace(&mut stdout),
                Key::Left => input.move_cursor_left(&mut stdout),
                Key::Right => input.move_cursor_right(&mut stdout),
                Key::Char(c) => {
                    input.insert_char(c);
                    write!(stdout, "{}", c)?;
                    Ok(())
                }
                _ => Ok(()),
            };

            if let Err(e) = result {
                eprintln!("Error: {}", e);
                continue 'outer;
            }
            stdout.flush()?;
        }

        stdout.suspend_raw_mode()?;
        input.run()?;
    }
}
