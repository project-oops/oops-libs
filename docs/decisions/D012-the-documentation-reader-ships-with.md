# D012 - The documentation reader ships with pages, or it is not finished


**decided** · 2026-08-30

`oops-docs` was written before anything it could display existed - every `docs/` file in the four
projects is development record: decision logs, worklogs, backlogs, protocol notes. Useful, and
not what somebody clicking *documentation* is asking for.

So the crate sat with zero consumers, and would have gone on looking finished. Seven pages now
exist under `docs/features/` in the two projects with windows, and both readers are wired to
them.

Worth stating because the temptation was to wire the reader to what was already there. It would
have worked: orbistoun's decision log is markdown, it renders, and it would have put most of a
megabyte of internal reasoning into every binary and called it a manual.

