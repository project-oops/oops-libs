# Changelog

oops-libs publishes **no artifact**. It is consumed as a path dependency by the four projects
in the collection, so there is no version and no release: the commit its consumers were built
against is the only version that means anything.

Entries are grouped **Added / Changed / Fixed**, newest first.

Nothing has shipped yet - this is the initial commit.

## [unreleased] - as of 2026-09-01

### Added

- **Four crates for what every project needs and none of them owns**: `oops-build` (build
  stamps), `oops-docs`, `oops-log`, `oops-paths`. The admission test is narrow on purpose - a
  thing belongs here when writing it twice would mean two chances to write it differently.
- **Documentation published as its own site**, with no landing page. The other four each have
  one because each is a project with something to show; this is infrastructure underneath them,
  and inventing a front page for it would claim it is a fifth project when it is deliberately
  not.
