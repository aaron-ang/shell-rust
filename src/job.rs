use std::{
    fmt,
    io::Write,
    sync::{Arc, RwLock},
};

use anyhow::Result;

#[allow(dead_code)]
struct JobEntry {
    number: usize,
    pid: u32,
    command: String,
}

impl fmt::Display for JobEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]+  {:<24}{} &", self.number, "Running", self.command)
    }
}

#[derive(Clone)]
pub struct Jobs {
    inner: Arc<RwLock<Vec<JobEntry>>>,
}

impl Jobs {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add(&self, pid: u32, command: String) {
        let mut entries = self.inner.write().unwrap();
        let number = entries.last().map_or(1, |e| e.number + 1);
        entries.push(JobEntry {
            number,
            pid,
            command,
        });
    }

    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    pub fn print<W: Write>(&self, writer: &mut W) -> Result<()> {
        let entries = self.inner.read().unwrap();
        for entry in entries.iter() {
            writeln!(writer, "{entry}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn add_and_print() {
        let jobs = Jobs::new();
        jobs.add(1234, "sleep 10".into());
        let mut buf = Cursor::new(Vec::new());
        jobs.print(&mut buf).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert_eq!(output, "[1]+  Running                 sleep 10 &\n");
    }

    #[test]
    fn job_numbers_increment() {
        let jobs = Jobs::new();
        jobs.add(100, "sleep 1".into());
        jobs.add(200, "sleep 2".into());
        let mut buf = Cursor::new(Vec::new());
        jobs.print(&mut buf).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.starts_with("[1]+"));
        assert!(output.contains("[2]+"));
    }

    #[test]
    fn empty_jobs_no_output() {
        let jobs = Jobs::new();
        let mut buf = Cursor::new(Vec::new());
        jobs.print(&mut buf).unwrap();
        let output = String::from_utf8(buf.into_inner()).unwrap();
        assert!(output.is_empty());
    }
}
