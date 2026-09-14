# oops-libs Host Developer Guide

Welcome to the **oops-libs** developer guide.

This guide provides practical instructions for **Rust developers and contributors** building host-side command-line or GUI utilities within the OOPS ecosystem.

If you are an AI coding agent or systems architect seeking individual crate source listings or compile-time macro designs, see the **[Technical Reference](README.md)**.

---

## Table of Contents

1. [Shared Directory Standard (`oops-paths`)](#1-shared-directory-standard-oops-paths)
2. [Unified Logging (`oops-log`)](#2-unified-logging-oops-log)
   - [Configuring the `OOPS_LOG` Environment Variable](#configuring-the-oops_log-environment-variable)
   - [Log Levels & Filtering](#log-levels--filtering)
3. [Deterministic Build Stamps (`oops-build`)](#3-deterministic-build-stamps-oops-build)
4. [Using `oops-libs` in a New Rust Tool](#4-using-oops-libs-in-a-new-rust-tool)

---

## 1. Shared Directory Standard (`oops-paths`)

Rather than each tool inventing its own configuration directory or scattering files across the filesystem, all OOPS host utilities share a unified data root managed by `oops-paths`:

| OS | Default Root Directory |
| :--- | :--- |
| **Windows** | `%APPDATA%\OOPS\` |
| **Linux / macOS** | `~/.local/share/OOPS/` (or `$XDG_DATA_HOME/OOPS/`) |

### Standard Directory Hierarchy:
```text
%APPDATA%\OOPS\
├── targets/         <- Registered console targets (shared between pros and orbistoun)
├── titles/          <- Staged titles and homebrew applications
├── saves/           <- Mounted game save files and overlays
├── reports/         <- Conformance probe outputs and hardware logs
└── logs/            <- Runtime session telemetry
```

### Using in Rust:
```rust
use oops_paths::Paths;

let paths = Paths::resolve()?;
let targets_dir = paths.targets_dir();
let reports_dir = paths.reports_dir();
```

---

## 2. Unified Logging (`oops-log`)

All tools in the collection initialize logging through `oops-log`. This guarantees consistent format, timestamps, and environment variable control across `pros`, `selfish`, and `orbistoun`.

### Configuring the `OOPS_LOG` Environment Variable

You can control verbosity without rebuilding by setting `OOPS_LOG`:

```powershell
# Set global level to debug
$env:OOPS_LOG = "debug"

# Set global level to info, but enable trace for a specific crate
$env:OOPS_LOG = "info,pros_link=trace,orbistoun_hle=debug"
```

### Log Levels:
- `error`: Fatal errors that cause a command or emulator run to abort.
- `warn`: Unexpected conditions that were recovered from.
- `info`: High-level operational events (e.g. title launched, file copied).
- `debug`: Subsystem decisions and resolved configurations.
- `trace`: Byte-level and per-syscall execution events.

---

## 3. Deterministic Build Stamps (`oops-build`)

`oops-build` provides a zero-dependency `build.rs` helper that injects the current git commit hash, dirty status, and target architecture into your binary:

In `build.rs`:
```rust
fn main() {
    oops_build::stamp();
}
```

In your binary's `main.rs`:
```rust
println!("Tool version: {}", oops_build::version_string!());
// Output: "pros 0.1.0 (commit abc1234, dirty, x86_64-pc-windows-msvc)"
```

---

## 4. Using `oops-libs` in a New Rust Tool

Add the path dependencies to your `Cargo.toml`:

```toml
[dependencies]
oops-paths = { path = "../oops-libs/crates/oops-paths" }
oops-log = { path = "../oops-libs/crates/oops-log" }
oops-build = { path = "../oops-libs/crates/oops-build" }
```

