use std::{
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
        let len = entries.len();
        for (i, entry) in entries.iter().enumerate() {
            let marker = match i {
                _ if i + 1 == len => '+',
                _ if i + 2 == len => '-',
                _ => ' ',
            };
            writeln!(
                writer,
                "[{}]{}  {:<24}{} &",
                entry.number, marker, "Running", entry.command
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn print_jobs(jobs: &Jobs) -> String {
        let mut buf = Cursor::new(Vec::new());
        jobs.print(&mut buf).unwrap();
        String::from_utf8(buf.into_inner()).unwrap()
    }

    #[test]
    fn single_job_has_plus_marker() {
        let jobs = Jobs::new();
        jobs.add(1234, "sleep 10".into());
        assert_eq!(
            print_jobs(&jobs),
            "[1]+  Running                 sleep 10 &\n"
        );
    }

    #[test]
    fn two_jobs_markers() {
        let jobs = Jobs::new();
        jobs.add(100, "sleep 10".into());
        jobs.add(200, "sleep 20".into());
        let output = print_jobs(&jobs);
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].starts_with("[1]-"));
        assert!(lines[1].starts_with("[2]+"));
    }

    #[test]
    fn three_jobs_markers() {
        let jobs = Jobs::new();
        jobs.add(100, "sleep 10".into());
        jobs.add(200, "sleep 20".into());
        jobs.add(300, "sleep 30".into());
        let output = print_jobs(&jobs);
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].starts_with("[1] "));
        assert!(lines[1].starts_with("[2]-"));
        assert!(lines[2].starts_with("[3]+"));
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
