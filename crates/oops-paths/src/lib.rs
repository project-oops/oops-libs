//! Where a tool keeps what it writes.
//!
//! ```no_run
//! let paths = oops_paths::Paths::resolve("orbistoun");
//! std::fs::create_dir_all(paths.logs_dir()).unwrap();
//! ```
//!
//! Only the locations every tool needs are here - a root, logs, cache, a config file. Anything
//! that names a concept belonging to one project stays in that project: this crate should never
//! learn what a title, a payload or a package is.
//!
//! # Two layouts, and the platform's own is the default
//!
//! [`Layout::PlatformNative`] uses the directory the operating system nominates -
//! `%APPDATA%\OOPS` on Windows, `~/Library/Application Support/OOPS` on macOS,
//! `~/.local/share/OOPS` on Linux. [`Layout::Home`] puts everything under `$HOME/.config/OOPS`
//! on every platform.
//!
//! **`PlatformNative` is the default**, because it is where a person, a backup tool and a
//! roaming profile all already look. A dotted directory under `$HOME` on Windows is a Unix
//! convention that nothing on Windows knows about.
//!
//! This was briefly the other way round, on the following reasoning: a tool running inside a
//! packaged container has its writes to the per-user application data directory redirected into
//! a per-package cache, invisible to the same user in an ordinary shell, and a configuration
//! file the user cannot find is worse than none.
//!
//! **That effect is real and it is not this collection's.** It applies to an application running
//! *inside* an `MSIX`/`AppX` container. These are plain executables, so the hazard was borrowed
//! from a case that does not apply, and generalising it cost the platform's own answer
//! everywhere. `Home` stays available for a tool that is genuinely packaged, which is when it
//! becomes the right choice rather than a cautious one.
//!
//! # Portable mode
//!
//! A portable run keeps everything beside the binary and touches nothing else on the machine. It
//! turns on when **any** of these is true, checked in this order:
//!
//! 1. `<APP>_PORTABLE` or `OOPS_PORTABLE` is set to `1`, `true`, `yes` or `on`.
//! 2. A `.portable` directory sits beside the executable - see [`enable_portable_sentinel`].
//! 3. The executable's own name contains `portable`, which is how a downloaded build can
//!    announce itself without anybody configuring anything.
//!
//! An explicit `<APP>_DATA_DIR` (or `OOPS_DATA_DIR`) beats the layout but **not** portable mode:
//! somebody who asked for a self-contained run gets one.
//!
//! # Two roots, because not everything deserves to be carried
//!
//! Windows distinguishes *roaming* application data from *local*, and it is not a formality: a
//! domain profile synchronises the roaming one at logon. Four gigabytes of downloaded model
//! weights in there is a slow login for something that can be fetched again.
//!
//! So [`Paths::data_root`] is what a person would want on their next machine - configuration,
//! the address book, saves, established names - and [`Paths::cache_root`] is what can be
//! rebuilt: models, runtimes, compiled shaders, downloaded packages, traces, logs.
//!
//! On Linux and macOS this is the same distinction the platform already makes
//! (`~/.local/share` against `~/.cache`, `Application Support` against `Caches`), so it is one
//! rule rather than a Windows special case. **In a portable run they are the same directory**,
//! because the point of portable mode is that everything is in one place somebody can carry.
//!
//! # When there is nowhere
//!
//! No home directory *and* no readable executable location is rare and real. What should happen
//! is the application's call, not this crate's, so it is a parameter - see [`Nowhere`]:
//!
//! ```no_run
//! # use oops_paths::{Options, Paths};
//! // A cache: somewhere is better than nowhere.
//! let paths = Paths::resolve("orbistoun");
//!
//! // A registry a person will go looking for: refuse rather than hide it.
//! let paths = Paths::resolve_with_options("prosperous", Options::new().refusing());
//! ```

use std::io;
use std::path::{Path, PathBuf};

/// The directory beside the binary that marks a portable installation.
///
/// Not named after any one tool, so a directory holding several of them is portable for all of
/// them at once - which is what somebody unpacking a bundle onto a stick expects.
pub const PORTABLE_DIR: &str = ".portable";

