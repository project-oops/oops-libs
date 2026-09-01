# D011 - What five binaries were about to write twice, the library writes once


**decided** · 2026-08-30 · while wiring the fourth consumer

Three shapes turned up in prosperous, then again the moment obSCEne, SELFish and orbistoun were
wired. Each was small enough to retype and none of them should have been:

- **`line!()`** - `clap` builds `--version` from a `&'static str`, and the line cannot be a
  `const`, because part of it is when the *executable* was written. So every command-line tool
  needs the same `OnceLock` to hold it. Three lines, five times, each an opportunity to hold it
  differently.
- **The startup line** - which build, and where it writes: the two facts every bug report needs
  and nobody remembers to ask for. It was hand-written in two binaries before it became
  `Logging::build` and `Logging::root`. Centralising it also fixes an ordering trap each tool
  would otherwise have had to get right separately: it has to be emitted *after* the subscriber
  exists, or it goes nowhere.
- **`Paths::resolve_found`** - the infallible half of resolution. orbistoun layers a dozen
  directories of its own on top of the root and always wants one, so going through the
  `Nowhere`-honouring form would have meant an `expect` at that call site for a branch that
  cannot happen. Exposing what was already computed removes it.

The pattern in all three: **a shared crate that covers the easy case and hands the awkward one
back gets the awkward one solved five different ways.** `is_fallback` (D009) was the same
mistake, caught earlier.

