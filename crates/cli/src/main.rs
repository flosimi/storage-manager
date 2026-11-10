use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use serde::Serialize;
use storage_manager_core::alerts::{self, AlertRules};
use storage_manager_core::cleanup::{self, CleanupCategory};
use storage_manager_core::disk;
use storage_manager_core::ram::RamMonitor;
use storage_manager_core::util::format_bytes;

/// Inspect RAM and disk usage from the terminal.
#[derive(Parser)]
#[command(name = "storage-manager", version, about)]
struct Cli {
    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show RAM usage and the processes using the most memory.
    Ram {
        /// How many processes to list.
        #[arg(long, default_value_t = 10)]
        top: usize,
    },
    /// List mounted volumes and their free space.
    Disks,
    /// Show where space goes inside a folder.
    Scan {
        path: PathBuf,
        /// How many of the largest files to list.
        #[arg(long, default_value_t = 10)]
        top: usize,
    },
    /// List temporary files that could be cleaned up. Never deletes anything.
    Cleanup {
        /// Only include files not modified for at least this many days.
        #[arg(long, default_value_t = 7)]
        min_age_days: u64,
        /// How many of the largest candidates to list.
        #[arg(long, default_value_t = 20)]
        top: usize,
    },
    /// Check RAM and disks against alert thresholds.
    Alerts {
        /// Alert when RAM usage exceeds this percentage.
        #[arg(long, default_value_t = 90.0)]
        ram_max: f64,
        /// Alert when a disk's usage exceeds this percentage.
        #[arg(long, default_value_t = 90.0)]
        disk_max: f64,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn run(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    match &cli.command {
        Command::Ram { top } => {
            let mut monitor = RamMonitor::new()?;
            let snapshot = monitor.snapshot();
            let processes = monitor.top_processes(*top);
            if cli.json {
                return print_json(&serde_json::json!({
                    "ram": snapshot,
                    "top_processes": processes,
                }));
            }
            println!(
                "RAM:  {} used of {} ({:.0}%), {} available",
                format_bytes(snapshot.used),
                format_bytes(snapshot.total),
                snapshot.used_percent(),
                format_bytes(snapshot.available)
            );
            println!(
                "Swap: {} used of {}",
                format_bytes(snapshot.swap_used),
                format_bytes(snapshot.swap_total)
            );
            println!("\nTop processes by memory:");
            for p in &processes {
                println!(
                    "  {:>10}  {:>7}  {}",
                    format_bytes(p.memory_bytes),
                    p.pid,
                    p.name
                );
            }
        }
        Command::Disks => {
            let disks = disk::list_disks()?;
            if cli.json {
                return print_json(&disks);
            }
            for d in &disks {
                println!(
                    "{:<20} {:>10} free of {:>10} ({:.0}% used, {:?}, {})",
                    d.mount_point,
                    format_bytes(d.available),
                    format_bytes(d.total),
                    d.used_percent(),
                    d.kind,
                    d.file_system
                );
            }
        }
        Command::Scan { path, top } => {
            let result = disk::scan(path, *top)?;
            if cli.json {
                return print_json(&result);
            }
            println!(
                "{}: {} in {} files, {} folders ({} unreadable)",
                result.root,
                format_bytes(result.total_bytes),
                result.file_count,
                result.dir_count,
                result.skipped
            );
            println!("\nBy folder:");
            for e in result.children.iter().take(15) {
                let marker = if e.is_dir { "/" } else { "" };
                println!("  {:>10}  {}{}", format_bytes(e.size), e.name, marker);
            }
            println!("\nLargest files:");
            for e in &result.largest_files {
                println!("  {:>10}  {}", format_bytes(e.size), e.path);
            }
        }
        Command::Cleanup { min_age_days, top } => {
            let min_age = Duration::from_secs(min_age_days * 86_400);
            let plan = cleanup::plan(
                &[CleanupCategory::UserTemp, CleanupCategory::SystemTemp],
                min_age,
            )?;
            if cli.json {
                return print_json(&plan);
            }
            println!(
                "{} files could be cleaned, freeing {} ({} unreadable). Nothing was deleted.",
                plan.candidates.len(),
                format_bytes(plan.total_bytes),
                plan.skipped
            );
            for c in plan.candidates.iter().take(*top) {
                println!(
                    "  {:>10}  {:>4}d  {}",
                    format_bytes(c.size),
                    c.age_days,
                    c.path
                );
            }
        }
        Command::Alerts { ram_max, disk_max } => {
            let mut monitor = RamMonitor::new()?;
            let snapshot = monitor.snapshot();
            let disks = disk::list_disks()?;
            let rules = AlertRules {
                ram_used_percent_max: *ram_max,
                disk_used_percent_max: *disk_max,
            };
            let found = alerts::evaluate(&rules, &snapshot, &disks);
            if cli.json {
                return print_json(&found);
            }
            if found.is_empty() {
                println!("All good: RAM and disks are within thresholds.");
            }
            for a in &found {
                println!("[{:?}] {}", a.severity, a.message);
            }
        }
    }
    Ok(())
}
