# D009 - The nowhere-to-write case is a parameter

**Status:** decided
**Date:** 2026-08-29

When a machine offers no home directory and no executable location, the caller chooses the
outcome with `Nowhere`: fall back to the working directory, or `Options::refusing()` and get
`None`. `Paths::resolve_found` returns the answer and whether it is proper, for callers that
always want a root.

**Why:** the library handles the edge case once instead of each caller inspecting a flag and
handling it differently. Every input is on `Process`, so the edge case is testable.

**Rejected:**
- An infallible resolve plus an `is_fallback()` query: hands the hard case back to every caller.
