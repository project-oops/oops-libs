# D010 - Logging levels are defined once, in the shared conventions


**decided** · 2026-08-29

`oops-log` sets logging up; it does not say what belongs at which level, and four projects each
deciding that separately is how `warn` comes to mean "I did something" in one of them and "you
should worry" in another.

The table lives in [OOPS conventions §9](https://github.com/project-oops/OOPS/blob/main/docs/CONVENTIONS.md#9-logging).
The two rules that do the work:

**A library logs facts; a binary logs outcomes.** A function returning `Err` has decided
nothing - the caller may expect that failure. So a digest mismatch is `warn` where it is
detected and `error` where a command gives up on it. Logging `error` from the library reports
one problem twice, at the wrong severity, from the layer that knows least about why it was
asked.

**Logging is not printing.** Prosperous holds an explicit rule that a library must not write to
the terminal, because that decides the interface of every tool using it. `tracing` is a facade
the binary points wherever it likes, including nowhere - which is what makes it admissible in a
crate holding that rule.

