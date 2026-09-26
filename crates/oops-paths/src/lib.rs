//! Where a tool keeps what it writes.
//!
//! ```no_run
//! let paths = oops_paths::Paths::resolve("orbistoun");
//! std::fs::create_dir_all(paths.logs_dir()).unwrap();
//! ```
//!
//! Only locations every tool needs are named here: the roots, logs, cache, a config file and
//! per-title data. Concepts owned by one project stay in that project.
//!
//! # One directory for the collection
//!
//! Every tool resolves the same `OOPS` directory, so tools share files about the same titles
//! and hardware. Only the config file is per tool, named after it (D013).
//!
//! # Layout
//!
//! [`Layout::PlatformNative`], the default, uses the directory the operating system nominates:
//! `%APPDATA%\OOPS` on Windows, `~/Library/Application Support/OOPS` on macOS,
//! `~/.local/share/OOPS` on Linux. [`Layout::Home`] uses `$HOME/.config/OOPS` everywhere, for
//! a tool packaged in an `MSIX`/`AppX` container, where the platform directory is redirected
//! out of the user's sight (D007).
//!
//! # Two roots
//!
//! [`Paths::data_root`] holds what a person would carry to their next machine: config, saves,
//! established names. [`Paths::cache_root`] holds what can be rebuilt: models, runtimes,
//! shaders, downloads, traces, logs. On Windows they are roaming and local application data;
//! on Linux and macOS, the data and cache directories. In a portable run they are the same
//! directory (D014).
//!
//! # Portable mode
//!
//! A portable run keeps everything beside the binary. It is on when any of these holds:
//!
//! 1. `<APP>_PORTABLE` or `OOPS_PORTABLE` is `1`, `true`, `yes` or `on`.
//! 2. A `.portable` directory sits beside the executable ([`enable_portable_sentinel`]).
//! 3. The executable's name contains `portable`.
//!
//! Precedence: portable mode, then `<APP>_DATA_DIR` or `OOPS_DATA_DIR`, then the layout.
//!
//! # Nowhere to write
//!
//! With no home directory and no readable executable location, the caller chooses through
//! [`Nowhere`] whether to fall back to the working directory or get `None` (D009):
//!
//! ```no_run
//! # use oops_paths::{Options, Paths};
//! let paths = Paths::resolve("orbistoun");
//! let paths = Paths::resolve_with_options("prosperous", Options::new().refusing());
//! ```

use std::io;
use std::path::{Path, PathBuf};

/// The directory beside the binary that marks a portable installation. Shared by every tool,
/// so one bundle directory is portable for all of them.
pub const PORTABLE_DIR: &str = ".portable";

/// The note written inside [`PORTABLE_DIR`]. The name is shared; the words are the caller's.
pub const PORTABLE_NOTE: &str = "PORTABLE.txt";

/// The directory every project in the collection writes to.
pub const OOPS_DIR: &str = "OOPS";

/// Where the root goes when the run is not portable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// The directory the operating system nominates. Without the `platform-dirs` feature this
    /// behaves as [`Layout::Home`].
    #[default]
    PlatformNative,
    /// `$HOME/.config/OOPS` on every platform, for a tool running inside a packaged container.
    Home,
}

/// The environment variables resolution reads, captured once so tests can supply them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvSnapshot {
    /// Whether a portable variable is truthy.
    pub portable_flag: bool,
    /// An explicit data root, if one was given.
    pub data_dir: Option<PathBuf>,
}

