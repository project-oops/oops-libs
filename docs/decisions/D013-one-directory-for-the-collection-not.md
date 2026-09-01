# D013 - One directory for the collection, not one per tool


**decided** · 2026-08-30 · reversing D-none-of-them, on being asked what the partition was for

The first shape was `OOPS/<tool>/` with an `OOPS/shared/` beside it, on the grounds that the
configs have different shapes and "delete what Orbistoun kept" should be one operation.

**It was the wrong default and the reasoning was backwards.** Partitioning by tool made sharing
the thing that had to be argued for, artefact by artefact - when sharing is the reason these four
exist together. What settled it was measuring rather than arguing: across the three projects that
had written anything to disk, **no filename appeared in more than one of them.** The partition
was not preventing a collision, because there was none to prevent.

What it *was* doing was visible in the listing the moment they sat side by side: obSCEne's
`hardware.txt` and Prosperous's `targets.txt` are both *which console, at what address*, in two
formats, because neither had ever been able to see the other's. And `orbistoun/title-data/` and
`prosperous/titles/` were the same directory under two names.

The one real collision a shared directory has is a bare `config.toml` per tool. Those are named
after the tool - `orbistoun.toml` - which costs a filename and avoids the only argument for the
partition that survived contact with the data.

A side effect worth noting: orbistoun#D251 wanted one title to be one directory, and the
`shared/` split had quietly made it two. It is one again.

