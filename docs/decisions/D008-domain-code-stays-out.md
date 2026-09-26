# D008 - Domain code stays out

**Status:** decided
**Date:** 2026-08-29

oops-libs holds only code no project owns. Formats belong to SELFish, an emulator's ABI to
orbistoun, a registry of hardware to Prosperous.

**Why:** moving domain code into a library named for being shared decides who owns it by
accident.

**Rejected:**
- Moving the ELF, NID and ABI code shared by SELFish and orbistoun here: it is format code, and
  its home is SELFish.
