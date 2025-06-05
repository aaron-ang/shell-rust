use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    sync::{Arc, RwLock},
};

use anyhow::Result;

#[derive(Clone)]
pub struct History {
    entries: Arc<RwLock<Vec<String>>>,
}

impl History {
    pub fn open() -> Self {
        let histfile = env::var("HISTFILE").unwrap_or_default();
        let entries = if let Ok(history) = fs::read_to_string(histfile) {
            history.lines().map(String::from).collect()
        } else {
            Vec::new()
        };
        Self {
            entries: Arc::new(RwLock::new(entries)),
        }
    }

    pub fn append_from_file<P: AsRef<Path>>(&self, path: P) {
        let entries = if let Ok(history) = fs::read_to_string(path) {
            history.lines().map(String::from).collect()
        } else {
            Vec::new()
        };
        self.entries.write().unwrap().extend(entries);
    }

    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    pub fn add(&mut self, command: String) {
        // Don't add empty commands or duplicates of the last command
        if command.is_empty() {
            return;
        }
        let is_duplicate = self
            .entries
            .read()
            .unwrap()
            .last()
            .map_or(false, |last| last == &command);
        if is_duplicate {
            return;
        }
        self.entries.write().unwrap().push(command);
    }

    pub fn get(&self, index: usize) -> Option<String> {
        self.entries.read().unwrap().get(index).cloned()
    }

    pub fn set(&mut self, index: usize, command: String) {
        if index < self.entries.read().unwrap().len() {
            let mut entries = self.entries.write().unwrap();
            entries[index] = command;
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
            writeln!(writer, "{:5} {}", start + i + 1, cmd)?;
        }
        Ok(())
    }

    pub fn save(&self) -> std::io::Result<()> {
        let histfile = env::var("HISTFILE").unwrap_or_default();
        let file = File::create(histfile)?;
        let mut writer = BufWriter::new(file);
        for entry in self.entries.read().unwrap().iter() {
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()
    }
}
