# oops-libs documentation

The host-side Rust crates every [OOPS](https://github.com/project-oops/OOPS) tool shares:
build stamp, logging, paths and embedded documentation.

- [The repository README](../README.md) - the crates, what each costs, and how to build.
- [USER_GUIDE.md](USER_GUIDE.md) - using each crate in a tool.
- [DECISIONS.md](DECISIONS.md) - the decisions in force, generated from `decisions/` by
  `tools/split-decisions.sh --index oops-libs` in the collection. Never edit it by hand.

Shared rules are in the collection's
[CONVENTIONS](https://github.com/project-oops/OOPS/blob/main/docs/CONVENTIONS.md) and
[STYLE](https://github.com/project-oops/OOPS/blob/main/docs/STYLE.md). Vocabulary is the
collection's [glossary](https://github.com/project-oops/OOPS/blob/main/docs/GLOSSARY.md);
**host**, the one word this repository turns on, is defined in
[CONVENTIONS section 2](https://github.com/project-oops/OOPS/blob/main/docs/CONVENTIONS.md#the-words-for-our-own-layers).
