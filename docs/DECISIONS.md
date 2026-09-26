# Decisions

The decisions in force, one file each under `decisions/`. Format and rules are in
[STYLE](https://github.com/project-oops/OOPS/blob/main/docs/STYLE.md#decisions).

**This table is generated.** Edit an entry under `decisions/`, then run
`tools/split-decisions.sh --index oops-libs`. A number resolves to exactly one file.

| | # | decision | status | date |
|---|---|---|---|---|
| 🟢 | D001 | [A separate repository for what every project needs and none owns](decisions/D001-a-fifth-repository-for-what-every.md) | decided | 2026-08-29 |
| 🟢 | D002 | [One shared build stamp](decisions/D002-the-build-stamp-is-one-implementation.md) | decided | 2026-08-29 |
| 🟢 | D004 | [Documentation is embedded, and the page list stays in the consumer](decisions/D004-documentation-is-embedded-and-the.md) | decided | 2026-08-29 |
| 🟢 | D005 | [The markdown renderer is written here](decisions/D005-the-markdown-renderer-is-written-here.md) | decided | 2026-08-29 |
| 🟢 | D006 | [Logging is `tracing`, and every destination past stderr is a feature](decisions/D006-logging-is-tracing-and-everything-past.md) | decided | 2026-08-29 |
| 🟢 | D007 | [The platform's own directory is the default layout](decisions/D007-the-platform-directory-is-the-default.md) | decided | 2026-09-26 |
| 🟢 | D008 | [Domain code stays out](decisions/D008-domain-code-stays-out.md) | decided | 2026-08-29 |
| 🟢 | D009 | [The nowhere-to-write case is a parameter](decisions/D009-the-nowhere-to-write-case-is-a.md) | decided | 2026-08-29 |
| 🟢 | D010 | [Logging levels are defined once, in the shared conventions](decisions/D010-logging-levels-are-defined-once-in-the.md) | decided | 2026-08-29 |
| 🟢 | D013 | [One directory for the collection, not one per tool](decisions/D013-one-directory-for-the-collection-not.md) | decided | 2026-08-30 |
| 🟢 | D014 | [A data root and a cache root](decisions/D014-two-roots-because-a-roaming-profile-is.md) | decided | 2026-08-30 |
| 🟢 | D015 | [A shared directory is not a shared file](decisions/D015-the-console-registry-belongs-to.md) | decided | 2026-09-09 |

| | meaning |
|---|---|
| 🟢 | settled, and the reasoning rests on something checkable |
| 🟡 | assumed or proposed - made without input, and in the review queue |
| 🔴 | reversed, superseded or blocked |
| ⚪ | no status recorded |

A date with `~` is **not recorded** - it is worked out from the dated entries either
side, because an entry between two of them was written between their dates. `~` alone
is a day both neighbours agree on; `~a..b` is a span, and no day inside it is claimed;
`~>a` and `~<a` are entries with a dated neighbour on only one side. A bare `-` has no
dated entry either side to reason from.
