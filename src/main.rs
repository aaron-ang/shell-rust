use anyhow::Result;

mod command;
mod history;
mod pipeline;
mod state;
mod token;
use state::Terminal;

fn main() -> Result<()> {
    let mut term = Terminal::new()?;
    term.start()
}
