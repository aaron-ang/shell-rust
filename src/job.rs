use std::{
    io::Write,
    process::Child,
    sync::{Arc, RwLock},
};

use anyhow::Result;

enum Status {
    Running,
    Done,
}

struct JobEntry {
    number: usize,
    child: Child,
    command: String,
    status: Status,
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

    pub fn add(&self, child: Child, command: String) {
        let mut entries = self.inner.write().unwrap();
        let number = entries.last().map_or(1, |e| e.number + 1);
        entries.push(JobEntry {
            number,
            child,
            command,
            status: Status::Running,
        });
    }

    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    pub fn print<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut entries = self.inner.write().unwrap();
        // Reap finished processes
        for entry in entries.iter_mut() {
            if let Status::Running = entry.status {
                if entry.child.try_wait()?.is_some() {
                    entry.status = Status::Done;
                }
            }
        }
        // Print all entries
        let len = entries.len();
        for (i, entry) in entries.iter().enumerate() {
            let marker = match i {
                _ if i + 1 == len => '+',
                _ if i + 2 == len => '-',
                _ => ' ',
            };
            let (status, suffix) = match entry.status {
                Status::Running => ("Running", " &"),
                Status::Done => ("Done", ""),
            };
            writeln!(
                writer,
                "[{}]{}  {:<24}{}{}",
                entry.number, marker, status, entry.command, suffix
            )?;
        }
        // Remove done jobs
        entries.retain(|e| matches!(e.status, Status::Running));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::process::Command;

    fn print_jobs(jobs: &Jobs) -> String {
        let mut buf = Cursor::new(Vec::new());
        jobs.print(&mut buf).unwrap();
        String::from_utf8(buf.into_inner()).unwrap()
    }

    #[test]
    fn single_running_job() {
        let jobs = Jobs::new();
        let child = Command::new("sleep").arg("10").spawn().unwrap();
        jobs.add(child, "sleep 10".into());
        let output = print_jobs(&jobs);
        assert_eq!(output, "[1]+  Running                 sleep 10 &\n");
    }

    #[test]
    fn done_job_shown_then_removed() {
        let jobs = Jobs::new();
        let child = Command::new("true").spawn().unwrap();
        jobs.add(child, "true".into());
        // Wait a moment for the process to exit
        std::thread::sleep(std::time::Duration::from_millis(50));

        let output = print_jobs(&jobs);
        assert!(output.contains("Done"));
        assert!(!output.ends_with("&\n"));

        // Second call should be empty
        let output = print_jobs(&jobs);
        assert!(output.is_empty());
    }

    #[test]
    fn three_jobs_markers() {
        let jobs = Jobs::new();
        for _ in 0..3 {
            let child = Command::new("sleep").arg("10").spawn().unwrap();
            jobs.add(child, "sleep 10".into());
        }
        let output = print_jobs(&jobs);
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].starts_with("[1] "));
        assert!(lines[1].starts_with("[2]-"));
        assert!(lines[2].starts_with("[3]+"));
    }

    #[test]
    fn empty_jobs_no_output() {
        let jobs = Jobs::new();
        let output = print_jobs(&jobs);
        assert!(output.is_empty());
    }
}
