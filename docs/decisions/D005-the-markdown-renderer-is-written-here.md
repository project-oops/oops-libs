# D005 - The markdown renderer is written here rather than taken


**decided** · 2026-08-29

`egui_commonmark` does this job. It is also pinned to an egui version, so adopting it means
every future egui bump across the collection waits on a matching release of it - for four
projects, to render documents using about eight markdown features between them. It was also not
in the local registry cache, where `pulldown-cmark` already was.

So: `pulldown-cmark` for parsing, which is what most of the ecosystem parses with and which
depends on nothing that moves, and roughly two hundred lines of display code owned here.

Parsing produces a flat block list and egui draws that, in two passes. The first version walked
the event stream and drew as it went, which meant eight mutable locals threaded through a
hundred-line match - and it had a real bug in it that a test caught immediately: in a *tight*
list there is no paragraph event to close, so a parent item's own text was swallowed into its
first nested child and the parent was never emitted. `1. one` with a nested bullet produced two
items instead of three. The state is a struct now, for exactly that reason.

