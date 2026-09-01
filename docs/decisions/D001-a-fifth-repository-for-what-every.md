# D001 - A fifth repository, for what every project needs and none of them owns


**decided** · 2026-08-29 · from a survey of the four before writing anything

Four projects, and a survey of what was actually duplicated across them rather than what might
be. Exactly one thing was: the build stamp. Everything else here is either new work that would
otherwise be written four times (`oops-log`, `oops-docs`) or one good implementation that a
second project was open-coding badly (`oops-paths`).

**Why a repository and not a crate inside one of the four.** The obvious home was Orbistoun,
because it is the largest and most mature. It is also a *leaf*: nothing depends on it, and it
deliberately carries its own `abi`, `elf` and `nid` rather than using SELFish's. Putting shared
code there would mean SELFish - seventy-nine resolved packages, `unsafe_code = "forbid"`, built
to stay out of a loader's way - taking a dependency edge on a workspace of five hundred and
forty-one that pulls Vulkan and an HTTP client. Cargo resolves per-crate so the *cost* would
have been small, but the *direction* would have been wrong: the repository nothing depends on
becoming the one everything depends on.

SELFish was the other candidate, being the established shared home. It is the wrong one for a
different reason: its charter is formats and *nothing that knows what a consumer is for*, and a
documentation viewer is precisely something that knows what a consumer is for.

So: a fifth repository, which is not a fifth project. OOPS remains four.

