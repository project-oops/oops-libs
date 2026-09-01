# D009 - The nowhere-to-write case is a parameter, not a fact the caller inspects


**decided** · 2026-08-29 · reversing the first attempt, on review

The first version of `oops-paths` resolved infallibly and exposed `is_fallback()`, so a caller
that would rather refuse than write to the working directory could ask and then refuse itself.
It worked, and it was the wrong shape: the library covered the easy case and handed the edge
case back.

Now the caller says what it wants and the library does it - `Options::new().refusing()`, and
resolution returns `None`. The reason is at the call site rather than three lines below it:

```rust
Paths::resolve_with_options("prosperous", Options::new().refusing())
```

**Two structural things fell out of the reversal**, and both are why it was worth doing rather
than patching:

- `resolve` had been an `expect` over the fallible form - an unreachable branch documented as a
  panic. Splitting *compute the answer* from *is this answer acceptable* removed it entirely.
  `found` cannot fail; the policy filters afterwards.
- The three inputs to resolution became a `Process`, which now also carries the home directory.
  It had been read where it was used, so **the entire `Nowhere` policy could not be tested** -
  the machine running the test has a home, so the branch never ran. A rule only exercisable on a
  machine that happens to lack a home directory is a rule nobody ever checks.

Neither was visible while the design was "expose the fact and let the caller decide".