/// The directory every project in the collection writes to.
///
/// # One directory, not one per project
///
/// Because they are about the same console and the same titles. Prosperous pulls a save off
/// real hardware; Orbistoun mounts it. obSCEne records which machine it probed; Prosperous
/// already knows that machine's address. Cheats, titles, reports and the address book are all
/// facts about the platform rather than possessions of one tool.
///
/// This began as a subdirectory per project with a `shared/` beside them, on the grounds that
/// the configs have different shapes and "delete what Orbistoun kept" should be one operation.
/// **It was the wrong default.** Partitioning by tool made sharing the exception that had to be
/// argued for each time, when sharing is the reason these four live together at all - and the
/// partition was buying almost nothing: across the three projects that had written anything,
/// **no filename appeared in more than one of them.**
///
/// The one thing that would genuinely collide is a per-tool configuration file, so those are
/// named after the tool - see [`Paths::config_file`] - rather than buried a directory deep.
pub const OOPS_DIR: &str = "OOPS";

/// Where the root goes when the run is not portable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// The directory the operating system nominates. The default - see the crate note.
    ///
    /// Needs the `platform-dirs` feature, which is on by default; without it this behaves as
    /// [`Layout::Home`], because falling back to a working location beats failing to start.
    #[default]
    PlatformNative,
    /// `$HOME/.config/OOPS`, on every platform.
    ///
    /// Right for an application that is packaged in a container, where the platform's own
    /// directory is redirected somewhere the user cannot reach. See the crate note.
    Home,
}

/// What the environment says, read once so resolution can be tested without touching it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvSnapshot {
    /// Whether a portable variable was set to something truthy.
    pub portable_flag: bool,
    /// An explicit data root, if one was given.
    pub data_dir: Option<PathBuf>,
}

impl EnvSnapshot {
    /// Reads the real process environment for one application.
    ///
    /// The application's own variable wins over the shared one, so two tools from this
    /// collection can be pointed at different roots in the same shell - which a single
    /// `OOPS_DATA_DIR` would make impossible.
    #[must_use]
    pub fn from_process(app: &str) -> Self {
        let prefix = app.to_ascii_uppercase().replace(['-', ' '], "_");
        let first = |suffix: &str| {
            std::env::var_os(format!("{prefix}_{suffix}"))
                .or_else(|| std::env::var_os(format!("OOPS_{suffix}")))
        };
        Self {
            portable_flag: first("PORTABLE")
                .and_then(|v| v.into_string().ok())
                .is_some_and(|v| is_truthy(&v)),
            data_dir: first("DATA_DIR")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
        }
    }
}

/// Whether an environment value counts as "on".
///
/// Deliberately narrow and case-insensitive. An unrecognised value is **not** truthy, because
/// silently reading `OOPS_PORTABLE=no` as on would be exactly the kind of surprise portable mode
/// must not have.
fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// What resolution reads about the running process.
///
/// Gathered into one value so it can be built by hand in a test - the rules are worth checking
/// without a real home directory, a real environment or a real executable, and none of those
/// can be arranged from inside a test that must not touch the machine it runs on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Process {
    /// The environment, already read.
    pub env: EnvSnapshot,
    /// Where the executable lives, if that can be determined.
    pub binary_dir: Option<PathBuf>,
    /// The executable's file stem, if that can be determined.
    pub binary_name: Option<String>,
    /// The user's home directory, if this machine has one.
    ///
    /// Carried here rather than read where it is used, so that "a machine with no home" is a
    /// value a test can construct. It is the condition the whole [`Nowhere`] policy exists for,
    /// and a rule that can only be exercised on a machine that happens to lack a home directory
    /// is a rule nobody ever checks.
    pub home: Option<PathBuf>,
    /// The directory the operating system nominates for application data.
    ///
    /// `%APPDATA%` on Windows, `~/Library/Application Support` on macOS, `~/.local/share` on
    /// Linux. Carried for exactly the same reason as [`Process::home`], and it had to be: when
    /// this was read where it was used, the platform lookup answered from the real machine and
    /// the "nowhere to write" branch became unreachable from a test again - the second time the
    /// same hole opened, through the same cause.
    pub platform_data: Option<PathBuf>,
    /// The directory the operating system nominates for material that can be rebuilt.
    ///
    /// `%LOCALAPPDATA%` on Windows, `~/Library/Caches` on macOS, `~/.cache` on Linux. `None`
    /// falls back to [`Process::platform_data`], so a machine that offers only one gets one.
    pub platform_cache: Option<PathBuf>,
}

