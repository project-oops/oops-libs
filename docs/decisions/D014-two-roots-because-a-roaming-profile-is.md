# D014 - Two roots, because a roaming profile is not a place for four gigabytes


**decided** · 2026-08-30

Windows distinguishes roaming application data from local, and it is not a formality - a domain
profile synchronises the roaming one at logon. The collection had put **4.1 GB of downloaded
model weights, a 95 MB runtime and 439 MB of packages** there.

So [`Paths::data_root`] is what somebody would want on their next machine and
[`Paths::cache_root`] is what can be rebuilt. The test for which side something belongs on:
**can you get it back without the console?**

- *Cache*: models and runtimes download, shaders compile, the base filesystem is materialised
  from a manifest it ships with, packages and payloads have a URL and a digest in the manifest,
  and a trace is one re-run away. Logs too - a log is one machine's account of one run.
- *Data*: a report measured against real hardware cannot be regenerated without the hardware.
  Neither can an override somebody typed, a save, or an established name.

Linux and macOS already make the same split (`~/.local/share` against `~/.cache`), so this is
one rule rather than a Windows special case. **In a portable run they are the same directory**,
because the point of portable mode is that everything is on the stick.

The move took the roaming root from 4.6 GB to 14 MB.
