# D002 - The build stamp is one implementation, because two had become complementary


**decided** · 2026-08-29 · measured by diffing the two build scripts

Two projects had a build stamp. They were written to match each other - one says so in its own
module comment - and they had still drifted into solving different halves:

- Orbistoun asks git directly, handles a modified tree with `-dirty`, shortens hashes, and
  watches `.git/HEAD`. It emits no build time and no assembled line.
- Prosperous assembles a readable line with a UTC timestamp. Its commit comes from
  `PROSPEROUS_COMMIT`, which **nothing sets** - not CI, not its own shell script, nowhere in the
  repository outside the build script that reads it. Every binary it has ever produced is
  stamped `no commit`. Its timestamp is also a constant baked into one crate, which records when
  *that crate* was last compiled rather than when the binary was linked.

Neither defect was noticed because neither looks like one. `no commit` reads as a local build.

This is the argument for the whole repository, and it is why the stamp was extracted first
rather than the documentation viewer, which was the request that started the work. A shared
crate justified by "we might need this twice" is speculation. This one was already wrong twice.

[`Stamp::is_exact`] exists because of it: report code that asks `commit.is_some()` gets `true`
from a dirty tree, and the thing that hid the original defect was exactly a stamp that looked
populated.

