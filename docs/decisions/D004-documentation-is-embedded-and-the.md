# D004 - Documentation is embedded, and the page list stays in the consumer

**Status:** decided
**Date:** 2026-08-29

`oops-docs` renders pages the consumer embeds with `include_str!`. The viewer is shared; the
list of pages is each consumer's. `check()` validates a list.

**Why:** embedded pages always match the running build and need no copy step or fetch.
`include_str!` resolves relative to the file it is written in, so this crate cannot embed
another crate's files. `include_str!` proves only that a file exists, so `check()` catches
empty pages, repeated slugs and missing headings.

**Rejected:**
- Copying docs into a bundle and fetching by URL: an egui app has no webview to serve them.
