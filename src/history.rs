use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    sync::{Arc, RwLock},
};

use anyhow::Result;

const HISTFILE_ENV: &str = "HISTFILE";

#[derive(Clone)]
pub struct History {
    inner: Arc<RwLock<HistoryData>>,
}

struct HistoryData {
    entries: Vec<String>,
    append_index: usize,
}

impl History {
    pub fn open() -> Self {
        let histfile = env::var(HISTFILE_ENV).unwrap_or_default();
        let entries = fs::read_to_string(histfile)
            .map(|s| s.lines().map(String::from).collect::<Vec<_>>())
            .unwrap_or_default();
        let len = entries.len();
        let data = HistoryData {
            entries,
            append_index: len,
        };
        History {
            inner: Arc::new(RwLock::new(data)),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.read().unwrap().entries.len()
    }

    pub fn add(&mut self, command: String) {
        // Don't add empty commands or duplicates of the last command
        if command.is_empty() {
            return;
        }
        let mut data = self.inner.write().unwrap();
        let is_duplicate = data.entries.last().map_or(false, |last| last == &command);
        if is_duplicate {
            return;
        }
        data.entries.push(command);
    }

    pub fn get(&self, index: usize) -> Option<String> {
        self.inner.read().unwrap().entries.get(index).cloned()
    }

    pub fn set(&mut self, index: usize, command: String) {
        let mut data = self.inner.write().unwrap();
        if index < data.entries.len() {
            data.entries[index] = command;
        }
    }

    pub fn clear(&mut self) {
        self.inner.write().unwrap().entries.clear();
    }

    pub fn print<W: Write>(&self, writer: &mut W, limit: Option<usize>) -> Result<()> {
        let data = self.inner.read().unwrap();
        let limit = limit.unwrap_or(data.entries.len());
        let start = data.entries.len().saturating_sub(limit);
        for (i, cmd) in data.entries.iter().skip(start).enumerate() {
            writeln!(writer, "{:5} {}", start + i + 1, cmd)?;
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let histfile = env::var(HISTFILE_ENV).unwrap_or_default();
        self.append_to_file(histfile)
    }

    pub fn append_from_file<P: AsRef<Path>>(&self, path: P) {
        let new_entries = fs::read_to_string(path)
            .map(|s| s.lines().map(String::from).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut data = self.inner.write().unwrap();
        data.entries.extend(new_entries);
    }

    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        for entry in &self.inner.read().unwrap().entries {
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()?;
        Ok(())
    }

    pub fn append_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut data = self.inner.write().unwrap();
        let file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;
        let mut writer = BufWriter::new(file);
        for entry in data.entries.iter().skip(data.append_index) {
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()?;
        data.append_index = data.entries.len();
        Ok(())
    }
}
