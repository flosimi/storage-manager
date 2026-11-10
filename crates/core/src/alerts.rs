//! Threshold rules that turn raw readings into user-facing alerts.

use serde::Serialize;

use crate::disk::DiskInfo;
use crate::ram::RamSnapshot;
use crate::util::format_bytes;

/// Usage at or above this percentage is reported as critical.
const CRITICAL_PERCENT: f64 = 95.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlertRules {
    pub ram_used_percent_max: f64,
    pub disk_used_percent_max: f64,
}

impl Default for AlertRules {
    fn default() -> Self {
        Self {
            ram_used_percent_max: 90.0,
            disk_used_percent_max: 90.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AlertSource {
    Ram,
    Disk { mount_point: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Alert {
    pub source: AlertSource,
    pub severity: Severity,
    pub message: String,
}

fn severity_for(used_percent: f64) -> Severity {
    if used_percent >= CRITICAL_PERCENT {
        Severity::Critical
    } else {
        Severity::Warning
    }
}

/// Checks current readings against `rules`. Volumes with no capacity are ignored.
pub fn evaluate(rules: &AlertRules, ram: &RamSnapshot, disks: &[DiskInfo]) -> Vec<Alert> {
    let mut alerts = Vec::new();

    let ram_percent = ram.used_percent();
    if ram_percent > rules.ram_used_percent_max {
        alerts.push(Alert {
            source: AlertSource::Ram,
            severity: severity_for(ram_percent),
            message: format!(
                "RAM is {ram_percent:.0}% full ({} of {} used)",
                format_bytes(ram.used),
                format_bytes(ram.total)
            ),
        });
    }

    for disk in disks.iter().filter(|d| d.total > 0) {
        let percent = disk.used_percent();
        if percent > rules.disk_used_percent_max {
            alerts.push(Alert {
                source: AlertSource::Disk {
                    mount_point: disk.mount_point.clone(),
                },
                severity: severity_for(percent),
                message: format!(
                    "{} is {percent:.0}% full ({} free)",
                    disk.mount_point,
                    format_bytes(disk.available)
                ),
            });
        }
    }

    alerts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::DriveKind;

    fn ram(used: u64, total: u64) -> RamSnapshot {
        RamSnapshot {
            total,
            used,
            available: total - used,
            swap_total: 0,
            swap_used: 0,
        }
    }

    fn disk(mount: &str, used: u64, total: u64) -> DiskInfo {
        DiskInfo {
            name: mount.into(),
            mount_point: mount.into(),
            file_system: "NTFS".into(),
            kind: DriveKind::Ssd,
            total,
            available: total - used,
            removable: false,
        }
    }

    #[test]
    fn no_alerts_below_thresholds() {
        let alerts = evaluate(
            &AlertRules::default(),
            &ram(50, 100),
            &[disk("C:\\", 50, 100)],
        );
        assert!(alerts.is_empty());
    }

    #[test]
    fn warning_then_critical() {
        let alerts = evaluate(
            &AlertRules::default(),
            &ram(92, 100),
            &[disk("C:\\", 97, 100), disk("D:\\", 91, 100)],
        );
        assert_eq!(alerts.len(), 3);
        assert_eq!(alerts[0].severity, Severity::Warning);
        assert_eq!(alerts[1].severity, Severity::Critical);
        assert_eq!(alerts[2].severity, Severity::Warning);
        assert!(alerts[1].message.contains("C:\\"));
    }

    #[test]
    fn exactly_at_threshold_does_not_alert() {
        let alerts = evaluate(&AlertRules::default(), &ram(90, 100), &[]);
        assert!(alerts.is_empty());
    }

    #[test]
    fn zero_capacity_disks_are_ignored() {
        let alerts = evaluate(&AlertRules::default(), &ram(1, 100), &[disk("X:\\", 0, 0)]);
        assert!(alerts.is_empty());
    }
}
