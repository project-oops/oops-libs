# D015 - A shared directory is not a shared file

**Status:** decided
**Date:** 2026-09-09

oops-paths names where the collection writes, not what is written there. The registry of
hardware is Prosperous's: `pros register` and `pros forget` write it, and other tools read it
through `pros_core::target::load()`. oops-paths has no accessor for it.

**Why:** an accessor such as `console_registry()` would teach this crate what the hardware is
and who may register it, which is domain (D008).

**Rejected:**
- A registry accessor in oops-paths beside `config_file`.
