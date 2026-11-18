//! Sorting and trimming process lists, shared by the sampler, the command line and the MCP server.

use crate::model::ProcessSample;
use serde::{Deserialize, Serialize};

/// What to order processes by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProcessSort {
    #[default]
    Cpu,
    Memory,
    DiskRead,
    DiskWrite,
    Name,
}

impl ProcessSort {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "cpu" => Some(Self::Cpu),
            "memory" | "mem" => Some(Self::Memory),
            "disk_read" | "read" => Some(Self::DiskRead),
            "disk_write" | "write" => Some(Self::DiskWrite),
            "name" => Some(Self::Name),
            _ => None,
        }
    }
}

/// Sorts in place: biggest first for numbers, A to Z for names. Ties are broken by pid so the order is stable.
pub fn sort_processes(processes: &mut [ProcessSample], by: ProcessSort) {
    processes.sort_by(|a, b| {
        let order = match by {
            ProcessSort::Cpu => b.cpu_pct.total_cmp(&a.cpu_pct),
            ProcessSort::Memory => b.memory_bytes.cmp(&a.memory_bytes),
            ProcessSort::DiskRead => b.disk_read_bps.total_cmp(&a.disk_read_bps),
            ProcessSort::DiskWrite => b.disk_write_bps.total_cmp(&a.disk_write_bps),
            ProcessSort::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        };
        order.then(a.pid.cmp(&b.pid))
    });
}

/// The first `limit` processes by `by`, optionally only those whose name contains `filter` (ignoring case).
pub fn top(processes: &[ProcessSample], by: ProcessSort, limit: usize, filter: Option<&str>) -> Vec<ProcessSample> {
    let needle = filter.map(str::to_lowercase).filter(|f| !f.is_empty());
    let mut picked: Vec<ProcessSample> = processes
        .iter()
        .filter(|p| needle.as_ref().is_none_or(|n| p.name.to_lowercase().contains(n)))
        .cloned()
        .collect();
    sort_processes(&mut picked, by);
    picked.truncate(limit);
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: i32, name: &str, cpu: f64, mem: u64) -> ProcessSample {
        ProcessSample { pid, name: name.into(), cpu_pct: cpu, memory_bytes: mem, ..Default::default() }
    }

    fn sample() -> Vec<ProcessSample> {
        vec![p(1, "launchd", 0.1, 10), p(2, "Safari", 40.0, 900), p(3, "Xcode", 40.0, 2_000), p(4, "mds", 5.0, 50)]
    }

    #[test]
    fn sorts_by_cpu_with_pid_as_the_tiebreak() {
        let mut v = sample();
        sort_processes(&mut v, ProcessSort::Cpu);
        assert_eq!(v.iter().map(|p| p.pid).collect::<Vec<_>>(), [2, 3, 4, 1]);
    }

    #[test]
    fn sorts_by_memory_and_name() {
        let mut v = sample();
        sort_processes(&mut v, ProcessSort::Memory);
        assert_eq!(v[0].name, "Xcode");
        sort_processes(&mut v, ProcessSort::Name);
        assert_eq!(v.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["launchd", "mds", "Safari", "Xcode"]);
    }

    #[test]
    fn top_filters_and_limits() {
        let v = sample();
        assert_eq!(top(&v, ProcessSort::Cpu, 2, None).len(), 2);
        let found = top(&v, ProcessSort::Cpu, 10, Some("SAFA"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Safari");
        assert_eq!(top(&v, ProcessSort::Cpu, 10, Some("")).len(), 4, "an empty filter matches everything");
        assert!(top(&v, ProcessSort::Cpu, 0, None).is_empty());
    }

    #[test]
    fn knows_its_sort_keys() {
        assert_eq!(ProcessSort::from_key("mem"), Some(ProcessSort::Memory));
        assert_eq!(ProcessSort::from_key("write"), Some(ProcessSort::DiskWrite));
        assert_eq!(ProcessSort::from_key("nope"), None);
    }
}