impl Process {
    /// Reads the real process.
    #[must_use]
    pub fn read(app: &str) -> Self {
        let exe = std::env::current_exe().ok();
        Self {
            env: EnvSnapshot::from_process(app),
            binary_dir: exe.as_deref().and_then(Path::parent).map(Path::to_path_buf),
            binary_name: exe
                .as_deref()
                .and_then(Path::file_stem)
                .and_then(|s| s.to_str())
                .map(str::to_owned),
            home: home_dir(),
            platform_data: platform_data_dir(),
            platform_cache: platform_cache_dir(),
        }
    }
}

/// The platform's own cache directory, when the feature that can find it is on.
fn platform_cache_dir() -> Option<PathBuf> {
    #[cfg(feature = "platform-dirs")]
    {
        dirs::cache_dir()
    }
    #[cfg(not(feature = "platform-dirs"))]
    {
        None
    }
}

/// The platform's own application-data directory, when the feature that can find it is on.
fn platform_data_dir() -> Option<PathBuf> {
    #[cfg(feature = "platform-dirs")]
    {
        dirs::data_dir()
    }
    #[cfg(not(feature = "platform-dirs"))]
    {
        None
    }
}

/// A resolved set of writable locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    data_root: PathBuf,
    /// Where rebuildable bulk goes. The same as `data_root` in a portable run.
    cache_root: PathBuf,
    /// Which tool is asking. Not part of any directory - it names this tool's configuration
    /// file, and prefixes the environment variables it answers to.
    app: String,
    portable: bool,
}

/// What resolution should do when the machine offers nowhere proper to write.
///
/// This arises when there is no home directory *and* the executable's own location cannot be
/// read - rare, and real: some launchers, some containers, some sandboxes.
///
/// The choice belongs to the application rather than to this crate, because the two answers
/// suit different tools and neither is wrong:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Nowhere {
    /// Use a directory named after the application, under the working directory.
    ///
    /// Right for a tool that just needs somewhere to put a cache and would rather run than
    /// refuse.
    #[default]
    UseWorkingDirectory,
    /// Resolve to nothing, so the caller can say so.
    ///
    /// Right for a tool keeping something a person will go looking for later. **A
    /// configuration file the user cannot find is worse than no configuration file**, and a
    /// registry written beside wherever they happened to be standing is exactly that.
    Refuse,
}

/// Everything resolution is allowed to vary by.
///
/// A struct rather than a widening list of arguments, and rather than a `bool` that reads as
/// `resolve(app, true, false)` at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Options {
    /// Where the non-portable root goes.
    pub layout: Layout,
    /// What to do when there is nowhere proper.
    pub nowhere: Nowhere,
}

impl Options {
    /// Defaults: [`Layout::PlatformNative`] and [`Nowhere::UseWorkingDirectory`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the layout.
    #[must_use]
    pub const fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    /// Refuse rather than write to the working directory.
    #[must_use]
    pub const fn refusing(mut self) -> Self {
        self.nowhere = Nowhere::Refuse;
        self
    }
}

impl Paths {
    /// Resolves for the current process with default [`Options`].
    ///
    /// Never fails: with [`Nowhere::UseWorkingDirectory`] there is always an answer. A tool that
    /// would rather refuse wants [`Paths::resolve_with_options`] and
    /// [`Options::refusing`].
    #[must_use]
    pub fn resolve(app: &str) -> Self {
        Self::resolve_found(app, Layout::default(), &Process::read(app)).0
    }

    /// Resolves for the current process.
    ///
    /// `None` only when [`Options::nowhere`] is [`Nowhere::Refuse`] and there was nowhere.
    #[must_use]
    pub fn resolve_with_options(app: &str, options: Options) -> Option<Self> {
        let (paths, proper) = Self::resolve_found(app, options.layout, &Process::read(app));
        (proper || options.nowhere != Nowhere::Refuse).then_some(paths)
    }

