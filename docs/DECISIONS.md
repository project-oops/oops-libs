# Decisions

Numbered, with reasoning, as they are made. The reasoning is the point - it is what stops a
choice being re-litigated by somebody who only has the choice.

**This table is generated.** Edit an entry under `decisions/`, then run
`tools/split-decisions.sh --index oops-libs`. A number resolves to exactly one file.

| | # | decision | status | date |
|---|---|---|---|---|
| 🟢 | D001 | [A fifth repository, for what every project needs and none of them owns](decisions/D001-a-fifth-repository-for-what-every.md) | decided | 2026-08-29 |
| 🟢 | D002 | [The build stamp is one implementation, because two had become complementary](decisions/D002-the-build-stamp-is-one-implementation.md) | decided | 2026-08-29 |
| 🟢 | D003 | [The reading half of the stamp is macros, not functions](decisions/D003-the-reading-half-of-the-stamp-is-macros.md) | decided | 2026-08-29 |
| 🟢 | D004 | [Documentation is embedded, and the registry stays in the consumer](decisions/D004-documentation-is-embedded-and-the.md) | decided | 2026-08-29 |
| 🟢 | D005 | [The markdown renderer is written here rather than taken](decisions/D005-the-markdown-renderer-is-written-here.md) | decided | 2026-08-29 |
| 🟢 | D006 | [Logging is `tracing`, and everything past stderr is a feature](decisions/D006-logging-is-tracing-and-everything-past.md) | decided | 2026-08-29 |
| 🟢 | D007 | [The home directory is the default layout, not the platform's](decisions/D007-the-home-directory-is-the-default.md) | decided | 2026-08-29 |
| 🟢 | D008 | [Domain code stays out](decisions/D008-domain-code-stays-out.md) | decided | 2026-08-29 |
| 🟢 | D009 | [The nowhere-to-write case is a parameter, not a fact the caller inspects](decisions/D009-the-nowhere-to-write-case-is-a.md) | decided | 2026-08-29 |
| 🟢 | D010 | [Logging levels are defined once, in the shared conventions](decisions/D010-logging-levels-are-defined-once-in-the.md) | decided | 2026-08-29 |
| 🟢 | D011 | [What five binaries were about to write twice, the library writes once](decisions/D011-what-five-binaries-were-about-to-write.md) | decided | 2026-08-30 |
| 🟢 | D012 | [The documentation reader ships with pages, or it is not finished](decisions/D012-the-documentation-reader-ships-with.md) | decided | 2026-08-30 |
| 🟢 | D013 | [One directory for the collection, not one per tool](decisions/D013-one-directory-for-the-collection-not.md) | decided | 2026-08-30 |
| 🟢 | D014 | [Two roots, because a roaming profile is not a place for four gigabytes](decisions/D014-two-roots-because-a-roaming-profile-is.md) | decided | 2026-08-30 |

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
