# Using oops-libs in a tool

## Paths: `oops-paths`

Every tool resolves the same collection directory, `OOPS`, in two roots:

| Root | Windows | macOS | Linux | Holds |
|---|---|---|---|---|
| data | `%APPDATA%\OOPS` | `~/Library/Application Support/OOPS` | `~/.local/share/OOPS` | config, saves, established names |
| cache | `%LOCALAPPDATA%\OOPS` | `~/Library/Caches/OOPS` | `~/.cache/OOPS` | logs, downloads, rebuildable material |

```text
<data root>/
├── orbistoun.toml     one config file per tool, named after it
└── titles/<id>/       per-title data shared between tools
<cache root>/
├── cache/             regenerable; deleting it is always safe
└── logs/
```

```rust
use oops_paths::Paths;

let paths = Paths::resolve("orbistoun");
paths.ensure_dirs()?;
let logs = paths.logs_dir();
let config = paths.config_file();          // <data root>/orbistoun.toml
let title = paths.title_dir("CUSA00001");  // <data root>/titles/CUSA00001
```

`Paths::resolve` always answers. A tool that must not write to the working directory when
the machine offers nowhere else uses
`Paths::resolve_with_options(app, Options::new().refusing())`, which returns `None` instead.

Overrides, highest first:

1. Portable mode: `<APP>_PORTABLE` or `OOPS_PORTABLE` set to `1`/`true`/`yes`/`on`, a
   `.portable` directory beside the executable, or `portable` in the executable's name. Both
   roots become `<binary dir>/.portable`.
2. `<APP>_DATA_DIR` or `OOPS_DATA_DIR`: both roots become that directory.
3. The layout: the platform directories above, or `$HOME/.config/OOPS` with `Layout::Home`.

Only locations every tool needs are named. A file one tool owns, such as the hardware
registry, is that tool's to name.

## Logging: `oops-log`

```rust
let _guard = oops_log::Logging::new("pros")
    .build(oops_build::line!())
    .root(paths.data_root())
    .init();
tracing::info!("registered");
```

Hold the guard for the life of the program. Log with `tracing`'s macros.

Set the level with `OOPS_LOG`, or `RUST_LOG`, in `EnvFilter` syntax:

```powershell
$env:OOPS_LOG = "debug"
$env:OOPS_LOG = "info,pros_link=trace,orbistoun_hle=debug"
```

What belongs at each level is in
[CONVENTIONS section 6](https://github.com/project-oops/OOPS/blob/main/docs/CONVENTIONS.md#6-logging).
The `file` feature adds `Logging::to_file(paths.logs_dir())`; the `otlp` feature adds
`Logging::to_otlp(endpoint)`.

## Build stamp: `oops-build`

`build.rs`:

```rust
fn main() {
    oops_build::emit();
}
```

The binary:

```rust
println!("{}", oops_build::line!());
// v0.1.0 - abc1234
// v0.1.0 - abc1234-dirty                  built from a modified tree
// v0.1.0 - built 2026-08-29 14:03 UTC     no commit, e.g. outside a repository
```

`oops_build::line!()` is a `&'static str` and fits `#[command(version = ...)]`.
`stamp!().is_exact()` says whether the build names a commit someone else can check out.

## Embedded documentation: `oops-docs`

```rust
const DOCS: &[oops_docs::Doc] = &[oops_docs::Doc::new(
    "running", "Running a title", "Loading, and what happens after",
    include_str!("../../docs/features/running.md"),
)];

#[test]
fn the_registry_is_sound() {
    assert_eq!(oops_docs::check(DOCS), Vec::<String>::new());
}
```

Keep one `DocsWindow`, call `open()` from a menu and `show(ctx, DOCS)` every frame. Register
pages written for users.
