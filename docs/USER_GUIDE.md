# oops-libs Host Developer Guide

Welcome to the **oops-libs** developer guide.

This guide provides practical instructions for **Rust developers and contributors** building host-side command-line or GUI utilities within the OOPS ecosystem.

For the crate reference, the API surface and where oops-libs sits in the loop, see the **[Technical Reference](../README.md)**.

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

`oops-paths` resolves **two roots**: a *data root* for what a person would want on their next
machine, and a *cache root* for what can be rebuilt. On platforms that distinguish them (Windows
roaming vs. local, `~/.local/share` vs. `~/.cache`) they differ; in a portable run they are the
same directory.

```text
%APPDATA%\OOPS\               <- data root: kept, worth carrying to the next machine
├── orbistoun.toml            <- this tool's config file, named after the tool
└── titles/                   <- per-title data, shared between pros and orbistoun

%LOCALAPPDATA%\OOPS\          <- cache root: rebuildable (same as the data root when portable)
├── cache/                    <- regenerable material; deleting the whole directory is safe
└── logs/                     <- rolling log files
```

`oops-paths` names only these locations. Concepts belonging to one tool - a console/target
registry, for instance - stay in that tool, so there is no `targets/`, `saves/` or `reports/`
directory here.

### Using in Rust:
```rust
use oops_paths::Paths;

// Infallible, and named after the calling tool - there is always an answer.
let paths = Paths::resolve("orbistoun");
paths.ensure_dirs()?;                       // create the directories, failing early and once

let logs_dir = paths.logs_dir();            // <cache-root>/logs
let cache_dir = paths.cache_dir();          // <cache-root>/cache
let config = paths.config_file();           // <data-root>/orbistoun.toml
let title = paths.title_dir("CUSA00001");   // <data-root>/titles/CUSA00001
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

`oops-build` provides a zero-dependency `build.rs` helper that stamps the current git commit - with a `-dirty` suffix when the working tree is modified - into your binary:

In `build.rs`:
```rust
fn main() {
    oops_build::emit();
}
```

In your binary's `main.rs`:
```rust
println!("Tool version: {}", oops_build::line!());
// Output: "v0.1.0 - abc1234"
//   or    "v0.1.0 - abc1234-dirty"                    (built from a modified tree)
//   or    "v0.1.0 - built 2026-08-29 14:03 UTC"       (no commit, e.g. outside a repo)
```

---

## 4. Using `oops-libs` in a New Rust Tool

Add the path dependencies to your `Cargo.toml`:

```toml
[dependencies]
oops-paths = { path = "../../oops-libs/crates/oops-paths" }
oops-log = { path = "../../oops-libs/crates/oops-log" }
oops-build = { path = "../../oops-libs/crates/oops-build" }
```

