# D007 - The home directory is the default layout, not the platform's


**decided** · 2026-08-29 · from reasoning already written down in Prosperous

The two projects with a path story disagreed, and it is a real disagreement rather than an
oversight:

- Orbistoun resolves through `ProjectDirs`, so the non-portable root is the platform's own -
  `%APPDATA%` on Windows.
- Prosperous uses `$HOME/.config/<app>` on every platform, **deliberately**, with the reasoning
  in its own module: a tool running inside a packaged container has its writes to the per-user
  application data directory redirected into a per-package cache, invisible to the same user
  running the same tool from an ordinary shell. A configuration file the user cannot find is
  worse than no configuration file.

Prosperous is right, and it is right from experience rather than preference. `Layout::Home` is
the default. `Layout::PlatformNative` stays available behind a feature, because it is the
correct answer for a tool that is genuinely installed rather than run from wherever it landed.

Deliberately **not** resolved by picking a winner and migrating: this crate offers both and each
application chooses. Nothing has shipped, so there is no data to move, but the choice is a
property of how a tool is distributed and this crate does not know that.

