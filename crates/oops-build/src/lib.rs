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
/// the reflog `logs/HEAD` (which covers packed refs). In a worktree checkout, `.git` is a
/// file pointing to the worktree's git directory, and shared refs live under the repository's
/// `commondir`. Only existing paths are named, because cargo re-runs on every build for a
/// missing one. The index is not watched, so `-dirty` can lag one build behind an edit.
fn watch_git() {
    let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    for path in git_paths_to_watch(Path::new(&manifest)) {
        println!("cargo:rerun-if-changed={}", path.display());
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitDirs {
    /// Directory holding this checkout's `HEAD` and `logs/HEAD`.
    git_dir: PathBuf,
    /// Directory holding shared references (`refs/heads/...`).
    common_dir: PathBuf,
}

/// Resolves the git directories for a crate manifest directory.
///
/// Follows `.git` directories directly, or `.git` files (`gitdir: <path>`) for worktrees.
/// In a worktree, `commondir` (if present) points to the repository holding shared refs.
fn resolve_git_dirs(manifest_dir: &Path) -> Option<GitDirs> {
    let mut dir: Option<&Path> = Some(manifest_dir);
    while let Some(here) = dir {
        let candidate = here.join(".git");
        if candidate.is_dir() {
            return Some(GitDirs {
                git_dir: candidate.clone(),
                common_dir: candidate,
            });
        }
        if candidate.is_file() {
            let content = std::fs::read_to_string(&candidate).ok()?;
            let gitdir_str = parse_gitdir_line(&content)?;
            let git_dir_path = Path::new(&gitdir_str);
            let git_dir = if git_dir_path.is_absolute() {
                git_dir_path.to_path_buf()
            } else {
                here.join(git_dir_path)
            };
            if !git_dir.is_dir() {
                return None;
            }
            let commondir_file = git_dir.join("commondir");
            let common_dir = if let Ok(common_content) = std::fs::read_to_string(&commondir_file) {
                let common_str = common_content.trim();
                let common_path = Path::new(common_str);
                if common_path.is_absolute() {
                    common_path.to_path_buf()
                } else {
                    git_dir.join(common_path)
                }
            } else {
                git_dir.clone()
            };
            return Some(GitDirs {
                git_dir,
                common_dir,
            });
        }
        dir = here.parent();
    }
    None
}

/// The paths to watch for changes to the commit or branch.
///
/// Only existing files are returned: `HEAD`, the ref `HEAD` points to (under the common
/// git directory), and `logs/HEAD`.
fn git_paths_to_watch(manifest_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let Some(dirs) = resolve_git_dirs(manifest_dir) else {
        return paths;
    };
    let head = dirs.git_dir.join("HEAD");
    if !head.exists() {
        return paths;
    }
    paths.push(head.clone());

    if let Some(reference) = head_reference(&head) {
        let ref_path = dirs.common_dir.join(&reference);
        if ref_path.exists() {
            paths.push(ref_path);
        }
    }

    let reflog = dirs.git_dir.join("logs").join("HEAD");
    if reflog.exists() {
        paths.push(reflog);
    }

    paths
}

/// Extracts the target path from a `.git` file (`gitdir: <path>`).
fn parse_gitdir_line(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(gitdir) = trimmed.strip_prefix("gitdir:") {
            let path = gitdir.trim();
            if !path.is_empty() {
                return Some(path.to_owned());
            }
        }
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

    /// A `gitdir:` line extracts the path to the target git directory.
    #[test]
    fn a_gitdir_line_extracts_path() {
        assert_eq!(
            parse_gitdir_line("gitdir: /path/to/worktree\n").as_deref(),
            Some("/path/to/worktree")
        );
        assert_eq!(
            parse_gitdir_line("gitdir:   relative/path  \r\n").as_deref(),
            Some("relative/path")
        );
        assert_eq!(
            parse_gitdir_line("# comment\ngitdir: /target/dir\n").as_deref(),
            Some("/target/dir")
        );
        assert_eq!(parse_gitdir_line("other: foo"), None);
        assert_eq!(parse_gitdir_line("gitdir:"), None);
        assert_eq!(parse_gitdir_line(""), None);
    }

    /// A standard repository watches its `HEAD`, the ref `HEAD` points to, and `logs/HEAD`.
    #[test]
    fn a_standard_git_repo_watches_head_and_refs() {
        let temp = std::env::temp_dir().join(format!("oops-build-test-std-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);

        let crate_dir = temp.join("repo").join("crates").join("my-crate");
        let git_dir = temp.join("repo").join(".git");

        std::fs::create_dir_all(&crate_dir).unwrap();
        std::fs::create_dir_all(git_dir.join("refs").join("heads")).unwrap();
        std::fs::create_dir_all(git_dir.join("logs")).unwrap();

        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(git_dir.join("refs").join("heads").join("main"), "hash\n").unwrap();
        std::fs::write(git_dir.join("logs").join("HEAD"), "log\n").unwrap();

        let watched = git_paths_to_watch(&crate_dir);
        assert_eq!(watched.len(), 3);
        assert_eq!(watched[0], git_dir.join("HEAD"));
        assert_eq!(watched[1], git_dir.join("refs").join("heads").join("main"));
        assert_eq!(watched[2], git_dir.join("logs").join("HEAD"));

        let _ = std::fs::remove_dir_all(&temp);
    }

    /// A worktree checkout watches its own `HEAD`, `logs/HEAD`, and the ref under `commondir`.
    #[test]
    fn a_git_worktree_watches_worktree_head_and_common_refs() {
        let temp = std::env::temp_dir().join(format!("oops-build-test-wt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);

        let crate_dir = temp.join("repo-worktree").join("crates").join("my-crate");
        let worktree_dir = temp.join("repo-worktree");
        let common_git = temp.join("main-repo").join(".git");
        let worktree_git = common_git.join("worktrees").join("repo-worktree");

        std::fs::create_dir_all(&crate_dir).unwrap();
        std::fs::create_dir_all(worktree_git.join("logs")).unwrap();
        std::fs::create_dir_all(common_git.join("refs").join("heads")).unwrap();

        // Write .git file in worktree pointing to worktree_git
        std::fs::write(
            worktree_dir.join(".git"),
            format!("gitdir: {}\n", worktree_git.display()),
        )
        .unwrap();

        // Write HEAD in worktree_git pointing to branch ref
        std::fs::write(worktree_git.join("HEAD"), "ref: refs/heads/feature\n").unwrap();
        // Write commondir pointing to common .git (relative to worktree_git)
        std::fs::write(worktree_git.join("commondir"), "../..\n").unwrap();
        // Write logs/HEAD in worktree_git
        std::fs::write(worktree_git.join("logs").join("HEAD"), "reflog entry\n").unwrap();
        // Write branch ref in common_git
        let branch_ref = common_git.join("refs").join("heads").join("feature");
        std::fs::write(&branch_ref, "0123456789abcdef\n").unwrap();

        let watched = git_paths_to_watch(&crate_dir);
        assert_eq!(watched.len(), 3);
        assert_eq!(watched[0], worktree_git.join("HEAD"));
        assert_eq!(
            watched[1],
            worktree_git.join("../..").join("refs/heads/feature")
        );
        assert_eq!(watched[2], worktree_git.join("logs").join("HEAD"));

        let _ = std::fs::remove_dir_all(&temp);
    }
}