impl EnvSnapshot {
    /// Reads the process environment for `app`. `<APP>_...` wins over `OOPS_...`, so two
    /// tools in one shell can use different roots.
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

/// Whether an environment value means "on": `1`, `true`, `yes` or `on`, any case. Anything
/// else is off.
fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Everything resolution reads about the running process, as a value a test can build.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Process {
    /// The environment.
    pub env: EnvSnapshot,
    /// The executable's directory, if known.
    pub binary_dir: Option<PathBuf>,
    /// The executable's file stem, if known.
    pub binary_name: Option<String>,
    /// The user's home directory, if the machine has one.
    pub home: Option<PathBuf>,
    /// The platform's application-data directory: `%APPDATA%`,
    /// `~/Library/Application Support`, `~/.local/share`.
    pub platform_data: Option<PathBuf>,
    /// The platform's cache directory: `%LOCALAPPDATA%`, `~/Library/Caches`, `~/.cache`.
    /// `None` falls back to [`Process::platform_data`].
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

/// The platform's cache directory, when the `platform-dirs` feature is on.
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

/// The platform's application-data directory, when the `platform-dirs` feature is on.
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
    cache_root: PathBuf,
    /// The asking tool; names its config file.
    app: String,
    portable: bool,
}

/// What resolution does when the machine offers no home and no readable executable location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Nowhere {
    /// Use a directory under the working directory. For a tool that needs somewhere for a
    /// cache and would rather run than refuse.
    #[default]
    UseWorkingDirectory,
    /// Resolve to `None`. For a tool keeping something a person will look for later, which
    /// must not land wherever they happened to be standing.
    Refuse,
}

/// What resolution may vary by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Options {
    /// Where the non-portable root goes.
    pub layout: Layout,
    /// What to do when there is nowhere proper.
    pub nowhere: Nowhere,
}

impl Options {
    /// [`Layout::PlatformNative`] and [`Nowhere::UseWorkingDirectory`].
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

    /// Refuses rather than writing to the working directory.
    #[must_use]
    pub const fn refusing(mut self) -> Self {
        self.nowhere = Nowhere::Refuse;
        self
    }
}

impl Paths {
    /// Resolves for the current process with default [`Options`]. Never fails.
    #[must_use]
    pub fn resolve(app: &str) -> Self {
        Self::resolve_found(app, Layout::default(), &Process::read(app)).0
    }

    /// Resolves for the current process. `None` only when refusing and there is nowhere.
    #[must_use]
    pub fn resolve_with_options(app: &str, options: Options) -> Option<Self> {
        Self::resolve_with(app, options, &Process::read(app))
    }

    /// Resolves from a given [`Process`]. `None` only when refusing and there is nowhere.
    #[must_use]
    pub fn resolve_with(app: &str, options: Options, process: &Process) -> Option<Self> {
        let (paths, proper) = Self::resolve_found(app, options.layout, process);
        (proper || options.nowhere != Nowhere::Refuse).then_some(paths)
    }

    /// Resolves from a given [`Process`], always returning an answer plus whether it is a
    /// proper location. For a caller that always wants a root and applies no [`Nowhere`]
    /// policy.
    #[must_use]
    pub fn resolve_found(app: &str, layout: Layout, process: &Process) -> (Self, bool) {
        let binary_dir = process.binary_dir.as_deref();
        let sentinel = binary_dir.is_some_and(|dir| dir.join(PORTABLE_DIR).exists());
        let named = process
            .binary_name
            .as_ref()
            .is_some_and(|n| n.to_ascii_lowercase().contains("portable"));

        // Portable mode beats an explicit root, so a stale variable cannot undo it.
        if process.env.portable_flag || sentinel || named {
            // With no binary directory the run is rooted at the working directory, which is
            // not a proper location.
            let base = binary_dir.map_or_else(|| PathBuf::from("."), Path::to_path_buf);
            let root = base.join(PORTABLE_DIR);
            return (
                Self::new(root.clone(), root, app, true),
                binary_dir.is_some(),
            );
        }
        // An explicit root is used as given, with no `OOPS` appended, for data and cache.
        if let Some(dir) = process.env.data_dir.as_ref() {
            return (Self::new(dir.clone(), dir.clone(), app, false), true);
        }
        let (base, proper) = default_root(layout, process);
        let cache = match (layout, process.platform_cache.as_ref()) {
            (Layout::PlatformNative, Some(native)) => native.join(OOPS_DIR),
            _ => base.clone(),
        };
        (Self::new(base, cache, app, false), proper)
    }

