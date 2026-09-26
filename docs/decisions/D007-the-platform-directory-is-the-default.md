# D007 - The platform's own directory is the default layout

**Status:** decided
**Date:** 2026-09-26

`Layout::PlatformNative` is the default: `%APPDATA%`, `~/Library/Application Support` or
`~/.local/share`, plus `OOPS`. `Layout::Home` (`$HOME/.config/OOPS`) is available to a tool
that runs inside an `MSIX`/`AppX` container.

**Why:** the platform directory is where people, backup tools and roaming profiles look. The
redirection that hides it applies only inside a packaged container, and these tools are plain
executables.

**Rejected:**
- `Home` as the default: a Unix convention nothing on Windows knows, chosen for a container
  case that does not apply.
