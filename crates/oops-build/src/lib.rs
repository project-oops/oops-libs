//! Which build this is.
//!
//! # Why a running program should be able to say
//!
//! A screenshot, a bug report and a working copy are three claims about the same software, and
//! without a stamp there is no way to tell whether they agree.
//!
//! # Why this is shared rather than written per project
//!
//! It was written twice before this crate existed, and the two copies drifted into being
//! *complementary*: each solved half the problem and carried the half the other had already
//! fixed. Neither was wrong on purpose, and nothing caught it, because a stamp that reads
//! `no commit` looks like a local build rather than a defect.
//!
//! - One asked git directly, handled a modified tree, and shortened hashes - but emitted no
//!   build time and no assembled line.
//! - The other assembled a readable line with a UTC timestamp - but read its commit from an
//!   environment variable **nothing ever set**, in CI or anywhere else, so every binary it ever
//!   produced stamped `no commit`. Its timestamp was also a constant baked into one crate,
//!   which records when *that crate* was last compiled rather than when the binary was made.
//!
//! Both halves are here, and the failure mode that hid the first defect is what
//! [`Stamp::is_exact`] exists to make visible.
//!
//! # Using it
//!
//! This crate goes in **both** dependency tables, because half of it runs at build time and half
//! at run time:
//!
//! ```toml
//! [dependencies]
//! oops-build = { path = "../../oops-libs/crates/oops-build" }
//! [build-dependencies]
//! oops-build = { path = "../../oops-libs/crates/oops-build" }
//! ```
//!
//! In the consumer's `build.rs`:
//!
//! ```ignore
//! fn main() {
//!     oops_build::emit();
//! }
//! ```
//!
//! Then anywhere in the consumer:
//!
//! ```ignore
//! let stamp = oops_build::stamp!();   // "v0.3.1 - a1b2c3d - built 2026-08-29 14:03 UTC"
//! ```
//!
//! # Why the reading half is macros
//!
//! `env!("CARGO_PKG_VERSION")` inside *this* crate would report this crate's version, and
//! `option_env!` here would read the environment of this crate's compilation rather than the
//! consumer's. Both have to expand at the call site, so both are macros. That is not an
//! aesthetic choice - a plain function here silently reports the wrong thing.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The environment variable the build script sets and the macros read.
///
/// One name across every project, so a CI workflow that exports it works for all of them.
pub const COMMIT_ENV: &str = "OOPS_COMMIT";

// ---------------------------------------------------------------------------------------
// Build-script half
// ---------------------------------------------------------------------------------------

/// Stamps the commit into the calling crate. Call from a consumer's `build.rs`.
///
/// # Why this asks git rather than waiting to be told
///
/// A build script that only reads an environment variable is correct exactly when something
/// sets it, and silently useless otherwise - which is how one of the two original copies came
/// to stamp `no commit` on every binary it ever produced. Asking git needs no configuration and
/// works in a checkout, in CI, and for anybody who clones. An explicitly supplied value still
/// wins, so a build system that knows better can say so.
///
/// A modified tree gets `-dirty`, because a binary built from edits is not the commit it would
/// otherwise name, and a stamp pointing at a commit somebody can check out has to be true or it
/// is worse than saying nothing.
pub fn emit() {
    // Always watched: this is how CI supplies the commit, and a value that changed without
    // re-stamping would put the previous run's SHA into this run's binary.
    println!("cargo:rerun-if-env-changed={COMMIT_ENV}");
    watch_git();

    if let Ok(supplied) = std::env::var(COMMIT_ENV) {
        let supplied = supplied.trim();
        if !supplied.is_empty() {
            println!("cargo:rustc-env={COMMIT_ENV}={}", shorten(supplied));
            return;
        }
    }

    let Some(short) = git(&["rev-parse", "--short", "HEAD"]) else {
        // No commit, no git, or a repository with no history yet. All ordinary, and all
        // meaning the same thing to a reader: this build has no commit to name. The reading
        // half falls back to when the binary was written.
        return;
    };
    // `--quiet` exits non-zero when there is something to report, so a failure here means the
    // tree is modified rather than that the command did not run.
    let dirty = Command::new("git")
        .args(["diff", "--quiet", "HEAD"])
        .status()
        .is_ok_and(|status| !status.success());
    let suffix = if dirty { "-dirty" } else { "" };
    println!("cargo:rustc-env={COMMIT_ENV}={short}{suffix}");
}

