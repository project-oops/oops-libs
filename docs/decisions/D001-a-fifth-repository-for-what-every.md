# D001 - A separate repository for what every project needs and none owns

**Status:** decided
**Date:** 2026-08-29

Host-side Rust code that every OOPS tool needs, and that belongs to no one project, lives in
its own repository, oops-libs. It is a library, not a project.

**Why:** a shared crate needs a home that depends on nothing. orbistoun is a leaf with a large
dependency tree (Vulkan, HTTP), and making it a dependency of SELFish would invert the
direction. SELFish's charter is file formats and nothing that knows what a consumer is for.

**Rejected:**
- A crate inside orbistoun: turns the repository nothing depends on into one everything does.
- A crate inside SELFish: outside its charter.
