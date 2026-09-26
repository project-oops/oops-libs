# D006 - Logging is `tracing`, and every destination past stderr is a feature

**Status:** decided
**Date:** 2026-08-29

`oops-log` configures `tracing` and does not wrap its macros. The default build logs to stderr
at a level; the `file` and `otlp` features add a rolling file and OTLP export.

**Why:** `tracing` is the facade the ecosystem uses, and wrapping it would break
`#[instrument]` and structured fields. Features keep the cost of a small CLI at two crates.
Stderr, because tools pipe stdout.

**Rejected:**
- Wrapping the macros: loses `tracing`'s own features.
- Always compiling every destination: a format tool would build a tokio runtime to get a level.
