# D003 - The reading half of the stamp is macros, not functions


**decided** · 2026-08-29 · from the first compile

`env!("CARGO_PKG_VERSION")` inside `oops-build` reports *`oops-build`'s* version. `option_env!`
there reads the environment of *its* compilation, not the consumer's. Both have to expand at the
call site.

Worth recording because the failure is silent: a plain `pub fn version()` compiles, runs, and
returns a confidently wrong answer. The macro is not stylistic.

The same constraint appears in `oops-docs` (D005) from the same root cause - a shared crate
cannot see its consumer's compile-time environment - and it is the thing most likely to be
"simplified" back into a function by somebody who has not hit it.