    /// Resolution core, parameterised on every input it reads.
    ///
    /// Separate from [`Paths::resolve`] so the rules can be tested without a real environment, a
    /// real home directory, or a real executable. `binary_dir` is `None` where it cannot be
    /// determined, in which case the sentinel is not looked for and a forced portable run is
    /// rooted at the working directory.
    #[must_use]
    pub fn resolve_with(app: &str, options: Options, process: &Process) -> Option<Self> {
        let (paths, proper) = Self::resolve_found(app, options.layout, process);
        (proper || options.nowhere != Nowhere::Refuse).then_some(paths)
    }

    /// Resolution itself: always an answer, plus whether it is a proper one.
    ///
    /// The infallible half. [`Nowhere`] is applied by the callers above rather than in here, so
    /// this has no way to fail and needs no `unwrap` anywhere to express "the default cannot
    /// refuse" - splitting *compute* from *is this acceptable* removes the unreachable branch
    /// rather than documenting it.
    ///
    /// Public because a caller may want the same split: orbistoun layers a dozen directories of
    /// its own on top of this root and always wants one, so it takes the answer and ignores the
    /// flag. Going through [`Paths::resolve_with`] would have meant an `expect` at that call
    /// site for a branch that cannot happen - which is the thing this shape exists to avoid.
    #[must_use]
    pub fn resolve_found(app: &str, layout: Layout, process: &Process) -> (Self, bool) {
        let binary_dir = process.binary_dir.as_deref();
        let sentinel = binary_dir.is_some_and(|dir| dir.join(PORTABLE_DIR).exists());
        let named = process
            .binary_name
            .as_ref()
            .is_some_and(|n| n.to_ascii_lowercase().contains("portable"));

        // Checked before the explicit root on purpose: somebody who asked for a self-contained
        // run gets one, and a stale variable in their shell does not quietly undo it.
        if process.env.portable_flag || sentinel || named {
            // No binary directory is the same "nowhere proper" condition as no home: the run
            // would be rooted at whatever directory the user happened to be standing in.
            let base = binary_dir.map_or_else(|| PathBuf::from("."), Path::to_path_buf);
            // The portable root *is* the collection root - a stick holding several of these
            // tools shares between them exactly as an installed set does.
            return (
                Self::under(&base.join(PORTABLE_DIR), app, true),
                binary_dir.is_some(),
            );
        }
        if let Some(dir) = process.env.data_dir.as_ref() {
            // Somebody who names a path means that path: no `OOPS` appended, because they have
            // already said where the collection is.
            return (
                Self {
                    data_root: dir.clone(),
                    // Somebody who named one directory meant one directory.
                    cache_root: dir.clone(),
                    app: app.to_owned(),
                    portable: false,
                },
                true,
            );
        }
        let (base, proper) = default_root(layout, process);
        // The cache falls back to the data root when the platform offers no separate one, so a
        // machine with a single directory still works and nothing has to handle an absence.
        let cache = match (layout, process.platform_cache.as_ref()) {
            (Layout::PlatformNative, Some(native)) => native.join(OOPS_DIR),
            _ => base.clone(),
        };
        (Self::split(&base, &cache, app), proper)
    }

    /// The collection's directory, which is every tool's directory.
    fn under(oops_root: &Path, app: &str, portable: bool) -> Self {
        Self {
            data_root: oops_root.to_path_buf(),
            cache_root: oops_root.to_path_buf(),
            app: app.to_owned(),
            portable,
        }
    }

    /// The same, with a separate home for what can be rebuilt.
    fn split(data_root: &Path, cache_root: &Path, app: &str) -> Self {
        Self {
            data_root: data_root.to_path_buf(),
            cache_root: cache_root.to_path_buf(),
            app: app.to_owned(),
            portable: false,
        }
    }

    /// Builds from a root somebody else chose. For tests, and for a caller with its own rule.
    #[must_use]
    pub fn rooted_at(data_root: impl Into<PathBuf>) -> Self {
        let data_root = data_root.into();
        Self {
            cache_root: data_root.clone(),
            data_root,
            app: "oops".to_owned(),
            portable: false,
        }
    }

    /// Whether this run is confined beside its binary.
    #[must_use]
    pub const fn is_portable(&self) -> bool {
        self.portable
    }

