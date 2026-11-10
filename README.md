# Storage Manager

[![CI](https://github.com/flosimi/storage-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/flosimi/storage-manager/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A free, local, ad-free tool for seeing and managing what uses your PC's
**RAM** and **disk space**. Written in Rust, built for Windows.

> **Status: early development (v0.1).** The core library and a command-line
> interface work today. The desktop app is next. See the [roadmap](#roadmap).

## Why

Task Manager is too basic, and the popular cleaners are bloated, full of
upsells, or locked behind a subscription. Storage Manager aims to be the
opposite: small, fast, transparent, and free.

- No ads, no accounts, no subscription, no telemetry
- Everything runs locally
- Safe by default: it shows what it would remove before it removes anything

## What works today

| Feature | Command | State |
| --- | --- | --- |
| RAM usage and top processes | `ram` | ✅ |
| Volumes and free space | `disks` | ✅ |
| Where disk space goes (per folder, largest files) | `scan <path>` | ✅ |
| Temp files that could be cleaned (dry run only) | `cleanup` | ✅ never deletes |
| Threshold alerts for RAM and disks | `alerts` | ✅ |
| Disk health (SMART) | | planned |
| Safe deletion to the Recycle Bin | | planned |
| Desktop app | | planned |

## Usage

Requires [Rust](https://rustup.rs) (stable).

```powershell
git clone https://github.com/flosimi/storage-manager
cd storage-manager

cargo run -p storage-manager-cli -- ram --top 5
cargo run -p storage-manager-cli -- disks
cargo run -p storage-manager-cli -- scan C:\Users --top 10
cargo run -p storage-manager-cli -- cleanup --min-age-days 7
cargo run -p storage-manager-cli -- alerts --ram-max 85 --disk-max 90
```

Add `--json` to any command for machine-readable output.

## Development

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same checks on Windows and Linux. See
[docs/architecture.md](docs/architecture.md) for how the pieces fit together.

## Roadmap

- **v0.1** Core + CLI (this release)
- **v0.2** Desktop UI with Tauri: RAM dashboard and disk explorer
- **v0.3** Safe deletion, disk health, tray alerts
- **v0.4** Windows installer and automated releases

## License

[MIT](LICENSE)
