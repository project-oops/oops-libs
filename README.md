# oops-libs

Host-side Rust crates shared by every tool in the [OOPS](../README.md) collection: the build
stamp, logging setup, where a tool keeps its files, and documentation shipped inside the
binary. Everything here runs on the host, never on the hardware; the target-side counterpart
is [oops-sdk](../oops-sdk/).

A crate belongs here when every project needs it and none owns it (D001, D008).

| Crate | Purpose | Dependencies |
|---|---|---|
| `oops-build` | commit, version and build time, as one line | none |
| `oops-log` | `tracing` set up once; `OOPS_LOG` levels; file and OTLP behind features | `tracing`, `tracing-subscriber` |
| `oops-paths` | the collection's data and cache roots, portable mode, per-tool config file | `dirs`, behind the default `platform-dirs` feature |
| `oops-docs` | markdown pages embedded in the binary, and an egui window to read them | `egui`, `pulldown-cmark` |

The [guide](docs/USER_GUIDE.md) shows how to use each one.

## Using the crates

Consumers take them by path, from the collection checkout:

```toml
[dependencies]
oops-build = { path = "../../oops-libs/crates/oops-build" }
oops-log   = { path = "../../oops-libs/crates/oops-log" }
oops-paths = { path = "../../oops-libs/crates/oops-paths" }

[build-dependencies]
oops-build = { path = "../../oops-libs/crates/oops-build" }
```

## Building

A Rust toolchain is the only requirement; no sibling checkout is needed.

```bash
./bin/oops-libs check    # rustfmt, clippy at -D warnings, tests
```

Every verb runs over the whole workspace with every feature enabled, so the optional log
destinations and the platform directory lookup are always compiled. `./bin/oops-libs --help`
lists the verbs. CI runs the same `check` through `oops check oops-libs`.
