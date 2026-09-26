# D013 - One directory for the collection, not one per tool

**Status:** decided
**Date:** 2026-08-30

Every tool resolves the same `OOPS` data root. Titles, reports and the address book are
shared; only each tool's config file is named after it (`orbistoun.toml`).

**Why:** the tools describe the same titles and hardware. Partitioning by tool produced two
files recording one fact and two directories holding one title's data, while no filename
collided across tools.

**Rejected:**
- `OOPS/<tool>/` with an `OOPS/shared/` beside it: makes sharing the exception.
