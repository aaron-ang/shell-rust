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

    /// Check for exited jobs, print Done lines, and remove them.
    pub fn reap<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut entries = self.inner.write().unwrap();
        // Check for newly finished processes
        for entry in entries.iter_mut() {
            if let Status::Running = entry.status {
                if entry.child.try_wait()?.is_some() {
                    entry.status = Status::Done;
                }
            }
        }
        // Print and remove done jobs
        let len = entries.len();
        for (i, entry) in entries.iter().enumerate() {
            if let Status::Done = entry.status {
                let marker = marker(i, len);
                writeln!(
                    writer,
                    "[{}]{}  {:<24}{}",
                    entry.number, marker, "Done", entry.command
                )?;
            }
        }
        entries.retain(|e| matches!(e.status, Status::Running));
        Ok(())
    }

    /// Check statuses, list all jobs in order, then remove Done ones.
    pub fn print<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut entries = self.inner.write().unwrap();
        for entry in entries.iter_mut() {
            if let Status::Running = entry.status {
                if entry.child.try_wait()?.is_some() {
                    entry.status = Status::Done;
                }
            }
        }
        let len = entries.len();
        for (i, entry) in entries.iter().enumerate() {
            let m = marker(i, len);
            let (status, suffix) = match entry.status {
                Status::Running => ("Running", " &"),
                Status::Done => ("Done", ""),
            };
            writeln!(
                writer,
                "[{}]{}  {:<24}{}{}",
                entry.number, m, status, entry.command, suffix
            )?;
        }
        entries.retain(|e| matches!(e.status, Status::Running));
        Ok(())
    }
}

fn marker(index: usize, len: usize) -> char {
    match index {
        _ if index + 1 == len => '+',
        _ if index + 2 == len => '-',
        _ => ' ',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::process::Command;

    fn collect<F: Fn(&Jobs, &mut Cursor<Vec<u8>>)>(jobs: &Jobs, f: F) -> String {
        let mut buf = Cursor::new(Vec::new());
        f(jobs, &mut buf);
        String::from_utf8(buf.into_inner()).unwrap()
    }

    fn print_jobs(jobs: &Jobs) -> String {
        collect(jobs, |j, w| {
            j.print(w).unwrap();
        })
    }

    fn reap_jobs(jobs: &Jobs) -> String {
        collect(jobs, |j, w| {
            j.reap(w).unwrap();
        })
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
        std::thread::sleep(std::time::Duration::from_millis(50));

        let output = print_jobs(&jobs);
        assert!(output.contains("Done"));
        assert!(!output.contains("&"));

        let output = print_jobs(&jobs);
        assert!(output.is_empty());
    }

    #[test]
    fn reap_prints_done_and_removes() {
        let jobs = Jobs::new();
        let child = Command::new("true").spawn().unwrap();
        jobs.add(child, "true".into());
        std::thread::sleep(std::time::Duration::from_millis(50));

        let output = reap_jobs(&jobs);
        assert!(output.contains("Done"));

        // After reap, jobs list is empty
        let output = print_jobs(&jobs);
        assert!(output.is_empty());
    }

    #[test]
    fn reap_skips_running_jobs() {
        let jobs = Jobs::new();
        let child = Command::new("sleep").arg("10").spawn().unwrap();
        jobs.add(child, "sleep 10".into());

        let output = reap_jobs(&jobs);
        assert!(output.is_empty());
        assert_eq!(jobs.len(), 1);
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
    fn mixed_running_and_done_listed_in_order() {
        let jobs = Jobs::new();
        let child1 = Command::new("sleep").arg("10").spawn().unwrap();
        jobs.add(child1, "sleep 10".into());
        let child2 = Command::new("true").spawn().unwrap();
        jobs.add(child2, "true".into());
        let child3 = Command::new("sleep").arg("10").spawn().unwrap();
        jobs.add(child3, "sleep 10".into());
        std::thread::sleep(std::time::Duration::from_millis(50));

        let output = print_jobs(&jobs);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("Running"));
        assert!(lines[1].contains("Done"));
        assert!(lines[2].contains("Running"));

        // After print, done job is removed
        let output = print_jobs(&jobs);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn empty_jobs_no_output() {
        let jobs = Jobs::new();
        let output = print_jobs(&jobs);
        assert!(output.is_empty());
    }
}