    /// The root everything else hangs off.
    #[must_use]
    pub fn data_root(&self) -> &Path {
        &self.data_root
    }

    /// Everything known about one title, by its identifier.
    ///
    /// `fs/` under it is the title's guest filesystem, **shaped by the guest's own paths**:
    /// Prosperous writes one by pulling `savedata` off real hardware, Orbistoun mounts the same
    /// tree as that title's overlay. Neither had to learn the other's format, because the
    /// guest's path is the format.
    #[must_use]
    pub fn title_dir(&self, title: &str) -> PathBuf {
        self.data_root.join("titles").join(title)
    }

    /// Where rebuildable bulk goes: models, runtimes, compiled shaders, downloads, logs.
    ///
    /// A separate directory from [`Paths::data_root`] on every platform that distinguishes
    /// them, and the same directory in a portable run. See the crate note.
    #[must_use]
    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }

    /// Where log files go. Hand this to `oops-log`'s `to_file`.
    ///
    /// Under the cache root: a log is a record of one machine's run, and carrying it to another
    /// machine would be carrying somebody else's answers.
    #[must_use]
    pub fn logs_dir(&self) -> PathBuf {
        self.cache_root.join("logs")
    }

    /// Where regenerable material goes.
    ///
    /// Named so that deleting the whole directory is obviously safe - anything that could not
    /// survive that does not belong in it.
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.cache_root.join("cache")
    }

    /// This tool's configuration file, named after the tool.
    ///
    /// `orbistoun.toml`, `prosperous.toml`. **The one thing that would genuinely collide** now
    /// that the tools share a directory - a bare `config.toml` each. Naming them costs nothing
    /// and avoids the only real argument for partitioning by tool.
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.data_root.join(format!("{}.toml", self.app))
    }

    /// Every directory this type names, for a tool that wants to show a person where things are.
    #[must_use]
    pub fn named_dirs(&self) -> Vec<(&'static str, PathBuf)> {
        vec![
            ("root", self.data_root.clone()),
            ("cache-root", self.cache_root.clone()),
            ("logs", self.logs_dir()),
            ("cache", self.cache_dir()),
            ("titles", self.data_root.join("titles")),
        ]
    }

    /// Creates the directories, so a caller can fail early and once rather than at each write.
    ///
    /// # Errors
    ///
    /// If any directory cannot be created. The error names the path, because "permission denied"
    /// without one is a message that sends somebody looking.
    pub fn ensure_dirs(&self) -> io::Result<()> {
        for (_, dir) in self.named_dirs() {
            std::fs::create_dir_all(&dir).map_err(|error| {
                io::Error::new(error.kind(), format!("{}: {error}", dir.display()))
            })?;
        }
        Ok(())
    }
}

/// The collection's root under a layout, and whether it is a real location.
///
/// The *collection's*, not the application's - the application name is appended by
/// [`Paths::under`], so every project lands beside its siblings rather than beside unrelated
/// software.
///
/// Reads nothing. Both candidate directories arrive on [`Process`], so every branch here -
/// including "this machine offers nowhere" - can be reached from a test.
fn default_root(layout: Layout, process: &Process) -> (PathBuf, bool) {
    // A let-chain rather than two nested `if`s. Edition 2024 stabilised them, and clippy's
    // `collapsible_if` fires on the nested form there - so this shape is what the edition the
    // rest of the collection uses actually asks for.
    //
    // Falling through means no platform directory, which also means the feature that finds one
    // is off. The home layout below is the fallback rather than a failure: somewhere the user
    // can find beats refusing to start.
    if layout == Layout::PlatformNative
        && let Some(native) = process.platform_data.as_ref()
    {
        return (native.join(OOPS_DIR), true);
    }
    process.home.as_ref().map_or_else(
        // Nowhere to call home either. A visible directory under the working directory beats
        // panicking - but the caller is told, so it can refuse instead. See `Nowhere`.
        || (PathBuf::from(OOPS_DIR), false),
        |home| (home.join(".config").join(OOPS_DIR), true),
    )
}

