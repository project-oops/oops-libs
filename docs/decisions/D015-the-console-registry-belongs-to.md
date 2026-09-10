# D015 - The console registry belongs to Prosperous, and oops-paths gains nothing


**decided** · 2026-09-09

[`Paths::data_root`] is the collection's directory rather than one tool's, which is D013 working
as intended - and it means two tools can write into it without either of them being wrong. Two
did. Prosperous keeps the registered console at `data_root()/targets.txt`; obSCEne kept the same
fact in `hardware.txt` beside it, and obSCEne's own manifest called that "two files recording one
fact, which is the next thing to fix rather than a design".

The shape that suggests itself is an accessor here - something beside [`Paths::config_file`] that
names the shared file, so that neither tool composes the path out of a string it picked alone.

**It is not coming.** Asked which of the two shapes it wanted, Prosperous answered that the
registry has one owner. It is Prosperous's; `pros register` and `pros forget` are the only
writers; a tool that wants to know which console is registered calls `pros_core::target::load()`
rather than keeping a second copy. There is no jointly-owned file here for this crate to name.

That is the answer this library should have hoped for, because the accessor would have been D008
breaking in the quietest way available. A function called `console_registry()` teaches this crate
what a console is, what it means for one to be registered, and who is entitled to say so - three
pieces of domain, arriving disguised as a path. `data_root()` already exists, and the filename
and its format are Prosperous's published contract. Naming that file here would have moved a
decision out of the project that owns it and into the library whose whole claim is that it owns
nothing.

The rule to carry forward, because this will come round again: **a shared directory is not a
shared file.** This crate says where the collection writes. What is written there, and who may
write it, belongs to whoever owns the fact.

Asked and answered as `REQ-20260909T1253Z-0708` on the cross-project request mesh.
