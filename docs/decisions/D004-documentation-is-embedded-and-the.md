# D004 - Documentation is embedded, and the registry stays in the consumer


**decided** · 2026-08-29 · after reading how two sibling projects do it

Two other projects of the author's ship in-app documentation, both web-shaped: one copies
`docs/features/` into the bundle from CI, the other from `build.rs`, and the page fetches by
relative URL. Neither mechanism applies to an egui app, which has no webview and nothing to
serve assets over.

`include_str!` is the whole mechanism here. No build script, no copy step, no runtime fetch -
which makes the egui version the *simplest* of the three rather than a port of the complexity.
The pages ride in the executable, so they cannot disagree with the build somebody is running.

**The registry cannot be shared**, for D003's reason: `include_str!` resolves relative to the
file it is written in, so this crate cannot embed another crate's documents. That is a hard
constraint, and it draws the line in the right place anyway - the viewer is shared, the list of
pages is not.

`check()` exists because `include_str!` proves a file *exists* and nothing more. A page
truncated to nothing, two entries claiming one slug, a page with no heading: all three compile,
and all three look like documentation that has not been written yet.

