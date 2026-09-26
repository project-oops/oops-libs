# D010 - Logging levels are defined once, in the shared conventions

**Status:** decided
**Date:** 2026-08-29

What belongs at each level is defined in
[OOPS conventions §6](https://github.com/project-oops/OOPS/blob/main/docs/CONVENTIONS.md#6-logging),
not in `oops-log` or any project.

**Why:** `oops-log` sets logging up; if each project defined levels, `warn` would mean
different things in different tools.

**Rejected:**
- Level guidance in this crate's docs: consumers read the conventions, not this crate.
