# Architecture

## Goal

A free, local, ad-free desktop tool for Windows that shows and manages what
uses a PC's memory:

- **RAM** — working memory.
- **Persistent storage** (SSD/HDD). Firmware (BIOS/UEFI) is out of scope.

## Principles

- No ads, no subscription, no telemetry. Everything runs locally.
- Clean, fast UI.
- Safe by default: cleanup is a dry-run first, deletion goes to the Recycle Bin
  after confirmation, and only recognised temp folders are ever touched.

## Layout

```
crates/core   library, no UI: ram, disk, cleanup, alerts (health planned)
crates/cli    command-line front end over the core
app/          Tauri v2 + React + TypeScript desktop app (planned)
```

The core returns plain, `serde`-serialisable structs, so the CLI prints them
and the desktop app can pass them straight to the UI as JSON.

## Key decisions

- **Scanning** reports apparent file sizes, does not follow symlinks, and
  aggregates sizes per immediate child so the UI can drill down one level at a
  time (treemap).
- **Cleanup** is split into planning (`cleanup::plan`) and execution. Only
  planning exists today; `validate_root` refuses any folder that does not look
  like a temp directory.
- **Alerts** are pure functions over readings, so they are easy to test and
  independent of how often the UI polls.
- **Disk health (SMART)** will read Windows' storage reliability counters via
  WMI, with `smartctl` as a possible fallback.

## Roadmap

| Version | Scope |
| --- | --- |
| v0.1 | Core + CLI: RAM, disks, scan, cleanup dry-run, alerts |
| v0.2 | Tauri UI: RAM dashboard, disk explorer |
| v0.3 | Safe deletion, disk health, tray alerts |
| v0.4 | Windows installer and automated releases |
