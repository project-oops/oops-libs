# Changelog

oops-libs publishes **no artifact**. It is consumed as a path dependency by the four projects
in the collection, so there is no version and no release: the commit its consumers were built
against is the only version that means anything.

Entries are grouped **Added / Changed / Fixed**, newest first.

Nothing has shipped yet - this is the initial commit.

## [unreleased] - as of 2026-09-01

### Changed

- **`oops_paths::enable_portable_sentinel` takes the note's words as an argument**, so its
  signature is now `(binary_dir, note: Option<&str>)`. The sentinel gets a `PORTABLE.txt` saying
  what the directory does and how to undo it, and those words name a tool, so they cannot live
  in a shared crate. They arrive as a parameter rather than as each caller's own business
  because the alternative is each caller keeping a copy of the whole function - which is the
  duplication the argument exists to end. `None` writes no file at all: an empty note reads as a
  write that failed. A failed note fails the call.

### Fixed

- **A `.portable` file no longer breaks every write of a portable run.** The sentinel and the
  portable root are the same path, and resolution tests it with `exists()`, which a file
  satisfies - so a `.portable` *file* put a run into portable mode rooted at a path no directory
  could be created at, and everything beneath it failed, `enable_portable_sentinel` included.
  The file is now removed before the directory is made. The marker-*file* convention is the
  common one, so this arrives by hand as easily as by history.

### Added

- **Four crates for what every project needs and none of them owns**: `oops-build` (build
  stamps), `oops-docs`, `oops-log`, `oops-paths`. The admission test is narrow on purpose - a
  thing belongs here when writing it twice would mean two chances to write it differently.
- **Documentation published as its own site**, with no landing page. The other four each have
  one because each is a project with something to show; this is infrastructure underneath them,
  and inventing a front page for it would claim it is a fifth project when it is deliberately
  not.
