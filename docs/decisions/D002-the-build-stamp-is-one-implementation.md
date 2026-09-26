# D002 - One shared build stamp

**Status:** decided
**Date:** 2026-08-29

Every binary stamps its build through `oops-build`: the commit from git (with `-dirty` for a
modified tree, or from `OOPS_COMMIT` when CI supplies it), the version, and the executable's
own modification time. `Stamp::is_exact` is the check for a reproducible build.

**Why:** per-project copies each covered half the job and none noticed its gap, because a
stamp reading `no commit` looks like a local build rather than a defect.

**Rejected:**
- A commit read only from an environment variable: correct only when something sets it.
- A compile-time build timestamp: records when one crate compiled, not when the binary linked.
