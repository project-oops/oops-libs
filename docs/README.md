# oops-libs documentation

What every project in [OOPS](https://github.com/project-oops/OOPS) needs and none of them
owns: the build stamp, logging setup, where a tool keeps what it writes, and documentation
shipped inside the binary.

Not one of the four. This is infrastructure underneath them - one of two libraries rather than
a fifth project. Its sibling [oops-sdk](https://github.com/project-oops/oops-sdk) is the
freestanding C the target-side payloads link; oops-libs is the Rust the host-side tools share.

New here? The [root README](../README.md) has the crates, what each costs you, and the
rule this library is held to - **nothing goes in here because it might be shared; things go in
here because they were already being written twice.**

## Guide

- **[BUILDING.md](BUILDING.md)** - `bin/oops-libs`, what each verb does, and why every one of
  them passes `--all-features`. That last part is the only surprise here, and it has already
  gone wrong once.

## Project memory

- [DECISIONS.md](DECISIONS.md) - a generated index over `decisions/`, one file per
  entry. Every non-obvious choice, numbered, with the reasoning.
  Starting with why there is a shared library at all, and what the survey of the four
  actually found duplicated.

Shared rules - provenance, naming, decision logs, honest failure, gates - are in
[the OOPS conventions](https://github.com/project-oops/OOPS/blob/main/docs/CONVENTIONS.md) and
not restated here.

## The words

Vocabulary is the collection's, not this repository's. Nothing here defines a term of its own -
these crates are host-side plumbing and touch none of the platform's formats:

- [the collection's glossary](https://github.com/project-oops/OOPS/blob/main/docs/GLOSSARY.md) - standard ELF, `DT_`/`PT_`, and the cross-repository word collisions
- [SELFish](https://github.com/project-oops/SELFish/blob/main/docs/GLOSSARY.md) - NID, fSELF, PFS, packages, the generation split

**host** is defined for all repositories in
[CONVENTIONS.md section 2](https://github.com/project-oops/OOPS/blob/main/docs/CONVENTIONS.md#the-words-for-our-own-layers)
and is the one word this repository turns on: everything here runs on the **host** - the machine
a tool is used from - and never on the target. Its sibling
[oops-sdk](https://github.com/project-oops/oops-sdk) is the target-side half.

## Adding to a log

The long-running documents are **directories with a generated index**. Add a file under
`decisions/`, `backlog/` or `worklog/`, then regenerate the table:

```bash
tools/split-decisions.sh --index oops-libs
tools/split-doc.sh --index oops-libs BACKLOG 2 backlog
```

Do not edit the index by hand - it is overwritten. The split exists because two sessions
appending to one file collide, which is where the duplicate numbers and out-of-order entries
came from, and because a log past half a megabyte stops rendering on GitHub entirely.
