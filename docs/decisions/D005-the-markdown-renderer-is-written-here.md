# D005 - The markdown renderer is written here

**Status:** decided
**Date:** 2026-08-29

`oops-docs` parses with `pulldown-cmark` and draws with its own display code, in two passes:
events to a flat block list, then the list to egui.

**Why:** `pulldown-cmark` depends on nothing that moves. The display half is small, and a flat
list is testable without a UI.

**Rejected:**
- `egui_commonmark`: pinned to an egui version, so every egui upgrade in the collection would
  wait for a matching release.
