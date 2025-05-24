use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    sync::{Arc, RwLock},
};

use anyhow::Result;

#[derive(Clone)]
pub struct History {
    entries: Arc<RwLock<Vec<String>>>,
}

const HISTORY_FILE_PATH: &str = ".history";

impl History {
    pub fn open() -> Self {
        let entries = if let Ok(history) = fs::read_to_string(HISTORY_FILE_PATH) {
            history.lines().map(String::from).collect()
        } else {
            Vec::new()
        };
        Self {
            entries: Arc::new(RwLock::new(entries)),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    pub fn add(&mut self, command: String) {
        // Don't add empty commands or duplicates of the last command
        if command.is_empty()
            || self
                .entries
                .read()
                .unwrap()
                .last()
                .map_or(false, |last| last == &command)
        {
            return;
        }
        self.entries.write().unwrap().push(command.clone());
    }

    pub fn get(&self, index: usize) -> Option<String> {
        self.entries.read().unwrap().get(index).cloned()
    }

    pub fn set(&mut self, index: usize, command: String) {
        if index < self.entries.read().unwrap().len() {
            self.entries.write().unwrap()[index] = command;
        }
    }

    pub fn clear(&mut self) {
        self.entries.write().unwrap().clear();
    }

    pub fn print<W: Write>(&self, writer: &mut W, limit: Option<usize>) -> Result<()> {
        let entries = self.entries.read().unwrap();
        let limit = limit.unwrap_or(entries.len());
        let start = entries.len().saturating_sub(limit);
        for (i, cmd) in entries.iter().skip(start).enumerate() {
            writeln!(writer, "    {} {}", i + 1, cmd)?;
        }
        Ok(())
    }

    pub fn save(&self) -> std::io::Result<()> {
        let file = File::create(HISTORY_FILE_PATH)?;
        let mut writer = BufWriter::new(file);
        for entry in self.entries.read().unwrap().iter() {
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()
    }
}