    fn new(data_root: PathBuf, cache_root: PathBuf, app: &str, portable: bool) -> Self {
        Self {
            data_root,
            cache_root,
            app: app.to_owned(),
            portable,
        }
    }

    /// Paths under a root chosen by the caller, with the cache in the same place. For tests
    /// and callers with their own rule.
    #[must_use]
    pub fn rooted_at(data_root: impl Into<PathBuf>) -> Self {
        let data_root = data_root.into();
        Self::new(data_root.clone(), data_root, "oops", false)
    }

    /// Whether this run is confined beside its binary.
    #[must_use]
    pub const fn is_portable(&self) -> bool {
        self.portable
    }

    /// The root for data worth keeping.
    #[must_use]
    pub fn data_root(&self) -> &Path {
        &self.data_root
    }

    /// Everything known about one title, by identifier.
    ///
    /// `fs/` under it is the title's guest filesystem, laid out by the guest's own paths:
    /// Prosperous pulls `savedata` into it from hardware and Orbistoun mounts it as the
    /// title's overlay.
    #[must_use]
    pub fn title_dir(&self, title: &str) -> PathBuf {
        self.data_root.join("titles").join(title)
    }

    /// The root for rebuildable material. Equal to [`Paths::data_root`] in a portable run.
    #[must_use]
    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }

    /// Where log files go; a log belongs to one machine, so it is under the cache root.
    #[must_use]
    pub fn logs_dir(&self) -> PathBuf {
        self.cache_root.join("logs")
    }

    /// Regenerable material. Deleting the whole directory is always safe.
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.cache_root.join("cache")
    }

    /// This tool's config file, `<app>.toml` in the data root.
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.data_root.join(format!("{}.toml", self.app))
    }

    /// Every directory this type names, for display.
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

    /// Creates every named directory.
    ///
    /// # Errors
    ///
    /// If a directory cannot be created; the error names the path.
    pub fn ensure_dirs(&self) -> io::Result<()> {
        for (_, dir) in self.named_dirs() {
            std::fs::create_dir_all(&dir).map_err(|error| {
                io::Error::new(error.kind(), format!("{}: {error}", dir.display()))
            })?;
        }
        Ok(())
    }
}

/// The collection root under a layout, and whether it is a proper location.
///
/// Falls back from the platform directory to the home layout, and from the home layout to
/// `OOPS` under the working directory, which is not proper.
fn default_root(layout: Layout, process: &Process) -> (PathBuf, bool) {
    if layout == Layout::PlatformNative
        && let Some(native) = process.platform_data.as_ref()
    {
        return (native.join(OOPS_DIR), true);
    }
    process.home.as_ref().map_or_else(
        || (PathBuf::from(OOPS_DIR), false),
        |home| (home.join(".config").join(OOPS_DIR), true),
    )
}

