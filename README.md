# oops-libs

What every project in [OOPS](https://github.com/project-oops/OOPS) needs and none of them owns.

Site: **[project-oops.github.io/oops-libs](https://project-oops.github.io/oops-libs/)** -
the documentation, rendered. No landing page: this is not one of the four.

| crate | what it is | dependencies |
|---|---|---|
| **oops-build** | Which build this is: commit, version, build time, one line | none |
| **oops-log** | Turning logging on, the same way everywhere | `tracing`, `tracing-subscriber` |
| **oops-paths** | Where a tool keeps what it writes, portable mode included | none by default |
| **oops-docs** | Documentation shipped inside the binary, and an egui window for it | `egui`, `pulldown-cmark` |

## The rule this library is held to

**Nothing goes in here because it might be shared. Things go in here because they were already
being written twice.**

That is not a slogan - it is what the first crate is for. `oops-build` existed as two
implementations in two projects, and they had drifted into being *complementary*: each solved
half the problem and carried the half the other had already fixed. One asked git for the commit
and handled a modified tree, but emitted no build time. The other assembled a readable line with
a timestamp, but read its commit from a variable **nothing ever set** - so every binary it
produced was stamped `no commit`, and nothing noticed, because that looks like a local build
rather than a defect.

Two copies of a thing do not stay two copies of the same thing.

The counter-case is just as important. Logging looked like an obvious candidate and is not:
the two projects that appear to have logging modules have a *console system-log reader* and a
*guest call tracer*, which are domain code that happen to share a word. There was nothing to
extract, so `oops-log` is new work rather than a merge - and it is thin on purpose.

## What is deliberately not here

**The nid, elf and abi overlap between SELFish and Orbistoun.** It is the largest duplication in
the collection, and it is domain code: its home is SELFish if anywhere. Whether to resolve it is
an open question with arguments on both sides, recorded in
[OOPS architecture](https://github.com/project-oops/OOPS/blob/main/docs/ARCHITECTURE.md). Moving it
here would quietly answer a question that was deliberately left open.

## Paying only for what you use

Every tool in the collection is expected to take `oops-build` and `oops-log`. That is only
reasonable if the floor is low, so:

- `oops-build` has **no dependencies at all**, and should keep none. It is compiled twice for
  every consumer - once as a build-dependency, once as a normal one.
- `oops-log` is `tracing` and a subscriber. Files (`file`) and OTLP (`otlp`) are cargo features,
  off by default. A format library's CLI should not compile an OTLP exporter and a tokio runtime
  to get a `--verbose` flag.
- `oops-paths` needs nothing but `std`; the platform-native layout is behind `platform-dirs`.
- `oops-docs` is the only crate that pulls egui, which is why it is a separate crate rather than
  a module of a single `oops-support`.

## Using it

While the collection is unpublished, path dependencies from the sibling layout:

```toml
[dependencies]
oops-build = { path = "../../oops-libs/crates/oops-build" }
oops-log   = { path = "../../oops-libs/crates/oops-log" }

[build-dependencies]
oops-build = { path = "../../oops-libs/crates/oops-build" }
```

`oops-build` appears twice because half of it runs at build time. See its crate documentation.

Once these are published, the same line gains a `version` and cargo takes the registry copy for
anybody who cloned one project alone - see
[OOPS publishing](https://github.com/project-oops/OOPS/blob/main/docs/PUBLISHING.md).

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option -
the Rust ecosystem convention.

## Where it sits

Not one of the four. **OOPS** is Orbistoun, obSCEne, Prosperous and SELFish - four projects
aimed at one console's operating system. This is infrastructure underneath them, and it is a
fifth repository rather than a fifth project.

Shared rules - provenance, naming, decision logs, honest failure, gates - live in
[the OOPS conventions](https://github.com/project-oops/OOPS/blob/main/docs/CONVENTIONS.md) and are
not restated here. What this repository adds is above.

## Building

The same entry point every OOPS repository carries, so `oops test oops-libs` and
`./bin/oops-libs test` are one command reached two ways:

```bash
./bin/oops-libs check     # fmt --check, then clippy at -D warnings, then the tests
```

**`--all-features` is not optional here**, and every verb passes it. The `file` and `otlp`
paths in `oops-log` and the platform layout in `oops-paths` are compiled by nothing else, and a
feature nobody builds is a feature that has already stopped compiling. The one time that was
left off, the OTLP path went unbuilt for a day.

**[docs/BUILDING.md](docs/BUILDING.md)** has every verb, and what CI runs.
