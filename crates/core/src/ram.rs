use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::error::{Error, Result};
use crate::util::percent;

/// Point-in-time view of physical memory and swap, in bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RamSnapshot {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub swap_total: u64,
    pub swap_used: u64,
}

impl RamSnapshot {
    pub fn used_percent(&self) -> f64 {
        percent(self.used, self.total)
    }
}

/// Memory consumed by a single process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessMemory {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
}

/// Reads RAM and per-process memory usage. Keep one instance alive and call
/// it repeatedly (e.g. once per second) rather than creating a new one each time.
pub struct RamMonitor {
    sys: System,
}

impl RamMonitor {
    pub fn new() -> Result<Self> {
        let sys = System::new().map_err(|e| Error::Sys(e.to_string()))?;
        Ok(Self { sys })
    }

    pub fn snapshot(&mut self) -> RamSnapshot {
        self.sys.refresh_memory();
        RamSnapshot {
            total: self.sys.total_memory(),
            used: self.sys.used_memory(),
            available: self.sys.available_memory(),
            swap_total: self.sys.total_swap(),
            swap_used: self.sys.used_swap(),
        }
    }

    /// The `limit` processes using the most memory, largest first.
    pub fn top_processes(&mut self, limit: usize) -> Vec<ProcessMemory> {
        self.refresh_processes();
        let mut processes: Vec<ProcessMemory> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, process)| ProcessMemory {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().into_owned(),
                memory_bytes: process.memory(),
            })
            .collect();
        processes.sort_by(|a, b| {
            b.memory_bytes
                .cmp(&a.memory_bytes)
                .then_with(|| a.pid.cmp(&b.pid))
        });
        processes.truncate(limit);
        processes
    }

    /// Asks the OS to terminate a process. Refuses to terminate this process.
    pub fn kill_process(&mut self, pid: u32) -> Result<()> {
        if pid == std::process::id() {
            return Err(Error::RefusedSelf);
        }
        self.refresh_processes();
        let process = self
            .sys
            .process(Pid::from_u32(pid))
            .ok_or(Error::ProcessNotFound(pid))?;
        if process.kill() {
            Ok(())
        } else {
            Err(Error::KillFailed(pid))
        }
    }

    fn refresh_processes(&mut self) {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_consistent() {
        let mut monitor = RamMonitor::new().expect("system info");
        let snap = monitor.snapshot();
        assert!(snap.total > 0);
        assert!(snap.used <= snap.total);
        assert!((0.0..=100.0).contains(&snap.used_percent()));
    }

    #[test]
    fn top_processes_are_sorted_and_limited() {
        let mut monitor = RamMonitor::new().expect("system info");
        let top = monitor.top_processes(5);
        assert!(top.len() <= 5);
        assert!(top
            .windows(2)
            .all(|w| w[0].memory_bytes >= w[1].memory_bytes));
    }

    #[test]
    fn refuses_to_kill_itself() {
        let mut monitor = RamMonitor::new().expect("system info");
        let err = monitor.kill_process(std::process::id()).unwrap_err();
        assert!(matches!(err, Error::RefusedSelf));
    }
}