/// The user's home directory, from `USERPROFILE` or `HOME`.
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Marks `binary_dir` as a portable installation by creating [`PORTABLE_DIR`] there.
///
/// A `.portable` file in the way is replaced: the sentinel is also the portable root, so a
/// file there makes the run portable with nowhere to write. `note`, when given, is written as
/// [`PORTABLE_NOTE`] to say what the directory does; `None` writes no file. Calling this again
/// is a no-op.
///
/// # Errors
///
/// If the stale file cannot be removed, the directory cannot be created, or the note cannot be
/// written.
pub fn enable_portable_sentinel(binary_dir: &Path, note: Option<&str>) -> io::Result<()> {
    let sentinel = binary_dir.join(PORTABLE_DIR);
    if sentinel.is_file() {
        std::fs::remove_file(&sentinel)?;
    }
    std::fs::create_dir_all(&sentinel)?;
    let Some(body) = note else {
        return Ok(());
    };
    std::fs::write(sentinel.join(PORTABLE_NOTE), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A process with a binary, a home and platform directories.
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

    /// A process with no home and no executable location.
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

    /// Portable mode wins over an explicit root.
    #[test]
    fn portable_beats_an_explicit_root() {
        let paths = resolve(
            Options::new(),
            &process(true, Some("/elsewhere"), Some("app")),
        )
        .unwrap();
        assert!(paths.is_portable());
        assert_eq!(paths.data_root(), Path::new("/opt/app").join(PORTABLE_DIR));
    }

    /// An explicit root wins over the layout.
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

    /// An executable whose name contains `portable` runs portable.
    #[test]
    fn a_binary_calling_itself_portable_is_portable() {
        let paths = resolve(
            Options::new(),
            &process(false, None, Some("app-portable-x86_64")),
        )
        .unwrap();
        assert!(paths.is_portable());
    }

    /// Only the listed values turn portable mode on; `no` does not.
    #[test]
    fn only_recognised_values_turn_portable_mode_on() {
        for on in ["1", "true", "YES", " on "] {
            assert!(is_truthy(on), "{on:?} should be on");
        }
        for off in ["no", "0", "false", "off", "", "maybe"] {
            assert!(!is_truthy(off), "{off:?} should not be on");
        }
    }

    /// With nowhere to stand, the default falls back to the working directory.
    #[test]
    fn nowhere_to_stand_falls_back_by_default() {
        let paths = resolve(Options::new(), &nowhere(true)).unwrap();
        assert!(paths.is_portable());
        assert_eq!(paths.data_root(), Path::new(".").join(PORTABLE_DIR));
    }

    /// With nowhere to stand, refusing yields `None`.
    #[test]
    fn nowhere_to_stand_refuses_when_asked_to() {
        assert!(resolve(Options::new().refusing(), &nowhere(true)).is_none());
        assert!(resolve(Options::new().refusing(), &nowhere(false)).is_none());
    }

    /// Refusing changes nothing when there is a proper location.
    #[test]
    fn refusing_changes_nothing_when_there_is_somewhere_to_stand() {
        let here = process(true, None, Some("app"));
        assert_eq!(
            resolve(Options::new(), &here),
            resolve(Options::new().refusing(), &here)
        );
    }

    /// An explicit root is a proper answer even when refusing.
    #[test]
    fn an_explicit_root_is_an_answer_even_when_refusing() {
        let mut nothing = nowhere(false);
        nothing.env.data_dir = Some(PathBuf::from("/elsewhere"));
        let paths = resolve(Options::new().refusing(), &nothing).unwrap();
        assert_eq!(paths.data_root(), Path::new("/elsewhere"));
    }

    /// The default layout is the platform's data directory.
    #[test]
    fn the_default_layout_is_the_platform_s_own() {
        let paths = resolve(Options::new(), &process(false, None, Some("app"))).unwrap();
        assert_eq!(paths.data_root(), Path::new("/appdata").join(OOPS_DIR));
        assert!(!paths.is_portable());
    }

    /// The home layout is `.config/OOPS` under the home directory.
    #[test]
    fn the_home_layout_is_a_dotted_directory_under_the_home() {
        let options = Options::new().layout(Layout::Home);
        let paths = resolve(options, &process(false, None, Some("app"))).unwrap();
        assert_eq!(
            paths.data_root(),
            Path::new("/home/someone").join(".config").join(OOPS_DIR)
        );
        assert!(!paths.is_portable());
    }

    /// Two tools share the root and title directories; only their config files differ.
    #[test]
    fn two_projects_resolve_to_the_same_directory() {
        let here = process(false, None, Some("app"));
        let one = Paths::resolve_with("prosperous", Options::new(), &here).unwrap();
        let two = Paths::resolve_with("orbistoun", Options::new(), &here).unwrap();
        assert_eq!(one.data_root(), two.data_root());
        assert_eq!(one.title_dir("CUSA00001"), two.title_dir("CUSA00001"));
        assert_ne!(one.config_file(), two.config_file());
        assert!(one.config_file().ends_with("prosperous.toml"));
    }

    /// Portable tools beside one another share one root too.
    #[test]
    fn a_portable_run_shares_between_projects_too() {
        let here = process(true, None, Some("app"));
        let one = Paths::resolve_with("prosperous", Options::new(), &here).unwrap();
        let two = Paths::resolve_with("orbistoun", Options::new(), &here).unwrap();
        assert_eq!(one.data_root(), two.data_root());
        assert!(one.data_root().starts_with("/opt/app"));
    }

    /// Logs and cache go to the platform cache root, not the data root.
    #[test]
    fn bulk_goes_to_the_cache_root_and_not_the_roaming_one() {
        let paths = resolve(Options::new(), &process(false, None, Some("app"))).unwrap();
        assert_eq!(paths.data_root(), Path::new("/appdata").join(OOPS_DIR));
        assert_eq!(
            paths.cache_root(),
            Path::new("/localappdata").join(OOPS_DIR)
        );
        assert!(paths.logs_dir().starts_with(paths.cache_root()));
    }

    /// A portable run keeps data and cache in one directory.
    #[test]
    fn a_portable_run_keeps_everything_in_one_directory() {
        let paths = resolve(Options::new(), &process(true, None, Some("app"))).unwrap();
        assert_eq!(paths.data_root(), paths.cache_root());
        assert!(paths.logs_dir().starts_with(paths.data_root()));
    }

    /// Every named directory and the config file sit under the root.
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

    /// A hyphenated app name reads its variables without panicking.
    #[test]
    fn an_apps_own_variable_is_named_after_it() {
        let snapshot = EnvSnapshot::from_process("obscene-tool");
        assert_eq!(snapshot, EnvSnapshot::default());
    }

    /// A fresh directory under the system temporary one.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("oops-paths-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the system temporary directory should be writable");
        dir
    }

    /// A `.portable` file blocks every write; enabling the sentinel replaces it with a
    /// directory, and enabling again is a no-op.
    #[test]
    fn a_stale_portable_file_is_healed_rather_than_left_to_fail_every_write() {
        let dir = scratch("stale-sentinel");
        let sentinel = dir.join(PORTABLE_DIR);
        std::fs::write(&sentinel, b"a marker file").expect("should write");

        let mut here = process(false, None, Some("app"));
        here.binary_dir = Some(dir.clone());
        let (paths, _) = Paths::resolve_found("app", Layout::default(), &here);
        assert!(paths.is_portable(), "a file satisfies `exists()`");
        assert_eq!(paths.data_root(), sentinel);
        assert!(
            paths.ensure_dirs().is_err(),
            "nothing can be created beneath a file"
        );

        enable_portable_sentinel(&dir, None).expect("should replace the stale file");
        assert!(sentinel.is_dir());
        paths
            .ensure_dirs()
            .expect("the same run should now have somewhere to write");

        enable_portable_sentinel(&dir, None).expect("should be idempotent");
        assert!(sentinel.is_dir());

        std::fs::remove_dir_all(&dir).expect("should clean up after itself");
    }

    /// No note writes no file; a note is written verbatim.
    #[test]
    fn the_note_is_the_callers_words_and_no_file_at_all_without_them() {
        let dir = scratch("sentinel-note");
        let sentinel = dir.join(PORTABLE_DIR);

        enable_portable_sentinel(&dir, None).expect("should create the sentinel");
        assert!(sentinel.is_dir());
        assert!(!sentinel.join(PORTABLE_NOTE).exists());

        let body = "This directory makes the tool run in portable mode.\n";
        enable_portable_sentinel(&dir, Some(body)).expect("should write the note");
        assert_eq!(
            std::fs::read_to_string(sentinel.join(PORTABLE_NOTE)).expect("should read back"),
            body
        );

        std::fs::remove_dir_all(&dir).expect("should clean up after itself");
    }
}