/// Re-run when the commit moves or the index changes, and not on every build.
///
/// Naming a path that does not exist makes cargo re-run the script on **every** build, so a
/// source tarball with no `.git` would pay a rebuild for a stamp it can never have. The
/// directory is found by walking up rather than hardcoded, because the depth from a crate to
/// the repository root differs across these projects and a wrong relative path fails silently.
fn watch_git() {
    let Some(root) = repo_root() else { return };
    for name in ["HEAD", "index"] {
        let watched = root.join(name);
        if watched.exists() {
            println!("cargo:rerun-if-changed={}", watched.display());
        }
    }
}

/// The nearest `.git` at or above the crate being built, if there is one.
///
/// Handles the worktree and submodule case, where `.git` is a file pointing elsewhere: there is
/// nothing useful to watch then, so it reports nothing rather than watching a file that never
/// changes.
fn repo_root() -> Option<PathBuf> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let mut dir: Option<&Path> = Some(Path::new(&manifest));
    while let Some(here) = dir {
        let candidate = here.join(".git");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if candidate.is_file() {
            return None;
        }
        dir = here.parent();
    }
    None
}

/// One git command, or nothing at all when git is absent or unhappy.
fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// A commit as it should be displayed.
///
/// # Why the shortening happens here rather than in the workflow
///
/// GitHub Actions hands out the full forty-character SHA and its expression syntax cannot slice
/// a string, so shortening in the workflow means a shell step whose only job is to cut seven
/// characters off - in every workflow, in every project, kept in step by hand. Doing it here
/// means a workflow passes the SHA unmodified and one place decides how long a displayed commit
/// is.
///
/// **Only a hash is shortened.** Anything that is not plain hex - a tag, a `git describe`
/// string, a branch name - passes through whole, because truncating those would produce
/// something that looks like an identifier and identifies nothing.
fn shorten(supplied: &str) -> String {
    const DISPLAY_LENGTH: usize = 7;
    let looks_like_a_hash =
        supplied.len() > DISPLAY_LENGTH && supplied.chars().all(|c| c.is_ascii_hexdigit());
    if looks_like_a_hash {
        supplied[..DISPLAY_LENGTH].to_owned()
    } else {
        supplied.to_owned()
    }
}

// ---------------------------------------------------------------------------------------
// Reading half
// ---------------------------------------------------------------------------------------

/// What a build can say about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    /// The consumer's package version.
    pub version: &'static str,
    /// The commit, when the build knew one. `None` is the honest answer for a build made
    /// outside a repository.
    pub commit: Option<&'static str>,
    /// When the running executable was written, in seconds since the epoch.
    pub built_at: Option<u64>,
}

impl Stamp {
    /// Whether this names a commit somebody else could check out.
    ///
    /// False for a local build, and **false for a modified tree** - a `-dirty` commit names a
    /// tree that only exists on one machine. Report code should ask this rather than test
    /// `commit.is_some()`, because the whole reason this crate exists is that a stamp which
    /// merely *looks* populated is the failure that goes unnoticed.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        self.commit.is_some_and(|c| !c.ends_with("-dirty"))
    }

    /// The one-line form: version, commit if there is one, otherwise when it was built.
    ///
    /// Assembled here so **every front end says the same thing**. A window footer and a
    /// `--version` that disagree are two claims about one binary.
    #[must_use]
    pub fn line(&self) -> String {
        match (self.commit, self.built_at) {
            (Some(commit), _) => format!("v{} - {commit}", self.version),
            (None, Some(at)) => format!("v{} - built {}", self.version, utc(at)),
            (None, None) => format!("v{} - no commit, build time unknown", self.version),
        }
    }
}

/// When the running executable was written, in seconds since the epoch.
///
/// # Why the file's own time rather than a compile-time constant
///
/// A constant baked into a crate records when *that crate* was last compiled, which is not the
/// same thing and is older whenever a change above it triggered the link. The executable's mtime
/// is when the binary a person is actually running came into existence. One of the two original
/// copies used a constant and was wrong in exactly this way.
#[must_use]
pub fn built_at() -> Option<u64> {
    let exe = std::env::current_exe().ok()?;
    exe.metadata()
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|since| since.as_secs())
}

/// Seconds since the epoch, as a date somebody can read.
///
/// Always UTC, and it says so: a build stamp in local time is ambiguous the moment it is pasted
/// into a report by somebody in another place.
///
/// The first version of this emitted the raw seconds, on the grounds that it needed no
/// dependency and no locale. It was correct and useless - the stamp exists to be read by a
/// person glancing at a window, and `1787734934` is read by nobody.
#[must_use]
pub fn utc(seconds: u64) -> String {
    let seconds = i64::try_from(seconds).unwrap_or(0);
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let (hour, minute) = (rest / 3600, (rest % 3600) / 60);
    let (year, month, day) = civil(days);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02} UTC")
}

