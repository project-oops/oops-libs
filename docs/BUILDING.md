# Building oops-libs

There is one command and it is `bin/oops-libs`.

```bash
./bin/oops-libs check
```

## What you need

**A Rust toolchain, and nothing else.** oops-libs is the bottom of the collection - it depends
on no sibling checkout, and a clone of only this repository builds.

## Why every verb passes `--all-features`

This is the one thing about building here that is not obvious, and it has already gone wrong
once.

`oops-log`'s file and OTLP paths and `oops-paths`'s platform layout are **all behind cargo
features**, and a default build compiles none of them. A feature nobody builds is a feature
that has already stopped compiling. So every verb that touches the code passes
`--workspace --all-features`, and the one time that was left off, the OTLP path went unbuilt
for a day.

There is no per-crate mode, for the same reason. These crates exist to be taken together, and
a check that passed them one at a time would say nothing about the combination anybody
actually depends on.

## The verbs

The same seven every OOPS repository carries, so `oops test oops-libs` and
`./bin/oops-libs test` are one command reached two ways.

| verb | what it does |
|---|---|
| `build` | `cargo build --workspace --all-features --release` |
| `test` | `cargo test --workspace --all-features` |
| `lint` | clippy, all targets and all features, at `-D warnings` |
| `fmt` | `cargo fmt --all` |
| `check` | `fmt --check`, then `lint`, then `test` - in that order |
| `clean` | `cargo clean` |
| `doc` | `cargo doc --no-deps`, all features |

`check` runs its steps in the order CI would, so a local failure is the failure CI would have
reported rather than a different one found earlier.

## The four crates

| crate | what it is | what it costs you |
|---|---|---|
| `oops-build` | which build this is: commit, version, build time, one line | nothing - no dependencies at all |
| `oops-log` | turning logging on, the same way everywhere | `tracing` and `tracing-subscriber`; files and OTLP behind features |
| `oops-paths` | where a tool keeps what it writes, portable mode included | nothing by default; the platform layout behind a feature |
| `oops-docs` | documentation shipped inside the binary, and an egui window for it | `egui`, `pulldown-cmark` |

Versions are pinned in the workspace `Cargo.toml` rather than in each crate, so the four
cannot drift apart and a consumer reading one manifest learns what the whole library costs.

## What CI runs

`.github/workflows/check.yml`, one job, one step: `oops check oops-libs` - a thin wrapper
over `./bin/oops-libs check`, the same command reached the same way a person reaches it.

**There was no workflow here at all until recently.** All four projects take these crates by
path, so a change here reached every one of them and was checked by none of them until their
own pipelines failed afterwards - which, since none of those pipelines has ever executed
either, meant not at all.

The job still runs `oops bootstrap oops-libs` even though this repository needs no siblings.
One shape, so that reading any workflow in the collection teaches you all of them, and so the
day this grows a dependency is not also the day somebody discovers the preamble was special
here.

## From the collection

[OOPS](https://github.com/project-oops/OOPS) holds it beside the four that use it:

```bash
./bin/oops check oops-libs
./bin/oops all                  # the meta gates, then every project's own gate
```

`oops-libs` is included in the collection's sweeps precisely because every one of the four
depends on it, so a change that breaks a consumer should be caught by the same command that
checks the consumers.
[The collection's BUILDING.md](https://github.com/project-oops/OOPS/blob/main/docs/BUILDING.md)
covers `bootstrap`, `gates`, `all` and the rest.