/// The user's home directory, without a dependency for it.
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Marks a directory as a portable installation by creating the sentinel.
///
/// # Errors
///
/// If the directory cannot be created - typically an installation directory the user cannot
/// write to, which is exactly the case where they wanted portable mode and cannot have it, so it
/// is worth reporting rather than swallowing.
pub fn enable_portable_sentinel(binary_dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(binary_dir.join(PORTABLE_DIR))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A process with somewhere to stand, which is the ordinary case.
    fn process(portable: bool, data_dir: Option<&str>, name: Option<&str>) -> Process {
        Process {
            env: EnvSnapshot {
                portable_flag: portable,
                data_dir: data_dir.map(PathBuf::from),
            },
            binary_dir: Some(PathBuf::from("/opt/app")),
            binary_name: name.map(str::to_owned),
            home: Some(PathBuf::from("/home/someone")),
            platform_data: Some(PathBuf::from("/appdata")),
            platform_cache: Some(PathBuf::from("/localappdata")),
        }
    }

    /// A process with nowhere: no home to be found and no readable executable location.
    fn nowhere(portable: bool) -> Process {
        Process {
            env: EnvSnapshot {
                portable_flag: portable,
                data_dir: None,
            },
            binary_dir: None,
            binary_name: None,
            home: None,
            platform_data: None,
            platform_cache: None,
        }
    }

    fn resolve(options: Options, process: &Process) -> Option<Paths> {
        Paths::resolve_with("app", options, process)
    }

    #[test]
    fn portable_beats_an_explicit_root() {
        // Somebody who asked for a self-contained run gets one; a variable left in their shell
        // does not quietly undo it.
        let paths = resolve(
            Options::new(),
            &process(true, Some("/elsewhere"), Some("app")),
        )
        .unwrap();
        assert!(paths.is_portable());
        assert_eq!(paths.data_root(), Path::new("/opt/app").join(PORTABLE_DIR));
    }

    #[test]
    fn an_explicit_root_beats_the_layout() {
        let paths = resolve(
            Options::new(),
            &process(false, Some("/elsewhere"), Some("app")),
        )
        .unwrap();
        assert!(!paths.is_portable());
        assert_eq!(paths.data_root(), Path::new("/elsewhere"));
    }

    #[test]
    fn a_binary_calling_itself_portable_is_portable() {
        let paths = resolve(
            Options::new(),
            &process(false, None, Some("app-portable-x86_64")),
        )
        .unwrap();
        assert!(paths.is_portable());
    }

    #[test]
    fn only_recognised_values_turn_portable_mode_on() {
        for on in ["1", "true", "YES", " on "] {
            assert!(is_truthy(on), "{on:?} should be on");
        }
        // The one that matters: reading `no` as on would be the surprise portable mode must
        // never have.
        for off in ["no", "0", "false", "off", "", "maybe"] {
            assert!(!is_truthy(off), "{off:?} should not be on");
        }
    }

    #[test]
    fn nowhere_to_stand_falls_back_by_default() {
        // The default is to run rather than refuse, so there is an answer and it is under the
        // working directory.
        let paths = resolve(Options::new(), &nowhere(true)).unwrap();
        assert!(paths.is_portable());
        assert_eq!(paths.data_root(), Path::new(".").join(PORTABLE_DIR));
    }

    #[test]
    fn nowhere_to_stand_refuses_when_asked_to() {
        // The same condition, and the other answer. A registry written beside wherever the
        // user happened to be standing is a file they will never find again.
        assert!(resolve(Options::new().refusing(), &nowhere(true)).is_none());
        assert!(resolve(Options::new().refusing(), &nowhere(false)).is_none());
    }

    #[test]
    fn refusing_changes_nothing_when_there_is_somewhere_to_stand() {
        // The parameter must only govern the edge case. If it altered the ordinary answer it
        // would be a second layout rather than a policy about failure.
        let here = process(true, None, Some("app"));
        assert_eq!(
            resolve(Options::new(), &here),
            resolve(Options::new().refusing(), &here)
        );
    }

    #[test]
    fn an_explicit_root_is_an_answer_even_when_refusing() {
        // Somebody who named a directory has answered the question themselves, so there is
        // nothing left to refuse - even on a machine with nowhere else at all.
        let mut nothing = nowhere(false);
        nothing.env.data_dir = Some(PathBuf::from("/elsewhere"));
        let paths = resolve(Options::new().refusing(), &nothing).unwrap();
        assert_eq!(paths.data_root(), Path::new("/elsewhere"));
    }

    #[test]
    fn the_default_layout_is_the_platform_s_own() {
        // The default, and the thing most likely to be "simplified" back to a dotted directory
        // under `$HOME` by somebody who has only ever run this on Linux.
        let paths = resolve(Options::new(), &process(false, None, Some("app"))).unwrap();
        assert_eq!(paths.data_root(), Path::new("/appdata").join(OOPS_DIR));
        assert!(!paths.is_portable());
    }

    #[test]
    fn the_home_layout_is_a_dotted_directory_under_the_home() {
        // Available for an application packaged in a container, where the platform's own
        // directory is redirected somewhere the user cannot reach.
        let options = Options::new().layout(Layout::Home);
        let paths = resolve(options, &process(false, None, Some("app"))).unwrap();
        assert_eq!(
            paths.data_root(),
            Path::new("/home/someone").join(".config").join(OOPS_DIR)
        );
        assert!(!paths.is_portable());
    }

    #[test]
    fn two_projects_resolve_to_the_same_directory() {
        // The whole reason for one root. A save Prosperous pulls off real hardware and the
        // overlay Orbistoun mounts have to be the same tree, or this is four tools that merely
        // live near each other.
        let here = process(false, None, Some("app"));
        let one = Paths::resolve_with("prosperous", Options::new(), &here).unwrap();
        let two = Paths::resolve_with("orbistoun", Options::new(), &here).unwrap();
        assert_eq!(one.data_root(), two.data_root());
        assert_eq!(one.title_dir("CUSA00001"), two.title_dir("CUSA00001"));
        // The one thing that is per-tool, because a bare `config.toml` each is the only real
        // collision a shared directory has.
        assert_ne!(one.config_file(), two.config_file());
        assert!(one.config_file().ends_with("prosperous.toml"));
    }

    #[test]
    fn a_portable_run_shares_between_projects_too() {
        // A stick holding several of these tools shares exactly as an installed set does,
        // rather than being a special case somebody discovers later.
        let here = process(true, None, Some("app"));
        let one = Paths::resolve_with("prosperous", Options::new(), &here).unwrap();
        let two = Paths::resolve_with("orbistoun", Options::new(), &here).unwrap();
        assert_eq!(one.data_root(), two.data_root());
        assert!(one.data_root().starts_with("/opt/app"));
    }

    #[test]
    fn bulk_goes_to_the_cache_root_and_not_the_roaming_one() {
        // Four gigabytes of model weights in a roaming profile is a slow login for something
        // that can be downloaded again. The two roots are what stops that.
        let paths = resolve(Options::new(), &process(false, None, Some("app"))).unwrap();
        assert_eq!(paths.data_root(), Path::new("/appdata").join(OOPS_DIR));
        assert_eq!(
            paths.cache_root(),
            Path::new("/localappdata").join(OOPS_DIR)
        );
        assert!(paths.logs_dir().starts_with(paths.cache_root()));
    }

    #[test]
    fn a_portable_run_keeps_everything_in_one_directory() {
        // The whole point of portable mode is that it can be carried, so splitting it across
        // two directories would defeat it - and the platform's own cache directory is not on
        // the stick.
        let paths = resolve(Options::new(), &process(true, None, Some("app"))).unwrap();
        assert_eq!(paths.data_root(), paths.cache_root());
        assert!(paths.logs_dir().starts_with(paths.data_root()));
    }

    #[test]
    fn the_directories_all_sit_under_the_root() {
        let paths = Paths::rooted_at("/data/app");
        for (name, dir) in paths.named_dirs() {
            assert!(
                dir.starts_with("/data/app"),
                "{name} escaped the root: {}",
                dir.display()
            );
        }
        assert!(paths.config_file().starts_with("/data/app"));
    }

    #[test]
    fn an_apps_own_variable_is_named_after_it() {
        // Two tools in one shell must be able to disagree about where their data lives, which a
        // single shared variable would make impossible.
        let snapshot = EnvSnapshot::from_process("obscene-tool");
        // Nothing is set in the test environment; what is pinned is that reading a hyphenated
        // name does not panic and produces the empty answer.
        assert_eq!(snapshot, EnvSnapshot::default());
    }
}