/// The civil date for a count of days since 1970-01-01.
///
/// The standard era-based conversion: shift the epoch to the start of a 400-year era so the
/// leap-year rules become arithmetic rather than branches. Left in the well-known form rather
/// than rewritten to look nicer, so it can be checked against the published version.
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_position = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_position + 2) / 5 + 1;
    let month = if month_position < 10 {
        month_position + 3
    } else {
        month_position - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Assembles a [`Stamp`] from values the macros captured at the call site.
///
/// Not called directly - see [`stamp!`].
#[doc(hidden)]
#[must_use]
pub fn assemble(version: &'static str, commit: Option<&'static str>) -> Stamp {
    Stamp {
        version,
        // An empty variable and an unset one mean the same thing to a reader, and CI sets
        // empty ones by accident far more often than it sets wrong ones.
        commit: commit.filter(|c| !c.is_empty()),
        built_at: built_at(),
    }
}

/// The calling crate's package version.
#[macro_export]
macro_rules! version {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}

/// The commit the calling crate was built from, if [`emit`] found one.
#[macro_export]
macro_rules! commit {
    () => {
        option_env!("OOPS_COMMIT")
    };
}

/// This build, as a [`Stamp`].
///
/// Expands at the call site so it reads the consumer's version and the consumer's stamped
/// commit rather than this crate's.
#[macro_export]
macro_rules! stamp {
    () => {
        $crate::assemble($crate::version!(), $crate::commit!())
    };
}

/// This build in one line, borrowed for the life of the process.
///
/// # Why this is not just `stamp!().line()`
///
/// `clap` builds its `--version` text from a `&'static str`, and the line cannot be a `const`:
/// part of it is when the *executable* was written, which is only knowable at run time. So it
/// has to be computed once and kept, and every command-line tool in the collection needs the
/// same three lines to do it. They are here instead.
///
/// ```ignore
/// #[command(version = oops_build::line!())]
/// ```
///
/// Each expansion has its own storage, so using it twice in one crate is fine.
#[macro_export]
macro_rules! line {
    () => {{
        static LINE: ::std::sync::OnceLock<::std::string::String> = ::std::sync::OnceLock::new();
        LINE.get_or_init(|| $crate::stamp!().line()).as_str()
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hash_is_shortened_and_anything_else_is_not() {
        assert_eq!(shorten("a1b2c3d4e5f6a7b8"), "a1b2c3d");
        // A tag truncated to seven characters looks like an identifier and identifies nothing.
        assert_eq!(shorten("v1.2.3"), "v1.2.3");
        assert_eq!(shorten("release-2026-08"), "release-2026-08");
        // Already short enough to be left alone.
        assert_eq!(shorten("a1b2c3d"), "a1b2c3d");
    }

    #[test]
    fn the_epoch_and_a_known_date_read_correctly() {
        assert_eq!(utc(0), "1970-01-01 00:00 UTC");
        // 2026-08-29 00:00:00 UTC.
        assert_eq!(utc(1_787_961_600), "2026-08-29 00:00 UTC");
    }

    #[test]
    fn a_leap_day_is_a_day() {
        // 2024-02-29, the case a naive 365-day conversion gets wrong.
        assert_eq!(utc(1_709_164_800), "2024-02-29 00:00 UTC");
    }

    #[test]
    fn a_dirty_tree_is_not_an_exact_build() {
        let exact = Stamp {
            version: "0.1.0",
            commit: Some("a1b2c3d"),
            built_at: Some(0),
        };
        let dirty = Stamp {
            version: "0.1.0",
            commit: Some("a1b2c3d-dirty"),
            built_at: Some(0),
        };
        assert!(exact.is_exact());
        // The point of the crate: `commit.is_some()` is true here and the answer is still no.
        assert!(!dirty.is_exact());
        assert!(dirty.commit.is_some());
    }

    #[test]
    fn a_build_with_no_commit_says_when_it_was_made_instead() {
        let local = Stamp {
            version: "0.1.0",
            commit: None,
            built_at: Some(0),
        };
        assert_eq!(local.line(), "v0.1.0 - built 1970-01-01 00:00 UTC");
        assert!(!local.is_exact());
    }

    #[test]
    fn an_empty_commit_is_treated_as_no_commit() {
        // CI exports an empty variable far more often than a wrong one, and `Some("")` would
        // otherwise render as a stamp with a blank where the commit should be.
        assert_eq!(assemble("0.1.0", Some("")).commit, None);
    }

    #[test]
    fn nothing_at_all_still_produces_a_line() {
        let nothing = Stamp {
            version: "0.1.0",
            commit: None,
            built_at: None,
        };
        assert_eq!(nothing.line(), "v0.1.0 - no commit, build time unknown");
    }
}
