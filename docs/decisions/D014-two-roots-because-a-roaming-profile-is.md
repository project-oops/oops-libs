# D014 - A data root and a cache root

**Status:** decided
**Date:** 2026-08-30

`Paths::data_root` holds what cannot be regenerated without the hardware or a person:
configuration, saves, established names, hardware measurements, overrides.
`Paths::cache_root` holds what can: models, runtimes, compiled shaders, packages, traces and
logs. In a portable run both are the same directory.

**Why:** on Windows the data root roams and is synchronised at logon; gigabytes of
downloadable material do not belong there. Linux and macOS make the same split.

**Rejected:**
- A single root: puts rebuildable bulk in the roaming profile.
