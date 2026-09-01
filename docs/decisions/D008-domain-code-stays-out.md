# D008 - Domain code stays out


**decided** · 2026-08-29

SELFish and Orbistoun hold five thousand lines each of `nid`, `elf` and `abi`, and that overlap
is the largest duplication in the collection - far larger than anything in this repository.

It is not coming here. It is domain code: its home is SELFish if anywhere, and whether to
resolve it at all is an open question with arguments on both sides, recorded in OOPS's
architecture notes. Moving it into a library named for being shared would answer that question
by accident.

The line this repository holds: **what every project needs and none of them owns.** A format
belongs to SELFish. An emulator's ABI belongs to Orbistoun. A build stamp belongs to nobody.

