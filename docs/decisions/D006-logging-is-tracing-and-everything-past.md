# D006 - Logging is `tracing`, and everything past stderr is a feature


**decided** · 2026-08-29 · from a survey that found nothing to extract

Logging looked like the obvious second candidate for this repository and turned out to be the
counter-case. Of the four projects, one uses `tracing`; two have modules with `log` in the name
that are a *console system-log reader* and a *guest call tracer* - domain code sharing a word;
and one has no logging at all. There was nothing duplicated to merge.

So this crate is new work, and being new it is thin on purpose. It is `tracing` underneath -
the facade the ecosystem already agrees on - and it does not wrap the macros. A wrapper would
break `#[instrument]`, structured fields, and every editor that knows what `tracing` is.

**Destinations are cargo features.** Every tool in the collection is meant to take this crate,
including a format library's CLI, and that is only reasonable if the floor is low: stderr and a
level by default, `file` for a rolling file, `otlp` for the wire format an LGTM stack ingests. A
tool that wants `--verbose` should not compile a tokio runtime to get it.

Stderr rather than stdout, always: every one of these tools has an output somebody redirects,
and a logger on stdout corrupts it.

The guard returned by `init` is load-bearing and is the one sharp edge. Dropping it early stops
the file writer mid-batch, and the symptom is a log missing its last few seconds - the part
somebody was reading it for.

