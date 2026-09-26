//! Which build this is: version, commit and build time, stamped once and read the same way
//! by every tool.
//!
//! The crate goes in both dependency tables, because half of it runs at build time:
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
//! let stamp = oops_build::stamp!(); // "v0.3.1 - a1b2c3d"
//! ```
//!
//! The reading half is macros because `env!` and `option_env!` must expand in the consumer to
//! see the consumer's version and commit; a function here would report this crate's.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The environment variable the build script sets and the macros read. CI may set it to
/// supply the commit.
pub const COMMIT_ENV: &str = "OOPS_COMMIT";

// Build-script half.

/// Stamps the commit into the calling crate. Call from the consumer's `build.rs`.
///
/// A non-empty [`COMMIT_ENV`] wins; otherwise git is asked, and a modified tree gets a
/// `-dirty` suffix. Outside a repository nothing is stamped.
pub fn emit() {
    // Watched so a CI-supplied commit re-stamps when it changes.
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
        return;
    };
    // `--quiet` exits non-zero when the tree differs from HEAD.
    let dirty = Command::new("git")
        .args(["diff", "--quiet", "HEAD"])
        .status()
        .is_ok_and(|status| !status.success());
    let suffix = if dirty { "-dirty" } else { "" };
    println!("cargo:rustc-env={COMMIT_ENV}={short}{suffix}");
}

/// Re-runs the build script when the commit moves.
///
/// Watches `HEAD`, the ref it names (which is the file that changes on a branch commit) and
/// the reflog `logs/HEAD` (which covers packed refs). Only existing paths are named, because
/// cargo re-runs on every build for a missing one. The index is not watched, so `-dirty` can
/// lag one build behind an edit.
fn watch_git() {
    let Some(root) = repo_root() else { return };
    let head = root.join("HEAD");
    if !head.exists() {
        return;
    }
    println!("cargo:rerun-if-changed={}", head.display());
    if let Some(reference) = head_reference(&head) {
        let ref_path = root.join(&reference);
        if ref_path.exists() {
            println!("cargo:rerun-if-changed={}", ref_path.display());
        }
    }
    let reflog = root.join("logs").join("HEAD");
    if reflog.exists() {
        println!("cargo:rerun-if-changed={}", reflog.display());
    }
}

/// The ref `HEAD` names, e.g. `refs/heads/main`; `None` when detached or unreadable.
fn head_reference(head: &Path) -> Option<String> {
    parse_head_reference(&std::fs::read_to_string(head).ok()?)
}

/// The ref in a `HEAD` file's contents: `ref: <path>` on a branch, a bare id when detached.
fn parse_head_reference(contents: &str) -> Option<String> {
    let reference = contents.strip_prefix("ref:")?.trim();
    (!reference.is_empty()).then(|| reference.to_owned())
}

/// The nearest `.git` directory at or above the crate being built.
///
/// `None` when `.git` is a file (a worktree or submodule): its contents never change on a
/// commit, so there is nothing useful to watch.
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

/// One git command's trimmed output, or `None` when git is absent or fails.
fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// A supplied commit as displayed: a hex hash is cut to seven characters, anything else (a
/// tag, a `git describe` string) is kept whole. Workflows pass the full SHA unmodified.
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

// Reading half.

/// What a build can say about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    /// The consumer's package version.
    pub version: &'static str,
    /// The commit, or `None` for a build made outside a repository.
    pub commit: Option<&'static str>,
    /// When the running executable was written, in seconds since the epoch.
    pub built_at: Option<u64>,
}

impl Stamp {
    /// Whether this names a commit somebody else could check out: false for a local build
    /// and for a `-dirty` tree. Ask this rather than `commit.is_some()`.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        self.commit.is_some_and(|c| !c.ends_with("-dirty"))
    }

    /// The one-line form every front end shows: version, then the commit, or the build time
    /// when there is no commit.
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
/// The executable's modification time, not a compile-time constant: a constant records when
/// one crate was compiled, which predates the link whenever only a dependant changed.
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

/// Seconds since the epoch as `YYYY-MM-DD HH:MM UTC`. Always UTC, so a pasted stamp is
/// unambiguous.
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
/// Howard Hinnant's era-based `civil_from_days`, kept in its published form so it can be
/// checked against the source. This crate takes no dependencies, so no date library.
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

/// Assembles a [`Stamp`] from values captured at the call site. Used by [`stamp!`].
#[doc(hidden)]
#[must_use]
pub fn assemble(version: &'static str, commit: Option<&'static str>) -> Stamp {
    Stamp {
        version,
        // CI often exports an empty variable; empty means no commit.
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
#[macro_export]
macro_rules! stamp {
    () => {
        $crate::assemble($crate::version!(), $crate::commit!())
    };
}

/// This build in one line, as a `&'static str`, for `clap`'s `version` attribute.
///
/// The line includes the executable's build time, which is known only at run time, so it is
/// computed once and kept. Each expansion has its own storage.
///
/// ```ignore
/// #[command(version = oops_build::line!())]
/// ```
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

    /// A branch `HEAD` names the ref to watch; a detached or empty one names nothing.
    #[test]
    fn a_symbolic_head_names_its_ref_and_a_detached_one_does_not() {
        assert_eq!(
            parse_head_reference("ref: refs/heads/main\n").as_deref(),
            Some("refs/heads/main")
        );
        assert_eq!(
            parse_head_reference("ref: refs/heads/feature/x\n").as_deref(),
            Some("refs/heads/feature/x")
        );
        assert_eq!(parse_head_reference("b4f7e73aabbccddeeff0011\n"), None);
        assert_eq!(parse_head_reference("ref:   \n"), None);
        assert_eq!(parse_head_reference(""), None);
    }

    /// Only a hex hash is shortened; a tag keeps its full name.
    #[test]
    fn a_hash_is_shortened_and_anything_else_is_not() {
        assert_eq!(shorten("a1b2c3d4e5f6a7b8"), "a1b2c3d");
        assert_eq!(shorten("v1.2.3"), "v1.2.3");
        assert_eq!(shorten("release-2026-08"), "release-2026-08");
        assert_eq!(shorten("a1b2c3d"), "a1b2c3d");
    }

    /// The epoch and a known date format correctly.
    #[test]
    fn the_epoch_and_a_known_date_read_correctly() {
        assert_eq!(utc(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc(1_787_961_600), "2026-08-29 00:00 UTC");
    }

    /// A leap day is a real date.
    #[test]
    fn a_leap_day_is_a_day() {
        assert_eq!(utc(1_709_164_800), "2024-02-29 00:00 UTC");
    }

    /// A `-dirty` commit is populated but not exact.
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
        assert!(!dirty.is_exact());
        assert!(dirty.commit.is_some());
    }

    /// Without a commit, the line gives the build time and the stamp is not exact.
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

    /// An empty commit variable counts as no commit.
    #[test]
    fn an_empty_commit_is_treated_as_no_commit() {
        assert_eq!(assemble("0.1.0", Some("")).commit, None);
    }

    /// With neither commit nor build time there is still a line.
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
