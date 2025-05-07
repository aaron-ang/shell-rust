use anyhow::Result;

mod command;
mod state;
use state::Terminal;

fn main() -> Result<()> {
    let mut term = Terminal::new()?;
    term.start()
}
